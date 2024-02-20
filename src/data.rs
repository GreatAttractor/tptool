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
use cgmath::{Basis3, Deg, EuclideanSpace, InnerSpace, Point3, Rad, Rotation, Rotation3, Vector3};
use crate::{
    config::Configuration,
    cursive_stepper::CursiveRunnableStepper,
    data_receiver::DataReceiver,
    mount,
    tracking::Tracking,
    tui::TuiData
};
use pointing_utils::{cgmath, GeoPos, to_global_unit, uom};
use std::{cell::{Ref, RefCell}, future::Future, marker::Unpin, pin::Pin, rc::Rc, task::{Context, Poll}};
use uom::{si::f64, si::{angle, angular_velocity, length, time}};
use pasts::notify::Notify;

pub mod timers {
    use super::TimerId;

    pub const MAIN: TimerId = 1;
    pub const TARGET_LOG: TimerId = 2;
}

pub struct RefPositionPreset {
    pub name: String,
    pub azimuth: f64::Angle,
    pub altitude: f64::Angle
}

impl std::fmt::Display for RefPositionPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{};{};{}", as_deg(self.azimuth), as_deg(self.altitude), self.name)
    }
}

impl std::str::FromStr for RefPositionPreset {
    type Err = std::num::ParseFloatError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(3, ';').collect();
        Ok(RefPositionPreset{
            azimuth: deg(parts[0].parse::<f64>()?),
            altitude: deg(parts[1].parse::<f64>()?),
            name: parts[2].into()
        })
    }
}

pub struct Slewing {
    // values from [-1.0, 1.0]
    pub axis1_rel: f64,
    pub axis2_rel: f64,
}

