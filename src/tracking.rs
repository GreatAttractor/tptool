use crate::{data, data::{as_deg, as_deg_per_s, deg, deg_per_s, time, MountSpeed}, mount::{Axis, Mount}};
use pasts::notify::Notify;
use pointing_utils::uom;
use std::{cell::RefCell, error::Error, pin::Pin, rc::Rc, task::{Context, Poll, Waker}};
use uom::si::f64;

// TODO: convert to const `angular_velocity::degree_per_second` once supported
const MATCH_POS_SPD_DEG_PER_S: f64 = 0.25;

const TIMER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(500);

pub type AngSpeed = f64::AngularVelocity;

pub struct TrackingController {
    state: Rc<RefCell<State>>,
}

impl TrackingController {
    pub fn start(&self) {
        log::info!("start tracking");
        self.state.borrow_mut().timer = Some(data::Timer::new(0, TIMER_INTERVAL));
    }

    pub fn stop(&self) {
        log::info!("stop tracking");
        self.state.borrow_mut().stop_tracking();
    }

    pub fn is_active(&self) -> bool {
        self.state.borrow().timer.is_some()
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

pub struct Tracking {
    max_spd: AngSpeed,
    mount: Rc<RefCell<dyn Mount>>,
    mount_spd: Rc<RefCell<MountSpeed>>, // TODO: make it unwriteable from here
    state: Rc<RefCell<State>>,
    target: Rc<RefCell<Option<data::Target>>>, // TODO: make it unwriteable from here
}

impl Tracking {
    pub fn new(
        max_spd: AngSpeed,
        mount: Rc<RefCell<dyn Mount>>,
        mount_spd: Rc<RefCell<MountSpeed>>,
        target: Rc<RefCell<Option<data::Target>>>,
    ) -> Tracking {
        Tracking{
            max_spd,
            mount,
            mount_spd,
            state: Rc::new(RefCell::new(State::new())),
            target,
        }
    }

    fn on_timer(&mut self) -> Result<(), Box<dyn Error>> {
        if self.mount_spd.borrow().get().is_none() {
            log::debug!("waiting for mount speed estimation");
            return Ok(());
        }

        let (mount_az, mount_alt) = self.mount.borrow_mut().position()?;
        let az_delta;
        let alt_delta;
        let target_az_spd;
        let target_alt_spd;
        {
            let t = self.target.borrow();
            let target = t.as_ref().ok_or::<Box<dyn Error>>("no target".into())?;
            az_delta = angle_diff(mount_az, target.azimuth);
            alt_delta = angle_diff(mount_alt, target.altitude);
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
        self.mount.borrow_mut().slew_axis(axis, spd)?;

        Ok(())
    }

    pub fn controller(&self) -> TrackingController {
        TrackingController{ state: Rc::clone(&self.state) }
    }
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