//! Original NotchLite process-loopback implementation. No microphone/global fallback.
use super::*;
use ::windows::core::{implement, Interface, IUnknown, PROPVARIANT, HRESULT};
use ::windows::Win32::Media::Audio::*;
use ::windows::Win32::System::Com::CoTaskMemAlloc;
use ::windows::Win32::System::SystemInformation::OSVERSIONINFOW;
use std::sync::atomic::AtomicUsize;

// Windows can finish activation after cancellation. Keep parameters alive in the
// OS-owned completion object, and bound such late operations across monitors.
static ACTIVATIONS: AtomicUsize = AtomicUsize::new(0);
const MAX_ACTIVATIONS: usize = 2;
const MAX_PACKET_FRAMES: usize = SAMPLE_RATE as usize;
const ACTIVATION_TIMEOUT: Duration = Duration::from_secs(10);

#[link(name = "ntdll")]
extern "system" { fn RtlGetVersion(version: *mut OSVERSIONINFOW) -> i32; }

fn supported_build(major: u32, build: u32) -> bool { major >= 10 && build >= 20348 }
fn check_windows() -> Result<(), String> {
    let mut version = OSVERSIONINFOW { dwOSVersionInfoSize: std::mem::size_of::<OSVERSIONINFOW>() as u32, ..Default::default() };
    if unsafe { RtlGetVersion(&mut version) } < 0 || !supported_build(version.dwMajorVersion, version.dwBuildNumber) {
        return Err("Process-loopback capture requires Windows build 20348 or later".into());
    }
    Ok(())
}

struct Ticket;
impl Ticket {
    fn acquire() -> Result<Self, String> {
        ACTIVATIONS.fetch_update(Ordering::AcqRel, Ordering::Acquire,
            |count| (count < MAX_ACTIVATIONS).then_some(count + 1))
            .map(|_| Self).map_err(|_| "An earlier Windows audio activation is still finishing; try later".into())
    }
}
impl Drop for Ticket { fn drop(&mut self) { ACTIVATIONS.fetch_sub(1, Ordering::AcqRel); } }

struct ActivationSignal {
    complete: AtomicBool,
    shared: Arc<Shared>,
    params: PROPVARIANT,
    _ticket: Ticket,
}
// The VT_BLOB contains only immutable POD allocated with CoTaskMemAlloc. It is
// never read or mutated in the callback; the last Arc frees it exactly once.
unsafe impl Send for ActivationSignal {}
unsafe impl Sync for ActivationSignal {}

#[implement(IActivateAudioInterfaceCompletionHandler)]
struct Completion { signal: Arc<ActivationSignal> }
impl IActivateAudioInterfaceCompletionHandler_Impl for Completion_Impl {
    fn ActivateCompleted(&self, _: Option<&IActivateAudioInterfaceAsyncOperation>) -> ::windows::core::Result<()> {
        // windows-implement 0.58 exposes IAgileObject for this thread-safe object.
        // All COM interface retrieval/initialization stays on the owner MTA.
        self.signal.complete.store(true, Ordering::Release);
        self.signal.shared.wake.signal();
        Ok(())
    }
}

fn activation_params(pid: u32) -> Result<PROPVARIANT, String> {
    let value = AUDIOCLIENT_ACTIVATION_PARAMS {
        ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
        Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 { ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
            TargetProcessId: pid, ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
        } },
    };
    let size = std::mem::size_of_val(&value);
    let allocation = unsafe { CoTaskMemAlloc(size) };
    if allocation.is_null() { return Err("Cannot allocate audio activation parameters".into()); }
    unsafe {
        std::ptr::copy_nonoverlapping((&value as *const AUDIOCLIENT_ACTIVATION_PARAMS).cast::<u8>(), allocation.cast::<u8>(), size);
        // PROPVARIANT owns this VT_BLOB; its Drop calls PropVariantClear, matching
        // the CoTaskMem allocator. No borrowed stack pointer escapes activation.
        let mut raw: windows_core::imp::PROPVARIANT = std::mem::zeroed();
        raw.Anonymous.Anonymous.vt = 65; // VT_BLOB
        raw.Anonymous.Anonymous.Anonymous.blob = windows_core::imp::BLOB { cbSize: size as u32, pBlobData: allocation.cast() };
        Ok(PROPVARIANT::from_raw(raw))
    }
}

struct Activation { operation: IActivateAudioInterfaceAsyncOperation, signal: Arc<ActivationSignal>, began: Instant }
struct Stream { client: IAudioClient, capture: IAudioCaptureClient, first_packet: bool, began:Instant, input:SignalWatch }
impl Drop for Stream { fn drop(&mut self) { unsafe { let _ = self.client.Stop(); } } }
struct Packet<'a> { capture: &'a IAudioCaptureClient, frames: u32 }
impl Drop for Packet<'_> { fn drop(&mut self) { unsafe { let _ = self.capture.ReleaseBuffer(self.frames); } } }

