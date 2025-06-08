use std::{cell::RefCell, rc::Rc};

pub type FrameBuffer = Vec<Vec<RGB>>;

pub type RGB = (u8, u8, u8);

pub const PIXELS_HEIGHT: usize = 144;
pub const PIXELS_WIDTH: usize = 160;

pub const RGB_WHITE: RGB = (255, 255, 255);
pub const RGB_LIGHT_GRAY: RGB = (170, 170, 170);
pub const RGB_DARK_GRAY: RGB = (85, 85, 85);
pub const RGB_BLACK: RGB = (0, 0, 0);

pub trait LCD {
    fn draw_buffer(&mut self, matrix: &FrameBuffer);
}

pub struct Fake {
    output: Rc<RefCell<String>>,
}

impl Fake {
    pub fn new(output: Rc<RefCell<String>>) -> Self {
        Self { output }
    }
}

impl LCD for Fake {
    fn draw_buffer(&mut self, matrix: &FrameBuffer) {
        let mut output = "".to_owned();
        for line in matrix {
            for pixel in line {
                match pixel {
                    &RGB_WHITE => output.push(' '),
                    &RGB_LIGHT_GRAY => output.push('.'),
                    &RGB_DARK_GRAY => output.push('o'),
                    &RGB_BLACK => output.push('#'),
                    _ => unreachable!(),
                }
            }
            output.push('\n');
        }
        self.output.replace(output);
    }
}
