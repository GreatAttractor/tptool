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

use crate::{data, data::{as_deg, deg}};
use configparser::ini::Ini;
use std::path::{Path, PathBuf};

const CONFIG_FILE_NAME: &str = "tptool.cfg";

mod sections {
    pub const MAIN: &str = "Main";
    pub const REF_POS_PRESETS: &str = "ReferencePositionPresets";
}

mod keys {
    pub const MOUNT_TYPE: &str = "MountType";
    pub const MOUNT_SIM_ADDRESS: &str = "MountSimulatorAddr";
    pub const MOUNT_IOPTRON_DEVICE: &str = "MountIoptronDevice";
    pub const DATA_SOURCE_ADDRESS: &str = "DataSourceAddr";
    pub const REF_POS_PRESET: &str = "preset";
}

const MAX_NUM_REF_POS_PRESETS: usize = 128;

pub struct Configuration {
    config_file: Ini
}

impl Configuration {
    pub fn store(&self) -> Result<(), std::io::Error> {
        self.config_file.write(config_file_path())
    }

    pub fn new() -> Configuration {
        let mut config_file = Ini::new_cs();
        config_file.set_comment_symbols(&['#']);
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

    pub fn ref_pos_presets(&self) -> Vec<data::RefPositionPreset> {
        let mut result = vec![];
        let presets = match self.config_file.get_map_ref().get(sections::REF_POS_PRESETS) {
            Some(p) => p,
            None => return result
        };

        let mut idx = 1;
        loop {
            match presets.get(&format!("{}{}", keys::REF_POS_PRESET, idx)) {
                Some(preset) => match preset.as_ref().unwrap().parse::<data::RefPositionPreset>() {
                    Ok(preset) => result.push(preset),
                    Err(e) => log::error!("invalid ref. position preset: {}", e)
                },

                None => break
            }
            idx += 1;
            if idx > MAX_NUM_REF_POS_PRESETS {
                log::warn!("too many ref. position presets; ignoring the rest");
                break;
            }
        }
        result
    }

    pub fn add_ref_pos_preset(&mut self, preset: data::RefPositionPreset) {
        let num_existing = if let Some(presets) = self.config_file.get_map_ref().get(sections::REF_POS_PRESETS) {
            presets.len()
        } else {
            0
        };

        self.config_file.set(
            sections::REF_POS_PRESETS,
            &format!("{}{}", keys::REF_POS_PRESET, num_existing + 1),
            Some(preset.to_string())
        );
    }

    // pub fn set_ref_pos_presets(&mut self, presets: &[data::RefPositionPreset]) {
    //     self.config_file.remove_section(sections::REF_POS_PRESETS);
    //     for (idx, preset) in presets.iter().enumerate() {
    //         self.config_file.set(
    //             sections::REF_POS_PRESETS,
    //             &format!("{}{}", keys::REF_POS_PRESET, idx),
    //             Some(format!("{};{};{}", as_deg(preset.azimuth), as_deg(preset.altitude), preset.name))
    //         );
    //     }
    // }
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
