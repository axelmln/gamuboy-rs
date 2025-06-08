use crate::{memory::MemReadWriter, stereo::StereoPlayer};

const MASTER_CLOCK_FREQ: u32 = 4_194_304;

const SQUARE_CHANNEL_PERIOD_FREQ: u32 = 1_048_576;
const WAVE_CHANNEL_PERIOD_FREQ: u32 = 2_097_152;

// pub const SAMPLE_RATE: u32 = 44_100;
pub const SAMPLE_RATE: u32 = 48000;
const CYCLES_BEFORE_SAMPLE: u32 = MASTER_CLOCK_FREQ / SAMPLE_RATE;

// const SAMPLES_BUFFER_SIZE: usize = (SAMPLE_RATE / CYCLES_BEFORE_SAMPLE * 2) as usize;
pub const SAMPLES_BUFFER_SIZE: usize = 1024;

const TWO_BITS: u8 = 0b11;
const THREE_BITS: u8 = 0b111;
const FOUR_BITS: u8 = 0b1111;
const SIX_BITS: u8 = 0b111111;

const BIT_0: u8 = 0b1;
const BIT_1: u8 = 0b10;
const BIT_2: u8 = 0b100;
const BIT_3: u8 = 0b1000;
const BIT_4: u8 = 0b10000;
const BIT_5: u8 = 0b100000;
const BIT_6: u8 = 0b1000000;
const BIT_7: u8 = 0b10000000;

#[derive(Clone, Debug)]
enum SweepDirection {
    Addition,
    Substraction,
}

impl From<u8> for SweepDirection {
    fn from(value: u8) -> Self {
        match value & BIT_3 {
            0 => Self::Addition,
            _ => Self::Substraction,
        }
    }
}

#[derive(Clone, Debug)]
enum EnvelopeDirection {
    Decrease,
    Increase,
}

impl From<u8> for EnvelopeDirection {
    fn from(value: u8) -> Self {
        match value & BIT_3 {
            0 => Self::Decrease,
            _ => Self::Increase,
        }
    }
}

impl EnvelopeDirection {
    fn coeff(&self) -> i8 {
        match self {
            Self::Decrease => -1,
            Self::Increase => 1,
        }
    }
}

#[derive(Clone, Debug)]
struct Envelope {
    initial_volume: u8,
    dir: EnvelopeDirection,
    sweep_pace: u8,

    volume: u8,
    dir_shadow: EnvelopeDirection,
    pace_shadow: u8,
    timer: u8,
}

impl Envelope {
    fn new() -> Self {
        Self {
            initial_volume: 0,
            dir: EnvelopeDirection::Decrease,
            sweep_pace: 0,

            volume: 0,
            dir_shadow: EnvelopeDirection::Decrease,
            pace_shadow: 0,
            timer: 0,
        }
    }

    fn read(&self) -> u8 {
        (self.initial_volume << 4) | ((self.dir.clone() as u8) << 3) | self.sweep_pace
    }

    /// TO IMPROVE
    ///
    /// Currently return caller channel's new dac state
    fn write(&mut self, value: u8) -> bool {
        self.initial_volume = value >> 4;
        self.dir = EnvelopeDirection::from(value);
        self.sweep_pace = value & THREE_BITS;

        !(self.initial_volume == 0
            && (self.dir.clone() as u8) == (EnvelopeDirection::Decrease as u8))
    }

    fn reset(&mut self) {
        self.dir_shadow = self.dir.clone();
        self.pace_shadow = self.sweep_pace;
        self.volume = self.initial_volume;
        self.timer = self.pace_shadow;
    }

    fn tick(&mut self) {
        if self.pace_shadow == 0 {
            return;
        }

        if self.timer == 0 {
            self.volume =
                (self.volume as i8 + (1 * self.dir_shadow.coeff())).clamp(0, 0b1111 as i8) as u8;
            self.timer = self.pace_shadow;
        } else {
            self.timer -= 1;
        }
    }
}

#[derive(Clone, Debug)]
struct Period {
    high: u8,
    low: u8,
    timer: u16,
}

impl Period {
    fn new() -> Self {
        Self {
            high: 0,
            low: 0,
            timer: 0,
        }
    }

    fn write_low(&mut self, value: u8) {
        self.low = value;
    }

    fn write_high(&mut self, value: u8) {
        self.high = value & THREE_BITS;
    }

    fn write(&mut self, value: u16) {
        self.write_low(value as u8);
        self.write_high((value >> 8) as u8);
    }

    fn value(&self) -> u16 {
        (self.high as u16) << 8 | self.low as u16
    }
}

