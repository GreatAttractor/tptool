use std::{collections::VecDeque, pin::Pin, rc::Rc, sync::{Arc, Mutex}, task::{Context, Poll}};
use core::future::Future;

pub struct KeyPoller {
    window: Rc<pancurses::Window>,
    keys: VecDeque<pancurses::Input>,
    waker: Arc<Mutex<Option<core::task::Waker>>>
}

impl KeyPoller {
    pub fn new(window: Rc<pancurses::Window>) -> KeyPoller {
        let waker = Arc::new(Mutex::<Option<core::task::Waker>>::new(None));

        {
            const STDIN_READABLE: usize = 0;
            let waker = waker.clone();
            let poller = polling::Poller::new().unwrap();
            unsafe { poller.add(&std::io::stdin(), polling::Event::readable(STDIN_READABLE)).unwrap(); }

            std::thread::Builder::new().spawn(move || {
                let mut events = polling::Events::new();
                loop {
                    events.clear();
                    poller.wait(&mut events, None).unwrap();
                    if events.iter().find(|i| i.key == STDIN_READABLE).is_some() {
                        match *waker.lock().unwrap() {
                            Some(ref waker) =>  waker.wake_by_ref(),
                            None => ()
                        }
                    }
                    poller.modify(&std::io::stdin(), polling::Event::readable(STDIN_READABLE)).unwrap();
                }
            }).unwrap();
        }

        KeyPoller{
            window,
            keys: VecDeque::new(),
            waker
        }
    }
}

impl Future for KeyPoller {
    type Output = pancurses::Input;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<pancurses::Input> {
        {
            let mut data = self.waker.lock().unwrap();
            if data.is_none() {
                *data = Some(cx.waker().clone());
            }
        }

        loop {
            match self.window.getch() {
                Some(input) => self.keys.push_back(input),
                None => break
            }
        }

        if !self.keys.is_empty() {
            Poll::Ready(self.keys.pop_front().unwrap())
        } else {
            Poll::Pending
        }
    }
}
