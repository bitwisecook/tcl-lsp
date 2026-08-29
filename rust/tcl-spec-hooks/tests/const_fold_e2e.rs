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

//! **The proving case**: a pack's `const_fold` body folds in the optimiser.
//!
//! Everything below the surface of this test is the whole stack — the DSL's
//! calling convention, the emitter protocol, the sandboxed VM, the registry
//! slot and its thunk, and the compiler's const-substitution engine — but the
//! test only ever asks the two questions that matter:
//!
//! 1. Does `tcl_compiler::optimiser` actually replace `[mylib::strlen abcde]`
//!    with `5`, driven by a Tcl body a pack declared?
//! 2. Does that body agree with the **native** folder for `string length` on
//!    the same inputs, input for input?
//!
//! The second question is what makes the first meaningful: a fold that fires
//! but disagrees with the shipped folder is a miscompile, not a feature.

use std::rc::Rc;

use tcl_compiler::optimiser::manager::optimise_raw;
use tcl_registry::pack_hooks::{self, HookFamily, HookInputs};
use tcl_registry::spec::CommandSpec;
use tcl_registry::{CommandRegistry, arity::Arity};
use tcl_spec_hooks::{HookProgram, PackPrograms, tclvm_host};
use tcl_dialect::model::Family;

/// The pack body under test: `string length` re-expressed in the DSL, guarded
/// on `string is ascii` exactly as [`string.tclspec`] does so it stays inside
/// the range 8.x and 9.x agree on.
///
/// [`string.tclspec`]: docs/design/spec-dsl-examples/string.tclspec
const STRLEN_BODY: &str = r"
    set subject [lindex $words 0]
    if {![string is ascii $subject]} { return }
    fold [string length $subject]
";

/// Inputs the two folders are compared on. ASCII only: the body declines
/// anything else, and so does the equivalence claim.
const INPUTS: &[&str] = &["", "a", "abcde", "a b c", "12345678901234567890"];

struct Folders {
    pack: tcl_registry::hooks::ConstFoldFn,
    registry: CommandRegistry,
}

/// Load the pack, install the host, and build a registry carrying
/// `mylib::strlen` with the pack's folder installed.
fn load() -> Folders {
    let host = Rc::new(tclvm_host());
    let installed = host.install_pack_hooks(
        PackPrograms::new("mylib").with(
            HookProgram::new("mylib::strlen", HookFamily::ConstFold, STRLEN_BODY)
                .with_inputs(HookInputs::parse(&["words"])),
        ),
    );
    assert_eq!(installed.len(), 1);
    assert!(
        installed[0].declined.is_none(),
        "{:?}",
        installed[0].declined
    );
    pack_hooks::install_host(host);

    let slot = installed[0].slot.expect("the hook claimed a slot");
    let pack = pack_hooks::const_fold_fn(slot).expect("a fold slot has a fold thunk");

    let mut registry = CommandRegistry::build_default();
    registry.insert(CommandSpec {
        name: "mylib::strlen",
        arity: Arity::exact(1),
        const_fold: Some(pack),
        ..CommandSpec::DEFAULT
    });
    Folders { pack, registry }
}

/// The shipped native folder for `string length`, for the same input.
fn native_fold(registry: &CommandRegistry, input: &str) -> Option<String> {
    registry
        .get("string")
        .expect("string is a shipped command")
        .resolve_subcommand("length")
        .expect("string length is a shipped subcommand")
        .run_const_fold(&[input], None)
}

#[test]
fn the_pack_fold_agrees_with_the_native_folder_on_the_same_inputs() {
    let folders = load();
    for input in INPUTS {
        let from_pack = (folders.pack)(&[input]);
        let from_native = native_fold(&folders.registry, input);
        assert_eq!(
            from_pack, from_native,
            "pack and native folders disagree on {input:?}"
        );
        assert!(from_pack.is_some(), "both folded {input:?}");
    }
    pack_hooks::clear_host();
}

#[test]
fn a_pack_fold_body_folds_in_the_optimiser_pipeline() {
    let folders = load();
    let optimisations = optimise_raw(
        "proc ::f {} {\n    set y [mylib::strlen abcde]\n}\n",
        &folders.registry,
        None,
    );
    let folded: Vec<String> = optimisations
        .into_iter()
        .filter(|optimisation| optimisation.code.as_str() == "O129")
        .map(|optimisation| optimisation.replacement)
        .collect();
    assert_eq!(
        folded,
        vec!["5".to_string()],
        "the optimiser must fold the pack-declared command"
    );
    pack_hooks::clear_host();
}

#[test]
fn the_body_abstains_the_way_the_native_folder_declines() {
    let folders = load();
    // Non-ASCII: the body returns early, which is an abstention, and the
    // optimiser leaves the call alone.
    assert_eq!((folders.pack)(&["café"]), None);
    let optimisations = optimise_raw(
        "proc ::f {} {\n    set y [mylib::strlen café]\n}\n",
        &folders.registry,
        None,
    );
    assert!(
        !optimisations
            .iter()
            .any(|optimisation| optimisation.code.as_str() == "O129"),
        "an abstaining body must leave the call site untouched"
    );
    pack_hooks::clear_host();
}

#[test]
fn a_dynamic_word_never_reaches_the_body() {
    let folders = load();
    // The literal-only precondition is the loader's: `[mylib::strlen $x]` has
    // a dynamic word, so no fold is attempted at all — the fold must not see
    // an empty string and answer `0`.
    let optimisations = optimise_raw(
        "proc ::f {x} {\n    set y [mylib::strlen $x]\n}\n",
        &folders.registry,
        None,
    );
    assert!(
        !optimisations
            .iter()
            .any(|optimisation| optimisation.code.as_str() == "O129"),
        "a dynamic word must not fold"
    );
    pack_hooks::clear_host();
}

#[test]
fn a_thread_without_the_host_abstains_rather_than_folding() {
    let folders = load();
    pack_hooks::clear_host();
    assert_eq!(
        (folders.pack)(&["abcde"]),
        None,
        "with no host installed the thunk must answer its family's silence"
    );
}