#[derive(Clone, Debug)]
enum DutyCycle {
    Eighth,
    Quarter,
    Half,
    ThreeQuarter,
}

impl From<u8> for DutyCycle {
    fn from(value: u8) -> Self {
        match value & TWO_BITS {
            0 => Self::Eighth,
            1 => Self::Quarter,
            2 => Self::Half,
            _ => Self::ThreeQuarter,
        }
    }
}

impl DutyCycle {
    fn waveform(&self) -> [u8; 8] {
        match self {
            Self::Eighth => [0, 0, 0, 0, 0, 0, 0, 1],
            Self::Quarter => [1, 0, 0, 0, 0, 0, 0, 1],
            Self::Half => [1, 0, 0, 0, 0, 1, 1, 1],
            Self::ThreeQuarter => [0, 1, 1, 1, 1, 1, 1, 0],
        }
    }

    fn signal(&self, position: u8) -> u8 {
        self.waveform()[position as usize]
    }
}

#[derive(Clone, Debug)]
struct LengthTimer {
    /// counter target before turning the channel off
    len: u16,
    length_enable: bool,
    init_length_timer: u8,
    timer: u16,
}

impl LengthTimer {
    fn new(len: u16) -> Self {
        Self {
            len,
            length_enable: false,
            init_length_timer: 0,
            timer: 0,
        }
    }

    fn read_control(&self) -> u8 {
        ((self.length_enable as u8) << 6) | 0b10111111
    }

    fn write_initial_length_timer(&mut self, value: u8) {
        self.init_length_timer = value;
        self.timer = self.len - (self.init_length_timer as u16);
    }

    /// TO IMPROVE
    ///
    /// Currently return false if the caller channel should be disabled
    fn write_enable(&mut self, value: u8, current_step: u8) -> bool {
        // (blargg's test) Enabling in first half of length period should clock length
        let enabled = value & BIT_6 == BIT_6;
        if !self.length_enable && enabled {
            if current_step % 2 == 0 && self.timer > 0 {
                self.length_enable = true;
                return self.tick();
            }
        }
        self.length_enable = enabled;
        true
    }

    fn reset(&mut self, current_step: u8) {
        if self.timer == 0 {
            self.timer = self.len;
            if self.length_enable && current_step % 2 == 0 {
                self.timer -= 1;
            }
        }
    }

    /// TO IMPROVE
    ///
    /// Currently this method returns the caller channel's new state
    fn tick(&mut self) -> bool {
        if self.length_enable && self.timer > 0 {
            self.timer -= 1;
            if self.timer == 0 {
                return false;
            }
        }
        true
    }
}

#[derive(Clone, Debug)]
struct Sweep {
    pace: u8,
    direction: SweepDirection,
    has_substracted: bool,
    step: u8,
    timer: u8,
    enabled_flag: bool,
    period_shadow_register: u16,
}

impl Sweep {
    fn new() -> Self {
        Self {
            pace: 0,
            direction: SweepDirection::Addition,
            has_substracted: false,
            step: 0,
            timer: 0,
            enabled_flag: false,
            period_shadow_register: 0,
        }
    }

    fn read(&self) -> u8 {
        (self.step & THREE_BITS)
            | (self.direction.clone() as u8) << 3
            | ((self.pace & THREE_BITS) << 4)
            | (1 << 7)
    }

    /// TO IMPROVE
    ///
    /// Currently returns the caller channel new state
    fn write(&mut self, value: u8) -> bool {
        self.step = value & THREE_BITS;
        self.direction = SweepDirection::from(value);
        self.pace = (value >> 4) & THREE_BITS;
        match self.direction {
            SweepDirection::Addition => !self.has_substracted,
            _ => true,
        }
    }

    /// Returns new freq and if overflowed
    fn compute_sweep_frequency(&mut self, value: u16) -> (u16, bool) {
        let shifted = value >> self.step;
        let new_freq = match self.direction {
            SweepDirection::Addition => value + shifted,
            SweepDirection::Substraction => {
                self.has_substracted = true;
                if shifted > value {
                    return (0, true);
                }
                value - shifted
            }
        };
        (new_freq, new_freq > 2047)
    }
}

#[derive(Clone, Debug)]
struct Panning {
    left: bool,
    right: bool,
}

impl Panning {
    fn new() -> Self {
        Self {
            left: false,
            right: false,
        }
    }

    fn write(&mut self, left: bool, right: bool) {
        (self.left, self.right) = (left, right);
    }
}

#[derive(Clone, Debug)]
struct Dac {}

impl Dac {
    fn new() -> Self {
        Self {}
    }

