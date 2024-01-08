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

mod data;
mod event_handling;
mod key_poller;

use std::rc::Rc;

fn main() {
    let window = Rc::new(pancurses::initscr());
    window.keypad(true);
    pancurses::noecho();
    pancurses::curs_set(0);
    window.nodelay(true);

    pasts::block_on(event_handling::event_loop(window));
}
