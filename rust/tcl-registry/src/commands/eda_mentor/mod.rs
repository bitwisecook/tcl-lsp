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

//! `eda_mentor` command specifications.

mod add_list;
mod add_log;
mod add_wave;
mod bc;
mod bd;
mod be;
mod bl;
mod bp;
mod calibre;
mod calibre_drc;
mod calibre_lvs;
mod calibre_pex;
mod change;
mod coverage;
mod describe;
mod drivers;
mod examine;
mod find;
mod force;
mod formal_analyze;
mod formal_compile;
mod formal_verify;
mod init_signal_driver;
mod init_signal_spy;
mod onbreak;
mod qrun;
mod qverilog;
mod qvhdl;
mod qwave;
mod readers;
mod release;
mod restart;
mod resume;
mod run;
mod signal_force;
mod signal_release;
mod toggle;
mod transcript;
mod vcom;
mod vcover;
mod vdel;
mod virtual_;
mod vlib;
mod vlog;
mod vmap;
mod vopt;
mod vsim;
mod wave;
mod when;

use crate::spec::CommandSpec;

/// The per-tool Mentor/Siemens package a command belongs to (design doc
/// `eda-library-packages.md`): Questa / `ModelSim` simulation is the bulk default;
/// Questa Formal and Calibre (DRC/LVS batch launch) are their own packages.
fn mentor_package_for(name: &str) -> &'static str {
    match name {
        "formal_analyze" | "formal_compile" | "formal_verify" => "questa-formal",
        "calibre" | "calibre_drc" | "calibre_lvs" | "calibre_pex" => "calibre",
        _ => "questa",
    }
}

/// Return all `eda_mentor` command specifications.
#[must_use]
pub fn eda_mentor_command_specs() -> Vec<CommandSpec> {
    let mut specs = vec![
        add_list::spec(),
        add_log::spec(),
        add_wave::spec(),
        bc::spec(),
        bd::spec(),
        be::spec(),
        bl::spec(),
        bp::spec(),
        calibre::spec(),
        calibre_drc::spec(),
        calibre_lvs::spec(),
        calibre_pex::spec(),
        change::spec(),
        coverage::spec(),
        describe::spec(),
        drivers::spec(),
        examine::spec(),
        find::spec(),
        force::spec(),
        formal_analyze::spec(),
        formal_compile::spec(),
        formal_verify::spec(),
        init_signal_driver::spec(),
        init_signal_spy::spec(),
        onbreak::spec(),
        qrun::spec(),
        qverilog::spec(),
        qvhdl::spec(),
        qwave::spec(),
        readers::spec(),
        release::spec(),
        restart::spec(),
        resume::spec(),
        run::spec(),
        signal_force::spec(),
        signal_release::spec(),
        toggle::spec(),
        transcript::spec(),
        vcom::spec(),
        vcover::spec(),
        vdel::spec(),
        virtual_::spec(),
        vlib::spec(),
        vlog::spec(),
        vmap::spec(),
        vopt::spec(),
        vsim::spec(),
        wave::spec(),
        when::spec(),
    ];
    for spec in &mut specs {
        spec.required_package = Some(mentor_package_for(spec.name));
    }
    specs
}
