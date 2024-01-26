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

use crate::mount::Mount;
use std::error::Error;
use pointing_utils::{MountSimulatorMessage, read_line, uom};
use std::{io::Write, net::TcpStream};
use uom::si::f64;

pub struct Simulator {
    address: String,
    stream: TcpStream
}

impl Simulator {
    pub fn new(address: &str) -> Result<Simulator, Box<dyn Error>> {
        let stream = TcpStream::connect(address)?;
        Ok(Simulator{ address: address.into(), stream })
    }
}

type Msg = MountSimulatorMessage;

impl Mount for Simulator {
    fn get_info(&self) -> String {
        format!("Simulator on {}", self.address)
    }

    fn slew(&mut self, axis1: f64::AngularVelocity, axis2: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        self.stream.write_all(Msg::Slew{axis1, axis2}.to_string().as_bytes())?;
        let resp_str = read_line(&mut self.stream)?;
        let msg = resp_str.parse::<Msg>()?;
        if let Msg::Reply(reply) = msg {
            reply
        } else {
            Err(format!("invalid message: {}", resp_str).into())
        }
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
