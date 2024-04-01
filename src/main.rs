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

mod config;
mod controller;
mod cursive_stepper;
mod data;
mod data_receiver;
mod event_handling;
mod mount;
mod tracking;
mod tui;

use event_handling::on_tracking_state_changed;
use std::{cell::RefCell, future::Future, rc::Rc};

const MAIN_TIMER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
const TARGET_LOG_TIMER_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

pub const VERSION_STRING: &'static str = include_str!(concat!(env!("OUT_DIR"), "/version"));

fn main() {
    set_up_logging();

    #[cfg(target_os = "windows")]
    unsafe { pdcurses::resize_term(20, 105) };

	let curs = cursive::default();
    let data_receiver = data_receiver::DataReceiver::new();
    let mut listener = stick::Listener::default();
    let mount = Rc::new(RefCell::new(None));
    let mount_spd = Rc::new(RefCell::new(data::MountSpeed::new()));
    let target = Rc::new(RefCell::new(None));
    let tui = Rc::new(RefCell::new(None));
    let config = Rc::new(RefCell::new(config::Configuration::new()));
    let ctrl_actions = config.borrow().controller_actions();

    let mut state = data::ProgramState{
        config,
        controllers: vec![],
        controller_names: vec![],
        cursive_stepper: cursive_stepper::CursiveRunnableStepper{ curs: curs.into_runner() },
        data_receiver,
        listener: Box::pin(pasts::notify::poll_fn(move |ctx| std::pin::Pin::new(&mut listener).poll(ctx))),
        mount: mount.clone(),
        mount_spd: mount_spd.clone(),
        slewing: Default::default(),
        slew_speed: Rc::new(RefCell::new(data::deg_per_s(5.0))),
        target: Rc::clone(&target),
        timers: vec![
            data::Timer::new(data::timers::MAIN, MAIN_TIMER_INTERVAL),
            data::Timer::new(data::timers::TARGET_LOG, TARGET_LOG_TIMER_INTERVAL)
        ],
        tracking: tracking::Tracking::new(
            data::deg_per_s(5.0),
            mount,
            mount_spd,
            target,
            Box::new(cclone!([@weak tui], move |running| on_tracking_state_changed(running, tui.clone())))
        ),
        tui,
        refresher: tui::Refresher::new(),
        ctrl_actions
    };

    tui::init(&mut state);

    pasts::Executor::default().block_on(event_handling::event_loop(state));
}

fn set_up_logging() {
    std::panic::set_hook(Box::new(|info| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("{}\n\n{}", info, backtrace);
    }));

    let tz_offset = chrono::Local::now().offset().clone();
    let logfile = dirs::data_dir().unwrap_or(std::path::Path::new("").to_path_buf())
        .join(format!("tptool_{}.log", chrono::Local::now().format("%Y-%m-%d_%H%M%S")));

    println!("Logging to: {}", logfile.to_string_lossy());
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Info,
        simplelog::ConfigBuilder::new()
            .set_target_level(simplelog::LevelFilter::Error)
            .set_time_offset(time::UtcOffset::from_whole_seconds(tz_offset.local_minus_utc()).unwrap())
            .set_time_format_custom(simplelog::format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"
            ))
            .add_filter_ignore_str("cursive_core")
            .build(),
        std::fs::File::create(logfile).unwrap()
    ).unwrap();
}
