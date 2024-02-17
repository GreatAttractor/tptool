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

use crate::{cclone, mount::Mount, tui, tui::WithShadow, upgrade};
use cursive::{
    align::HAlign,
    Cursive,
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
        TextContent,
        TextView,
        ThemedView
    },
};
use std::rc::Rc;

pub fn show<F: Fn(&mut Cursive, &str) + 'static>(
    curs: &mut cursive::Cursive,
    title: &str,
    text: &str,
    text_width: usize,
    on_accept: Rc<F>
) {
    let dialog_theme = tui::create_dialog_theme(curs);
    curs.screen_mut().add_transparent_layer_at(
        Position::new(Offset::Center, Offset::Center),
        WithShadow::new(ThemedView::new(
            dialog_theme,
            Dialog::around(
                LinearLayout::horizontal()
                    .child(TextView::new(text))
                    .child(EditView::new()
                        .on_submit(cclone!([on_accept], move |curs, value| {
                            on_accept(curs, value);
                            curs.pop_layer();
                        }))
                        .with_name(tui::names::SIMPLE_DIALOG_TEXT)
                        .fixed_width(text_width)
                    )
            )
            .title(title)
            .button("OK", move |curs| {
                let value = &tui::get_edit_view_str(curs, tui::names::SIMPLE_DIALOG_TEXT);
                on_accept(curs, value);
                curs.pop_layer();
            })
            .dismiss_button("Cancel")
        ))
    );
}
