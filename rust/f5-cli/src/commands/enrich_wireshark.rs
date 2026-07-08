// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The `enrich-wireshark` (`ws-profile`) verb — emit a Wireshark profile dir.
//!
//! Builds a [`tcl_bigip::wireshark_profile::WiresharkProfile`] from one or more
//! configs and writes the `hosts` / `subnets` / `vlans` / `dfilters` /
//! `services` / `ethers` / `colorfilters` / `preferences` / `README.md` files
//! into the output directory.

use std::path::{Path, PathBuf};

use tcl_bigip::wireshark_profile::build_wireshark_profile;

use super::enrich_pcapng::load_configs;

/// `f5 enrich-wireshark`.
// Returns `Result` to match the uniform `run_*` command-dispatch signature;
// handlers print their own errors and resolve to an exit code.
#[allow(clippy::unnecessary_wraps)]
pub fn run_enrich_wireshark(configs: &[PathBuf], output: &Path, force: bool) -> anyhow::Result<u8> {
    let configs_with_sources = match load_configs(configs) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return Ok(2);
        }
    };

    if output.exists() && !output.is_dir() {
        eprintln!("error: not a directory: {}", output.display());
        return Ok(2);
    }

    if output.exists() && !force {
        let non_empty = std::fs::read_dir(output).is_ok_and(|mut it| it.next().is_some());
        if non_empty {
            eprintln!(
                "error: {} already exists and is not empty; pass --force to overwrite",
                output.display()
            );
            return Ok(2);
        }
    }

    let profile = build_wireshark_profile(&configs_with_sources);

    let profile_name = output
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let files = profile.render_files(&profile_name);

    if let Err(e) = std::fs::create_dir_all(output) {
        eprintln!("error: cannot write profile: {e}");
        return Ok(2);
    }
    for (name, content) in &files {
        if let Err(e) = std::fs::write(output.join(name), content) {
            eprintln!("error: cannot write profile: {e}");
            return Ok(2);
        }
    }

    eprintln!(
        "enrich-wireshark: {} host(s), {} subnet(s), {} vlan(s), {} display-filter(s), \
         {} service(s), {} ether(s), {} colour rule(s) written to {}",
        profile.hosts.len(),
        profile.subnets.len(),
        profile.vlans.len(),
        profile.dfilters.len(),
        profile.services.len(),
        profile.ethers.len(),
        profile.colorfilters.len(),
        output.display(),
    );
    Ok(0)
}
