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
    data::deg,
    mount,
    tui,
    tui::{
        close_dialog,
        make_closure,
        make_closure4,
        make_closure5,
        msg_box,
        names,
        TuiData
    }
};
use cursive::{
    view::{Nameable, Resizable, View},
    views::{
        CircularFocus,
        Dialog,
        EditView,
        LinearLayout,
        TextView,
    },
    With
};
use std::{cell::RefCell, rc::Rc};

pub fn dialog(
    tui: &Rc<RefCell<Option<TuiData>>>,
    mount: &Rc<RefCell<Option<mount::MountWrapper>>>
) -> impl View {
    Dialog::around(
        LinearLayout::vertical()
            // TODO: give (and implement) the option of "go to zero position"
            .child(TextView::new("Mark the current mount position as the zero (home) position?"))
    )
    .button("OK", make_closure5(tui, mount, move |curs, tui, mount| {
        if let Err(e) = mount.borrow_mut().as_mut().unwrap().set_zero_position() {
            msg_box(curs, &format!("Failed to set zero position: {}.", e), "Error");
        } else {
            close_dialog(curs, tui)
        }
    }))
    .button("Cancel", make_closure(tui, |curs, tui| close_dialog(curs, tui)))
    .title("Zero position")
    .wrap_with(CircularFocus::new)
    .wrap_tab()
}
