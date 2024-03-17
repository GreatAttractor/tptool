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

use pointing_utils::scan_fmt;
use std::{cell::RefCell, collections::HashMap, error::Error, rc::Rc};
use strum::{EnumDiscriminants, IntoEnumIterator};
use strum_macros as sm;
use strum_macros::{IntoStaticStr, EnumString};


mod serialized_event {
    use std::error::Error;

    #[derive(Clone, Debug)]
    pub struct SerializedEvent(String);

    impl SerializedEvent {
        pub fn as_str(&self) -> &str { &self.0 }

        pub fn from_str(s: &str) -> Result<SerializedEvent, Box<dyn Error>> {
            Ok(SerializedEvent(s.to_string()))
        }

        pub fn from_event(event: &stick::Event) -> SerializedEvent {
            SerializedEvent(format!("{}", event).split(' ').next().unwrap().to_string())
        }
    }
}

pub use serialized_event::SerializedEvent;

#[derive(Debug)]
pub struct StickEvent {
    pub id: u64,
    pub event: stick::Event
}

pub enum EventValue {
    Discrete(bool),
    Analog(f64)
}

#[derive(Clone, Debug)]
pub struct SourceAction {
    pub ctrl_id: u64,
    pub ctrl_name: String, // only for user information, not used to filter controller events
    pub event: SerializedEvent
}

impl SourceAction {
    pub fn serialize(&self) -> String {
        format!("[{:016X}]{}", self.ctrl_id, self.event.as_str())
    }

    pub fn matches(&self, event: &StickEvent) -> bool {
        self.ctrl_id == event.id && SerializedEvent::from_event(&event.event).as_str() == self.event.as_str()
    }
}

impl std::str::FromStr for SourceAction {
    type Err = Box<dyn Error>;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (ctrl_id, event_str) =
            scan_fmt::scan_fmt!(s, "[{x}]{}", [hex u64], String)?;

        Ok(SourceAction{ ctrl_id, ctrl_name: "".into(), event: SerializedEvent::from_str(&event_str)? })
    }
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq, sm::EnumIter, sm::EnumDiscriminants, sm::IntoStaticStr)]
//#[strum_discriminants(derive(IntoStaticStr, EnumString))]
pub enum TargetAction {
    MountAxis1, // via controller's analog axis
    MountAxis2, // via controller's analog axis
    MountAxis1Pos, // via controller's discrete button
    MountAxis1Neg, // via controller's discrete button
    MountAxis2Pos, // via controller's discrete button
    MountAxis2Neg, // via controller's discrete button
    StopMount,
    ToggleTracking,
    SaveAdjustment,
    CancelAdjustment,
    IncreaseSlewSpeed,
    DecreaseSlewSpeed,
}

impl TargetAction {
    pub fn config_key(&self) -> &'static str { self.into() }
}

impl std::fmt::Display for TargetAction  {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", match self {
            TargetAction::MountAxis1 => "Mount axis 1",
            TargetAction::MountAxis2 => "Mount axis 2",
            TargetAction::MountAxis1Pos => "Mount axis 1 / positive",
            TargetAction::MountAxis1Neg => "Mount axis 1 / negative",
            TargetAction::MountAxis2Pos => "Mount axis 2 / positive",
            TargetAction::MountAxis2Neg => "Mount axis 2 / negative",
            TargetAction::StopMount => "Stop mount",
            TargetAction::ToggleTracking => "Toggle tracking",
            TargetAction::SaveAdjustment => "Save adjustment",
            TargetAction::CancelAdjustment => "Cancel adjustment",
            TargetAction::IncreaseSlewSpeed => "Increase slew speed",
            TargetAction::DecreaseSlewSpeed => "Decrease slew speed",
        })
    }
}

#[derive(Debug)]
pub struct ActionAssignments {
    map: std::collections::HashMap<TargetAction, Option<SourceAction>>
}

impl ActionAssignments {
    pub fn new() -> ActionAssignments {
        let mut map = std::collections::HashMap::new();
        for target_action in TargetAction::iter() {
            map.insert(target_action, None);
        }

        ActionAssignments{ map }
    }

    pub fn get(&self, target_action: TargetAction) -> &Option<SourceAction> {
        self.map.get(&target_action).unwrap()
    }

    pub fn set(&mut self, target_action: TargetAction, src_action: Option<SourceAction>) {
        self.map.entry(target_action).and_modify(|e| *e = src_action);
    }
}

