use crate::{bus::Bus, config::Config, registers};

const INSTRUCTION_PREFIX: u8 = 0xCB;

pub struct CPU<B: Bus> {
    is_halted: bool,
    is_stopped: bool,
    ime: bool,
    ime_delayed: bool,
    registers: registers::Registers,
    pc: u16,
    sp: u16,

    bus: B,

    cycles_synced: u8,
}

impl<B: Bus> CPU<B> {
    pub fn new(cfg: &Config, bus: B) -> Self {
        let skip_boot = cfg.bootrom.is_none();

        Self {
            is_halted: false,
            is_stopped: false,
            ime: false,
            ime_delayed: false,

            registers: if skip_boot {
                registers::Registers::new_post_boot()
            } else {
                registers::Registers::new()
            },
            pc: if skip_boot { 0x0100 } else { 0 },
            sp: if skip_boot { 0xFFFE } else { 0 },

            bus,

            cycles_synced: 0,
        }
    }

    fn read_byte(&mut self, address: u16) -> u8 {
        let v = self.bus.read_byte(address);
        self.bus.step_peripherals(4);
        self.cycles_synced += 4;
        v
    }

    fn read_two_bytes(&mut self, address: u16) -> u16 {
        let left = self.read_byte(address);
        let right = self.read_byte(address.wrapping_add(1));
        (right as u16) << 8 | left as u16
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        self.bus.write_byte(address, value);
        self.bus.step_peripherals(4);
        self.cycles_synced += 4;
    }

    fn write_two_bytes(&mut self, address: u16, value: u16) {
        self.write_byte(address, value as u8);
        self.write_byte(address.wrapping_add(1), (value >> 8) as u8);
    }

    fn enable_ime(&mut self) {
        self.ime_delayed = true;
    }

