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

//! The cross-crate drift gate for Tcl's dict canonicalisation rule (#1608).
//!
//! "A repeated key keeps its **first-occurrence position** and its **last
//! value**" (`SetDictFromAny`, `tmp/tcl9.0.4/generic/tclDictObj.c:589`, over
//! `Tcl_DictObjPut`'s hash overwrite) is one rule, and it now has one
//! implementation — [`tcl_syntax::value::canonical_dict_slots`] — which three
//! layers bind:
//!
//! 1. the runtime seam `ValueOps::dict_pairs` (this crate's `Vm` binds it, and
//!    every VM `dict` opcode goes through that),
//! 2. `tcl_registry::const_fold`'s `dict` folders (`get`/`exists`/`size`/
//!    `keys`/`values`/`create`/`merge`), reached here through the public
//!    `run_const_fold` path the optimiser uses,
//! 3. `tcl_compiler::codegen::helpers::fold_dict_create_cmd`, the codegen's
//!    `[dict create …]` fold.
//!
//! Before #1608 those were three independent copies of the walk, plus a fourth
//! place where the rule had been *missed* — `parse_dict`, which fed six folders
//! first-match semantics, so `[dict get {a 1 a 2} a]` folded to `1` where both
//! oracles say `2` (#1427 / #1591). `cargo xtask owner-resolution` cannot see
//! that class of drift: it validates the manifest, not whether a surface calls
//! the owner at all.
//!
//! So this suite is the semantic gate. It feeds one duplicate-key / odd-shape
//! corpus through **every** layer and asserts byte identity, and — when a real
//! `tclsh8.6` / `tclsh9.0` is present (`TCL_LSP_TCLSH86` / `TCL_LSP_TCLSH90`
//! override the PATH names) — against C Tcl too. A fourth copy, or a diverging
//! edit to any binding, fails a test by name here rather than shipping a fold
//! that changes program results.

use std::io::Write as _;

use tcl_dialect::TclVersion;
use tcl_registry::CommandRegistry;
use tcl_syntax::value::ValueOps as _;

/// Dict *strings* — what `SetDictFromAny` canonicalises on the way in.
///
/// Rows cover: no duplicate, adjacent and separated duplicates, a triple, the
/// empty dict, an empty key and an empty value, a braced key whose string rep
/// needs quoting, a nested dict value (never re-canonicalised), numeric-looking
/// keys that are *not* equal as strings (`1` vs `01`), and a key that repeats
/// only after a value spelling that looks like a key.
const DICT_CORPUS: &[&str] = &[
    "",
    "a 1",
    "a 1 b 2",
    "a 1 a 2",
    "a 1 a 2 a 3",
    "a 1 b 2 a 3",
    "x 1 x 2 y 3",
    "{} 1 {} 2",
    "a {} a {}",
    "{k k} 1 {k k} 2",
    "o {a 1 a 2} o {b 3}",
    "1 one 01 oh-one 1 uno",
    "a b b a a c",
    // A leading `#` is comment-unsafe only in list position 0, so these rows
    // pin the *rendering* of a canonicalised dict as well as its pairing.
    "# 1 a 2",
    "a 1 # 2",
    "# 1 # 2",
];

/// `dict create` **argument lists** — `DictCreateCmd`'s walk, which puts each
/// pair into a fresh dict and therefore obeys the same rule.
const CREATE_CORPUS: &[&[&str]] = &[
    &[],
    &["a", "1"],
    &["a", "1", "a", "2"],
    &["a", "1", "b", "2", "a", "3"],
    &["a", "1", "a", "2", "a", "3"],
    &["", "1", "", "2"],
    &["k k", "1", "k k", "2"],
    &["a", "x y", "a", "z w"],
    &["#", "1"],
    &["a", "1", "#", "2"],
];

// -- the three bindings -----------------------------------------------------

/// Leg 1 — the owner, through this crate's `ValueOps` binding (the same code
/// path every VM `dict` opcode takes).
fn owner_pairs(dict: &str) -> Vec<(String, String)> {
    let mut vm = tcl_vm::Vm::new();
    let value = vm.new_str(dict);
    let pairs = vm
        .dict_pairs(&value)
        .expect("every corpus row is an even-length list");
    pairs
        .into_iter()
        .map(|(k, v)| (vm.as_str(&k).to_string(), vm.as_str(&v).to_string()))
        .collect()
}

