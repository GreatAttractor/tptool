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
    config::Configuration,
    mount,
    tracking::TrackingController,
    tui,
    tui::{close_dialog, get_edit_view_str, msg_box, names, set_edit_view_str, TuiData},
    upgrade
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
use std::{cell::RefCell, rc::{Rc, Weak}};

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
    tui: Weak<RefCell<Option<TuiData>>>,
    mount: Weak<RefCell<Option<mount::MountWrapper>>>,
    config: Weak<RefCell<Configuration>>,
    tracking: TrackingController
) -> impl View {
    let param_descr_content = TextContent::new(MountType::Simulator.connection_param_descr());
    let param_descr = TextView::new_with_content(param_descr_content.clone());

    let mut rb_group = RadioGroup::new()
        .on_change(cclone!([config], move |curs, mount_type: &MountType| {
            upgrade!(config);
            param_descr_content.set_content(mount_type.connection_param_descr());
            let prev_value = match mount_type {
                MountType::Simulator => config.borrow().mount_simulator_addr(),
                MountType::Ioptron => config.borrow().mount_ioptron_device()
            }.unwrap_or("".into());
            set_edit_view_str(curs, names::MOUNT_CONNECTION, prev_value);
        }));
    let rb_group2 = rb_group.clone();

    Dialog::around(
        LinearLayout::vertical()
            .child(rb_group.button(MountType::Simulator, "Simulator").selected())
            .child(rb_group.button(MountType::Ioptron, "iOptron"))
            .child(DummyView{})
            .child(param_descr)
            .child(EditView::new()
                .content(config.upgrade().unwrap().borrow().mount_simulator_addr().unwrap_or("".into()))
                .on_submit(cclone!([tui, mount, config, tracking], move |curs, s| {
                    upgrade!(tui, mount, config);
                    on_connect_to_mount(curs, &tui, &mount, &config, *rb_group.selection(), s, tracking.clone());
                }))
                .with_name(names::MOUNT_CONNECTION)
                .fixed_width(20)
            )
    )
    .button("OK", cclone!([tui, mount, config, tracking], move |curs| {
        upgrade!(tui, mount, config);
        let connection_param = get_edit_view_str(curs, names::MOUNT_CONNECTION);
        on_connect_to_mount(curs, &tui, &mount, &config, *rb_group2.selection(), &connection_param, tracking.clone());
    }))

    .button("Cancel",crate::cclone!([tui], move |curs| { upgrade!(tui); close_dialog(curs, &tui); }))
    .title("Connect to mount")
    .wrap_with(CircularFocus::new)
    .wrap_tab()
}

fn on_connect_to_mount(
    curs: &mut cursive::Cursive,
    tui: &Rc<RefCell<Option<TuiData>>>,
    mount: &Rc<RefCell<Option<mount::MountWrapper>>>,
    config: &Rc<RefCell<Configuration>>,
    mount_type: MountType,
    connection_param: &str,
    tracking: TrackingController
) {
    let result = match mount_type {
        MountType::Simulator => mount::Simulator::new(connection_param),
        MountType::Ioptron => mount::Ioptron::new(connection_param)
    };

    match result {
        Ok(m) => {
            log::info!("connected to {}", m.get_info());
            tui!(tui).text_content.mount_name.set_content(m.get_info());
            let mut wrapper = mount::MountWrapper::new(m);
            wrapper.set_on_max_travel_exceeded(Box::new(cclone!(
                [tracking],
                move |mount, axis1, axis2| crate::event_handling::on_max_travel_exceeded(
                    mount, axis1, axis2, tracking.clone()
                )
            )));
            *mount.borrow_mut() = Some(wrapper);
            match mount_type {
                MountType::Simulator => config.borrow_mut().set_mount_simulator_addr(connection_param),
                MountType::Ioptron => config.borrow_mut().set_mount_ioptron_device(connection_param)
            }
            close_dialog(curs, tui);
        },
        Err(e) => {
            log::error!("error connecting to mount at \"{}\": {}", connection_param, e);
            msg_box(curs, &format!("Failed to connect to mount: {}.", e), "Error");
        }
    }
}