pub fn event_value(event: &stick::Event) -> EventValue {
    match event {
        stick::Event::ActionA(b) => EventValue::Discrete(*b),
        stick::Event::ActionB(b) => EventValue::Discrete(*b),
        stick::Event::ActionC(b) => EventValue::Discrete(*b),
        stick::Event::ActionD(b) => EventValue::Discrete(*b),
        stick::Event::ActionH(b) => EventValue::Discrete(*b),
        stick::Event::ActionL(b) => EventValue::Discrete(*b),
        stick::Event::ActionM(b) => EventValue::Discrete(*b),
        stick::Event::ActionR(b) => EventValue::Discrete(*b),
        stick::Event::ActionV(b) => EventValue::Discrete(*b),
        stick::Event::Apu(b) => EventValue::Discrete(*b),
        stick::Event::AutopilotAlt(b) => EventValue::Discrete(*b),
        stick::Event::AutopilotPath(b) => EventValue::Discrete(*b),
        stick::Event::AutopilotToggle(b) => EventValue::Discrete(*b),
        stick::Event::BoatBackward(b) => EventValue::Discrete(*b),
        stick::Event::BoatForward(b) => EventValue::Discrete(*b),
        stick::Event::Brake(f) => EventValue::Analog(*f),
        stick::Event::Bumper(b) => EventValue::Discrete(*b),
        stick::Event::BumperL(b) => EventValue::Discrete(*b),
        stick::Event::BumperR(b) => EventValue::Discrete(*b),
        stick::Event::Cam(b) => EventValue::Discrete(*b),
        stick::Event::CamX(f) => EventValue::Analog(*f),
        stick::Event::CamY(f) => EventValue::Analog(*f),
        stick::Event::CamZ(f) => EventValue::Analog(*f),
        stick::Event::ChinaBackward(b) => EventValue::Discrete(*b),
        stick::Event::ChinaForward(b) => EventValue::Discrete(*b),
        stick::Event::Context(b) => EventValue::Discrete(*b),
        stick::Event::Down(b) => EventValue::Discrete(*b),
        stick::Event::Dpi(b) => EventValue::Discrete(*b),
        stick::Event::Eac(b) => EventValue::Discrete(*b),
        stick::Event::EngineFuelFlowL(b) => EventValue::Discrete(*b),
        stick::Event::EngineFuelFlowR(b) => EventValue::Discrete(*b),
        stick::Event::EngineIgnitionL(b) => EventValue::Discrete(*b),
        stick::Event::EngineIgnitionR(b) => EventValue::Discrete(*b),
        stick::Event::EngineMotorL(b) => EventValue::Discrete(*b),
        stick::Event::EngineMotorR(b) => EventValue::Discrete(*b),
        stick::Event::Exit(b) => EventValue::Discrete(*b),
        stick::Event::FlapsDown(b) => EventValue::Discrete(*b),
        stick::Event::FlapsUp(b) => EventValue::Discrete(*b),
        stick::Event::Gas(f) => EventValue::Analog(*f),
        stick::Event::HatDown(b) => EventValue::Discrete(*b),
        stick::Event::HatLeft(b) => EventValue::Discrete(*b),
        stick::Event::HatRight(b) => EventValue::Discrete(*b),
        stick::Event::HatUp(b) => EventValue::Discrete(*b),
        stick::Event::Joy(b) => EventValue::Discrete(*b),
        stick::Event::JoyX(f) => EventValue::Analog(*f),
        stick::Event::JoyY(f) => EventValue::Analog(*f),
        stick::Event::JoyZ(f) => EventValue::Analog(*f),
        stick::Event::LandingGearSilence(b) => EventValue::Discrete(*b),
        stick::Event::Left(b) => EventValue::Discrete(*b),
        stick::Event::MenuL(b) => EventValue::Discrete(*b),
        stick::Event::MenuR(b) => EventValue::Discrete(*b),
        stick::Event::MicDown(b) => EventValue::Discrete(*b),
        stick::Event::MicLeft(b) => EventValue::Discrete(*b),
        stick::Event::MicPush(b) => EventValue::Discrete(*b),
        stick::Event::MicRight(b) => EventValue::Discrete(*b),
        stick::Event::MicUp(b) => EventValue::Discrete(*b),
        stick::Event::Mouse(b) => EventValue::Discrete(*b),
        stick::Event::MouseX(f) => EventValue::Analog(*f),
        stick::Event::MouseY(f) => EventValue::Analog(*f),
        stick::Event::Number(_, b) => EventValue::Discrete(*b),
        stick::Event::PaddleLeft(b) => EventValue::Discrete(*b),
        stick::Event::PaddleRight(b) => EventValue::Discrete(*b),
        stick::Event::Pinky(b) => EventValue::Discrete(*b),
        stick::Event::PinkyBackward(b) => EventValue::Discrete(*b),
        stick::Event::PinkyForward(b) => EventValue::Discrete(*b),
        stick::Event::PinkyLeft(b) => EventValue::Discrete(*b),
        stick::Event::PinkyRight(b) => EventValue::Discrete(*b),
        stick::Event::PovDown(b) => EventValue::Discrete(*b),
        stick::Event::PovLeft(b) => EventValue::Discrete(*b),
        stick::Event::PovRight(b) => EventValue::Discrete(*b),
        stick::Event::PovUp(b) => EventValue::Discrete(*b),
        stick::Event::RadarAltimeter(b) => EventValue::Discrete(*b),
        stick::Event::Right(b) => EventValue::Discrete(*b),
        stick::Event::Rudder(f) => EventValue::Analog(*f),
        stick::Event::Scroll(b) => EventValue::Discrete(*b),
        stick::Event::ScrollX(f) => EventValue::Analog(*f),
        stick::Event::ScrollY(f) => EventValue::Analog(*f),
        stick::Event::Slew(f) => EventValue::Analog(*f),
        stick::Event::SpeedbrakeBackward(b) => EventValue::Discrete(*b),
        stick::Event::SpeedbrakeForward(b) => EventValue::Discrete(*b),
        stick::Event::Throttle(f) => EventValue::Analog(*f),
        stick::Event::ThrottleButton(b) => EventValue::Discrete(*b),
        stick::Event::ThrottleL(f) => EventValue::Analog(*f),
        stick::Event::ThrottleR(f) => EventValue::Analog(*f),
        stick::Event::Trigger(b) => EventValue::Discrete(*b),
        stick::Event::TriggerL(f) => EventValue::Analog(*f),
        stick::Event::TriggerR(f) => EventValue::Analog(*f),
        stick::Event::TrimDown(b) => EventValue::Discrete(*b),
        stick::Event::TrimLeft(b) => EventValue::Discrete(*b),
        stick::Event::TrimRight(b) => EventValue::Discrete(*b),
        stick::Event::TrimUp(b) => EventValue::Discrete(*b),
        stick::Event::Up(b) => EventValue::Discrete(*b),
        stick::Event::Volume(f) => EventValue::Analog(*f),
        stick::Event::Wheel(f) => EventValue::Analog(*f),

        _ => panic!("unrecognized event: {:?}", event)
    }
}

