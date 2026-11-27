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
//! `tcl-spectcl` (`docs/design/registry/spec-packs.md`, "the EDA vendor libraries ship
//! as bundled `.tclspec` loadables … so the loader path is exercised in
//! production from day one").

pub mod argparse;
pub mod bpf;
mod closed_evidence;
pub mod expect;
pub mod iapps;
pub mod irules;
pub mod itcl;
pub mod spectcl;
pub mod sslictcl;
pub mod stdlib;
pub mod tcl;
pub mod tcllib;
pub mod thread;
pub mod ticklecharts;
pub mod tk;

use crate::registry::surfaces::{HookTable, Placement, SpecCell, SpecSource, Surface, SurfaceView};
use tcl_dialect::model::{Family, SurfaceLayer};

/// Every shipped surface, in the order a browser lists them and provenance
/// answers: the core language, the libraries that layer on it, then the
/// vendor surfaces and the authoring DSLs.
///
/// An entry point installs this list ([`crate::install_shipped`]); the core
/// reads only what was installed. The EDA vendor libraries are absent: they
/// ship as bundled `.tclspec` loadables and reach a registry through
/// `tcl_spectcl::bundled` ([`spec-packs.md`](../../../../docs/design/registry/spec-packs.md)).
/// `tmsh` is a view of `iapps`, so its provenance is `iapps`.
pub static SHIPPED: &[&Surface] = &[
    &TCL,
    &STDLIB,
    &TCLLIB,
    &ARGPARSE,
    &TICKLECHARTS,
    &ITCL,
    &THREAD,
    &TK,
    &EXPECT,
    &BPF,
    &IRULES,
    &IAPPS,
    &SPECTCL,
    &SSLICTCL,
];

