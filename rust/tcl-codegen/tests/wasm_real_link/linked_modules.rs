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

//! Emitted modules run against the real runtime, and a module stating another ABI or intrinsic table is refused.

use super::*;

/// Emitted modules linked against the real runtime produce real side effects in
/// the live interpreter — observed by reading state back through the same interp.
#[test]
fn emitted_modules_run_against_the_real_runtime() {
    let Some(runtime) = real_link_runtime() else {
        return;
    };

    // (program run by `::top`, the query the bootstrap then evaluates, expected).
    // The lowered Set uses the general plan and its void entry ABI.
    assert_eq!(
        run_real_link(&runtime, "mixed0", "set x 42\n", "set x"),
        "42"
    );
    let cases = [
        // A proc defined and called: frame push + parameter binding + `return` +
        // string interpolation + command substitution — all pure, so it runs on
        // the wasm build (no `expr`/numeric tower, no host capabilities).
        (
            "proc greet {name} {return \"hi $name\"}\nset r [greet world]\n",
            "set r",
            "hi world",
        ),
        (
            "proc add {b c} {\n    return [expr {$b + $c}]\n}\n\nset e 2\nset f 4\nputs [add $e $f]\n",
            "set e",
            "6\n2",
        ),
    ];
    for (index, (program, query, expected)) in cases.into_iter().enumerate() {
        assert_eq!(
            run_real_link(&runtime, &format!("mixed{}", index + 1), program, query),
            expected,
            "program {program:?}, query {query:?}"
        );
    }
}

/// A module states the ABI and the intrinsic table it was emitted against in
/// its `tcl.manifest` section, the runtime states its own through
/// `tcl_runtime_identity`, and a disagreement is refused before anything is
/// linked. The matching module is the control: the same program, emitted for
/// the runtime's own table, links and runs.
#[test]
fn a_module_with_a_foreign_intrinsic_table_is_refused() {
    use tcl_runtime_api::ManifestField;

    let Some(runtime) = real_link_runtime() else {
        return;
    };
    let program = "set x 42\n";
    let compile = |edit: fn(&mut tcl_runtime_api::ArtefactIdentityManifest)| {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
        let mut output = compile_wasm(&unit, &registry, WasmCompileOptions::runtime_linked());
        let abi = top_abi(&output);
        edit(
            output
                .manifest
                .as_mut()
                .expect("the emitter states a manifest"),
        );
        (output.to_bytes(), abi)
    };

    // The runtime's own statement is the table this checkout's compiler keyed
    // against: the honest module agrees with it and links.
    let (honest, honest_abi) = compile(|_| {});
    assert_eq!(check_manifest(&runtime, &honest), Ok(()));
    let identity = common::wasm_link::runtime_identity(&runtime);
    assert_eq!(
        identity.intrinsic_table_hash,
        tcl_registry::intrinsic_table_hash()
    );
    assert_eq!(
        run_link_bytes_with_top_abi(
            &runtime,
            "manifest_control",
            &honest,
            program,
            "set x",
            honest_abi,
        ),
        "42"
    );

    // A module keyed against another intrinsic table is refused, naming both.
    let (foreign, foreign_abi) = compile(|manifest| manifest.intrinsic_table_hash[0] ^= 1);
    assert_eq!(foreign_abi, honest_abi);
    match check_manifest(&runtime, &foreign) {
        Err(LinkRefusal::Disagrees {
            field: ManifestField::IntrinsicTableHash,
            module,
            runtime: held,
        }) => {
            assert_ne!(module, held);
            assert_eq!(held.len(), 64, "the runtime's hash is 32 bytes of hex");
        }
        other => panic!("a foreign intrinsic table was not refused: {other:?}"),
    }

    // So is one emitted for another ABI, and one that states nothing.
    let (other_abi, other_top_abi) = compile(|manifest| manifest.abi_version ^= 1);
    assert_eq!(other_top_abi, honest_abi);
    assert!(matches!(
        check_manifest(&runtime, &other_abi),
        Err(LinkRefusal::Disagrees {
            field: ManifestField::AbiVersion,
            ..
        })
    ));
    let mut bare = {
        let registry = CommandRegistry::build_default();
        let unit = CompilationUnit::build_for_dialect(program, &registry, false, "tcl9.0");
        compile_wasm(&unit, &registry, WasmCompileOptions::runtime_linked())
    };
    bare.manifest = None;
    assert_eq!(
        check_manifest(&runtime, &bare.to_bytes()),
        Err(LinkRefusal::NoManifest)
    );
}