#[derive(Default)]
struct SignalWatch { health:InputHealth, last_packet:Option<Duration>, last_signal:Option<Duration> }
impl SignalWatch {
    fn packet(&mut self,elapsed:Duration,samples:&[i16]) {
        if samples.is_empty(){return;}
        self.health.packets=self.health.packets.saturating_add(1);
        self.health.samples=self.health.samples.saturating_add(samples.len() as u64);
        self.health.peak=samples.iter().map(|sample|sample.unsigned_abs()).max().unwrap_or(0);
        self.last_packet=Some(elapsed);
        if self.health.peak>2 {self.last_signal=Some(elapsed);}
    }
    fn snapshot(&self,elapsed:Duration)->InputHealth {
        let mut health=self.health;let limit=Duration::from_secs(12);
        health.signal=if self.last_packet.map_or(elapsed>=limit,|last|elapsed.saturating_sub(last)>=limit) {InputSignal::NoPackets}
            else if self.last_signal.is_some_and(|last|elapsed.saturating_sub(last)<limit) {InputSignal::Receiving}
            else if elapsed>=limit {InputSignal::Silent} else {InputSignal::Waiting};
        health
    }
}

pub(super) struct Capture {
    pub(super) target: AudioTarget,
    pub(super) generation: u64,
    // A held handle plus creation time prevents following a reused PID.
    process: windows::Process,
    activation: Option<Activation>,
    stream: Option<Stream>,
}
impl Capture {
    /// Called only by the metadata worker after a confirmed, still-live session.
    pub(super) fn begin(shared: &Arc<Shared>, target: AudioTarget, generation: u64) -> Result<Self, String> {
        check_windows()?;
        let process = windows::Process::open(target.pid).ok_or("The selected call process is no longer available")?;
        if process.started != target.process_started { return Err("The selected call process has changed".into()); }
        let ticket = Ticket::acquire()?;
        let signal = Arc::new(ActivationSignal { complete: AtomicBool::new(false), shared: Arc::clone(shared),
            params: activation_params(target.pid)?, _ticket: ticket });
        let callback: IActivateAudioInterfaceCompletionHandler = Completion { signal: Arc::clone(&signal) }.into();
        let operation = unsafe { ActivateAudioInterfaceAsync(VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
            &IAudioClient::IID, Some(&signal.params), &callback) }.map_err(|e| format!("Cannot activate process loopback: {e}"))?;
        Ok(Self { target, generation, process, activation: Some(Activation { operation, signal, began: Instant::now() }), stream: None })
    }
    pub(super) fn poll(&mut self, shared: &Arc<Shared>) -> Result<(), String> {
        if shared.wanted_generation.load(Ordering::Acquire) != self.generation { return Ok(()); }
        if !self.process.alive() { return Err("The selected call process has ended".into()); }
        if let Some(pending) = &self.activation {
            if !pending.signal.complete.load(Ordering::Acquire) {
                if pending.began.elapsed() > ACTIVATION_TIMEOUT { return Err("Windows audio activation timed out".into()); }
                return Ok(());
            }
            let mut result = HRESULT(0);
            let mut object: Option<IUnknown> = None;
            unsafe { pending.operation.GetActivateResult(&mut result, &mut object) }
                .map_err(|e| format!("Cannot complete audio activation: {e}"))?;
            result.ok().map_err(|e| format!("Process-loopback activation failed: {e}"))?;
            let client = object.ok_or("Windows returned no audio interface")?.cast::<IAudioClient>()
                .map_err(|_| "Windows returned an incompatible audio interface")?;
            let format = WAVEFORMATEX { wFormatTag: 1, nChannels: 1, nSamplesPerSec: SAMPLE_RATE,
                nAvgBytesPerSec: SAMPLE_RATE * 2, nBlockAlign: 2, wBitsPerSample: 16, cbSize: 0 };
            unsafe {
                client.Initialize(AUDCLNT_SHAREMODE_SHARED,
                    AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                    0, 0, &format, None).map_err(|e| format!("Cannot initialize process-loopback PCM: {e}"))?;
                client.SetEventHandle(shared.wake.handle()).map_err(|e| format!("Cannot register audio-ready event: {e}"))?;
            }
            let capture: IAudioCaptureClient = unsafe { client.GetService() }.map_err(|e| format!("Cannot obtain audio buffers: {e}"))?;
            let stream = Stream { client, capture, first_packet: true, began:Instant::now(), input:SignalWatch::default() };
            if shared.wanted_generation.load(Ordering::Acquire) != self.generation { return Ok(()); }
            unsafe { stream.client.Start() }.map_err(|e| format!("Cannot start process loopback: {e}"))?;
            if shared.wanted_generation.load(Ordering::Acquire) != self.generation || !shared.state().listening(self.generation) {
                return Ok(()); // stream Drop stops it before reading any packet.
            }
            self.stream = Some(stream); self.activation = None;
        }
        let Some(stream) = &mut self.stream else { return Ok(()); };
        for _ in 0..32 {
            if shared.wanted_generation.load(Ordering::Acquire) != self.generation { break; }
            let available = unsafe { stream.capture.GetNextPacketSize() }.map_err(|e| format!("Audio stream unavailable: {e}"))?;
            if available == 0 { break; }
            let (mut data, mut frames, mut flags) = (std::ptr::null_mut(), 0, 0);
            unsafe { stream.capture.GetBuffer(&mut data, &mut frames, &mut flags, None, None) }
                .map_err(|e| format!("Cannot read process-loopback buffer: {e}"))?;
            let _packet = Packet { capture: &stream.capture, frames };
            if frames == 0 { break; }
            if frames as usize > MAX_PACKET_FRAMES { return Err("Audio packet exceeded the bounded capture format".into()); }
            if flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0 && !stream.first_packet {
                return Err("Audio discontinuity: capture stopped; confirm again to resume".into());
            }
            stream.first_packet = false;
            let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
            let samples = unsafe { decode_pcm(data, frames as usize, silent) }?;
            stream.input.packet(stream.began.elapsed(),&samples);
            shared.state().accept_pcm(self.generation, &samples)?;
        }
        let mut state=shared.state();
        if state.generation==self.generation && state.snapshot.capture.as_ref().is_some_and(|capture|capture.state==CapturePhase::Listening) {
            state.snapshot.input_health=stream.input.snapshot(stream.began.elapsed());
        }
        Ok(())
    }
}

