use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_FILE: Mutex<Option<File>> = Mutex::new(None);
static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init(data_dir: &PathBuf) {
    let _ = std::fs::create_dir_all(data_dir);
    let log_path = data_dir.join("clipslot.log");

    // Rotate: if file is > 2MB, rename to .old and start fresh
    if let Ok(meta) = std::fs::metadata(&log_path) {
        if meta.len() > 2 * 1024 * 1024 {
            let old_path = data_dir.join("clipslot.old.log");
            let _ = std::fs::rename(&log_path, old_path);
        }
    }

    if let Ok(file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    {
        *LOG_FILE.lock().unwrap() = Some(file);
        *LOG_PATH.lock().unwrap() = Some(log_path.clone());
        log(&format!(
            "=== ClipSlot v{} started ===",
            env!("CARGO_PKG_VERSION")
        ));
        log(&format!("OS: {}", std::env::consts::OS));
        log(&format!("Arch: {}", std::env::consts::ARCH));
        log(&format!("Log file: {}", log_path.display()));
    }
}

pub fn log(msg: &str) {
    let timestamp = chrono::Local::now().format("%H:%M:%S%.3f");
    let line = format!("[{}] {}", timestamp, msg);
    eprintln!("{}", line);
    if let Ok(mut guard) = LOG_FILE.lock() {
        if let Some(ref mut file) = *guard {
            let _ = writeln!(file, "{}", line);
            let _ = file.flush();
        }
    }
}

pub fn log_path() -> Option<String> {
    LOG_PATH
        .lock()
        .ok()
        .and_then(|g| g.as_ref().map(|p| p.display().to_string()))
}

/// Convenience macro for logging with format args.
#[macro_export]
macro_rules! clog {
    ($($arg:tt)*) => {
        $crate::logging::log(&format!($($arg)*))
    };
}
