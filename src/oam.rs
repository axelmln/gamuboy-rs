use crate::memory::MemReadWriter;

pub const BASE_ADDRESS: u16 = 0xFE00;

#[derive(Clone)]
pub struct OAM {
    mem: [u8; 0xFEA0],
}

impl OAM {
    pub fn new() -> Self {
        Self { mem: [0; 0xFEA0] }
    }
}

impl MemReadWriter for OAM {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            BASE_ADDRESS..=0xFE9F => self.mem[address as usize],
            _ => unreachable!("OAM reading address {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            BASE_ADDRESS..=0xFE9F => self.mem[address as usize] = value,
            _ => unreachable!("OAM writing address {:#04x}", address),
        }
    }
}