/// For non-silent PCM, `data` must cover `frames * 2` readable bytes until return.
unsafe fn decode_pcm(data: *mut u8, frames: usize, silent: bool) -> Result<Vec<i16>, String> {
    if frames > MAX_PACKET_FRAMES { return Err("Audio packet too large".into()); }
    if silent { return Ok(vec![0; frames]); }
    if frames == 0 { return Ok(Vec::new()); }
    if data.is_null() { return Err("Windows returned an empty audio packet".into()); }
    // Native PCM may be unaligned; decode explicit little-endian bytes instead
    // of manufacturing an aligned &[i16]. Buffer validity is scoped by Packet.
    let bytes = unsafe { std::slice::from_raw_parts(data, frames * 2) };
    Ok(bytes.chunks_exact(2).map(|v| i16::from_le_bytes([v[0], v[1]])).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn build_gate_excludes_common_windows10_without_activating_any_audio() {
        assert!(!supported_build(10,19045)); assert!(!supported_build(6,26300));
        assert!(supported_build(10,20348)); assert!(supported_build(10,26300));
    }
    #[test]
    fn bounded_pcm_decodes_silence_and_unaligned_little_endian() {
        assert_eq!(unsafe { decode_pcm(std::ptr::null_mut(),3,true) }.unwrap(),[0,0,0]);
        assert!(unsafe { decode_pcm(std::ptr::null_mut(),1,false) }.is_err());
        assert!(unsafe { decode_pcm(std::ptr::null_mut(),MAX_PACKET_FRAMES+1,true) }.is_err());
        let mut bytes=[99,1,0,255,255,0,128];
        assert_eq!(unsafe {decode_pcm(bytes.as_mut_ptr().add(1),3,false)}.unwrap(),[1,-1,i16::MIN]);
    }
    #[test]
    fn input_watch_distinguishes_no_packets_silence_and_recovery_without_recording() {
        let mut watch=SignalWatch::default();
        assert_eq!(watch.snapshot(Duration::from_secs(1)).signal,InputSignal::Waiting);
        assert_eq!(watch.snapshot(Duration::from_secs(12)).signal,InputSignal::NoPackets);
        watch.packet(Duration::from_secs(12),&[0,0,0]);
        let silent=watch.snapshot(Duration::from_secs(12));
        assert_eq!(silent.signal,InputSignal::Silent);assert_eq!(silent.samples,3);assert!(silent.warning().is_some());
        watch.packet(Duration::from_secs(13),&[0,i16::MIN,4]);
        let receiving=watch.snapshot(Duration::from_secs(13));
        assert_eq!(receiving.signal,InputSignal::Receiving);assert_eq!(receiving.peak,32768);assert_eq!(receiving.packets,2);assert!(receiving.warning().is_none());
        assert_eq!(watch.snapshot(Duration::from_secs(25)).signal,InputSignal::NoPackets);
        watch.packet(Duration::from_secs(25),&[0,0]);
        assert_eq!(watch.snapshot(Duration::from_secs(25)).signal,InputSignal::Silent);
        assert_eq!(watch.snapshot(Duration::from_secs(1)).signal,InputSignal::Receiving,"Saturating elapsed arithmetic cannot wrap");
    }
}
