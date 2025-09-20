use crate::mode::Mode;

#[derive(Debug)]
pub struct Config {
    pub mode: Mode,
    pub rom: Vec<u8>,
    pub headless_mode: bool,
    pub bootrom: Option<Vec<u8>>,
    pub log_file_path: Option<String>,
}
