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

use crate::data::ProgramState;
use cursive::{
    views::{TextContent, TextView}
};

pub struct TuiData {
    pub tick: TextContent
}

pub fn init(state: &mut ProgramState) {
    let curs = &mut state.cursive_stepper.curs;
	curs.add_global_callback('q', |c| { c.quit(); });

    init_theme(curs);

    let tick = TextContent::new("");
    let tick_label = TextView::new_with_content(tick.clone());
    curs.add_layer(tick_label);

    let tui_data = TuiData{
        tick
    };
    state.tui = Some(tui_data);
}

fn init_theme(curs: &mut cursive::Cursive) {
    let mut theme = curs.current_theme().clone();
    theme.shadow = false;
    theme.borders = cursive::theme::BorderStyle::None;
    theme.palette[cursive::theme::PaletteColor::View] = cursive::theme::BaseColor::Black.light();
    theme.palette[cursive::theme::PaletteColor::Background] = cursive::theme::BaseColor::Black.dark();
    theme.palette[cursive::theme::PaletteColor::TitlePrimary] = cursive::theme::BaseColor::White.light();
    theme.palette[cursive::theme::PaletteColor::Primary] = cursive::theme::BaseColor::White.dark();
    curs.set_theme(theme);
}
