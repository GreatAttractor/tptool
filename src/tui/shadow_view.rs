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

use cursive::{direction::Orientation, theme, view::*, views::*};

/// As of cursive 0.20.0, it is unclear how to selectively turn on shadows (`ShadowView` + themes with toggled shadows
/// do not work sastisfactorily). For now use a shadow-adding wrapper with custom `draw`.
pub struct WithShadow<V: View> {
    view: V
}

impl<V: View> WithShadow<V> {
    pub fn new(view: V) -> WithShadow<V> {
        WithShadow{ view }
    }
}

impl<V: View> View for WithShadow<V> {
    fn draw(&self, printer: &cursive::Printer) {
        // draw shadow
        let style = theme::ColorStyle::new(
            theme::Color::Rgb(0, 0, 0),
            theme::Color::Rgb(0, 0, 0)
        );

        for y in 1..printer.size.y {
            printer.with_color(style, |printer| printer.print_line(
                cursive::direction::Orientation::Horizontal,
                (1, y),
                printer.size.x - 1,
                " "
            ));
        }

        // draw view background
        let style = theme::ColorStyle::new(
            printer.theme.palette[theme::PaletteColor::Primary],
            printer.theme.palette[theme::PaletteColor::View]
        );

        for y in 0..printer.size.y - 1 {
            printer.with_color(style, |printer| printer.print_line(
                cursive::direction::Orientation::Horizontal,
                (0, y),
                printer.size.x - 1,
                " "
            ));
        }

        // draw view
        let mut printer_int = printer.clone();
        printer_int.size.x -= 1;
        printer_int.size.y -= 1;
        printer_int.with_style(theme::PaletteStyle::View, |printer| {
            self.view.draw(printer);
        });
    }

    fn layout(&mut self, xy: cursive::Vec2) {
        let internal = cursive::Vec2{ x: xy.x - 1, y: xy.y - 1 };
        self.view.layout(internal);
    }

    fn needs_relayout(&self) -> bool {
        self.view.needs_relayout()
    }

    fn focus_view(&mut self, sel: &Selector) -> Result<cursive::event::EventResult, ViewNotFound> {
        self.view.focus_view(sel)
    }

    fn take_focus(
        &mut self,
        source: cursive::direction::Direction
    ) -> Result<cursive::event::EventResult, CannotFocus> {
        self.view.take_focus(source)
    }

    fn important_area(&self, view_size: cursive::Vec2) -> cursive::Rect {
        self.view.important_area(view_size)
    }

    fn required_size(&mut self, constraint: cursive::Vec2) -> cursive::Vec2 {
        let internal = self.view.required_size(constraint);
        cursive::Vec2{ x: internal.x + 1, y: internal.y + 1 }
    }

    fn on_event(&mut self, event: cursive::event::Event) -> cursive::event::EventResult {
        self.view.on_event(event)
    }

    fn call_on_any<'a>(
        &mut self,
        sel: &Selector<'_>,
        callback: &'a mut (dyn FnMut(&mut (dyn View + 'static)) + 'a)
    ) {
        self.view.call_on_any(sel, callback);
    }

    fn type_name(&self) -> &'static str {
        "WithShadow"
    }
}
