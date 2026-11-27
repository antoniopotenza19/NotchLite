//! Original NotchLite code. Local, opt-in, bounded whisper.cpp subprocess STT.
//! This module never opens an audio endpoint or sends text/audio to a server.

use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

pub const SAMPLE_RATE: u32 = 16_000;
pub const MAX_SAMPLES: usize = 8 * SAMPLE_RATE as usize;
const MAX_TEXT: u64 = 8 * 1024;
const PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug)]
pub struct Configuration {
    pub executable: PathBuf,
    pub model: PathBuf,
    pub vad_model: Option<PathBuf>,
    pub language: String,
    pub threads: u8,
}
impl Configuration {
    /// Only called on explicit consent, never by a paint callback.
    pub fn discover() -> Result<Self, String> {
        let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").ok_or("Local application directory unavailable")?)
            .join("NotchLite").join("whisper");
        let manifest_path=root.join("runtime.json");
        ensure_local_tree(&manifest_path).map_err(|reason|format!("{reason}. Install local transcription with tools/Setup-CallTranscription.ps1 first"))?;
        if !fs::symlink_metadata(&manifest_path).map_err(|_|"Transcription configuration unavailable")?.is_file() {
            return Err("Transcription configuration must be a regular local file".into());
        }
        let file = File::open(manifest_path).map_err(|_| "Install local transcription with tools/Setup-CallTranscription.ps1 first")?;
        let mut bytes = Vec::new();
        file.take(4097).read_to_end(&mut bytes).map_err(|_| "Cannot read transcription configuration")?;
        if bytes.len() > 4096 { return Err("Transcription configuration too large".into()); }
        let manifest: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "Invalid transcription configuration")?;
        let executable = manifest["executable"].as_str().ok_or("Transcription executable missing")?;
        let model = manifest["model"].as_str().ok_or("Transcription model missing")?;
        // Keep legacy installations usable; setup adds the speech detector.
        let vad_model=match manifest.get("vadModel").filter(|v|!v.is_null()) {
            Some(value)=>Some(relative_file(&root,value.as_str().ok_or("Invalid speech detector configuration")?)?),
            None=>None,
        };
        let language=match manifest.get("language") {Some(value)=>value.as_str().ok_or("Invalid transcription language")?,None=>"it"};
        let config = Self { executable: relative_file(&root, executable)?, model: relative_file(&root, model)?, vad_model, language: language.into(), threads: 2 };
        config.validate()?;
        Ok(config)
    }
    fn validate(&self) -> Result<(), String> {
        if !(1..=8).contains(&self.threads) || !valid_language(&self.language) {
            return Err("Use 1–8 CPU threads and an ISO language code or auto".into());
        }
        for path in [&self.executable, &self.model].into_iter().chain(self.vad_model.iter()) {
            ensure_local_tree(path)?;
            let info = fs::symlink_metadata(path).map_err(|_| "Transcription runtime/model unavailable")?;
            if !info.is_file() || is_reparse(&info) { return Err("Transcription files cannot be links or directories".into()); }
        }
        Ok(())
    }
}
fn relative_file(root: &Path, value: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if value.is_empty() || value.contains(':') || !path.components().all(|p| matches!(p, Component::Normal(_))) {
        return Err("Transcription configuration must contain relative file paths".into());
    }
    Ok(root.join(path))
}
fn valid_language(value: &str) -> bool { value == "auto" || ((2..=3).contains(&value.len()) && value.bytes().all(|b| b.is_ascii_lowercase())) }
#[cfg(windows)]
fn is_reparse(info: &fs::Metadata) -> bool { use std::os::windows::fs::MetadataExt; info.file_attributes() & 0x400 != 0 }
#[cfg(not(windows))]
fn is_reparse(info: &fs::Metadata) -> bool { info.file_type().is_symlink() }
fn ensure_local_tree(path: &Path) -> Result<(), String> {
    if !path.is_absolute() || path.to_string_lossy().starts_with(r"\\") {
        return Err("Transcription paths must be local and absolute".into());
    }
    #[cfg(windows)] {
        use windows::core::PCWSTR;
        use windows::Win32::Storage::FileSystem::GetDriveTypeW;
        let raw=path.to_string_lossy();let bytes=raw.as_bytes();
        if bytes.len()<3 || !bytes[0].is_ascii_alphabetic() || bytes[1]!=b':' || !matches!(bytes[2],b'\\'|b'/') {
            return Err("Transcription paths must use a local drive".into());
        }
        let root=[u16::from(bytes[0]),u16::from(b':'),u16::from(b'\\'),0];
        // DRIVE_REMOTE=4; check before filesystem metadata to avoid mapped shares.
        if unsafe{GetDriveTypeW(PCWSTR(root.as_ptr()))}==4 {return Err("Transcription paths cannot use network drives".into());}
    }
    // Validate parents from the drive down before probing a child beneath them.
    // Otherwise a parent junction could already redirect the first metadata read.
    for component in path.ancestors().collect::<Vec<_>>().into_iter().rev() {
        let info=fs::symlink_metadata(component).map_err(|_| "Transcription path unavailable")?;
        if is_reparse(&info) {return Err("Transcription paths cannot contain links or reparse points".into());}
    }
    Ok(())
}

