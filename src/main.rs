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

mod cursive_stepper;
mod data;
mod event_handling;
mod mount;
mod tui;

use data::AsyncLinesWrapper;
use async_std::io::prelude::BufReadExt;

const MOUNT_SERVER_PORT: u16 = 45501;
const DATA_SOURCE_PORT: u16 = 45500;
const TIMER_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);

fn main() {
    std::panic::set_hook(Box::new(|_| {
        let backtrace = std::backtrace::Backtrace::force_capture();
        log::error!("panicked!\n\n{}", backtrace);
    }));

    let tz_offset = chrono::Local::now().offset().clone();
    let logfile = std::path::Path::new("tptool.log");
    println!("Logging to: {}", logfile.to_string_lossy());
    simplelog::WriteLogger::init(
        simplelog::LevelFilter::Debug,
        simplelog::ConfigBuilder::new()
            .set_target_level(simplelog::LevelFilter::Error)
            .set_time_offset(time::UtcOffset::from_whole_seconds(tz_offset.local_minus_utc()).unwrap())
            .set_time_format_custom(simplelog::format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6]"
            ))
            .build(),
        std::fs::File::create(logfile).unwrap()
    ).unwrap();

	let curs = cursive::default();

    log::info!("Connecting to data source...");

    let data_receiver = AsyncLinesWrapper::new(
        async_std::io::BufReader::new(
            futures::executor::block_on(async { async_std::net::TcpStream::connect(format!("127.0.0.1:{}", DATA_SOURCE_PORT)).await }).unwrap()
        ).lines()
    );

    let mut state = data::ProgramState{
        timer: Box::pin(pasts::Past::new((), |()| async_std::task::sleep(TIMER_INTERVAL))),
        cursive_stepper: cursive_stepper::CursiveRunnableStepper { curs: curs.into_runner() },
        tui: None,
        listener: stick::Listener::default(),
        controllers: vec![],
        data_receiver,
        mount: Box::new(mount::Simulator::new(&format!("127.0.0.1:{}", MOUNT_SERVER_PORT)).unwrap()),
        slewing: Default::default()
    };

    tui::init(&mut state);

    pasts::block_on(event_handling::event_loop(state));
}
