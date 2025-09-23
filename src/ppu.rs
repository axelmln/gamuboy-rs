use std::{
    array,
    time::{Duration, Instant},
};

use crate::{
    config::Config,
    interrupts::InterruptRegisters,
    lcd::{
        self, LCD, PIXELS_HEIGHT, PIXELS_WIDTH, RGB, RGB_BLACK, RGB_DARK_GRAY, RGB_LIGHT_GRAY,
        RGB_WHITE,
    },
    memory::MemReadWriter,
    mode, oam, vram,
};

const OAM_DOTS: u32 = 80;
const VRAM_DOTS: u32 = 172;
const SCANLINE_DOTS: u32 = 456;

const DOTS_PER_FRAME: u32 = 70224;

const BIT_7: u8 = 1 << 7;

pub const BG_COLOR_PALETTE_SPEC_REG: u16 = 0xFF68;
pub const BG_COLOR_PALETTE_DATA_REG: u16 = 0xFF69;

pub const OBJ_COLOR_PALETTE_SPEC_REG: u16 = 0xFF6A;
pub const OBJ_COLOR_PALETTE_DATA_REG: u16 = 0xFF6B;

#[derive(Copy, Clone, PartialEq, Debug)]
enum GrayShade {
    White,
    LightGray,
    DarkGray,
    Black,
}

impl From<u8> for GrayShade {
    fn from(value: u8) -> Self {
        match value & 3 {
            0 => Self::White,
            1 => Self::LightGray,
            2 => Self::DarkGray,
            _ => Self::Black,
        }
    }
}

impl GrayShade {
    fn to_rgb(&self) -> lcd::RGB {
        match self {
            Self::White => RGB_WHITE,
            Self::LightGray => RGB_LIGHT_GRAY,
            Self::DarkGray => RGB_DARK_GRAY,
            Self::Black => RGB_BLACK,
        }
    }
}

fn select_bit(byte: u8, n: u8) -> u8 {
    byte >> n & 1
}

fn get_color_id_from_two_bytes(left: u8, right: u8, i: u8) -> u8 {
    select_bit(right, 7 - i) << 1 | select_bit(left, 7 - i)
}

trait Palette {
    fn get_color_from_id(&self, id: u8) -> lcd::RGB;
}

#[derive(Clone)]
struct MonochromePalette {
    palette: [GrayShade; 4],
}

impl Palette for MonochromePalette {
    fn get_color_from_id(&self, id: u8) -> lcd::RGB {
        self.palette[id as usize].to_rgb()
    }
}

impl MonochromePalette {
    fn new() -> Self {
        Self {
            palette: [
                GrayShade::White,
                GrayShade::LightGray,
                GrayShade::DarkGray,
                GrayShade::Black,
            ],
        }
    }

    fn read(&self) -> u8 {
        (self.palette[3] as u8) << 6
            | (self.palette[2] as u8) << 4
            | (self.palette[1] as u8) << 2
            | self.palette[0] as u8
    }

    fn update(&mut self, value: u8) {
        self.palette[0] = GrayShade::from(value);
        self.palette[1] = GrayShade::from(value >> 2);
        self.palette[2] = GrayShade::from(value >> 4);
        self.palette[3] = GrayShade::from(value >> 6);
    }
}

fn scale_up_color_channel(ch: u8) -> u8 {
    (ch as u16 * 255 / 31) as u8
}

#[derive(Clone)]
struct ColorPalette {
    color_data: [u8; 8],
}

impl Palette for ColorPalette {
    fn get_color_from_id(&self, id: u8) -> lcd::RGB {
        let raw_rgb_bytes = (self.color_data[id as usize * 2 + 1] as u16) << 8
            | self.color_data[id as usize * 2] as u16;
        let mask = 0b11111;
        let red = scale_up_color_channel(raw_rgb_bytes as u8 & mask);
        let green = scale_up_color_channel((raw_rgb_bytes >> 5) as u8 & mask);
        let blue = scale_up_color_channel((raw_rgb_bytes >> 10) as u8 & mask);

        (red, green, blue)
    }
}

impl ColorPalette {
    fn new() -> Self {
        Self {
            color_data: [
                0xFF, 0xFF, // white
                0xB5, 0x56, // light gray
                0x4A, 0x29, // dark gray
                0x00, 0x00, // black
            ],
        }
    }

    fn write(&mut self, address: u8, value: u8) {
        self.color_data[address as usize % 8] = value;
    }

    fn read(&self, address: u8) -> u8 {
        self.color_data[address as usize % 8]
    }
}

