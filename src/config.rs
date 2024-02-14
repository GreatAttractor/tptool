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

use configparser::ini::Ini;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "tptool.cfg";

mod sections {
    pub const MAIN: &str = "Main";
}

mod keys {
    pub const MOUNT_TYPE: &str = "MountType";
    pub const MOUNT_SIM_ADDRESS: &str = "MountSimulatorAddr";
    pub const MOUNT_IOPTRON_DEVICE: &str = "MountIoptronDevice";
    pub const DATA_SOURCE_ADDRESS: &str = "DataSourceAddr";
}

pub struct Configuration {
    config_file: Ini
}

impl Configuration {
    pub fn store(&self) -> Result<(), std::io::Error> {
        self.config_file.write(config_file_path())
    }

    pub fn new() -> Configuration {
        let mut config_file = Ini::new_cs();
        let file_path = config_file_path();
        if config_file.load(file_path.clone()).is_err() {
            log::info!(
                "could not load configuration from {}; a new configuration file will be created",
                file_path.to_string_lossy()
            );
        }

        Configuration{ config_file }
    }

    fn get_string(&self, section: &str, key: &str) -> Option<String> {
        self.config_file.get(section, key)
    }

    fn set_string(&mut self, section: &str, key: &str, value: &str) {
        self.config_file.set(section, key, Some(value.into()));
    }

    pub fn mount_simulator_addr(&self) -> Option<String> {
        self.get_string(sections::MAIN, keys::MOUNT_SIM_ADDRESS)
    }

    pub fn set_mount_simulator_addr(&mut self, value: &str) {
        self.set_string(sections::MAIN, keys::MOUNT_SIM_ADDRESS, value);
    }

    pub fn mount_ioptron_device(&self) -> Option<String> {
        self.get_string(sections::MAIN, keys::MOUNT_IOPTRON_DEVICE)
    }

    pub fn set_mount_ioptron_device(&mut self, value: &str) {
        self.set_string(sections::MAIN, keys::MOUNT_IOPTRON_DEVICE, value);
    }

    pub fn data_source_addr(&self) -> Option<String> {
        self.get_string(sections::MAIN, keys::DATA_SOURCE_ADDRESS)
    }

    pub fn set_data_source_addr(&mut self, value: &str) {
        self.set_string(sections::MAIN, keys::DATA_SOURCE_ADDRESS, value);
    }
}

impl Drop for Configuration {
    fn drop(&mut self) {
        if let Err(e) = self.store() {
            log::error!("error saving configuration: {}", e.to_string());
        }
    }
}

fn config_file_path() -> PathBuf {
    Path::new(&dirs::config_dir().or(Some(Path::new("").to_path_buf())).unwrap()).join(CONFIG_FILE_NAME)
}
