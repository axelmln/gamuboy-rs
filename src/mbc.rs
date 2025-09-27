use crate::{memory::MemReadWriter, saver::GameSave};

fn right_nibble(byte: u8) -> u8 {
    byte & 0x0F
}

struct NoMBC {
    rom: Vec<u8>,
    ram: [u8; 0xC000],
}

impl NoMBC {
    fn new(rom: Vec<u8>) -> Self {
        Self {
            rom,
            ram: [0; 0xC000],
        }
    }
}

impl MemReadWriter for NoMBC {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            ..=0x7FFF => self.rom[address as usize],
            _ => self.ram[address as usize],
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            ..=0x7FFF => {}
            _ => self.ram[address as usize] = value,
        }
    }
}

const MBC1_BANKING_MODE_REG_START_ADDR: u16 = 0x6000;
const MBC1_BANKING_MODE_REG_END_ADDR: u16 = 0x7FFF;

const MBC1_ROM_BANK_NUM_REG_START_ADDR: u16 = 0x2000;
const MBC1_ROM_BANK_NUM_REG_END_ADDR: u16 = 0x3FFF;

const MBC1_RAM_BANK_NUM_REG_START_ADDR: u16 = 0x4000;
const MBC1_RAM_BANK_NUM_REG_END_ADDR: u16 = 0x5FFF;

const MBC1_ROM_BANK_0_START_ADDR: u16 = 0x0000;
const MBC1_ROM_BANK_0_END_ADDR: u16 = 0x3FFF;

const MBC1_ROM_BANK_01_7F_START_ADDR: u16 = 0x4000;
const MBC1_ROM_BANK_01_7F_END_ADDR: u16 = 0x7FFF;

const MBC1_RAM_START_ADDR: u16 = 0xA000;
const MBC1_RAM_END_ADDR: u16 = 0xBFFF;

#[derive(Debug)]
enum BankingMode {
    /// Rom mode
    Simple,
    /// Ram mode
    Advanced,
}

impl From<u8> for BankingMode {
    fn from(value: u8) -> Self {
        match value & 1 {
            0x00 => Self::Simple,
            _ => Self::Advanced,
        }
    }
}

fn load_saved_ram<S: GameSave>(saver: &S, ram_size: usize) -> Vec<u8> {
    let mut saved_ram = saver.load().unwrap_or(vec![0; ram_size]);
    if saved_ram.len() != ram_size {
        warn!("Mismatching ram size and saved ram size.");
        warn!("Skipping saved ram.");
        saved_ram = vec![0; ram_size];
    }

    saved_ram
}

struct MBC1<S: GameSave> {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_lower: u8,
    ram_or_upper_rom_bank: u8,
    ram_enabled: bool,
    banking_mode: BankingMode,
    saver: S,
}

impl<S: GameSave> MBC1<S> {
    fn new(rom: Vec<u8>, ram_size: usize, saver: S) -> Self {
        Self {
            rom,
            ram: load_saved_ram(&saver, ram_size),
            rom_bank_lower: 1,
            ram_or_upper_rom_bank: 0,
            ram_enabled: false,
            banking_mode: BankingMode::Simple,
            saver,
        }
    }

    fn get_rom_address(&self, address: u16) -> usize {
        match address {
            MBC1_ROM_BANK_0_START_ADDR..=MBC1_ROM_BANK_0_END_ADDR => match self.banking_mode {
                BankingMode::Simple => address as usize,
                BankingMode::Advanced => {
                    (self.ram_or_upper_rom_bank << 5) as usize * 0x4000 + address as usize
                }
            },
            MBC1_ROM_BANK_01_7F_START_ADDR..=MBC1_ROM_BANK_01_7F_END_ADDR => {
                let rom_bank = self.ram_or_upper_rom_bank << 5 | self.rom_bank_lower;
                (address - MBC1_ROM_BANK_01_7F_START_ADDR) as usize + (rom_bank as usize) * 0x4000
            }
            _ => unreachable!(),
        }
    }

