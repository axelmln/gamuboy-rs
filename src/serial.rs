use crate::memory::MemReadWriter;

/// Not implemented
#[derive(Clone)]
pub struct Serial {}

impl Serial {
    pub fn new() -> Self {
        Self {}
    }
}

impl MemReadWriter for Serial {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0xFF01..=0xFF02 => 0xFF,
            _ => unreachable!("Serial reading address {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, _value: u8) {
        match address {
            0xFF01..=0xFF02 => {}
            _ => unreachable!("Serial writing address {:#04x}", address),
        }
    }
}
