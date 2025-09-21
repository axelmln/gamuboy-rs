use crate::{memory::MemReadWriter, mode::Mode};

pub const BASE_ADDRESS: u16 = 0x8000;
const END_ADDRESS: u16 = 0x9FFF;

const BANK_SIZE: usize = (END_ADDRESS - BASE_ADDRESS + 1) as usize;

#[derive(Clone)]
pub struct VRAM {
    mem: [u8; 0xA000],
    bank: u8,
    mode: Mode,
}

impl VRAM {
    pub fn new(mode: Mode) -> Self {
        Self {
            mem: [0; 0xA000],
            bank: 0,
            mode,
        }
    }

    fn get_address(&self, address: u16) -> usize {
        match self.mode {
            Mode::DMG => address as usize,
            Mode::CGB => BANK_SIZE * self.bank as usize + address as usize,
        }
    }
}

impl MemReadWriter for VRAM {
    fn read_byte(&self, address: u16) -> u8 {
        match self.mode {
            Mode::CGB => match address {
                0xFF4F => return 0b11111110 | self.bank,
                _ => {}
            },
            _ => {}
        }

        match address {
            BASE_ADDRESS..=END_ADDRESS => {
                self.mem[self.get_address(address) - (BASE_ADDRESS as usize)]
            }
            _ => unreachable!("VRAM reading address {:#04x}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match self.mode {
            Mode::CGB => match address {
                0xFF4F => return self.bank = value & 1,
                _ => {}
            },
            _ => {}
        }

        match address {
            BASE_ADDRESS..=END_ADDRESS => {
                self.mem[self.get_address(address) - (BASE_ADDRESS as usize)] = value
            }
            _ => unreachable!("VRAM writing address {:#04x}", address),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cgb_bank_switch() {
        let mut vram = VRAM::new(Mode::CGB);

        for i in 0..=1 {
            vram.write_byte(0xFF4F, i);
            for addr in BASE_ADDRESS..=END_ADDRESS {
                vram.write_byte(addr, i);
            }
        }

        for i in 0..=1 {
            vram.write_byte(0xFF4F, i);
            let expected = i;
            for addr in BASE_ADDRESS..=END_ADDRESS {
                assert_eq!(expected, vram.read_byte(addr));
            }
        }
    }
}