#[derive(Clone)]
struct ColorPaletteRAM {
    spec: ColorPaletteSpec,
    data: [ColorPalette; 8],
}

impl ColorPaletteRAM {
    fn new() -> Self {
        Self {
            spec: ColorPaletteSpec::new(),
            data: array::from_fn(|_| ColorPalette::new()),
        }
    }

    fn write_spec(&mut self, value: u8) {
        self.spec.write(value);
    }

    fn read_spec(&self) -> u8 {
        self.spec.read()
    }

    fn write_data(&mut self, value: u8) {
        self.data[self.spec.address as usize / 8].write(self.spec.address % 8, value);
        self.spec.address += self.spec.auto_inc as u8;
        self.spec.address %= 64;
    }

    fn read_data(&self) -> u8 {
        self.get_palette(self.spec.address / 8)
            .read(self.spec.address % 8)
    }

    fn get_palette(&self, i: u8) -> ColorPalette {
        self.data[i as usize].clone()
    }
}

#[derive(Clone)]
struct ColorPaletteSpec {
    auto_inc: bool,
    address: u8,
}

impl ColorPaletteSpec {
    fn new() -> Self {
        Self {
            auto_inc: false,
            address: 0,
        }
    }

    fn read(&self) -> u8 {
        (self.auto_inc as u8) << 7 | self.address
    }

    fn write(&mut self, value: u8) {
        self.auto_inc = value & BIT_7 == BIT_7;
        self.address = value & 0x3F;
    }
}

#[derive(Clone, PartialEq, Debug)]
enum BGWinTileMapArea {
    First = 0x9800,
    Second = 0x9C00,
}

impl From<u8> for BGWinTileMapArea {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::First,
            _ => Self::Second,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
enum BGWinTileDataArea {
    First = 0x9000,
    Second = 0x8000,
}

impl From<u8> for BGWinTileDataArea {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::First,
            _ => Self::Second,
        }
    }
}

impl BGWinTileDataArea {
    fn get_tile_address(&self, index: u8) -> u16 {
        match self {
            BGWinTileDataArea::First => {
                let signed_index = index as i8;
                (BGWinTileDataArea::First as i16 + (signed_index as i16 * 16)) as u16
            }
            BGWinTileDataArea::Second => BGWinTileDataArea::Second as u16 + index as u16 * 16,
        }
    }
}

#[derive(Clone, Debug)]
struct LCDC {
    bg_win_enable: bool,
    obj_enable: bool,
    double_height_obj: bool,
    bg_tile_map_area: BGWinTileMapArea,
    bg_win_tile_data_area: BGWinTileDataArea,
    win_enable: bool,
    win_tile_map_area: BGWinTileMapArea,
    lcd_ppu_enable: bool,
}

impl LCDC {
    fn new() -> Self {
        Self {
            bg_win_enable: false,
            obj_enable: false,
            double_height_obj: false,
            bg_tile_map_area: BGWinTileMapArea::First,
            bg_win_tile_data_area: BGWinTileDataArea::First,
            win_enable: false,
            win_tile_map_area: BGWinTileMapArea::First,
            lcd_ppu_enable: false,
        }
    }

    fn read(&self) -> u8 {
        self.bg_win_enable as u8
            | (self.obj_enable as u8) << 1
            | (self.double_height_obj as u8) << 2
            | ((self.bg_tile_map_area == BGWinTileMapArea::Second) as u8) << 3
            | ((self.bg_win_tile_data_area == BGWinTileDataArea::Second) as u8) << 4
            | (self.win_enable as u8) << 5
            | ((self.win_tile_map_area == BGWinTileMapArea::Second) as u8) << 6
            | (self.lcd_ppu_enable as u8) << 7
    }

    fn write(&mut self, value: u8) {
        self.bg_win_enable = value & 1 != 0;
        self.obj_enable = value & 2 != 0;
        self.double_height_obj = value & 4 != 0;
        self.bg_tile_map_area = BGWinTileMapArea::from(value & 8);
        self.bg_win_tile_data_area = BGWinTileDataArea::from(value & 16);
        self.win_enable = value & 32 != 0;
        self.win_tile_map_area = BGWinTileMapArea::from(value & 64);
        self.lcd_ppu_enable = value & 128 != 0;
    }
}

#[derive(Clone, Debug)]
enum Mode {
    HBlank = 0,
    VBlank = 1,
    OAM = 2,
    VRAM = 3,
}

#[derive(Clone)]
struct Stat {
    hblank_int_select: bool,
    vblank_int_select: bool,
    oam_int_select: bool,
    lyc_int_select: bool,
}

