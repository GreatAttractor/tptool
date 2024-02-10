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

use crate::{data::deg_per_s, mount::{Axis, Mount}};
use std::error::Error;
use pointing_utils::{MountSimulatorMessage, read_line, uom};
use std::{io::Write, net::TcpStream};
use uom::si::f64;

pub struct Simulator {
    address: String,
    stream: TcpStream,
    /// Last requested speed of primary axis.
    axis1_req_spd: f64::AngularVelocity,
    /// Last requested speed of secondary axis.
    axis2_req_spd: f64::AngularVelocity,
}

impl Simulator {
    pub fn new(address: &str) -> Result<Box<dyn Mount>, Box<dyn Error>> {
        let stream = TcpStream::connect(address)?;
        Ok(Box::new(Simulator{ address: address.into(), stream, axis1_req_spd: deg_per_s(0.0), axis2_req_spd: deg_per_s(0.0) }))
    }
}

type Msg = MountSimulatorMessage;

impl Mount for Simulator {
    fn get_info(&self) -> String {
        format!("Simulator on {}", self.address)
    }

    fn slew(&mut self, axis1: f64::AngularVelocity, axis2: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        self.axis1_req_spd = axis1;
        self.axis2_req_spd = axis2;

        self.stream.write_all(Msg::Slew{axis1, axis2}.to_string().as_bytes())?;
        let resp_str = read_line(&mut self.stream)?;
        let msg = resp_str.parse::<Msg>()?;
        if let Msg::Reply(reply) = msg {
            reply
        } else {
            Err(format!("invalid message: {}", resp_str).into())
        }
    }

    fn slew_axis(&mut self, axis: Axis, speed: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        let axis1_speed = if let Axis::Primary = axis { speed } else { self.axis1_req_spd };
        let axis2_speed = if let Axis::Secondary = axis { speed } else { self.axis2_req_spd };

        self.slew(axis1_speed, axis2_speed)
    }

    fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        self.stream.write_all(Msg::Stop.to_string().as_bytes())?;
        let resp_str = read_line(&mut self.stream)?;
        let msg = resp_str.parse::<Msg>()?;
        if let Msg::Reply(reply) = msg {
            reply
        } else {
            Err(format!("invalid message: {}", resp_str).into())
        }
    }

    fn position(&mut self) -> Result<(f64::Angle, f64::Angle), Box<dyn Error>> {
        self.stream.write_all(Msg::GetPosition.to_string().as_bytes())?;
        let resp_str = read_line(&mut self.stream)?;
        let msg = resp_str.parse::<Msg>()?;
        if let Msg::Position(reply) = msg {
            reply
        } else {
            Err(format!("invalid message: {}", resp_str).into())
        }
    }
}
