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

//! Broad data/accessor coverage sweep for the `tcl-registry` crate.
//!
//! ## Purpose
//!
//! The sibling `registry_commands.rs` ports a curated set of
//! C-Tcl-observable facts about *selected* commands. This file is the
//! complement: a **broad sweep** that iterates the *entire* command set in
//! every known dialect, plus *every* BIG-IP object spec, and exercises *every*
//! public query accessor on each. Its goal is data/accessor **coverage** — the
//! per-spec data tables, the hover/forms/option metadata, and the per-spec
//! query helpers that no other test references — verified with **invariants**
//! (no panic; arity self-consistent; typed results where a flag is set; hover
//! is non-empty when present) rather than brittle exact counts, so it survives
//! registry data growth.
//!
//! ## What is tclsh-anchored vs registry-metadata
//!
//! Most of what the sweep touches is **registry-internal metadata** (traits,
//! taint colours, side-effect classifications, hover prose, arg roles, forms,
//! body-kinds) and **F5/iRules/BigIP dialect data** — none of which exists in
//! real `tclsh`. Those are asserted against the registry only and marked
//! `// registry-metadata`, `// f5-dialect`, or `// bigip`.
//!
//! A handful of facts the registry asserts about **real Tcl commands** are
//! pinned to C-Tcl ground truth, verified with `scripts/dev/tclsh_check.sh`
//! against tclsh8.6 and tclsh9.0 and recorded in `// tclsh:` comments:
//!   - core command existence (`info commands`),
//!   - the `string` / `dict` / `info` / `array` / `namespace` / `clock`
//!     subcommand sets,
//!   - the `socket -server` switch,
//!   - the `lsort` / `lsearch` / `string is` option/class sets,
//!   - the `--` option terminator on `switch`.
//!
//! tclsh8.4 / tclsh8.5 are not installed in this environment; introduction
//! versions cited for those are well-known Tcl history consistent with the
//! 8.6/9.0 probes.

use std::collections::BTreeSet;
use tcl_dialect::model::Family;
use tcl_dialect::model::SpecSurface;
use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::surface_admits;
use tcl_registry::arity::{Arity, ArityWindow};
use tcl_registry::bigip::{BigipRegistry, ValueKind, default_registry};
use tcl_registry::events::EventRegistry;
use tcl_registry::hover::FormKind;
use tcl_registry::lifecycle::Lifecycle;
use tcl_registry::profiles::ProfileRegistry;
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::stamp_window::StampWindow;
use tcl_registry::taint::TaintColour;
// `registry_for_dialect` is deliberately *not* imported: this file defines its
// own below, routing the sweep through the shipped `.tclspec` loadables so the
// EDA specs are present. The tests `origin/rust` added here call it too, and
// want that same pack-carrying registry.
use tcl_registry::{
    ArgRole, CallbackEffect, CommandRegistry, DispatchDependencies, DispatchDependencyDescriptor,
    ResolvedDispatchDependencies, ResultStability, StateTransitionArgumentShape,
    StateTransitionCommit, StateTransitionComposition, StateTransitionDescriptor, Traits,
    WorldEffectComposition, WorldEffectDescriptor,
};

mod accessor_sweep;
mod arity_and_stamp_windows;
mod command_families;
mod extension_default;
mod f5_sweeps;
mod lifecycles;
mod member_effects;
mod role_and_option_metadata;
mod semantic_declarations;
mod spec_consistency;

use arity_and_stamp_windows::{assert_arity_consistent, assert_arity_windows_consistent};

// Helpers

/// The registry for `dialect` **with the shipped `.tclspec` loadables
/// installed** — the sweep's stand-in for [`tcl_registry::registry_for_dialect`].
///
/// The EDA vendor libraries are bundled loadables (`sdc_base` and the five
/// vendor packs have no Rust modules; see `docs/design/registry/spec-packs.md`),
/// so the plain per-profile registry carries no `get_cells` or `synth_design`
/// of its own. Routing the sweep through the pack loader is what
/// keeps those ~350 specs under every accessor assertion below — and it means
/// the sweep now tests the loader's output as well as the registry's, which is
/// the point of shipping them as loadables.
///
/// The pack directory is named outright (the repository's `specs/`, which is
/// what a release lays down beside the executable) rather than discovered, so
/// the sweep does not depend on where the test binary sits or on the ambient
/// `TCL_LSP_SPEC_PACK_DIR`.
fn registry_for_dialect(dialect: &str) -> std::sync::Arc<CommandRegistry> {
    use std::sync::OnceLock;
    static PACKS: OnceLock<tcl_spectcl::PackSet> = OnceLock::new();
    let packs = PACKS.get_or_init(|| {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs");
        let set = tcl_spectcl::bundled::load_from(&dir);
        assert!(
            !set.is_empty(),
            "the shipped EDA loadables must be present at {}",
            dir.display()
        );
        set
    });
    tcl_spectcl::bundled::registry_for_dialect_from(dialect, packs)
}

