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

use crate::{
    cclone,
    data::deg,
    mount,
    tui,
    tui::{close_dialog, msg_box, names, TuiData},
    upgrade
};
use cursive::{
    event,
    view::{Nameable, Resizable, View},
    views::{
        CircularFocus,
        Dialog,
        EditView,
        LinearLayout,
        OnEventView,
        TextView,
    },
    With
};
use std::{cell::RefCell, rc::{Rc, Weak}};

pub fn dialog(
    tui: Weak<RefCell<Option<TuiData>>>,
    mount: Weak<RefCell<Option<mount::MountWrapper>>>
) -> impl View {
    Dialog::around(
        LinearLayout::vertical()
            // TODO: give (and implement) the option of "go to zero position"
            .child(TextView::new("Mark the current mount position as the zero (home) position?"))
    )
    .button("OK", cclone!([tui, mount], move |curs| {
        upgrade!(tui, mount);
        let mut mount = mount.borrow_mut();
        if let Err(e) = mount.as_mut().unwrap().set_zero_position() {
            msg_box(curs, &format!("Failed to set zero position: {}.", e), "Error");
        } else {
            close_dialog(curs, &tui)
        }
    }))
    .button("Cancel", crate::cclone!([tui], move |curs| { upgrade!(tui); close_dialog(curs, &tui); }))
    .title("Zero position")
    .wrap_with(CircularFocus::new)
    .wrap_tab()
    .wrap_with(OnEventView::new)
    .on_event(event::Event::Key(event::Key::Esc), crate::cclone!([tui],
        move |curs| { upgrade!(tui); close_dialog(curs, &tui); }
    ))
}