    fn convert(&self, input: u8) -> f32 {
        (input as f32 / 7.5) - 1.
    }
}

#[derive(Clone, Debug)]
struct SquareChannel {
    on: bool,
    dac_on: bool,
    dac: Dac,

    panning: Panning,

    sweep: Option<Sweep>,

    length_timer: LengthTimer,
    wave_duty: DutyCycle,
    duty_step_counter: u8,

    envelope: Envelope,

    period: Period,
}

impl SquareChannel {
    fn new(with_sweep: bool) -> Self {
        Self {
            on: false,
            dac_on: false,
            dac: Dac::new(),

            panning: Panning::new(),

            sweep: if with_sweep { Some(Sweep::new()) } else { None },

            length_timer: LengthTimer::new(64),
            wave_duty: DutyCycle::Eighth,
            duty_step_counter: 0,

            envelope: Envelope::new(),

            period: Period::new(),
        }
    }

    fn enabled(&self) -> bool {
        self.on && self.dac_on
    }

    fn write_sweep(&mut self, value: u8) {
        self.on = self.sweep.as_mut().unwrap().write(value) && self.on;
    }

    fn write_timer_and_duty(&mut self, value: u8) {
        self.length_timer
            .write_initial_length_timer(value & SIX_BITS);
        self.wave_duty = DutyCycle::from(value >> 6);
    }

    fn write_envelope(&mut self, value: u8) {
        self.dac_on = self.envelope.write(value);
        if !self.dac_on {
            self.on = false;
        }
    }

    fn read_duty(&self) -> u8 {
        ((self.wave_duty.clone() as u8) << 6) | SIX_BITS
    }

    fn compute_period_timer(&self) -> u16 {
        (2048 - self.period.value()) * (MASTER_CLOCK_FREQ / SQUARE_CHANNEL_PERIOD_FREQ) as u16
    }

    fn trigger(&mut self, current_step: u8) {
        self.on = true;

        self.length_timer.reset(current_step);

        self.envelope.reset();

        self.period.timer = self.compute_period_timer();

        if let Some(swp) = &mut self.sweep {
            swp.has_substracted = false;
            let period_value = self.period.value();
            swp.period_shadow_register = period_value;
            swp.timer = if swp.pace > 0 { swp.pace } else { 8 };
            swp.enabled_flag = swp.pace != 0 || swp.step != 0;
            if swp.step != 0 {
                let (_, overflowed) = swp.compute_sweep_frequency(period_value);
                self.on = !overflowed;
            }
        }
    }

    fn handle_frequency_timer(&mut self, cycles: u8) {
        let mut cycles = cycles as u16;
        while cycles >= self.period.timer {
            cycles -= self.period.timer;
            self.period.timer = self.compute_period_timer();
            self.duty_step_counter = (self.duty_step_counter + 1) % 8;
        }
        self.period.timer -= cycles;
    }

    fn step(&mut self, cycles: u8) {
        if !self.on {
            return;
        }
        self.handle_frequency_timer(cycles);
    }

    fn tick_sweep(&mut self) {
        if !self.on {
            return;
        }

        if let Some(swp) = &mut self.sweep {
            if swp.timer > 0 {
                swp.timer -= 1;
            }

            if swp.timer == 0 {
                swp.timer = if swp.pace > 0 { swp.pace } else { 8 };

                if !swp.enabled_flag || swp.pace == 0 {
                    return;
                }

                let (new_freq, overflowed) =
                    swp.compute_sweep_frequency(swp.period_shadow_register);
                self.on = !overflowed;

                if !overflowed && swp.step > 0 {
                    self.period.write(new_freq);
                    swp.period_shadow_register = new_freq;

                    let (_, overflowed) = swp.compute_sweep_frequency(new_freq);
                    self.on = !overflowed;
                }
            }
        }
    }

    fn tick_envelope(&mut self) {
        if !self.on {
            return;
        }
        self.envelope.tick();
    }

    fn tick_length_timer(&mut self) {
        self.on = self.length_timer.tick() && self.on;
    }

    fn output(&self) -> f32 {
        if !self.dac_on {
            return 0.;
        }
        let input = self.wave_duty.signal(self.duty_step_counter) * self.envelope.volume;
        self.dac.convert(input)
    }
}

#[derive(Clone, Debug)]
enum OutputLevel {
    Mute,
    Full,
    Half,
    Quarter,
}

impl From<u8> for OutputLevel {
    fn from(value: u8) -> Self {
        match value & 0b11 {
            0 => Self::Mute,
            0b01 => Self::Full,
            0b10 => Self::Half,
            _ => Self::Quarter,
        }
    }
}