impl Stat {
    fn new() -> Self {
        Self {
            hblank_int_select: false,
            vblank_int_select: false,
            oam_int_select: false,
            lyc_int_select: false,
        }
    }

    fn read(&self, mode: Mode, lyc: bool) -> u8 {
        mode as u8
            | (lyc as u8) << 2
            | (self.hblank_int_select as u8) << 3
            | (self.vblank_int_select as u8) << 4
            | (self.oam_int_select as u8) << 5
            | (self.lyc_int_select as u8) << 6
    }

    fn write(&mut self, value: u8) {
        self.hblank_int_select = value & 8 != 0;
        self.vblank_int_select = value & 16 != 0;
        self.oam_int_select = value & 32 != 0;
        self.lyc_int_select = value & 64 != 0;
    }
}

#[derive(Clone)]
struct ObjectFlags {
    cgb_palette: u8,
    vram_bank: u8,
    dmg_palette: u8,
    x_flip: bool,
    y_flip: bool,
    bg_win_priority: bool,
}

impl From<u8> for ObjectFlags {
    fn from(value: u8) -> Self {
        Self {
            cgb_palette: value & 3,
            vram_bank: value >> 3 & 1,
            dmg_palette: value >> 4 & 1,
            x_flip: value >> 5 & 1 != 0,
            y_flip: value >> 6 & 1 != 0,
            bg_win_priority: value >> 7 & 1 != 0,
        }
    }
}

#[derive(Clone)]
struct ObjectAttributes {
    y_pos: u8,
    x_pos: u8,
    tile_index: u8,
    flags: ObjectFlags,
}

impl ObjectAttributes {
    fn new(oam: &oam::OAM, address: u16) -> Self {
        let y_pos = oam.read_byte(address);
        let x_pos = oam.read_byte(address.wrapping_add(1));
        let tile_index = oam.read_byte(address.wrapping_add(2));
        let flags_byte = oam.read_byte(address.wrapping_add(3));
        Self {
            y_pos,
            x_pos,
            tile_index,
            flags: ObjectFlags::from(flags_byte),
        }
    }
}

struct BGMapAttributes {
    priority: bool,
    y_flip: bool,
    x_flip: bool,
    bank: u8,
    color_palette: u8,
}

impl BGMapAttributes {
    fn new(value: u8) -> Self {
        Self {
            priority: value >> 7 & 1 == 1,
            y_flip: value >> 6 & 1 == 1,
            x_flip: value >> 5 & 1 == 1,
            bank: value >> 3 & 1,
            color_palette: value & 0b111,
        }
    }
}

#[derive(Clone)]
pub enum DMARequest {
    OAM(u8),
    VRAM {
        src: u16,
        dst: u16,
        len: u16,
        is_hdma: bool,
    },
}

#[derive(Clone)]
pub struct PPU<L: LCD + 'static> {
    gb_mode: mode::Mode,

    headless_mode: bool,

    dots: u32,

    frame_buffer: Vec<Vec<RGB>>,

    vram: vram::VRAM,
    oam: oam::OAM,

    lcd: L,

    lcdc: LCDC,

    ly: u8,
    lyc: u8,

    mode: Mode,

    stat: Stat,
    /// https://gbdev.io/pandocs/Interrupt_Sources.html#int-48--stat-interrupt
    stat_int_line: bool,

    scy: u8,
    scx: u8,

    wy: u8,
    wx: u8,

    monochrome_bg_palette: MonochromePalette,
    monochrome_obj_palettes: [MonochromePalette; 2],

    bg_palette_ram: ColorPaletteRAM,
    obj_palette_ram: ColorPaletteRAM,

    line_objects: Vec<ObjectAttributes>,

    dma_request: Option<DMARequest>,
    pending_dma_request: Option<DMARequest>,

    frame_cycles_acc: u32,

    last_frame_instant: Instant,

    high_vram_dma_src: u8,
    low_vram_dma_src: u8,

    high_vram_dma_dst: u8,
    low_vram_dma_dst: u8,

    vram_dma_transfer_len: u8,
}

