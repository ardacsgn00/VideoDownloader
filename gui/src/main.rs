#![windows_subsystem = "windows"]

use std::io::BufRead;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use serde::Deserialize;
use tao::{
    dpi::{LogicalSize, PhysicalPosition},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

const CREATE_NO_WINDOW: u32 = 0x08000000;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "get_init_info")]
    GetInitInfo,
    #[serde(rename = "paste_clipboard")]
    PasteClipboard,
    #[serde(rename = "select_folder")]
    SelectFolder,
    #[serde(rename = "open_folder")]
    OpenFolder { path: String },
    #[serde(rename = "update_system")]
    UpdateSystem,
    #[serde(rename = "start_download")]
    StartDownload {
        url: String,
        mode: u32,
        target_dir: String,
    },
    #[serde(rename = "cancel_download")]
    CancelDownload,
}

enum AppEvent {
    Js(serde_json::Value),
}

struct AppState {
    current_pid: Arc<Mutex<Option<u32>>>,
    is_cancelled: Arc<AtomicBool>,
}

#[cfg(windows)]
fn read_clipboard() -> Option<String> {
    use std::ptr::null_mut;
    use windows_sys::Win32::System::DataExchange::{
        CloseClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    };
    use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

    const CF_UNICODETEXT: u32 = 13;

    unsafe {
        if OpenClipboard(null_mut() as _) == 0 {
            return None;
        }
        if IsClipboardFormatAvailable(CF_UNICODETEXT) == 0 {
            CloseClipboard();
            return None;
        }
        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            CloseClipboard();
            return None;
        }
        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(ptr, len);
        let s = String::from_utf16_lossy(slice);
        GlobalUnlock(handle);
        CloseClipboard();
        Some(s)
    }
}

fn get_default_download_dir() -> String {
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let desktop = PathBuf::from(&userprofile).join("Desktop");
        if desktop.exists() {
            return desktop.to_string_lossy().to_string();
        }
    }
    std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| "C:\\".to_string())
}

fn find_ytdlp_path() -> PathBuf {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            let p = parent.join("yt-dlp.exe");
            if p.exists() {
                return p;
            }
        }
    }
    let standard = PathBuf::from("C:\\yt-dlp\\yt-dlp.exe");
    if standard.exists() {
        return standard;
    }
    PathBuf::from("yt-dlp")
}

fn find_ffmpeg_dir() -> String {
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            if parent.join("ffmpeg.exe").exists() {
                return parent.to_string_lossy().to_string();
            }
        }
    }
    let standard = PathBuf::from("C:\\yt-dlp\\ffmpeg.exe");
    if standard.exists() {
        return "C:\\yt-dlp".to_string();
    }
    "".to_string()
}