    fn get_ram_address(&self, address: u16) -> usize {
        match self.banking_mode {
            BankingMode::Simple => (address - MBC1_RAM_START_ADDR) as usize,
            BankingMode::Advanced => {
                (address - MBC1_RAM_START_ADDR) as usize
                    + self.ram_or_upper_rom_bank as usize * 0x2000
            }
        }
    }
}

impl<S: GameSave> MemReadWriter for MBC1<S> {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            MBC1_ROM_BANK_0_START_ADDR..=MBC1_ROM_BANK_0_END_ADDR
            | MBC1_ROM_BANK_01_7F_START_ADDR..=MBC1_ROM_BANK_01_7F_END_ADDR => {
                let addr = self.get_rom_address(address) & (self.rom.len() - 1);
                self.rom[addr]
            }
            MBC1_RAM_START_ADDR..=MBC1_RAM_END_ADDR => {
                let mut val = 0xFF;
                if self.ram_enabled && self.ram.len() > 0 {
                    let addr = self.get_ram_address(address) & (self.ram.len() - 1);
                    val = self.ram[addr];
                }

                val
            }
            _ => unreachable!("invalid read address for MBC1: {:#04x}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x1FFF => {
                let enabled = right_nibble(value) == 0xA;
                if self.ram_enabled && !enabled {
                    self.saver.save(&self.ram).unwrap();
                }
                self.ram_enabled = enabled;
            }
            MBC1_ROM_BANK_NUM_REG_START_ADDR..=MBC1_ROM_BANK_NUM_REG_END_ADDR => {
                self.rom_bank_lower = (value & 0b11111).max(1);
            }
            MBC1_RAM_BANK_NUM_REG_START_ADDR..=MBC1_RAM_BANK_NUM_REG_END_ADDR => {
                self.ram_or_upper_rom_bank = value & 3
            }
            MBC1_BANKING_MODE_REG_START_ADDR..=MBC1_BANKING_MODE_REG_END_ADDR => {
                self.banking_mode = BankingMode::from(value);
            }
            MBC1_RAM_START_ADDR..=MBC1_RAM_END_ADDR => {
                if self.ram_enabled && self.ram.len() > 0 {
                    let addr = self.get_ram_address(address) & (self.ram.len() - 1);
                    self.ram[addr] = value;
                }
            }
            _ => unreachable!("invalid write address MBC1: {:#04x}", address),
        }
    }
}

struct MBC2<S: GameSave> {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank: usize,
    ram_enabled: bool,
    saver: S,
}

impl<S: GameSave> MBC2<S> {
    fn new(rom: Vec<u8>, saver: S) -> Self {
        Self {
            rom,
            ram: load_saved_ram(&saver, 512),
            rom_bank: 1,
            ram_enabled: false,
            saver,
        }
    }

    fn get_rom_address(&self, address: u16) -> usize {
        address as usize - 0x4000 + self.rom_bank * 0x4000
    }
}

impl<S: GameSave> MemReadWriter for MBC2<S> {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => {
                let adrr = self.get_rom_address(address) & (self.rom.len() - 1);
                self.rom[adrr]
            }
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    self.ram[(address as usize - 0xA000) % 512]
                } else {
                    0xFF
                }
            }
            _ => unreachable!("invalid address reading for MBC2: {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x3FFF => {
                if address & 0x0100 == 0 {
                    let enabled = right_nibble(value) == 0xA;
                    if self.ram_enabled && !enabled {
                        self.saver.save(&self.ram).unwrap();
                    }
                    self.ram_enabled = enabled;
                } else {
                    self.rom_bank = right_nibble(value).max(1) as usize;
                }
            }
            0x4000..=0x7FFF => {}
            0xA000..=0xBFFF => {
                if self.ram_enabled {
                    self.ram[(address as usize - 0xA000) % 512] = 0xF0 | right_nibble(value);
                }
            }
            _ => {
                unreachable!("invalid address writing for MBC2: {:#04x}", address)
            }
        }
    }
}

struct MBC5<S: GameSave> {
    rom: Vec<u8>,
    ram: Vec<u8>,
    rom_bank_lower: u8,
    rom_bank_9th_bit: bool,
    ram_enabled: bool,
    ram_bank: u8,
    saver: S,
}

