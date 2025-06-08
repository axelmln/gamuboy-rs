use crate::memory::MemReadWriter;

pub const VBLANK_ISR: u16 = 0x40;
pub const STAT_LCD_ISR: u16 = 0x48;
pub const TIMER_ISR: u16 = 0x50;
pub const SERIAL_ISR: u16 = 0x58;
pub const JOYPAD_ISR: u16 = 0x60;

const ISR_ADDRESSES: [u16; 5] = [VBLANK_ISR, STAT_LCD_ISR, TIMER_ISR, SERIAL_ISR, JOYPAD_ISR];

const VBLANK_BIT: usize = 0;
const STAT_LCD_BIT: usize = 1;
const TIMER_BIT: usize = 2;
const SERIAL_BIT: usize = 3;
const JOYPAD_BIT: usize = 4;

trait Interrupts {
    fn read(&self) -> u8;
    fn write(&mut self, value: u8);
}

impl Interrupts for [bool; 5] {
    fn read(&self) -> u8 {
        let mut val = 0xE0;
        for bit in 0..self.len() {
            val |= (self[bit] as u8) << bit;
        }

        val
    }

    fn write(&mut self, value: u8) {
        for bit in 0..self.len() {
            self[bit] = value & (1 << bit) != 0;
        }
    }
}

#[derive(Debug)]
pub struct InterruptRegisters {
    enables: [bool; 5],
    flags: [bool; 5],
}

impl InterruptRegisters {
    pub fn new() -> Self {
        Self {
            enables: [false; 5],
            flags: [false; 5],
        }
    }

    pub fn request_vblank(&mut self) {
        self.flags[VBLANK_BIT] = true;
    }

    pub fn request_stat_lcd(&mut self) {
        self.flags[STAT_LCD_BIT] = true;
    }

    pub fn request_timer(&mut self) {
        self.flags[TIMER_BIT] = true;
    }

    #[allow(dead_code)]
    pub fn request_serial(&mut self) {
        self.flags[SERIAL_BIT] = true;
    }

    pub fn request_joypad(&mut self) {
        self.flags[JOYPAD_BIT] = true;
    }

    pub fn check(&mut self, reset_flag: bool) -> Option<u16> {
        for bit in 0..ISR_ADDRESSES.len() {
            if self.enables[bit] && self.flags[bit] {
                self.flags[bit] = !reset_flag;
                return Some(ISR_ADDRESSES[bit]);
            }
        }
        None
    }
}

impl MemReadWriter for InterruptRegisters {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0xFF0F => self.flags.read(),
            0xFFFF => self.enables.read(),
            _ => unreachable!("Interrupts: reading address {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0xFF0F => self.flags.write(value),
            0xFFFF => self.enables.write(value),
            _ => unreachable!(
                "Interrupts: writing address {:#04x} value {:#04x}",
                address, value
            ),
        }
    }
}