fn parse_progress(line: &str) -> Option<(f32, String, String, String)> {
    if !line.contains("[download]") || !line.contains('%') {
        return None;
    }

    let percent_idx = line.find('%')?;
    let start_idx = line[..percent_idx]
        .rfind(|c: char| c.is_whitespace() || c == ']')?
        + 1;
    let pct_str = line[start_idx..percent_idx].trim();
    let percent = pct_str.parse::<f32>().ok()?;

    let total_size = if let Some(of_idx) = line.find("of ") {
        let rest = &line[of_idx + 3..];
        if let Some(at_idx) = rest.find(" at ") {
            rest[..at_idx].trim().to_string()
        } else if let Some(in_idx) = rest.find(" in ") {
            rest[..in_idx].trim().to_string()
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    };

    let speed = if let Some(at_idx) = line.find("at ") {
        let rest = &line[at_idx + 3..];
        if let Some(eta_idx) = rest.find(" ETA") {
            rest[..eta_idx].trim().to_string()
        } else {
            rest.split_whitespace().next().unwrap_or("").to_string()
        }
    } else {
        "".to_string()
    };

    let eta = if let Some(eta_idx) = line.find("ETA ") {
        line[eta_idx + 4..]
            .split_whitespace()
            .next()
            .unwrap_or("")
            .to_string()
    } else {
        "".to_string()
    };

    Some((percent, total_size, speed, eta))
}

fn handle_message(msg: ClientMessage, proxy: &EventLoopProxy<AppEvent>, state: &Arc<AppState>) {
    match msg {
        ClientMessage::GetInitInfo => {
            let default_dir = get_default_download_dir();
            let _ = proxy.send_event(AppEvent::Js(serde_json::json!({
                "type": "init_info",
                "target_dir": default_dir
            })));
        }

        ClientMessage::PasteClipboard => {
            let text = read_clipboard().unwrap_or_default();
            let _ = proxy.send_event(AppEvent::Js(serde_json::json!({
                "type": "pasted_text",
                "text": text
            })));
        }

        ClientMessage::SelectFolder => {
            let current = get_default_download_dir();
            let proxy_clone = proxy.clone();
            thread::spawn(move || {
                let folder = rfd::FileDialog::new()
                    .set_directory(&current)
                    .pick_folder();
                if let Some(path) = folder {
                    let path_str = path.to_string_lossy().to_string();
                    let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                        "type": "folder_selected",
                        "path": path_str
                    })));
                }
            });
        }

        ClientMessage::OpenFolder { path } => {
            let folder_to_open = if path.trim().is_empty() {
                get_default_download_dir()
            } else {
                path
            };
            let _ = std::process::Command::new("explorer")
                .arg(&folder_to_open)
                .spawn();
        }

        ClientMessage::CancelDownload => {
            state.is_cancelled.store(true, Ordering::SeqCst);
            if let Some(pid) = state.current_pid.lock().unwrap().take() {
                let _ = std::process::Command::new("taskkill")
                    .args(&["/F", "/T", "/PID", &pid.to_string()])
                    .creation_flags(CREATE_NO_WINDOW)
                    .status();
            }
        }

        ClientMessage::UpdateSystem => {
            let proxy_clone = proxy.clone();
            thread::spawn(move || {
                let ytdlp_path = find_ytdlp_path();
                let mut cmd = std::process::Command::new(&ytdlp_path);
                cmd.arg("-U")
                    .arg("--newline")
                    .creation_flags(CREATE_NO_WINDOW)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                match cmd.spawn() {
                    Ok(mut child) => {
                        if let Some(out) = child.stdout.take() {
                            let reader = std::io::BufReader::new(out);
                            for line in reader.lines().flatten() {
                                let l = line.trim().to_string();
                                if !l.is_empty() {
                                    let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                        "type": "log",
                                        "text": l
                                    })));
                                }
                            }
                        }
                        let res = child.wait();
                        let success = res.map(|s| s.success()).unwrap_or(false);
                        let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                            "type": "update_result",
                            "success": success
                        })));
                    }
                    Err(e) => {
                        let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                            "type": "update_result",
                            "success": false,
                            "message": e.to_string()
                        })));
                    }
                }
            });
        }

        ClientMessage::StartDownload {
            url,
            mode,
            target_dir,
        } => {
            let target_path = if target_dir.trim().is_empty() {
                get_default_download_dir()
            } else {
                target_dir
            };

            let state_clone = Arc::clone(state);
            let proxy_clone = proxy.clone();

            thread::spawn(move || {
                state_clone.is_cancelled.store(false, Ordering::SeqCst);

                let ytdlp_path = find_ytdlp_path();
                let ffmpeg_dir = find_ffmpeg_dir();

                let mut cmd = std::process::Command::new(&ytdlp_path);
                cmd.creation_flags(CREATE_NO_WINDOW)
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped());

                cmd.arg("-P").arg(&target_path);

                if !ffmpeg_dir.is_empty() {
                    cmd.arg("--ffmpeg-location").arg(&ffmpeg_dir);
                }

                cmd.arg("--newline");

                match mode {
                    1 => {
                        // En Yuksek Kalite Video
                        cmd.arg("-S")
                            .arg("vcodec:h264,res,acodec:aac")
                            .arg("--merge-output-format")
                            .arg("mp4")
                            .arg("-o")
                            .arg("%(title)s.%(ext)s");
                    }
                    2 => {
                        // Sadece Ses (MP3)
                        cmd.arg("-x")
                            .arg("--audio-format")
                            .arg("mp3")
                            .arg("-o")
                            .arg("%(title)s.%(ext)s");
                    }
                    3 => {
                        // 1080p Video (Boyut Tasarrufu)
                        cmd.arg("-S")
                            .arg("vcodec:h264,res:1080,acodec:aac")
                            .arg("--merge-output-format")
                            .arg("mp4")
                            .arg("-o")
                            .arg("%(title)s.%(ext)s");
                    }
                    _ => {
                        cmd.arg("-S")
                            .arg("vcodec:h264,res,acodec:aac")
                            .arg("--merge-output-format")
                            .arg("mp4")
                            .arg("-o")
                            .arg("%(title)s.%(ext)s");
                    }
                }

                cmd.arg(&url);

                match cmd.spawn() {
                    Ok(mut child) => {
                        *state_clone.current_pid.lock().unwrap() = Some(child.id());

                        let stdout = child.stdout.take();
                        let stderr = child.stderr.take();

                        // Handle stderr in a thread
                        if let Some(err) = stderr {
                            let proxy_err = proxy_clone.clone();
                            thread::spawn(move || {
                                let reader = std::io::BufReader::new(err);
                                for line in reader.lines().flatten() {
                                    let l = line.trim().to_string();
                                    if !l.is_empty() {
                                        let _ = proxy_err.send_event(AppEvent::Js(serde_json::json!({
                                            "type": "log",
                                            "text": l
                                        })));
                                    }
                                }
                            });
                        }

                        // Handle stdout line by line
                        if let Some(out) = stdout {
                            let reader = std::io::BufReader::new(out);
                            for line in reader.lines().flatten() {
                                let l = line.trim().to_string();
                                if l.is_empty() {
                                    continue;
                                }

                                if let Some((pct, total_size, speed, eta)) = parse_progress(&l) {
                                    let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                        "type": "progress",
                                        "percent": pct,
                                        "size": total_size,
                                        "speed": speed,
                                        "eta": eta
                                    })));
                                } else if l.contains("Destination: ") {
                                    if let Some(dest) = l.split("Destination: ").nth(1) {
                                        let file_name = Path::new(dest.trim())
                                            .file_name()
                                            .map(|f| f.to_string_lossy().to_string())
                                            .unwrap_or_else(|| dest.trim().to_string());
                                        let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                            "type": "progress",
                                            "file": file_name
                                        })));
                                    }
                                } else if l.contains("Merging formats into ") {
                                    if let Some(dest) = l.split("Merging formats into ").nth(1) {
                                        let file_name = Path::new(dest.trim().trim_matches('"'))
                                            .file_name()
                                            .map(|f| f.to_string_lossy().to_string())
                                            .unwrap_or_else(|| dest.trim().to_string());
                                        let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                            "type": "progress",
                                            "file": file_name
                                        })));
                                    }
                                }

                                let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                    "type": "log",
                                    "text": l
                                })));
                            }
                        }

                        let wait_res = child.wait();
                        *state_clone.current_pid.lock().unwrap() = None;

                        if state_clone.is_cancelled.load(Ordering::SeqCst) {
                            let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                "type": "status",
                                "status": "cancelled"
                            })));
                        } else {
                            match wait_res {
                                Ok(status) if status.success() => {
                                    let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                        "type": "status",
                                        "status": "completed"
                                    })));
                                }
                                Ok(status) => {
                                    let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                        "type": "status",
                                        "status": "error",
                                        "message": format!("Hata kodu: {}", status)
                                    })));
                                }
                                Err(e) => {
                                    let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                                        "type": "status",
                                        "status": "error",
                                        "message": e.to_string()
                                    })));
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = proxy_clone.send_event(AppEvent::Js(serde_json::json!({
                            "type": "status",
                            "status": "error",
                            "message": format!("yt-dlp calistirilamadi: {}", e)
                        })));
                    }
                }
            });
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("Video Indirici")
        .with_inner_size(LogicalSize::new(760.0, 540.0))
        .with_min_inner_size(LogicalSize::new(600.0, 420.0))
        .build(&event_loop)?;

    // Ekranın ortasına konumlandır
    if let Some(monitor) = window.current_monitor().or_else(|| event_loop.primary_monitor()) {
        let screen_size = monitor.size();
        let window_size = window.outer_size();
        let x = screen_size.width.saturating_sub(window_size.width) / 2;
        let y = screen_size.height.saturating_sub(window_size.height) / 2;
        window.set_outer_position(PhysicalPosition::new(x as i32, y as i32));
    }

    let app_state = Arc::new(AppState {
        current_pid: Arc::new(Mutex::new(None)),
        is_cancelled: Arc::new(AtomicBool::new(false)),
    });

    let proxy_ipc = proxy.clone();
    let state_ipc = Arc::clone(&app_state);

    let html_content = include_str!("ui.html");

    let webview = WebViewBuilder::new()
        .with_clipboard(true)
        .with_ipc_handler(move |req| {
            if let Ok(msg) = serde_json::from_str::<ClientMessage>(req.body()) {
                handle_message(msg, &proxy_ipc, &state_ipc);
            }
        })
        .with_html(html_content)
        .build(&window)?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(AppEvent::Js(val)) => {
                let json_str = val.to_string();
                let script = format!("window.onAppEvent({});", json_str);
                let _ = webview.evaluate_script(&script);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                // If a download is running, kill it on close
                if let Some(pid) = app_state.current_pid.lock().unwrap().take() {
                    let _ = std::process::Command::new("taskkill")
                        .args(&["/F", "/T", "/PID", &pid.to_string()])
                        .creation_flags(CREATE_NO_WINDOW)
                        .status();
                }
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}
