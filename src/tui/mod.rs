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

mod data_source_dialog;
mod mount_dialog;
mod ref_pos_dialog;
mod shadow_view;
mod simple_dialog;
mod zero_pos_dialog;

use crate::{cclone, data::ProgramState, mount::Mount, upgrade};
use cursive::{
    align::HAlign,
    CursiveRunnable,
    CursiveRunner,
    reexports::enumset,
    Rect,
    theme,
    theme::Theme,
    Vec2,
    View,
    view::{Nameable, Offset, Position, Resizable},
    views::{
        Dialog,
        DummyView,
        EditView,
        FixedLayout,
        LinearLayout,
        OnLayoutView,
        Panel,
        ResizedView,
        SelectView,
        TextContent,
        TextView,
        ThemedView
    },
    With
};
use shadow_view::WithShadow;
use std::{cell::RefCell, rc::Rc};

/// Unique Cursive view names.
mod names {
    pub const SERVER_ADDR: &str = "server_addr";
    pub const MOUNT_CONNECTION: &str = "mount_connection";
    pub const REF_POS_AZ: &str = "ref_pos_azimuth";
    pub const REF_POS_ALT: &str = "ref_pos_altitude";
    pub const REF_POS_SEL_PRESET: &str = "ref_pos_selected_preset";
    pub const SIMPLE_DIALOG_TEXT: &str = "simple_dialog_text";
}

#[macro_export]
macro_rules! tui {
    ($tui_rc:expr) => { $tui_rc.borrow().as_ref().unwrap() };
}

macro_rules! tui_mut {
    ($tui_rc:expr) => { $tui_rc.borrow_mut().as_mut().unwrap() };
}

macro_rules! show_dlg_on_global_callback {
    ($dialog_func:expr, $curs:expr, $tui:expr, $($dialog_params:expr),*) => {
        if tui!($tui.upgrade().unwrap()).showing_dialog { return; }
        tui_mut!($tui.upgrade().unwrap()).showing_dialog = true;
        let dialog_theme = create_dialog_theme($curs);

        $curs.screen_mut().add_transparent_layer_at(
            Position::new(Offset::Center, Offset::Center),
            WithShadow::new(ThemedView::new(
                dialog_theme.clone(),
                $dialog_func($tui.clone(), $($dialog_params),*)
            ))
        );
    };
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
    pub mount_total_az_travel: TextContent,
    pub tracking_state: TextContent
}

struct CommandBarBuilder {
    highlight: theme::Style,
    contents: cursive::utils::span::SpannedString<theme::Style>
}

impl CommandBarBuilder {
    fn new() -> CommandBarBuilder {
        CommandBarBuilder{
            highlight: theme::Style{
                effects: enumset::EnumSet::from(theme::Effect::Simple),
                color: theme::ColorStyle{
                    front: theme::ColorType::Color(theme::Color::Rgb(0, 0, 0)),
                    back: theme::ColorType::Color(theme::Color::Rgb(200, 200, 200)),
                }
            },
            contents: cursive::utils::span::SpannedString::new(),
        }
    }

    fn command(mut self, highlighted: char, descr: &str) -> CommandBarBuilder {
        for s in [' ', highlighted, ' '] {
            self.contents.append_styled(s, self.highlight);
        }
        for s in [" ", descr, "  "] {
            self.contents.append_plain(s);
        }

        self
    }

    fn build(self) -> TextView {
        TextView::new(self.contents)
    }
}

pub fn get_edit_view_str(curs: &mut cursive::Cursive, name: &str) -> Rc<String> {
    curs.call_on_name(name, |v: &mut EditView| { v.get_content() }).unwrap()
}

pub fn set_edit_view_str<S: Into<String>>(curs: &mut cursive::Cursive, name: &str, value: S) {
    curs.call_on_name(name, |v: &mut EditView| { v.set_content(value) });
}

pub fn get_select_view_idx(curs: &mut cursive::Cursive, name: &str) -> usize {
    curs.call_on_name(name, |v: &mut SelectView<usize>| *v.selection().unwrap()).unwrap()
}

pub fn init(state: &mut ProgramState) {
    let curs = &mut state.cursive_stepper.curs;

	curs.add_global_callback('q', |c| { c.quit(); });

    curs.add_global_callback('s', cclone!([@weak (state.mount) as mount, (state.tracking.controller()) as tracking],
        move |_| {
            let mount = mount.upgrade().unwrap();
            let mut mount = mount.borrow_mut();
            if let Some(mount) = mount.as_mut() {
                if let Err(e) = mount.stop() {
                    log::error!("error stopping the mount: {}", e);
                }
                tracking.stop();
            }
        }
    ));

    curs.add_global_callback('t', cclone!([(state.tracking.controller()) as tracking], move |_| {
        if tracking.is_active() {
            tracking.stop();
        } else {
            tracking.start();
        }
    }));

    curs.add_global_callback('d', cclone!([
        @weak (state.tui) as tui,
        (state.data_receiver.connection()) as connection,
        @weak (state.config) as config
        ], move |curs| {
            show_dlg_on_global_callback!(data_source_dialog::dialog, curs, tui, connection.clone(), config.clone());
        }
    ));

    curs.add_global_callback('m', cclone!([
        @weak (state.tui) as tui,
        @weak (state.mount) as mount,
        @weak (state.config) as config,
        (state.tracking.controller()) as tracking
        ], move |curs| {
            show_dlg_on_global_callback!(mount_dialog::dialog, curs, tui, mount.clone(), config.clone(), tracking.clone());
        }
    ));

    curs.add_global_callback('r', cclone!([
        @weak (state.tui) as tui,
        @weak (state.mount) as mount,
        @weak (state.config) as config
        ], move |curs| {
            if mount.upgrade().unwrap().borrow().is_none() {
                msg_box(curs, "Not connected to a mount.", "Error");
            } else {
                show_dlg_on_global_callback!(ref_pos_dialog::dialog, curs, tui.clone(), mount.clone(), config.clone());
            }
        }
    ));

    curs.add_global_callback('z', cclone!([@weak (state.tui) as tui, @weak (state.mount) as mount], move |curs| {
        if mount.upgrade().unwrap().borrow().is_none() {
            msg_box(curs, "Not connected to a mount.", "Error");
        } else {
            show_dlg_on_global_callback!(zero_pos_dialog::dialog, curs, tui.clone(), mount.clone());
        }
    }));

    let main_theme = create_main_theme(curs.current_theme());
    curs.set_theme(main_theme);

    let text_content = init_views(curs);
    init_command_bar(curs);

    *state.tui.borrow_mut() = Some(TuiData{
        text_content,
        showing_dialog: false
    });

    curs.refresh();
}

