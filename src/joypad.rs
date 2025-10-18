use crate::{interrupts::InterruptRegisters, memory::MemReadWriter};

#[derive(Clone)]
pub enum Button {
    A,
    B,
    Select,
    Start,

    Right,
    Left,
    Up,
    Down,
}

impl From<u8> for Button {
    fn from(value: u8) -> Self {
        match value {
            0 => Button::A,
            1 => Button::B,
            2 => Button::Select,
            3 => Button::Start,
            4 => Button::Right,
            5 => Button::Left,
            6 => Button::Up,
            7 => Button::Down,
            _ => unreachable!(),
        }
    }
}

#[derive(Clone, Debug)]
struct PadState {
    buttons: [bool; 4],
    dpad: [bool; 4],
}

impl PadState {
    fn new() -> Self {
        Self {
            buttons: [false; 4],
            dpad: [false; 4],
        }
    }
}

trait State {
    fn read(&self) -> u8;
}

impl State for [bool; 4] {
    fn read(&self) -> u8 {
        let mut val = 0;
        for bit in 0..self.len() {
            val |= (!self[bit] as u8) << bit;
        }

        val
    }
}

#[derive(Clone, Debug)]
pub struct Joypad {
    select_buttons: bool,
    select_dpad: bool,
    prev_state: PadState,
    state: PadState,
}

impl Joypad {
    pub fn new() -> Self {
        Self {
            select_buttons: false,
            select_dpad: false,
            prev_state: PadState::new(),
            state: PadState::new(),
        }
    }

    pub fn update(&mut self, button: Button, pressed: bool) {
        match button as usize {
            bit @ 0..=3 => self.state.buttons[bit] = pressed,
            bit @ 4..=7 => self.state.dpad[bit % 4] = pressed,
            _ => unreachable!(),
        }
    }

    fn read(&self) -> u8 {
        if self.select_buttons {
            self.state.buttons.read()
        } else if self.select_dpad {
            self.state.dpad.read()
        } else {
            0xFF
        }
    }

    fn write(&mut self, value: u8) {
        self.select_dpad = value & 0x10 == 0;
        self.select_buttons = value & 0x20 == 0;
    }

    pub fn check(&mut self, int_reg: &mut InterruptRegisters) {
        if !self.select_buttons && !self.select_dpad {
            return;
        }

        for bit in 0..self.state.buttons.len() {
            if self.select_buttons {
                if !self.prev_state.buttons[bit] && self.state.buttons[bit] {
                    int_reg.request_joypad();
                    break;
                }
            } else if !self.prev_state.dpad[bit] && self.state.dpad[bit] {
                int_reg.request_joypad();
                break;
            }
        }

        self.prev_state = self.state.clone();
    }
}

impl MemReadWriter for Joypad {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0xFF00 => self.read(),
            _ => unreachable!("Joypad reading address {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0xFF00 => self.write(value),
            _ => unreachable!("Joypad writing address {:#04x}", address),
        }
    }
}
