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

//! The pack store against the **real** hand-written packs.
//!
//! `src/store.rs`'s own tests use small fixtures, which is the right shape for
//! unit tests and the wrong shape for the claim that matters: that the studio
//! can open a `.tclspec` a human wrote — `lsort`, `string`, `switch`, the
//! `TclOO` definers, an iRules pack — hand its commands to the form, take an
//! edit back, and not damage the file. That is what this file tests, over
//! every example in `docs/design/spec-dsl-examples/`.
//!
//! Three properties, one pass each:
//!
//! 1. **Opening is lossless.** The store's source is the file's bytes, and its
//!    derived drafts are one per command the loader found.
//! 2. **An edit is a splice.** Writing a draft back changes the edited
//!    command's statement and *nothing else* — every other command's block
//!    survives byte for byte, and so does every comment outside the edited
//!    statement.
//! 3. **Meaning is preserved.** After the write-back, every other command
//!    still drafts to exactly what it drafted to before, and the edited one
//!    drafts to the edit.

use std::fs;
use std::path::PathBuf;

use serde_json::json;
use tcl_spec_studio::store::{Builtins, PackStore, Resolution, WriteBack};

fn examples() -> Vec<(String, String)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/spec-dsl-examples");
    let mut out: Vec<(String, String)> = fs::read_dir(&dir)
        .expect("the spec-dsl-examples directory")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|e| e == "tclspec"))
        .map(|path| {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            (name, fs::read_to_string(&path).expect("readable"))
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(
        out.len() >= 8,
        "expected the ported example packs, got {out:?}"
    );
    out
}

/// The **hook bodies** `command NAME { … }` declares at its top level, taken
/// verbatim out of the author's file.
///
/// This is the "did the write-back eat the author's Tcl" probe, and it is
/// deliberately narrow. A `hover` block is re-rendered on a write-back and that
/// is fine — its content survives, its whitespace does not, and the draft can
/// carry every word of it. A hook body is the opposite: the draft holds a
/// function pointer, so nothing but the original bytes can put it back. Those
/// are the statements that must appear again unchanged, or be named as lost.
///
/// A hook declaration is recognisable on sight: `name {words ctx} {` — a
/// property word, a parameter list, and an opening brace.
fn hook_bodies(source: &str, command: &str) -> Vec<String> {
    let Some(start) = source.find(&format!("\ncommand {command} ")) else {
        return Vec::new();
    };
    let opens = |line: &str| {
        let body = line.strip_prefix("    ")?;
        if body.starts_with(' ') {
            return None;
        }
        // `<property> {<params>} {`
        let head = body.strip_suffix(" {")?;
        let (word, params) = head.split_once(' ')?;
        if !params.starts_with('{') || !params.ends_with('}') {
            return None;
        }
        Some(word.to_owned())
    };

    let mut out = Vec::new();
    let mut lines = source[start + 1..].lines().skip(1);
    while let Some(line) = lines.next() {
        if line == "}" {
            break;
        }
        if opens(line).is_none() {
            continue;
        }
        let mut block = String::from(line);
        for inner in lines.by_ref() {
            block.push('\n');
            block.push_str(inner);
            if inner == "    }" {
                break;
            }
        }
        out.push(block);
    }
    out
}

/// Opening a hand-written pack yields its commands, and keeps its bytes.
#[test]
fn every_shipped_example_opens_as_a_store() {
    for (name, source) in examples() {
        let store = PackStore::from_source(&source);
        assert_eq!(store.source(), source, "{name}: the document is the store");
        assert!(!store.name().is_empty(), "{name}: no speclib name");
        assert!(
            !store.commands().is_empty(),
            "{name}: no commands came out of a real pack"
        );
        for (command, draft) in store.commands() {
            assert_eq!(
                draft.get("name"),
                Some(&json!(command)),
                "{name}: draft/name mismatch for {command}"
            );
        }
    }
}

/// A form edit on a real pack splices, and damages nothing else.
#[test]
fn a_form_edit_on_a_real_pack_splices_and_preserves_its_neighbours() {
    let mut spliced = 0_usize;
    let mut carried = 0_usize;
    for (name, source) in examples() {
        let before = PackStore::from_source(&source);
        let Some((target, draft)) = before.commands().first().cloned() else {
            continue;
        };

        // The most boring possible edit that is still an edit: give the
        // command a documented return type. Every command can carry one and
        // no example's first command already sets `Dict`.
        let mut edited = draft.clone();
        edited.insert("return_type".to_owned(), json!("Dict"));

        let mut store = PackStore::from_source(&source);
        let write = store.set_command(&target, &edited, before.overrides_shipped(&target));

        assert_eq!(
            store.draft(&target).and_then(|d| d.get("return_type")),
            Some(&json!("Dict")),
            "{name}: the edit did not land on {target}"
        );

        // Every *other* command still means exactly what it meant.
        for (other, was) in before.commands() {
            if other == &target {
                continue;
            }
            assert_eq!(
                store.draft(other),
                Some(was),
                "{name}: editing {target} changed {other}"
            );
            assert_eq!(
                store.overrides_shipped(other),
                before.overrides_shipped(other),
                "{name}: editing {target} changed {other}'s -override"
            );
        }
        assert_eq!(
            store.commands().len(),
            before.commands().len(),
            "{name}: the command list changed shape"
        );

        if write.how == WriteBack::Spliced {
            spliced += 1;
            // The file's own banner comment is above the `speclib`, so a
            // splice can never touch it.
            let banner: Vec<&str> = source
                .lines()
                .take_while(|line| !line.starts_with("speclib "))
                .filter(|line| line.starts_with('#'))
                .collect();
            for line in banner {
                assert!(
                    store.source().contains(line),
                    "{name}: a splice ate the banner line {line:?}"
                );
            }
        }

        // The loss report is the contract, so hold it to the file: a hook body
        // the write does NOT name as dropped has to still be there, byte for
        // byte. This is the regression that matters — a form edit turning the
        // author's Tcl into a `-native` placeholder that names nothing.
        let bodies = hook_bodies(&source, &target);
        for statement in &bodies {
            let word = statement.split_whitespace().next().unwrap_or("");
            if write.dropped.iter().any(|d| d == word) {
                continue;
            }
            assert!(
                store.source().contains(statement),
                "{name}: editing {target} silently lost `{word}` — it is not in \
                 the write's `dropped` list either:\n{statement}"
            );
        }
        carried += bodies.len();
    }
    // Every one of the ported packs takes the splice path today. The
    // re-render floor exists for documents the splice cannot verify, not as
    // the ordinary outcome — so hold the line at "all of them".
    assert_eq!(
        spliced,
        examples().len(),
        "the splice path should carry every example; it carried {spliced}"
    );
    // And the ports really do exercise the hard case: several of them declare
    // Tcl hook bodies that no draft can re-render.
    assert!(
        carried >= 3,
        "the examples should exercise hook-body carry-forward; they exercised {carried}"
    );
}

/// The merged view answers for pack commands and shipped commands alike, and
/// says which of the two is live.
#[test]
fn the_resolution_facade_answers_over_a_real_pack() {
    let (_, lsort) = examples()
        .into_iter()
        .find(|(name, _)| name == "lsort.tclspec")
        .expect("the lsort port");
    let store = PackStore::from_source(&lsort);
    let merged = Resolution::new(Builtins::for_dialect("tcl9.0"), &store);

    // `lsort` is shipped, and the port does not claim `-override`, so the
    // shipped spec is what an editor would use — and the studio has to say so
    // rather than quietly showing the pack's.
    let view = merged.view("lsort").expect("a merged view for lsort");
    assert_eq!(view["origin"], json!("shadowed"));
    assert_eq!(view["effective"], view["builtin"]);
    assert_ne!(view["pack"], serde_json::Value::Null);

    let report = merged.validate();
    assert_eq!(report["pack"], json!("tcl"));
    let collisions = report["collisions"].as_array().expect("collisions");
    assert!(
        collisions.iter().any(|c| c["name"] == json!("lsort")),
        "{report}"
    );
    assert_eq!(report["summary"]["shadowed_commands"], json!(1));

    // A name in neither model has no view at all.
    assert!(merged.view("definitely::not::a::command").is_none());
}

/// Canonical rendering keeps the pack's *shape*: same commands, in order,
/// with their `-override` flags intact.
///
/// Field-level equality is not asserted here because it is already the subject
/// of `tests/spectcl_roundtrip.rs`, which owns the renderer's documented gap
/// register and is the one place that allowance should live. What this adds is
/// the part that file cannot see: `-override` is a property of the
/// *declaration*, not of `CommandSpec`, so the renderer has nothing to render
/// it from and [`PackStore::canonical`] has to put it back.
#[test]
fn canonical_rendering_preserves_a_real_packs_shape_and_override_flags() {
    for (name, source) in examples() {
        let store = PackStore::from_source(&source);
        let canonical = PackStore::from_source(store.canonical());
        let before: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
        let after: Vec<&str> = canonical
            .commands()
            .iter()
            .map(|(n, _)| n.as_str())
            .collect();
        assert_eq!(before, after, "{name}: canonical render lost a command");
        assert_eq!(canonical.name(), store.name(), "{name}: pack renamed");
        for (command, _) in store.commands() {
            assert_eq!(
                canonical.overrides_shipped(command),
                store.overrides_shipped(command),
                "{name}: canonical render changed {command}'s -override"
            );
        }
    }
}

/// Every pack a human has written so far is **canonical** (E-R11), so the
/// studio keeps editing all of them in place.
///
/// This is the guard on [`PackStore::programmed`]'s straight-line check. That
/// check is a comparison between the document's own top-level statements and
/// the registration record the snapshot carries, and a false positive on it
/// would silently reroute an ordinary form edit into a patch pack — turning
/// every test above into a test of the wrong path. Running it over the
/// hand-written examples *and* the shipped `specs/` is what makes it safe to
/// trust.
#[test]
fn every_hand_written_pack_is_canonical() {
    let shipped: Vec<(String, String)> =
        fs::read_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../specs"))
            .expect("the specs directory")
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension().is_some_and(|e| e == "tclspec"))
            .map(|path| {
                let name = path
                    .file_name()
                    .expect("a file name")
                    .to_string_lossy()
                    .into_owned();
                (name, fs::read_to_string(&path).expect("readable"))
            })
            .collect();
    assert!(
        shipped.len() >= 8,
        "expected the shipped packs, got {shipped:?}"
    );

    for (name, source) in examples().into_iter().chain(shipped) {
        let store = PackStore::from_source(&source);
        assert_eq!(
            store.programmed(),
            None,
            "{name}: judged a program, so a form edit would become a patch pack"
        );
        assert_eq!(
            store.patch_source(),
            None,
            "{name}: an unedited pack has no patch"
        );
    }
}
