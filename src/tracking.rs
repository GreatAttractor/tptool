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

use cgmath::{Basis3, Deg, EuclideanSpace, InnerSpace, Point3, Rad, Rotation, Rotation3, Vector3};
use crate::{
    data,
    data::{angle_diff, as_deg, as_deg_per_s, deg, deg_per_s, time, MountSpeed},
    mount,
    mount::{Axis, Mount}
};
use pasts::notify::Notify;
use pointing_utils::{cgmath, uom};
use std::{cell::RefCell, error::Error, pin::Pin, rc::{Rc, Weak}, task::{Context, Poll, Waker}};
use uom::si::{angle, f64};

// TODO: convert to const `angular_velocity::degree_per_second` once supported
const MATCH_POS_SPD_DEG_PER_S: f64 = 0.25;
const MAX_ADJUSTMENT_SPD_DEG_PER_S: f64 = 0.5;

const TIMER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

pub type AngSpeed = f64::AngularVelocity;

#[derive(Clone)]
pub struct TrackingController {
    state: Weak<RefCell<State>>,
}

impl TrackingController {
    pub fn start(&self) {
        log::info!("start tracking");
        self.state.upgrade().unwrap().borrow_mut().start_tracking();
    }

    pub fn stop(&self) {
        log::info!("stop tracking");
        self.state.upgrade().unwrap().borrow_mut().stop_tracking();
    }

    pub fn is_active(&self) -> bool {
        self.state.upgrade().unwrap().borrow().timer.is_some()
    }

    pub fn change_adjustment_slew_speed(&self, factor: f64) {
        let state = self.state.upgrade().unwrap();
        let mut state = state.borrow_mut();
        state.adjustment_slew_speed = (state.adjustment_slew_speed * factor)
            .max(deg_per_s(0.025))
            .min(deg_per_s(MAX_ADJUSTMENT_SPD_DEG_PER_S));
    }
}

pub struct Running(pub bool);

/// Params: mount wrapper, axis1 travel exceeded, axis2 travel exceeded.
pub type OnTrackingStateChanged = dyn Fn(Running) + 'static;

struct State {
    timer: Option<data::Timer>,
    waker: Option<Waker>,
    callback: Box<OnTrackingStateChanged>,
    adjusting: bool,
    adjustment: Option<Adjustment>,
    adjustment_slew_speed: AngSpeed
}

impl State {
    fn new(callback: Box<OnTrackingStateChanged>) -> State {
        State{
            timer: None,
            waker: None,
            callback,
            adjusting: false,
            adjustment: None,
            adjustment_slew_speed: deg_per_s(MAX_ADJUSTMENT_SPD_DEG_PER_S)
        }
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }

    fn start_tracking(&mut self) {
        self.timer = Some(data::Timer::new(0, TIMER_INTERVAL));
        (*self.callback)(Running(true));
    }

    fn stop_tracking(&mut self) {
        self.timer = None;
        self.adjusting = false;
        self.adjustment = None;
        (*self.callback)(Running(false));
    }
}

struct Adjustment {
    /// Angle of rotation of tangent velocity around target position vector.
    rel_dir: f64::Angle,
    /// Angular displacement on the sky along the direction given by `rel_dir`.
    angle: f64::Angle
}

pub struct Tracking {
    max_spd: AngSpeed,
    mount: Rc<RefCell<Option<mount::MountWrapper>>>,
    mount_spd: Rc<RefCell<MountSpeed>>, // TODO: make it unwriteable from here
    state: Rc<RefCell<State>>,
    target: Rc<RefCell<Option<data::Target>>>, // TODO: make it unwriteable from here
}

impl Tracking {
    pub fn new(
        max_spd: AngSpeed,
        mount: Rc<RefCell<Option<mount::MountWrapper>>>,
        mount_spd: Rc<RefCell<MountSpeed>>,
        target: Rc<RefCell<Option<data::Target>>>,
        callback: Box<OnTrackingStateChanged>
    ) -> Tracking {
        Tracking{
            max_spd,
            mount,
            mount_spd,
            state: Rc::new(RefCell::new(State::new(callback))),
            target
        }
    }

    fn on_timer(&mut self) -> Result<(), Box<dyn Error>> {
        if self.mount.borrow().is_none() {
            return Err("mount not connected".into());
        }

        if self.state.borrow().adjusting { return Ok(()); }

        if self.mount_spd.borrow().get().is_none() {
            log::debug!("waiting for mount speed estimation");
            return Ok(());
        }

        let (mount_az, mount_alt) = match self.mount.borrow_mut().as_mut().unwrap().position() {
            Ok(p) => p,
            Err(e) => {
                log::warn!("failed to get mount position: {}", e);
                return Ok(());
            }
        };
        // calling `MountWrapper::position` might have triggered the max travel exceeded callback and disabled tracking
        if self.state.borrow().timer.is_none() { return Ok(()); }

        let az_delta;
        let alt_delta;
        let target_az_spd;
        let target_alt_spd;
        {
            let t = self.target.borrow();
            let target = t.as_ref().ok_or::<Box<dyn Error>>("no target".into())?;

            let (target_az, target_alt) = if let Some(adj) = self.state.borrow().adjustment.as_ref() {
                get_adjusted_pos(target.azimuth, target.altitude, target.v_tangential, adj)
            } else {
                (target.azimuth, target.altitude)
            };

            az_delta = angle_diff(mount_az, target_az);
            alt_delta = angle_diff(mount_alt, target_alt);
            target_az_spd = target.az_spd;
            target_alt_spd = target.alt_spd;
        }

        log::debug!("az. delta = {:.1}°, alt. delta = {:.1}°", as_deg(az_delta), as_deg(alt_delta));

        self.update_axis(Axis::Primary, az_delta, target_az_spd)?;
        self.update_axis(Axis::Secondary, alt_delta, target_alt_spd)?;

        Ok(())
    }

