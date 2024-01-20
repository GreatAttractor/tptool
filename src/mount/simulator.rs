use crate::mount::Mount;
use std::error::Error;
use pointing_utils::{MountSimulatorMessage, uom};
use std::{io::{Read, Write}, net::TcpStream};
use uom::{si::f64, si::{angle, angular_velocity}};

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

fn read_line<R: Read>(r: &mut R) -> Result<String, Box<dyn Error>> {
    let mut buf = Vec::<u8>::new();
    loop {
        let b_len = buf.len();
        if b_len > 512 { return Err("message too long".into()); }
        r.read_exact(&mut buf[b_len - 1..b_len])?;
        if buf[b_len - 1] == '\n' as u8 {
            buf.truncate(b_len - 1);
            break;
        }
    }

    Ok(std::str::from_utf8(&buf)?.into())
}
