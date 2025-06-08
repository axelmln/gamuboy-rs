use chrono::Utc;
use std::sync::Once;

pub static mut LOG_PATH: Option<String> = None;
static INIT: Once = Once::new();

pub fn init_logger(path: Option<String>) {
    unsafe {
        INIT.call_once(|| {
            LOG_PATH = path;
        });
    }
}

pub fn get_timestamp() -> String {
    let now = Utc::now();
    let formatted_time = now.format("%Y-%m-%d %H:%M:%S%.3f").to_string();

    formatted_time
}

#[macro_export]
macro_rules! log {
    ($level:expr, $($arg:tt)*) => {
        {
            use std::fs::OpenOptions;
            use std::io::Write;

            let timestamp = $crate::logger::get_timestamp();
            let log_line = format!("[{}] [{}:{}] [{}] {}\n", timestamp, file!(), line!(), $level, &format!($($arg)*));

            unsafe {
                if let Some(ref file_path) = $crate::logger::LOG_PATH{
                    let mut file = OpenOptions::new()
                        .append(true)
                        .create(true)
                        .open(file_path)
                        .unwrap();
                    file.write_all(log_line.as_bytes()).unwrap();
                } else {
                    print!("{}", log_line);
                }
            }
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        log!("INFO", $($arg)*);
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        log!("WARN", $($arg)*);
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        log!("ERROR", $($arg)*);
    };
}