impl<L: lcd::LCD> PPU<L> {
    pub fn new(cfg: &Config, vram: vram::VRAM, oam: oam::OAM, lcd: L) -> Self {
        let skip_boot = cfg.bootrom.is_none();

        Self {
            gb_mode: cfg.mode.clone(),

            headless_mode: cfg.headless_mode,

            dots: 0,

            frame_buffer: vec![vec![(0, 0, 0); PIXELS_WIDTH]; PIXELS_HEIGHT],

            vram,
            oam,

            lcd,

            lcdc: if skip_boot {
                let mut lcdc = LCDC::new();
                lcdc.write(0x91); // https://gbdev.io/pandocs/Power_Up_Sequence.html
                lcdc
            } else {
                LCDC::new()
            },

            ly: 0,
            lyc: 0,

            mode: Mode::OAM,

            stat: if skip_boot {
                let mut stat = Stat::new();
                stat.write(0x85); // https://gbdev.io/pandocs/Power_Up_Sequence.html
                stat
            } else {
                Stat::new()
            },
            stat_int_line: false,

            scy: 0,
            scx: 0,

            wy: 0,
            wx: 0,

            monochrome_bg_palette: MonochromePalette::new(),
            monochrome_obj_palettes: array::from_fn(|_| MonochromePalette::new()),

            bg_palette_ram: ColorPaletteRAM::new(),
            obj_palette_ram: ColorPaletteRAM::new(),

            line_objects: vec![],

            dma_request: None,
            pending_dma_request: None,

            frame_cycles_acc: 0,

            last_frame_instant: Instant::now(),

            high_vram_dma_src: 0,
            low_vram_dma_src: 0,

            high_vram_dma_dst: 0,
            low_vram_dma_dst: 0,

            vram_dma_transfer_len: 0,
        }
    }

    fn draw_frame_buffer(&mut self) {
        if !self.headless_mode {
            self.lcd.draw_buffer(&self.frame_buffer);
        }
    }

    fn is_win_enabled(&self) -> bool {
        self.lcdc.win_enable && self.ly >= self.wy
    }

    fn get_bg_palette(&self, tile_attr: Option<BGMapAttributes>) -> Box<dyn Palette> {
        match self.gb_mode {
            mode::Mode::DMG => Box::new(self.monochrome_bg_palette.clone()),
            mode::Mode::CGB => Box::new(
                self.bg_palette_ram
                    .get_palette(tile_attr.unwrap().color_palette),
            ),
        }
    }

    fn get_obj_palette(&self, obj_attr: &ObjectAttributes) -> Box<dyn Palette> {
        match self.gb_mode {
            mode::Mode::DMG => {
                Box::new(self.monochrome_obj_palettes[obj_attr.flags.dmg_palette as usize].clone())
            }
            mode::Mode::CGB => {
                Box::new(self.obj_palette_ram.get_palette(obj_attr.flags.cgb_palette))
            }
        }
    }

    fn buffer_pix_bg(&mut self, x: u8, bg_win_color_id: &mut u8) {
        if !self.lcdc.bg_win_enable {
            return;
        }

        let scroll_y = self.scy.wrapping_add(self.ly);
        let tile_y = scroll_y as u16 / 8 * 32;

        let scroll_x = self.scx.wrapping_add(x);
        let tile_x = scroll_x / 8;

        let base_map_addr = self.lcdc.bg_tile_map_area.clone() as u16;

        self.switch_vram_bank(0);
        let tile_index = self
            .vram
            .read_byte(base_map_addr + tile_y as u16 + tile_x as u16);

        let tile_attributes =
            self.get_bg_tile_attributes(base_map_addr + tile_y as u16 + tile_x as u16);

        let tile_addr = self.lcdc.bg_win_tile_data_area.get_tile_address(tile_index);

        let tile_y_offset = scroll_y % 8 * 2;
        let color_id = get_color_id_from_two_bytes(
            self.vram.read_byte(tile_addr + tile_y_offset as u16),
            self.vram.read_byte(tile_addr + tile_y_offset as u16 + 1),
            scroll_x % 8,
        );
        *bg_win_color_id = color_id;

        let pixel = self
            .get_bg_palette(tile_attributes)
            .get_color_from_id(color_id);
        self.frame_buffer[self.ly as usize][x as usize] = pixel;
    }

