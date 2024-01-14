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

use cgmath::{Deg, EuclideanSpace, InnerSpace, Rad, Vector3};
use crate::{cursive_stepper::Running, data::ProgramState};
use pointing_utils::{cgmath, TargetInfoMessage, uom};
use std::task::Poll;
use uom::{si::f64, si::{length, velocity}};


// TODO: make configurable
const CONTROLLER_ID: u64 = 0x03006D041DC21440;

pub async fn event_loop(mut state: ProgramState) {

    pasts::Loop::new(&mut state)
        .when(|s| &mut s.listener, on_controller_connected)
        .poll(|s| &mut s.controllers, on_controller_event)
        .when(|s| &mut s.timer, on_timer)
        .when(|s| &mut s.data_receiver, on_data_received)
        // FIXME: why no controller and timer events when this is specified first? Too frequent polls/busy loop?
        // Should we use STDIN polling?
        .when(|s| &mut s.cursive_stepper, on_cursive_step)
        .await;
}

fn on_timer(state: &mut ProgramState, _: ()) -> Poll<()> {
    state.tui().text_content.tick.set_content(format!("tick: {}", state.counter));
    state.refresh_tui();
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

fn nop<S, C, T>(_: &mut S, _: C) -> Poll<T> {
    Poll::Pending
}

fn on_controller_connected(state: &mut ProgramState, controller: stick::Controller) -> Poll<()> {
    if controller.id() == CONTROLLER_ID {
        state.tui().text_content.controller_name.set_content(format!("[{:016X}] {}", controller.id(), controller.name()));
        state.refresh_tui();
    }
    state.controllers.push(controller);
    std::task::Poll::Pending
}

fn on_controller_event(state: &mut ProgramState, index: usize, event: stick::Event) -> std::task::Poll<()> {
    if let stick::Event::Disconnect = event {
        if state.controllers[index].id() == CONTROLLER_ID {
            state.tui().text_content.controller_name.set_content("(disconnected)");
            state.refresh_tui();
        }
        state.controllers.remove(index);
    } else {
        state.tui().text_content.controller_event.set_content(format!("{}", event));
        state.refresh_tui();
    }

    std::task::Poll::Pending
}

fn on_data_received(state: &mut ProgramState, message: Option<Result<String, std::io::Error>>) -> Poll<()> {
    //TODO: when received None, stop reception

    let ti = message.unwrap().unwrap().parse::<TargetInfoMessage>().unwrap();
    let len = ti.position.0.to_vec().magnitude();
    let dist = f64::Length::new::<length::meter>(len);
    let speed = f64::Velocity::new::<velocity::meter_per_second>(ti.velocity.0.magnitude());
    let atan2 = Deg::from(Rad(ti.position.0.y.atan2(ti.position.0.x)));
    let azimuth = if atan2 < Deg(0.0) && atan2 > Deg(-180.0) { -atan2 } else { Deg(360.0) - atan2 };
    let altitude = Deg::from(Rad((ti.position.0.z / len).asin()));

    let texts = &state.tui().text_content;
    texts.target_dist.set_content(format!("{:.1} km", dist.get::<length::kilometer>(),));
    texts.target_spd.set_content(format!("{:.0} km/h", speed.get::<velocity::kilometer_per_hour>()));
    texts.target_az.set_content(format!("{:.1}°", azimuth.0));
    texts.target_alt.set_content(format!("{:.1}°", altitude.0));

    state.refresh_tui();

    Poll::Pending
}