/// Leg 2 — a registry `dict` const-fold, resolved exactly as the optimiser
/// resolves it.
fn registry_fold(registry: &CommandRegistry, sub: &str, args: &[&str]) -> Option<String> {
    registry
        .get("dict")?
        .subcommand(sub)?
        .run_const_fold(args, Some(TclVersion::V9_0))
}

/// Leg 3 — the codegen's `[dict create …]` fold, fed the source spelling it
/// sees in a compiled word.
fn compiler_dict_create_fold(args: &[&str]) -> Option<String> {
    let mut source = String::from("[dict create");
    for arg in args {
        source.push(' ');
        source.push_str(&tcl_syntax::list::list_element(arg));
    }
    source.push(']');
    tcl_compiler::codegen::helpers::fold_dict_create_cmd(
        &source,
        tcl_syntax::word_rules::WordValueRules::TCL,
    )
}

/// Leg 4 — real C Tcl, when it is installed. `None` means "not available", not
/// "disagreed": the suite still checks the three in-tree legs against each
/// other and prints a skip note.
fn tclsh_value(script: &str) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for (env, name) in [
        ("TCL_LSP_TCLSH86", "tclsh8.6"),
        ("TCL_LSP_TCLSH90", "tclsh9.0"),
    ] {
        let mut candidates = Vec::new();
        if let Ok(explicit) = std::env::var(env) {
            candidates.push(explicit);
        }
        candidates.push(name.to_owned());
        for candidate in candidates {
            let Ok(mut child) = std::process::Command::new(&candidate)
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .spawn()
            else {
                continue;
            };
            child
                .stdin
                .as_mut()
                .expect("stdin")
                .write_all(script.as_bytes())
                .expect("write");
            let result = child.wait_with_output().expect("run");
            if result.status.success() {
                out.push((
                    name,
                    String::from_utf8_lossy(&result.stdout).trim().to_owned(),
                ));
            }
            break;
        }
    }
    out
}

/// Render a Tcl script word for a literal string.
fn word(s: &str) -> String {
    tcl_syntax::list::list_element(s)
}

// -- the gates --------------------------------------------------------------

/// The owner and the registry folders must agree on every dict, key for key
/// and value for value — the folders read a *string* and the owner a value,
/// but the canonicalisation between them is the same function.
#[test]
fn registry_dict_folds_agree_with_the_owner() {
    let registry = CommandRegistry::build_default();
    for dict in DICT_CORPUS {
        let pairs = owner_pairs(dict);
        let keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_str()).collect();
        let values: Vec<&str> = pairs.iter().map(|(_, v)| v.as_str()).collect();

        assert_eq!(
            registry_fold(&registry, "size", &[dict]),
            Some(pairs.len().to_string()),
            "`dict size` disagrees with the owner on {dict:?}"
        );
        assert_eq!(
            registry_fold(&registry, "keys", &[dict]),
            Some(tcl_syntax::list::join_list(&keys)),
            "`dict keys` disagrees with the owner on {dict:?}"
        );
        assert_eq!(
            registry_fold(&registry, "values", &[dict]),
            Some(tcl_syntax::list::join_list(&values)),
            "`dict values` disagrees with the owner on {dict:?}"
        );
        for (key, value) in &pairs {
            assert_eq!(
                registry_fold(&registry, "get", &[dict, key]),
                Some(value.clone()),
                "`dict get {key:?}` disagrees with the owner on {dict:?}"
            );
            assert_eq!(
                registry_fold(&registry, "exists", &[dict, key]),
                Some("1".to_owned()),
                "`dict exists {key:?}` disagrees with the owner on {dict:?}"
            );
        }
    }
}

/// The registry's `dict create` fold and the codegen's are the two
/// compile-time renderings of the same walk — byte identity, not merely the
/// same pairs.
#[test]
fn dict_create_folds_agree_across_registry_and_codegen() {
    let registry = CommandRegistry::build_default();
    for args in CREATE_CORPUS {
        let from_registry = registry_fold(&registry, "create", args);
        let from_codegen = compiler_dict_create_fold(args);
        // Declining is always safe — an unfolded call is simply evaluated at
        // run time — so the codegen fold is allowed to abstain, as it does on
        // the no-argument form (`[dict create]` has no argument text for its
        // `"[dict create "` prefix to match). Only a fold that *fires* has to
        // agree, and it has to agree byte for byte.
        if let Some(folded) = &from_codegen {
            assert_eq!(
                Some(folded),
                from_registry.as_ref(),
                "registry and codegen `dict create` folds differ on {args:?}"
            );
        } else {
            assert!(
                args.is_empty(),
                "the codegen `dict create` fold stopped firing on {args:?}"
            );
        }
        // And both must be the owner's canonical pairing of the same words.
        let owner = owner_pairs(&tcl_syntax::list::join_list(*args));
        let expected: Vec<String> = owner
            .iter()
            .flat_map(|(k, v)| [k.clone(), v.clone()])
            .collect();
        assert_eq!(
            from_registry,
            Some(tcl_syntax::list::join_list(&expected)),
            "the `dict create` fold is not the owner's canonicalisation on {args:?}"
        );
    }
}

