mod simulator;

use pointing_utils::uom;
use std::error::Error;
use uom::si::f64;

pub use simulator::Simulator;

#[derive(Copy, Clone, PartialEq)]
pub enum Axis {
    Primary,
    Secondary
}

impl std::fmt::Display for Axis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", match self {
            Axis::Primary => "primary",
            Axis::Secondary => "secondary",
        })
    }
}

pub trait Mount {
    fn get_info(&self) -> String;

    #[must_use]
    fn slew(&mut self, axis1: f64::AngularVelocity, axis2: f64::AngularVelocity) -> Result<(), Box<dyn Error>>;

    #[must_use]
    fn slew_axis(&mut self, axis: Axis, speed: f64::AngularVelocity) -> Result<(), Box<dyn Error>>;

    #[must_use]
    fn stop(&mut self) -> Result<(), Box<dyn Error>>;

    /// Returns position of primary and secondary axes.
    #[must_use]
    fn position(&mut self) -> Result<(f64::Angle, f64::Angle), Box<dyn Error>>;
}
