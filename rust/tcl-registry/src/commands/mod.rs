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

//! Command specification modules — one directory per dialect.
//!
//! The EDA vendor libraries are **not** here: `sdc_base` and the five vendor
//! packs ship as bundled `.tclspec` loadables under `specs/`, loaded by
//! `tcl-spectcl` (`docs/design/spec-packs.md`, "the EDA vendor libraries ship
//! as bundled `.tclspec` loadables … so the loader path is exercised in
//! production from day one").

pub mod argparse;
pub mod bpf;
pub mod expect;
pub mod iapps;
pub mod irules;
pub mod itcl;
pub mod spectcl;
pub mod sslictcl;
pub mod stdlib;
pub mod tcl;
pub mod tcllib;
pub mod ticklecharts;
pub mod tk;

use crate::CommandSpec;

/// One **authoring pack**: the `commands/<id>/` module a group of shipped
/// specs is declared in, with the words a registry browser uses to name it.
///
/// This is provenance, not availability.
/// [`SpecSurface`](tcl_dialect::model::SpecSurface) already says *where a
/// command is reachable from*; a pack says *where its spec is written down*.
/// The two answer different questions and legitimately disagree: `open` is
/// surfaced by core Tcl **and** by iRules, but it is authored once, in the
/// `tcl` pack; `wm` is surfaced by the `Tk` package and authored in `tk`.
/// Provenance is what a spec author navigates by, and it is the directory
/// the studio's `.rs` renderer emits a path into
/// (`rust/tcl-registry/src/commands/<id>/<stem>.rs`).
///
/// The EDA vendor libraries are deliberately absent. They ship as bundled
/// `.tclspec` loadables under `specs/` and reach a registry through
/// `tcl_spectcl::bundled`, so their provenance is the pack *file*, which the
/// loader reports — see [`spec-packs.md`](../../../../docs/design/spec-packs.md).
#[derive(Debug, Clone, Copy)]
pub struct SpecPack {
    /// The module directory name, and the id every surface keys on.
    pub id: &'static str,
    /// Author-facing name.
    pub label: &'static str,
    /// One line saying what the pack holds.
    pub blurb: &'static str,
    /// The specs the module declares.
    ///
    /// Held as the builder rather than a slice because a pack's specs are
    /// built lazily and leaked once, by the registry's own `shared_group!`
    /// cells; calling this directly builds a fresh `Vec` and is for callers
    /// that want one (the provenance index does not — see
    /// [`crate::registry::spec_pack_of`]).
    pub specs: fn() -> Vec<CommandSpec>,
}

/// Every authoring pack, in the order a browser lists them: the core
/// language, then the libraries that layer on it, then the vendor surfaces
/// and the authoring DSLs.
///
/// `tmsh` is **not** a row. The tmsh shell's commands are a filtered view of
/// the same `commands/iapps/` sources (`tmsh_command_specs`), so their
/// provenance is `iapps`; a surface that wants the tmsh *surface* asks
/// `SpecSurface`, which is the question it is actually asking.
pub const SPEC_PACKS: &[SpecPack] = &[
    SpecPack {
        id: "tcl",
        label: "Tcl core",
        blurb: "The core interpreter's own commands, across the 8.4-9.1 ladder.",
        specs: tcl::tcl_command_specs,
    },
    SpecPack {
        id: "stdlib",
        label: "Tcl standard library",
        blurb: "The packages shipped with Tcl itself: http, msgcat, platform, registry.",
        specs: stdlib::stdlib_command_specs,
    },
    SpecPack {
        id: "tcllib",
        label: "Tcllib",
        blurb: "The Tcllib collection: base64, cmdline, csv, json, md5, struct and the rest.",
        specs: tcllib::tcllib_command_specs,
    },
    SpecPack {
        id: "argparse",
        label: "argparse",
        blurb: "The argparse option-parsing package.",
        specs: argparse::argparse_command_specs,
    },
    SpecPack {
        id: "ticklecharts",
        label: "ticklecharts",
        blurb: "The ticklecharts ECharts binding.",
        specs: ticklecharts::ticklecharts_command_specs,
    },
    SpecPack {
        id: "itcl",
        label: "[incr Tcl]",
        blurb: "The itcl class system's declaration and instance commands.",
        specs: itcl::itcl_command_specs,
    },
    SpecPack {
        id: "tk",
        label: "Tk",
        blurb: "Tk's widgets, geometry managers, and window-system commands.",
        specs: tk::tk_command_specs,
    },
    SpecPack {
        id: "expect",
        label: "Expect",
        blurb: "The Expect extension's spawn / expect / interact surface.",
        specs: expect::expect_command_specs,
    },
    SpecPack {
        id: "bpf",
        label: "BPF-Tcl",
        blurb: "The BPF-Tcl packet-filter dialect.",
        specs: bpf::bpf_command_specs,
    },
    SpecPack {
        id: "irules",
        label: "F5 iRules",
        blurb: "The F5 BIG-IP iRules events and namespace commands.",
        specs: irules::irules_command_specs,
    },
    SpecPack {
        id: "iapps",
        label: "F5 iApps & tmsh",
        blurb: "The iApps templating surface and the tmsh:: commands it shares with the shell.",
        specs: iapps::iapps_command_specs,
    },
    SpecPack {
        id: "spectcl",
        label: "SpecTcl",
        blurb: "The .tclspec authoring DSL's own declaration words.",
        specs: spectcl::spectcl_command_specs,
    },
    SpecPack {
        id: "sslictcl",
        label: "SslicTcl",
        blurb: "The .sslictcl TLS-assurance authoring DSL's declaration words.",
        specs: sslictcl::sslictcl_command_specs,
    },
];

impl SpecPack {
    /// The pack with this id.
    #[must_use]
    pub fn by_id(id: &str) -> Option<&'static Self> {
        SPEC_PACKS.iter().find(|pack| pack.id == id)
    }
}
