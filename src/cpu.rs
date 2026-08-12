use crate::{
    bus::Bus,
    config::Config,
    instr::{cb, op},
    mode::Mode,
    registers,
};

pub struct CPU<B: Bus> {
    mode: Mode,

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
            mode: cfg.mode.clone(),

            is_halted: false,
            is_stopped: false,
            ime: false,
            ime_delayed: false,

            registers: if skip_boot {
                registers::Registers::new_post_boot(cfg.mode.clone())
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
        self.bus.step_peripherals(4, false);
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
        self.bus.step_peripherals(4, false);
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
            op::NOP => Some((self.pc.wrapping_add(1), 4)),
            op::STOP => {
                self.is_stopped = true;
                match self.mode {
                    Mode::CGB => self.bus.switch_speed(),
                    _ => {}
                }
                Some((self.pc.wrapping_add(1), 4))
            }
            op::INC_BC => {
                let val = self.inc_16bits(self.registers.get_bc());
                self.registers.set_bc(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::INC_DE => {
                let val = self.inc_16bits(self.registers.get_de());
                self.registers.set_de(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::INC_HL => {
                let val = self.inc_16bits(self.registers.get_hl());
                self.registers.set_hl(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::INC_SP => {
                self.sp = self.inc_16bits(self.sp);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::INC_B => {
                self.registers.b = self.inc(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::INC_D => {
                self.registers.d = self.inc(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::INC_H => {
                self.registers.h = self.inc(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::INC_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let inc_val = self.inc(mem_val);
                self.write_byte(hl_reg_val, inc_val);
                Some((self.pc.wrapping_add(1), 12))
            }
            op::DEC_B => {
                self.registers.b = self.dec(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DEC_D => {
                self.registers.d = self.dec(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DEC_H => {
                self.registers.h = self.dec(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DEC_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let inc_val = self.dec(mem_val);
                self.write_byte(hl_reg_val, inc_val);
                Some((self.pc.wrapping_add(1), 12))
            }
            op::RLCA => {
                self.rlca();
                Some((self.pc.wrapping_add(1), 4))
            }
            op::RLA => {
                self.rla();
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DAA => {
                self.daa();
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SCF => {
                self.scf();
                Some((self.pc.wrapping_add(1), 4))
            }

            op::ADD_HL_BC => {
                self.addhl(self.registers.get_bc());
                Some((self.pc.wrapping_add(1), 8))
            }
            op::ADD_HL_DE => {
                self.addhl(self.registers.get_de());
                Some((self.pc.wrapping_add(1), 8))
            }
            op::ADD_HL_HL => {
                self.addhl(self.registers.get_hl());
                Some((self.pc.wrapping_add(1), 8))
            }
            op::ADD_HL_SP => {
                self.addhl(self.sp);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::DEC_BC => {
                let val = self.dec_16bits(self.registers.get_bc());
                self.registers.set_bc(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::DEC_DE => {
                let val = self.dec_16bits(self.registers.get_de());
                self.registers.set_de(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::DEC_HL => {
                let val = self.dec_16bits(self.registers.get_hl());
                self.registers.set_hl(val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::DEC_SP => {
                self.sp = self.dec_16bits(self.sp);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::INC_C => {
                self.registers.c = self.inc(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::INC_E => {
                self.registers.e = self.inc(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::INC_L => {
                self.registers.l = self.inc(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::INC_A => {
                self.registers.a = self.inc(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DEC_C => {
                self.registers.c = self.dec(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DEC_E => {
                self.registers.e = self.dec(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DEC_L => {
                self.registers.l = self.dec(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::DEC_A => {
                self.registers.a = self.dec(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::RRCA => {
                self.rrca();
                Some((self.pc.wrapping_add(1), 4))
            }
            op::RRA => {
                self.rra();
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CPL => {
                self.cpl();
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CCF => {
                self.ccf();
                Some((self.pc.wrapping_add(1), 4))
            }

            op::ADD_A_B => {
                self.add(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADD_A_C => {
                self.add(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADD_A_D => {
                self.add(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADD_A_E => {
                self.add(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADD_A_H => {
                self.add(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADD_A_L => {
                self.add(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADD_A_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.add(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::ADD_A_A => {
                self.add(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADC_A_B => {
                self.adc(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADC_A_C => {
                self.adc(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADC_A_D => {
                self.adc(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADC_A_E => {
                self.adc(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADC_A_H => {
                self.adc(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADC_A_L => {
                self.adc(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::ADC_A_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.adc(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::ADC_A_A => {
                self.adc(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            op::SUB_B => {
                self.sub(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SUB_C => {
                self.sub(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SUB_D => {
                self.sub(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SUB_E => {
                self.sub(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SUB_H => {
                self.sub(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SUB_L => {
                self.sub(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SUB_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.sub(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::SUB_A => {
                self.sub(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SBC_A_B => {
                self.sbc(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SBC_A_C => {
                self.sbc(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SBC_A_D => {
                self.sbc(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SBC_A_E => {
                self.sbc(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SBC_A_H => {
                self.sbc(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SBC_A_L => {
                self.sbc(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::SBC_A_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.sbc(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::SBC_A_A => {
                self.sbc(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            op::AND_B => {
                self.and(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::AND_C => {
                self.and(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::AND_D => {
                self.and(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::AND_E => {
                self.and(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::AND_H => {
                self.and(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::AND_L => {
                self.and(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::AND_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.and(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::AND_A => {
                self.and(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::XOR_B => {
                self.xor(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::XOR_C => {
                self.xor(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::XOR_D => {
                self.xor(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::XOR_E => {
                self.xor(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::XOR_H => {
                self.xor(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::XOR_L => {
                self.xor(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::XOR_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.xor(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::XOR_A => {
                self.xor(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            op::OR_B => {
                self.or(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::OR_C => {
                self.or(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::OR_D => {
                self.or(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::OR_E => {
                self.or(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::OR_H => {
                self.or(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::OR_L => {
                self.or(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::OR_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.or(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::OR_A => {
                self.or(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CP_B => {
                self.cp(self.registers.b);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CP_C => {
                self.cp(self.registers.c);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CP_D => {
                self.cp(self.registers.d);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CP_E => {
                self.cp(self.registers.e);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CP_H => {
                self.cp(self.registers.h);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CP_L => {
                self.cp(self.registers.l);
                Some((self.pc.wrapping_add(1), 4))
            }
            op::CP_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.cp(mem_val);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::CP_A => {
                self.cp(self.registers.a);
                Some((self.pc.wrapping_add(1), 4))
            }

            op::ADD_A_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.add(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            op::SUB_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.sub(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            op::AND_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.and(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            op::OR_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.or(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }

            op::ADC_A_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.adc(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            op::SBC_A_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.sbc(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            op::XOR_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.xor(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }
            op::CP_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.cp(mem_val);
                Some((self.pc.wrapping_add(2), 8))
            }

            op::JR_NZ_R8 => Some(self.jr(!self.registers.f.zero)),
            op::JR_NC_R8 => Some(self.jr(!self.registers.f.carry)),
            op::JR_R8 => Some(self.jr(true)),
            op::JR_Z_R8 => Some(self.jr(self.registers.f.zero)),
            op::JR_C_R8 => Some(self.jr(self.registers.f.carry)),

            op::JP_NZ_A16 => Some(self.jp(!self.registers.f.zero)),
            op::JP_NC_A16 => Some(self.jp(!self.registers.f.carry)),
            op::JP_A16 => Some(self.jp(true)),
            op::JP_Z_A16 => Some(self.jp(self.registers.f.zero)),
            op::JP_C_A16 => Some(self.jp(self.registers.f.carry)),

            // LDs
            op::LD_BC_D16 => {
                let mem_val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.set_bc(mem_val);
                Some((self.pc.wrapping_add(3), 12))
            }
            op::LD_DE_D16 => {
                let mem_val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.set_de(mem_val);
                Some((self.pc.wrapping_add(3), 12))
            }
            op::LD_HL_D16 => {
                let mem_val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.set_hl(mem_val);
                Some((self.pc.wrapping_add(3), 12))
            }
            op::LD_SP_D16 => {
                self.sp = self.read_two_bytes(self.pc.wrapping_add(1));
                Some((self.pc.wrapping_add(3), 12))
            }
            op::LD_BC_IND_A => {
                self.write_byte(self.registers.get_bc(), self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_DE_IND_A => {
                self.write_byte(self.registers.get_de(), self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_HLI_A => {
                let hl_reg_val = self.registers.get_hl();
                self.write_byte(hl_reg_val, self.registers.a);
                self.registers.set_hl(hl_reg_val.wrapping_add(1));
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_HLD_A => {
                let hl_reg_val = self.registers.get_hl();
                self.write_byte(hl_reg_val, self.registers.a);
                self.registers.set_hl(hl_reg_val.wrapping_sub(1));
                Some((self.pc.wrapping_add(1), 8))
            }

            op::LD_B_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.b = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            op::LD_D_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.d = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            op::LD_H_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.h = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            op::LD_HL_IND_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.write_byte(self.registers.get_hl(), mem_val);
                Some((self.pc.wrapping_add(2), 12))
            }

            op::LD_A16_IND_SP => {
                let val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.write_two_bytes(val, self.sp);
                Some((self.pc.wrapping_add(3), 20))
            }

            op::LD_A_BC_IND => {
                self.registers.a = self.read_byte(self.registers.get_bc());
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_A_DE_IND => {
                self.registers.a = self.read_byte(self.registers.get_de());
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_A_HLI => {
                let hl_reg_val = self.registers.get_hl();
                self.registers.a = self.read_byte(hl_reg_val);
                self.registers.set_hl(hl_reg_val.wrapping_add(1));
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_A_HLD => {
                let hl_reg_val = self.registers.get_hl();
                self.registers.a = self.read_byte(hl_reg_val);
                self.registers.set_hl(hl_reg_val.wrapping_sub(1));
                Some((self.pc.wrapping_add(1), 8))
            }

            op::LD_C_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.c = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            op::LD_E_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.e = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            op::LD_L_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.l = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }
            op::LD_A_D8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.a = mem_val;
                Some((self.pc.wrapping_add(2), 8))
            }

            op::LD_B_B => {
                self.registers.b = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_B_C => {
                self.registers.b = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_B_D => {
                self.registers.b = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_B_E => {
                self.registers.b = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_B_H => {
                self.registers.b = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_B_L => {
                self.registers.b = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_B_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.b = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_B_A => {
                self.registers.b = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_C_B => {
                self.registers.c = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_C_C => {
                self.registers.c = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_C_D => {
                self.registers.c = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_C_E => {
                self.registers.c = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_C_H => {
                self.registers.c = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_C_L => {
                self.registers.c = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_C_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.c = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_C_A => {
                self.registers.c = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_D_B => {
                self.registers.d = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_D_C => {
                self.registers.d = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_D_D => {
                self.registers.d = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_D_E => {
                self.registers.d = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_D_H => {
                self.registers.d = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_D_L => {
                self.registers.d = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_D_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.d = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_D_A => {
                self.registers.d = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_E_B => {
                self.registers.e = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_E_C => {
                self.registers.e = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_E_D => {
                self.registers.e = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_E_E => {
                self.registers.e = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_E_H => {
                self.registers.e = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_E_L => {
                self.registers.e = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_E_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.e = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_E_A => {
                self.registers.e = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_H_B => {
                self.registers.h = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_H_C => {
                self.registers.h = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_H_D => {
                self.registers.h = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_H_E => {
                self.registers.h = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_H_H => {
                self.registers.h = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_H_L => {
                self.registers.h = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_H_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.h = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_H_A => {
                self.registers.h = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_L_B => {
                self.registers.l = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_L_C => {
                self.registers.l = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_L_D => {
                self.registers.l = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_L_E => {
                self.registers.l = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_L_H => {
                self.registers.l = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_L_L => {
                self.registers.l = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_L_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.l = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_L_A => {
                self.registers.l = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_HL_IND_B => {
                self.write_byte(self.registers.get_hl(), self.registers.b);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_HL_IND_C => {
                self.write_byte(self.registers.get_hl(), self.registers.c);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_HL_IND_D => {
                self.write_byte(self.registers.get_hl(), self.registers.d);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_HL_IND_E => {
                self.write_byte(self.registers.get_hl(), self.registers.e);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_HL_IND_H => {
                self.write_byte(self.registers.get_hl(), self.registers.h);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_HL_IND_L => {
                self.write_byte(self.registers.get_hl(), self.registers.l);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::HALT => {
                self.is_halted = true;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_HL_IND_A => {
                self.write_byte(self.registers.get_hl(), self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_A_B => {
                self.registers.a = self.registers.b;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_A_C => {
                self.registers.a = self.registers.c;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_A_D => {
                self.registers.a = self.registers.d;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_A_E => {
                self.registers.a = self.registers.e;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_A_H => {
                self.registers.a = self.registers.h;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_A_L => {
                self.registers.a = self.registers.l;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LD_A_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.registers.a = mem_val;
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_A_A => {
                self.registers.a = self.registers.a;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::LDH_A8_IND_A => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.write_byte(0xFF00 + mem_val as u16, self.registers.a);
                Some((self.pc.wrapping_add(2), 12))
            }
            op::LDH_A_IND_A8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                self.registers.a = self.read_byte(0xFF00 + mem_val as u16);
                Some((self.pc.wrapping_add(2), 12))
            }
            op::LD_C_IND_A => {
                self.write_byte(0xFF00 + self.registers.c as u16, self.registers.a);
                Some((self.pc.wrapping_add(1), 8))
            }
            op::LD_A_C_IND => {
                self.registers.a = self.read_byte(0xFF00 + self.registers.c as u16);
                Some((self.pc.wrapping_add(1), 8))
            }

            op::ADD_SP_R8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1)) as i8;
                let init_sp = self.sp;

                self.sp = add_u16_i8(init_sp, mem_val);

                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = ((init_sp & 0xF) + ((mem_val as u16) & 0xF)) > 0xF;
                self.registers.f.carry = ((init_sp & 0xFF) + ((mem_val as u16) & 0xFF)) > 0xFF;

                Some((self.pc.wrapping_add(2), 16))
            }

            op::LD_HL_SP_R8 => {
                let mem_val = self.read_byte(self.pc.wrapping_add(1));
                let val = mem_val;
                self.registers.set_hl(add_u16_i8(self.sp, val as i8));
                self.registers.f.zero = false;
                self.registers.f.subtract = false;
                self.registers.f.half_carry = half_carry_add_8bits(self.sp as u8, val);
                self.registers.f.carry = ((self.sp & 0xFF).wrapping_add(val as u16)) > 0xFF;
                Some((self.pc.wrapping_add(2), 12))
            }
            op::LD_SP_HL => {
                self.sp = self.registers.get_hl();
                Some((self.pc.wrapping_add(1), 8))
            }

            op::LD_A16_IND_A => {
                let val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.write_byte(val, self.registers.a);
                Some((self.pc.wrapping_add(3), 16))
            }
            op::LD_A_A16_IND => {
                let val = self.read_two_bytes(self.pc.wrapping_add(1));
                self.registers.a = self.read_byte(val);
                Some((self.pc.wrapping_add(3), 16))
            } // END LDs

            op::POP_BC => {
                let val = self.pop();
                self.registers.set_bc(val);
                Some((self.pc.wrapping_add(1), 12))
            }
            op::POP_DE => {
                let val = self.pop();
                self.registers.set_de(val);
                Some((self.pc.wrapping_add(1), 12))
            }
            op::POP_HL => {
                let val = self.pop();
                self.registers.set_hl(val);
                Some((self.pc.wrapping_add(1), 12))
            }
            op::POP_AF => {
                let val = self.pop();
                self.registers.set_af(val);
                self.registers.f.zero = ((val & 0xFF) >> 7) & 1 == 1;
                self.registers.f.subtract = ((val & 0xFF) >> 6) & 1 == 1;
                self.registers.f.half_carry = ((val & 0xFF) >> 5) & 1 == 1;
                self.registers.f.carry = ((val & 0xFF) >> 4) & 1 == 1;
                Some((self.pc.wrapping_add(1), 12))
            }

            op::PUSH_BC => {
                self.push(self.registers.get_bc());
                Some((self.pc.wrapping_add(1), 16))
            }
            op::PUSH_DE => {
                self.push(self.registers.get_de());
                Some((self.pc.wrapping_add(1), 16))
            }
            op::PUSH_HL => {
                self.push(self.registers.get_hl());
                Some((self.pc.wrapping_add(1), 16))
            }
            op::PUSH_AF => {
                self.push(self.registers.get_af());
                Some((self.pc.wrapping_add(1), 16))
            }

            op::RET_NZ => Some(self.ret(!self.registers.f.zero)),
            op::RET_NC => Some(self.ret(!self.registers.f.carry)),

            op::CALL_NZ_A16 => Some(self.call(!self.registers.f.zero)),
            op::CALL_NC_A16 => Some(self.call(!self.registers.f.carry)),

            op::RET_Z => Some(self.ret(self.registers.f.zero)),
            op::RET_C => Some(self.ret(self.registers.f.carry)),
            op::RET => {
                let (pc, _) = self.ret(true);
                Some((pc, 16))
            }
            op::RETI => {
                self.ime = true;
                let (pc, _) = self.ret(true);
                Some((pc, 16))
            }

            op::JP_HL => Some((self.registers.get_hl(), 4)),

            op::CALL_Z_A16 => Some(self.call(self.registers.f.zero)),
            op::CALL_C_A16 => Some(self.call(self.registers.f.carry)),
            op::CALL_A16 => Some(self.call(true)),

            op::DI => {
                self.ime = false;
                Some((self.pc.wrapping_add(1), 4))
            }
            op::EI => {
                self.enable_ime();
                Some((self.pc.wrapping_add(1), 4))
            }

            op::RST_00 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x00, 16))
            }
            op::RST_10 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x10, 16))
            }
            op::RST_20 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x20, 16))
            }
            op::RST_30 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x30, 16))
            }
            op::RST_08 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x08, 16))
            }
            op::RST_18 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x18, 16))
            }
            op::RST_28 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x28, 16))
            }
            op::RST_38 => {
                self.push(self.pc.wrapping_add(1));
                Some((0x38, 16))
            }

            _ => None,
        }
    }

    fn execute_prefixed(&mut self, instruction_byte: u8) -> Option<(u16, u8)> {
        match instruction_byte {
            cb::RLC_B => {
                self.registers.b = self.rlc(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RLC_C => {
                self.registers.c = self.rlc(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RLC_D => {
                self.registers.d = self.rlc(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RLC_E => {
                self.registers.e = self.rlc(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RLC_H => {
                self.registers.h = self.rlc(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RLC_L => {
                self.registers.l = self.rlc(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RLC_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rlc(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RLC_A => {
                self.registers.a = self.rlc(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RRC_B => {
                self.registers.b = self.rrc(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RRC_C => {
                self.registers.c = self.rrc(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RRC_D => {
                self.registers.d = self.rrc(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RRC_E => {
                self.registers.e = self.rrc(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RRC_H => {
                self.registers.h = self.rrc(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RRC_L => {
                self.registers.l = self.rrc(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RRC_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rrc(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RRC_A => {
                self.registers.a = self.rrc(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            cb::RL_B => {
                self.registers.b = self.rl(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RL_C => {
                self.registers.c = self.rl(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RL_D => {
                self.registers.d = self.rl(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RL_E => {
                self.registers.e = self.rl(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RL_H => {
                self.registers.h = self.rl(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RL_L => {
                self.registers.l = self.rl(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RL_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rl(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RL_A => {
                self.registers.a = self.rl(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            cb::RR_B => {
                self.registers.b = self.rr(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RR_C => {
                self.registers.c = self.rr(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RR_D => {
                self.registers.d = self.rr(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RR_E => {
                self.registers.e = self.rr(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RR_H => {
                self.registers.h = self.rr(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RR_L => {
                self.registers.l = self.rr(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RR_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.rr(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RR_A => {
                self.registers.a = self.rr(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            cb::SLA_B => {
                self.registers.b = self.sla(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SLA_C => {
                self.registers.c = self.sla(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SLA_D => {
                self.registers.d = self.sla(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SLA_E => {
                self.registers.e = self.sla(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SLA_H => {
                self.registers.h = self.sla(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SLA_L => {
                self.registers.l = self.sla(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SLA_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.sla(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SLA_A => {
                self.registers.a = self.sla(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRA_B => {
                self.registers.b = self.sra(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRA_C => {
                self.registers.c = self.sra(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRA_D => {
                self.registers.d = self.sra(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRA_E => {
                self.registers.e = self.sra(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRA_H => {
                self.registers.h = self.sra(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRA_L => {
                self.registers.l = self.sra(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRA_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.sra(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SRA_A => {
                self.registers.a = self.sra(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SWAP_B => {
                self.registers.b = self.swap(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SWAP_C => {
                self.registers.c = self.swap(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SWAP_D => {
                self.registers.d = self.swap(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SWAP_E => {
                self.registers.e = self.swap(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SWAP_H => {
                self.registers.h = self.swap(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SWAP_L => {
                self.registers.l = self.swap(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SWAP_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.swap(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SWAP_A => {
                self.registers.a = self.swap(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRL_B => {
                self.registers.b = self.srl(self.registers.b);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRL_C => {
                self.registers.c = self.srl(self.registers.c);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRL_D => {
                self.registers.d = self.srl(self.registers.d);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRL_E => {
                self.registers.e = self.srl(self.registers.e);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRL_H => {
                self.registers.h = self.srl(self.registers.h);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRL_L => {
                self.registers.l = self.srl(self.registers.l);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SRL_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.srl(mem_val);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SRL_A => {
                self.registers.a = self.srl(self.registers.a);
                Some((self.pc.wrapping_add(2), 8))
            }

            cb::BIT_0_B => {
                self.bit(self.registers.b, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_0_C => {
                self.bit(self.registers.c, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_0_D => {
                self.bit(self.registers.d, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_0_E => {
                self.bit(self.registers.e, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_0_H => {
                self.bit(self.registers.h, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_0_L => {
                self.bit(self.registers.l, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_0_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 0);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_0_A => {
                self.bit(self.registers.a, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_1_B => {
                self.bit(self.registers.b, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_1_C => {
                self.bit(self.registers.c, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_1_D => {
                self.bit(self.registers.d, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_1_E => {
                self.bit(self.registers.e, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_1_H => {
                self.bit(self.registers.h, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_1_L => {
                self.bit(self.registers.l, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_1_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 1);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_1_A => {
                self.bit(self.registers.a, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_2_B => {
                self.bit(self.registers.b, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_2_C => {
                self.bit(self.registers.c, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_2_D => {
                self.bit(self.registers.d, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_2_E => {
                self.bit(self.registers.e, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_2_H => {
                self.bit(self.registers.h, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_2_L => {
                self.bit(self.registers.l, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_2_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 2);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_2_A => {
                self.bit(self.registers.a, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_3_B => {
                self.bit(self.registers.b, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_3_C => {
                self.bit(self.registers.c, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_3_D => {
                self.bit(self.registers.d, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_3_E => {
                self.bit(self.registers.e, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_3_H => {
                self.bit(self.registers.h, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_3_L => {
                self.bit(self.registers.l, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_3_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 3);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_3_A => {
                self.bit(self.registers.a, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_4_B => {
                self.bit(self.registers.b, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_4_C => {
                self.bit(self.registers.c, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_4_D => {
                self.bit(self.registers.d, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_4_E => {
                self.bit(self.registers.e, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_4_H => {
                self.bit(self.registers.h, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_4_L => {
                self.bit(self.registers.l, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_4_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 4);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_4_A => {
                self.bit(self.registers.a, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_5_B => {
                self.bit(self.registers.b, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_5_C => {
                self.bit(self.registers.c, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_5_D => {
                self.bit(self.registers.d, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_5_E => {
                self.bit(self.registers.e, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_5_H => {
                self.bit(self.registers.h, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_5_L => {
                self.bit(self.registers.l, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_5_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 5);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_5_A => {
                self.bit(self.registers.a, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_6_B => {
                self.bit(self.registers.b, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_6_C => {
                self.bit(self.registers.c, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_6_D => {
                self.bit(self.registers.d, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_6_E => {
                self.bit(self.registers.e, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_6_H => {
                self.bit(self.registers.h, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_6_L => {
                self.bit(self.registers.l, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_6_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 6);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_6_A => {
                self.bit(self.registers.a, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_7_B => {
                self.bit(self.registers.b, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_7_C => {
                self.bit(self.registers.c, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_7_D => {
                self.bit(self.registers.d, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_7_E => {
                self.bit(self.registers.e, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_7_H => {
                self.bit(self.registers.h, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_7_L => {
                self.bit(self.registers.l, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::BIT_7_HL_IND => {
                let mem_val = self.read_byte(self.registers.get_hl());
                self.bit(mem_val, 7);
                Some((self.pc.wrapping_add(2), 12))
            }
            cb::BIT_7_A => {
                self.bit(self.registers.a, 7);
                Some((self.pc.wrapping_add(2), 8))
            }

            cb::RES_0_B => {
                self.registers.b = self.reset(self.registers.b, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_0_C => {
                self.registers.c = self.reset(self.registers.c, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_0_D => {
                self.registers.d = self.reset(self.registers.d, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_0_E => {
                self.registers.e = self.reset(self.registers.e, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_0_H => {
                self.registers.h = self.reset(self.registers.h, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_0_L => {
                self.registers.l = self.reset(self.registers.l, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_0_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 0);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_0_A => {
                self.registers.a = self.reset(self.registers.a, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_1_B => {
                self.registers.b = self.reset(self.registers.b, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_1_C => {
                self.registers.c = self.reset(self.registers.c, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_1_D => {
                self.registers.d = self.reset(self.registers.d, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_1_E => {
                self.registers.e = self.reset(self.registers.e, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_1_H => {
                self.registers.h = self.reset(self.registers.h, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_1_L => {
                self.registers.l = self.reset(self.registers.l, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_1_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 1);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_1_A => {
                self.registers.a = self.reset(self.registers.a, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_2_B => {
                self.registers.b = self.reset(self.registers.b, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_2_C => {
                self.registers.c = self.reset(self.registers.c, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_2_D => {
                self.registers.d = self.reset(self.registers.d, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_2_E => {
                self.registers.e = self.reset(self.registers.e, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_2_H => {
                self.registers.h = self.reset(self.registers.h, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_2_L => {
                self.registers.l = self.reset(self.registers.l, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_2_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 2);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_2_A => {
                self.registers.a = self.reset(self.registers.a, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_3_B => {
                self.registers.b = self.reset(self.registers.b, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_3_C => {
                self.registers.c = self.reset(self.registers.c, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_3_D => {
                self.registers.d = self.reset(self.registers.d, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_3_E => {
                self.registers.e = self.reset(self.registers.e, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_3_H => {
                self.registers.h = self.reset(self.registers.h, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_3_L => {
                self.registers.l = self.reset(self.registers.l, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_3_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 3);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_3_A => {
                self.registers.a = self.reset(self.registers.a, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_4_B => {
                self.registers.b = self.reset(self.registers.b, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_4_C => {
                self.registers.c = self.reset(self.registers.c, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_4_D => {
                self.registers.d = self.reset(self.registers.d, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_4_E => {
                self.registers.e = self.reset(self.registers.e, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_4_H => {
                self.registers.h = self.reset(self.registers.h, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_4_L => {
                self.registers.l = self.reset(self.registers.l, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_4_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 4);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_4_A => {
                self.registers.a = self.reset(self.registers.a, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_5_B => {
                self.registers.b = self.reset(self.registers.b, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_5_C => {
                self.registers.c = self.reset(self.registers.c, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_5_D => {
                self.registers.d = self.reset(self.registers.d, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_5_E => {
                self.registers.e = self.reset(self.registers.e, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_5_H => {
                self.registers.h = self.reset(self.registers.h, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_5_L => {
                self.registers.l = self.reset(self.registers.l, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_5_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 5);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_5_A => {
                self.registers.a = self.reset(self.registers.a, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_6_B => {
                self.registers.b = self.reset(self.registers.b, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_6_C => {
                self.registers.c = self.reset(self.registers.c, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_6_D => {
                self.registers.d = self.reset(self.registers.d, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_6_E => {
                self.registers.e = self.reset(self.registers.e, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_6_H => {
                self.registers.h = self.reset(self.registers.h, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_6_L => {
                self.registers.l = self.reset(self.registers.l, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_6_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 6);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_6_A => {
                self.registers.a = self.reset(self.registers.a, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_7_B => {
                self.registers.b = self.reset(self.registers.b, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_7_C => {
                self.registers.c = self.reset(self.registers.c, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_7_D => {
                self.registers.d = self.reset(self.registers.d, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_7_E => {
                self.registers.e = self.reset(self.registers.e, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_7_H => {
                self.registers.h = self.reset(self.registers.h, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_7_L => {
                self.registers.l = self.reset(self.registers.l, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::RES_7_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.reset(mem_val, 7);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::RES_7_A => {
                self.registers.a = self.reset(self.registers.a, 7);
                Some((self.pc.wrapping_add(2), 8))
            }

            cb::SET_0_B => {
                self.registers.b = self.set(self.registers.b, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_0_C => {
                self.registers.c = self.set(self.registers.c, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_0_D => {
                self.registers.d = self.set(self.registers.d, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_0_E => {
                self.registers.e = self.set(self.registers.e, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_0_H => {
                self.registers.h = self.set(self.registers.h, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_0_L => {
                self.registers.l = self.set(self.registers.l, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_0_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 0);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_0_A => {
                self.registers.a = self.set(self.registers.a, 0);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_1_B => {
                self.registers.b = self.set(self.registers.b, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_1_C => {
                self.registers.c = self.set(self.registers.c, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_1_D => {
                self.registers.d = self.set(self.registers.d, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_1_E => {
                self.registers.e = self.set(self.registers.e, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_1_H => {
                self.registers.h = self.set(self.registers.h, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_1_L => {
                self.registers.l = self.set(self.registers.l, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_1_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 1);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_1_A => {
                self.registers.a = self.set(self.registers.a, 1);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_2_B => {
                self.registers.b = self.set(self.registers.b, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_2_C => {
                self.registers.c = self.set(self.registers.c, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_2_D => {
                self.registers.d = self.set(self.registers.d, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_2_E => {
                self.registers.e = self.set(self.registers.e, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_2_H => {
                self.registers.h = self.set(self.registers.h, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_2_L => {
                self.registers.l = self.set(self.registers.l, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_2_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 2);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_2_A => {
                self.registers.a = self.set(self.registers.a, 2);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_3_B => {
                self.registers.b = self.set(self.registers.b, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_3_C => {
                self.registers.c = self.set(self.registers.c, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_3_D => {
                self.registers.d = self.set(self.registers.d, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_3_E => {
                self.registers.e = self.set(self.registers.e, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_3_H => {
                self.registers.h = self.set(self.registers.h, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_3_L => {
                self.registers.l = self.set(self.registers.l, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_3_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 3);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_3_A => {
                self.registers.a = self.set(self.registers.a, 3);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_4_B => {
                self.registers.b = self.set(self.registers.b, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_4_C => {
                self.registers.c = self.set(self.registers.c, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_4_D => {
                self.registers.d = self.set(self.registers.d, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_4_E => {
                self.registers.e = self.set(self.registers.e, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_4_H => {
                self.registers.h = self.set(self.registers.h, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_4_L => {
                self.registers.l = self.set(self.registers.l, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_4_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 4);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_4_A => {
                self.registers.a = self.set(self.registers.a, 4);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_5_B => {
                self.registers.b = self.set(self.registers.b, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_5_C => {
                self.registers.c = self.set(self.registers.c, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_5_D => {
                self.registers.d = self.set(self.registers.d, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_5_E => {
                self.registers.e = self.set(self.registers.e, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_5_H => {
                self.registers.h = self.set(self.registers.h, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_5_L => {
                self.registers.l = self.set(self.registers.l, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_5_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 5);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_5_A => {
                self.registers.a = self.set(self.registers.a, 5);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_6_B => {
                self.registers.b = self.set(self.registers.b, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_6_C => {
                self.registers.c = self.set(self.registers.c, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_6_D => {
                self.registers.d = self.set(self.registers.d, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_6_E => {
                self.registers.e = self.set(self.registers.e, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_6_H => {
                self.registers.h = self.set(self.registers.h, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_6_L => {
                self.registers.l = self.set(self.registers.l, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_6_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 6);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_6_A => {
                self.registers.a = self.set(self.registers.a, 6);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_7_B => {
                self.registers.b = self.set(self.registers.b, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_7_C => {
                self.registers.c = self.set(self.registers.c, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_7_D => {
                self.registers.d = self.set(self.registers.d, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_7_E => {
                self.registers.e = self.set(self.registers.e, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_7_H => {
                self.registers.h = self.set(self.registers.h, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_7_L => {
                self.registers.l = self.set(self.registers.l, 7);
                Some((self.pc.wrapping_add(2), 8))
            }
            cb::SET_7_HL_IND => {
                let hl_reg_val = self.registers.get_hl();
                let mem_val = self.read_byte(hl_reg_val);
                let new_val = self.set(mem_val, 7);
                self.write_byte(hl_reg_val, new_val);
                Some((self.pc.wrapping_add(2), 16))
            }
            cb::SET_7_A => {
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
            self.bus.step_peripherals(cycles, true);
            return cycles;
        }

        let (next_pc, cycles) = match self.read_byte(self.pc) {
            cb::PREFIX => {
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
            self.bus
                .step_peripherals(cycles - self.cycles_synced, false);
        }
        self.cycles_synced = 0;

        cycles
    }

    pub fn is_frame_buffer_ready(&mut self) -> bool {
        self.bus.is_frame_buffer_ready()
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

        fn switch_speed(&mut self) {}

        fn step_peripherals(&mut self, _cycles: u8, _is_halted: bool) {}

        fn is_frame_buffer_ready(&mut self) -> bool {
            false
        }
    }

    fn make_test_cpu() -> CPU<FakeBus> {
        CPU::new(
            &Config {
                mode: Mode::DMG,
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