impl<S: GameSave> MBC5<S> {
    fn new(rom: Vec<u8>, ram_size: usize, saver: S) -> Self {
        Self {
            rom,
            ram: load_saved_ram(&saver, ram_size),
            rom_bank_lower: 1,
            rom_bank_9th_bit: false,
            ram_enabled: false,
            ram_bank: 0,
            saver,
        }
    }

    fn get_rom_address(&self, address: u16) -> usize {
        let rom_bank = ((self.rom_bank_9th_bit as u32) << 8) | (self.rom_bank_lower) as u32;
        let addr = (address - 0x4000) as u32 + rom_bank * 0x4000;
        addr as usize
    }

    fn get_ram_address(&self, address: u16) -> usize {
        let addr = (address - 0xA000) + self.ram_bank as u16 * 0x2000;
        addr as usize
    }
}

impl<S: GameSave> MemReadWriter for MBC5<S> {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            ..=0x3FFF => self.rom[address as usize],
            0x4000..=0x7FFF => {
                let addr = self.get_rom_address(address) & (self.rom.len() - 1);
                self.rom[addr]
            }
            0xA000..=0xBFFF => {
                let mut val = 0xFF;
                if self.ram_enabled && self.ram.len() > 0 {
                    let addr = self.get_ram_address(address) & (self.ram.len() - 1);
                    val = self.ram[addr];
                }

                val
            }
            _ => unreachable!("invalid read address for MBC5: {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            ..=0x1FFF => {
                let enabled = right_nibble(value) == 0xA;
                if self.ram_enabled && !enabled {
                    self.saver.save(&self.ram).unwrap();
                }
                self.ram_enabled = enabled;
            }
            0x2000..=0x2FFF => self.rom_bank_lower = value,
            0x3000..=0x3FFF => self.rom_bank_9th_bit = value & 1 != 0,
            0x4000..=0x5FFF => self.ram_bank = value & 0xF,
            0xA000..=0xBFFF => {
                if self.ram_enabled && self.ram.len() > 0 {
                    let addr = self.get_ram_address(address) & (self.ram.len() - 1);
                    self.ram[addr] = value;
                }
            }
            _ => {}
        }
    }
}

fn get_target_mbc<S: GameSave + 'static>(
    code: u8,
    rom: Vec<u8>,
    ram_size: usize,
    saver: S,
) -> Box<dyn MemReadWriter> {
    match code {
        0x00 => Box::new(NoMBC::new(rom)),
        0x01..=0x03 => Box::new(MBC1::new(rom, ram_size, saver)),
        0x05..=0x06 => Box::new(MBC2::new(rom, saver)),
        0x19..=0x1E => Box::new(MBC5::new(rom, ram_size, saver)),
        _ => panic!("unimplemented or unreachable: {:#04x}", code),
    }
}

pub struct MBC {
    target_mbc: Box<dyn MemReadWriter>,
}

impl MBC {
    pub fn new<S: GameSave + 'static>(code: u8, rom: Vec<u8>, ram_size: usize, saver: S) -> Self {
        Self {
            target_mbc: get_target_mbc(code, rom, ram_size, saver),
        }
    }
}

impl MemReadWriter for MBC {
    fn read_byte(&self, address: u16) -> u8 {
        self.target_mbc.read_byte(address)
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        self.target_mbc.write_byte(address, value);
    }
}

#[cfg(test)]
mod tests {
    use crate::saver;

    use super::*;

    // https://gbdev.io/pandocs/MBC1.html#addressing-diagrams

    fn new_mbc1() -> MBC1<saver::Fake> {
        MBC1::new(vec![], 0x2000, saver::Fake)
    }

    #[test]
    fn test_mbc1_addressing_bank_0_simple_mode() {
        let mut mbc1 = new_mbc1();

        for addr in MBC1_ROM_BANK_0_START_ADDR..=MBC1_ROM_BANK_0_END_ADDR {
            assert_eq!(addr as usize, mbc1.get_rom_address(addr));
        }

        mbc1.write_byte(MBC1_RAM_BANK_NUM_REG_START_ADDR, 1);

        for addr in MBC1_ROM_BANK_0_START_ADDR..=MBC1_ROM_BANK_0_END_ADDR {
            assert_eq!(addr as usize, mbc1.get_rom_address(addr));
        }
    }

