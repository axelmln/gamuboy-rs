use std::sync::mpsc;

use crate::joypad::Joypad;

pub trait EventsHandler<T> {
    fn handle_events(&mut self, _rx: &mpsc::Receiver<T>, _joypad: &mut Joypad) {}
}

pub struct Fake;

impl EventsHandler<()> for Fake {}