static TCL: Surface = Surface {
    id: "tcl",
    label: "Tcl core",
    blurb: "The core interpreter's own commands, across the 8.4-9.1 ladder.",
    source_dir: "rust/tcl-registry/src/commands/tcl",
    specs: SpecSource::Built(tcl::tcl_command_specs),
    placement: Placement::Base,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static STDLIB: Surface = Surface {
    id: "stdlib",
    label: "Tcl standard library",
    blurb: "The packages shipped with Tcl itself: http, msgcat, platform, registry.",
    source_dir: "rust/tcl-registry/src/commands/stdlib",
    specs: SpecSource::Built(stdlib::stdlib_command_specs),
    placement: Placement::Base,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static TCLLIB: Surface = Surface {
    id: "tcllib",
    label: "Tcllib",
    blurb: "The Tcllib collection: base64, cmdline, csv, json, md5, struct and the rest.",
    source_dir: "rust/tcl-registry/src/commands/tcllib",
    specs: SpecSource::Built(tcllib::tcllib_command_specs),
    placement: Placement::Base,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static ARGPARSE: Surface = Surface {
    id: "argparse",
    label: "argparse",
    blurb: "The argparse option-parsing package.",
    source_dir: "rust/tcl-registry/src/commands/argparse",
    specs: SpecSource::Built(argparse::argparse_command_specs),
    placement: Placement::Base,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static TICKLECHARTS: Surface = Surface {
    id: "ticklecharts",
    label: "ticklecharts",
    blurb: "The ticklecharts ECharts binding.",
    source_dir: "rust/tcl-registry/src/commands/ticklecharts",
    specs: SpecSource::Built(ticklecharts::ticklecharts_command_specs),
    placement: Placement::Base,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static ITCL: Surface = Surface {
    id: "itcl",
    label: "[incr Tcl]",
    blurb: "The itcl class system's declaration and instance commands.",
    source_dir: "rust/tcl-registry/src/commands/itcl",
    specs: SpecSource::Built(itcl::itcl_command_specs),
    placement: Placement::Base,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

// Outside the W002 universe today, pending the owner's decision (design
// open question 1).
static THREAD: Surface = Surface {
    id: "thread",
    label: "Thread",
    blurb: "Thread script-entry and shared-store contracts.",
    source_dir: "rust/tcl-registry/src/commands/thread",
    specs: SpecSource::Built(thread::thread_command_specs),
    placement: Placement::Base,
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: false,
    on_install: None,
    cell: SpecCell::new(),
};

static TK: Surface = Surface {
    id: "tk",
    label: "Tk",
    blurb: "Tk's widgets, geometry managers, and window-system commands.",
    source_dir: "rust/tcl-registry/src/commands/tk",
    specs: SpecSource::Built(tk::tk_command_specs),
    placement: Placement::BaseLayer(SurfaceLayer::Package("Tk")),
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static EXPECT: Surface = Surface {
    id: "expect",
    label: "Expect",
    blurb: "The Expect extension's spawn / expect / interact surface.",
    source_dir: "rust/tcl-registry/src/commands/expect",
    specs: SpecSource::Built(expect::expect_command_specs),
    placement: Placement::Layer(SurfaceLayer::Package("expect")),
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static BPF: Surface = Surface {
    id: "bpf",
    label: "BPF-Tcl",
    blurb: "The BPF-Tcl packet-filter dialect.",
    source_dir: "rust/tcl-registry/src/commands/bpf",
    specs: SpecSource::Built(bpf::bpf_command_specs),
    placement: Placement::Layer(SurfaceLayer::Package("bpf")),
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static IRULES: Surface = Surface {
    id: "irules",
    label: "F5 iRules",
    blurb: "The F5 BIG-IP iRules events and namespace commands.",
    source_dir: "rust/tcl-registry/src/commands/irules",
    specs: SpecSource::Built(irules::irules_command_specs),
    placement: Placement::Layer(SurfaceLayer::Core(Family::F5Irules, "")),
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

static IAPPS: Surface = Surface {
    id: "iapps",
    label: "F5 iApps & tmsh",
    blurb: "The iApps templating surface and the tmsh:: commands it shares with the shell.",
    source_dir: "rust/tcl-registry/src/commands/iapps",
    specs: SpecSource::Built(iapps::iapps_command_specs),
    placement: Placement::Layer(SurfaceLayer::Package("iapps")),
    views: &TMSH_VIEWS,
    hooks: &HookTable::EMPTY,
    document_grammar: None,
    in_universe: true,
    on_install: None,
    cell: SpecCell::new(),
};

// The DSLs' statement words are ordinary English nouns outside a pack body,
// so they are not counted as commands of some other dialect.
static SPECTCL: Surface = Surface {
    id: "spectcl",
    label: "SpecTcl",
    blurb: "The .tclspec authoring DSL's own declaration words.",
    source_dir: "rust/tcl-registry/src/commands/spectcl",
    specs: SpecSource::Built(spectcl::spectcl_command_specs),
    placement: Placement::Layer(SurfaceLayer::Package("spectcl")),
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: Some(&crate::definer::SPECTCL_DOCUMENT_GRAMMAR),
    in_universe: false,
    on_install: None,
    cell: SpecCell::new(),
};

static SSLICTCL: Surface = Surface {
    id: "sslictcl",
    label: "SslicTcl",
    blurb: "The .sslictcl TLS-assurance authoring DSL's declaration words.",
    source_dir: "rust/tcl-registry/src/commands/sslictcl",
    specs: SpecSource::Built(sslictcl::sslictcl_command_specs),
    placement: Placement::Layer(SurfaceLayer::Package("sslictcl")),
    views: &[],
    hooks: &HookTable::EMPTY,
    document_grammar: Some(&crate::definer::SSLICTCL_DOCUMENT_GRAMMAR),
    in_universe: false,
    on_install: None,
    cell: SpecCell::new(),
};

/// The tmsh shell's own layer: the `tmsh::` surface shared with iApps,
/// without the iApp-only commands.
static TMSH_VIEWS: [SurfaceView; 1] = [SurfaceView {
    layer: SurfaceLayer::Package("tmsh"),
    filter: iapps::is_tmsh_spec,
    cell: SpecCell::new(),
}];