/// Every layer against real C Tcl. The oracle is the reason the parity above
/// is worth anything: three implementations can agree and still be wrong.
#[test]
fn dict_canonicalisation_matches_real_tclsh() {
    let registry = CommandRegistry::build_default();
    let mut checked = 0usize;

    for dict in DICT_CORPUS {
        let script = format!(
            "set d {}\nputs [dict size $d]\nputs [dict keys $d]\nputs [dict values $d]\n",
            word(dict)
        );
        for (release, output) in tclsh_value(&script) {
            let mut lines = output.lines();
            let (size, keys, values) = (
                lines.next().unwrap_or_default(),
                lines.next().unwrap_or_default(),
                lines.next().unwrap_or_default(),
            );
            let pairs = owner_pairs(dict);
            let owner_keys: Vec<&str> = pairs.iter().map(|(k, _)| k.as_str()).collect();
            let owner_values: Vec<&str> = pairs.iter().map(|(_, v)| v.as_str()).collect();
            assert_eq!(
                pairs.len().to_string(),
                size,
                "{release}: owner `dict size` differs on {dict:?}"
            );
            assert_eq!(
                tcl_syntax::list::join_list(&owner_keys),
                keys,
                "{release}: owner `dict keys` differs on {dict:?}"
            );
            assert_eq!(
                tcl_syntax::list::join_list(&owner_values),
                values,
                "{release}: owner `dict values` differs on {dict:?}"
            );
            assert_eq!(
                registry_fold(&registry, "keys", &[dict]).as_deref(),
                Some(keys),
                "{release}: folded `dict keys` differs on {dict:?}"
            );
            assert_eq!(
                registry_fold(&registry, "values", &[dict]).as_deref(),
                Some(values),
                "{release}: folded `dict values` differs on {dict:?}"
            );
            checked += 1;
        }
    }

    for args in CREATE_CORPUS {
        let mut script = String::from("puts [dict create");
        for arg in *args {
            script.push(' ');
            script.push_str(&word(arg));
        }
        script.push_str("]\n");
        for (release, want) in tclsh_value(&script) {
            assert_eq!(
                registry_fold(&registry, "create", args).as_deref(),
                Some(want.as_str()),
                "{release}: registry `dict create` fold differs on {args:?}"
            );
            if let Some(folded) = compiler_dict_create_fold(args) {
                assert_eq!(
                    folded, want,
                    "{release}: codegen `dict create` fold differs on {args:?}"
                );
            }
            checked += 1;
        }
    }

    if checked == 0 {
        eprintln!(
            "SKIPPING the tclsh oracle comparison: neither tclsh8.6 (or \
             $TCL_LSP_TCLSH86) nor tclsh9.0 (or $TCL_LSP_TCLSH90) was found"
        );
    }
}

/// The rule itself, stated once as slot indices, on the shapes the string and
/// argument walks share. If this drifts, everything above drifts with it —
/// which is the point of there being only one of it.
#[test]
fn canonical_dict_slots_is_first_position_last_value() {
    use tcl_syntax::value::canonical_dict_slots;

    assert_eq!(canonical_dict_slots(Vec::<&str>::new()), vec![]);
    assert_eq!(canonical_dict_slots(["a", "b"]), vec![(0, 0), (1, 1)]);
    assert_eq!(canonical_dict_slots(["a", "a"]), vec![(0, 1)]);
    assert_eq!(canonical_dict_slots(["a", "b", "a"]), vec![(0, 2), (1, 1)]);
    assert_eq!(canonical_dict_slots(["a", "a", "a"]), vec![(0, 2)]);
    // Keys compare by their exact rep — `1` and `01` are different keys.
    assert_eq!(canonical_dict_slots(["1", "01", "1"]), vec![(0, 2), (1, 1)]);
    // Byte keys bind the same rule (the shape a byte-oriented value model uses).
    assert_eq!(
        canonical_dict_slots([b"a".as_slice(), b"a".as_slice()]),
        vec![(0, 1)]
    );
}
