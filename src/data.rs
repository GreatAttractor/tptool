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

use core::future::Future;
use crate::{cursive_stepper::CursiveRunnableStepper, tui::TuiData};
use std::pin::Pin;

pub struct ProgramState {
    pub counter: usize,
    pub cursive_stepper: CursiveRunnableStepper,
    pub timer: Pin<Box<dyn Future<Output = ()>>>,
    pub tui: Option<TuiData> // always `Some` after program start
}
