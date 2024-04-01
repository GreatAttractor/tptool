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

use crate::{cclone, tui::{close_dialog, TuiData}, upgrade};
use cursive::{
    event,
    View,
    views::{Dialog, LinearLayout, OnEventView, TextView},
    With
};
use std::{cell::RefCell, rc::Weak};

pub fn dialog(tui: Weak<RefCell<Option<TuiData>>>) -> impl View {
    Dialog::around(
        LinearLayout::vertical()
            .child(TextView::new(format!(
                "TPTool\n\n\
                Copyright © Filip Szczerek 2024 (ga.software@yahoo.com)\n\n\
                This program comes with ABSOLUTELY NO WARRANTY.\n\
                This is free software, licensed under GNU General Public License v3\n\
                and you are welcome to redistribute it under certain conditions.\n\
                See the LICENSE file for details.\n\n\
                version: {}\n\
                OS: {}",
                crate::VERSION_STRING,
                os_info::get()
            )))
    )
    .button("OK", crate::cclone!([tui], move |curs| { upgrade!(tui); close_dialog(curs, &tui); }))
    .title("About TPTool")
    .wrap_with(OnEventView::new)
    .on_event(event::Event::Key(event::Key::Esc), crate::cclone!([tui],
        move |curs| { upgrade!(tui); close_dialog(curs, &tui); }
    ))
}
