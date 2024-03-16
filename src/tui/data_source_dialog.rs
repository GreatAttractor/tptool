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
    data_receiver,
    tui::{
        close_dialog,
        get_edit_view_str,
        msg_box,
        names,
        TuiData
    },
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
    connection: data_receiver::Connection,
    config: Weak<RefCell<Configuration>>
) -> impl View {
    Dialog::around(
        LinearLayout::horizontal()
            .child(TextView::new("Server address:"))
            .child(EditView::new()
                .content(config.upgrade().unwrap().borrow().data_source_addr().unwrap_or("".into()))
                .on_submit(cclone!([tui, connection, config], move |curs, s| {
                    upgrade!(tui, config);
                    on_connect_to_data_source(curs, &tui, connection.clone(), &config, s);
                }))
                .with_name(names::SERVER_ADDR)
                .fixed_width(20)
        )
    )
    .button("OK", cclone!([tui, connection, config], move |curs| {
        upgrade!(tui, config);
        let server_address = get_edit_view_str(curs, names::SERVER_ADDR);
        on_connect_to_data_source(curs, &tui, connection.clone(), &config, &server_address);
    }))
    .button("Cancel", cclone!([tui], move |curs| { upgrade!(tui); close_dialog(curs, &tui); }))
    .title("Connect to data source")
    .wrap_with(CircularFocus::new)
    .wrap_tab()
    .wrap_with(OnEventView::new)
    .on_event(event::Event::Key(event::Key::Esc), crate::cclone!([tui],
        move |curs| { upgrade!(tui); close_dialog(curs, &tui); }
    ))
}

fn on_connect_to_data_source(
    curs: &mut cursive::Cursive,
    tui: &Rc<RefCell<Option<TuiData>>>,
    connection: data_receiver::Connection,
    config: &Rc<RefCell<Configuration>>,
    server_addr: &str
) {
    match connection.connect(server_addr) {
        Ok(()) => {
            log::info!("connected to data source {}", server_addr);
            config.borrow_mut().set_data_source_addr(server_addr);
            close_dialog(curs, tui);
        },

        Err(e) => {
            log::error!("error connecting to data source \"{}\": {}", server_addr, e);
            msg_box(curs, &format!("Failed to connect to \"{}\":\n{}.", server_addr, e), "Error");
        }
    }
}