    fn buffer_pix_win(&mut self, x: u8, bg_win_color_id: &mut u8) {
        if !self.lcdc.bg_win_enable || !self.is_win_enabled() {
            return;
        }

        let wx = if self.wx < 7 { 0 } else { self.wx - 7 };
        if x < wx {
            return;
        }

        let win_y = self.ly - self.wy;
        let tile_y = win_y as u16 / 8 * 32;
        let tile_y_offset = win_y % 8 * 2;

        let win_x = x - wx;
        let tile_x = win_x as u16 / 8;

        let base_map_addr = self.lcdc.win_tile_map_area.clone() as u16;

        self.switch_vram_bank(0);
        let tile_index = self.vram.read_byte(base_map_addr + tile_y + tile_x);

        let tile_attributes =
            self.get_bg_tile_attributes(base_map_addr + tile_y as u16 + tile_x as u16);

        let tile_addr = self.lcdc.bg_win_tile_data_area.get_tile_address(tile_index);

        let color_id = get_color_id_from_two_bytes(
            self.vram.read_byte(tile_addr + tile_y_offset as u16),
            self.vram.read_byte(tile_addr + tile_y_offset as u16 + 1),
            win_x % 8,
        );
        *bg_win_color_id = color_id;

        let pixel = self
            .get_bg_palette(tile_attributes)
            .get_color_from_id(color_id);
        self.frame_buffer[self.ly as usize][x as usize] = pixel;
    }

    fn buffer_pix_obj(&mut self, x: u8, bg_win_color_id: u8) {
        if !self.lcdc.obj_enable {
            return;
        }

        for obj_attr in &self.line_objects {
            if obj_attr.flags.bg_win_priority && bg_win_color_id != 0 {
                continue;
            }

            let is_in_tile = x as isize >= obj_attr.x_pos as isize - 8 && x < obj_attr.x_pos;
            if !is_in_tile {
                continue;
            }

            let obj_height = 8 * (self.lcdc.double_height_obj as u8 + 1);

            let obj_y = self.ly + 16 - obj_attr.y_pos;
            let obj_y = if obj_attr.flags.y_flip {
                obj_height - 1 - obj_y
            } else {
                obj_y
            };

            let tile_index = if !self.lcdc.double_height_obj {
                obj_attr.tile_index
            } else if obj_y < 8 {
                obj_attr.tile_index & 0xFE
            } else {
                obj_attr.tile_index | 1
            };

            let y_offset = (obj_y as u16 % 8) * 2;
            let tile_addr = vram::BASE_ADDRESS + tile_index as u16 * 16 + y_offset;

            match self.gb_mode {
                // In CGB Mode this could be either in VRAM bank 0 or 1, depending on bit 3 of the following byte. (https://gbdev.io/pandocs/OAM.html#byte-2--tile-index)
                mode::Mode::CGB => {
                    self.vram
                        .write_byte(vram::BANK_REGISTER, obj_attr.flags.vram_bank);
                }
                _ => {}
            }

            let tile_high_byte = self.vram.read_byte(tile_addr);
            let tile_low_byte = self.vram.read_byte(tile_addr + 1);

            let obj_x = x + 8 - obj_attr.x_pos;
            let obj_x = if obj_attr.flags.x_flip {
                7 - obj_x
            } else {
                obj_x
            };

            let color_id = get_color_id_from_two_bytes(tile_high_byte, tile_low_byte, obj_x);
            if color_id == 0 {
                continue;
            }

            let pixel = self.get_obj_palette(obj_attr).get_color_from_id(color_id);

            self.frame_buffer[self.ly as usize][x as usize] = pixel;
        }
    }

    fn buffer_pix(&mut self, x: u8) {
        if !self.lcdc.lcd_ppu_enable {
            return;
        }

        let mut bg_win_color_id = 0;

        self.buffer_pix_bg(x, &mut bg_win_color_id);
        self.buffer_pix_win(x, &mut bg_win_color_id);
        self.buffer_pix_obj(x, bg_win_color_id);
    }

    fn buffer_line(&mut self) {
        for x in 0..PIXELS_WIDTH as u8 {
            self.buffer_pix(x);
        }
    }

    fn search_line_objects(&mut self) {
        self.line_objects = vec![];
        for i in 0..40 {
            let obj_attr = ObjectAttributes::new(&self.oam, oam::BASE_ADDRESS + i * 4);

            if self.ly + 16 < obj_attr.y_pos {
                continue;
            }

            let obj_height = 8 * (self.lcdc.double_height_obj as u8 + 1);
            if self.ly + 16 - obj_attr.y_pos >= obj_height {
                continue;
            }

            self.line_objects.push(obj_attr);
        }

        self.line_objects.sort_by_key(|o| o.x_pos);
        self.line_objects.truncate(10);
    }

    fn get_bg_tile_attributes(&mut self, address: u16) -> Option<BGMapAttributes> {
        match self.gb_mode {
            mode::Mode::CGB => {
                self.switch_vram_bank(1);
                let tile_attributes_byte = self.vram.read_byte(address);
                let tile_attributes = BGMapAttributes::new(tile_attributes_byte);
                self.switch_vram_bank(tile_attributes.bank);

                Some(tile_attributes)
            }
            mode::Mode::DMG => None,
        }
    }

