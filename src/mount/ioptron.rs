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
use pointing_utils::uom;
use std::error::Error;
use uom::si::{f64, angle, angular_velocity};

// HAE69B takes up to 1.8 s to toggle special mode
const SPECIAL_MODE_SWITCH_MAX_DURATION: std::time::Duration = std::time::Duration::from_secs(5);

pub struct Ioptron {
    model: String,
    device: String,
    serial_port: Box<dyn serialport::SerialPort>,
}

#[derive(Debug)]
enum ResponseType {
    None,
    EndsWith(char),
    NumCharsReceived(usize),
    CharsReceived(String)
}

// HAE69B often does not return command confirmations (e.g., "1")
enum InvalidResponseTreatment {
    Fail,
    IgnoreAndLog(bool)
}

impl Ioptron {
    /// Creates an iOptron mount instance.
    ///
    /// # Parameters
    ///
    /// * `device` - System device name to use for connecting to the mount,
    ///     e.g., "COM3" on Windows or "/dev/ttyUSB0" on Linux.
    ///
    #[must_use]
    pub fn new(device: &str) -> Result<Box<dyn Mount>, Box<dyn Error>> {
        let mut serial_port = serialport::new(device, 115200)
            .data_bits(serialport::DataBits::Eight)
            .flow_control(serialport::FlowControl::None)
            .parity(serialport::Parity::None)
            .stop_bits(serialport::StopBits::One)
            .timeout(std::time::Duration::from_millis(50))
            .open()?;

        let mut mount_id = vec![];

        let model = if let Ok(chars) = send_cmd_and_get_reply(
            &mut serial_port,
            ":MountInfo#".into(),
            ResponseType::NumCharsReceived(4),
            InvalidResponseTreatment::Fail
        ) {
            mount_id = chars.clone();
            if let Ok(s) = String::from_utf8(chars) { model_from_id(s.as_str()) } else { "(unknown)".into() }
        } else {
            "(unknown)".into()
        };

        if mount_id.len() < 1 { return Err("mount ID is empty".into()); }
        if mount_id[0] != b'8' && mount_id[0] != b'9' {
            log::debug!("mount not in special mode, switching...");
            toggle_special_mode(&mut serial_port)?;
            log::debug!("switched successfully");
        }

        Ok(Box::new(Ioptron{
            model,
            device: device.to_string(),
            serial_port,
        }))
    }
}

impl Drop for Ioptron {
    fn drop(&mut self) {
        let _ = self.stop();
        log::debug!("switching mount back to normal mode...");
        if let Err(e) = toggle_special_mode(&mut self.serial_port) {
            log::error!("failed to switch back to normal mode: {}", e);
        } else {
            log::debug!("switched successfully");
        }
    }
}

impl Mount for Ioptron {
    fn get_info(&self) -> String {
        format!("iOptron {} on {}", self.model, self.device)
    }

    fn slew(&mut self, axis1: f64::AngularVelocity, axis2: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        self.slew_axis(Axis::Primary, axis1)?;
        self.slew_axis(Axis::Secondary, axis2)
    }

    fn slew_axis(&mut self, axis: Axis, speed: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        send_cmd_and_get_reply(
            &mut self.serial_port,
            format!(
                ":M{}{:+08}#",
                if axis == Axis::Primary { "0" } else { "1" },
                (speed.get::<angular_velocity::degree_per_second>() * 3600.0 * 100.0) as i32
            ),
            ResponseType::CharsReceived("1".into()),
            InvalidResponseTreatment::IgnoreAndLog(true)
        ).map(|_| ())
    }

    fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        self.slew(deg_per_s(0.0), deg_per_s(0.0))
    }

    fn position(&mut self) -> Result<(f64::Angle, f64::Angle), Box<dyn Error>> {
        let pos1 = &send_cmd_and_get_reply(
            &mut self.serial_port,
            ":P0#".into(),
            ResponseType::NumCharsReceived(11),
            InvalidResponseTreatment::Fail
        )?;
        let pos1 = String::from_utf8(pos1[..10].to_vec())?;
        let pos1 = pos1.parse::<i32>()?;

        let pos2 = &send_cmd_and_get_reply(
            &mut self.serial_port,
            ":P1#".into(),
            ResponseType::NumCharsReceived(11),
            InvalidResponseTreatment::Fail
        )?;
        let pos2 = String::from_utf8(pos2[..10].to_vec())?;
        let pos2 = pos2.parse::<i32>()?;

        Ok((
            f64::Angle::new::<angle::second>(pos1 as f64 * 0.01),
            f64::Angle::new::<angle::second>(pos2 as f64 * 0.01)
        ))
    }
}

