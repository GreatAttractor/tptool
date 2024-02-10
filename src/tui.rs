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

use crate::{data::ProgramState, data_receiver};
use cursive::{
    align::HAlign,
    reexports::enumset,
    Rect,
    theme,
    theme::Theme,
    Vec2,
    View,
    view::{Nameable, Offset, Position, Resizable},
    views::{
        CircularFocus,
        Dialog,
        DummyView,
        EditView,
        FixedLayout,
        LinearLayout,
        OnLayoutView,
        Panel,
        TextContent,
        TextView,
        ThemedView
    },
    With
};
use std::{cell::RefCell, rc::Rc};

mod names {
    pub const SERVER_ADDR: &str = "server_addr";
}

macro_rules! tui {
    ($tui_rc:ident) => { $tui_rc.borrow().as_ref().unwrap() };
}

macro_rules! tui_mut {
    ($tui_rc:ident) => { $tui_rc.borrow_mut().as_mut().unwrap() };
}

pub struct TuiData {
    pub text_content: Texts,
    pub showing_dialog: bool
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

    let tui = Rc::clone(&state.tui);
    let connection = state.data_receiver.connection();
    curs.add_global_callback('c', move |curs| {
        show_data_source_dialog(curs, &tui, &connection);
    });

    let main_theme = create_main_theme(curs.current_theme());
    curs.set_theme(main_theme);

    let text_content = init_views(curs);
    init_command_bar(curs);

    *state.tui.borrow_mut() = Some(TuiData{
        text_content,
        showing_dialog: false
    });
}

fn init_command_bar(curs: &mut cursive::Cursive) {
    curs.screen_mut().add_transparent_layer(
        OnLayoutView::new(
            FixedLayout::new().child(
                Rect::from_point(Vec2::zero()),
                TextView::new(">T<Toggle target tracking  >C<Connect to data source"),
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

fn create_main_theme(base: &Theme) -> Theme {
    let mut theme = base.clone();

    theme.shadow = false;
    theme.borders = theme::BorderStyle::None;
    theme.palette[theme::PaletteColor::View] = theme::Color::Rgb(60, 60, 60);
    theme.palette[theme::PaletteColor::Background] = theme::Color::Rgb(30, 30, 30);
    theme.palette[theme::PaletteColor::TitlePrimary] = theme::Color::Rgb(255, 255, 255);
    theme.palette[theme::PaletteColor::Primary] = theme::Color::Rgb(180, 180, 180);
    theme
}

fn make_closure<T>(
    arg1: &Rc<RefCell<T>>,
    f: impl Fn(&mut cursive::Cursive, &Rc<RefCell<T>>)
) -> impl Fn(&mut cursive::Cursive) {
    let data = Rc::downgrade(arg1);
    move |curs| {
        let data = data.upgrade().unwrap();
        f(curs, &data);
    }
}

fn make_closure2<T1, T2: Clone>(
    arg1: &Rc<RefCell<T1>>,
    arg2: &T2,
    f: impl Fn(&mut cursive::Cursive, &Rc<RefCell<T1>>, T2, &str)
) -> impl Fn(&mut cursive::Cursive, &str) {
    let arg1 = Rc::downgrade(arg1);
    let arg2 = arg2.clone();
    move |curs, s| {
        let arg1 = arg1.upgrade().unwrap();
        f(curs, &arg1, arg2.clone(), s);
    }
}

fn make_closure3<T1, T2: Clone>(
    arg1: &Rc<RefCell<T1>>,
    arg2: &T2,
    f: impl Fn(&mut cursive::Cursive, &Rc<RefCell<T1>>, T2)
) -> impl Fn(&mut cursive::Cursive) {
    let arg1 = Rc::downgrade(arg1);
    let arg2 = arg2.clone();
    move |curs| {
        let arg1 = arg1.upgrade().unwrap();
        f(curs, &arg1, arg2.clone());
    }
}

fn show_data_source_dialog(
    curs: &mut cursive::Cursive,
    tui: &Rc<RefCell<Option<TuiData>>>,
    connection: &data_receiver::Connection
) {
    if tui!(tui).showing_dialog { return; }
    tui_mut!(tui).showing_dialog = true;
    let dialog_theme = create_dialog_theme(curs);

    curs.screen_mut().add_layer_at(
        Position::new(Offset::Center, Offset::Center),
        ThemedView::new(
            dialog_theme.clone(),
            Dialog::around(
                LinearLayout::horizontal()
                    .child(TextView::new("Server address:"))
                    .child(EditView::new()
                        .on_submit(make_closure2(tui, connection, |curs, tui, connection, s| {
                            on_connect_to_data_source(curs, tui, connection, s);
                        }))
                        .with_name(names::SERVER_ADDR)
                        .fixed_width(20)
                )
            )
            .button("OK", make_closure3(tui, connection, |curs, tui, connection| {
                let server_address = curs.call_on_name(
                    names::SERVER_ADDR, |v: &mut EditView| { v.get_content() }
                ).unwrap();
                on_connect_to_data_source(curs, tui, connection, &server_address);
            }))
            .button("Cancel", make_closure(tui, |curs, tui| close_dialog(curs, tui)))
            .title("Connect to data source")
            .wrap_with(CircularFocus::new)
            .wrap_tab()
        )
    );
}

fn on_connect_to_data_source(
    curs: &mut cursive::Cursive,
    tui: &Rc<RefCell<Option<TuiData>>>,
    connection: data_receiver::Connection,
    server_addr: &str
) {
    match connection.connect(server_addr) {
        Ok(()) => {
            log::info!("connected to data source {}", server_addr);
            close_dialog(curs, tui);
        },

        Err(e) => {
            log::error!("error connecting to data source \"{}\": {}", server_addr, e);
            curs.add_layer(ThemedView::new(
                create_dialog_theme(curs),
                Dialog::info(format!("Failed to connect to \"{}\":\n{}.", server_addr, e)).title("Error")
            ));
        }
    }
}

fn create_dialog_theme(curs: &cursive::Cursive) -> theme::Theme {
    let mut theme = curs.current_theme().clone();
    theme.borders = theme::BorderStyle::Simple;
    theme
}

fn close_dialog(curs: &mut cursive::Cursive, tui: &Rc<RefCell<Option<TuiData>>>) {
    curs.pop_layer();
    tui_mut!(tui).showing_dialog = false;
}
