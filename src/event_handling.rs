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

use cgmath::{Deg, EuclideanSpace, InnerSpace, Point3, Rad, Vector3};
use crate::{
    cursive_stepper::Running,
    data,
    data::{as_deg, as_deg_per_s, ProgramState, TimerId, timers},
    mount::{Mount, MountWrapper},
    tracking,
    tracking::TrackingController,
    tui::TuiData,
    upgrade
};
use pointing_utils::{cgmath, TargetInfoMessage, uom};
use std::{cell::RefCell, future::Future, rc::{Rc, Weak}, task::Poll};
use uom::{si::f64, si::{angle, angular_velocity, length, velocity}};


// TODO: make configurable
const CONTROLLER_ID: u64 = 0x03006D041DC21440;

macro_rules! tui_s {
    ($state:ident) => { $state.tui().as_ref().unwrap() };
}

pub async fn event_loop(mut state: ProgramState) {

    pasts::Loop::new(&mut state)
        .on(|s| &mut s.cursive_stepper, on_cursive_step)
        .on(|s| &mut s.listener, on_controller_connected)
        .on(|s| &mut s.controllers[..], on_controller_event)
        .on(|s| &mut s.timers[..], on_timer)
        .on(|s| &mut s.data_receiver, on_data_received)
        .on(|s| &mut s.tracking, nop)
        .await;
}

fn on_main_timer(state: &mut ProgramState) {
    let pos = {
        let mut mount = state.mount.borrow_mut();
        if mount.is_none() { return; }
        mount.as_mut().unwrap().position()
    };
    if let Ok((axis1, axis2)) = pos {
        state.mount_spd.borrow_mut().notify_pos(axis1, axis2);
        let a1deg = as_deg(axis1);
        let azimuth = (if a1deg >= 0.0 && a1deg <= 180.0 { a1deg } else { 360.0 + a1deg }) % 360.0;

        let mut mount_az_str = format!("{:.2}°", azimuth);
        let mut mount_alt_str = format!("{:.2}°", as_deg(axis2));
        if let Some((az_spd, alt_spd)) = state.mount_spd.borrow().get() {
            mount_az_str += &format!("  {:.2}°/s", az_spd.get::<angular_velocity::degree_per_second>());
            mount_alt_str += &format!("  {:.2}°/s", alt_spd.get::<angular_velocity::degree_per_second>());
        }
        tui_s!(state).text_content.mount_az.set_content(mount_az_str);
        tui_s!(state).text_content.mount_alt.set_content(mount_alt_str);

        tui_s!(state).text_content.mount_total_az_travel.set_content(
            format!("{:.1}°", as_deg(state.mount.borrow().as_ref().unwrap().total_axis_travel().0))
        );
        tui_s!(state).text_content.mount_total_alt_travel.set_content(
            format!("{:.1}°", as_deg(state.mount.borrow().as_ref().unwrap().total_axis_travel().1))
        );

        state.refresh_tui();
    }
}

fn on_target_log(state: &mut ProgramState) {
    if let Some(target) = state.target.borrow().as_ref() {
        log::info!(
            "target-log;dist;{:.01};speed;{};altitude;{}",
            target.dist.get::<length::meter>(),
            target.speed.get::<velocity::meter_per_second>(),
            target.alt_above_gnd.get::<length::meter>()
        );
    }
}