impl OutputLevel {
    fn shift(&self) -> u8 {
        match self {
            Self::Mute => 4,
            Self::Full => 0,
            Self::Half => 2,
            _ => 3,
        }
    }
}

const WAVE_RAM_START_ADDR: u16 = 0xFF30;
const WAVE_RAM_END_ADDR: u16 = 0xFF3F;

#[derive(Clone, Debug)]
struct WaveRam {
    ram: [u8; 16],
    sample_index: u8,
    sample_buffer: u8,
}

impl WaveRam {
    fn new() -> Self {
        Self {
            ram: [
                0x84, 0x40, 0x43, 0xAA, 0x2D, 0x78, 0x92, 0x3C, 0x60, 0x59, 0x59, 0xB0, 0x34, 0xB5,
                0xCA, 0x6E,
            ],
            sample_index: 0,
            sample_buffer: 0,
        }
    }

    fn reset(&mut self) {
        self.sample_index = 0;
    }

    fn handle_period(&mut self) {
        self.sample_index = (self.sample_index + 1) % 32;
        let byte = self.ram[(self.sample_index / 2) as usize];
        // [ [0][1], [2][3], [4][5], [6][7], [8][9], [10][11], [12][13], [14][15], [16][17], [18][19], [20][21], [22][23], [24][25], [26][27], [28][29], [30][31] ]
        //      0       1       2       3       4        5         6         7        8          9         10        11        12        13        14       15
        self.sample_buffer = if self.sample_index % 2 == 0 {
            (byte & 0xF0) >> 4
        } else {
            byte & 0x0F
        };
    }
}

#[derive(Clone, Debug)]
struct WaveChannel {
    on: bool,
    dac_on: bool,
    dac: Dac,

    panning: Panning,

    length_timer: LengthTimer,

    initial_output_level: OutputLevel,
    output_level: OutputLevel,

    period: Period,

    wave_ram: WaveRam,

    started_sampling: bool,
}

impl WaveChannel {
    fn new() -> Self {
        Self {
            on: false,
            dac_on: false,
            dac: Dac::new(),

            panning: Panning::new(),

            length_timer: LengthTimer::new(256),

            initial_output_level: OutputLevel::Mute,
            output_level: OutputLevel::Mute,

            period: Period::new(),

            wave_ram: WaveRam::new(),

            started_sampling: false,
        }
    }

    fn enabled(&self) -> bool {
        self.on && self.dac_on
    }

    fn write_dac_enable(&mut self, value: u8) {
        self.dac_on = value & BIT_7 == BIT_7;
        if !self.dac_on {
            self.on = false;
        }
    }

    fn read_output_level(&self) -> u8 {
        (1 << 7) | ((self.initial_output_level.clone() as u8) << 5) | 0b11111
    }

    fn write_initial_output_level(&mut self, value: u8) {
        self.initial_output_level = OutputLevel::from(value >> 5);
    }

    fn read_wave_ram(&self, address: u16) -> u8 {
        // wave read while on dmg behaviour
        if self.enabled() {
            if self.started_sampling && self.period.timer == self.compute_period_timer() {
                self.wave_ram.ram[self.wave_ram.sample_index as usize / 2]
            } else {
                0xFF
            }
        } else {
            self.wave_ram.ram[(address - WAVE_RAM_START_ADDR) as usize]
        }
    }

    fn write_wave_ram(&mut self, address: u16, value: u8) {
        // wave write while on dmg behaviour
        if self.enabled() {
            if self.started_sampling && self.period.timer == self.compute_period_timer() {
                self.wave_ram.ram[self.wave_ram.sample_index as usize / 2] = value;
            }
        } else {
            self.wave_ram.ram[(address - WAVE_RAM_START_ADDR) as usize] = value;
        }
    }

    fn read_dac_enable(&self) -> u8 {
        ((self.dac_on as u8) << 7) | !(1 << 7)
    }

    fn compute_period_timer(&self) -> u16 {
        (2048 - self.period.value()) * (MASTER_CLOCK_FREQ / WAVE_CHANNEL_PERIOD_FREQ) as u16
    }

