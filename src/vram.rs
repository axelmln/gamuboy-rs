use crate::memory::MemReadWriter;

pub const BASE_ADDRESS: u16 = 0x8000;

#[derive(Clone)]
pub struct VRAM {
    mem: [u8; 0xA000],
}

impl VRAM {
    pub fn new() -> Self {
        Self { mem: [0; 0xA000] }
    }
}

impl MemReadWriter for VRAM {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x8000..=0x9FFF => self.mem[address as usize],
            _ => unreachable!("VRAM reading address {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x8000..=0x9FFF => self.mem[address as usize] = value,
            _ => unreachable!("VRAM writing address {:#04x}", address),
        }
    }
}
