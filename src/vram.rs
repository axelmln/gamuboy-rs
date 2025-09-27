use crate::{memory::MemReadWriter, mode::Mode};

pub const BANK_REGISTER: u16 = 0xFF4F;

pub const BASE_ADDRESS: u16 = 0x8000;
const END_ADDRESS: u16 = 0x9FFF;

const BANK_SIZE: usize = (END_ADDRESS - BASE_ADDRESS + 1) as usize;

#[derive(Clone)]
pub struct VRAM {
    mem: [u8; BANK_SIZE * 2],
    bank: u8,
    mode: Mode,
}

impl VRAM {
    pub fn new(mode: Mode) -> Self {
        Self {
            mem: [0; BANK_SIZE * 2],
            bank: 0,
            mode,
        }
    }

    fn get_address(&self, address: u16) -> usize {
        match self.mode {
            Mode::DMG => compute_address_from_bank(address, 0),
            Mode::CGB => compute_address_from_bank(address, self.bank),
        }
    }

    pub fn read_at_bank(&self, address: u16, bank: u8) -> u8 {
        self.mem[compute_address_from_bank(address, bank)]
    }
}

fn compute_address_from_bank(address: u16, bank: u8) -> usize {
    (BANK_SIZE * bank as usize + address as usize) - (BASE_ADDRESS as usize)
}

impl MemReadWriter for VRAM {
    fn read_byte(&self, address: u16) -> u8 {
        match self.mode {
            Mode::CGB => match address {
                BANK_REGISTER => return 0b11111110 | self.bank,
                _ => {}
            },
            _ => {}
        }

        match address {
            BASE_ADDRESS..=END_ADDRESS => self.mem[self.get_address(address)],
            _ => unreachable!("VRAM reading address {:#04x}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match self.mode {
            Mode::CGB => match address {
                BANK_REGISTER => return self.bank = value & 1,
                _ => {}
            },
            _ => {}
        }

        match address {
            BASE_ADDRESS..=END_ADDRESS => self.mem[self.get_address(address)] = value,
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
            vram.write_byte(BANK_REGISTER, i);
            for addr in BASE_ADDRESS..=END_ADDRESS {
                vram.write_byte(addr, i);
            }
        }

        for i in 0..=1 {
            vram.write_byte(BANK_REGISTER, i);
            let expected = i;
            for addr in BASE_ADDRESS..=END_ADDRESS {
                assert_eq!(expected, vram.read_byte(addr));
            }
        }
    }
}