#[derive(Debug)]
pub struct DiagnosticResult { pub text: String, pub samples: usize, pub elapsed_millis: u128 }
/// Explicit file-only check. Never activates an endpoint, microphone or network client.
pub fn check_file(path: &Path, language: &str) -> Result<DiagnosticResult, String> {
    ensure_local_tree(path)?;
    let mut bytes = Vec::new();
    File::open(path).map_err(|_| "Cannot read the diagnostic WAV")?.take((MAX_SAMPLES * 2 + 65_537) as u64)
        .read_to_end(&mut bytes).map_err(|_| "Cannot read the diagnostic WAV")?;
    if bytes.len() > MAX_SAMPLES * 2 + 65_536 { return Err("Diagnostic WAV is too large (maximum eight seconds)".into()); }
    let samples = read_wav(&bytes)?;
    let sample_count = samples.len();
    let mut config = Configuration::discover()?;
    config.language = language.to_string();
    let mut worker = SttWorker::new(config)?;
    let started = Instant::now();
    worker.submit(1, samples)?;
    loop {
        if let Some(result) = worker.poll() {
            return Ok(DiagnosticResult { text: result.text?, samples: sample_count, elapsed_millis: started.elapsed().as_millis() });
        }
        if started.elapsed() > PROCESS_TIMEOUT + Duration::from_secs(5) { return Err("File-only transcription check timed out".into()); }
        thread::sleep(Duration::from_millis(20));
    }
}
fn read_wav(bytes: &[u8]) -> Result<Vec<i16>, String> {
    let invalid = || "Expected a bounded PCM16 mono 16 kHz WAV (maximum eight seconds)".to_string();
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" { return Err(invalid()); }
    let riff_size = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize;
    if riff_size.checked_add(8) != Some(bytes.len()) { return Err(invalid()); }
    let mut offset = 12usize;
    let mut format = false; let mut pcm = None;
    while offset < bytes.len() {
        let header = bytes.get(offset..offset+8).ok_or_else(invalid)?;
        let size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
        offset += 8;
        let end = offset.checked_add(size).ok_or_else(invalid)?;
        let chunk = bytes.get(offset..end).ok_or_else(invalid)?;
        if &header[..4] == b"fmt " {
            if format || chunk.len() < 16 || &chunk[..2] != 1u16.to_le_bytes().as_slice()
                || &chunk[2..4] != 1u16.to_le_bytes().as_slice() || &chunk[4..8] != SAMPLE_RATE.to_le_bytes().as_slice()
                || &chunk[8..12] != (SAMPLE_RATE*2).to_le_bytes().as_slice()
                || &chunk[12..14] != 2u16.to_le_bytes().as_slice() || &chunk[14..16] != 16u16.to_le_bytes().as_slice() { return Err(invalid()); }
            format = true;
        } else if &header[..4] == b"data" {
            if pcm.is_some() || size == 0 || size % 2 != 0 || size > MAX_SAMPLES * 2 { return Err(invalid()); }
            pcm = Some(chunk);
        }
        offset = end.checked_add(size % 2).ok_or_else(invalid)?;
    }
    if !format || offset != bytes.len() { return Err(invalid()); }
    Ok(pcm.ok_or_else(invalid)?.chunks_exact(2).map(|v| i16::from_le_bytes([v[0],v[1]])).collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubmitOutcome { Queued, ReplacedQueued }
#[derive(Debug)]
pub struct SttResult {
    /// Capture generation, passed through unchanged. Separate from cancellation epoch.
    pub generation: u64,
    /// Controller-owned segment identity; overlap deduplication is never guessed.
    pub segment_id: u64,
    pub text: Result<String, String>,
    /// Unpolled older results replaced by this latest result; never an unbounded queue.
    pub dropped_results: u64,
}
struct Job { generation: u64, segment_id: u64, epoch: u64, samples: Vec<i16> }
#[derive(Default)]
struct Queue { pending: Option<Job>, result: Option<SttResult> }
struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
    stopped: AtomicBool,
    epoch: AtomicU64,
    child: Mutex<Option<Child>>,
}
impl Shared {
    fn queue(&self) -> std::sync::MutexGuard<'_, Queue> { self.queue.lock().unwrap_or_else(|e| e.into_inner()) }
    fn cancelled(&self, epoch: u64) -> bool { self.stopped.load(Ordering::Acquire) || self.epoch.load(Ordering::Acquire) != epoch }
    fn publish(&self, job: &Job, text: Result<String, String>) {
        let mut queue = self.queue();
        if self.cancelled(job.epoch) { return; }
        let dropped_results = queue.result.as_ref().map_or(0, |r| r.dropped_results.saturating_add(1));
        queue.result = Some(SttResult { generation: job.generation, segment_id: job.segment_id, text, dropped_results });
    }
}
pub struct SttWorker { config: Configuration, shared: Arc<Shared>, worker: Option<JoinHandle<()>> }
impl SttWorker {
    /// Checks configuration but starts neither a worker nor a model process.
    pub fn new(config: Configuration) -> Result<Self, String> {
        config.validate()?;
        Ok(Self { config, shared: Arc::new(Shared { queue: Mutex::new(Queue::default()), ready: Condvar::new(), stopped: AtomicBool::new(false), epoch: AtomicU64::new(0), child: Mutex::new(None) }), worker: None })
    }
    pub fn submit(&mut self, generation: u64, samples: Vec<i16>) -> Result<SubmitOutcome, String> {
        self.submit_segment(generation,0,samples)
    }
    pub fn submit_segment(&mut self, generation: u64, segment_id:u64, samples: Vec<i16>) -> Result<SubmitOutcome, String> {
        if samples.is_empty() || samples.len() > MAX_SAMPLES { return Err("Expected 1–128000 mono 16 kHz PCM16 samples".into()); }
        if self.shared.stopped.load(Ordering::Acquire) { return Err("Transcription worker stopped".into()); }
        if self.worker.is_none() {
            let shared = Arc::clone(&self.shared); let config = self.config.clone();
            self.worker = Some(thread::Builder::new().name("call-transcription".into()).spawn(move || run(shared, config)).map_err(|_| "Cannot start local transcription worker")?);
        }
        let mut queue = self.shared.queue();
        if self.shared.stopped.load(Ordering::Acquire) { return Err("Transcription worker stopped".into()); }
        let outcome = if queue.pending.is_some() { SubmitOutcome::ReplacedQueued } else { SubmitOutcome::Queued };
        queue.pending = Some(Job { generation, segment_id, epoch: self.shared.epoch.load(Ordering::Acquire), samples });
        self.shared.ready.notify_one();
        Ok(outcome)
    }
    pub fn poll(&self) -> Option<SttResult> { self.shared.queue().result.take() }
    /// Terminal cancellation. The controller creates a new worker on new consent.
    pub fn stop(&mut self) {
        self.shared.stopped.store(true, Ordering::Release);
        self.shared.epoch.fetch_add(1, Ordering::AcqRel);
        { let mut queue = self.shared.queue(); queue.pending = None; queue.result = None; }
        if let Some(child) = self.shared.child.lock().unwrap_or_else(|e| e.into_inner()).as_mut() { let _ = child.kill(); }
        self.shared.ready.notify_all();
        let until = Instant::now() + Duration::from_millis(500);
        while self.worker.as_ref().is_some_and(|w| !w.is_finished()) && Instant::now() < until { thread::sleep(Duration::from_millis(5)); }
        if let Some(worker) = self.worker.take() { if worker.is_finished() { let _ = worker.join(); } }
    }
}
impl Drop for SttWorker { fn drop(&mut self) { self.stop(); } }
fn run(shared: Arc<Shared>, config: Configuration) {
    loop {
        let job = {
            let mut queue = shared.queue();
            while queue.pending.is_none() && !shared.stopped.load(Ordering::Acquire) {
                queue = shared.ready.wait(queue).unwrap_or_else(|e| e.into_inner());
            }
            if shared.stopped.load(Ordering::Acquire) { return; }
            queue.pending.take().expect("pending job checked")
        };
        let text = transcribe(&shared, &config, &job);
        shared.publish(&job, text);
    }
}
fn transcribe(shared: &Shared, config: &Configuration, job: &Job) -> Result<String, String> {
    if shared.cancelled(job.epoch) { return Err("Transcription cancelled".into()); }
    // Digital silence carries no words. Never ask a language model to fill it.
    if job.samples.iter().all(|sample|*sample==0) {return Ok(String::new());}
    let temp = TempAudio::new()?;
    write_wav(&temp.input(), &job.samples).map_err(|_| "Cannot prepare private audio segment")?;
    if shared.cancelled(job.epoch) { return Err("Transcription cancelled".into()); }
    let mut command = Command::new(&config.executable);
    command.args(["--no-gpu", "--no-prints", "--no-timestamps", "--output-txt", "--best-of", "5", "--beam-size", "5", "--suppress-nst", "--max-context", "0", "--audio-ctx", "0"])
        .arg("--threads").arg(config.threads.to_string()).arg("--language").arg(&config.language)
        .arg("--model").arg(&config.model).arg("--file").arg(temp.input()).arg("--output-file").arg(temp.output())
        .current_dir(&temp.0).env("GGML_BACKEND_PATH", config.executable.parent().ok_or("Invalid runtime directory")?)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if let Some(vad)=&config.vad_model {
        command.args(["--vad","--vad-model"]).arg(vad)
            .args(["--vad-min-speech-duration-ms","100","--vad-min-silence-duration-ms","300","--vad-speech-pad-ms","160"]);
    }
    #[cfg(windows)] { use std::os::windows::process::CommandExt; command.creation_flags(0x0800_0000); }
    let mut child = command.spawn().map_err(|_| "Cannot launch local whisper.cpp CPU runtime")?;
    {
        let mut slot = shared.child.lock().unwrap_or_else(|e| e.into_inner());
        if shared.cancelled(job.epoch) { drop(slot); let _ = child.kill(); let _ = child.wait(); return Err("Transcription cancelled".into()); }
        *slot = Some(child);
    }
    let started = Instant::now();
    loop {
        let status = {
            let mut slot = shared.child.lock().unwrap_or_else(|e| e.into_inner());
            if shared.cancelled(job.epoch) || started.elapsed() > PROCESS_TIMEOUT {
                let mut child = slot.take().ok_or("Transcription process unavailable")?;
                drop(slot);
                let _ = child.kill();
                // Keep ownership until reaped; cleanup happens on this worker, never UI.
                let _ = child.wait();
                return Err(if shared.cancelled(job.epoch) { "Transcription cancelled" } else { "Local transcription exceeded 30 seconds" }.into());
            }
            match slot.as_mut().ok_or("Transcription process unavailable")?.try_wait() {
                Ok(Some(status)) => { *slot = None; Some(status) }
                Ok(None) => None,
                Err(_) => { let mut child=slot.take().expect("child present");drop(slot);let _ = child.kill(); let _ = child.wait();return Err("Cannot monitor local transcription process".into()); }
            }
        };
        if let Some(status) = status { if !status.success() { return Err("Local transcription failed; check the CPU runtime/model installation".into()); } break; }
        thread::sleep(Duration::from_millis(20));
    }
    if shared.cancelled(job.epoch) { return Err("Transcription cancelled".into()); }
    let mut bytes = Vec::new();
    File::open(temp.text()).map_err(|_| "Local transcription produced no text")?.take(MAX_TEXT + 1).read_to_end(&mut bytes).map_err(|_| "Cannot read transcription result")?;
    if bytes.len() as u64 > MAX_TEXT { return Err("Transcription result exceeded its text limit".into()); }
    let text = String::from_utf8(bytes).map_err(|_| "Transcription result is not UTF-8")?;
    Ok(text.trim().to_owned())
}
struct TempAudio(PathBuf);
impl TempAudio {
    fn new() -> Result<Self, String> {
        use ring::rand::{SecureRandom, SystemRandom};
        let mut id = [0u8; 16]; SystemRandom::new().fill(&mut id).map_err(|_| "Cannot create private audio identity")?;
        let name: String = id.iter().map(|b| format!("{b:02x}")).collect();
        let temp_root=std::env::temp_dir();
        ensure_local_tree(&temp_root)?;
        let path = temp_root.join(format!("NotchLite-stt-{name}"));
        create_private_directory(&path)?;
        Ok(Self(path))
    }
    fn input(&self) -> PathBuf { self.0.join("segment.wav") }
    fn output(&self) -> PathBuf { self.0.join("transcript") }
    fn text(&self) -> PathBuf { self.0.join("transcript.txt") }
}
impl Drop for TempAudio {
    fn drop(&mut self) { let _ = fs::remove_file(self.input()); let _ = fs::remove_file(self.text()); let _ = fs::remove_dir(&self.0); }
}
#[cfg(windows)]
fn create_private_directory(path: &Path) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::{PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES};
    use windows::Win32::Security::Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW;
    use windows::Win32::Storage::FileSystem::CreateDirectoryW;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        ConvertStringSecurityDescriptorToSecurityDescriptorW(w!("D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;OW)"), 1, &mut descriptor, None).map_err(|_| "Cannot secure temporary audio directory")?;
        let attributes = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: descriptor.0, bInheritHandle: false.into() };
        let created = CreateDirectoryW(PCWSTR(wide.as_ptr()), Some(&attributes));
        let _ = LocalFree(HLOCAL(descriptor.0));
        created.map_err(|_| "Cannot create private temporary audio directory")?;
    }
    Ok(())
}
#[cfg(not(windows))]
fn create_private_directory(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::DirBuilderExt;
    fs::DirBuilder::new().mode(0o700).create(path).map_err(|_| "Cannot create private temporary audio directory".into())
}
fn write_wav(path: &Path, samples: &[i16]) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let data_size = (samples.len() * 2) as u32;
    file.write_all(b"RIFF")?; file.write_all(&(36 + data_size).to_le_bytes())?; file.write_all(b"WAVEfmt ")?;
    file.write_all(&16u32.to_le_bytes())?; file.write_all(&1u16.to_le_bytes())?; file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&SAMPLE_RATE.to_le_bytes())?; file.write_all(&(SAMPLE_RATE * 2).to_le_bytes())?;
    file.write_all(&2u16.to_le_bytes())?; file.write_all(&16u16.to_le_bytes())?; file.write_all(b"data")?; file.write_all(&data_size.to_le_bytes())?;
    let mut buffer = [0u8; 4096];
    for chunk in samples.chunks(buffer.len() / 2) {
        for (bytes, sample) in buffer.chunks_exact_mut(2).zip(chunk) { bytes.copy_from_slice(&sample.to_le_bytes()); }
        file.write_all(&buffer[..chunk.len() * 2])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shared() -> Shared { Shared { queue: Mutex::new(Queue::default()), ready: Condvar::new(), stopped: AtomicBool::new(false), epoch: AtomicU64::new(0), child: Mutex::new(None) } }
    #[test] fn language_and_runtime_paths_cannot_add_cli_options_or_escape_root() {
        assert!(valid_language("it") && valid_language("auto"));
        assert!(!valid_language("it --server") && !valid_language("../en"));
        let root=Path::new("root");
        for path in ["../model", "C:\\model", "/model", "\\\\host\\file", ""] { assert!(relative_file(root,path).is_err()); }
        assert_eq!(relative_file(root,"models/tiny.bin").unwrap(),root.join("models/tiny.bin"));
    }
    #[test] fn cancellation_rejects_old_results_without_rewriting_capture_generation() {
        let state=shared(); let job=Job{generation:42,segment_id:17,epoch:0,samples:vec![0]};
        state.publish(&job,Ok("test".into())); let result=state.queue().result.take().unwrap();assert_eq!(result.generation,42);assert_eq!(result.segment_id,17);
        state.epoch.store(1,Ordering::Release);state.publish(&job,Ok("late".into()));assert!(state.queue().result.is_none());
        state.stopped.store(true,Ordering::Release);let current=Job{epoch:1,..job};state.publish(&current,Ok("late".into()));assert!(state.queue().result.is_none());
    }
    #[test] fn unpolled_results_are_bounded_and_report_replacement() {
        let state=shared();let job=Job{generation:9,segment_id:3,epoch:0,samples:vec![0]};
        state.publish(&job,Ok("first".into()));state.publish(&job,Ok("second".into()));
        let result=state.queue().result.take().unwrap();assert_eq!(result.dropped_results,1);assert_eq!(result.text.unwrap(),"second");
    }
    #[test] fn private_wav_is_pcm16_mono_and_cleanup_is_owned() {
        let temp=TempAudio::new().unwrap(); let root=temp.0.clone();
        write_wav(&temp.input(),&[0, i16::MIN,i16::MAX]).unwrap();let bytes=fs::read(temp.input()).unwrap();
        assert_eq!(bytes.len(),50);assert_eq!(&bytes[..4],b"RIFF");assert_eq!(&bytes[24..28],&16000u32.to_le_bytes());
        assert_eq!(&bytes[40..44],&6u32.to_le_bytes());assert_eq!(&bytes[44..],&[0,0,0,128,255,127]);
        assert!(write_wav(&temp.input(),&[1]).is_err());drop(temp);assert!(!root.exists());
    }
    #[test] fn diagnostic_wav_rejects_truncation_and_wrong_format_before_inference() {
        let temp=TempAudio::new().unwrap();write_wav(&temp.input(),&[0,1,-1]).unwrap();
        let mut bytes=fs::read(temp.input()).unwrap();assert_eq!(read_wav(&bytes).unwrap(),[0,1,-1]);
        assert!(read_wav(&bytes[..49]).is_err());bytes[22]=2;assert!(read_wav(&bytes).is_err());
        assert!(read_wav(b"RIFF").is_err());
    }
    #[test] fn local_paths_reject_unc_and_relative_inputs_before_metadata() {
        for input in ["relative.wav",r"C:relative.wav",r"\\host.invalid\share\file.wav",r"\\?\UNC\host.invalid\share\file.wav"] {
            assert!(ensure_local_tree(Path::new(input)).is_err());
        }
    }
    #[test] fn digital_silence_never_invokes_even_an_unusable_runtime() {
        let temp=TempAudio::new().unwrap();
        fs::write(temp.input(),b"not an executable").unwrap();fs::write(temp.text(),b"not a model").unwrap();
        let config=Configuration{executable:temp.input(),model:temp.text(),vad_model:None,language:"it".into(),threads:1};
        let mut worker=SttWorker::new(config).unwrap();worker.submit_segment(4,17,vec![0;16000]).unwrap();
        let until=Instant::now()+Duration::from_secs(2);
        loop {if let Some(result)=worker.poll(){assert_eq!(result.segment_id,17);assert_eq!(result.text.unwrap(),"");break;}
            assert!(Instant::now()<until);thread::sleep(Duration::from_millis(5));}
    }
}
