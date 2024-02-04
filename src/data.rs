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
use crate::{cursive_stepper::CursiveRunnableStepper, mount::Mount, tracking::Tracking, tui::TuiData};
use pointing_utils::uom;
use std::{cell::RefCell, future::Future, marker::Unpin, pin::Pin, rc::Rc, task::{Context, Poll}};
use uom::{si::f64, si::{angle, angular_velocity, time}};
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

pub fn deg(value: f64) -> f64::Angle {
    f64::Angle::new::<angle::degree>(value)
}

pub fn deg_per_s(value: f64) -> f64::AngularVelocity {
    f64::AngularVelocity::new::<angular_velocity::degree_per_second>(value)
}

pub fn as_deg(angle: f64::Angle) -> f64 {
    angle.get::<angle::degree>()
}

pub fn as_deg_per_s(speed: f64::AngularVelocity) -> f64 {
    speed.get::<angular_velocity::degree_per_second>()
}

pub fn time(duration: std::time::Duration) -> f64::Time { f64::Time::new::<time::second>(duration.as_secs_f64()) }

pub struct Target {
    pub azimuth: f64::Angle,
    pub altitude: f64::Angle,
    pub az_spd: f64::AngularVelocity,
    pub alt_spd: f64::AngularVelocity
}

struct MountLastPos {
    t: std::time::Instant,
    axis1_pos: f64::Angle,
    axis2_pos: f64::Angle
}

pub struct MountSpeed {
    last_pos: Option<MountLastPos>,
    axes_spd: Option<(f64::AngularVelocity, f64::AngularVelocity)>,
}

// TODO: make it updatable only from main timer handler
impl MountSpeed {
    pub fn new() -> MountSpeed {
        MountSpeed{ last_pos: None, axes_spd: None }
    }

    pub fn notify_pos(&mut self, axis1_pos: f64::Angle, axis2_pos: f64::Angle) {
        if let Some(last_pos) = &self.last_pos {
            let dt = time(last_pos.t.elapsed());
            if dt.get::<time::second>() > 0.0 {
                self.axes_spd = Some((
                    Into::<f64::AngularVelocity>::into((axis1_pos - last_pos.axis1_pos) / dt),
                    Into::<f64::AngularVelocity>::into((axis2_pos - last_pos.axis2_pos) / dt)
                ));
            }
        }

        self.last_pos = Some(MountLastPos{ t: std::time::Instant::now(), axis1_pos, axis2_pos });
    }

    pub fn get(&self) -> Option<(f64::AngularVelocity, f64::AngularVelocity)> { self.axes_spd }
}

pub struct ProgramState {
    pub controllers: Vec<Pin<Box<dyn pasts::notify::Notify<Event = (u64, stick::Event)>>>>,
    pub cursive_stepper: CursiveRunnableStepper,
    pub data_receiver: Pin<Box<dyn pasts::notify::Notify<Event = Option<Result<String, std::io::Error>>>>>,
    pub listener: Pin<Box<dyn pasts::notify::Notify<Event = stick::Controller>>>,
    pub mount: Rc<RefCell<dyn Mount>>,
    pub mount_spd: Rc<RefCell<MountSpeed>>,
    pub slewing: Slewing,
    pub timers: Vec<Timer>,
    pub tracking: Tracking,
    pub tui: Option<TuiData>, // always `Some` after program start
    pub target: Rc<RefCell<Option<Target>>>
}

impl ProgramState {
    pub fn tui(&self) -> &TuiData { self.tui.as_ref().unwrap() }

    pub fn refresh_tui(&mut self) {
        self.cursive_stepper.curs.refresh();
    }
}

pub type TimerId = u64;

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

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut std::task::Context<'_>) -> Poll<Self::Event> {
        match Pin::new(&mut self.timer).poll_next(ctx) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(_) => { Poll::Ready(self.id) }
        }
    }
}
