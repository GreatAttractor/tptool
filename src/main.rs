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

mod cursive_stepper;
mod data;
mod event_handling;
mod tui;

fn main() {
	let curs = cursive::default();

    let mut state = data::ProgramState{
        timer: Box::pin(pasts::Past::new((), |()| async_std::task::sleep(std::time::Duration::from_secs(1)))),
        cursive_stepper: cursive_stepper::CursiveRunnableStepper{ curs: curs.into_runner() },
        counter: 0,
        tui: None,
        listener: stick::Listener::default(),
        controllers: vec![]
    };

    tui::init(&mut state);

    pasts::block_on(event_handling::event_loop(state));
}
