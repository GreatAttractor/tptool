use cgmath::{Basis3, Deg, EuclideanSpace, InnerSpace, Point3, Rad, Rotation, Rotation3, Vector3};
use crate::{data, data::{as_deg, as_deg_per_s, deg, deg_per_s, time, MountSpeed}, mount, mount::{Axis, Mount}};
use pasts::notify::Notify;
use pointing_utils::{cgmath, uom};
use std::{cell::RefCell, error::Error, pin::Pin, rc::{Rc, Weak}, task::{Context, Poll, Waker}};
use uom::si::{angle, f64};

// TODO: convert to const `angular_velocity::degree_per_second` once supported
const MATCH_POS_SPD_DEG_PER_S: f64 = 0.25;
const ADJUSTMENT_SPD_DEG_PER_S: f64 = 0.5;

const TIMER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

pub type AngSpeed = f64::AngularVelocity;

#[derive(Clone)]
pub struct TrackingController {
    state: Weak<RefCell<State>>,
}

impl TrackingController {
    pub fn start(&self) {
        log::info!("start tracking");
        self.state.upgrade().unwrap().borrow_mut().timer = Some(data::Timer::new(0, TIMER_INTERVAL));
    }

    pub fn stop(&self) {
        log::info!("stop tracking");
        self.state.upgrade().unwrap().borrow_mut().stop_tracking();
    }

    pub fn is_active(&self) -> bool {
        self.state.upgrade().unwrap().borrow().timer.is_some()
    }
}

struct State {
    timer: Option<data::Timer>,
    waker: Option<Waker>,
}

impl State {
    fn new() -> State {
        State{
            timer: None,
            waker: None
        }
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }

    fn stop_tracking(&mut self) {
        self.timer = None;
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
    adjusting: bool,
    adjustment: Option<Adjustment>
}

impl Tracking {
    pub fn new(
        max_spd: AngSpeed,
        mount: Rc<RefCell<Option<mount::MountWrapper>>>,
        mount_spd: Rc<RefCell<MountSpeed>>,
        target: Rc<RefCell<Option<data::Target>>>,
    ) -> Tracking {
        Tracking{
            max_spd,
            mount,
            mount_spd,
            state: Rc::new(RefCell::new(State::new())),
            target,
            adjusting: false,
            adjustment: None
        }
    }

    fn on_timer(&mut self) -> Result<(), Box<dyn Error>> {
        if self.mount.borrow().is_none() {
            return Err("mount not connected".into());
        }

        if self.adjusting { return Ok(()); }

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
        let az_delta;
        let alt_delta;
        let target_az_spd;
        let target_alt_spd;
        {
            let t = self.target.borrow();
            let target = t.as_ref().ok_or::<Box<dyn Error>>("no target".into())?;

            let (target_az, target_alt) = if let Some(adj) = self.adjustment.as_ref() {
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
        if !self.adjusting {
            self.adjusting = true;
            log::info!("begin manual adjustment");
        }

        let t = self.target.borrow();
        if let Some(target) = t.as_ref() {
            let new_axis1_spd = target.az_spd + axis1_rel_spd * deg_per_s(ADJUSTMENT_SPD_DEG_PER_S);
            let new_axis2_spd = target.alt_spd + axis2_rel_spd * deg_per_s(ADJUSTMENT_SPD_DEG_PER_S);

            if let Err(e) = self.mount.borrow_mut().as_mut().unwrap().slew(new_axis1_spd, new_axis2_spd) {
                log::error!("error when slewing: {}", e);
            }
        } else {
            log::error!("no target");
            self.adjustment = None;
            self.state.borrow_mut().timer = None;
        }
    }

    pub fn save_adjustment(&mut self) {
        if !self.adjusting { return; }

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

        self.adjustment = Some(adjustment);

        self.adjusting = false;
    }

    pub fn cancel_adjustment(&mut self) {
        self.adjusting = false;
        self.adjustment = None;
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

fn angle_diff(a1: f64::Angle, a2: f64::Angle) -> f64::Angle {
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