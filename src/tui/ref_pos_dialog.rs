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

use cgmath::Deg;
use crate::{
    cclone,
    config::Configuration,
    data,
    data::{as_deg, deg},
    mount,
    tui,
    tui::{
        close_dialog,
        create_dialog_theme,
        get_edit_view_str,
        get_select_view_idx,
        msg_box,
        names,
        set_edit_view_str,
        TuiData,
        WithShadow
    },
    upgrade
};
use cursive::{
    align::HAlign,
    event,
    view::{Nameable, Resizable, View},
    views::{
        Button,
        CircularFocus,
        Dialog,
        DummyView,
        EditView,
        LinearLayout,
        OnEventView,
        PaddedView,
        Panel,
        SelectView,
        TextContent,
        TextView,
        ThemedView
    },
    With
};
use pointing_utils::{cgmath, GeoPos, LatLon, uom};
use std::{cell::RefCell, rc::{Rc, Weak}};
use uom::si::{f64, length};

pub fn dialog(
    tui: Weak<RefCell<Option<TuiData>>>,
    mount: Weak<RefCell<Option<mount::MountWrapper>>>,
    config: Weak<RefCell<Configuration>>
) -> impl View {
    let preset_name = TextContent::new("(none)");

    Dialog::around(LinearLayout::vertical()
        .child(
            LinearLayout::horizontal()
                .child(TextView::new("Preset:"))
                .child(DummyView{}.min_width(1))
                .child(TextView::new_with_content(preset_name.clone()))
                .child(DummyView{}.min_width(1))
                .child(Button::new("Load", cclone!(
                    [config, preset_name],
                    move |curs| on_load_preset(curs, preset_name.clone(), config.clone())
                )))
                .child(Button::new("Store", cclone!([config, preset_name], move |curs| {
                    on_store_preset(curs, preset_name.clone(), config.clone());
                })))
        )
        .child(DummyView{}.min_height(1))
        .child(Button::new(
            "Calc. from lat., lon. of observer and target",
            |curs| on_calc_from_observer_and_target_pos(curs)
        ))
        .child(DummyView{}.min_height(1))
        .child(
            LinearLayout::horizontal()
                .child(TextView::new("azimuth:  ")) // TODO: ensure alignment of `EditView`s
                .child(tui::styled_edit_view()
                    .with_name(names::REF_POS_AZ)
                    .fixed_width(10)
                )
                .child(TextView::new("°"))
        )
        .child(
            LinearLayout::horizontal()
                .child(TextView::new("altitude: "))
                .child(tui::styled_edit_view()
                    .with_name(names::REF_POS_ALT)
                    .fixed_width(10)
                )
                .child(TextView::new("°"))
        )
    )
    .button("OK", cclone!([tui, mount], move |curs| {
        upgrade!(tui, mount);

        let ref_az = get_edit_view_str(curs, names::REF_POS_AZ);
        let ref_alt = get_edit_view_str(curs, names::REF_POS_ALT);

        let ref_az = (*ref_az).parse::<f64>();
        let ref_alt = (*ref_alt).parse::<f64>();

        let err: Option<_> = match (ref_az, ref_alt) {
            (Ok(ref_az), Ok(ref_alt)) => {
                close_dialog(curs, &tui);
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
    .button("Cancel", crate::cclone!([tui], move |curs| { upgrade!(tui); close_dialog(curs, &tui); }))
    .title("Set current reference position")
    .wrap_with(CircularFocus::new)
    .wrap_tab()
    .wrap_with(OnEventView::new)
    .on_event(event::Event::Key(event::Key::Esc), crate::cclone!([tui],
        move |curs| { upgrade!(tui); close_dialog(curs, &tui); }
    ))
}

fn on_preset_chosen(
    curs: &mut cursive::Cursive,
    preset_name: &TextContent,
    preset_idx: usize,
    config: Weak<RefCell<Configuration>>
) {
    upgrade!(config);
    let preset = &config.borrow().ref_pos_presets()[preset_idx];
    set_edit_view_str(curs, names::REF_POS_AZ, format!("{:.3}", as_deg(preset.azimuth)));
    set_edit_view_str(curs, names::REF_POS_ALT, format!("{:.3}", as_deg(preset.altitude)));
    preset_name.set_content(preset.name.clone());
    curs.pop_layer();
}

fn on_load_preset(curs: &mut cursive::Cursive, preset_name: TextContent, config: Weak<RefCell<Configuration>>) {
    let sel_view = {
        let mut sel_view = SelectView::new().on_submit(
            cclone!([preset_name, config], move |curs, idx| on_preset_chosen(curs, &preset_name, *idx, config.clone()))
        );
        upgrade!(config);
        for (idx, preset) in config.borrow().ref_pos_presets().iter().enumerate() {
            sel_view.add_item(&preset.name, idx);
        }
        sel_view.with_name(names::REF_POS_SEL_PRESET)
    };

    let dt = create_dialog_theme(curs);
    curs.screen_mut().add_transparent_layer(WithShadow::new(ThemedView::new(
        dt,
        Dialog::around(sel_view)
            .title("Choose preset")
            .button("OK", cclone!([preset_name, config], move |curs| {
                let idx = get_select_view_idx(curs, names::REF_POS_SEL_PRESET);
                on_preset_chosen(curs, &preset_name, idx, config.clone());
            }))
            .dismiss_button("Cancel")
            .wrap_with(OnEventView::new)
            .on_event(event::Event::Key(event::Key::Esc), |curs| { curs.pop_layer(); })
    )));
}

fn on_store_preset(curs: &mut cursive::Cursive, preset_name: TextContent, config: Weak<RefCell<Configuration>>) {
    let az = get_edit_view_str(curs, names::REF_POS_AZ).parse::<f64>();
    let alt = get_edit_view_str(curs, names::REF_POS_ALT).parse::<f64>();

    if let (Ok(az), Ok(alt)) = (az, alt) {
        tui::simple_dialog::show(
            curs,
            "Enter preset name",
            "",
            25,
            Rc::new(cclone!([config], move |_: &mut cursive::Cursive, name: &str| {
                upgrade!(config);
                config.borrow_mut().add_ref_pos_preset(
                    data::RefPositionPreset{ azimuth: deg(az), altitude: deg(alt), name: name.into() }
                );
                preset_name.set_content(name);
            }))
        );
    } else {
        msg_box(curs, "Invalid azimuth or altitude value.", "Error");
    }
}

fn on_calc_from_observer_and_target_pos(curs: &mut cursive::Cursive) {
    let dt = create_dialog_theme(curs);
    curs.screen_mut().add_transparent_layer(WithShadow::new(ThemedView::new(
        dt,
        Dialog::around(LinearLayout::vertical()
            .child(Panel::new(PaddedView::lrtb(0, 0, 1, 1,
                LinearLayout::horizontal()
                    .child(TextView::new("lat.: "))
                    .child(tui::styled_edit_view()
                        .with_name(names::OBS_LAT)
                        .fixed_width(10)
                    )
                    .child(TextView::new("°"))
                    .child(DummyView{}.min_width(1))
                    .child(TextView::new("lon.: "))
                    .child(tui::styled_edit_view()
                        .with_name(names::OBS_LON)
                        .fixed_width(10)
                        )
                    .child(TextView::new("°"))
                    .child(DummyView{}.min_width(1))
                    .child(TextView::new("elev.: "))
                    .child(tui::styled_edit_view()
                        .with_name(names::OBS_ELEVATION)
                        .fixed_width(10)
                    )
                    .child(TextView::new(" m"))
            )).title("Observer").title_position(HAlign::Left))
            .child(Panel::new(PaddedView::lrtb(0, 0, 1, 1,
                LinearLayout::horizontal()
                    .child(TextView::new("lat.: "))
                    .child(tui::styled_edit_view()
                        .with_name(names::TARGET_LAT)
                        .fixed_width(10)
                    )
                    .child(TextView::new("°"))
                    .child(DummyView{}.min_width(1))
                    .child(TextView::new("lon.: "))
                    .child(tui::styled_edit_view()
                        .with_name(names::TARGET_LON)
                        .fixed_width(10)
                    )
                    .child(TextView::new("°"))
                    .child(DummyView{}.min_width(1))
                    .child(TextView::new("elev.: "))
                    .child(tui::styled_edit_view()
                        .with_name(names::TARGET_ELEVATION)
                        .fixed_width(10)
                    )
                    .child(TextView::new(" m"))
            )).title("Target").title_position(HAlign::Left))
        )
        .title("Calculate mount position")
        .button("OK", |curs| {
            let obs_lat_str = get_edit_view_str(curs, names::OBS_LAT);
            let obs_lon_str = get_edit_view_str(curs, names::OBS_LON);
            let obs_el_str = get_edit_view_str(curs, names::OBS_ELEVATION);

            let target_lat_str = get_edit_view_str(curs, names::TARGET_LAT);
            let target_lon_str = get_edit_view_str(curs, names::TARGET_LON);
            let target_el_str = get_edit_view_str(curs, names::TARGET_ELEVATION);

            if let Err(e) = || -> Result<(), String> {
                let parse = |s: Rc<String>| (*s).parse::<f64>().map_err(|_| format!("invalid value: {}", *s));

                let obs_lat = parse(obs_lat_str)?;
                let obs_lon = parse(obs_lon_str)?;
                let obs_el = parse(obs_el_str)?;

                let target_lat = parse(target_lat_str)?;
                let target_lon = parse(target_lon_str)?;
                let target_el = parse(target_el_str)?;

                let (az, alt) = data::calc_az_alt_between_points(
                    &GeoPos{
                        lat_lon: LatLon{ lat: Deg(obs_lat), lon: Deg(obs_lon) },
                        elevation: f64::Length::new::<length::meter>(obs_el)
                    },
                    &GeoPos{
                        lat_lon: LatLon{ lat: Deg(target_lat), lon: Deg(target_lon) },
                        elevation: f64::Length::new::<length::meter>(target_el)
                    }
                );

                set_edit_view_str(curs, names::REF_POS_AZ, format!("{:.04}", as_deg(az)));
                set_edit_view_str(curs, names::REF_POS_ALT, format!("{:.04}", as_deg(alt)));

                curs.pop_layer();

                Ok(())
            }() {
                msg_box(curs, &format!("Error calculating position: {}.", e), "Error");
            }
        })
        .dismiss_button("Cancel")
        .wrap_with(OnEventView::new)
        .on_event(event::Event::Key(event::Key::Esc), |curs| { curs.pop_layer(); })
    )));
}
