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
use uom::{si::f64, si::{angular_velocity, length, velocity}};


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
    let radians = |value| f64::AngularVelocity::new::<angular_velocity::radian_per_second>(value);

    let ti = message.unwrap().unwrap().parse::<TargetInfoMessage>().unwrap();
    let r = ti.position.0.to_vec();
    let r_len2 = r.magnitude2();
    let r_len = r_len2.sqrt();
    let dist = f64::Length::new::<length::meter>(r_len);
    let speed = f64::Velocity::new::<velocity::meter_per_second>(ti.velocity.0.magnitude());
    let atan2 = Deg::from(Rad(ti.position.0.y.atan2(ti.position.0.x)));
    let azimuth = if atan2 < Deg(0.0) && atan2 > Deg(-180.0) { -atan2 } else { Deg(360.0) - atan2 };
    let altitude = Deg::from(Rad((ti.position.0.z / r_len).asin()));
    let v_radial = r * ti.velocity.0.dot(r) / r_len2;
    let v_tangential = ti.velocity.0 - v_radial;
    let ang_speed = radians(v_tangential.magnitude() / r_len);
    const ZENITH: Vector3<f64> = Vector3{ x: 0.0, y: 0.0, z: 1.0 };
    let pos_az = r.cross(ZENITH);
    let to_zenith = pos_az.cross(r);
    let v_up_down = to_zenith * v_tangential.dot(to_zenith) / to_zenith.magnitude2();
    let v_left_right = v_tangential - v_up_down;
    let ang_speed_az_sign = -r.cross(v_tangential).z.signum();
    let ang_speed_az = ang_speed_az_sign * radians(v_left_right.magnitude() / (r.x.powi(2) + r.y.powi(2)).sqrt());
    let ang_speed_el = v_up_down.z.signum() * radians(v_up_down.magnitude() / r_len);

    let texts = &state.tui().text_content;
    texts.target_dist.set_content(format!("{:.1} km", dist.get::<length::kilometer>(),));
    texts.target_spd.set_content(format!(
        "{:.0} km/h  {:.02}°/s",
        speed.get::<velocity::kilometer_per_hour>(),
        ang_speed.get::<angular_velocity::degree_per_second>()
    ));
    texts.target_az.set_content(
        format!("{:.1}°  {:.02}°/s", azimuth.0, ang_speed_az.get::<angular_velocity::degree_per_second>())
    );
    texts.target_alt.set_content(
        format!("{:.1}°  {:.02}°/s", altitude.0, ang_speed_el.get::<angular_velocity::degree_per_second>())
    );

    state.refresh_tui();

    Poll::Pending
}
