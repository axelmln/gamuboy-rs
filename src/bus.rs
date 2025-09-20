use std::sync::mpsc::Receiver;

use crate::{
    apu::APU, cartridge::Cartridge, interrupts::InterruptRegisters, joypad::Joypad,
    joypad_events_handler, lcd::LCD, memory::MemReadWriter, ppu::PPU, ram::RAM, serial::Serial,
    stereo::StereoPlayer, timer::Timer,
};

/// Bus acts as an interface between the cpu and other system components
pub trait Bus {
    fn read_byte(&self, address: u16) -> u8;
    fn write_byte(&mut self, address: u16, value: u8);

    fn check_interrupts(&mut self, reset_flag: bool) -> Option<u16>;

    fn switch_speed(&mut self);

    fn step_peripherals(&mut self, cycles: u8);
}

pub struct SystemBus<
    'a,
    L: LCD + 'static,
    E: Send + 'static,
    H: joypad_events_handler::EventsHandler<E>,
    S: StereoPlayer + 'static,
> {
    dummy_mem: Vec<u8>,

    cartridge: Cartridge,
    apu: APU<S>,
    ppu: PPU<L>,
    int_reg: InterruptRegisters,
    joypad: Joypad,
    timer: Timer,
    serial: Serial,
    ram: RAM,
    joypad_events_handler: H,
    event_rx: &'a Receiver<E>,

    double_speed_mode: bool,
    switch_armed: bool,
}

impl<
        'a,
        L: LCD,
        E: Send + 'static,
        H: joypad_events_handler::EventsHandler<E>,
        S: StereoPlayer,
    > SystemBus<'a, L, E, H, S>
{
    pub fn new(
        cartridge: Cartridge,
        apu: APU<S>,
        ppu: PPU<L>,
        int_reg: InterruptRegisters,
        joypad: Joypad,
        timer: Timer,
        serial: Serial,
        ram: RAM,
        joypad_events_handler: H,
        event_rx: &'a Receiver<E>,
    ) -> Self {
        Self {
            dummy_mem: vec![0xFF; 0xA0000],

            cartridge,
            apu,
            ppu,
            int_reg,
            joypad,
            timer,
            serial,
            ram,
            joypad_events_handler,
            event_rx,

            double_speed_mode: false,
            switch_armed: false,
        }
    }

    fn dma_transfer(&mut self, value: u8) {
        let src = value as u16 * 0x100;
        for (i, addr) in (0xFE00..=0xFE9F).enumerate() {
            let val = self.read_byte(src + i as u16);
            self.ppu.write_oam(addr, val);
        }
    }
}

impl<
        'a,
        L: LCD,
        E: Send + 'static,
        H: joypad_events_handler::EventsHandler<E>,
        S: StereoPlayer,
    > Bus for SystemBus<'a, L, E, H, S>
{
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF | 0xFF50..=0xFF50 => {
                self.cartridge.read_byte(address)
            }
            0xFF10..=0xFF3F => self.apu.read_byte(address),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => self.ppu.read_byte(address),
            0xFF0F | 0xFFFF => self.int_reg.read_byte(address),
            0xFF00 => self.joypad.read_byte(address),
            0xFF04..=0xFF07 => self.timer.read_byte(address),
            0xFF01..=0xFF02 => self.serial.read_byte(address),
            0xC000..=0xFDFF | 0xFF80..=0xFFFE => self.ram.read_byte(address),

            0xFF4D => {
                let spd = (self.double_speed_mode as u8) << 7 | self.switch_armed as u8;
                spd
            }

            _ => self.dummy_mem[address as usize],
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        match address {
            0x0000..=0x7FFF | 0xA000..=0xBFFF | 0xFF50..=0xFF50 => {
                self.cartridge.write_byte(address, value)
            }
            0xFF10..=0xFF3F => self.apu.write_byte(address, value),
            0x8000..=0x9FFF | 0xFE00..=0xFE9F | 0xFF40..=0xFF4B => {
                self.ppu.write_byte(address, value)
            }
            0xFF0F | 0xFFFF => self.int_reg.write_byte(address, value),
            0xFF00 => self.joypad.write_byte(address, value),
            0xFF04..=0xFF07 => self.timer.write_byte(address, value),
            0xFF01..=0xFF02 => self.serial.write_byte(address, value),
            0xC000..=0xFDFF | 0xFF80..=0xFFFE => self.ram.write_byte(address, value),

            0xFF4D => self.switch_armed = value & 1 == 1,

            _ => self.dummy_mem[address as usize] = value,
        };
    }

    fn check_interrupts(&mut self, reset_flag: bool) -> Option<u16> {
        self.int_reg.check(reset_flag)
    }

    fn switch_speed(&mut self) {
        if self.switch_armed {
            self.double_speed_mode = !self.double_speed_mode;
            self.switch_armed = false;
        }
    }

    fn step_peripherals(&mut self, cycles: u8) {
        let normal_speed_cycles = if self.double_speed_mode {
            cycles / 2
        } else {
            cycles
        };

        self.joypad_events_handler
            .handle_events(self.event_rx, &mut self.joypad);

        self.ppu.step(&mut self.int_reg, normal_speed_cycles);

        if let Some(value) = self.ppu.check_dma_request() {
            self.dma_transfer(value); // TODO: handle with cycle accuracy
        }

        self.timer
            .step(&mut self.int_reg, cycles, self.double_speed_mode);

        let div_apu_event = self.timer.check_apu_div();

        self.apu.step(normal_speed_cycles, div_apu_event);

        self.joypad.check(&mut self.int_reg);
    }
}
