mod ioptron;
mod simulator;

use crate::data;
use pointing_utils::uom;
use std::error::Error;
use uom::si::f64;

pub use ioptron::Ioptron;
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

pub struct MountWrapper {
    wrapped: Box<dyn Mount>,
    axis1_ofs: f64::Angle,
    axis2_ofs: f64::Angle,
    /// User-specified zero position (in terms of mount's internal axes' positions).
    zero_pos: Option<(f64::Angle, f64::Angle)>
}

impl MountWrapper {
    pub fn new(wrapped: Box<dyn Mount>) -> MountWrapper {
        MountWrapper{ wrapped, axis1_ofs: data::deg(0.0), axis2_ofs: data::deg(0.0), zero_pos: None }
    }

    pub fn set_reference_position(&mut self, axis1: f64::Angle, axis2: f64::Angle) -> Result<(), Box<dyn Error>> {
        let (internal1, internal2) = self.wrapped.position()?;
        self.axis1_ofs = axis1 - internal1;
        self.axis2_ofs = axis2 - internal2;
        Ok(())
    }

    pub fn has_zero_position(&self) -> bool { self.zero_pos.is_some() }

    pub fn set_zero_position(&mut self) -> Result<(), Box<dyn Error>> {
        let (internal1, internal2) = self.wrapped.position()?;
        self.zero_pos = Some((internal1, internal2));
        Ok(())
    }
}

impl Mount for MountWrapper {
    fn get_info(&self) -> String {
        self.wrapped.get_info()
    }

    fn slew(&mut self, axis1: f64::AngularVelocity, axis2: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        self.wrapped.slew(axis1, axis2)
    }

    fn slew_axis(&mut self, axis: Axis, speed: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        self.wrapped.slew_axis(axis, speed)
    }

    fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        self.wrapped.stop()
    }

    fn position(&mut self) -> Result<(f64::Angle, f64::Angle), Box<dyn Error>> {
        let (internal1, internal2) = self.wrapped.position()?;
        Ok((self.axis1_ofs + internal1, self.axis2_ofs + internal2))
    }
}
