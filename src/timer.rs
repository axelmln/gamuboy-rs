use crate::{interrupts::InterruptRegisters, memory::MemReadWriter};

const BIT_4: u8 = 1 << 4;
const BIT_5: u8 = 1 << 5;

#[derive(Clone, Debug)]
struct TimerControl {
    inc_freq: u8,
    enabled: bool,
}

impl TimerControl {
    fn from(v: u8) -> Self {
        Self {
            inc_freq: v & 3,
            enabled: (v >> 2) & 1 != 0,
        }
    }

    fn falling_edge_bit(&self) -> u8 {
        match self.inc_freq {
            0 => 9,
            1 => 3,
            2 => 5,
            3 => 7,
            _ => unreachable!(),
        }
    }

    fn read(&self) -> u8 {
        self.inc_freq | ((self.enabled as u8) << 2)
    }

    fn write(&mut self, value: u8) {
        *self = Self::from(value);
    }
}

#[derive(Clone, Debug)]
struct SystemCounter {
    counter: u16,
    prev: u16,
    ticked: bool,
    div_apu_event: bool,
}

impl SystemCounter {
    fn new() -> Self {
        Self {
            counter: 0,
            prev: 0,
            ticked: false,
            div_apu_event: false,
        }
    }

    fn timer_ticked(&self, falling_edge_bit: u8) -> bool {
        let (prev_bit, curr_bit) = (
            (self.prev >> falling_edge_bit) & 1,
            (self.counter >> falling_edge_bit) & 1,
        );

        prev_bit == 1 && curr_bit == 0
    }

    fn div_apu_ticked(&self, double_speed_mode: bool) -> bool {
        let div = self.div();
        let bit = if double_speed_mode { BIT_5 } else { BIT_4 };
        let prev = (self.prev >> 8) as u8;

        (prev & bit == bit) && (div & bit == 0)
    }

    fn inc(&mut self, cycles: u8, falling_edge_bit: u8, double_speed_mode: bool) {
        self.ticked = false;

        self.counter = self.counter.wrapping_add(cycles as u16);

        self.ticked = self.timer_ticked(falling_edge_bit);

        self.div_apu_event = self.div_apu_ticked(double_speed_mode);

        self.prev = self.counter;
    }

    fn has_ticked(&self) -> bool {
        self.ticked
    }

    fn div(&self) -> u8 {
        (self.counter >> 8) as u8
    }

    fn reset(&mut self) {
        self.counter = 0;
    }
}

#[derive(Clone, Debug)]
pub struct Timer {
    system_counter: SystemCounter,
    delayed_timer: bool,
    tima: u8,
    tma: u8,
    tac: TimerControl,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            system_counter: SystemCounter::new(),
            delayed_timer: true,
            tima: 0,
            tma: 0,
            tac: TimerControl::from(0),
        }
    }

    pub fn check_apu_div(&self) -> bool {
        self.system_counter.div_apu_event
    }

    pub fn step(&mut self, int_reg: &mut InterruptRegisters, cycles: u8, double_speed_mode: bool) {
        self.system_counter
            .inc(cycles, self.tac.falling_edge_bit(), double_speed_mode);

        if !self.tac.enabled {
            return;
        }

        if self.delayed_timer {
            self.tima = self.tma;
            int_reg.request_timer();
            self.delayed_timer = false;
        }

        if self.system_counter.has_ticked() {
            let (new_tima, overflowed) = self.tima.overflowing_add(1);
            self.tima = new_tima;
            if overflowed {
                self.delayed_timer = true;
            }
        }
    }
}

impl MemReadWriter for Timer {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0xFF04 => self.system_counter.div(),
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac.read(),
            _ => unreachable!("Timer reading address {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0xFF04 => self.system_counter.reset(),
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac.write(value),
            _ => unreachable!("Timer writing address {:#04x}", address),
        }
    }
}