    fn update_axis(
        &mut self,
        axis: Axis,
        pos_delta: f64::Angle,
        target_spd: f64::AngularVelocity,
    ) -> Result<(), Box<dyn Error>> {
        let mut spd = target_spd + deg_per_s(as_deg(pos_delta) * MATCH_POS_SPD_DEG_PER_S);
        if spd < -self.max_spd { spd = -self.max_spd; } else if spd > self.max_spd { spd = self.max_spd; }
        self.mount.borrow_mut().as_mut().unwrap().slew_axis(axis, spd)?;

        Ok(())
    }

    pub fn controller(&self) -> TrackingController {
        TrackingController{ state: Rc::downgrade(&self.state) }
    }

    pub fn is_active(&self) -> bool {
        self.state.borrow().timer.is_some()
    }

    /// Parameters are between [-1.0; 1.0].
    pub fn adjust_slew(&mut self, axis1_rel_spd: f64, axis2_rel_spd: f64) {
        if !self.state.borrow().adjusting {
            self.state.borrow_mut().adjusting = true;
            log::info!("begin manual adjustment");
        }

        let adj_speed = self.state.borrow().adjustment_slew_speed;

        let t = self.target.borrow();
        if let Some(target) = t.as_ref() {
            let new_axis1_spd = target.az_spd + axis1_rel_spd * adj_speed;
            let new_axis2_spd = target.alt_spd + axis2_rel_spd * adj_speed;
            if let Err(e) = self.mount.borrow_mut().as_mut().unwrap().slew(new_axis1_spd, new_axis2_spd) {
                log::error!("error when slewing: {}", e);
            }
        } else {
            log::error!("no target");
            self.state.borrow_mut().stop_tracking();
        }
    }

    pub fn save_adjustment(&mut self) {
        if !self.state.borrow().adjusting { return; }

        let target = self.target.borrow();
        if target.is_none() {
            log::error!("no target");
            return;
        }
        let target = target.as_ref().unwrap();
        let target_pos = data::spherical_to_unit(target.azimuth, target.altitude);
        let (mount_az, mount_alt) = match self.mount.borrow_mut().as_mut().unwrap().position() {
            Ok(pos) => pos,
            Err(e) => {
                log::warn!("failed to get mount position: {}", e);
                return;
            }
        };
        let adjusted_pos = data::spherical_to_unit(mount_az, mount_alt);

        // To be precise, before calculating the offset and its angle to `v_tangential` we should project
        // the `adjusted_pos` vector onto the plane tangent at `target_pos`; but since the angles involved are small,
        // there will not be much difference.
        let offset = adjusted_pos - target_pos;

        let is_obtuse = target.v_tangential.dot(offset) < 0.0;

        let rotation = Rad(
            (target.v_tangential.cross(offset).magnitude() / (offset.magnitude() * target.v_tangential.magnitude())
        ).asin()); // rotation angle of `offset` relative to `v_tangential`

        let rotation = if is_obtuse { Rad::from(Deg(180.0)) - rotation } else { rotation };
        let is_rotation_cw = target.v_tangential.cross(offset).dot(target_pos.to_vec()) < 0.0;
        let rotation = if is_rotation_cw { -rotation } else { rotation };

        // approximate, since we use chord instead of arc length
        let angular_offset = f64::Angle::new::<angle::radian>(offset.magnitude());

        let adjustment = Adjustment{
            rel_dir: deg(Deg::from(rotation).0),
            angle: angular_offset
        };
        log::info!(
            "using new adjustment: rel_dir = {:.01}°, angle = {:.02}°",
            as_deg(adjustment.rel_dir),
            as_deg(adjustment.angle)
        );

        let mut state = self.state.borrow_mut();
        state.adjustment = Some(adjustment);
        state.adjusting = false;
    }

    pub fn cancel_adjustment(&mut self) {
        let mut state = self.state.borrow_mut();
        state.adjusting = false;
        state.adjustment = None;
        log::info!("cancel manual adjustment");
    }
}

fn get_adjusted_pos(
    azimuth: f64::Angle,
    altitude: f64::Angle,
    v_tangential: Vector3<f64>,
    adj: &Adjustment
) -> (f64::Angle, f64::Angle) {
    let r = data::spherical_to_unit(azimuth, altitude).to_vec();
    let vt_unit = v_tangential.normalize();
    let adjustment_dir = Basis3::from_axis_angle(r, Deg(as_deg(adj.rel_dir))).rotate_vector(vt_unit);
    let adjusted_pos = Point3::from_vec(r) + adjustment_dir * adj.angle.get::<angle::radian>();

    let result = data::to_spherical(adjusted_pos);
    log::debug!("adjusted position: az. {:.1}°, alt. {:.1}°", as_deg(result.0), as_deg(result.1));

    result
}

impl Notify for Tracking {
    type Event = ();

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<()> {
        if self.state.borrow().waker.is_none() {
            self.state.borrow_mut().waker = Some(ctx.waker().clone());
        }

        let ticked = if let Some(timer) = self.state.borrow_mut().timer.as_mut() {
            Pin::new(timer).poll_next(ctx).is_ready()
        } else {
            false
        };

        if ticked {
            if let Err(e) = self.on_timer() {
                log::error!("error while tracking: {}", e);
                self.state.borrow_mut().stop_tracking();
            }
        }

        Poll::Pending
    }
}
