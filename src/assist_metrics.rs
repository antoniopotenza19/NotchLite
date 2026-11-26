//! Bounded process metrics, sampled only while the dashboard is enabled.
//! These are process totals; shared Codex processes cannot be charged to a chat.
use std::collections::BTreeMap;
use std::mem::size_of;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows::Win32::Foundation::{CloseHandle, FILETIME, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::*;
use windows::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS, PROCESS_MEMORY_COUNTERS_EX};
use windows::Win32::System::Threading::{GetCurrentProcess, GetCurrentProcessId, GetProcessTimes, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ};

const MAX_CODEX_PROCESSES: usize = 256;
const INTERVAL: Duration = Duration::from_secs(2);

#[derive(Clone, Debug, Default)]
pub struct Usage {
    pub working_set: u64,
    pub private_bytes: u64,
    pub cpu_percent: Option<f64>,
    pub processes: usize,
    pub unavailable: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ResourceSnapshot {
    pub notch: Option<Usage>,
    pub codex: Option<Usage>,
    pub error: Option<String>,
}

impl ResourceSnapshot {
    pub fn display(&self) -> String {
        fn row(name: &str, usage: &Option<Usage>) -> String {
            match usage {
                Some(u) => format!("{name}: WS {:.0} MiB / privati {:.0} MiB · CPU {}{}",
                    u.working_set as f64 / 1_048_576.0, u.private_bytes as f64 / 1_048_576.0,
                    u.cpu_percent.map(|p| format!("{p:.1}%")).unwrap_or_else(|| "…".into()),
                    if u.unavailable > 0 { " (parziale)" } else { "" }),
                None => format!("{name}: non disponibile"),
            }
        }
        let name = self.codex.as_ref().map(|u| format!("Codex, {} processi", u.processes)).unwrap_or_else(|| "Codex".into());
        format!("{}\r\n{}", row("NotchLite", &self.notch), row(&name, &self.codex))
    }
}

pub struct MetricsWorker {
    stop: SyncSender<()>,
    results: Receiver<ResourceSnapshot>,
    worker: Option<JoinHandle<()>>,
}

impl MetricsWorker {
    pub fn start() -> Result<Self, String> {
        let (stop, stopping) = mpsc::sync_channel(1);
        let (send, results) = mpsc::sync_channel(1);
        let worker = thread::Builder::new().name("notch-metrics".into()).spawn(move || {
            let mut sampler = Sampler::default();
            loop {
                if stopping.try_recv().is_ok() { break; }
                let snapshot = sampler.sample();
                let _ = send.try_send(snapshot);
                match stopping.recv_timeout(INTERVAL) {
                    Err(mpsc::RecvTimeoutError::Timeout) => {},
                    _ => break,
                }
            }
        }).map_err(|_| "Impossibile avviare il campionamento risorse".to_string())?;
        Ok(Self { stop, results, worker: Some(worker) })
    }

    pub fn poll(&self) -> Option<ResourceSnapshot> { self.results.try_iter().last() }
}

impl Drop for MetricsWorker {
    fn drop(&mut self) {
        let _ = self.stop.try_send(());
        if let Some(worker) = self.worker.take() {
            // Process metadata calls can be delayed by the OS. Never freeze the UI.
            let deadline = Instant::now() + Duration::from_millis(200);
            while !worker.is_finished() && Instant::now() < deadline { thread::sleep(Duration::from_millis(5)); }
            if worker.is_finished() { let _ = worker.join(); }
        }
    }
}

struct Handle(HANDLE);
impl Drop for Handle { fn drop(&mut self) { if !self.0.is_invalid() { unsafe { let _ = CloseHandle(self.0); } } } }

fn filetime(value: FILETIME) -> u64 { u64::from(value.dwLowDateTime) | (u64::from(value.dwHighDateTime) << 32) }

fn normalized_cpu(previous: u64, current: u64, elapsed: Duration, processors: usize) -> Option<f64> {
    if current < previous || elapsed.is_zero() || processors == 0 { return None; }
    let seconds = (current - previous) as f64 / 10_000_000.0;
    Some((seconds / elapsed.as_secs_f64() / processors as f64 * 100.0).clamp(0.0, 100.0))
}

#[derive(Default)]
struct Sampler { previous: BTreeMap<(u32, u64), (u64, Instant)> }

impl Sampler {
    fn process(&mut self, pid: u32, process: HANDLE, now: Instant, processors: usize) -> Option<Usage> {
        let mut memory = PROCESS_MEMORY_COUNTERS_EX::default();
        memory.cb = size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        let mut creation = FILETIME::default();
        let mut exit = FILETIME::default();
        let mut kernel = FILETIME::default();
        let mut user = FILETIME::default();
        unsafe {
            GetProcessMemoryInfo(process, (&mut memory as *mut PROCESS_MEMORY_COUNTERS_EX).cast::<PROCESS_MEMORY_COUNTERS>(), memory.cb).ok()?;
            GetProcessTimes(process, &mut creation, &mut exit, &mut kernel, &mut user).ok()?;
        }
        let total = filetime(kernel).saturating_add(filetime(user));
        let key = (pid, filetime(creation));
        let cpu = self.previous.get(&key).and_then(|&(old, at)| normalized_cpu(old, total, now.saturating_duration_since(at), processors));
        self.previous.insert(key, (total, now));
        Some(Usage { working_set: memory.WorkingSetSize as u64, private_bytes: memory.PrivateUsage as u64, cpu_percent: cpu, processes: 1, unavailable: 0 })
    }

