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
    data_receiver,
    tui::{
        close_dialog,
        make_closure,
        make_closure2,
        make_closure3,
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
    connection: &data_receiver::Connection
) -> impl View {
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
            msg_box(curs, &format!("Failed to connect to \"{}\":\n{}.", server_addr, e), "Error");
        }
    }
}
