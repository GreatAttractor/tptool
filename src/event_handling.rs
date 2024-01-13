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

use core::task::Poll;
use crate::{cursive_stepper::Running, data::ProgramState};

pub async fn event_loop(mut state: ProgramState) {
    pasts::Loop::new(&mut state)
        .when(|s| &mut s.timer, on_timer)
        .when(|s| &mut s.cursive_stepper, on_cursive_step)
        .await;
}

fn on_timer(state: &mut ProgramState, _: ()) -> Poll<()> {
    state.tui.as_ref().unwrap().tick.set_content(format!("tick: {}", state.counter));
    state.cursive_stepper.curs.refresh();
    state.counter += 1;

    Poll::Pending
}

fn on_cursive_step(_: &mut ProgramState, running: Running) -> Poll<()> {
    if running.0 {
        Poll::Pending
    } else {
        Poll::Ready(())
    }
}

fn nop<S, T>(_: &mut S, _: T) -> Poll<T> {
    Poll::Pending
}