    fn sample(&mut self) -> ResourceSnapshot {
        let now = Instant::now();
        let processors = thread::available_parallelism().map(usize::from).unwrap_or(1);
        let own_pid = unsafe { GetCurrentProcessId() };
        let notch = self.process(own_pid, unsafe { GetCurrentProcess() }, now, processors);
        let mut result = ResourceSnapshot { notch, ..Default::default() };
        let snapshot = match unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) } {
            Ok(handle) => Handle(handle),
            Err(_) => { result.error = Some("Elenco processi non disponibile".into()); return result; }
        };
        let mut entry = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
        let mut next = unsafe { Process32FirstW(snapshot.0, &mut entry) }.is_ok();
        let mut total = Usage::default();
        let mut warm = true;
        let mut any = false;
        let mut count = 0;
        while next {
            let end = entry.szExeFile.iter().position(|&c| c == 0).unwrap_or(entry.szExeFile.len());
            let name = String::from_utf16_lossy(&entry.szExeFile[..end]);
            if name.eq_ignore_ascii_case("codex.exe") {
                count += 1;
                if count > MAX_CODEX_PROCESSES { total.unavailable += 1; break; }
                match unsafe { OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, entry.th32ProcessID) } {
                    Ok(handle) => {
                        let process = Handle(handle);
                        if let Some(usage) = self.process(entry.th32ProcessID, process.0, now, processors) {
                            total.working_set = total.working_set.saturating_add(usage.working_set);
                            total.private_bytes = total.private_bytes.saturating_add(usage.private_bytes);
                            total.cpu_percent = Some(total.cpu_percent.unwrap_or(0.0) + usage.cpu_percent.unwrap_or(0.0));
                            warm &= usage.cpu_percent.is_some();
                            total.processes += 1; any = true;
                        } else { total.unavailable += 1; }
                    }
                    Err(_) => { total.unavailable += 1; }
                }
            }
            next = unsafe { Process32NextW(snapshot.0, &mut entry) }.is_ok();
        }
        if !warm { total.cpu_percent = None; }
        if any { result.codex = Some(total); }
        self.previous.retain(|_, (_, at)| *at == now);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cpu_is_normalized_by_elapsed_time_and_processor_count() {
        assert_eq!(normalized_cpu(0, 10_000_000, Duration::from_secs(2), 4), Some(12.5));
        assert_eq!(normalized_cpu(7, 7, Duration::from_secs(2), 8), Some(0.0));
        assert_eq!(normalized_cpu(0, 500_000_000, Duration::from_secs(1), 1), Some(100.0));
    }
    #[test]
    fn cpu_does_not_invent_a_value_for_resets_or_missing_intervals() {
        assert_eq!(normalized_cpu(100, 90, Duration::from_secs(1), 8), None);
        assert_eq!(normalized_cpu(0, 100, Duration::ZERO, 8), None);
        assert_eq!(normalized_cpu(0, 100, Duration::from_secs(1), 0), None);
    }
    #[test]
    fn resource_labels_are_process_totals_and_keep_unknowns_visible() {
        let snapshot = ResourceSnapshot::default();
        assert!(snapshot.display().contains("non disponibile"));
        let snapshot = ResourceSnapshot { codex: Some(Usage { processes: 2, private_bytes: 1048576, working_set: 2097152, ..Default::default() }), ..Default::default() };
        assert!(snapshot.display().contains("Codex, 2 processi"));
        assert!(snapshot.display().contains("privati 1 MiB"));
        assert!(snapshot.display().contains("CPU …"));
    }
}