    fn trigger(&mut self, current_step: u8) {
        // triggering while on corrupts the first 4 bytes of wave ram on dmg
        // if next step will clock timer, then simulate corruption
        if self.enabled()
            && self.period.timer <= (MASTER_CLOCK_FREQ / WAVE_CHANNEL_PERIOD_FREQ) as u16
        {
            let mut corrupt = |pos: usize| {
                for i in 0..4 {
                    self.wave_ram.ram[i] = self.wave_ram.ram[pos + i]
                }
            };
            let pos = (self.wave_ram.sample_index + 1) % 32;
            match pos / 2 {
                0..=3 => self.wave_ram.ram[0] = self.wave_ram.ram[pos as usize / 2],
                4..=7 => corrupt(4),
                8..=11 => corrupt(8),
                12..=15 => corrupt(12),
                _ => unreachable!(),
            }
        }

        self.on = true;

        self.length_timer.reset(current_step);

        self.output_level = self.initial_output_level.clone();

        self.period.timer = self.compute_period_timer() + 6; // adding delay to pass "wave read while on" test

        self.wave_ram.reset();
        self.started_sampling = false;
    }

    fn handle_frequency_timer(&mut self, cycles: u8) {
        let mut cycles = cycles as u16;

        while cycles >= self.period.timer {
            cycles -= self.period.timer;
            self.period.timer = self.compute_period_timer();
            self.wave_ram.handle_period();
            self.started_sampling = true;
        }

        self.period.timer -= cycles;
    }

    fn step(&mut self, cycles: u8) {
        if !self.on {
            return;
        }
        self.handle_frequency_timer(cycles);
    }

    fn tick_length_timer(&mut self) {
        self.on = self.length_timer.tick() && self.on;
    }

    fn output(&self) -> f32 {
        if !self.dac_on {
            return 0.;
        }
        let input = self.wave_ram.sample_buffer >> self.output_level.shift();
        self.dac.convert(input)
    }
}

#[derive(Clone, Debug)]
struct NoiseChannel {
    on: bool,
    dac_on: bool,
    dac: Dac,

    panning: Panning,

    length_timer: LengthTimer,

    envelope: Envelope,

    lfsr: u16,

    clock_shift: u8,
    /// LFSR width, NR43 bit 3
    short_mode: bool,
    clock_divider: u8,

    period_div: u32,
}

impl NoiseChannel {
    fn new() -> Self {
        Self {
            on: false,
            dac_on: false,
            dac: Dac::new(),

            panning: Panning::new(),
            length_timer: LengthTimer::new(64),
            envelope: Envelope::new(),

            lfsr: 0,

            clock_shift: 0,
            short_mode: false,
            clock_divider: 0,

            period_div: 0,
        }
    }

    fn enabled(&self) -> bool {
        self.on && self.dac_on
    }

    fn reset_lfsr(&mut self) {
        self.lfsr = 0xFFFF >> 1;
    }

    fn read_lfsr(&self) -> u8 {
        (self.clock_shift << 4) | ((self.short_mode as u8) << 3) | self.clock_divider
    }

    fn write_lfsr(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & FOUR_BITS;
        self.short_mode = value & BIT_3 == BIT_3;
        self.clock_divider = value & THREE_BITS;
    }

    fn write_envelope(&mut self, value: u8) {
        self.dac_on = self.envelope.write(value);
        if !self.dac_on {
            self.on = false;
        }
    }

    fn trigger(&mut self, current_step: u8) {
        self.on = true;

        self.length_timer.reset(current_step);

        self.envelope.reset();

        self.reset_lfsr();
    }

    fn divisor(&self) -> u32 {
        if self.clock_divider == 0 {
            8
        } else {
            (self.clock_divider * 16) as u32
        }
    }

    fn handle_lfsr_period(&mut self) {
        let xored_low_bits = (self.lfsr & 1 as u16) ^ ((self.lfsr >> 1) & 1 as u16);
        self.lfsr = (self.lfsr >> 1) | xored_low_bits << 14;
        if self.short_mode {
            self.lfsr &= !(1 << 6);
            self.lfsr |= xored_low_bits << 6;
        }
    }

    fn handle_frequency_timer(&mut self, cycles: u8) {
        let mut cycles = cycles as u32;
        while cycles >= self.period_div {
            cycles -= self.period_div;
            self.period_div = (self.divisor() << self.clock_shift).max(1);
            self.handle_lfsr_period();
        }
        self.period_div -= cycles;
    }

    fn step(&mut self, cycles: u8) {
        if !self.on {
            return;
        }
        self.handle_frequency_timer(cycles);
    }

    fn tick_length_timer(&mut self) {
        self.on = self.length_timer.tick() && self.on;
    }

    fn tick_envelope(&mut self) {
        if !self.on {
            return;
        }
        self.envelope.tick();
    }

    fn amplitude(&self) -> u8 {
        !self.lfsr as u8 & 1
    }

    fn output(&self) -> f32 {
        if !self.dac_on {
            return 0.;
        }
        let input = self.amplitude() * self.envelope.volume;
        self.dac.convert(input)
    }
}