/// Every dialect name that loads a non-trivial command pack — a compiled-in
/// one by its `SpecSurface` bit, or the EDA vendor libraries through the
/// bundled loadables [`registry_for_dialect`] installs. (The config-only
/// `f5-bigip` / `f5-tmsh` catalogue names collapse to plain Tcl, so
/// they are covered via the catalogue sweep rather than as load targets.)
const LOADABLE_DIALECTS: &[&str] = &[
    "tcl8.4",
    "tcl8.5",
    "tcl8.6",
    "tcl9.0",
    "f5-irules",
    "f5-iapps",
    "expect",
    "synopsys-eda-tcl",
    "cadence-eda-tcl",
    "xilinx-eda-tcl",
    "intel-quartus-eda-tcl",
    "mentor-eda-tcl",
    "microchip-libero-eda-tcl",
    "bpf",
    "spectcl",
    "sslictcl",
];

/// Every trait bit, so the sweep queries `commands_with_trait` across the whole
/// lattice (executing the per-trait scan + classification helpers).
const ALL_TRAITS: &[Traits] = &[
    Traits::CONTROL_FLOW,
    Traits::LANGUAGE_KEYWORD,
    Traits::HAS_BOOLEAN_COND,
    Traits::TERMINATES_BLOCK,
    Traits::TERMINATES_PROCESS,
    Traits::HAS_LOOP_BODY,
    Traits::NEVER_INLINE_BODY,
    Traits::LOOP_LIST_HEADER,
    Traits::PURE,
    Traits::CSE_CANDIDATE,
    Traits::PURE_EVALUATION,
    Traits::DEFINES_PROCEDURE,
    Traits::DESTROYS_VARIABLE,
    Traits::READS_BEFORE_WRITE,
    Traits::CREATES_SCOPE_ALIAS,
    Traits::CREATES_BARRIER,
    Traits::EVALUATES_CODE,
    Traits::PERFORMS_SUBSTITUTION,
    Traits::OPENS_CHANNEL,
    Traits::SOURCES_FILE,
    Traits::IS_EVENT_HANDLER,
    Traits::UNNORMALISED_HTTP_GETTER,
    Traits::RETURNS_PATH,
    Traits::IS_UNESCAPE,
    Traits::PRODUCES_CANONICAL_LIST,
    Traits::BUILDS_COMMAND_PREFIX,
    Traits::UNSAFE,
    Traits::IS_SIDE_SWITCH,
    Traits::IRULES_TOP_LEVEL_ONLY,
    Traits::SETS_EVENT_PRIORITY,
    Traits::IS_OO_METACLASS,
    Traits::DIAGRAM_ACTION,
    Traits::NEEDS_START_CMD,
    Traits::TAINT_SINK,
    Traits::TAINT_SOURCE,
    Traits::CREATES_DYNAMIC_BARRIER,
    Traits::INVOKES_USER_PROC,
    Traits::BYTE_COMPILED,
    Traits::NOT_PROC_FACTORY,
    Traits::FRAMELESS_RUNTIME,
    Traits::FIRST_ARG_VARNAME,
    Traits::WHOLE_ARRAY_ARG,
    Traits::DYNAMIC_EVAL_BODY,
    Traits::INTROSPECTS_BY_NAME,
    Traits::TARGETS_VARIABLE_BY_NAME,
    Traits::FRAME_HASH_BUILTIN,
    Traits::OVERRIDABLE_LIBRARY_PROC,
    Traits::EXPR_CONCATENATES_ARGS,
    Traits::STRUCTURALLY_CHECKED_ARITY,
    Traits::TCLOO_NEXT_CHAIN,
    Traits::ESTABLISHES_VARIABLE_TRACE,
    Traits::TRANSFERS_CONTROL,
    Traits::COROUTINE_PRIMITIVE,
    Traits::FIRE_AND_FORGET_TEARDOWN,
    Traits::SCRIPT_CONCATENATES_ARGS,
    Traits::SCRIPT_APPENDS_LIST_ARGS,
    Traits::TCLOO_SELF_DISPATCH,
    Traits::TCLOO_INTROSPECTION,
    Traits::TCLOO_RUNTIME_HANDLE_RESULT,
    Traits::BRANCH_SELECTED_BODY,
];
