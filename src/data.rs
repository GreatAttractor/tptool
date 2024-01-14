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

use async_std::stream::Stream;
use crate::{cursive_stepper::CursiveRunnableStepper, tui::TuiData};
use std::{future::Future, marker::Unpin, pin::Pin, task::{Context, Poll}};


pub struct ProgramState {
    pub counter: usize,
    pub cursive_stepper: CursiveRunnableStepper,
    pub timer: Pin<Box<dyn Future<Output = ()>>>,
    pub tui: Option<TuiData>, // always `Some` after program start
    pub listener: stick::Listener,
    pub controllers: Vec<stick::Controller>,
    pub data_receiver: AsyncLinesWrapper<async_std::io::BufReader<async_std::net::TcpStream>>
}

impl ProgramState {
    pub fn tui(&self) -> &TuiData { self.tui.as_ref().unwrap() }

    pub fn refresh_tui(&mut self) { self.cursive_stepper.curs.refresh(); }
}

pub struct AsyncLinesWrapper<R: async_std::io::BufRead + Unpin> {
    object: async_std::io::Lines<R>
}

impl<R: async_std::io::BufRead + Unpin> AsyncLinesWrapper<R> {
    pub fn new(object: async_std::io::Lines<R>) -> AsyncLinesWrapper<R> { AsyncLinesWrapper{ object } }
}

impl<R: async_std::io::BufRead + Unpin> Future for AsyncLinesWrapper<R>  {
    type Output = Option<Result<String, std::io::Error>>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.object).poll_next(ctx)
    }
}
