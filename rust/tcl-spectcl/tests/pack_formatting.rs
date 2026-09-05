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

//! Formatting a `.tclspec` must not change the pack it loads.
//!
//! A pack file is a Tcl script, so `textDocument/formatting` on one runs the
//! ordinary Tcl formatter — and much of what a pack says is **byte-significant
//! prose**. `description`, `summary`, `snippet` and `examples` are every byte
//! between their braces, verbatim: the loader never reflows, dedents or joins
//! one, because that is what lets a ported field compare byte for byte against
//! the `&'static str` it came from. A formatter that rewrapped one would
//! silently change what the editor shows for the command the pack describes,
//! and nothing else in the tree would notice.
//!
//! So this is a round trip over the **shipped example packs**
//! (`docs/design/spec-dsl-examples/`, the frozen syntax's own worked
//! examples): format each one, load both spellings, and require the loaded
//! packs to agree.
//!
//! Two differences are legitimate and are normalised away rather than
//! asserted against:
//!
//! * **Declaration lines move.** The formatter removes a blank line and
//!   re-indents; a declaration's recorded `line` follows the text it is on.
//!   That is the formatter doing its job, and every consumer of a line reads
//!   it against the same buffer.
//! * **A hook body is laid out.** A hook body is ordinary Tcl held as text
//!   and run in the sandbox, so laying it out — re-indenting it under its new
//!   column, splitting `a ; b`, moving a long word onto a continuation line —
//!   is exactly what the formatter is for, and none of it changes what the
//!   body does. The body *text* is therefore exempt; that a hook is still
//!   declared, on the same field, with the same parameter list, is not.
//!
//! Everything else — every property, every trait, every prose field, every
//! arity — must survive untouched.
//!
//! registry-metadata: `SpecTcl` is our own DSL, so its own worked examples
//! are the oracle here, not C Tcl.

use std::path::{Path, PathBuf};

use tcl_spectcl::discovery::{Origin, PackFile, Tier};

/// The frozen syntax's worked examples — the richest packs in the tree, and
/// the ones whose prose fields a rewrap would visibly damage.
fn example_packs() -> Vec<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root")
        .join("docs/design/spec-dsl-examples");
    let mut packs: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "tclspec"))
        .collect();
    packs.sort();
    assert!(
        !packs.is_empty(),
        "no example packs under {}",
        dir.display()
    );
    packs
}

/// The pack `source` loads to, rendered so two loads can be compared.
fn loaded(path: &Path, source: &str) -> Vec<String> {
    let set = tcl_spectcl::pack::load_in_memory(vec![(
        PackFile {
            tier: Tier::Workspace,
            path: path.to_path_buf(),
            origin: Origin::DotDir,
        },
        source.to_owned(),
    )]);
    assert!(!set.packs.is_empty(), "{} loaded no pack", path.display());
    normalise(&format!("{:#?}", set.packs))
}

/// Erase the two differences formatting is *allowed* to make (see the module
/// docs): a declaration's line number, and the whitespace inside a hook body.
fn normalise(dump: &str) -> Vec<String> {
    dump.lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if let Some(rest) = trimmed.strip_prefix("line: ") {
                return format!(
                    "line: <moved>{}",
                    rest.trim_matches(|c: char| c.is_ascii_digit())
                );
            }
            if trimmed.starts_with("body: \"") {
                return "body: <laid out>".to_owned();
            }
            trimmed.to_owned()
        })
        .collect()
}

#[test]
fn formatting_a_pack_does_not_change_the_pack_it_loads() {
    let registry = tcl_lsp_core::registry_for_dialect("spectcl");
    for path in example_packs() {
        let source = std::fs::read_to_string(&path).expect("example pack");
        let formatted = tcl_lsp_core::formatting::format_tcl(
            &source,
            &tcl_lsp_core::formatting::FormatterConfig::default(),
            registry,
        );
        let before = loaded(&path, &source);
        let after = loaded(&path, &formatted);
        // A whole `Pack` dump is thousands of lines wide; report the first
        // field that moved rather than both dumps.
        let first_difference = before
            .iter()
            .zip(&after)
            .enumerate()
            .find(|(_, (a, b))| a != b)
            .map(|(at, (a, b))| format!("field {at}:\n  before: {a}\n  after:  {b}"));
        assert!(
            first_difference.is_none() && before.len() == after.len(),
            "formatting changed the pack {} loads — {}",
            path.display(),
            first_difference.unwrap_or_else(|| format!(
                "{} fields before, {} after",
                before.len(),
                after.len()
            )),
        );
    }
}

/// The narrower claim the one above rests on, stated where a failure names it:
/// a braced prose word comes out of the formatter byte for byte.
///
/// The formatter *may* move such a word onto its own continuation line — a
/// backslash-newline is a word separator, so the word it introduces is
/// unchanged — but it may never reflow, re-indent or rewrap what is inside the
/// braces.
#[test]
fn formatting_leaves_a_packs_prose_byte_for_byte() {
    const PROSE: &str = "Two  spaces, a line break,\n    and a deliberately indented line.";
    let source = format!(
        "speclib mylib 1 {{\n    command mylib::x {{\n        arity 1\n        \
         hover {{\n            summary {{One line.}}\n            \
         description {{{PROSE}}}\n        }}\n    }}\n}}\n"
    );
    let formatted = tcl_lsp_core::formatting::format_tcl(
        &source,
        &tcl_lsp_core::formatting::FormatterConfig::default(),
        tcl_lsp_core::registry_for_dialect("spectcl"),
    );
    assert!(
        formatted.contains(PROSE),
        "the prose was rewritten:\n{formatted}"
    );
}
