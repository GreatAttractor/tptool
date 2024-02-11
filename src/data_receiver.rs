// TPTool (Telescope Pointing Tool) — following a target in the sky
// Copyright (C) 2024 Filip Szczerek <ga.software@yahoo.com>
//
// This file is part of TPTool
//
// TPTool is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License version 3
// as published by the Free Software Foundation.
//
// TPTool is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with TPTool.  If not, see <http://www.gnu.org/licenses/>.
//

use async_std::{io::prelude::BufReadExt, stream::Stream};
use crate::data;
use pasts::notify::Notify;
use std::{cell::RefCell, error::Error, pin::Pin, rc::{Rc, Weak}, task::{Context, Poll}};

pub struct DataReceiver {
    source: Rc<RefCell<Option<Pin<Box<dyn Notify<Event = Option<Result<String, std::io::Error>>>>>>>>
}

impl DataReceiver {
    pub fn new() -> DataReceiver {
        DataReceiver{ source: Rc::new(RefCell::new(None)) }
    }

    pub fn connection(&self) -> Connection {
        Connection{ source: Rc::downgrade(&self.source) }
    }
}

#[derive(Clone)]
pub struct Connection {
    source: Weak<RefCell<Option<Pin<Box<dyn Notify<Event = Option<Result<String, std::io::Error>>>>>>>>
}

impl Connection {
    #[must_use]
    pub fn connect(&self, address: &str) -> Result<(), Box<dyn Error>> {
        let stream = futures::executor::block_on(
            async { async_std::net::TcpStream::connect(address).await }
        )?;

        let mut lines = async_std::io::BufReader::new(stream).lines();
        *self.source.upgrade().unwrap().borrow_mut() = Some(Box::pin(
            pasts::notify::poll_fn(move |ctx| Pin::new(&mut lines).poll_next(ctx))
        ));

        Ok(())
    }

    pub fn disconnect(&self) {
        if let Some(source) = self.source.upgrade() {
            *source.borrow_mut() = None;
        }
    }
}

impl data::WeakWrapper for Connection {}

impl Notify for DataReceiver {
    type Event = Result<String, std::io::Error>;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Event> {
        let mut must_close = false;

        let result = match &mut *self.source.borrow_mut() {
            None => Poll::Pending,
            Some(s) => {
                match Pin::new(s).poll_next(ctx) {
                    Poll::Ready(result) => match result {
                        Some(data) => Poll::Ready(data),
                        None => {
                            must_close = true;
                            Poll::Pending
                        }
                    },
                    Poll::Pending => Poll::Pending
                }
            }
        };

        if must_close { *self.source.borrow_mut() = None; }

        result
    }
}
