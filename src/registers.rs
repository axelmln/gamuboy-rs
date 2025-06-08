#[derive(Debug)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub f: FlagsRegister,
    pub h: u8,
    pub l: u8,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: FlagsRegister::new(),
            h: 0,
            l: 0,
        }
    }

    pub fn new_post_boot() -> Self {
        Self {
            a: 0x01, // DMG
            // a: 0x11, // CGB
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            f: FlagsRegister::new_debug(),
            h: 0x01,
            l: 0x4D,
        }
    }

    pub fn get_af(&self) -> u16 {
        as_16bits(self.a, u8::from(self.f.clone()))
    }

    pub fn set_af(&mut self, value: u16) {
        self.a = get_16bits_left(value);
        self.f = FlagsRegister::from(get_16bits_right(value));
    }

    pub fn get_bc(&self) -> u16 {
        as_16bits(self.b, self.c)
    }

    pub fn set_bc(&mut self, value: u16) {
        self.b = get_16bits_left(value);
        self.c = get_16bits_right(value);
    }

    pub fn get_de(&self) -> u16 {
        as_16bits(self.d, self.e)
    }

    pub fn set_de(&mut self, value: u16) {
        self.d = get_16bits_left(value);
        self.e = get_16bits_right(value);
    }

    pub fn get_hl(&self) -> u16 {
        as_16bits(self.h, self.l)
    }

    pub fn set_hl(&mut self, value: u16) {
        self.h = get_16bits_left(value);
        self.l = get_16bits_right(value);
    }
}

fn as_16bits(left: u8, right: u8) -> u16 {
    (left as u16) << 8 | right as u16
}

fn get_16bits_left(value: u16) -> u8 {
    (value >> 8) as u8
}

fn get_16bits_right(value: u16) -> u8 {
    value as u8
}

#[derive(Clone, Debug)]
pub struct FlagsRegister {
    pub zero: bool,
    pub subtract: bool,
    pub half_carry: bool,
    pub carry: bool,
}

impl FlagsRegister {
    fn new() -> Self {
        Self {
            zero: false,
            subtract: false,
            half_carry: false,
            carry: false,
        }
    }

    fn new_debug() -> Self {
        Self {
            zero: true,
            subtract: false,
            half_carry: true,
            carry: true,
        }
    }
}

const ZERO_FLAG_BYTE_BIT: u8 = 7;
const SUBTRACT_FLAG_BYTE_BIT: u8 = 6;
const HALF_CARRY_FLAG_BYTE_BIT: u8 = 5;
const CARRY_FLAG_BYTE_BIT: u8 = 4;

impl From<FlagsRegister> for u8 {
    fn from(flag: FlagsRegister) -> u8 {
        (flag.zero as u8) << ZERO_FLAG_BYTE_BIT
            | (flag.subtract as u8) << SUBTRACT_FLAG_BYTE_BIT
            | (flag.half_carry as u8) << HALF_CARRY_FLAG_BYTE_BIT
            | (flag.carry as u8) << CARRY_FLAG_BYTE_BIT
    }
}

impl From<u8> for FlagsRegister {
    fn from(byte: u8) -> Self {
        Self {
            zero: ((byte >> ZERO_FLAG_BYTE_BIT) & 1) != 0,
            subtract: ((byte >> SUBTRACT_FLAG_BYTE_BIT) & 1) != 0,
            half_carry: ((byte >> HALF_CARRY_FLAG_BYTE_BIT) & 1) != 0,
            carry: ((byte >> CARRY_FLAG_BYTE_BIT) & 1) != 0,
        }
    }
}
