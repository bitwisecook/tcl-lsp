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

//! The shipped EDA loadables, end to end.
//!
//! `sdc_base` and the five vendor libraries used to be ~350 Rust modules under
//! `tcl-registry/src/commands/`. They are `.tclspec` files under `specs/` now
//! (`docs/design/spec-packs.md`: "the EDA vendor libraries ship as bundled
//! `.tclspec` loadables … so the loader path is exercised in production from
//! day one rather than reserved for private packs"), so *every* fact an EDA
//! dialect knows arrives through discovery → parse → merge → install.
//!
//! This file is what used to be true by construction and now has to be tested:
//! that the packs are found, that each EDA profile resolves its own vendor's
//! commands and none of a rival's, and that the facts the analyser and the
//! compiler read off those specs — arity, roles, traits, the shared
//! `foreach_in_collection` analyser hook — survive the round trip from disk.

use std::path::PathBuf;
use std::sync::OnceLock;

use tcl_registry::Traits;
use tcl_registry::arg_role::ArgRole;
use tcl_registry::hooks::AnalyserHookId;
use tcl_registry::registry::CommandRegistry;
use tcl_spectcl::{PackSet, bundled};

/// The repository's `specs/` — what a release lays down beside the executable.
fn specs_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs")
}

/// The shipped set, loaded once for this binary.
fn shipped() -> &'static PackSet {
    static PACKS: OnceLock<PackSet> = OnceLock::new();
    PACKS.get_or_init(|| bundled::load_from(&specs_dir()))
}

fn registry(dialect: &str) -> std::sync::Arc<CommandRegistry> {
    bundled::registry_for_dialect_from(dialect, shipped())
}

/// Every EDA profile, with the vendor package it ships and one command that
/// only that vendor has.
const PROFILES: &[(&str, &str, &str)] = &[
    ("xilinx-eda-tcl", "vivado", "synth_design"),
    ("synopsys-eda-tcl", "synopsys-dc", "compile_ultra"),
    ("cadence-eda-tcl", "cadence-innovus", "ccopt_design"),
    ("intel-quartus-eda-tcl", "quartus-flow", "execute_flow"),
    ("mentor-eda-tcl", "questa", "vsim"),
];

/// The six packs are on disk, load without a warning, and carry the whole
/// command surface the Rust modules used to.
#[test]
fn the_six_packs_load_clean_and_carry_every_command() {
    let set = shipped();
    let counts: Vec<(String, usize)> = set
        .packs
        .iter()
        .map(|pack| (pack.name.clone(), pack.commands.len()))
        .collect();
    assert_eq!(
        counts,
        vec![
            ("eda_cadence".to_owned(), 56),
            ("eda_mentor".to_owned(), 49),
            ("eda_quartus".to_owned(), 48),
            ("eda_synopsys".to_owned(), 68),
            ("eda_xilinx".to_owned(), 64),
            ("sdc_base".to_owned(), 61),
        ],
        "the migrated command counts, pack by pack"
    );
    let total: usize = set.packs.iter().map(|p| p.commands.len()).sum();
    assert_eq!(total, 346, "every command the six Rust packs used to build");

    let warnings: Vec<String> = set
        .notices
        .iter()
        .filter(|n| n.severity == tcl_spectcl::pack::Severity::Warning)
        .map(|n| format!("{}:{} {}", n.path.display(), n.line, n.message))
        .collect();
    assert!(
        warnings.is_empty(),
        "a shipped pack must never load with a degradation notice:\n{}",
        warnings.join("\n")
    );
}

/// Each EDA profile resolves its own vendor's commands, plus the shared SDC
/// library, through the loader.
#[test]
fn every_eda_profile_resolves_its_own_vendor_library() {
    for &(dialect, package, own) in PROFILES {
        let reg = registry(dialect);
        let spec = reg
            .get(own)
            .unwrap_or_else(|| panic!("{dialect}: `{own}` must come from the bundled pack"));
        assert_eq!(spec.required_package, Some(package), "{dialect}/{own}");

        // The shared SDC constraint library rides along with every shell.
        let sdc = reg
            .get("create_clock")
            .unwrap_or_else(|| panic!("{dialect}: sdc_base must load too"));
        assert_eq!(sdc.required_package, Some("sdc"), "{dialect}/create_clock");
    }
}

