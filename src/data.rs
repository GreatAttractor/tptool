use core::future::Future;
use crate::key_poller::KeyPoller;
use std::{pin::Pin, rc::Rc};

pub struct ProgramState {
    pub counter: usize,
    pub window: Rc<pancurses::Window>,
    pub key_poller: KeyPoller,
    pub timer: Pin<Box<dyn Future<Output = ()>>>
}
