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

mod ioptron;
mod simulator;

use crate::data;
use pointing_utils::uom;
use std::{error::Error, rc::Rc};
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

/// Params: mount wrapper, axis1 travel exceeded, axis2 travel exceeded.
type AxisTravelExceeded = dyn Fn(&mut MountWrapper, bool, bool) + 'static;

pub struct MountWrapper {
    wrapped: Box<dyn Mount>,
    axis1_ofs: f64::Angle,
    axis2_ofs: f64::Angle,
    /// User-specified zero position (in terms of mount's internal axes' positions).
    zero_pos: Option<(f64::Angle, f64::Angle)>,
    total_axis_travel: (f64::Angle, f64::Angle),
    last_pos: Option<(f64::Angle, f64::Angle)>,
    max_travel_exceeded_callback: Option<Rc<AxisTravelExceeded>>,
}

impl MountWrapper {
    pub fn new(wrapped: Box<dyn Mount>) -> MountWrapper {
        MountWrapper{
            wrapped,
            axis1_ofs: data::deg(0.0),
            axis2_ofs: data::deg(0.0),
            zero_pos: None,
            total_axis_travel: (data::deg(0.0), data::deg(0.0)),
            last_pos: None,
            max_travel_exceeded_callback: None,
        }
    }

    /// Triggers only once each time the max travel is exceeded.
    pub fn set_on_max_travel_exceeded(&mut self, callback: Box<AxisTravelExceeded>) {
        self.max_travel_exceeded_callback = Some(Rc::new(callback));
    }

    pub fn set_reference_position(&mut self, axis1: f64::Angle, axis2: f64::Angle) -> Result<(), Box<dyn Error>> {
        let (internal1, internal2) = self.wrapped.position()?;
        self.axis1_ofs = axis1 - internal1;
        self.axis2_ofs = axis2 - internal2;
        Ok(())
    }

    pub fn zero_position(&self) -> &Option<(f64::Angle, f64::Angle)> { &self.zero_pos }

    pub fn set_zero_position(&mut self) -> Result<(), Box<dyn Error>> {
        let t0 = std::time::Instant::now();
        while t0.elapsed() < std::time::Duration::from_secs(1) {
            if let Ok((internal1, internal2)) = self.wrapped.position() {
                self.zero_pos = Some((internal1, internal2));
                self.total_axis_travel = (data::deg(0.0), data::deg(0.0));
                return Ok(());
            }
        }
        Err("failed to read mount position".into())
    }

    pub fn total_axis_travel(&self) -> (f64::Angle, f64::Angle) {
        self.total_axis_travel
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
        if let Some((last_axis1_pos, last_axis2_pos)) = self.last_pos {
            let max_travel = data::deg(360.0); // TODO: make it configurable

            let was_axis1_exceeded = self.total_axis_travel.0.abs() > max_travel;
            let was_axis2_exceeded = self.total_axis_travel.1.abs() > max_travel;
            self.total_axis_travel.0 += data::angle_diff(last_axis1_pos, internal1);
            self.total_axis_travel.1 += data::angle_diff(last_axis2_pos, internal2);
            let axis1_exceeded = self.total_axis_travel.0.abs() > max_travel;
            let axis2_exceeded = self.total_axis_travel.1.abs() > max_travel;

            if !was_axis1_exceeded && axis1_exceeded || !was_axis2_exceeded && axis2_exceeded {
                let cb = self.max_travel_exceeded_callback.clone().unwrap();
                cb(self, axis1_exceeded, axis2_exceeded);
            }
        }
        self.last_pos = Some((internal1, internal2));
        Ok((self.axis1_ofs + internal1, self.axis2_ofs + internal2))
    }
}
