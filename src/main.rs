// Modified for NotchLite: isolated identity, opt-in profile and explicit panel opening.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod anim;
mod assist;
mod assist_layout;
mod assist_metrics;
mod assist_overlay;
mod call_audio;
mod call_ai;
mod call_stt;
mod call_session;
mod call_coach;
mod call_panel_size;
mod codex_bridge;
mod codex_server;
mod codex_diagnostic;
mod desktop_clock;
mod droptarget;
mod gfx;
mod http;
mod mascot;
mod navigation;
mod notch_monitor;
mod pages;
mod phone;
mod phone_qr;
mod profile;
mod shell;
mod store;
mod theme;
mod worker;
mod window_policy;

use windows::core::w;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Ole::OleInitialize;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if window_policy::run_diagnostic()? { return Ok(()); }
    if call_ai::run_diagnostic()? { return Ok(()); }
    if call_coach::run_diagnostic()? { return Ok(()); }
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.first().is_some_and(|a| a == "--call-transcription-check") {
        if args.len() < 2 || args.len() > 3 { return Err("Usa --call-transcription-check <wav PCM16 mono 16kHz, max 8s> [lingua]".into()); }
        let language = args.get(2).map(|v| v.to_string_lossy().into_owned()).unwrap_or_else(|| "it".into());
        let result = call_stt::check_file(std::path::Path::new(&args[1]), &language)?;
        println!("{}", serde_json::json!({"text":result.text,"samples":result.samples,"elapsedMillis":result.elapsed_millis,"profile":result.profile,"gpuVerified":result.gpu_verified,"audioCapture":false,"guiOpened":false}));
        return Ok(());
    }
    if codex_diagnostic::run()? { return Ok(()); }
    if std::env::args_os().skip(1).any(|arg| arg == "--codex-check") {
        // Metadata-only diagnostics before settings, COM, mutex or window creation.
        match codex_bridge::check_local(None) {
            Ok(path) => println!("Codex shared endpoint available: {}", path.display()),
            Err(reason) => println!("Codex dashboard unavailable: {reason}"),
        }
        return Ok(());
    }
    let open_panel = !std::env::args_os().skip(1).any(|arg| arg == "--background");
    unsafe {
        let _mutex = CreateMutexW(None, true, w!("Local\\NotchLite.SingleInstance"))?;
        if GetLastError() == ERROR_ALREADY_EXISTS {
            if open_panel {
                shell::request_open_existing()?;
            }
            return Ok(());
        }
        profile::initialize_at(&store::data_dir().join("settings.txt"))?;
        // OLE (STA) on the UI thread: needed for drag-and-drop with the shelf.
        OleInitialize(None)?;
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    shell::run(open_panel)?;
    Ok(())
}
