use crate::{data, mount::Mount};
use pasts::notify::Notify;
use pointing_utils::uom;
use std::{cell::RefCell, pin::Pin, rc::Rc, task::{Context, Poll, Waker}};
use uom::si::{f64, angle};

#[derive(PartialEq)]
enum Phase {
    CatchUp,
    MatchSpeed,
    MatchPos,
    Idle,
}

pub struct TrackingController {
    state: Rc<RefCell<State>>,
}

impl TrackingController {
    pub fn start(&self) {
        let mut state = self.state.borrow_mut();
        state.timer = Some(data::Timer::new(0, std::time::Duration::from_secs(1)));
        state.wake();
    }

    pub fn stop(&self) {
        self.state.borrow_mut().phase = Phase::Idle;
        self.state.borrow_mut().timer = None;
    }

    pub fn is_active(&self) -> bool {
        self.state.borrow().phase != Phase::Idle
    }
}

struct State {
    phase: Phase,
    timer: Option<data::Timer>,
    waker: Option<Waker>
}

impl State {
    fn new() -> State {
        State{
            phase: Phase::Idle,
            timer: None,
            waker: None
        }
    }

    fn wake(&self) {
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

pub struct Tracking {
    max_az_spd: f64::AngularVelocity,
    max_alt_spd: f64::AngularVelocity,
    mount: Rc<RefCell<dyn Mount>>,
    state: Rc<RefCell<State>>,
    target: Rc<RefCell<Option<data::Target>>>,
}

impl Tracking {
    pub fn new(
        max_az_spd: f64::AngularVelocity,
        max_alt_spd: f64::AngularVelocity,
        mount: Rc<RefCell<dyn Mount>>,
        target: Rc<RefCell<Option<data::Target>>>
    ) -> Tracking {
        Tracking{
            max_az_spd,
            max_alt_spd,
            mount,
            state: Rc::new(RefCell::new(State::new())),
            target
        }
    }

    fn on_timer(&mut self) {
        log::info!(
            "tracking timer; target at {}, {}",
            self.target.borrow().as_ref().unwrap().azimuth.get::<angle::degree>(),
            self.target.borrow().as_ref().unwrap().altitude.get::<angle::degree>()
        );
        self.state.borrow_mut().phase = Phase::CatchUp;
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
            self.on_timer();
        }

        Poll::Pending
    }
}
