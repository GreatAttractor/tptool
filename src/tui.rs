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
    align::HAlign,
    Rect,
    theme,
    view::{Offset, Position, Resizable},
    views::{DummyView, FixedLayout, LinearLayout, OnLayoutView, Panel, TextContent, TextView},
    View,
    XY
};

pub struct TuiData {
    pub text_content: Texts
}

pub struct Texts {
    pub controller_name: TextContent,
    pub controller_event: TextContent,
    pub tick: TextContent,
}

pub fn init(state: &mut ProgramState) {
    let curs = &mut state.cursive_stepper.curs;
	curs.add_global_callback('q', |c| { c.quit(); });

    init_theme(curs);

    let text_content = init_views(curs);

    let tui_data = TuiData{
        text_content
    };
    state.tui = Some(tui_data);
}

fn init_views(curs: &mut cursive::Cursive) -> Texts {
    let tick = TextContent::new("");
    let tick_label = TextView::new_with_content(tick.clone());
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(0), Offset::Absolute(1)),
        tick_label
    );

    let controller_name = TextContent::new("(disconnected)");
    let controller_event = TextContent::new("");
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(15), Offset::Absolute(1)),
        Panel::new(LinearLayout::vertical()
            .child(TextView::new_with_content(controller_name.clone()))
            .child(TextView::new_with_content(controller_event.clone()))
        )
        .title("Controller")
        .title_position(HAlign::Left)
    );

    Texts{ controller_name, controller_event, tick }
}

fn init_theme(curs: &mut cursive::Cursive) {
    let mut theme = curs.current_theme().clone();
    theme.shadow = false;
    theme.borders = theme::BorderStyle::None;
    theme.palette[theme::PaletteColor::View] = theme::Color::Rgb(60, 60, 60);
    theme.palette[theme::PaletteColor::Background] = theme::Color::Rgb(30, 30, 30);
    theme.palette[theme::PaletteColor::TitlePrimary] = theme::Color::Rgb(255, 255, 255);
    theme.palette[theme::PaletteColor::Primary] = theme::Color::Rgb(180, 180, 180);
    curs.set_theme(theme);
}
