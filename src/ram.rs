use crate::memory::MemReadWriter;

pub struct RAM {
    mem: [u8; 0xFFFF],
}

impl RAM {
    pub fn new() -> Self {
        Self { mem: [0; 0xFFFF] }
    }
}

impl MemReadWriter for RAM {
    fn read_byte(&self, address: u16) -> u8 {
        let addr = match address {
            0xC000..=0xDFFF | 0xFF80..=0xFFFE => address,
            0xE000..=0xFDFF => address.wrapping_sub(0x2000),
            _ => 0xFF,
        };
        self.mem[addr as usize]
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        let addr = match address {
            0xC000..=0xDFFF | 0xFF80..=0xFFFE => address,
            0xE000..=0xFDFF => address.wrapping_sub(0x2000),
            _ => return,
        };
        self.mem[addr as usize] = value
    }
}