/// A profile never sees a rival vendor's library — the filter that replaced
/// `CommandRegistry::load_eda_packs`'s by-profile selection.
///
/// This is not pedantry: four packs declare a `report_timing`, so without the
/// filter a Vivado document's `report_timing` would be whichever pack sorted
/// first, and — failing its `required_package` gate under this profile — would
/// then resolve to nothing at all.
#[test]
fn a_profile_never_sees_a_rival_vendors_library() {
    for &(dialect, _, _) in PROFILES {
        let reg = registry(dialect);
        for &(other, _, rival) in PROFILES {
            if other == dialect {
                continue;
            }
            assert!(
                reg.get(rival).is_none(),
                "{dialect} must not carry {other}'s `{rival}`"
            );
        }
    }
    // `report_timing` is the collision itself: every shell that has one must
    // have *its own*, gated on a package that shell ships.
    for &(dialect, _, _) in PROFILES {
        let reg = registry(dialect);
        let spec = reg
            .get("report_timing")
            .unwrap_or_else(|| panic!("{dialect}: report_timing"));
        let package = spec.required_package.expect("an EDA package gate");
        assert!(
            tcl_dialect::DialectProfile::by_name(dialect).is_ambient_package(package),
            "{dialect}: report_timing resolved to `{package}`, which this shell does not ship"
        );
    }
}

/// A plain-Tcl profile gets none of it, exactly as before the migration: the
/// packs are discovered for every dialect, and the vendor gate is what keeps
/// `get_cells` out of a `tcl9.0` document.
#[test]
fn plain_tcl_sees_no_eda_commands() {
    let reg = registry("tcl9.0");
    for name in [
        "get_cells",
        "create_clock",
        "synth_design",
        "compile_ultra",
        "vsim",
    ] {
        assert!(reg.get(name).is_none(), "tcl9.0 must not carry `{name}`");
    }
}

/// The facts the analyser and the compiler read off an EDA spec survive the
/// trip through the DSL — the round-trip gate's claim, re-checked against the
/// files that actually ship rather than against a rendered string.
#[test]
fn the_loaded_specs_carry_their_analysis_facts() {
    let reg = registry("synopsys-eda-tcl");

    let foreach = reg
        .get("foreach_in_collection")
        .expect("the SDC loop command");
    assert_eq!(foreach.analyser_hook, Some(AnalyserHookId::Foreach));
    assert!(
        foreach.traits.contains(
            Traits::CONTROL_FLOW
                | Traits::HAS_LOOP_BODY
                | Traits::NEVER_INLINE_BODY
                | Traits::LOOP_LIST_HEADER
        ),
        "the loop traits: {:?}",
        foreach.traits
    );
    assert_eq!(
        foreach.arg_roles,
        &[(0, ArgRole::VarWrite), (2, ArgRole::Body)],
        "the variable it writes and the body it runs"
    );
    assert!(
        foreach.arity.accepts(3),
        "foreach_in_collection var coll body"
    );
    assert!(!foreach.arity.accepts(2));

    let append = reg.get("append_to_collection").expect("an SDC var writer");
    assert_eq!(append.arg_roles, &[(0, ArgRole::VarWrite)]);

    let compile = reg.get("compile_ultra").expect("a Synopsys command");
    let hover = compile.hover.expect("hover survives the migration");
    assert_eq!(hover.summary, "Compile with advanced optimizations.");
    assert_eq!(
        compile.primary_synopsis(),
        Some(
            "compile_ultra ?-incremental? ?-retime? ?-scan? ?-no_autoungroup? \
             ?-no_boundary_optimization? ?-gate_clock? ?-timing_high_effort_script?"
        )
    );
}

/// Nothing compiled in answers for these commands any more: the pack is the
/// only source. A registry built without the loadables has none of them.
#[test]
fn the_compiled_registry_has_no_eda_commands_left() {
    let plain = tcl_registry::registry_for_dialect("xilinx-eda-tcl");
    for name in ["synth_design", "get_cells", "create_clock"] {
        assert!(
            plain.get(name).is_none(),
            "`{name}` must reach a registry only through the bundled pack"
        );
    }
}