#[derive(Clone, Debug)]
pub struct APU<S: StereoPlayer + 'static> {
    on: bool,
    vin_left: bool,
    vin_right: bool,
    left_volume: u8,
    right_volume: u8,

    prev_div_apu: u8,
    current_step: u8,
    /// used to track when sample should be generated
    samples_cycle_acc: u32,

    ch1: SquareChannel,
    ch2: SquareChannel,
    ch3: WaveChannel,
    ch4: NoiseChannel,

    buffer: [f32; SAMPLES_BUFFER_SIZE],
    buffer_index: usize,

    stereo: S,
}

impl<S: StereoPlayer> APU<S> {
    pub fn new(stereo: S) -> Self {
        Self {
            on: false,
            vin_left: false,
            vin_right: false,
            left_volume: 0,
            right_volume: 0,

            prev_div_apu: 0,
            current_step: 0,
            samples_cycle_acc: 0,

            ch1: SquareChannel::new(true),
            ch2: SquareChannel::new(false),
            ch3: WaveChannel::new(),
            ch4: NoiseChannel::new(),

            buffer: [0.; SAMPLES_BUFFER_SIZE],
            buffer_index: 0,

            stereo,
        }
    }

    fn power_on(&mut self) {
        if !self.on {
            // When powered on, the frame sequencer is reset so that the next step will be 0
            self.current_step = 7;
        }

        self.on = true;
    }

    fn power_off(&mut self) {
        let wave_ram_copy = self.ch3.wave_ram.ram.clone();
        let ch1_len_timer = self.ch1.length_timer.timer;
        let ch2_len_timer = self.ch2.length_timer.timer;
        let ch3_len_timer = self.ch3.length_timer.timer;
        let ch4_len_timer = self.ch4.length_timer.timer;

        self.ch1 = SquareChannel::new(true);
        self.ch2 = SquareChannel::new(false);
        self.ch3 = WaveChannel::new();
        self.ch4 = NoiseChannel::new();

        self.ch3.wave_ram.ram = wave_ram_copy;
        self.ch1.length_timer.timer = ch1_len_timer;
        self.ch2.length_timer.timer = ch2_len_timer;
        self.ch3.length_timer.timer = ch3_len_timer;
        self.ch4.length_timer.timer = ch4_len_timer;

        self.on = false;
        self.vin_left = false;
        self.vin_right = false;
        self.left_volume = 0;
        self.right_volume = 0;

        self.prev_div_apu = 0;
        self.current_step = 0;
        self.samples_cycle_acc = 0;

        self.buffer = [0.; SAMPLES_BUFFER_SIZE];
        self.buffer_index = 0;
    }

    fn read_volume(&self) -> u8 {
        self.right_volume
            | (self.vin_right as u8) << 3
            | self.left_volume << 4
            | (self.vin_left as u8) << 7
    }

    fn write_volume(&mut self, value: u8) {
        self.right_volume = value & THREE_BITS;
        self.vin_right = value & BIT_3 == BIT_3;
        self.left_volume = (value >> 4) & THREE_BITS;
        self.vin_left = value & BIT_7 == BIT_7;
    }

    fn read_panning(&self) -> u8 {
        (self.ch4.panning.left as u8) << 7
            | (self.ch3.panning.left as u8) << 6
            | (self.ch2.panning.left as u8) << 5
            | (self.ch1.panning.left as u8) << 4
            | (self.ch4.panning.right as u8) << 3
            | (self.ch3.panning.right as u8) << 2
            | (self.ch2.panning.right as u8) << 1
            | (self.ch1.panning.right as u8)
    }

    fn write_panning(&mut self, value: u8) {
        self.ch1
            .panning
            .write(value & BIT_4 != 0, value & BIT_0 != 0);
        self.ch2
            .panning
            .write(value & BIT_5 != 0, value & BIT_1 != 0);
        self.ch3
            .panning
            .write(value & BIT_6 != 0, value & BIT_2 != 0);
        self.ch4
            .panning
            .write(value & BIT_7 != 0, value & BIT_3 != 0);
    }

    fn read_master_control(&self) -> u8 {
        (self.on as u8) << 7
            | (self.ch4.enabled() as u8) << 3
            | (self.ch3.enabled() as u8) << 2
            | (self.ch2.enabled() as u8) << 1
            | self.ch1.enabled() as u8
            | 0b01110000
    }

    fn write_master_control(&mut self, value: u8) {
        let on = value & BIT_7 == BIT_7;
        if !on {
            self.power_off();
        } else {
            self.power_on();
        }
    }