    fn switch_vram_bank(&mut self, bank: u8) {
        match self.gb_mode {
            mode::Mode::CGB => self.vram.write_byte(vram::BANK_REGISTER, bank),
            _ => {}
        }
    }

    fn inc_ly(&mut self) {
        self.ly += 1;
    }

    fn update_stat_line(&mut self) {
        self.stat_int_line = (self.mode.clone() as u8 == Mode::HBlank as u8
            && self.stat.hblank_int_select)
            || (self.mode.clone() as u8 == Mode::OAM as u8 && self.stat.oam_int_select)
            || (self.mode.clone() as u8 == Mode::VBlank as u8 && self.stat.vblank_int_select)
            || (self.ly == self.lyc && self.stat.lyc_int_select);
    }

    fn handle_stat_int(&mut self, int_reg: &mut InterruptRegisters) {
        let prev_stat_line = self.stat_int_line;
        self.update_stat_line();
        if !prev_stat_line && self.stat_int_line {
            int_reg.request_stat_lcd();
        }
    }

    fn enter_vblank(&mut self, int_reg: &mut InterruptRegisters) {
        int_reg.request_vblank();
        self.mode = Mode::VBlank;
    }

    fn enter_oam(&mut self) {
        self.mode = Mode::OAM;
    }

    fn handle_oam_mode(&mut self) {
        if self.dots < OAM_DOTS {
            return;
        }

        self.search_line_objects();

        self.mode = Mode::VRAM;
    }

    /// https://gbdev.io/pandocs/Rendering.html#mode-3-length
    fn compute_vram_mode_penalty(&self) -> u32 {
        // quick attempt, probably not very accurate
        // (self.scx as u32 % 8)
        //     + ((self.is_win_enabled() && self.wx > 0) as u32 * 6)
        //     + (self.line_objects.iter().fold(0, |acc, _o| acc + 1) * 6)
        0
    }

    fn handle_vram_mode(&mut self) {
        if self.dots < OAM_DOTS + VRAM_DOTS + self.compute_vram_mode_penalty() {
            return;
        }

        self.buffer_line();

        self.mode = Mode::HBlank;
    }

    fn handle_hblank_mode(&mut self, int_reg: &mut InterruptRegisters) {
        if self.dots < SCANLINE_DOTS {
            return;
        }

        self.dots -= SCANLINE_DOTS;
        self.inc_ly();

        match self.pending_dma_request {
            Some(DMARequest::VRAM { is_hdma: true, .. }) => {
                self.dma_request = self.pending_dma_request.clone()
            }
            _ => {}
        }

        if self.ly >= PIXELS_HEIGHT as u8 {
            self.enter_vblank(int_reg);
        } else {
            self.enter_oam();
        }
    }

    fn handle_vblank_mode(&mut self) {
        if self.dots < SCANLINE_DOTS {
            return;
        }
        self.dots -= SCANLINE_DOTS;
        if self.ly == 153 {
            self.enter_oam();
            self.ly = 0;
        } else {
            self.inc_ly();
        }
    }

    fn enable(&mut self) {
        self.dots = 0;
        self.ly = 0;
        self.mode = Mode::OAM;
    }

    fn cap_fps(&mut self) {
        const FRAME_DURATION: Duration = Duration::from_nanos((1_000_000_000.0 / 59.73) as u64);
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_instant);
        if elapsed < FRAME_DURATION {
            std::thread::sleep(FRAME_DURATION - elapsed);
        }
        self.last_frame_instant = Instant::now();
    }

    pub fn write_oam(&mut self, address: u16, value: u8) {
        self.oam.write_byte(address, value);
    }

    pub fn write_vram(&mut self, address: u16, value: u8) {
        self.vram.write_byte(address, value);
    }

    pub fn check_dma_request(&mut self) -> Option<DMARequest> {
        let req = self.dma_request.clone();
        match req {
            Some(DMARequest::VRAM {
                src,
                dst,
                is_hdma: true,
                ..
            }) => {
                if self.vram_dma_transfer_len == 0 {
                    self.pending_dma_request = None;
                } else {
                    self.vram_dma_transfer_len -= 1;
                    self.pending_dma_request = Some(DMARequest::VRAM {
                        src: src + 0x10,
                        dst: dst + 0x10,
                        len: 0x10,
                        is_hdma: true,
                    });
                }
            }
            _ => {}
        }

        self.dma_request = None;

        req
    }

    pub fn step(&mut self, int_reg: &mut InterruptRegisters, cycles: u8) {
        if !self.lcdc.lcd_ppu_enable {
            return;
        }

        self.dots += cycles as u32;

        match self.mode {
            Mode::OAM => self.handle_oam_mode(),
            Mode::VRAM => self.handle_vram_mode(),
            Mode::HBlank => self.handle_hblank_mode(int_reg),
            Mode::VBlank => self.handle_vblank_mode(),
        }

        self.handle_stat_int(int_reg);

        self.frame_cycles_acc = self.frame_cycles_acc.wrapping_add(cycles as u32);
        if self.frame_cycles_acc >= DOTS_PER_FRAME {
            self.frame_cycles_acc -= DOTS_PER_FRAME;
            self.draw_frame_buffer();

            self.cap_fps();
        }
    }
}

