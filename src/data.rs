use crate::key_poller::KeyPoller;
use std::rc::Rc;

pub struct ProgramState {
    pub window: Rc<pancurses::Window>,
    pub key_poller: KeyPoller,
}