    fn div_apu_ticked(&self, div_apu: u8) -> bool {
        (self.prev_div_apu & BIT_4 == BIT_4) && (div_apu & BIT_4 == 0)
    }

    fn step_frame_sequencer(&mut self) {
        self.current_step = (self.current_step + 1) % 8;

        if self.current_step == 2 || self.current_step == 6 {
            self.ch1.tick_sweep();
        }
        if self.current_step % 2 == 0 {
            self.ch1.tick_length_timer();
            self.ch2.tick_length_timer();
            self.ch3.tick_length_timer();
            self.ch4.tick_length_timer();
        }
        if self.current_step == 7 {
            self.ch1.tick_envelope();
            self.ch2.tick_envelope();
            self.ch4.tick_envelope();
        }
    }

    /// Returns (left, right) mixing output
    fn mix(&self) -> (f32, f32) {
        let mut left_amps = 0.;
        if self.ch1.panning.left {
            left_amps += self.ch1.output();
        }
        if self.ch2.panning.left {
            left_amps += self.ch2.output();
        }
        if self.ch3.panning.left {
            left_amps += self.ch3.output();
        }
        if self.ch4.panning.left {
            left_amps += self.ch4.output();
        }

        let mut right_amps = 0.;
        if self.ch1.panning.right {
            right_amps += self.ch1.output();
        }
        if self.ch2.panning.right {
            right_amps += self.ch2.output();
        }
        if self.ch3.panning.right {
            right_amps += self.ch3.output();
        }
        if self.ch4.panning.right {
            right_amps += self.ch4.output();
        }

        (left_amps / 4., right_amps / 4.)
    }

    fn get_master_volume(&self, vol: u8) -> u8 {
        // Pandocs:
        // A value of 0 is treated as a volume of 1 (very quiet), and a value of 7 is treated as a volume of 8 (no volume reduction).
        vol + (vol == 0 || vol == 7) as u8
    }

    pub fn step(&mut self, cycles: u8, div_apu: u8) {
        if !self.on {
            return;
        }

        if self.div_apu_ticked(div_apu) {
            self.step_frame_sequencer();
        }
        self.prev_div_apu = div_apu;

        self.samples_cycle_acc = self.samples_cycle_acc.wrapping_add(cycles as u32);

        self.ch1.step(cycles);
        self.ch2.step(cycles);
        self.ch3.step(cycles);
        self.ch4.step(cycles);

        if self.samples_cycle_acc >= CYCLES_BEFORE_SAMPLE {
            self.samples_cycle_acc -= CYCLES_BEFORE_SAMPLE;
            let (left_mixed, right_mixed) = self.mix();
            self.buffer[self.buffer_index] =
                left_mixed * (self.get_master_volume(self.left_volume) as f32 / 7.);
            self.buffer[self.buffer_index + 1] =
                right_mixed * (self.get_master_volume(self.right_volume) as f32 / 7.);
            self.buffer_index += 2;
        }
        if self.buffer_index >= SAMPLES_BUFFER_SIZE {
            self.buffer_index = 0;
            self.stereo.play(&self.buffer);
        }
    }
}

const fn nr(x: u16, y: u16) -> u16 {
    assert!(x <= 5);
    0xFF10 + 5 * (x - 1) + y
}

const NR10: u16 = nr(1, 0);
const NR11: u16 = nr(1, 1);
const NR12: u16 = nr(1, 2);
const NR13: u16 = nr(1, 3);
const NR14: u16 = nr(1, 4);

const NR21: u16 = nr(2, 1);
const NR22: u16 = nr(2, 2);
const NR23: u16 = nr(2, 3);
const NR24: u16 = nr(2, 4);

const NR30: u16 = nr(3, 0);
const NR31: u16 = nr(3, 1);
const NR32: u16 = nr(3, 2);
const NR33: u16 = nr(3, 3);
const NR34: u16 = nr(3, 4);

const NR41: u16 = nr(4, 1);
const NR42: u16 = nr(4, 2);
const NR43: u16 = nr(4, 3);
const NR44: u16 = nr(4, 4);

const NR50: u16 = nr(5, 0);
const NR51: u16 = nr(5, 1);
const NR52: u16 = nr(5, 2);