impl Default for Slewing {
    fn default() -> Slewing {
        Slewing{ axis1_rel: 0.0, axis2_rel: 0.0 }
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
    pub dist: f64::Length,
    pub speed: f64::Velocity,
    pub alt_above_gnd: f64::Length,
    pub azimuth: f64::Angle,
    pub altitude: f64::Angle,
    pub az_spd: f64::AngularVelocity,
    pub alt_spd: f64::AngularVelocity,
    pub v_tangential: Vector3<f64> // m/s
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
    pub config: Rc<RefCell<Configuration>>,
    pub controllers: Vec<Pin<Box<dyn pasts::notify::Notify<Event = (u64, stick::Event)>>>>,
    pub cursive_stepper: CursiveRunnableStepper,
    pub data_receiver: DataReceiver,
    pub listener: Pin<Box<dyn pasts::notify::Notify<Event = stick::Controller>>>,
    pub mount: Rc<RefCell<Option<mount::MountWrapper>>>,
    pub mount_spd: Rc<RefCell<MountSpeed>>,
    pub slewing: Slewing,
    pub slew_speed: Rc<RefCell<f64::AngularVelocity>>,
    pub timers: Vec<Timer>,
    pub tracking: Tracking,
    pub tui: Rc<RefCell<Option<TuiData>>>, // always `Some` after program start
    pub target: Rc<RefCell<Option<Target>>>,
}

impl ProgramState {
    pub fn tui(&self) -> Ref<Option<TuiData>> { self.tui.borrow() }

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

pub fn to_spherical(pos: Point3<f64>) -> (f64::Angle, f64::Angle) {
    let atan2 = Deg::from(Rad(pos.y.atan2(pos.x)));
    let azimuth = if atan2 < Deg(0.0) && atan2 > Deg(-180.0) { -atan2 } else { Deg(360.0) - atan2 };
    let altitude = Deg::from(Rad((pos.z / pos.to_vec().magnitude()).asin()));

    (deg(azimuth.0), deg(altitude.0))
}

pub fn spherical_to_unit(azimuth: f64::Angle, altitude: f64::Angle) -> Point3<f64> {
    const NORTH: Vector3<f64> = Vector3{ x: 1.0, y: 0.0, z: 0.0 };
    const UP: Vector3<f64> = Vector3{ x: 0.0, y: 0.0, z: 1.0 };
    const WEST: Vector3<f64> = Vector3{ x: 0.0, y: 1.0, z: 0.0 };

    let dir = Basis3::from_axis_angle(WEST, -Rad(altitude.get::<angle::radian>())).rotate_vector(NORTH);
    let dir = Basis3::from_axis_angle(UP, -Rad(azimuth.get::<angle::radian>())).rotate_vector(dir);

    Point3::from_vec(dir)
}

fn unit_tangent_to_great_circle_between_points_on_unit_sphere(p1: Point3<f64>, p2: Point3<f64>) -> Vector3<f64> {
    let a_unit = p1.to_vec().cross(p2.to_vec()).normalize();
    a_unit.cross(p1.to_vec())
}

pub fn calc_az_alt_between_points(p1: &GeoPos, p2: &GeoPos) -> (f64::Angle, f64::Angle) {
    // -------- azimuth --------
    const NORTH_POLE: Point3<f64> = Point3{ x: 0.0, y: 0.0, z: 1.0 };
    let p1_unit_0_alt = to_global_unit(&p1.lat_lon).0;
    let p2_unit_0_alt = to_global_unit(&p2.lat_lon).0;
    let to_north_pole = unit_tangent_to_great_circle_between_points_on_unit_sphere(p1_unit_0_alt, NORTH_POLE);
    let to_p2_0_alt = unit_tangent_to_great_circle_between_points_on_unit_sphere(p1_unit_0_alt, p2_unit_0_alt);

    let cross_p = to_north_pole.cross(to_p2_0_alt);
    let mut azimuth = Rad(to_north_pole.dot(to_p2_0_alt).acos());

    if cross_p.dot(p1_unit_0_alt.to_vec()) > 0.0 { azimuth = -azimuth; }

    // -------- altitude --------
    let ang_dist_cos = p1_unit_0_alt.to_vec().dot(p2_unit_0_alt.to_vec());
    let ang_dist_sin = (1.0 - ang_dist_cos.powi(2)).sqrt();
    let R = f64::Length::new::<length::meter>(pointing_utils::EARTH_RADIUS_M);
    let altitude = Rad(((ang_dist_cos - Into::<f64>::into((R + p1.elevation) / (R + p2.elevation))) / ang_dist_sin).atan());

    (deg(Deg::from(azimuth).0), deg(Deg::from(altitude).0))
}

pub fn angle_diff(a1: f64::Angle, a2: f64::Angle) -> f64::Angle {
    let mut a1 = a1 % deg(360.0);
    let mut a2 = a2 % deg(360.0);

    if a1.signum() != a2.signum() {
        if a1.is_sign_negative() { a1 = deg(360.0) + a1; } else { a2 = deg(360.0) + a2; }
    }

    if a2 - a1 > deg(180.0) {
        a2 - a1 - deg(360.0)
    } else if a2 - a1 < deg(-180.0) {
        a2 - a1 + deg(360.0)
    } else {
        a2 - a1
    }
}

mod tests {
    use super::*;
    use super::uom::si::angle;

    macro_rules! assert_almost_eq {
        ($expected:expr, $actual:expr) => {
            if ($expected - $actual).abs() > deg(1.0e-10) {
                panic!("expected: {:.1}, but was: {:.1}", $expected.get::<angle::degree>(), $actual.get::<angle::degree>());
            }
        };
    }

    #[test]
    fn azimuth_difference_calculation() {
        assert_almost_eq!(deg(20.0), angle_diff(deg(10.0), deg(30.0)));
        assert_almost_eq!(deg(-20.0), angle_diff(deg(10.0), deg(350.0)));
        assert_almost_eq!(deg(20.0), angle_diff(deg(350.0), deg(10.0)));
        assert_almost_eq!(deg(-10.0), angle_diff(deg(350.0), deg(340.0)));
        assert_almost_eq!(deg(-10.0), angle_diff(deg(-10.0), deg(340.0)));
        assert_almost_eq!(deg(10.0), angle_diff(deg(10.0), deg(-340.0)));
    }
}