fn init_command_bar(curs: &mut cursive::Cursive) {
    curs.screen_mut().add_transparent_layer(
        OnLayoutView::new(
            FixedLayout::new().child(
                Rect::from_point(Vec2::zero()),
                CommandBarBuilder::new()
                    .command('T', "Toggle tracking")
                    .command('S', "Stop slewing")
                    .command('D', "Data source")
                    .command('M', "Mount")
                    .command('R', "Ref. position")
                    .command('Z', "Zero position")
                    .command('Q', "Quit")
                    .build()
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
    // Status
    //
    let tracking_state = TextContent::new("disabled");
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(1), Offset::Absolute(8)),
        Panel::new(LinearLayout::vertical()
            .child(label_and_content("Tracking: ", tracking_state.clone()))
        )
        .title("Status")
        .title_position(HAlign::Left)
    );

    // ---------------------------------
    // Controller
    //
    let controller_name = TextContent::new("(disconnected)");
    let controller_event = TextContent::new("");
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(45), Offset::Absolute(8)),
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
    let mount_total_az_travel = TextContent::new("");
    curs.screen_mut().add_layer_at(
        Position::new(Offset::Absolute(45), Offset::Absolute(1)),
        Panel::new(LinearLayout::vertical()
            .child(TextView::new_with_content(mount_name.clone()))
            .child(
                LinearLayout::horizontal()
                    .child(label_and_content("az. ", mount_az.clone()))
                    .child(DummyView{}.min_width(2))
                    .child(label_and_content("alt. ", mount_alt.clone()))
            )
            .child(
                LinearLayout::horizontal()
                    .child(label_and_content("total az. travel ", mount_total_az_travel.clone()))
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
        mount_total_az_travel,
        tracking_state
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

/// Simplifies passing weak references to closures. Instead of:
///
///   let r1 = Rc::new(1);
///   let r2 = Rc::new(2);
///   let r3 = Rc::new(3);
///
///   let r1_weak_ref = Rc::downgrade(&r1);
///   let r2_weak_ref = Rc::downgrade(&r2);
///   let r3_strong_ref = Rc::clone(&r3);
///   let c = move || { / *do sth with r1_weak_ref, r2_weak_ref, r3_strong_ref*/ };
///
/// one can write:
///
///   let c = cclone!([@weak r1, @weak r2, r3], move || { / *do sth with r1, r2, r3*/ });
///
/// To pass an expression:
///
///   let c = cclone!([@weak (struct1.struct2.rc_field) as my_name_of_rc_field], || { /* ... */ });
///
/// Use `upgrade!` to automatically upgrade `Weak`s inside the closure; instead of:
///
///   let r1 = r1.upgrade().unwrap();
///   let r2 = r2.upgrade().unwrap();
///
/// one can write:
///
///   upgrade!(r1, r2);
///
#[macro_export]
macro_rules! cclone {
    ([$($tt:tt)*], $expr:expr) => {{
        cclone!($($tt)*);

        $expr
    }};

    ($(,)? @weak ($expr:expr) as $ident:ident $($tt:tt)*) => {
        let $ident = Rc::downgrade(&$expr);
        cclone!($($tt)*);
    };

    ($(,)? @weak $ident:ident $($tt:tt)*) => {
        let $ident = Rc::downgrade(&$ident);
        cclone!($($tt)*);
    };

    ($(,)? ($expr:expr) as $ident:ident $($tt:tt)*) => {
        let $ident = ::std::clone::Clone::clone(&$expr);
        cclone!($($tt)*);
    };

    ($(,)? $ident:ident $($tt:tt)*) => {
        let $ident = ::std::clone::Clone::clone(&$ident);
        cclone!($($tt)*);
    };

    ($(,)?) => {};
}

#[macro_export]
macro_rules! upgrade {
    ($(,)? $ident:ident $($tt:tt)*) => {
        let $ident = $ident.upgrade().unwrap();
        crate::upgrade!($($tt)*);
    };

    ($(,)?) => {};
}

pub fn msg_box(curs: &mut cursive::Cursive, text: &str, title: &str) {
    let dt = create_dialog_theme(curs);
    curs.screen_mut().add_transparent_layer(WithShadow::new(ThemedView::new(
        dt,
        Dialog::text(text).title(title).dismiss_button("OK")
    )));
}

fn create_dialog_theme(curs: &cursive::Cursive) -> theme::Theme {
    let mut theme = curs.current_theme().clone();
    theme.borders = theme::BorderStyle::Simple;
    theme
}

fn close_dialog(curs: &mut cursive::Cursive, tui: &Rc<RefCell<Option<TuiData>>>) {
    curs.pop_layer();
    tui_mut!(tui).showing_dialog = false; // TODO: make sure only global-callback triggered dialogs call this
}