impl<S: StereoPlayer> MemReadWriter for APU<S> {
    fn read_byte(&self, address: u16) -> u8 {
        match address {
            NR10 => self.ch1.sweep.as_ref().unwrap().read(),
            NR11 => self.ch1.read_duty(),
            NR12 => self.ch1.envelope.read(),
            NR14 => self.ch1.length_timer.read_control(),

            NR21 => self.ch2.read_duty(),
            NR22 => self.ch2.envelope.read(),
            NR24 => self.ch2.length_timer.read_control(),

            NR30 => self.ch3.read_dac_enable(),
            NR32 => self.ch3.read_output_level(),
            NR34 => self.ch3.length_timer.read_control(),
            WAVE_RAM_START_ADDR..=WAVE_RAM_END_ADDR => self.ch3.read_wave_ram(address),

            NR42 => self.ch4.envelope.read(),
            NR43 => self.ch4.read_lfsr(),
            NR44 => self.ch4.length_timer.read_control(),

            NR50 => self.read_volume(),
            NR51 => self.read_panning(),
            NR52 => self.read_master_control(),

            0xFF10..=0xFF3F => 0xFF,
            _ => unreachable!("APU reading address {:#04x}", address),
        }
    }

    fn write_byte(&mut self, address: u16, value: u8) {
        if !self.on
            && !((0xFF30..=0xFF3F).contains(&address)
                || [
                    NR52, /* DMG length timer reg should be writeable when off */
                    NR11, NR21, NR31, NR41,
                ]
                .contains(&address))
        {
            return;
        }

        match address {
            NR10 => self.ch1.write_sweep(value),
            NR11 => self
                .ch1
                .write_timer_and_duty(if self.on { value } else { value & 0x3F }),
            NR12 => self.ch1.write_envelope(value),
            NR13 => self.ch1.period.write_low(value),
            NR14 => {
                self.ch1.period.write_high(value);
                self.ch1.on =
                    self.ch1.length_timer.write_enable(value, self.current_step) && self.ch1.on;
                if value & BIT_7 == BIT_7 {
                    self.ch1.trigger(self.current_step);
                }
            }

            NR21 => self
                .ch2
                .write_timer_and_duty(if self.on { value } else { value & 0x3F }),
            NR22 => self.ch2.write_envelope(value),
            NR23 => self.ch2.period.write_low(value),
            NR24 => {
                self.ch2.period.write_high(value);
                self.ch2.on =
                    self.ch2.length_timer.write_enable(value, self.current_step) && self.ch2.on;
                if value & BIT_7 == BIT_7 {
                    self.ch2.trigger(self.current_step);
                }
            }

            NR30 => self.ch3.write_dac_enable(value),
            NR31 => self.ch3.length_timer.write_initial_length_timer(value),
            NR32 => self.ch3.write_initial_output_level(value),
            NR33 => self.ch3.period.write_low(value),
            NR34 => {
                self.ch3.period.write_high(value);
                self.ch3.on =
                    self.ch3.length_timer.write_enable(value, self.current_step) && self.ch3.on;
                if value & BIT_7 == BIT_7 {
                    self.ch3.trigger(self.current_step);
                }
            }
            WAVE_RAM_START_ADDR..=WAVE_RAM_END_ADDR => self.ch3.write_wave_ram(address, value),

            NR41 => self
                .ch4
                .length_timer
                .write_initial_length_timer(value & SIX_BITS),
            NR42 => self.ch4.write_envelope(value),
            NR43 => self.ch4.write_lfsr(value),
            NR44 => {
                self.ch4.on =
                    self.ch4.length_timer.write_enable(value, self.current_step) && self.ch4.on;
                if value & BIT_7 == BIT_7 {
                    self.ch4.trigger(self.current_step);
                }
            }

            NR50 => self.write_volume(value),
            NR51 => self.write_panning(value),
            NR52 => self.write_master_control(value),

            0xFF10..=0xFF3F => {}
            _ => unreachable!("APU writing address {:#04x} value: {:#04x}", address, value),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_waveram_handle_period() {
        let mut wave_ram = WaveRam::new();

        wave_ram.ram[0] = 0xD1;
        wave_ram.ram[1] = 0xF3;
        wave_ram.ram[2] = 0x7E;

        wave_ram.handle_period();
        assert_eq!(1, wave_ram.sample_index);
        assert_eq!(0x1, wave_ram.sample_buffer);

        wave_ram.handle_period();
        assert_eq!(2, wave_ram.sample_index);
        assert_eq!(0xF, wave_ram.sample_buffer);

        wave_ram.handle_period();
        assert_eq!(3, wave_ram.sample_index);
        assert_eq!(0x3, wave_ram.sample_buffer);

        wave_ram.handle_period();
        assert_eq!(4, wave_ram.sample_index);
        assert_eq!(0x7, wave_ram.sample_buffer);

        wave_ram.sample_index = 31;

        wave_ram.handle_period();
        assert_eq!(0, wave_ram.sample_index);
        assert_eq!(0xD, wave_ram.sample_buffer);
    }
}