    fn execute(&mut self, instruction_byte: u8) -> Option<(u16, u8)> {
        match instruction_byte {
            0x00 => Some((self.pc.wrapping_add(1), 4)),
            0x10 => {
                self.is_stopped = true;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x03 => {
                let val = self.inc_16bits(self.registers.get_bc());
                self.registers.set_bc(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x13 => {
                let val = self.inc_16bits(self.registers.get_de());
                self.registers.set_de(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x23 => {
                let val = self.inc_16bits(self.registers.get_hl());
                self.registers.set_hl(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x33 => {
                self.sp = self.inc_16bits(self.sp);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x04 => {
                self.registers.b = self.inc(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x14 => {
                self.registers.d = self.inc(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x24 => {
                self.registers.h = self.inc(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x34 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let inc_val = self.inc(mem_val);
                self.write_byte(hl_reg_val, inc_val);
                Some((self.pc.wrapping_add(1), 12))
            }
            0x05 => {
                self.registers.b = self.dec(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x15 => {
                self.registers.d = self.dec(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x25 => {
                self.registers.h = self.dec(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x35 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let inc_val = self.dec(mem_val);
                self.write_byte(hl_reg_val, inc_val);
                Some((self.pc.wrapping_add(1), 12))
            }
            0x07 => {
                self.rlca();
                Some((self.pc.wrapping_add(1), 4))
            }
            0x17 => {
                self.rla();
                Some((self.pc.wrapping_add(1), 4))
            }
            0x27 => {
                self.daa();
                Some((self.pc.wrapping_add(1), 4))
            }
            0x37 => {
                self.scf();
                Some((self.pc.wrapping_add(1), 4))
            }

            0x09 => {
                self.addhl(self.registers.get_bc());
                Some((self.pc.wrapping_add(1), 8))
            }
            0x19 => {
                self.addhl(self.registers.get_de());
                Some((self.pc.wrapping_add(1), 8))
            }
            0x29 => {
                self.addhl(self.registers.get_hl());
                Some((self.pc.wrapping_add(1), 8))
            }
            0x39 => {
                self.addhl(self.sp);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x0B => {
                let val = self.dec_16bits(self.registers.get_bc());
                self.registers.set_bc(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x1B => {
                let val = self.dec_16bits(self.registers.get_de());
                self.registers.set_de(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x2B => {
                let val = self.dec_16bits(self.registers.get_hl());
                self.registers.set_hl(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x3B => {
                self.sp = self.dec_16bits(self.sp);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x0C => {
                self.registers.c = self.inc(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x1C => {
                self.registers.e = self.inc(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x2C => {
                self.registers.l = self.inc(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x3C => {
                self.registers.a = self.inc(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x0D => {
                self.registers.c = self.dec(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x1D => {
                self.registers.e = self.dec(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x2D => {
                self.registers.l = self.dec(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x3D => {
                self.registers.a = self.dec(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x0F => {
                self.rrca();
                Some((self.pc.wrapping_add(1), 4))
            }
            0x1F => {
                self.rra();
                Some((self.pc.wrapping_add(1), 4))
            }
            0x2F => {
                self.cpl();
                Some((self.pc.wrapping_add(1), 4))
            }
            0x3F => {
                self.ccf();
                Some((self.pc.wrapping_add(1), 4))
            }

            0x80 => {
                self.add(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x81 => {
                self.add(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x82 => {
                self.add(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x83 => {
                self.add(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x84 => {
                self.add(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x85 => {
                self.add(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x86 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.add(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x87 => {
                self.add(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x88 => {
                self.adc(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x89 => {
                self.adc(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x8A => {
                self.adc(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x8B => {
                self.adc(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x8C => {
                self.adc(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x8D => {
                self.adc(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x8E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.adc(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x8F => {
                self.adc(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            0x90 => {
                self.sub(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x91 => {
                self.sub(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x92 => {
                self.sub(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x93 => {
                self.sub(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x94 => {
                self.sub(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x95 => {
                self.sub(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x96 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.sub(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x97 => {
                self.sub(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x98 => {
                self.sbc(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x99 => {
                self.sbc(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x9A => {
                self.sbc(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x9B => {
                self.sbc(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x9C => {
                self.sbc(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x9D => {
                self.sbc(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0x9E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.sbc(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x9F => {
                self.sbc(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            0xA0 => {
                self.and(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA1 => {
                self.and(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA2 => {
                self.and(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA3 => {
                self.and(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA4 => {
                self.and(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA5 => {
                self.and(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA6 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.and(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0xA7 => {
                self.and(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA8 => {
                self.xor(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xA9 => {
                self.xor(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xAA => {
                self.xor(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xAB => {
                self.xor(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xAC => {
                self.xor(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xAD => {
                self.xor(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xAE => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.xor(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0xAF => {
                self.xor(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            0xB0 => {
                self.or(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB1 => {
                self.or(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB2 => {
                self.or(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB3 => {
                self.or(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB4 => {
                self.or(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB5 => {
                self.or(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB6 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.or(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0xB7 => {
                self.or(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB8 => {
                self.cp(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xB9 => {
                self.cp(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xBA => {
                self.cp(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xBB => {
                self.cp(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xBC => {
                self.cp(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xBD => {
                self.cp(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            0xBE => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.cp(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            0xBF => {
                self.cp(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            0xC6 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.add(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD6 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.sub(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE6 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.and(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF6 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.or(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }

            0xCE => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.adc(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xDE => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.sbc(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xEE => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.xor(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xFE => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.cp(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }

            0x20 => Some(self.jr(!self.registers.f.zero)),
            0x30 => Some(self.jr(!self.registers.f.carry)),
            0x18 => Some(self.jr(true)),
            0x28 => Some(self.jr(self.registers.f.zero)),
            0x38 => Some(self.jr(self.registers.f.carry)),

            0xC2 => Some(self.jp(!self.registers.f.zero)),
            0xD2 => Some(self.jp(!self.registers.f.carry)),
            0xC3 => Some(self.jp(true)),
            0xCA => Some(self.jp(self.registers.f.zero)),
            0xDA => Some(self.jp(self.registers.f.carry)),

            // LDs
            0x01 => {
                let mem_val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.set_bc(mem_val);
                Some((self.pc.wrapping_add(3), 12))
            }
            0x11 => {
                let mem_val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.set_de(mem_val);
                Some((self.pc.wrapping_add(3), 12))
            }
            0x21 => {
                let mem_val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.set_hl(mem_val);
                Some((self.pc.wrapping_add(3), 12))
            }
            0x31 => {
                self.sp = self.read_two_bytes(self.pc.wrapping_add(1));
                Some((self.pc.wrapping_add(3), 12))
            }
            0x02 => {
                self.write_byte(self.registers.get_bc(), self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x12 => {
                self.write_byte(self.registers.get_de(), self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x22 => {
                let hl_reg_val = self.registers.get_hl();
                self.write_byte(hl_reg_val, self.registers.a);
                self.registers.set_hl(hl_reg_val.wrapping_add(1));
                Some((self.pc.wrapping_add(1), 8))
            }
            0x32 => {
                let hl_reg_val = self.registers.get_hl();
                self.write_byte(hl_reg_val, self.registers.a);
                self.registers.set_hl(hl_reg_val.wrapping_sub(1));
                Some((self.pc.wrapping_add(1), 8))
            }

            0x06 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.b = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            0x16 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.d = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            0x26 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.h = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            0x36 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.write_byte(self.registers.get_hl(), mem_val);
                Some((self.pc.wrapping_add(2), 12))
            }

            0x08 => {
                let val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.write_two_bytes(val, self.sp);
                Some((self.pc.wrapping_add(3), 20))
            }

            0x0A => {
                self.registers.a = self.read_byte(self.registers.get_bc());
                Some((self.pc.wrapping_add(1), 8))
            }
            0x1A => {
                self.registers.a = self.read_byte(self.registers.get_de());
                Some((self.pc.wrapping_add(1), 8))
            }
            0x2A => {
                let hl_reg_val = self.registers.get_hl();
                self.registers.a = self.read_byte(hl_reg_val);
                self.registers.set_hl(hl_reg_val.wrapping_add(1));
                Some((self.pc.wrapping_add(1), 8))
            }
            0x3A => {
                let hl_reg_val = self.registers.get_hl();
                self.registers.a = self.read_byte(hl_reg_val);
                self.registers.set_hl(hl_reg_val.wrapping_sub(1));
                Some((self.pc.wrapping_add(1), 8))
            }

            0x0E => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.c = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            0x1E => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.e = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            0x2E => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.l = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            0x3E => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.a = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }

            0x40 => {
                self.registers.b = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x41 => {
                self.registers.b = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x42 => {
                self.registers.b = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x43 => {
                self.registers.b = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x44 => {
                self.registers.b = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x45 => {
                self.registers.b = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x46 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.b = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            0x47 => {
                self.registers.b = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x48 => {
                self.registers.c = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x49 => {
                self.registers.c = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x4A => {
                self.registers.c = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x4B => {
                self.registers.c = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x4C => {
                self.registers.c = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x4D => {
                self.registers.c = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x4E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.c = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            0x4F => {
                self.registers.c = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x50 => {
                self.registers.d = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x51 => {
                self.registers.d = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x52 => {
                self.registers.d = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x53 => {
                self.registers.d = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x54 => {
                self.registers.d = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x55 => {
                self.registers.d = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x56 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.d = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            0x57 => {
                self.registers.d = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x58 => {
                self.registers.e = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x59 => {
                self.registers.e = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x5A => {
                self.registers.e = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x5B => {
                self.registers.e = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x5C => {
                self.registers.e = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x5D => {
                self.registers.e = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x5E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.e = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            0x5F => {
                self.registers.e = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x60 => {
                self.registers.h = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x61 => {
                self.registers.h = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x62 => {
                self.registers.h = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x63 => {
                self.registers.h = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x64 => {
                self.registers.h = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x65 => {
                self.registers.h = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x66 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.h = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            0x67 => {
                self.registers.h = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x68 => {
                self.registers.l = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x69 => {
                self.registers.l = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x6A => {
                self.registers.l = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x6B => {
                self.registers.l = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x6C => {
                self.registers.l = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x6D => {
                self.registers.l = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x6E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.l = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            0x6F => {
                self.registers.l = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x70 => {
                self.write_byte(self.registers.get_hl(), self.registers.b);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x71 => {
                self.write_byte(self.registers.get_hl(), self.registers.c);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x72 => {
                self.write_byte(self.registers.get_hl(), self.registers.d);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x73 => {
                self.write_byte(self.registers.get_hl(), self.registers.e);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x74 => {
                self.write_byte(self.registers.get_hl(), self.registers.h);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x75 => {
                self.write_byte(self.registers.get_hl(), self.registers.l);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x76 => {
                self.is_halted = true;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x77 => {
                self.write_byte(self.registers.get_hl(), self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            0x78 => {
                self.registers.a = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x79 => {
                self.registers.a = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x7A => {
                self.registers.a = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x7B => {
                self.registers.a = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x7C => {
                self.registers.a = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x7D => {
                self.registers.a = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            0x7E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.a = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            0x7F => {
                self.registers.a = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            0xE0 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.write_byte(0xFF00 + mem_val as u16, self.registers.a);
                Some((self.pc.wrapping_add(2), 12))
            }
            0xF0 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.a = self.read_byte(0xFF00 + mem_val as u16);
                Some((self.pc.wrapping_add(2), 12))
            }
            0xE2 => {
                self.write_byte(0xFF00 + self.registers.c as u16, self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            0xF2 => {
                self.registers.a = self.read_byte(0xFF00 + self.registers.c as u16);
                Some((self.pc.wrapping_add(1), 8))
            }

            0xE8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1)) as i8;
                let init_sp = self.sp;

                self.sp = add_u16_i8(init_sp, mem_val);

                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = ((init_sp & 0xF) + ((mem_val as u16) & 0xF)) > 0xF;
                self.registers.f.carry = ((init_sp & 0xFF) + ((mem_val as u16) & 0xFF)) > 0xFF;

                Some((self.pc.wrapping_add(2), 16))
            }

            0xF8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                let val = mem_val;
                self.registers.set_hl(add_u16_i8(self.sp, val as i8));
                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = half_carry_add_8bits(self.sp as u8, val);
                self.registers.f.carry = ((self.sp & 0xFF).wrapping_add(val as u16)) > 0xFF;
                Some((self.pc.wrapping_add(2), 12))
            }
            0xF9 => {
                self.sp = self.registers.get_hl();
                Some((self.pc.wrapping_add(1), 8))
            }

            0xEA => {
                let val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.write_byte(val, self.registers.a);
                Some((self.pc.wrapping_add(3), 16))
            }
            0xFA => {
                let val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.a = self.read_byte(val);
                Some((self.pc.wrapping_add(3), 16))
            } // END LDs

            0xC1 => {
                let val = self.pop();
                self.registers.set_bc(val);
                Some((self.pc.wrapping_add(1), 12))
            }
            0xD1 => {
                let val = self.pop();
                self.registers.set_de(val);
                Some((self.pc.wrapping_add(1), 12))
            }
            0xE1 => {
                let val = self.pop();
                self.registers.set_hl(val);
                Some((self.pc.wrapping_add(1), 12))
            }
            0xF1 => {
                let val = self.pop();
                self.registers.set_af(val);
                self.registers.f.zero = ((val & 0xFF) >> 7) & 1 == 1;
                self.registers.f.subtract = ((val & 0xFF) >> 6) & 1 == 1;
                self.registers.f.half_carry = ((val & 0xFF) >> 5) & 1 == 1;
                self.registers.f.carry = ((val & 0xFF) >> 4) & 1 == 1;
                Some((self.pc.wrapping_add(1), 12))
            }

            0xC5 => {
                self.push(self.registers.get_bc());
                Some((self.pc.wrapping_add(1), 16))
            }
            0xD5 => {
                self.push(self.registers.get_de());
                Some((self.pc.wrapping_add(1), 16))
            }
            0xE5 => {
                self.push(self.registers.get_hl());
                Some((self.pc.wrapping_add(1), 16))
            }
            0xF5 => {
                self.push(self.registers.get_af());
                Some((self.pc.wrapping_add(1), 16))
            }

            0xC0 => Some(self.ret(!self.registers.f.zero)),
            0xD0 => Some(self.ret(!self.registers.f.carry)),

            0xC4 => Some(self.call(!self.registers.f.zero)),
            0xD4 => Some(self.call(!self.registers.f.carry)),

            0xC8 => Some(self.ret(self.registers.f.zero)),
            0xD8 => Some(self.ret(self.registers.f.carry)),
            0xC9 => {
                let (pc, _) = self.ret(true);
                Some((pc, 16))
            }
            0xD9 => {
                self.ime = true;
                let (pc, _) = self.ret(true);
                Some((pc, 16))
            }

            0xE9 => Some((self.registers.get_hl(), 4)),

            0xCC => Some(self.call(self.registers.f.zero)),
            0xDC => Some(self.call(self.registers.f.carry)),
            0xCD => Some(self.call(true)),

            0xF3 => {
                self.ime = false;
                Some((self.pc.wrapping_add(1), 4))
            }
            0xFB => {
                self.enable_ime();
                Some((self.pc.wrapping_add(1), 4))
            }

            0xC7 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x00, 16))
            }
            0xD7 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x10, 16))
            }
            0xE7 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x20, 16))
            }
            0xF7 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x30, 16))
            }
            0xCF => {
                self.push(self.pc.wrapping_add(1));
                Some((0x08, 16))
            }
            0xDF => {
                self.push(self.pc.wrapping_add(1));
                Some((0x18, 16))
            }
            0xEF => {
                self.push(self.pc.wrapping_add(1));
                Some((0x28, 16))
            }
            0xFF => {
                self.push(self.pc.wrapping_add(1));
                Some((0x38, 16))
            }

            _ => None,
        }
    }

    fn execute_prefixed(&mut self, instruction_byte: u8) -> Option<(u16, u8)> {
        match instruction_byte {
            0x00 => {
                self.registers.b = self.rlc(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x01 => {
                self.registers.c = self.rlc(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x02 => {
                self.registers.d = self.rlc(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x03 => {
                self.registers.e = self.rlc(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x04 => {
                self.registers.h = self.rlc(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x05 => {
                self.registers.l = self.rlc(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x06 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rlc(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x07 => {
                self.registers.a = self.rlc(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x08 => {
                self.registers.b = self.rrc(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x09 => {
                self.registers.c = self.rrc(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x0A => {
                self.registers.d = self.rrc(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x0B => {
                self.registers.e = self.rrc(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x0C => {
                self.registers.h = self.rrc(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x0D => {
                self.registers.l = self.rrc(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x0E => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rrc(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x0F => {
                self.registers.a = self.rrc(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            0x10 => {
                self.registers.b = self.rl(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x11 => {
                self.registers.c = self.rl(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x12 => {
                self.registers.d = self.rl(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x13 => {
                self.registers.e = self.rl(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x14 => {
                self.registers.h = self.rl(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x15 => {
                self.registers.l = self.rl(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x16 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rl(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x17 => {
                self.registers.a = self.rl(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            0x18 => {
                self.registers.b = self.rr(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x19 => {
                self.registers.c = self.rr(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x1A => {
                self.registers.d = self.rr(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x1B => {
                self.registers.e = self.rr(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x1C => {
                self.registers.h = self.rr(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x1D => {
                self.registers.l = self.rr(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x1E => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rr(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x1F => {
                self.registers.a = self.rr(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            0x20 => {
                self.registers.b = self.sla(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x21 => {
                self.registers.c = self.sla(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x22 => {
                self.registers.d = self.sla(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x23 => {
                self.registers.e = self.sla(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x24 => {
                self.registers.h = self.sla(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x25 => {
                self.registers.l = self.sla(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x26 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.sla(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x27 => {
                self.registers.a = self.sla(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x28 => {
                self.registers.b = self.sra(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x29 => {
                self.registers.c = self.sra(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x2A => {
                self.registers.d = self.sra(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x2B => {
                self.registers.e = self.sra(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x2C => {
                self.registers.h = self.sra(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x2D => {
                self.registers.l = self.sra(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x2E => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.sra(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x2F => {
                self.registers.a = self.sra(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x30 => {
                self.registers.b = self.swap(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x31 => {
                self.registers.c = self.swap(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x32 => {
                self.registers.d = self.swap(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x33 => {
                self.registers.e = self.swap(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x34 => {
                self.registers.h = self.swap(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x35 => {
                self.registers.l = self.swap(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x36 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.swap(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x37 => {
                self.registers.a = self.swap(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x38 => {
                self.registers.b = self.srl(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x39 => {
                self.registers.c = self.srl(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x3A => {
                self.registers.d = self.srl(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x3B => {
                self.registers.e = self.srl(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x3C => {
                self.registers.h = self.srl(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x3D => {
                self.registers.l = self.srl(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x3E => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.srl(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x3F => {
                self.registers.a = self.srl(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            0x40 => {
                self.bit(self.registers.b, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x41 => {
                self.bit(self.registers.c, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x42 => {
                self.bit(self.registers.d, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x43 => {
                self.bit(self.registers.e, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x44 => {
                self.bit(self.registers.h, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x45 => {
                self.bit(self.registers.l, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x46 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 0);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x47 => {
                self.bit(self.registers.a, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x48 => {
                self.bit(self.registers.b, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x49 => {
                self.bit(self.registers.c, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x4A => {
                self.bit(self.registers.d, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x4B => {
                self.bit(self.registers.e, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x4C => {
                self.bit(self.registers.h, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x4D => {
                self.bit(self.registers.l, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x4E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 1);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x4F => {
                self.bit(self.registers.a, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x50 => {
                self.bit(self.registers.b, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x51 => {
                self.bit(self.registers.c, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x52 => {
                self.bit(self.registers.d, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x53 => {
                self.bit(self.registers.e, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x54 => {
                self.bit(self.registers.h, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x55 => {
                self.bit(self.registers.l, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x56 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 2);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x57 => {
                self.bit(self.registers.a, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x58 => {
                self.bit(self.registers.b, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x59 => {
                self.bit(self.registers.c, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x5A => {
                self.bit(self.registers.d, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x5B => {
                self.bit(self.registers.e, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x5C => {
                self.bit(self.registers.h, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x5D => {
                self.bit(self.registers.l, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x5E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 3);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x5F => {
                self.bit(self.registers.a, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x60 => {
                self.bit(self.registers.b, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x61 => {
                self.bit(self.registers.c, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x62 => {
                self.bit(self.registers.d, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x63 => {
                self.bit(self.registers.e, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x64 => {
                self.bit(self.registers.h, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x65 => {
                self.bit(self.registers.l, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x66 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 4);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x67 => {
                self.bit(self.registers.a, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x68 => {
                self.bit(self.registers.b, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x69 => {
                self.bit(self.registers.c, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x6A => {
                self.bit(self.registers.d, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x6B => {
                self.bit(self.registers.e, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x6C => {
                self.bit(self.registers.h, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x6D => {
                self.bit(self.registers.l, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x6E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 5);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x6F => {
                self.bit(self.registers.a, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x70 => {
                self.bit(self.registers.b, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x71 => {
                self.bit(self.registers.c, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x72 => {
                self.bit(self.registers.d, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x73 => {
                self.bit(self.registers.e, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x74 => {
                self.bit(self.registers.h, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x75 => {
                self.bit(self.registers.l, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x76 => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 6);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x77 => {
                self.bit(self.registers.a, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x78 => {
                self.bit(self.registers.b, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x79 => {
                self.bit(self.registers.c, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x7A => {
                self.bit(self.registers.d, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x7B => {
                self.bit(self.registers.e, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x7C => {
                self.bit(self.registers.h, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x7D => {
                self.bit(self.registers.l, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x7E => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 7);
                Some((self.pc.wrapping_add(2), 12))
            }
            0x7F => {
                self.bit(self.registers.a, 7);
                Some((self.pc.wrapping_add(2), 8))
            }

            0x80 => {
                self.registers.b = self.reset(self.registers.b, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x81 => {
                self.registers.c = self.reset(self.registers.c, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x82 => {
                self.registers.d = self.reset(self.registers.d, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x83 => {
                self.registers.e = self.reset(self.registers.e, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x84 => {
                self.registers.h = self.reset(self.registers.h, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x85 => {
                self.registers.l = self.reset(self.registers.l, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x86 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 0);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x87 => {
                self.registers.a = self.reset(self.registers.a, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x88 => {
                self.registers.b = self.reset(self.registers.b, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x89 => {
                self.registers.c = self.reset(self.registers.c, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x8A => {
                self.registers.d = self.reset(self.registers.d, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x8B => {
                self.registers.e = self.reset(self.registers.e, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x8C => {
                self.registers.h = self.reset(self.registers.h, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x8D => {
                self.registers.l = self.reset(self.registers.l, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x8E => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 1);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x8F => {
                self.registers.a = self.reset(self.registers.a, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x90 => {
                self.registers.b = self.reset(self.registers.b, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x91 => {
                self.registers.c = self.reset(self.registers.c, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x92 => {
                self.registers.d = self.reset(self.registers.d, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x93 => {
                self.registers.e = self.reset(self.registers.e, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x94 => {
                self.registers.h = self.reset(self.registers.h, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x95 => {
                self.registers.l = self.reset(self.registers.l, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x96 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 2);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x97 => {
                self.registers.a = self.reset(self.registers.a, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x98 => {
                self.registers.b = self.reset(self.registers.b, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x99 => {
                self.registers.c = self.reset(self.registers.c, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x9A => {
                self.registers.d = self.reset(self.registers.d, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x9B => {
                self.registers.e = self.reset(self.registers.e, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x9C => {
                self.registers.h = self.reset(self.registers.h, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x9D => {
                self.registers.l = self.reset(self.registers.l, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0x9E => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 3);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0x9F => {
                self.registers.a = self.reset(self.registers.a, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA0 => {
                self.registers.b = self.reset(self.registers.b, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA1 => {
                self.registers.c = self.reset(self.registers.c, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA2 => {
                self.registers.d = self.reset(self.registers.d, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA3 => {
                self.registers.e = self.reset(self.registers.e, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA4 => {
                self.registers.h = self.reset(self.registers.h, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA5 => {
                self.registers.l = self.reset(self.registers.l, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA6 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 4);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xA7 => {
                self.registers.a = self.reset(self.registers.a, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA8 => {
                self.registers.b = self.reset(self.registers.b, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xA9 => {
                self.registers.c = self.reset(self.registers.c, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xAA => {
                self.registers.d = self.reset(self.registers.d, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xAB => {
                self.registers.e = self.reset(self.registers.e, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xAC => {
                self.registers.h = self.reset(self.registers.h, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xAD => {
                self.registers.l = self.reset(self.registers.l, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xAE => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 5);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xAF => {
                self.registers.a = self.reset(self.registers.a, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB0 => {
                self.registers.b = self.reset(self.registers.b, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB1 => {
                self.registers.c = self.reset(self.registers.c, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB2 => {
                self.registers.d = self.reset(self.registers.d, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB3 => {
                self.registers.e = self.reset(self.registers.e, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB4 => {
                self.registers.h = self.reset(self.registers.h, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB5 => {
                self.registers.l = self.reset(self.registers.l, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB6 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 6);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xB7 => {
                self.registers.a = self.reset(self.registers.a, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB8 => {
                self.registers.b = self.reset(self.registers.b, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xB9 => {
                self.registers.c = self.reset(self.registers.c, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xBA => {
                self.registers.d = self.reset(self.registers.d, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xBB => {
                self.registers.e = self.reset(self.registers.e, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xBC => {
                self.registers.h = self.reset(self.registers.h, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xBD => {
                self.registers.l = self.reset(self.registers.l, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xBE => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 7);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xBF => {
                self.registers.a = self.reset(self.registers.a, 7);
                Some((self.pc.wrapping_add(2), 8))
            }

            0xC0 => {
                self.registers.b = self.set(self.registers.b, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC1 => {
                self.registers.c = self.set(self.registers.c, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC2 => {
                self.registers.d = self.set(self.registers.d, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC3 => {
                self.registers.e = self.set(self.registers.e, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC4 => {
                self.registers.h = self.set(self.registers.h, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC5 => {
                self.registers.l = self.set(self.registers.l, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC6 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 0);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xC7 => {
                self.registers.a = self.set(self.registers.a, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC8 => {
                self.registers.b = self.set(self.registers.b, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xC9 => {
                self.registers.c = self.set(self.registers.c, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xCA => {
                self.registers.d = self.set(self.registers.d, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xCB => {
                self.registers.e = self.set(self.registers.e, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xCC => {
                self.registers.h = self.set(self.registers.h, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xCD => {
                self.registers.l = self.set(self.registers.l, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xCE => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 1);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xCF => {
                self.registers.a = self.set(self.registers.a, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD0 => {
                self.registers.b = self.set(self.registers.b, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD1 => {
                self.registers.c = self.set(self.registers.c, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD2 => {
                self.registers.d = self.set(self.registers.d, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD3 => {
                self.registers.e = self.set(self.registers.e, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD4 => {
                self.registers.h = self.set(self.registers.h, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD5 => {
                self.registers.l = self.set(self.registers.l, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD6 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 2);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xD7 => {
                self.registers.a = self.set(self.registers.a, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD8 => {
                self.registers.b = self.set(self.registers.b, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xD9 => {
                self.registers.c = self.set(self.registers.c, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xDA => {
                self.registers.d = self.set(self.registers.d, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xDB => {
                self.registers.e = self.set(self.registers.e, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xDC => {
                self.registers.h = self.set(self.registers.h, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xDD => {
                self.registers.l = self.set(self.registers.l, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xDE => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 3);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xDF => {
                self.registers.a = self.set(self.registers.a, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE0 => {
                self.registers.b = self.set(self.registers.b, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE1 => {
                self.registers.c = self.set(self.registers.c, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE2 => {
                self.registers.d = self.set(self.registers.d, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE3 => {
                self.registers.e = self.set(self.registers.e, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE4 => {
                self.registers.h = self.set(self.registers.h, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE5 => {
                self.registers.l = self.set(self.registers.l, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE6 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 4);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xE7 => {
                self.registers.a = self.set(self.registers.a, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE8 => {
                self.registers.b = self.set(self.registers.b, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xE9 => {
                self.registers.c = self.set(self.registers.c, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xEA => {
                self.registers.d = self.set(self.registers.d, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xEB => {
                self.registers.e = self.set(self.registers.e, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xEC => {
                self.registers.h = self.set(self.registers.h, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xED => {
                self.registers.l = self.set(self.registers.l, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xEE => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 5);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xEF => {
                self.registers.a = self.set(self.registers.a, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF0 => {
                self.registers.b = self.set(self.registers.b, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF1 => {
                self.registers.c = self.set(self.registers.c, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF2 => {
                self.registers.d = self.set(self.registers.d, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF3 => {
                self.registers.e = self.set(self.registers.e, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF4 => {
                self.registers.h = self.set(self.registers.h, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF5 => {
                self.registers.l = self.set(self.registers.l, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF6 => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 6);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xF7 => {
                self.registers.a = self.set(self.registers.a, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF8 => {
                self.registers.b = self.set(self.registers.b, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xF9 => {
                self.registers.c = self.set(self.registers.c, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xFA => {
                self.registers.d = self.set(self.registers.d, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xFB => {
                self.registers.e = self.set(self.registers.e, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xFC => {
                self.registers.h = self.set(self.registers.h, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xFD => {
                self.registers.l = self.set(self.registers.l, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            0xFE => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 7);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            0xFF => {
                self.registers.a = self.set(self.registers.a, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
        }
    }

    fn check_interrupts(&mut self) -> u8 {
        if !self.ime {
            if self.is_halted && self.bus.check_interrupts(false).is_some() {
                self.is_halted = false;
                // Halt bug handling
                // self.pc = self.pc.wrapping_add(1);
                return 4;
            }
            return 0;
        }

        let check_result = self.bus.check_interrupts(true);
        match check_result {
            Some(isr_addr) => {
                self.is_halted = false;
                self.ime = false;
                self.push(self.pc);
                self.pc = isr_addr;

                20
            }
            None => 0,
        }
    }

    pub fn step(&mut self) -> u8 {
        if self.ime_delayed {
            self.ime = true;
            self.ime_delayed = false;
        }

        if self.is_halted {
            let cycles = 4 + self.check_interrupts();
            self.bus.step_peripherals(cycles);
            return cycles;
        }

        let (next_pc, cycles) = match self.read_byte(self.pc) {
            INSTRUCTION_PREFIX => {
                let byte = self.read_byte(self.pc + 1);
                match self.execute_prefixed(byte) {
                    Some((next_pc, cycles)) => (next_pc, cycles),
                    None => (self.pc.wrapping_add(1), 4),
                }
            }
            byte => match self.execute(byte) {
                Some((next_pc, cycles)) => (next_pc, cycles),
                None => (self.pc.wrapping_add(1), 4),
            },
        };
        self.pc = next_pc;

        let cycles = cycles + self.check_interrupts();

        if cycles > self.cycles_synced {
            self.bus.step_peripherals(cycles - self.cycles_synced);
        }
        self.cycles_synced = 0;

        cycles
    }

    fn call(&mut self, jump: bool) -> (u16, u8) {
        let next_pc = self.pc.wrapping_add(3);
        if jump {
            self.push(next_pc);
            (self.read_two_bytes(self.pc.wrapping_add(1)), 24)
        } else {
            (next_pc, 12)
        }
    }

    fn ret(&mut self, jump: bool) -> (u16, u8) {
        if jump {
            (self.pop(), 20)
        } else {
            (self.pc.wrapping_add(1), 8)
        }
    }

    fn push(&mut self, value: u16) {
        self.sp = self.sp.wrapping_sub(2);
        self.write_two_bytes(self.sp, value);
    }

    fn pop(&mut self) -> u16 {
        let val = self.read_two_bytes(self.sp);
        self.sp = self.sp.wrapping_add(2);
        val
    }

    fn jp(&mut self, jump: bool) -> (u16, u8) {
        if jump {
            (self.read_two_bytes(self.pc.wrapping_add(1)), 16)
        } else {
            (self.pc.wrapping_add(3), 12)
        }
    }

    fn jr(&mut self, jump: bool) -> (u16, u8) {
        let mut pc = self.pc.wrapping_add(2);
        let mut cycles = 8;
        if jump {
            let i8_byte = self.read_byte(self.pc.wrapping_add(1)) as i8;
            pc = add_u16_i8(pc, i8_byte);
            cycles = 12;
        }
        (pc, cycles)
    }

    fn daa(&mut self) {
        let (has_carry, has_half_carry) = (self.registers.f.carry, self.registers.f.half_carry);

        self.registers.f.carry = false;

        let mut offset: u8 = 0;
        if (!self.registers.f.subtract && self.registers.a & 0xF > 0x09) || has_half_carry {
            offset |= 0x06;
        }
        if (!self.registers.f.subtract && self.registers.a > 0x99) || has_carry {
            offset |= 0x60;
            self.registers.f.carry = true;
        }

        self.registers.a = if self.registers.f.subtract {
            self.registers.a.wrapping_sub(offset)
        } else {
            self.registers.a.wrapping_add(offset)
        };

        self.registers.f.half_carry = false;
        self.registers.f.zero = self.registers.a == 0;
    }

    fn add(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_add(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = half_carry_add_8bits(self.registers.a, value);
        self.registers.a = new_value;
    }

    fn addhl(&mut self, value: u16) {
        let hl_reg_val = self.registers.get_hl();
        let (new_value, did_overflow) = hl_reg_val.overflowing_add(value);
        self.registers.f.subtract = false;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = half_carry_add_16bits(hl_reg_val, value);
        self.registers.set_hl(new_value);
    }

    fn adc(&mut self, value: u8) {
        let new_value =
            (self.registers.a as u16) + (value as u16) + (self.registers.f.carry as u16);
        self.registers.f.zero = new_value as u8 == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = half_carry_add_8bits(self.registers.a, value)
            || half_carry_add_8bits(
                (self.registers.a & 0xF) + (value & 0xF),
                self.registers.f.carry as u8,
            );
        self.registers.f.carry = new_value > 0xFF;
        self.registers.a = new_value as u8;
    }

    fn sub(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = half_carry_sub_8bits(self.registers.a, value);
        self.registers.a = new_value;
    }

    fn sbc(&mut self, value: u8) {
        let new_value =
            (self.registers.a as i16) - (value as i16) - (self.registers.f.carry as i16);
        self.registers.f.zero = new_value as u8 == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry =
            half_carry_sub_with_carry_8bits(self.registers.a, value, self.registers.f.carry);
        self.registers.f.carry = new_value < 0;
        self.registers.a = new_value as u8;
    }

    fn and(&mut self, value: u8) {
        let new_value = self.registers.a & value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = true;
        self.registers.f.carry = false;
        self.registers.a = new_value;
    }

    fn or(&mut self, value: u8) {
        let new_value = self.registers.a | value;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;
        self.registers.a = new_value;
    }

    fn xor(&mut self, value: u8) {
        self.registers.a ^= value;
        self.registers.f.zero = self.registers.a == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;
    }

    fn cp(&mut self, value: u8) {
        let (new_value, did_overflow) = self.registers.a.overflowing_sub(value);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.carry = did_overflow;
        self.registers.f.half_carry = half_carry_sub_8bits(self.registers.a, value);
    }

    fn inc(&mut self, value: u8) -> u8 {
        let (new_value, _) = value.overflowing_add(1);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = half_carry_add_8bits(value, 1);
        new_value
    }

    fn inc_16bits(&mut self, value: u16) -> u16 {
        let (new_value, _) = value.overflowing_add(1);
        new_value
    }

    fn dec(&mut self, value: u8) -> u8 {
        let (new_value, _) = value.overflowing_sub(1);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = half_carry_sub_8bits(value, 1);
        new_value
    }

    fn dec_16bits(&mut self, value: u16) -> u16 {
        let (new_value, _) = value.overflowing_sub(1);
        new_value
    }

    fn ccf(&mut self) {
        self.registers.f.carry = !self.registers.f.carry;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
    }

    fn scf(&mut self) {
        self.registers.f.carry = true;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
    }

    fn rra(&mut self) {
        (self.registers.a, self.registers.f.carry) =
            right_rotate_through_carry(self.registers.a, self.registers.f.carry);
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
    }

    fn rla(&mut self) {
        (self.registers.a, self.registers.f.carry) =
            left_rotate_through_carry(self.registers.a, self.registers.f.carry);
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
    }

    fn rrca(&mut self) {
        self.registers.a = self.registers.a.rotate_right(1);
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = self.registers.a >> 7 == 1;
    }

    fn rlca(&mut self) {
        let carry_out = self.registers.a >> 7;
        self.registers.a = self.registers.a.rotate_left(1);
        self.registers.f.zero = false;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = carry_out == 1;
    }

    fn cpl(&mut self) {
        self.registers.a = !self.registers.a;
        self.registers.f.subtract = true;
        self.registers.f.half_carry = true;
    }

    fn bit(&mut self, value: u8, pos: u8) {
        self.registers.f.zero = value & (1 << pos) == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = true;
    }

    fn reset(&mut self, value: u8, pos: u8) -> u8 {
        value & (0xFF & !(1 << pos))
    }

    fn set(&mut self, value: u8, pos: u8) -> u8 {
        value | (1 << pos)
    }

    fn srl(&mut self, value: u8) -> u8 {
        let new_value = value >> 1;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = value & 1 == 1;
        new_value
    }

    fn rr(&mut self, value: u8) -> u8 {
        let (new_value, has_carry) = right_rotate_through_carry(value, self.registers.f.carry);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = has_carry;
        new_value
    }

    fn rl(&mut self, value: u8) -> u8 {
        let (new_value, has_carry) = left_rotate_through_carry(value, self.registers.f.carry);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = has_carry;
        new_value
    }

    fn rrc(&mut self, value: u8) -> u8 {
        let new_value = value.rotate_right(1);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = new_value >> 7 == 1;
        new_value
    }

    fn rlc(&mut self, value: u8) -> u8 {
        let new_value = value.rotate_left(1);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = new_value << 7 == 0b10000000;
        new_value
    }

    fn sra(&mut self, value: u8) -> u8 {
        let new_value = value >> 1 | (value & 0b10000000);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = value & 1 == 1;
        new_value
    }

    fn sla(&mut self, value: u8) -> u8 {
        let new_value = value << 1;
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = value & 0b10000000 == 0b10000000;
        new_value
    }

    fn swap(&mut self, value: u8) -> u8 {
        let new_value = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);
        self.registers.f.zero = new_value == 0;
        self.registers.f.subtract = false;
        self.registers.f.half_carry = false;
        self.registers.f.carry = false;
        new_value
    }
}

fn half_carry_add_8bits(x: u8, y: u8) -> bool {
    (x & 0xF) + (y & 0xF) > 0xF
}

fn half_carry_add_16bits(x: u16, y: u16) -> bool {
    (x & 0xFFF) + (y & 0xFFF) > 0xFFF
}

fn half_carry_sub_8bits(x: u8, y: u8) -> bool {
    (x & 0xF).overflowing_sub(y & 0xF).1
}

fn half_carry_sub_with_carry_8bits(x: u8, y: u8, carry: bool) -> bool {
    ((x & 0xF) as i16) - ((y & 0xF) as i16) - (carry as i16) < 0
}

fn right_rotate_through_carry(value: u8, carry: bool) -> (u8, bool) {
    (value >> 1 | (carry as u8) << 7, value & 1 == 1)
}

fn left_rotate_through_carry(value: u8, carry: bool) -> (u8, bool) {
    (value << 1 | carry as u8, value & 0b10000000 == 0b10000000)
}

fn add_u16_i8(x: u16, y: i8) -> u16 {
    ((x as i16).wrapping_add(y as i16)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct FakeBus {
        mem: [u8; 0x10000],
    }

    impl FakeBus {
        pub fn new() -> Self {
            Self { mem: [0; 0x10000] }
        }
    }

    impl Bus for FakeBus {
        fn read_byte(&self, address: u16) -> u8 {
            self.mem[address as usize]
        }

        fn write_byte(&mut self, address: u16, value: u8) {
            self.mem[address as usize] = value;
        }

        fn check_interrupts(&mut self, _reset_flag: bool) -> Option<u16> {
            None
        }

        fn step_peripherals(&mut self, _cycles: u8) {}
    }

    fn make_test_cpu() -> CPU<FakeBus> {
        CPU::new(
            &Config {
                rom: vec![],
                headless_mode: false,
                bootrom: Some(vec![]),
                log_file_path: None,
            },
            FakeBus::new(),
        )
    }

    #[test]
    fn test_cpu_add_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b1;

        cpu.add(0b101);

        assert_eq!(0b110, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_add_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b01111111;

        cpu.add(0b11000000);

        assert_eq!(0b00111111, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_add_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00000111;

        cpu.add(0b00001011);

        assert_eq!(0b00010010, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_addhl_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.set_hl(0x1FF);
        cpu.registers.f.zero = true;

        cpu.addhl(0x001);

        assert_eq!(0x1FF + 0x001, cpu.registers.get_hl());
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(true, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_addhl_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.set_hl(0xEFFF);

        cpu.addhl(0xF000);

        assert_eq!(57343, cpu.registers.get_hl());
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_addhl_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.set_hl(0xEFF);

        cpu.addhl(0xF00);

        assert_eq!(0xEFF + 0xF00, cpu.registers.get_hl());
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_adc_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b1;
        cpu.registers.f.carry = true;

        cpu.adc(0b10);

        assert_eq!(0b100, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_adc_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b1;

        cpu.adc(0b10);

        assert_eq!(0b11, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_adc_carry_from_value() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b01111111;

        cpu.adc(0b11000000);

        assert_eq!(0b00111111, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_adc_carry_from_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0xFE;
        cpu.registers.f.carry = true;

        cpu.adc(0b1);

        assert_eq!(0, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(true, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_adc_half_carry_from_value() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0xF;
        cpu.registers.f.carry = true;

        cpu.adc(0b1);

        assert_eq!(0x11, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_adc_half_carry_from_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0xE;
        cpu.registers.f.carry = true;

        cpu.adc(0b1);

        assert_eq!(0x10, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_sub_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b11;

        cpu.sub(0b1);

        assert_eq!(0b10, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sub_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0xE0;

        cpu.sub(0xF0);

        assert_eq!(240, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sub_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x10;

        cpu.sub(0b10);

        assert_eq!(14, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sub_carry_and_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b1;

        cpu.sub(0b11);

        assert_eq!(254, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_sbc_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b100;
        cpu.registers.f.carry = true;

        cpu.sbc(0b1);

        assert_eq!(0b10, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sbc_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b100;

        cpu.sbc(0b1);

        assert_eq!(0b11, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sbc_carry_from_value() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0xE0;
        cpu.registers.f.carry = true;

        cpu.sbc(0xF0);

        assert_eq!(239, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sbc_carry_from_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0xF0;
        cpu.registers.f.carry = true;

        cpu.sbc(0xF0);

        assert_eq!(255, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sbc_half_carry_from_value() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b1;
        cpu.registers.f.carry = true;

        cpu.sbc(0b11);

        assert_eq!(253, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sbc_half_carry_from_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b11;
        cpu.registers.f.carry = true;

        cpu.sbc(0b11);

        assert_eq!(255, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_and_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b101;

        cpu.and(0b1);

        assert_eq!(0b1, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_or_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b101;

        cpu.or(0b11);

        assert_eq!(0b111, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_xor_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b101;

        cpu.xor(0b11);

        assert_eq!(0b110, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_inc_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b110;

        let val = cpu.inc(cpu.registers.b);

        assert_eq!(0b111, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_inc_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0xF;

        let val = cpu.inc(cpu.registers.b);

        assert_eq!(0x10, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_dec_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b111;

        let val = cpu.dec(cpu.registers.b);

        assert_eq!(0b110, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_dec_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0;

        let val = cpu.dec(cpu.registers.b);

        assert_eq!(255, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_ccf_toggle_on() {
        let mut cpu = make_test_cpu();
        cpu.registers.f.carry = false;

        cpu.ccf();

        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
    }
    #[test]
    fn test_cpu_ccf_toggle_off() {
        let mut cpu = make_test_cpu();
        cpu.registers.f.carry = true;

        cpu.ccf();

        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
    }

    #[test]
    fn test_cpu_scf_start_with_true() {
        let mut cpu = make_test_cpu();
        cpu.registers.f.carry = true;

        cpu.scf();

        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
    }
    #[test]
    fn test_cpu_scf_start_with_false() {
        let mut cpu = make_test_cpu();
        cpu.registers.f.carry = false;

        cpu.scf();

        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
    }

    #[test]
    fn test_cpu_rra_with_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001001;
        cpu.registers.f.carry = true;

        cpu.rra();

        assert_eq!(0b10000100, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rra_with_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001000;
        cpu.registers.f.carry = true;

        cpu.rra();

        assert_eq!(0b10000100, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rra_without_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001001;
        cpu.registers.f.carry = false;

        cpu.rra();

        assert_eq!(0b00000100, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rra_without_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001000;
        cpu.registers.f.carry = false;

        cpu.rra();

        assert_eq!(0b00000100, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_rla_with_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b10001000;
        cpu.registers.f.carry = true;

        cpu.rla();

        assert_eq!(0b00010001, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rla_with_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001000;
        cpu.registers.f.carry = true;

        cpu.rla();

        assert_eq!(0b00010001, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rla_without_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b10001000;
        cpu.registers.f.carry = false;

        cpu.rla();

        assert_eq!(0b00010000, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rla_without_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001000;
        cpu.registers.f.carry = false;

        cpu.rla();

        assert_eq!(0b00010000, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_rrca_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001001;
        cpu.registers.f.carry = false;

        cpu.rrca();

        assert_eq!(0b10000100, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rrca_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001000;
        cpu.registers.f.carry = false;

        cpu.rrca();

        assert_eq!(0b00000100, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_rlca_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b10001000;
        cpu.registers.f.carry = false;

        cpu.rlca();

        assert_eq!(0b00010001, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rlca_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b00001000;
        cpu.registers.f.carry = false;

        cpu.rlca();

        assert_eq!(0b00010000, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_cpl_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0b01001000;

        cpu.cpl();

        assert_eq!(0b10110111, cpu.registers.a);
        assert_eq!(true, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.subtract);
    }

    #[test]
    fn test_cpu_bit_is_set() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b01001000;

        cpu.bit(cpu.registers.b, 3);

        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(true, cpu.registers.f.half_carry);
    }
    #[test]
    fn test_cpu_bit_is_not_set() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b01001000;

        cpu.bit(cpu.registers.b, 4);

        assert_eq!(true, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(true, cpu.registers.f.half_carry);
    }

    #[test]
    fn test_cpu_reset_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b01001010;

        let val = cpu.reset(cpu.registers.b, 3);

        assert_eq!(0b01000010, val);
    }

    #[test]
    fn test_cpu_set_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b01000010;

        let val = cpu.set(cpu.registers.b, 3);

        assert_eq!(0b01001010, val);
    }

    #[test]
    fn test_cpu_srl_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b01000010;

        let val = cpu.srl(cpu.registers.b);

        assert_eq!(0b00100001, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_srl_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b01000011;

        let val = cpu.srl(cpu.registers.b);

        assert_eq!(0b00100001, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_rr_with_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00001001;
        cpu.registers.f.carry = true;

        let val = cpu.rr(cpu.registers.b);

        assert_eq!(0b10000100, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rr_with_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00001010;
        cpu.registers.f.carry = true;

        let val = cpu.rr(cpu.registers.b);

        assert_eq!(0b10000101, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rr_without_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00001001;
        cpu.registers.f.carry = false;

        let val = cpu.rr(cpu.registers.b);

        assert_eq!(0b00000100, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rr_without_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00001010;
        cpu.registers.f.carry = false;

        let val = cpu.rr(cpu.registers.b);

        assert_eq!(0b00000101, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_rl_with_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b10001000;
        cpu.registers.f.carry = true;

        let val = cpu.rl(cpu.registers.b);

        assert_eq!(0b00010001, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rl_with_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00101000;
        cpu.registers.f.carry = true;

        let val = cpu.rl(cpu.registers.b);

        assert_eq!(0b01010001, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rl_without_carry_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b10101000;
        cpu.registers.f.carry = false;

        let val = cpu.rl(cpu.registers.b);

        assert_eq!(0b01010000, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rl_without_carry_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00101000;
        cpu.registers.f.carry = false;

        let val = cpu.rl(cpu.registers.b);

        assert_eq!(0b01010000, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_rrc_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00001001;
        cpu.registers.f.carry = false;

        let val = cpu.rrc(cpu.registers.b);

        assert_eq!(0b10000100, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rrc_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00001000;
        cpu.registers.f.carry = false;

        let val = cpu.rrc(cpu.registers.b);

        assert_eq!(0b00000100, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_rlc_resulting_with_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b10001000;
        cpu.registers.f.carry = false;

        let val = cpu.rlc(cpu.registers.b);

        assert_eq!(0b00010001, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_rlc_resulting_with_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00001000;
        cpu.registers.f.carry = false;

        let val = cpu.rlc(cpu.registers.b);

        assert_eq!(0b00010000, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_sra_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b10001001;

        let val = cpu.sra(cpu.registers.b);

        assert_eq!(0b11000100, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sra_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b10001000;

        let val = cpu.sra(cpu.registers.b);

        assert_eq!(0b11000100, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_sla_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b10001010;

        let val = cpu.sla(cpu.registers.b);

        assert_eq!(0b00010100, val);
        assert_eq!(true, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }
    #[test]
    fn test_cpu_sla_no_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00101010;

        let val = cpu.sla(cpu.registers.b);

        assert_eq!(0b01010100, val);
        assert_eq!(false, cpu.registers.f.carry);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.zero);
    }

    #[test]
    fn test_cpu_swap_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.b = 0b00101010;

        let val = cpu.swap(cpu.registers.b);

        assert_eq!(0b10100010, val);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.subtract);
        assert_eq!(false, cpu.registers.f.carry);
    }

    #[test]
    fn test_cpu_daa_add_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x77;

        cpu.daa();

        assert_eq!(0x77, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.carry);
    }
    #[test]
    fn test_cpu_daa_add_half_carrying() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x6B;

        cpu.daa();

        assert_eq!(0x71, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.carry);
    }
    #[test]
    fn test_cpu_daa_add_carrying() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x9C;

        cpu.daa();

        assert_eq!(0x02, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.carry);
    }
    #[test]
    fn test_cpu_daa_add_having_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x10;
        cpu.registers.f.carry = true;

        cpu.daa();

        assert_eq!(0x70, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.carry);
    }
    #[test]
    fn test_cpu_daa_add_having_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x11;
        cpu.registers.f.half_carry = true;

        cpu.daa();

        assert_eq!(0x17, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.carry);
    }
    #[test]
    fn test_cpu_daa_sub_nominal() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x55;
        cpu.registers.f.subtract = true;

        cpu.daa();

        assert_eq!(0x55, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.carry);
    }
    #[test]
    fn test_cpu_daa_sub_having_half_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0x0D;
        cpu.registers.f.subtract = true;
        cpu.registers.f.half_carry = true;

        cpu.daa();

        assert_eq!(0x07, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(false, cpu.registers.f.carry);
    }
    #[test]
    fn test_cpu_daa_sub_having_carry() {
        let mut cpu = make_test_cpu();
        cpu.registers.a = 0xE4;
        cpu.registers.f.subtract = true;
        cpu.registers.f.carry = true;

        cpu.daa();

        assert_eq!(0x84, cpu.registers.a);
        assert_eq!(false, cpu.registers.f.zero);
        assert_eq!(false, cpu.registers.f.half_carry);
        assert_eq!(true, cpu.registers.f.carry);
    }

    #[test]
    fn test_cpu_push_nominal() {
        let mut cpu = make_test_cpu();
        cpu.sp = 128;

        cpu.push(0xEEAA);

        assert_eq!(126, cpu.sp);
        assert_eq!(0xAA, cpu.bus.read_byte(cpu.sp));
        assert_eq!(0xEE, cpu.bus.read_byte(cpu.sp + 1));
    }

    #[test]
    fn test_cpu_pop_nominal() {
        let mut cpu = make_test_cpu();
        cpu.sp = 126;

        cpu.write_two_bytes(126, 0xEEAA);

        let val = cpu.pop();

        assert_eq!(128, cpu.sp);
        assert_eq!(0xEEAA, val);
    }
}
