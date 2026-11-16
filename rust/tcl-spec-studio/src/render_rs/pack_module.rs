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

//! The `commands/<pack>/mod.rs` that declares each command's module, whole or as an addition to an existing pack.

use super::*;

/// How the generated `commands/<pack>/mod.rs` is meant to be applied.
///
/// The directory is the author's to name, and naming a pack the registry
/// already ships is the ordinary case — a contribution to `tcl` or `irules`
/// is a few commands added to hundreds. A whole-file render offered for that
/// directory is a drop-in that silently deletes the rest of the pack, so the
/// two cases are rendered differently and labelled differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleForm {
    /// The directory is new to the registry: the render is the whole file.
    Whole,
    /// The directory is a pack the registry already ships: the render is the
    /// lines to add to the `mod.rs` that is already there.
    Addition,
}

/// Render the `commands/<pack>/mod.rs` that declares each command's module
/// and collects their specs.
///
/// The one file in a pack contribution that is pure bookkeeping: a `mod` line
/// per command and a `vec![…]` naming each `spec()`. `command-registry.md`
/// tells a contributor to write both by hand, and the module stem has to match
/// the path [`suggested_path`] chose — two places to get wrong for no
/// judgement gained.
///
/// It carries `#![allow(non_snake_case)]` for the same reason every
/// hand-written pack does: [`suggested_path`] spells a command's `::` as the
/// registry's double underscore, so `HTTP::header` is `mod http__header` and
/// rustc's lint rejects the consecutive underscores.
///
/// The import is `crate::spec::CommandSpec`: the file is written *into*
/// `tcl-registry`, which does not alias itself, so naming the crate would not
/// resolve.
#[must_use]
pub fn render_pack_module(commands: &[(String, Draft)], pack: &str, form: ModuleForm) -> String {
    let stems: Vec<String> = commands.iter().map(|(name, _)| module_stem(name)).collect();
    let mut sorted: Vec<&String> = stems.iter().collect();
    sorted.sort_unstable();
    // Two commands whose stems collide own one module line: Rust has no
    // spelling for declaring the same module twice.  Which two they were is
    // not lost — `pack_export` reports the collision beside the files.
    sorted.dedup();

    if form == ModuleForm::Addition {
        return render_module_addition(&sorted, pack);
    }

    let mut out = String::from(COPYRIGHT_BANNER);
    let _ = write!(
        out,
        "\n\n//! `{pack}` command specifications.\n\n#![allow(non_snake_case)]\n\n"
    );
    for stem in &sorted {
        let _ = writeln!(out, "mod {stem};");
    }
    out.push_str("\nuse crate::spec::CommandSpec;\n");
    let collector = collector_fn(pack);
    let _ = write!(
        out,
        "\n/// Every command specification the `{pack}` pack declares.\n#[must_use]\npub fn {collector}() -> Vec<CommandSpec> {{\n    vec![\n"
    );
    for stem in &sorted {
        let _ = writeln!(out, "        {stem}::spec(),");
    }
    out.push_str("    ]\n}\n");
    out
}

/// The `mod.rs` rendered as an **addition** to a pack the registry already
/// ships: the `mod` lines and collector entries to put into the file that is
/// already there.
///
/// A whole-file render would be a drop-in that deletes every command the pack
/// already has, so this one is not a file at all — no copyright banner, no
/// collector body, and a banner comment that says which it is before anything
/// else on the page.
fn render_module_addition(stems: &[&String], pack: &str) -> String {
    let dir = format!("rust/tcl-registry/src/commands/{pack}/mod.rs");
    let collector = collector_fn(pack);
    let mut out = String::new();
    out.push_str("// ══════════════════════════════════════════════════════════════════\n");
    out.push_str("// ADDITIONS — NOT A DROP-IN FILE. DO NOT REPLACE mod.rs WITH THIS.\n");
    out.push_str("// ══════════════════════════════════════════════════════════════════\n");
    out.push_str("//\n");
    let _ = writeln!(out, "// `{pack}` is a pack tcl-registry already ships, and");
    let _ = writeln!(out, "// {dir}");
    out.push_str("// already declares every other command in it. Writing this file over\n");
    out.push_str("// that one would delete every last one of them. Merge the two lists\n");
    out.push_str("// below into the file instead, each in the order that file keeps.\n");
    if stems.iter().any(|stem| stem.contains("__")) {
        out.push_str("//\n");
        out.push_str("// A stem below spells a `::` as a double underscore, so the file needs\n");
        out.push_str("// `#![allow(non_snake_case)]` too if it does not carry one already.\n");
    }
    out.push_str("\n// Add beside the existing `mod` declarations:\n");
    for stem in stems {
        let _ = writeln!(out, "mod {stem};");
    }
    out.push('\n');
    let _ = writeln!(
        out,
        "// Add to the list `{collector}()` returns — a shipped pack may build"
    );
    out.push_str("// that from sub-builders or a `const` table, so the rows go wherever\n");
    out.push_str("// it keeps the rest of them:\n");
    for stem in stems {
        let _ = writeln!(out, "    {stem}::spec(),");
    }
    out
}

/// The name of the collector function a pack directory declares.
///
/// The directory is free text a user typed, and `commands/my-lib/` would give
/// `my-lib_command_specs`, which is not an identifier. Every non-alphanumeric
/// run becomes one underscore, exactly as a module stem does.
fn collector_fn(pack: &str) -> String {
    let mut ident = String::new();
    for ch in pack.chars() {
        if ch.is_alphanumeric() {
            ident.extend(ch.to_lowercase());
        } else if !ident.ends_with('_') {
            ident.push('_');
        }
    }
    let ident = ident.trim_matches('_');
    match ident.chars().next() {
        None => "pack_command_specs".to_owned(),
        Some(first) if first.is_ascii_digit() => format!("pack_{ident}_command_specs"),
        Some(_) => format!("{ident}_command_specs"),
    }
}

/// The module name [`suggested_path`] files a command under.
fn module_stem(name: &str) -> String {
    suggested_path(name, "x")
        .rsplit('/')
        .next()
        .and_then(|file| file.strip_suffix(".rs"))
        .unwrap_or("command")
        .to_owned()
}