    #[test]
    fn test_mbc1_addressing_bank_0_advanced_mode() {
        let mut mbc1 = new_mbc1();

        mbc1.write_byte(MBC1_BANKING_MODE_REG_START_ADDR, 1);
        mbc1.write_byte(MBC1_RAM_BANK_NUM_REG_START_ADDR, 1);

        for addr in MBC1_ROM_BANK_0_START_ADDR..=MBC1_ROM_BANK_0_END_ADDR {
            assert_eq!((1 << 19) | addr as usize, mbc1.get_rom_address(addr));
        }
    }

    #[test]
    fn test_mbc1_addressing_bank_01_7f_simple_mode() {
        let mut mbc1 = new_mbc1();

        mbc1.write_byte(MBC1_ROM_BANK_NUM_REG_START_ADDR, 1);
        mbc1.write_byte(MBC1_RAM_BANK_NUM_REG_START_ADDR, 1);

        for addr in MBC1_ROM_BANK_01_7F_START_ADDR..=MBC1_ROM_BANK_01_7F_END_ADDR {
            assert_eq!(
                (1 << 19) | (1 << 14) | addr as usize,
                mbc1.get_rom_address(addr)
            );
        }
    }

    #[test]
    fn test_mbc1_addressing_bank_01_7f_advanced_mode() {
        let mut mbc1 = new_mbc1();

        mbc1.write_byte(MBC1_BANKING_MODE_REG_START_ADDR, 1);
        mbc1.write_byte(MBC1_ROM_BANK_NUM_REG_START_ADDR, 1);
        mbc1.write_byte(MBC1_RAM_BANK_NUM_REG_START_ADDR, 1);

        for addr in MBC1_ROM_BANK_01_7F_START_ADDR..=MBC1_ROM_BANK_01_7F_END_ADDR {
            assert_eq!(
                (1 << 19) | (1 << 14) | addr as usize,
                mbc1.get_rom_address(addr)
            );
        }
    }

    #[test]
    fn test_mbc1_addressing_bank_01_7f_0_treated_as_1() {
        let mut mbc1 = new_mbc1();

        mbc1.write_byte(MBC1_ROM_BANK_NUM_REG_START_ADDR, 0);

        for addr in MBC1_ROM_BANK_01_7F_START_ADDR..=MBC1_ROM_BANK_01_7F_END_ADDR {
            assert_eq!((1 << 14) | addr as usize, mbc1.get_rom_address(addr));
        }
    }

    #[test]
    fn test_mbc1_addressing_ram_simple_mode() {
        let mut mbc1 = new_mbc1();

        for addr in MBC1_RAM_START_ADDR..=MBC1_RAM_END_ADDR {
            assert_eq!(
                (addr - MBC1_RAM_START_ADDR) as usize,
                mbc1.get_ram_address(addr)
            );
        }

        mbc1.write_byte(MBC1_RAM_BANK_NUM_REG_START_ADDR, 1);

        for addr in MBC1_RAM_START_ADDR..=MBC1_RAM_END_ADDR {
            assert_eq!(
                (addr - MBC1_RAM_START_ADDR) as usize,
                mbc1.get_ram_address(addr)
            );
        }
    }

    #[test]
    fn test_mbc1_addressing_ram_advanced_mode() {
        let mut mbc1 = new_mbc1();

        mbc1.write_byte(MBC1_BANKING_MODE_REG_START_ADDR, 1);
        mbc1.write_byte(MBC1_RAM_BANK_NUM_REG_START_ADDR, 1);

        for addr in MBC1_RAM_START_ADDR..=MBC1_RAM_END_ADDR {
            assert_eq!(
                (1 << 13) | (addr - MBC1_RAM_START_ADDR) as usize,
                mbc1.get_ram_address(addr)
            );
        }
    }
}
