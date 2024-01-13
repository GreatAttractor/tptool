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

use core::{future::Future, task::{Context, Poll}};
use cursive::{CursiveRunnable, CursiveRunner};
use std::pin::Pin;

pub struct CursiveRunnableStepper {
    pub curs: CursiveRunner<CursiveRunnable>
}

pub struct Running(pub bool);

impl Future for CursiveRunnableStepper {
    type Output = Running;

    fn poll(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Running> {
        if self.curs.is_running() {
            let received_something = self.curs.process_events();
            self.curs.post_events(received_something);
            Poll::Ready(Running(true))
        } else {
            Poll::Ready(Running(false))
        }
    }
}
