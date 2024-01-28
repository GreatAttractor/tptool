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
    reexports::enumset,
    Rect,
    theme,
    Vec2,
    View,
    view::{Offset, Position, Resizable},
    views::{DummyView, FixedLayout, LinearLayout, OnLayoutView, Panel, TextContent, TextView}
};

pub struct TuiData {
    pub text_content: Texts
}

pub struct Texts {
    pub controller_name: TextContent,
    pub controller_event: TextContent,
    pub target_dist: TextContent,
    pub target_spd: TextContent,
    pub target_az: TextContent,
    pub target_alt: TextContent,
    pub mount_name: TextContent,
    pub mount_az: TextContent,
    pub mount_alt: TextContent,
    pub mount_az_spd: TextContent,
    pub mount_alt_spd: TextContent,
}

pub fn init(state: &mut ProgramState) {
    let curs = &mut state.cursive_stepper.curs;
	curs.add_global_callback('q', |c| { c.quit(); });

    let tracking = state.tracking.controller();
    curs.add_global_callback('t', move |_| {
        if tracking.is_active() {
            tracking.stop();
        } else {
            tracking.start();
        }
    });

    init_theme(curs);

    let text_content = init_views(curs);
    init_command_bar(curs);

    let tui_data = TuiData{
        text_content
    };
    state.tui = Some(tui_data);
}

fn init_command_bar(curs: &mut cursive::Cursive) {
    curs.screen_mut().add_transparent_layer(
        OnLayoutView::new(
            FixedLayout::new().child(
                Rect::from_point(Vec2::zero()),
                TextView::new(">F<Follow target"),
            ),
            |layout, size| {
                let rect = Rect::from_size((0, size.y - 1), (size.x, 1));
                layout.set_child_position(0, rect);
                layout.layout(size);
            },
        )
        .full_screen(),
    );
}

fn init_views(curs: &mut cursive::Cursive) -> Texts {
    // ---------------------------------
    // Controller
    //
    let controller_name = TextContent::new("(disconnected)");
    let controller_event = TextContent::new("");
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(1), Offset::Absolute(8)),
        Panel::new(LinearLayout::vertical()
            .child(TextView::new_with_content(controller_name.clone()))
            .child(TextView::new_with_content(controller_event.clone()))
        )
        .title("Controller")
        .title_position(HAlign::Left)
    );

    // ---------------------------------
    // Mount
    //
    let mount_name = TextContent::new("(disconnected)");
    let mount_az = TextContent::new("");
    let mount_alt = TextContent::new("");
    let mount_az_spd = TextContent::new("");
    let mount_alt_spd = TextContent::new("");
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(45), Offset::Absolute(1)),
        Panel::new(LinearLayout::vertical()
            .child(TextView::new_with_content(mount_name.clone()))
            .child(
                LinearLayout::horizontal()
                    .child(label_and_content("az. ", mount_az.clone()))
                    .child(DummyView{}.min_width(1))
                    .child(label_and_content("alt. ", mount_alt.clone()))
            )
        )
        .title("Mount")
        .title_position(HAlign::Left)
    );

    // ---------------------------------
    // Target
    //
    let target_dist = TextContent::new("");
    let target_spd = TextContent::new("");
    let target_az = TextContent::new("");
    let target_alt = TextContent::new("");
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(1), Offset::Absolute(1)),
        Panel::new(LinearLayout::vertical()
            .child(
                LinearLayout::horizontal()
                    .child(label_and_content("dist. ", target_dist.clone()))
                    .child(DummyView{}.min_width(1))
                    .child(label_and_content("spd. ", target_spd.clone()))
            )
            .child(label_and_content("az. ", target_az.clone()))
            .child(label_and_content("alt. ", target_alt.clone()))
        )
        .title("Target")
        .title_position(HAlign::Left)
    );

    Texts{
        controller_name,
        controller_event,
        target_dist,
        target_spd,
        target_az,
        target_alt,
        mount_name,
        mount_az,
        mount_alt,
        mount_az_spd,
        mount_alt_spd
    }
}

fn label_and_content(label: &str, content: TextContent) -> LinearLayout {
    LinearLayout::horizontal()
        .child(TextView::new(label))
        .child(TextView::new_with_content(content)
            .style(theme::Style{
                effects: enumset::EnumSet::from(theme::Effect::Simple),
                color: theme::ColorStyle{
                    front: theme::ColorType::Color(theme::Color::Rgb(255, 255, 255)),
                    back: theme::ColorType::InheritParent
                }
            })
        )
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