fn model_from_id(id: &str) -> String {
    match id {
        "0026"                            => "CEM26".into(),
        "0027"                            => "CEM26-EC".into(),
        "0028"                            => "GEM28".into(),
        "0029"                            => "GEM28-EC".into(),
        "0033" | "0034" | "8033" | "8034" => "HAE29".into(),
        "0035" | "8035"                   => "HAZ31".into(),
        "0040"                            => "CEM40(G)".into(),
        "0041"                            => "CEM40(G)-EC".into(),
        "0043"                            => "GEM45(G)".into(),
        "0044"                            => "GEM45(G)-EC".into(),
        "0050" | "0051" | "8050" | "8051" => "HAE43".into(),
        "0052" | "8052"                   => "HAZ46".into(),
        "0066" | "0068" | "8064"          => "HAE69B".into(),
        "0070"                            => "CEM70(G)".into(),
        "0071"                            => "CEM70(G)-EC".into(),
        "0120"                            => "CEM120".into(),
        "0121"                            => "CEM120-EC".into(),
        "0122"                            => "CEM120-EC2".into(),
        _                                 => format!("(unknown - {})", id)
    }
}

fn toggle_special_mode<T: std::io::Read + std::io::Write>(device: &mut T) -> Result<(), Box<dyn Error>> {
    let id_before = send_cmd_and_get_reply(
        device,
        ":MountInfo#".into(),
        ResponseType::NumCharsReceived(4),
        InvalidResponseTreatment::Fail
    )?;

    send_cmd_and_get_reply(device, ":ZZZ#".into(), ResponseType::None, InvalidResponseTreatment::Fail)?;

    let t0 = std::time::Instant::now();
    while t0.elapsed() <= SPECIAL_MODE_SWITCH_MAX_DURATION {
        if let Ok(id_after) = send_cmd_and_get_reply(
            device,
            ":MountInfo#".into(),
            ResponseType::NumCharsReceived(4),
            InvalidResponseTreatment::IgnoreAndLog(false)
        ) {
            if id_after.len() == 4 && id_after[0] != id_before[0] { return Ok(()); }
        }
        std::thread::sleep(std::time::Duration::from_millis(333));
    }

    Err("toggling special mode is taking too long".into())
}

fn send_cmd_and_get_reply<T: std::io::Read + std::io::Write>(
    device: &mut T,
    cmd: String,
    response_type: ResponseType,
    on_invalid_resp: InvalidResponseTreatment
) -> Result<Vec<u8>, Box<dyn Error>> {
    device.write_all(&cmd.clone().into_bytes())?;

    match &response_type {
        ResponseType::CharsReceived(chars) => { if chars.is_empty() { return Ok(vec![]); } },
        ResponseType::NumCharsReceived(0) | ResponseType::None => { return Ok(vec![]); }
        _ => ()
    }

    let mut reply_error = false;

    let mut buf = vec![];
    let mut reply_received = false;
    while !reply_received {
        buf.push(0);
        if buf.len() > 1024 { return Err("response has too many characters".into()); }
        let blen = buf.len();
        if device.read_exact(&mut buf[blen - 1..blen]).is_err() {
            reply_error = true;
            break;
        }
        reply_received = match response_type {
            ResponseType::EndsWith(ch) => buf[blen - 1] == ch as u8,
            ResponseType::NumCharsReceived(num) => buf.len() == num,
            ResponseType::CharsReceived(ref chars) => buf.len() == chars.len(),
            ResponseType::None => unreachable!()
        };
    }

    if let ResponseType::CharsReceived(chars) = &response_type {
        if &buf != chars.as_bytes() { reply_error = true; }
    }

    if reply_error {
        let message = format!("cmd \"{}\" failed to get expected response: {:?}", cmd, response_type);
        match on_invalid_resp {
            InvalidResponseTreatment::Fail => return Err(message.into()),
            InvalidResponseTreatment::IgnoreAndLog(log) => if log { log::warn!("{}", message); }
        }
    }

    Ok(buf)
}
