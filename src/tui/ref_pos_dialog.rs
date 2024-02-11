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
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("azimuth:  ")) // TODO: ensure alignment of `EditView`s
                    .child(EditView::new()
                        .with_name(names::REF_POS_AZ)
                        .fixed_width(10)
                    )
                    .child(TextView::new("°"))
            )
            .child(
                LinearLayout::horizontal()
                    .child(TextView::new("altitude: "))
                    .child(EditView::new()
                        .with_name(names::REF_POS_ALT)
                        .fixed_width(10)
                    )
                    .child(TextView::new("°"))
            )
    )
    .button("OK", make_closure5(tui, mount, move |curs, tui, mount| {
        let ref_az = curs.call_on_name( names::REF_POS_AZ, |v: &mut EditView| { v.get_content() }).unwrap();
        let ref_alt = curs.call_on_name( names::REF_POS_ALT, |v: &mut EditView| { v.get_content() }).unwrap();

        let ref_az = (*ref_az).parse::<f64>();
        let ref_alt = (*ref_alt).parse::<f64>();

        let err: Option<_> = match (ref_az, ref_alt) {
            (Ok(ref_az), Ok(ref_alt)) => {
                close_dialog(curs, tui);
                if let Err(e) = mount.borrow_mut().as_mut().unwrap().set_reference_position(deg(ref_az), deg(ref_alt)) {
                    msg_box(curs, &format!("Failed to set ref. position:\n{}", e), "Error");
                }
                None
            },

            (Err(e), _) => Some(e),
            (Ok(_), Err(e)) => Some(e)
        };

        if let Some(err) = err { msg_box(curs, &format!("Invalid value: {}.", err), "Error"); }
    }))
    .button("Cancel", make_closure(tui, |curs, tui| close_dialog(curs, tui)))
    .title("Set current reference position")
    .wrap_with(CircularFocus::new)
    .wrap_tab()
}
