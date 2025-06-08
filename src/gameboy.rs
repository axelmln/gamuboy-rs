use std::{marker::Send, sync::mpsc::Receiver};

use crate::{
    apu::APU,
    bus::SystemBus,
    cartridge::Cartridge,
    config::Config,
    cpu::{self, CPU},
    interrupts::InterruptRegisters,
    joypad::Joypad,
    joypad_events_handler::EventsHandler,
    lcd::LCD,
    oam::OAM,
    ppu::PPU,
    ram::RAM,
    saver::GameSave,
    serial::Serial,
    stereo::StereoPlayer,
    timer::Timer,
    vram::VRAM,
};

pub struct GameBoy<
    'a,
    L: LCD + 'static,
    E: Send + 'static,
    H: EventsHandler<E>,
    S: StereoPlayer + 'static,
> {
    cpu: cpu::CPU<SystemBus<'a, L, E, H, S>>,
}

impl<'a, L: LCD, E: Send + 'static, H: EventsHandler<E>, S: StereoPlayer> GameBoy<'a, L, E, H, S> {
    pub fn new<GS: GameSave + 'static>(
        cfg: &Config,
        lcd: L,
        stereo: S,
        joypad_events_handler: H,
        saver: GS,
        event_rx: &'a Receiver<E>,
    ) -> Self {
        Self {
            cpu: CPU::new(
                cfg,
                SystemBus::new(
                    Cartridge::new(cfg, saver),
                    APU::new(stereo),
                    PPU::new(cfg, VRAM::new(), OAM::new(), lcd),
                    InterruptRegisters::new(),
                    Joypad::new(),
                    Timer::new(),
                    Serial::new(),
                    RAM::new(),
                    joypad_events_handler,
                    event_rx,
                ),
            ),
        }
    }

    pub fn step(&mut self) {
        let _cycles = self.cpu.step();
    }

    pub fn run(&mut self) {
        loop {
            self.step();
        }
    }
}