fn on_timer(state: &mut ProgramState, idx_id: (usize, TimerId)) -> std::task::Poll<()> {
    let (_, id) = idx_id;
    match id {
        timers::MAIN => on_main_timer(state),
        timers::TARGET_LOG => on_target_log(state),
        _ => ()
    }

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

fn on_controller_connected(state: &mut ProgramState, mut controller: stick::Controller) -> Poll<()> {
    if controller.id() == CONTROLLER_ID {
        let ctrl_str = format!("[{:016X}] {}", controller.id(), controller.name());
        log::info!("new controller: {}", ctrl_str);
        state.tui().as_ref().unwrap().text_content.controller_name.set_content(ctrl_str);
        state.refresh_tui();
    }
    state.controllers.push(
        Box::pin(pasts::notify::poll_fn(move |ctx| {
            match std::pin::Pin::new(&mut controller).poll(ctx) {
                Poll::Ready(event) => Poll::Ready((controller.id(), event)),
                Poll::Pending => Poll::Pending
            }
        })),
    );


    std::task::Poll::Pending
}

pub fn on_stop_mount(mount: &Rc<RefCell<Option<MountWrapper>>>, tracking: &TrackingController) {
    let mut mount = mount.borrow_mut();
    if let Some(mount) = mount.as_mut() {
        if let Err(e) = mount.stop() {
            log::error!("error stopping the mount: {}", e);
        }
        tracking.stop();
    }
}

pub fn on_toggle_tracking(tracking: &TrackingController) {
    if tracking.is_active() {
        tracking.stop();
    } else {
        tracking.start();
    }
}

fn on_controller_event(state: &mut ProgramState, idx_val: (usize, (u64, stick::Event))) -> std::task::Poll<()> {
    let (index, (id, event)) = idx_val;

    state.tui().as_ref().unwrap().text_content.controller_event.set_content(format!("{}", event)); //TESTING #########
    state.refresh_tui();

    if state.mount.borrow().is_none() { return Poll::Pending; }

    if let stick::Event::Disconnect = event {
        if id == CONTROLLER_ID {
            state.tui().as_ref().unwrap().text_content.controller_name.set_content("(disconnected)");
            state.refresh_tui();
        }
        state.controllers.remove(index);
    } else {
        let mut slew_change = false;

        // TODO: make all actions configurable
        match event {
            stick::Event::JoyX(value) => {
                state.slewing.axis1_rel = value;
                slew_change = true;
            },
            stick::Event::JoyY(value) => {
                state.slewing.axis2_rel = -value; // TODO: make axis reversals configurable
                slew_change = true;
            },
            stick::Event::PovLeft(pressed) => {
                state.slewing.axis1_rel = if pressed { -1.0 } else { 0.0 };
                slew_change = true;
            },
            stick::Event::PovRight(pressed) => {
                state.slewing.axis1_rel = if pressed { 1.0 } else { 0.0 };
                slew_change = true;
            },
            stick::Event::PovDown(pressed) => {
                state.slewing.axis2_rel = if pressed { -1.0 } else { 0.0 };
                slew_change = true;
            },
            stick::Event::PovUp(pressed) => {
                state.slewing.axis2_rel = if pressed { 1.0 } else { 0.0 };
                slew_change = true;
            },
            stick::Event::BumperL(pressed) => {
                if pressed { state.tracking.cancel_adjustment(); }
            },
            stick::Event::BumperR(pressed) => {
                if pressed { state.tracking.save_adjustment(); }
            },
            stick::Event::ActionV(pressed) => if pressed { on_stop_mount(&state.mount, &state.tracking.controller()); },
            stick::Event::ActionB(pressed) => if pressed {
                on_toggle_tracking(&state.tracking.controller());
            },
            _ => ()
        }

        if slew_change {
            if state.tracking.is_active() {
                state.tracking.adjust_slew(state.slewing.axis1_rel, state.slewing.axis2_rel);
            } else {
                let spd = *state.slew_speed.borrow();
                if let Err(e) = state.mount.borrow_mut().as_mut().unwrap().slew(
                    spd * state.slewing.axis1_rel,
                    spd * state.slewing.axis2_rel
                ) {
                    log::error!("error when slewing: {}", e);
                }
            }
        }
    }

    std::task::Poll::Pending
}

fn on_data_received(state: &mut ProgramState, message: Result<String, std::io::Error>) -> Poll<()> {
    let radians = |value| f64::AngularVelocity::new::<angular_velocity::radian_per_second>(value);

    let ti = message.unwrap().parse::<TargetInfoMessage>().unwrap();
    let r = ti.position.0.to_vec();
    let r_len2 = r.magnitude2();
    let r_len = r_len2.sqrt();
    let dist = f64::Length::new::<length::meter>(r_len);
    let speed = f64::Velocity::new::<velocity::meter_per_second>(ti.velocity.0.magnitude());
    let (azimuth, altitude) = data::to_spherical(ti.position.0);
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

    *state.target.borrow_mut() = Some(data::Target{
        dist,
        azimuth,
        alt_above_gnd: ti.altitude,
        altitude,
        az_spd: ang_speed_az,
        alt_spd: ang_speed_el,
        speed: f64::Velocity::new::<velocity::meter_per_second>(ti.velocity.0.magnitude()),
        v_tangential
    });

    {
        let tui = &state.tui();
        let tui = tui.as_ref().unwrap();
        let texts = &tui.text_content;

        texts.target_dist.set_content(format!("{:.1} km", dist.get::<length::kilometer>(),));
        texts.target_spd.set_content(format!(
            "{:.0} km/h  {:.02}°/s",
            speed.get::<velocity::kilometer_per_hour>(),
            ang_speed.get::<angular_velocity::degree_per_second>()
        ));
        texts.target_az.set_content(
            format!("{:.1}°  {:.02}°/s", as_deg(azimuth), as_deg_per_s(ang_speed_az))
        );
        texts.target_alt.set_content(
            format!("{:.1}°  {:.02}°/s", as_deg(altitude), as_deg_per_s(ang_speed_el))
        );
    }

    state.refresh_tui();

    Poll::Pending
}

pub fn on_max_travel_exceeded(
    mount: &mut MountWrapper,
    axis1: bool,
    axis2: bool,
    tracking: TrackingController
) {
    if axis1 {
        log::warn!("max travel in azimuth exceeded");
    }
    if axis2 {
        log::warn!("max travel in altitude exceeded");
    }

    if axis1 || axis2 {
        tracking.stop();
        if let Err(e) = mount.stop() { log::error!("error stopping mount: {}", e); }
    }
}

pub fn on_tracking_state_changed(running: tracking::Running, tui: Weak<RefCell<Option<TuiData>>>) {
    upgrade!(tui);
    tui.borrow().as_ref().unwrap().text_content.tracking_state.set_content(
        if running.0 { "enabled" } else { "disabled"}
    );
}
