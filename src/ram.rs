use crate::{memory::MemReadWriter, mode::Mode};

const WRAM_BANK0_START_ADDR: u16 = 0xC000;
const WRAM_BANK0_END_ADDR: u16 = 0xCFFF;

const WRAM_BANK1_7_START_ADDR: u16 = 0xD000;
const WRAM_BANK1_7_END_ADDR: u16 = 0xDFFF;

const ECHO_RAM_START_ADDR: u16 = 0xE000;
const ECHO_RAM_END_ADDR: u16 = 0xFDFF;

const HIGH_RAM_START_ADDR: u16 = 0xFF80;
const HIGH_RAM_END_ADDR: u16 = 0xFFFE;

const FOUR_KB: usize = 0x1000;

const HIGH_RAM_SIZE: usize = (HIGH_RAM_END_ADDR - HIGH_RAM_START_ADDR + 1) as usize;

pub struct RAM {
    mode: Mode,
    wram_bank0: [u8; FOUR_KB as usize],
    wram_bank1_7: [u8; FOUR_KB as usize * 7],
    high_ram: [u8; HIGH_RAM_SIZE],
    wram_bank: u8,
}

impl RAM {
    pub fn new(mode: Mode) -> Self {
        Self {
            wram_bank0: [0; FOUR_KB as usize],
            wram_bank1_7: [0; FOUR_KB as usize * 7],
            high_ram: [0; HIGH_RAM_SIZE],
            mode,
            wram_bank: 1,
        }
    }

    fn get_switchable_wram_addr(&self, address: u16) -> usize {
        match self.mode {
            Mode::DMG => address as usize,
            Mode::CGB => FOUR_KB * (self.wram_bank - 1) as usize + address as usize,
        }
    }
}

impl MemReadWriter for RAM {
    fn read_byte(&self, address: u16) -> u8 {
        match self.mode {
            Mode::CGB => match address {
                0xFF70 => return self.wram_bank,
                _ => {}
            },
            _ => {}
        }

        match address {
            WRAM_BANK0_START_ADDR..=WRAM_BANK0_END_ADDR => {
                self.wram_bank0[(address - WRAM_BANK0_START_ADDR) as usize]
            }
            WRAM_BANK1_7_START_ADDR..=WRAM_BANK1_7_END_ADDR => {
                self.wram_bank1_7
                    [self.get_switchable_wram_addr(address) - WRAM_BANK1_7_START_ADDR as usize]
            }
            HIGH_RAM_START_ADDR..=HIGH_RAM_END_ADDR => {
                self.high_ram[(address - HIGH_RAM_START_ADDR) as usize]
            }
            ECHO_RAM_START_ADDR..=ECHO_RAM_END_ADDR => {
                self.wram_bank0[(address.wrapping_sub(0x2000) - WRAM_BANK0_START_ADDR) as usize]
            }
            _ => 0xFF,
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match self.mode {
            Mode::CGB => match address {
                0xFF70 => return self.wram_bank = (value & 0b111).max(1),
                _ => {}
            },
            _ => {}
        }

        match address {
            WRAM_BANK0_START_ADDR..=WRAM_BANK0_END_ADDR => {
                self.wram_bank0[(address - WRAM_BANK0_START_ADDR) as usize] = value
            }
            WRAM_BANK1_7_START_ADDR..=WRAM_BANK1_7_END_ADDR => {
                self.wram_bank1_7
                    [self.get_switchable_wram_addr(address) - WRAM_BANK1_7_START_ADDR as usize] =
                    value
            }
            HIGH_RAM_START_ADDR..=HIGH_RAM_END_ADDR => {
                self.high_ram[(address - HIGH_RAM_START_ADDR) as usize] = value
            }
            ECHO_RAM_START_ADDR..=ECHO_RAM_END_ADDR => {
                self.wram_bank0[(address.wrapping_sub(0x2000) - WRAM_BANK0_START_ADDR) as usize] =
                    value
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgb_wram_bank_switch() {
        let mut ram = RAM::new(Mode::CGB);

        for i in 0..=7 {
            ram.write_byte(0xFF70, i);
            for addr in WRAM_BANK1_7_START_ADDR..=WRAM_BANK1_7_END_ADDR {
                ram.write_byte(addr, i);
            }
        }

        for i in 0..=7 {
            ram.write_byte(0xFF70, i);
            let expected = if i == 0 { 1 } else { i };
            for addr in WRAM_BANK1_7_START_ADDR..=WRAM_BANK1_7_END_ADDR {
                assert_eq!(expected, ram.read_byte(addr));
            }
        }
    }
}
