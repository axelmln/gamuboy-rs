use std::{
    io::{self, Write},
    str::Utf8Error,
};

use crc::{Crc, CRC_32_ISO_HDLC};

use crate::{config::Config, mbc, memory::MemReadWriter, mode::Mode, saver::GameSave};

const ROM_CHECKSUM_ADDRESS: usize = 0x014D;
const CARTRIDGE_TYPE_ADDRESS: usize = 0x0147;

fn compute_rom_checksum(rom: &Vec<u8>) -> u8 {
    let mut checksum: u8 = 0;
    for addr in 0x0134..=0x014C {
        checksum = checksum.wrapping_sub(rom[addr]).wrapping_sub(1);
    }
    checksum
}

fn validate_rom_checksum(rom: &Vec<u8>, checksum: u8) -> bool {
    rom[ROM_CHECKSUM_ADDRESS] == checksum
}

fn bytes_to_string(bytes: &[u8]) -> Result<String, Utf8Error> {
    Ok((std::str::from_utf8(bytes)?).to_string())
}

fn checksum_identifier(rom: &[u8]) -> u32 {
    const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_ISO_HDLC);
    CRC32.checksum(rom)
}

#[allow(dead_code)]
struct Header {
    title: String,
    new_licensee_code: String,
    old_licensee_code: u8,
    rom_size: u8,
    ram_size: u8,
    destination_code: u8,
    rom_version: u8,
}

pub struct Cartridge {
    mode: Mode,
    bootrom_enabled: bool,
    bootrom: Option<Vec<u8>>,
    #[allow(dead_code)]
    header: Header,
    mbc: mbc::MBC,
}

impl Cartridge {
    pub fn new<S: GameSave + 'static>(cfg: &Config, mut saver: S) -> Self {
        let rom = &cfg.rom;

        let rom_checksum = compute_rom_checksum(rom);
        if !validate_rom_checksum(rom, rom_checksum) {
            _ = io::stderr().write(
                format!(
                    "WARNING: game rom checksum mismatch! computed checksum: {}; rom checksum: {}\n",
                    rom_checksum & 0xFF,
                    rom[ROM_CHECKSUM_ADDRESS],
                )
                .as_bytes(),
            );
        }

        let header = Header {
            title: bytes_to_string(&rom[0x0134..=0x0143]).unwrap_or("ERROR PARSING TITLE".into()),
            new_licensee_code: bytes_to_string(&rom[0x0144..=0x0145])
                .unwrap_or("ERROR PARSING NEW LICENSEE CODE".into()),
            old_licensee_code: rom[0x014B],
            rom_size: rom[0x0148],
            ram_size: rom[0x0149],
            destination_code: rom[0x014A],
            rom_version: rom[0x014C],
        };

        let title = header.title.clone().trim_matches('\0').to_owned();
        saver.set_title(format!("{title}-{:08x}", checksum_identifier(rom)));

        let ram_size = match header.ram_size {
            0x00 | 0x01 => 0,
            0x02 => 8 * 1024,
            0x03 => 32 * 1024,
            0x04 => 128 * 1024,
            0x05 => 64 * 1024,
            _ => unreachable!(),
        };

        Self {
            mode: cfg.mode.clone(),
            bootrom_enabled: cfg.bootrom.is_some(),
            bootrom: cfg.bootrom.clone(),
            mbc: mbc::MBC::new(rom[CARTRIDGE_TYPE_ADDRESS], rom.clone(), ram_size, saver),
            header,
        }
    }
}

impl MemReadWriter for Cartridge {
    fn read_byte(&self, address: u16) -> u8 {
        match self.mode {
            Mode::DMG => {
                if self.bootrom_enabled && self.bootrom.is_some() && address <= 0x00FF {
                    return self.bootrom.as_ref().unwrap()[address as usize];
                }
            }
            Mode::CGB => {
                if self.bootrom_enabled && self.bootrom.is_some() {
                    match address {
                        ..=0x00FF => {
                            return self.bootrom.as_ref().unwrap()[address as usize];
                        }
                        0x0200..=0x08FF => {
                            return self.bootrom.as_ref().unwrap()[address as usize];
                        }
                        _ => {}
                    }
                }
            }
        }

        self.mbc.read_byte(address)
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match self.mode {
            Mode::DMG => {
                if self.bootrom_enabled && self.bootrom.is_some() && address <= 0x00FF {
                    panic!(
                        "writing to bootrom: address: {:#04x}, value: {:#04x}",
                        address, value
                    );
                }
            }
            Mode::CGB => {
                if self.bootrom_enabled
                    && self.bootrom.is_some()
                    && (address <= 0x00FF || (address >= 0x0200 && address <= 0x08FF))
                {
                    panic!(
                        "writing to bootrom: address: {:#04x}, value: {:#04x}",
                        address, value
                    );
                }
            }
        }

        if address == 0xFF50 {
            self.bootrom_enabled = false;
        } else {
            self.mbc.write_byte(address, value);
        }
    }
}