impl<L: lcd::LCD> MemReadWriter for PPU<L> {
    fn read_byte(&self, address: u16) -> u8 {
        match self.gb_mode {
            mode::Mode::CGB => match address {
                0xFF51..=0xFF54 => return 0xFF,
                0xFF55 => {
                    if self.dma_request.is_some() {
                        return self.vram_dma_transfer_len;
                    }
                    return 0xFF;
                }

                BG_COLOR_PALETTE_SPEC_REG => return self.bg_palette_ram.read_spec(),
                BG_COLOR_PALETTE_DATA_REG => return self.bg_palette_ram.read_data(),

                OBJ_COLOR_PALETTE_SPEC_REG => return self.obj_palette_ram.read_spec(),
                OBJ_COLOR_PALETTE_DATA_REG => return self.obj_palette_ram.read_data(),

                _ => {}
            },
            _ => {}
        }

        match address {
            0x8000..=0x9FFF | 0xFF4F => self.vram.read_byte(address),
            0xFE00..=0xFE9F => self.oam.read_byte(address),
            0xFF40 => self.lcdc.read(),
            0xFF41 => self.stat.read(self.mode.clone(), self.lyc == self.ly),
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF46 => 0xFF,
            0xFF47 => self.monochrome_bg_palette.read(),
            0xFF48 => self.monochrome_obj_palettes[0].read(),
            0xFF49 => self.monochrome_obj_palettes[1].read(),
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => unimplemented!("PPU: reading address: {:#04x}", address),
        }
    }
    fn write_byte(&mut self, address: u16, value: u8) {
        match self.gb_mode {
            mode::Mode::CGB => match address {
                0xFF51 => return self.high_vram_dma_src = value,
                0xFF52 => return self.low_vram_dma_src = value,
                0xFF53 => return self.high_vram_dma_dst = value,
                0xFF54 => return self.low_vram_dma_dst = value,
                0xFF55 => {
                    let src = (self.high_vram_dma_src as u16) << 8 | self.low_vram_dma_src as u16;
                    let dst = (self.high_vram_dma_dst as u16) << 8 | self.low_vram_dma_dst as u16;
                    self.vram_dma_transfer_len = value & 0x7F;
                    let len = (self.vram_dma_transfer_len as u16 + 1) * 0x10;
                    let is_hdma = value & BIT_7 == 1;
                    let req = DMARequest::VRAM {
                        src,
                        dst,
                        len,
                        is_hdma,
                    };
                    if is_hdma {
                        return self.pending_dma_request = Some(req);
                    }
                    return self.dma_request = Some(req);
                }

                BG_COLOR_PALETTE_SPEC_REG => return self.bg_palette_ram.write_spec(value),
                BG_COLOR_PALETTE_DATA_REG => return self.bg_palette_ram.write_data(value),

                OBJ_COLOR_PALETTE_SPEC_REG => return self.obj_palette_ram.write_spec(value),
                OBJ_COLOR_PALETTE_DATA_REG => return self.obj_palette_ram.write_data(value),

                _ => {}
            },
            _ => {}
        }

        match address {
            0x8000..=0x9FFF | 0xFF4F => self.vram.write_byte(address, value),
            0xFE00..=0xFE9F => self.oam.write_byte(address, value),
            0xFF40 => {
                let was_enabled = self.lcdc.lcd_ppu_enable;

                self.lcdc.write(value);

                if !was_enabled && self.lcdc.lcd_ppu_enable {
                    self.enable();
                }
            }
            0xFF41 => self.stat.write(value),
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => self.ly = 0,
            0xFF45 => self.lyc = value,
            0xFF46 => self.dma_request = Some(DMARequest::OAM(value)),
            0xFF47 => self.monochrome_bg_palette.update(value),
            0xFF48 => self.monochrome_obj_palettes[0].update(value),
            0xFF49 => self.monochrome_obj_palettes[1].update(value),
            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,
            _ => unimplemented!("PPU: writing to address: {:#04x}", address),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{oam::OAM, vram::VRAM};

    use super::*;

    #[test]
    fn test_get_color_id_from_two_bytes() {
        // https://gbdev.io/pandocs/Tile_Data.html#data-format

        struct TestCase {
            bytes: (u8, u8),
            expected: [u8; 8],
        }
        let test_cases = &[
            TestCase {
                bytes: (0x3C, 0x7E),
                expected: [0b00, 0b10, 0b11, 0b11, 0b11, 0b11, 0b10, 0b00],
            },
            TestCase {
                bytes: (0x42, 0x42),
                expected: [0b00, 0b11, 0b00, 0b00, 0b00, 0b00, 0b11, 0b00],
            },
            TestCase {
                bytes: (0x7E, 0x5E),
                expected: [0b00, 0b11, 0b01, 0b11, 0b11, 0b11, 0b11, 0b00],
            },
            TestCase {
                bytes: (0x7E, 0x0A),
                expected: [0b00, 0b01, 0b01, 0b01, 0b11, 0b01, 0b11, 0b00],
            },
            TestCase {
                bytes: (0x7C, 0x56),
                expected: [0b00, 0b11, 0b01, 0b11, 0b01, 0b11, 0b10, 0b00],
            },
            TestCase {
                bytes: (0x38, 0x7C),
                expected: [0b00, 0b10, 0b11, 0b11, 0b11, 0b10, 0b00, 0b00],
            },
        ];
        for tc in test_cases {
            for (i, e) in tc.expected.into_iter().enumerate() {
                let got = get_color_id_from_two_bytes(tc.bytes.0, tc.bytes.1, i as u8);
                assert_eq!(e, got);
            }
        }
    }

    #[test]
    fn test_cgb_bg_color_palette_access() {
        cgb_color_palette_access_no_auto_inc(BG_COLOR_PALETTE_SPEC_REG, BG_COLOR_PALETTE_DATA_REG);
        cgb_color_palette_access_with_auto_inc(
            BG_COLOR_PALETTE_SPEC_REG,
            BG_COLOR_PALETTE_DATA_REG,
        );
    }

    #[test]
    fn test_cgb_obj_color_palette_access() {
        cgb_color_palette_access_no_auto_inc(
            OBJ_COLOR_PALETTE_SPEC_REG,
            OBJ_COLOR_PALETTE_DATA_REG,
        );
        cgb_color_palette_access_with_auto_inc(
            OBJ_COLOR_PALETTE_SPEC_REG,
            OBJ_COLOR_PALETTE_DATA_REG,
        );
    }

    struct DummyLCD;
    impl LCD for DummyLCD {}

    fn cgb_color_palette_access_no_auto_inc(spec_reg: u16, data_reg: u16) {
        let mut ppu = PPU::new(
            &Config {
                mode: mode::Mode::CGB,
                rom: vec![],
                headless_mode: false,
                bootrom: None,
                log_file_path: None,
            },
            VRAM::new(mode::Mode::CGB),
            OAM::new(),
            DummyLCD,
        );

        for i in 0..64 {
            ppu.write_byte(spec_reg, i);
            ppu.write_byte(data_reg, i);
        }

        for i in 0..64 {
            ppu.write_byte(spec_reg, i);
            let expected = i;
            assert_eq!(expected, ppu.read_byte(data_reg));
        }
    }

    fn cgb_color_palette_access_with_auto_inc(spec_reg: u16, data_reg: u16) {
        let mut ppu = PPU::new(
            &Config {
                mode: mode::Mode::CGB,
                rom: vec![],
                headless_mode: false,
                bootrom: None,
                log_file_path: None,
            },
            VRAM::new(mode::Mode::CGB),
            OAM::new(),
            DummyLCD,
        );

        let auto_inc_bit = BIT_7;
        ppu.write_byte(spec_reg, auto_inc_bit);
        for i in 0..64 {
            ppu.write_byte(data_reg, i);
        }

        let auto_inc_bit = BIT_7;
        ppu.write_byte(spec_reg, auto_inc_bit);
        for i in 0..65 {
            let expected = i % 64;
            assert_eq!(expected, ppu.read_byte(data_reg));
            ppu.write_byte(data_reg, 0); // trigger inc
        }
    }
}
