pub type FrameBuffer = Vec<Vec<RGB>>;

pub type RGB = (u8, u8, u8);

pub const PIXELS_HEIGHT: usize = 144;
pub const PIXELS_WIDTH: usize = 160;

pub const RGB_WHITE: RGB = (255, 255, 255);
pub const RGB_LIGHT_GRAY: RGB = (170, 170, 170);
pub const RGB_DARK_GRAY: RGB = (85, 85, 85);
pub const RGB_BLACK: RGB = (0, 0, 0);

pub trait LCD {
    fn draw_buffer(&mut self, _matrix: &FrameBuffer) {}
}
