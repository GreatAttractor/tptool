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

fn main() {
    let output_dir = std::env::var("OUT_DIR").unwrap();
    let version_path = std::path::Path::new(&output_dir).join("version");

    let version_str = format!("{}", get_commit_hash());

    std::fs::write(version_path, version_str).unwrap();
}

fn get_commit_hash() -> String {
    let output = std::process::Command::new("git")
        .arg("log").arg("-1")
        .arg("--pretty=format:%h")
        .arg("--abbrev=8")
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .unwrap();

    if output.status.success() {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        "unspecified".to_string()
    }
}