/// Returns `true` for button-like events, `false` for analog-axis events.
fn is_discrete(event: &stick::Event) -> bool {
    match event {
        stick::Event::Exit(_)
        | stick::Event::ActionA(_)
        | stick::Event::ActionB(_)
        | stick::Event::ActionC(_)
        | stick::Event::ActionH(_)
        | stick::Event::ActionV(_)
        | stick::Event::ActionD(_)
        | stick::Event::MenuL(_)
        | stick::Event::MenuR(_)
        | stick::Event::Joy(_)
        | stick::Event::Cam(_)
        | stick::Event::BumperL(_)
        | stick::Event::BumperR(_)
        | stick::Event::Up(_)
        | stick::Event::Down(_)
        | stick::Event::Left(_)
        | stick::Event::Right(_)
        | stick::Event::PovUp(_)
        | stick::Event::PovDown(_)
        | stick::Event::PovLeft(_)
        | stick::Event::PovRight(_)
        | stick::Event::HatUp(_)
        | stick::Event::HatDown(_)
        | stick::Event::HatLeft(_)
        | stick::Event::HatRight(_)
        | stick::Event::TrimUp(_)
        | stick::Event::TrimDown(_)
        | stick::Event::TrimLeft(_)
        | stick::Event::TrimRight(_)
        | stick::Event::MicUp(_)
        | stick::Event::MicDown(_)
        | stick::Event::MicLeft(_)
        | stick::Event::MicRight(_)
        | stick::Event::MicPush(_)
        | stick::Event::Trigger(_)
        | stick::Event::Bumper(_)
        | stick::Event::ActionM(_)
        | stick::Event::ActionL(_)
        | stick::Event::ActionR(_)
        | stick::Event::Pinky(_)
        | stick::Event::PinkyForward(_)
        | stick::Event::PinkyBackward(_)
        | stick::Event::FlapsUp(_)
        | stick::Event::FlapsDown(_)
        | stick::Event::BoatForward(_)
        | stick::Event::BoatBackward(_)
        | stick::Event::AutopilotPath(_)
        | stick::Event::AutopilotAlt(_)
        | stick::Event::EngineMotorL(_)
        | stick::Event::EngineMotorR(_)
        | stick::Event::EngineFuelFlowL(_)
        | stick::Event::EngineFuelFlowR(_)
        | stick::Event::EngineIgnitionL(_)
        | stick::Event::EngineIgnitionR(_)
        | stick::Event::SpeedbrakeBackward(_)
        | stick::Event::SpeedbrakeForward(_)
        | stick::Event::ChinaBackward(_)
        | stick::Event::ChinaForward(_)
        | stick::Event::Apu(_)
        | stick::Event::RadarAltimeter(_)
        | stick::Event::LandingGearSilence(_)
        | stick::Event::Eac(_)
        | stick::Event::AutopilotToggle(_)
        | stick::Event::ThrottleButton(_)
        | stick::Event::Mouse(_)
        | stick::Event::Number(_, _)
        | stick::Event::PaddleLeft(_)
        | stick::Event::PaddleRight(_)
        | stick::Event::PinkyLeft(_)
        | stick::Event::PinkyRight(_)
        | stick::Event::Context(_)
        | stick::Event::Dpi(_)
        | stick::Event::Scroll(_) => true,

        _ => false
    }
}
