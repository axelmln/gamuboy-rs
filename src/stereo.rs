pub trait StereoPlayer {
    fn play(&self, _buffer: &[f32]) {}
}

pub struct Fake;

impl StereoPlayer for Fake {}
