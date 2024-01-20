use crate::mount::Mount;
use std::error::Error;
use pointing_utils::uom;
use std::net::TcpStream;
use uom::{si::f64, si::{angle, angular_velocity}};

pub struct Simulator {
    stream: TcpStream
}

impl Simulator {
    pub fn new(address: &str) -> Result<Simulator, Box<dyn Error>> {
        let stream = TcpStream::connect(address)?;
        Ok(Simulator{ stream })
    }
}

impl Mount for Simulator {
    fn get_info(&self) -> String {
        unimplemented!()
    }

    fn slew(&mut self, axis1: f64::AngularVelocity, axis2: f64::AngularVelocity) -> Result<(), Box<dyn Error>> {
        unimplemented!()
    }

    fn stop(&mut self) -> Result<(), Box<dyn Error>> {
        unimplemented!()
    }

    fn position(&mut self) -> Result<(f64::Angle, f64::Angle), Box<dyn Error>> {
        unimplemented!()
    }
}
