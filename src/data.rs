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

use async_std::stream::Stream;
use crate::{cursive_stepper::CursiveRunnableStepper, mount::Mount, tui::TuiData};
use pointing_utils::uom;
use std::{future::Future, marker::Unpin, pin::Pin, task::{Context, Poll}};
use uom::{si::f64, si::angular_velocity};
use pasts::notify::Notify;

pub mod timers {
    use super::TimerId;

    pub const MAIN: TimerId = 1;
}

pub struct Slewing {
    pub axis1: f64::AngularVelocity,
    pub axis2: f64::AngularVelocity,
}

impl Default for Slewing {
    fn default() -> Slewing {
        Slewing{ axis1: deg_per_s(0.0), axis2: deg_per_s(0.0) }
    }
}

pub fn deg_per_s(value: f64) -> f64::AngularVelocity {
    f64::AngularVelocity::new::<angular_velocity::degree_per_second>(value)
}

pub type TimerId = u64;

pub struct ProgramState {
    pub controllers: Vec<Pin<Box<dyn pasts::notify::Notify<Event = (u64, stick::Event)>>>>,
    pub cursive_stepper: CursiveRunnableStepper,
    pub data_receiver: Pin<Box<dyn pasts::notify::Notify<Event = Option<Result<String, std::io::Error>>>>>,
    pub listener: Pin<Box<dyn pasts::notify::Notify<Event = stick::Controller>>>,
    pub mount: Box<dyn Mount>,
    pub slewing: Slewing,
    pub timers: Vec<Timer>,
    pub tui: Option<TuiData>, // always `Some` after program start
}

impl ProgramState {
    pub fn tui(&self) -> &TuiData { self.tui.as_ref().unwrap() }

    pub fn refresh_tui(&mut self) {
        self.cursive_stepper.curs.refresh();
    }
}

pub struct Timer {
    timer: Pin<Box<dyn pasts::notify::Notify<Event = ()>>>,
    id: TimerId
}

impl Timer {
    pub fn new(id: TimerId, interval: std::time::Duration) -> Timer {
        Timer{
            id,
            timer: Box::pin(pasts::notify::future_fn(
                move || Box::pin(async_std::task::sleep(interval))
            ))
        }
    }
}

impl pasts::notify::Notify for Timer {
    type Event = TimerId;

    fn poll_next(mut self: Pin<&mut Self>, t: &mut std::task::Context<'_>) -> Poll<Self::Event> {
        match Pin::new(&mut self.timer).poll_next(t) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => { Poll::Ready(self.id) }
        }
    }
}
