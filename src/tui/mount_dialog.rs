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
        DummyView,
        EditView,
        LinearLayout,
        RadioGroup,
        TextContent,
        TextView,
    },
    With
};
use std::{cell::RefCell, rc::Rc};

#[derive(Copy, Clone)]
enum MountType {
    Simulator,
    Ioptron
}

impl MountType {
    fn connection_param_descr(&self) -> &'static str {
        match self {
            MountType::Simulator => "IP address:",
            MountType::Ioptron => "Serial device (e.g., \"/dev/ttyUSB0\" on Linux\nor \"COM3\" on Windows):",
        }
    }
}

pub fn dialog(
    tui: &Rc<RefCell<Option<TuiData>>>,
    mount: &Rc<RefCell<Option<mount::MountWrapper>>>
) -> impl View {
    let param_descr_content = TextContent::new("");
    let param_descr = TextView::new_with_content(param_descr_content.clone());

    let mut rb_group = RadioGroup::new()
        .on_change(move |_, mount_type: &MountType| {
            param_descr_content.set_content(mount_type.connection_param_descr());
        });
    let rb_group2 = rb_group.clone();

    Dialog::around(
        LinearLayout::vertical()
            .child(rb_group.button(MountType::Simulator, "Simulator"))
            .child(rb_group.button(MountType::Ioptron, "iOptron"))
            .child(DummyView{})
            .child(param_descr)
            .child(EditView::new()
                .on_submit(make_closure4(tui, mount, move |curs, tui, mount, s| {
                    on_connect_to_mount(curs, tui, &mount, *rb_group.selection(), s);
                }))
                .with_name(names::MOUNT_CONNECTION)
                .fixed_width(20)
            )
    )
    .button("OK", make_closure5(tui, mount, move |curs, tui, mount| {
        let connection_param = curs.call_on_name(
            names::MOUNT_CONNECTION, |v: &mut EditView| { v.get_content() }
        ).unwrap();
        on_connect_to_mount(curs, tui, &mount, *rb_group2.selection(), &connection_param);
}))
    .button("Cancel", make_closure(tui, |curs, tui| close_dialog(curs, tui)))
    .title("Connect to mount")
    .wrap_with(CircularFocus::new)
    .wrap_tab()
}

fn on_connect_to_mount(
    curs: &mut cursive::Cursive,
    tui: &Rc<RefCell<Option<TuiData>>>,
    mount: &Rc<RefCell<Option<mount::MountWrapper>>>,
    mount_type: MountType,
    connection_param: &str
) {
    let result = match mount_type {
        MountType::Simulator => mount::Simulator::new(connection_param),
        MountType::Ioptron => mount::Ioptron::new(connection_param)
    };

    match result {
        Ok(m) => {
            log::info!("connected to {}", m.get_info());
            tui!(tui).text_content.mount_name.set_content(m.get_info());
            *mount.borrow_mut() = Some(mount::MountWrapper::new(m));
            close_dialog(curs, tui);
        },
        Err(e) => {
            log::error!("error connecting to mount at \"{}\": {}", connection_param, e);
            msg_box(curs, &format!("Failed to connect to mount: {}.", e), "Error");
        }
    }
}
