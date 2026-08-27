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

//! Render a draft as a `SpecTcl` `.tclspec` pack — the inverse of the loader.
//!
//! The loader ([`crate::spectcl`]) turns a pack into live `CommandSpec`s, which
//! the studio seeds drafts from. This module closes the loop: draft in,
//! `.tclspec` text out, so a shipped spec can be *exported* to a pack, a pack
//! can be round-tripped through the studio's editor, and the studio's DSL pane
//! has something to show. The syntax is the frozen one in
//! `docs/design/spec-dsl-examples/README.md`, and the eleven `*.tclspec` ports
//! beside it are the formatting exemplars: one-screen simple commands, row
//! statements rather than nested blocks, `\`-continued option rows with the
//! prose flag last.
//!
//! ## Three rules the renderer follows
//!
//! 1. **Only non-default fields are emitted.** Every key is compared against
//!    [`crate::draft::default_command_draft`] /
//!    [`crate::draft::default_subcommand_draft`], so a rendered pack says
//!    exactly what its author set and nothing else — the same promise
//!    [`crate::render_rs`] makes with `..CommandSpec::DEFAULT`.
//! 2. **Catalogue spellings are verbatim.** A draft already holds the Rust
//!    variant spelling for every enum and flag (`VarWrite`, `BYTE_COMPILED`,
//!    `tcl9.0`), which is exactly what the DSL wants, so no field is
//!    re-spelled on the way out.
//! 3. **Nothing is dropped silently.** A field the draft holds but the DSL
//!    cannot carry is emitted as a `# TODO(spectcl):` line naming the field,
//!    its documented spelling, and why the value did not survive — see
//!    [`GAPS`]. A field whose value *names engine code* is emitted in the
//!    documented `-native ID` form, which is not a loss: the loader installs
//!    its family's abstention either way, and the draft records both the same.
//!
//! ## What round-trips
//!
//! `tests/spectcl_roundtrip.rs` is the gate: every command of every browsable
//! dialect is drafted, rendered here, loaded back through
//! [`crate::spectcl::load_pack`], and the two drafts are compared field by
//! field. Every field that legitimately differs is one of the [`GAPS`] below,
//! or a [`Loss`] this render reported for itself, and the test fails on
//! anything else.
//!
//! Two differences are neither, and are worth naming here because they are
//! properties of the *language* rather than of this renderer:
//!
//! - **`arg N -appends …` implies `-role CommandPrefix`.** So a spec that
//!   declares a command-prefix position with *no* role at that index cannot be
//!   said in `SpecTcl`: reloading always finds the implied role. 531 of the
//!   registry's command/dialect pairs are in that shape.
//! - **A quoted word is not verbatim.** The loader reads its words off the
//!   CST, where a braced word is every byte between the braces but a quoted
//!   one keeps its backslashes and has `$x` normalised to `${x}`. Prose with
//!   an unbalanced brace or a backslash-newline therefore has no faithful
//!   spelling at all, and is reported as a [`Loss`] rather than written down
//!   wrong — see [`word`].

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde_json::Value;

use crate::draft::{self, Draft, OPTION_DEPRECATION_FIX_HOOK_KEY, SOURCE_DIALECT_KEY};

/// The DSL **vocabulary** version a rendered pack declares — the word after
/// the pack name in `speclib <pack> <version> { … }`.
///
/// The rule is "declare the newest vocabulary the *body* actually uses": a
/// header naming an older vocabulary than the body needs is exactly the
/// inconsistency the loader's per-site notice reports, and pinning this to a
/// stale literal made the renderer produce it (#1627).
///
/// It parted company with [`tcl_spectcl::NEWEST_VOCABULARY_VERSION`] for one
/// release, and is back on it. The pin at `1.2` said: 2.0's additions are
/// `available`, the `environment` block, and the `dialect` block, the
/// renderer emitted none of them, and a 2.0 header over an entirely 1.x body
/// would be a failed upgrade wearing a 2.0 header — so the constant sat at
/// the newest vocabulary the renderer could actually *reach*, and would move
/// when the renderer learned the 2.0 spellings.
///
/// **It has learned the one that applies to it.** [`availability_rows`]
/// writes every dialect set the algebra can carry as `available` /
/// `-available` — the 2.0 word — at every scope the loader reads it:
/// command, subcommand, option, form, side effect, option conflict, and
/// second-level operation. A rendered body is therefore 2.0 wherever a
/// draft says anything about availability at all, which is nearly every
/// command in the registry, and pinning the header at 1.2 would now
/// reproduce #1627 exactly — the loader's per-site notice on a `available`
/// row under a 1.2 declaration.
///
/// The other two 2.0 additions are **pack-level blocks a draft cannot
/// hold**: a draft is one command's model, so nothing in it renders as an
/// `environment` or a `dialect` block, and waiting for those would pin this
/// constant forever. What remains of the old worry is a draft that says
/// nothing about availability, whose body is 1.x under a 2.0 header. That is
/// the harmless direction: over-declaring produces no notice anywhere (the
/// loader only reports a *site newer than* the declaration), and a pack the
/// renderer just wrote has no legacy rows left untranslated for U1 to fail
/// on — it has fields, not rows.
pub const DSL_VERSION: &str = tcl_spectcl::NEWEST_VOCABULARY_VERSION;

/// Column the renderer tries to keep rows inside before continuing a row with
/// a `\`, matching the ports' own wrapping.
const WRAP_COLUMN: usize = 92;

/// Column the one-line `subcommand NAME { … }` form is allowed to reach.
///
/// Wider than [`WRAP_COLUMN`] because that is what the ports do:
/// `irules-http-header.tclspec` writes fourteen subcommands as one line each,
/// the longest of them 143 columns, and breaking those into five-line blocks
/// would cost the file its one-screen shape for nothing.
const ONE_LINE_COLUMN: usize = 144;

// ---------------------------------------------------------------------------
// Why a field did not survive
// ---------------------------------------------------------------------------

/// Why a draft key cannot be written as `SpecTcl`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    /// The DSL has a spelling and the loader reads it, but the **draft** holds
    /// no value to write: seeding records the field as
    /// [`UNRENDERABLE`](crate::draft::UNRENDERABLE_KEY) — "set, but the
    /// defining expression could not be recovered" — because the spec field is
    /// a function pointer or a reference to a named `&'static` descriptor.
    DraftOpaque,
    /// The draft holds the value and the design memo documents a spelling, but
    /// the loader has no reader for that property word yet, so writing it
    /// would only produce an unknown-property notice.
    LoaderGap,
    /// The design excludes the field from what a pack may author at all.
    Excluded,
}

impl GapKind {
    /// One clause explaining this kind, used in the `TODO` line.
    const fn why(self) -> &'static str {
        match self {
            Self::DraftOpaque => {
                "a draft records only that the field is set, never the expression that set it"
            }
            Self::LoaderGap => "the loader has no reader for that property word yet",
            Self::Excluded => "a pack may not author this field",
        }
    }
}

/// One field the renderer cannot carry, with its documented DSL spelling.
#[derive(Debug, Clone, Copy)]
pub struct Gap {
    /// The draft / schema key.
    pub key: &'static str,
    /// How `docs/design/spec-dsl-examples/README.md`'s coverage matrix spells
    /// the field, or `""` where it spells none.
    pub spelling: &'static str,
    /// Why the value did not survive.
    pub kind: GapKind,
}

/// Every field a draft can hold that a rendered pack cannot carry.
///
/// This is the renderer's half of the round-trip contract: the gate in
/// `tests/spectcl_roundtrip.rs` allows a rendered-then-reloaded draft to differ
/// from its source **only** on these keys, and reports any other difference as
/// a failure.
pub const GAPS: &[Gap] = &[
    // --- the value is not in the draft ------------------------------------
    //
    // `object_class` left this bucket. It looked like the others — an
    // `Option<&'static …>` reference — but the descriptor behind it is plain
    // data all the way down, and its method table *is* `&[SubCommand]`, so
    // seeding now carries the whole thing and the renderer writes it as
    // `object_class NAME ?-superclass {…}? ?-allow-unknown? { method … }`.
    // What still lives here is genuinely opaque: a function pointer, or a
    // reference to a shared registry constant a pack can only name.
    Gap {
        key: "frame_effect",
        spelling: "frame_effect -level-word W -layout L",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "semantic_operation",
        spelling: "semantic_operation Invoke|{Intrinsic ID}|{StructuredLowering ID}",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "world_effects",
        spelling: "world_effects none|NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "state_transitions",
        spelling: "state_transitions NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_requires",
        spelling: "event_requires NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "case_list",
        spelling: "case_list NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "definition_body",
        spelling: "definition_body NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "body_scope",
        spelling: "body_scope NAME|{ … }",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "bpf_op",
        spelling: "bpf_op -native ID",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "data_collection",
        spelling: "data_collection -native ID",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_requirement_forms",
        spelling: "event_requirement_form {word …} ?-only-in {E …}? ?{ … }?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "side_switch_target",
        spelling: "side_switch_target Client|Server",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "event_handler_priority",
        spelling: "event_handler_priority -default N ?-warn-implicit?",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "irules_top_level_effect",
        spelling: "irules_top_level_effect Priority|Timing",
        kind: GapKind::DraftOpaque,
    },
    Gap {
        key: "result_stability",
        spelling: "result_stability Unknown|ReferentiallyTransparent|Volatile|{ReadsVersionedWorld {D …}}",
        kind: GapKind::DraftOpaque,
    },
    // --- the draft has it; the loader does not read it yet -----------------
    //
    // Empty. Eleven keys left this bucket as the loader grew their readers:
    // the three `{VARIANT payload …}` element-structure facts,
    // `default_form_first_word`, `setter_constraints`, `format_string_type`,
    // `deprecation_fix`, `byte_array_payload`, `defines_symbol`,
    // `byte_array_effect`'s payload-carrying `Rebinarifies` variant, and — with
    // vocabulary 1.1 — the option row's `-deprecation-fix {…}`. What a pack
    // still cannot say is above (no value in the draft) or below (excluded by
    // design), not here.
    // --- excluded by design -------------------------------------------------
    Gap {
        key: "completion",
        spelling: "",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "dispatch_dependencies",
        spelling: "",
        kind: GapKind::Excluded,
    },
    // This is a native resolver over a command's own OptionSpec table. Packs
    // can describe the table and static pattern facts, but not arbitrary
    // option-selected language dispatch yet; keeping it excluded makes the
    // native-only boundary explicit until a declarative selector exists.
    Gap {
        key: "pattern_arg_resolver",
        spelling: "",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "command_forms",
        spelling: "",
        kind: GapKind::Excluded,
    },
    Gap {
        key: "subcommand_forms",
        spelling: "",
        kind: GapKind::Excluded,
    },
];

/// The [`Gap`] for `key`, if the renderer cannot carry it.
#[must_use]
pub fn gap(key: &str) -> Option<&'static Gap> {
    GAPS.iter().find(|gap| gap.key == key)
}

// ---------------------------------------------------------------------------
// Tcl words
// ---------------------------------------------------------------------------

/// Whether `text` can be written as a bare word.
fn bare_safe(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('#')
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric() || "-_.:/+@=,<>*?!%^~|'`".contains(c) && c.is_ascii()
        })
}

/// Whether `{text}` is a well-formed braced word whose value is `text` byte
/// for byte.
///
/// The scan is Tcl's own: a backslash escapes the next character (so `\{` and
/// `\}` do not nest), the depth may never go negative, and a trailing
/// backslash would escape the closing brace. Anything else has to take the
/// quoted form.
fn brace_safe(text: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Backslash-newline is the one substitution Tcl still performs
                // inside braces, so a word carrying one does not come back out
                // byte for byte.
                match chars.next() {
                    None | Some('\n') => return false,
                    Some(_) => {}
                }
            }
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth < 0 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

/// One word carrying `text` verbatim, or `None` when no spelling does.
///
/// **There is no quoted fallback, because a quoted word is not verbatim.** The
/// loader reads its words off the CST, where a quoted word keeps its
/// backslashes (`\[` stays `\[`) and has its variable references normalised
/// (`$c` becomes `${c}`), while a braced word really is every byte between the
/// braces. So a bare or braced word is faithful and a quoted one is not, and
/// text that can be neither is reported as a loss rather than written down
/// wrong. In practice that is a handful of registry strings carrying a lone
/// `{` from a man-page extractor.
fn word(text: &str) -> Option<String> {
    if bare_safe(text) {
        Some(text.to_owned())
    } else if brace_safe(text) {
        Some(format!("{{{text}}}"))
    } else {
        None
    }
}

/// A braced word carrying `text` verbatim — the prose and block spelling.
///
/// Unlike [`word`] this never degrades to a bare word, because a `detail` or
/// `description` is braced in every port whether or not it needs to be.
fn braced(text: &str) -> Option<String> {
    brace_safe(text).then(|| format!("{{{text}}}"))
}

/// A Tcl list word holding `items`, each element carrying its text verbatim.
///
/// A one-element list is written as the bare element — `dialects all-tcl`,
/// `-dialects tcl8.6+` — which is how every port writes one, and which
/// `split_list_lenient` reads back as the same single-element list. The braces
/// stay whenever the element needs them, since a braced element inside a bare
/// word would split into several.
fn list_word<S: AsRef<str>>(items: &[S]) -> Option<String> {
    if let [only] = items
        && bare_safe(only.as_ref())
    {
        return Some(only.as_ref().to_owned());
    }
    let words: Option<Vec<String>> = items.iter().map(|item| word(item.as_ref())).collect();
    braced(&words?.join(" "))
}

/// The elements of a JSON string array as a list word.
fn str_list_word(value: &Value) -> Option<String> {
    let items: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    list_word(&items)
}

/// The `SpecTcl` 2.0 availability algebra, as the rows of an `available`
/// word — the canonical spelling of what 1.x said with `dialects`.
///
/// `docs/design/dialect-and-package-registry-redesign.md` §6.2 gives every
/// scope that accepts `dialects` a second spelling, and the loader projects
/// the two onto exactly the same fields (`tcl_spectcl::loader::available`),
/// so this is a **rewording, not a re-meaning**: the round trip cannot tell
/// which word carried the set, and the round-trip gate proves it command by
/// command.
///
/// `None` when the set has no faithful spelling in the algebra, and the
/// caller keeps `dialects`. Two cases, both real:
///
/// - **A bit with no provider.** `f5-iapps`, `expect`, `bpf`, `f5-tmsh`,
///   `f5-bigip`, and `spectcl` are dialect bits the algebra's closed
///   provider list (`tcl`, `f5-irules`, `jim`, `package NAME`) does not
///   name. §6.1 keeps `dialects` loading forever precisely so a set the new
///   word cannot say is still sayable.
/// - **The empty set.** `dialects {}` is *declared unavailable*, and
///   `available` with no rows is an error rather than an empty set.
fn availability_rows(members: &[&str]) -> Option<Vec<String>> {
    if members.is_empty() {
        return None;
    }
    let mut rows: Vec<String> = Vec::new();
    let versions: Vec<usize> = TCL_LADDER
        .iter()
        .enumerate()
        .filter(|(_, version)| members.contains(*version))
        .map(|(index, _)| index)
        .collect();
    // One row per contiguous run of releases, so a set the ladder covers to
    // its end is written open (`{tcl 8.6-}`) exactly as `tcl8.6+` is.
    let mut at = 0;
    while at < versions.len() {
        let mut end = at;
        while end + 1 < versions.len() && versions[end + 1] == versions[end] + 1 {
            end += 1;
        }
        let low = release_of(versions[at]);
        rows.push(if at == end {
            // A bare `X.Y` names that release line only, which is exactly
            // what one `tclX.Y` bit means — an open window would claim a
            // release the set does not hold.
            format!("tcl {low}")
        } else if versions[end] + 1 == TCL_LADDER.len() {
            // A run that reaches the newest release is written open, the way
            // `tclX.Y+` is: both track the ladder as it grows.
            format!("tcl {low}-")
        } else {
            // **The upper bound is exclusive**, as it is in every Tcl version
            // requirement (`package require`'s own `min-max` reads
            // `min <= v < max`), so a closed run names the release *after*
            // its last one: {8.4, 8.5, 8.6} is `tcl 8.4-9.0`.
            format!("tcl {low}-{}", release_of(versions[end] + 1))
        });
        at = end + 1;
    }
    for member in members {
        if TCL_LADDER.contains(member) {
            continue;
        }
        match *member {
            "f5-irules" => rows.push("f5-irules".to_owned()),
            // The exact inverse of `tcl spec upgrade`'s U2 `tk` →
            // `{package Tk}`, and the loader's `PACKAGE_DIALECT_BITS` maps it
            // straight back onto the same bit.
            "tk" => rows.push("package Tk".to_owned()),
            _ => return None,
        }
    }
    Some(rows)
}

/// The release word of a ladder index, without its `tcl` prefix.
fn release_of(index: usize) -> &'static str {
    TCL_LADDER
        .get(index)
        .and_then(|version| version.strip_prefix("tcl"))
        .unwrap_or_default()
}

/// The `available …` statement's words, or `None` to keep `dialects`.
fn available_statement(value: &Value) -> Option<String> {
    let members: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    let rows = availability_rows(&members)?;
    Some(
        rows.iter()
            .map(|row| format!("{{{row}}}"))
            .collect::<Vec<_>>()
            .join(" "),
    )
}

/// The `-available VALUE` flag's value, or `None` to keep `-dialects`.
///
/// One row is written bare (`-available {tcl 8.6-}`) and several are wrapped
/// into one list word, which is exactly how the loader's `from_flag` tells
/// the two apart.
fn available_flag(members: &[&str]) -> Option<String> {
    let rows = availability_rows(members)?;
    Some(match rows.as_slice() {
        [only] => format!("{{{only}}}"),
        many => format!(
            "{{{}}}",
            many.iter()
                .map(|row| format!("{{{row}}}"))
                .collect::<Vec<_>>()
                .join(" ")
        ),
    })
}

/// [`available_flag`] over a JSON member array.
fn available_flag_of(value: &Value) -> Option<String> {
    let members: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    available_flag(&members)
}

/// One availability flag on a row: the 2.0 `-available` spelling when the
/// algebra carries the set, and the legacy `-dialects` when it does not.
fn push_availability_flag(row: &mut Vec<String>, lost: &mut bool, value: &Value, algebra: bool) {
    if let Some(spelling) = algebra.then(|| available_flag_of(value)).flatten() {
        push_flag(row, lost, "-available", Some(spelling));
    } else {
        push_flag(row, lost, "-dialects", dialect_set_word(value));
    }
}

/// The five Tcl releases, in ladder order — the members the DSL's dialect-set
/// shorthands close over.
const TCL_LADDER: &[&str] = &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"];

/// A dialect set written the way the ports write one: with the `all-tcl`,
/// `tcl8.x`, and `tclX.Y+` shorthands rather than five version bits.
///
/// `README.md` adds the shorthands "because writing five version bits per
/// option is unreadable", and every port uses them — `{all-tcl f5-irules}`,
/// `tcl8.6+`. They are exact: `parse_dialects` unions the same bits either
/// way, so the round trip does not notice, but the file reads like a
/// hand-written one.
fn dialect_set_word(value: &Value) -> Option<String> {
    let members: Vec<&str> = as_array(value).iter().filter_map(Value::as_str).collect();
    dialect_words(&members)
}

/// [`dialect_set_word`] over member names already in hand.
fn dialect_words(members: &[&str]) -> Option<String> {
    let versions: Vec<&str> = TCL_LADDER
        .iter()
        .copied()
        .filter(|version| members.contains(version))
        .collect();
    let mut words: Vec<String> = Vec::new();
    if versions.len() == TCL_LADDER.len() {
        words.push("all-tcl".to_owned());
    } else if versions == TCL_LADDER[..3] {
        words.push("tcl8.x".to_owned());
    } else if versions.len() > 1
        && TCL_LADDER.ends_with(versions.as_slice())
        && let Some(first) = versions.first()
    {
        words.push(format!("{first}+"));
    } else {
        words.extend(versions.iter().map(|version| (*version).to_owned()));
    }
    words.extend(
        members
            .iter()
            .filter(|member| !TCL_LADDER.contains(member))
            .map(|member| (*member).to_owned()),
    );
    list_word(&words)
}

/// A declaration's own name word — a command, subcommand, or option name.
///
/// Unlike prose these are identifiers in every shipped spec, so the braced
/// form always works; a pathological name that even braces cannot hold would
/// break the pack loudly rather than lose a field silently.
fn name_word(text: &str) -> String {
    word(text).unwrap_or_else(|| format!("{{{text}}}"))
}

/// The spelling a `speclib` / `command` header uses for `name`.
///
/// Published for [`crate::store`], which rebuilds a header line byte for byte
/// when it puts an `-override` flag back on a re-rendered command — the flag
/// lives on the *declaration*, not on `CommandSpec`, so the renderer has
/// nothing to render it from and the store has to add it afterwards.
#[must_use]
pub fn header_word(name: &str) -> String {
    name_word(name)
}

/// Push one word onto a row, remembering when it had no faithful spelling.
fn push_word(row: &mut Vec<String>, lost: &mut bool, spelling: Option<String>) {
    match spelling {
        Some(spelling) => row.push(spelling),
        None => *lost = true,
    }
}

/// Push `flag value` onto a row, remembering when the value had no faithful
/// spelling.
fn push_flag(row: &mut Vec<String>, lost: &mut bool, flag: &str, spelling: Option<String>) {
    match spelling {
        Some(spelling) => {
            row.push(flag.to_owned());
            row.push(spelling);
        }
        None => *lost = true,
    }
}

/// Push the three lifecycle flags a gateable row's draft object carries, in
/// the order `README.md` documents them.
///
/// Every level the registry can gate spells its lifecycle the same way — the
/// option row's flags, unchanged, reused by `form`, `side_effect`,
/// `option_conflict`, `sub_subcommand` and a `values` table row.
fn push_lifecycle_flags(row: &mut Vec<String>, lost: &mut bool, entry: &Value) {
    for (key, flag) in [
        ("introduced_version", "-introduced"),
        ("deprecated_version", "-deprecated"),
        ("retired_version", "-retired"),
    ] {
        if let Some(version) = entry[key].as_str() {
            push_flag(row, lost, flag, word(version));
        }
    }
}

/// Whether a row's draft object declares any lifecycle release at all.
fn has_lifecycle(entry: &Value) -> bool {
    [
        "introduced_version",
        "deprecated_version",
        "retired_version",
    ]
    .iter()
    .any(|key| entry[key].is_string())
}

/// Write a row, or report the field when one of its words had no faithful
/// spelling.
fn emit_row(out: &mut Out, key: &str, row: &[String], lost: bool, break_before: &str) {
    if lost {
        unwritable(out, key);
    } else {
        out.row(row, break_before);
    }
}

fn as_array(value: &Value) -> &[Value] {
    value.as_array().map_or(&[], Vec::as_slice)
}

// ---------------------------------------------------------------------------
// The output buffer
// ---------------------------------------------------------------------------

/// An indented line buffer with a one-slot blank-line separator.
///
/// **Only the first physical line of a statement is indented.** A braced word
/// carrying prose may run over many lines, and the loader takes every byte
/// between its braces verbatim — so indenting a continuation line would change
/// the value, and the equivalence gate compares those strings byte for byte.
/// That is also why a block body is rendered *at its final indent* rather than
/// rendered flat and shifted afterwards.
#[derive(Debug, Default)]
struct Out {
    text: String,
    indent: usize,
    /// A blank line is owed before the next line, unless nothing follows.
    pending_blank: bool,
    /// Every statement written, unindented — what the one-line subcommand
    /// form is decided from.
    statements: Vec<String>,
    /// Every field this buffer could not carry, in the order they arose.
    losses: Vec<Loss>,
}

/// One field a render could not carry, and why.
///
/// Returned by [`render_pack_reporting`] so a caller — the studio's DSL pane,
/// or the round-trip gate — can say what a pack does *not* say without having
/// to re-derive it from the `TODO` comments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    /// The draft / schema key.
    pub key: String,
    /// One sentence naming the cause.
    pub reason: String,
}

impl Out {
    /// A buffer whose statements start at `indent` levels of four spaces.
    fn at(indent: usize) -> Self {
        Self {
            indent,
            ..Self::default()
        }
    }

    fn line(&mut self, text: &str) {
        if self.pending_blank && !self.text.is_empty() {
            self.text.push('\n');
        }
        self.pending_blank = false;
        if !text.is_empty() {
            for _ in 0..self.indent {
                self.text.push_str("    ");
            }
            self.text.push_str(text);
            self.statements.push(text.to_owned());
        }
        self.text.push('\n');
    }

    /// Ask for one blank line before whatever comes next.
    fn gap(&mut self) {
        if !self.text.is_empty() {
            self.pending_blank = true;
        }
    }

    /// Append an already-indented block, keeping its own statement list.
    fn block(&mut self, other: &Self) {
        if self.pending_blank && !self.text.is_empty() {
            self.text.push('\n');
        }
        self.pending_blank = false;
        self.text.push_str(&other.text);
        self.statements.extend(other.statements.iter().cloned());
        self.losses.extend(other.losses.iter().cloned());
    }

    fn indented<T>(&mut self, body: impl FnOnce(&mut Self) -> T) -> T {
        self.indent += 1;
        let out = body(self);
        self.indent -= 1;
        out
    }

    /// A `# …` comment, wrapped to a readable width.
    fn comment(&mut self, text: &str) {
        let width = 78usize.saturating_sub(self.indent * 4);
        let mut line = String::new();
        for token in text.split_whitespace() {
            if !line.is_empty() && line.len() + 1 + token.len() > width {
                let done = std::mem::take(&mut line);
                self.line(&format!("# {done}"));
                line.push_str("  ");
            }
            if !line.is_empty() && !line.ends_with("  ") {
                line.push(' ');
            }
            line.push_str(token);
        }
        if !line.is_empty() {
            self.line(&format!("# {line}"));
        }
    }

    /// One statement written as `word word …`, continued with `\` when it runs
    /// past [`WRAP_COLUMN`].
    ///
    /// `break_before` names the flag a long row is split in front of — the
    /// ports put the prose flag on its own continuation line and everything
    /// else on the first.
    fn row(&mut self, words: &[String], break_before: &str) {
        let flat = words.join(" ");
        let split = if self.indent * 4 + flat.len() <= WRAP_COLUMN || break_before.is_empty() {
            None
        } else {
            words
                .iter()
                .position(|w| w == break_before)
                .filter(|at| *at > 0)
        };
        let Some(at) = split else {
            self.line(&flat);
            return;
        };
        self.line(&format!("{} \\", words[..at].join(" ")));
        self.indented(|out| out.line(&words[at..].join(" ")));
        // One logical statement, however many physical lines it took.
        self.statements.truncate(self.statements.len() - 2);
        self.statements.push(flat);
    }
}

// ---------------------------------------------------------------------------
// Shared `values` tables
// ---------------------------------------------------------------------------

/// The pack-level `values NAME { … }` tables a render needs.
///
/// An `arg`/`option` row can spell a plain value set inline (`-values {a b}`),
/// but a value carrying a `detail`, a Tcl-version floor, or a completion code
/// has nowhere to put them on the row — so those become a shared table,
/// exactly as `string.tclspec`'s `is-classes` and `return.tclspec`'s
/// `return-codes` do.
#[derive(Debug, Default)]
struct ValueTables {
    /// `(name, values array)`, in declaration order.
    tables: Vec<(String, Value)>,
}

impl ValueTables {
    /// Register `values` under a name derived from `hint`, reusing an
    /// identical table when one is already declared.
    fn intern(&mut self, hint: &str, values: &Value) -> String {
        if let Some((name, _)) = self.tables.iter().find(|(_, table)| table == values) {
            return name.clone();
        }
        let base = slug(hint);
        let mut name = base.clone();
        let mut n = 2;
        while self.tables.iter().any(|(taken, _)| *taken == name) {
            name = format!("{base}-{n}");
            n += 1;
        }
        self.tables.push((name.clone(), values.clone()));
        name
    }

    fn render(&self, out: &mut Out) {
        for (name, values) in &self.tables {
            out.gap();
            out.line(&format!("values {name} {{"));
            out.indented(|out| {
                for value in as_array(values) {
                    let Some(spelling) = word(str_of(&value["value"])) else {
                        unwritable(out, "arg_values");
                        continue;
                    };
                    let mut row = vec!["value".to_owned(), spelling];
                    if let Some(version) = value["min_tcl"].as_str() {
                        row.push("-min-tcl".to_owned());
                        row.push(tcl_version_word(version));
                    }
                    let mut lost = false;
                    push_lifecycle_flags(&mut row, &mut lost, value);
                    if lost {
                        unwritable(out, "arg_values");
                    }
                    if let Some(code) = value["code"].as_i64() {
                        row.push("-code".to_owned());
                        row.push(code.to_string());
                    }
                    let detail = str_of(&value["detail"]);
                    if !detail.is_empty() {
                        match braced(detail) {
                            Some(spelling) => {
                                row.push("-detail".to_owned());
                                row.push(spelling);
                            }
                            None => unwritable(out, "arg_values"),
                        }
                    }
                    out.row(&row, "-detail");
                }
            });
            out.line("}");
        }
    }
}

/// A name safe to use as a pack-level declaration word.
fn slug(hint: &str) -> String {
    let mut out = String::with_capacity(hint.len());
    let mut last_dash = false;
    for c in hint.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_end_matches('-').to_owned();
    if trimmed.is_empty() {
        "values".to_owned()
    } else {
        trimmed
    }
}

/// The catalogue spelling of a `min_tcl` floor as the DSL's own version word.
///
/// A draft holds the `TclVersion` variant (`V8_6`); the DSL takes `tcl8.6`,
/// which is also what the shipped table's own key is.
fn tcl_version_word(variant: &str) -> String {
    let digits: String = variant
        .trim_start_matches('V')
        .chars()
        .map(|c| if c == '_' { '.' } else { c })
        .collect();
    format!("tcl{digits}")
}

fn str_of(value: &Value) -> &str {
    value.as_str().unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Rust-expression fields the draft *can* recover
// ---------------------------------------------------------------------------

/// Split `text` on top-level `sep`, honouring `{}` / `[]` / `()` nesting and
/// Rust string literals.
fn split_top(text: &str, sep: char) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut in_string = false;
    let mut escape = false;
    let mut start = 0usize;
    for (at, c) in text.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_string => escape = true,
            '"' => in_string = !in_string,
            '{' | '[' | '(' if !in_string => depth += 1,
            '}' | ']' | ')' if !in_string => depth -= 1,
            c if c == sep && depth == 0 && !in_string => {
                out.push(text[start..at].trim());
                start = at + c.len_utf8();
            }
            _ => {}
        }
    }
    let tail = text[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// The body of `prefix … suffix`, or `None` when `text` is not that shape.
fn between<'a>(text: &'a str, prefix: &str, suffix: &str) -> Option<&'a str> {
    text.trim()
        .strip_prefix(prefix)?
        .strip_suffix(suffix)
        .map(str::trim)
}

/// The `field: value` pairs of a `Name { … }` struct literal.
fn struct_fields(text: &str) -> Option<BTreeMap<&str, &str>> {
    let open = text.find('{')?;
    let body = text[open + 1..].trim_end().strip_suffix('}')?;
    let mut fields = BTreeMap::new();
    for item in split_top(body, ',') {
        let (key, value) = item.split_once(':')?;
        fields.insert(key.trim(), value.trim());
    }
    Some(fields)
}

/// The items of an `&[ … ]` slice literal.
fn slice_items(text: &str) -> Option<Vec<&str>> {
    let body = between(text, "&[", "]")?;
    Some(split_top(body, ','))
}

/// The value of a Rust string literal, unescaped.
fn rust_str(text: &str) -> Option<String> {
    let body = between(text, "\"", "\"")?;
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next()? {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            '0' => out.push('\0'),
            other => out.push(other),
        }
    }
    Some(out)
}

/// The variant word of a `Path::Variant` expression, payload included.
fn variant_of(text: &str) -> &str {
    text.trim().rsplit("::").next().unwrap_or("").trim()
}

/// `Some(inner)` → `inner`; anything else is returned unchanged.
fn unwrap_some(text: &str) -> Option<&str> {
    between(text, "Some(", ")")
}

/// The dialect member names behind a rendered `DialectSet` expression.
///
/// [`crate::render_rs::dialect_set`] writes the readable aggregates
/// (`DialectSet::ALL_TCL.union(DialectSet::IRULES)`); this reads them back so
/// an option constraint keeps its gate.
fn dialect_names(expr: &str) -> Option<Vec<&'static str>> {
    const CONSTANTS: &[(&str, &[&str])] = &[
        (
            "ALL_TCL",
            &["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"],
        ),
        ("TCL85_PLUS", &["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL86_PLUS", &["tcl8.6", "tcl9.0", "tcl9.1"]),
        ("TCL90_PLUS", &["tcl9.0", "tcl9.1"]),
        ("TCL84", &["tcl8.4"]),
        ("TCL85", &["tcl8.5"]),
        ("TCL86", &["tcl8.6"]),
        ("TCL90", &["tcl9.0"]),
        ("TCL91", &["tcl9.1"]),
        ("IRULES", &["f5-irules"]),
        ("IAPPS", &["f5-iapps"]),
        ("TK", &["tk"]),
        ("EXPECT", &["expect"]),
        ("BPF", &["bpf"]),
        ("TMSH", &["f5-tmsh"]),
        ("BIGIP", &["f5-bigip"]),
        ("SPECTCL", &["spectcl"]),
    ];
    let mut names: Vec<&'static str> = Vec::new();
    for part in expr
        .replace(".union(", " ")
        .replace(')', " ")
        .split_whitespace()
    {
        let key = part.trim().rsplit("::").next()?;
        let (_, members) = CONSTANTS.iter().find(|(name, _)| *name == key)?;
        for member in *members {
            if !names.contains(member) {
                names.push(member);
            }
        }
    }
    (!names.is_empty()).then_some(names)
}

/// `var_write_typing`'s four spellings, from its rendered Rust expression.
fn var_write_typing_word(expr: &str) -> Option<String> {
    let variant = variant_of(expr.split(['(', '{']).next()?);
    Some(match variant {
        "ReturnValue" | "Destructured" => variant.to_owned(),
        "Fixed" => {
            let payload = between(expr, "VarWriteTyping::Fixed(", ")")?;
            format!("{{Fixed {}}}", variant_of(payload))
        }
        "ElementsOf" => {
            let fields = struct_fields(expr)?;
            format!("{{ElementsOf {}}}", fields.get("container_arg")?)
        }
        _ => return None,
    })
}

/// `{ListOfArgs N}` … from a rendered `Option<ReturnElements>`.
///
/// The `{VARIANT payload …}` rule the coverage matrix gives for the three
/// element-structure facts: the variant word, then its fields in declaration
/// order.
fn return_elements_word(expr: &str) -> Option<String> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    let fields = struct_fields(inner)?;
    Some(match variant {
        "ListOfArgs" | "DictOfPairs" => format!("{{{variant} {}}}", fields.get("from")?),
        "ElementOf" | "SubListOf" => format!("{{{variant} {}}}", fields.get("container_arg")?),
        _ => return None,
    })
}

/// `{AppendsListElements N}` / `SetsDictValue` / … from a rendered
/// `Option<VarElementsEffect>`.
fn var_elements_effect_word(expr: &str) -> Option<String> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    Some(match variant {
        "SetsDictValue" | "ListifiesDictValue" => variant.to_owned(),
        "AppendsListElements" | "ExtendsDictValuesByName" => {
            format!(
                "{{{variant} {}}}",
                struct_fields(inner)?.get("values_from")?
            )
        }
        _ => return None,
    })
}

/// `None` / `{CopyOnWriteContainerMutation VAR MIN}` from a rendered
/// `Option<RepresentationEffect>`.
fn representation_effect_word(expr: &str) -> Option<String> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    Some(match variant {
        "None" => "None".to_owned(),
        "CopyOnWriteContainerMutation" => {
            let fields = struct_fields(inner)?;
            format!(
                "{{{variant} {} {}}}",
                fields.get("variable_arg")?,
                fields.get("minimum_arguments")?
            )
        }
        _ => return None,
    })
}

/// The `byte_array_effect` word, including the one variant that carries a
/// payload.
///
/// The draft holds the `Debug` spelling, so the five fieldless effects arrive
/// as a bare name and `Rebinarifies` as a struct literal.
fn byte_array_effect_word(value: &str) -> Option<String> {
    if !value.contains('{') {
        return Some(value.to_owned());
    }
    let variant = variant_of(value.split('{').next()?);
    if variant != "Rebinarifies" {
        return None;
    }
    Some(format!(
        "{{Rebinarifies {}}}",
        struct_fields(value)?.get("value_arg")?
    ))
}

/// `byte_array_payload -replace-data-index N ?-message-flag-shift?`.
fn byte_array_payload_row(expr: &str) -> Option<Vec<String>> {
    let fields = struct_fields(unwrap_some(expr)?)?;
    let mut row = vec![
        "byte_array_payload".to_owned(),
        "-replace-data-index".to_owned(),
        (*fields.get("replace_data_index")?).to_owned(),
    ];
    if *fields.get("message_flag_shift")? == "true" {
        row.push("-message-flag-shift".to_owned());
    }
    Some(row)
}

/// `defines_symbol -name-arg N ?-detail-arg N? ?-requires-arg N? -kind KIND`.
fn defines_symbol_row(expr: &str) -> Option<Vec<String>> {
    let fields = struct_fields(unwrap_some(expr)?)?;
    let mut row = vec![
        "defines_symbol".to_owned(),
        "-name-arg".to_owned(),
        (*fields.get("name_arg")?).to_owned(),
    ];
    for (flag, key) in [
        ("-detail-arg", "detail_arg"),
        ("-requires-arg", "requires_arg"),
    ] {
        if let Some(index) = unwrap_some(fields.get(key)?) {
            row.push(flag.to_owned());
            row.push(index.to_owned());
        }
    }
    row.push("-kind".to_owned());
    row.push(variant_of(fields.get("kind")?).to_owned());
    Some(row)
}

/// `deprecation_fix -replace WORD ?-replace-arg N? ?-replace-invocation?
/// -description {…} -safety S`.
///
/// The coverage matrix's `-replace WORD -description {…} -safety S` covers the
/// matched-word variant; the two positional variants add one word each so the
/// same statement says which of the three it is. `Custom { resolver }` names a
/// registry callback, which the matrix marks reference-only, and has no
/// spelling at all — it never reaches here, because the draft records it as
/// unrecoverable.
fn deprecation_fix_row(expr: &str) -> Option<Vec<String>> {
    let inner = unwrap_some(expr)?;
    let variant = variant_of(inner.split(['(', '{']).next()?);
    let fields = struct_fields(inner)?;
    let mut row = vec!["deprecation_fix".to_owned()];
    let mut lost = false;
    row.push("-replace".to_owned());
    push_word(
        &mut row,
        &mut lost,
        word(&rust_str(fields.get("replacement")?)?),
    );
    match variant {
        "ReplaceMatchedWord" => {}
        "ReplaceArgument" => {
            row.push("-replace-arg".to_owned());
            row.push((*fields.get("index")?).to_owned());
        }
        "ReplaceInvocation" => row.push("-replace-invocation".to_owned()),
        _ => return None,
    }
    row.push("-description".to_owned());
    push_word(
        &mut row,
        &mut lost,
        braced(&rust_str(fields.get("description")?)?),
    );
    row.push("-safety".to_owned());
    row.push(variant_of(fields.get("safety")?).to_owned());
    (!lost).then_some(row)
}

/// `setter_constraint N -prefix P -code CODE -message {…}` rows.
fn setter_constraint_rows(value: &Value) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for constraint in as_array(value) {
        let mut row = vec![
            "setter_constraint".to_owned(),
            constraint.get("arg_index")?.as_u64()?.to_string(),
            "-prefix".to_owned(),
        ];
        let mut lost = false;
        push_word(
            &mut row,
            &mut lost,
            word(str_of(constraint.get("required_prefix")?)),
        );
        row.push("-code".to_owned());
        row.push(str_of(constraint.get("code")?).to_owned());
        row.push("-message".to_owned());
        push_word(
            &mut row,
            &mut lost,
            braced(str_of(constraint.get("message")?)),
        );
        if lost {
            return None;
        }
        rows.push(row);
    }
    Some(rows)
}

/// `repeat ROLE …` rows from a rendered `&[RepeatedArgLayout]`.
fn repeat_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let mut row = vec![
            "repeat".to_owned(),
            variant_of(fields.get("role")?).to_owned(),
        ];
        let start = *fields.get("start")?;
        if start != "0" {
            row.push("-from".to_owned());
            row.push(start.to_owned());
        }
        let stride = *fields.get("stride")?;
        if stride != "1" {
            row.push("-stride".to_owned());
            row.push(stride.to_owned());
        }
        let exclude = *fields.get("exclude_trailing")?;
        if exclude != "0" {
            row.push("-exclude-trailing".to_owned());
            row.push(exclude.to_owned());
        }
        if *fields.get("optional_leading_word")? == "true" {
            row.push("-optional-leading".to_owned());
        }
        if *fields.get("conditional_binding")? == "true" {
            row.push("-conditional".to_owned());
        }
        rows.push(row);
    }
    Some(rows)
}

/// One `RelationTerm::…(…)` expression as an author spells the term.
///
/// The inverse of the loader's `relation_term`, so a relation round-trips
/// through the exporter byte-identically to how it was authored.
fn relation_term_word(expr: &str) -> Option<String> {
    let expr = expr.trim();
    let open = expr.find('(')?;
    let inner = expr[open + 1..].strip_suffix(')')?;
    let variant = expr[..open].rsplit("::").next()?.trim();
    let parts = split_top(inner, ',');
    let words: Vec<String> = match (variant, parts.as_slice()) {
        ("Option", [name]) => vec![rust_str(name)?],
        ("OptionValue", [name, value]) => vec![rust_str(name)?, rust_str(value)?],
        ("Argument", [index]) => vec!["arg".to_owned(), index.trim().to_owned()],
        ("ArgumentValue", [index, value]) => {
            vec!["arg".to_owned(), index.trim().to_owned(), rust_str(value)?]
        }
        _ => return None,
    };
    // The term's *content*, unquoted: the caller brace-quotes it once, either
    // as a standalone subject word or as one element of the term list.
    // Quoting here as well would double-brace `{arg 0}` inside the list.
    Some(words.join(" "))
}

/// The four E-R14 option-relation rows from a rendered `&[OptionRelation]`.
///
/// `option_conflict` keeps its 1.x shape exactly — statement word then the
/// term list — so an unchanged pack round-trips unchanged; the other three
/// take the subject between them.
fn option_conflict_rows(expr: &str, availability: bool) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let terms: Option<Vec<String>> = slice_items(fields.get("terms")?)?
            .into_iter()
            .map(relation_term_word)
            .collect();
        let terms = list_word(&terms?)?;
        let kind = fields
            .get("kind")
            .map_or("MutuallyExclusive", |k| variant_of(k));
        let statement = match kind {
            "MutuallyExclusive" => "option_conflict",
            "Requires" => "option_requires",
            "RequiresOneOf" => "option_requires_one_of",
            "Forbids" => "option_forbids",
            _ => return None,
        };
        let mut row = vec![statement.to_owned()];
        if kind != "MutuallyExclusive" {
            row.push(match fields.get("subject").and_then(|s| unwrap_some(s)) {
                Some(inner) => word(&relation_term_word(inner)?)?,
                None => "{}".to_owned(),
            });
        }
        row.push(terms);
        if let Some(inner) = unwrap_some(fields.get("dialects")?) {
            let members = dialect_names(inner)?;
            if let Some(spelling) = availability.then(|| available_flag(&members)).flatten() {
                row.push("-available".to_owned());
                row.push(spelling);
            } else {
                row.push("-dialects".to_owned());
                row.push(dialect_words(&members)?);
            }
        }
        if let Some(inner) = fields.get("message").and_then(|m| unwrap_some(m)) {
            row.push("-message".to_owned());
            row.push(word(&rust_str(inner)?)?);
        }
        push_literal_lifecycle_flags(&mut row, fields.get("lifecycle").copied())?;
        rows.push(row);
    }
    Some(rows)
}

/// `oo_context_fact WORD FACT` rows from a rendered fact table.
fn oo_context_fact_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let pair = between(item, "(", ")")?;
        let parts = split_top(pair, ',');
        let [name, fact] = parts.as_slice() else {
            return None;
        };
        rows.push(vec![
            "oo_context_fact".to_owned(),
            word(&rust_str(name)?)?,
            variant_of(fact).to_owned(),
        ]);
    }
    Some(rows)
}

/// The `binds_handle` row's list argument, from a rendered
/// `Some(&HandleBindingSpec { … })`.
fn binds_handle_word(expr: &str) -> Option<String> {
    let fields = struct_fields(unwrap_some(expr)?)?;
    let source = |text: &str| -> Option<String> {
        // `HandleName::Word(0)` / `HandleClassSource::ConstructionValue(1)` —
        // the path has to come off before the payload can be read.
        let (variant, payload) = variant_of(text).split_once('(')?;
        let payload = payload.strip_suffix(')')?;
        Some(match variant {
            "Implicit" => format!("{{Implicit {}}}", word(&rust_str(payload)?)?),
            other => format!("{{{other} {payload}}}"),
        })
    };
    let mut parts = vec![
        "-name-from".to_owned(),
        source(fields.get("name_from")?)?,
        "-class-from".to_owned(),
        source(fields.get("class_from")?)?,
    ];
    if let Some(keyword) = unwrap_some(fields.get("keyword")?) {
        let inner = struct_fields(keyword)?;
        parts.push("-keyword".to_owned());
        parts.push(format!(
            "{{{} {}}}",
            inner.get("at")?,
            word(&rust_str(inner.get("word")?)?)?
        ));
    }
    braced(&parts.join(" "))
}

/// Push the three lifecycle flags out of a rendered `Lifecycle { … }` literal.
///
/// `None` for a literal this reader cannot take apart, which makes the whole
/// row a reported loss rather than a half-written one; an absent field is the
/// registry's own `..DEFAULT`, so it contributes no flag.
fn push_literal_lifecycle_flags(row: &mut Vec<String>, literal: Option<&str>) -> Option<()> {
    let Some(literal) = literal else {
        return Some(());
    };
    let fields = struct_fields(literal)?;
    for (field, flag) in [
        ("introduced", "-introduced"),
        ("deprecated", "-deprecated"),
        ("retired", "-retired"),
    ] {
        let Some(value) = fields.get(field) else {
            continue;
        };
        if let Some(inner) = unwrap_some(value) {
            row.push(flag.to_owned());
            row.push(word(&rust_str(inner)?)?);
        }
    }
    Some(())
}

/// `versioned_arg_value N VALUE …` rows from a rendered gate table.
fn versioned_arg_value_rows(expr: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    for item in slice_items(expr)? {
        let fields = struct_fields(item)?;
        let mut row = vec![
            "versioned_arg_value".to_owned(),
            (*fields.get("index")?).to_owned(),
            word(&rust_str(fields.get("value")?)?)?,
        ];
        push_literal_lifecycle_flags(&mut row, fields.get("lifecycle").copied())?;
        rows.push(row);
    }
    Some(rows)
}

// ---------------------------------------------------------------------------
// Field-level emission
// ---------------------------------------------------------------------------

/// Everything one command or subcommand body needs while rendering.
struct Ctx<'a> {
    /// The default draft the body's fields are compared against.
    defaults: &'a Draft,
    /// The `-native` id prefix — `lsort`, or `string::is` for a subcommand.
    scope: String,
    tables: &'a mut ValueTables,
    /// Whether the pack being rendered declares `SpecTcl` 2.0, and may
    /// therefore use the availability algebra.
    ///
    /// A renderer that emitted `available` into a pack declaring 1.x would
    /// produce exactly the header/body inconsistency the loader reports per
    /// site (#1627), so the *header* decides the spelling: a 2.0 pack — every
    /// pack the studio creates, since [`DSL_VERSION`] is the newest
    /// vocabulary — takes `available`, and a document that declares an older
    /// vocabulary keeps `dialects` and stays a valid pack of its own
    /// vocabulary. Moving such a document to 2.0 is `tcl spec upgrade`'s job,
    /// not a side effect of opening it in the studio.
    availability: bool,
}

impl Ctx<'_> {
    /// Whether `draft`'s `key` differs from the default a fresh spec has.
    fn set(&self, draft: &Draft, key: &str) -> bool {
        let value = draft.get(key);
        value.is_some() && value != self.defaults.get(key)
    }

    fn native(&self, field: &str) -> Vec<String> {
        vec![
            field.to_owned(),
            "-native".to_owned(),
            format!("{}::{field}", self.scope),
        ]
    }
}

/// Whether `draft` records `key` as one whose defining expression seeding
/// could not recover.
fn unrecovered(draft: &Draft, key: &str) -> bool {
    draft
        .get(draft::UNRENDERABLE_KEY)
        .map(as_array)
        .is_some_and(|keys| keys.iter().any(|k| k.as_str() == Some(key)))
}

/// The `# TODO(spectcl):` line for a field that did not survive.
fn todo(out: &mut Out, key: &str) {
    let Some(gap) = gap(key) else {
        out.comment(&format!(
            "TODO(spectcl): `{key}` is set on the source spec and has no DSL spelling."
        ));
        out.losses.push(Loss {
            key: key.to_owned(),
            reason: "set on the source spec, with no DSL spelling".to_owned(),
        });
        return;
    };
    let spelling = if gap.spelling.is_empty() {
        String::new()
    } else {
        format!(" The DSL spells it `{}`, but ", gap.spelling)
    };
    let joiner = if spelling.is_empty() { " — " } else { "" };
    out.comment(&format!(
        "TODO(spectcl): `{key}` is set on the source spec.{spelling}{joiner}{}.",
        gap.kind.why()
    ));
    out.losses.push(Loss {
        key: key.to_owned(),
        reason: gap.kind.why().to_owned(),
    });
}

/// The `# TODO(spectcl):` line for text no Tcl word can carry verbatim.
///
/// A braced word is the only faithful spelling (see [`word`]), so a string
/// with an unbalanced brace or a backslash-newline has to be dropped rather
/// than written down wrong.
fn unwritable(out: &mut Out, key: &str) {
    out.comment(&format!(
        "TODO(spectcl): `{key}` holds text no Tcl word carries verbatim — an \
         unbalanced brace or a backslash-newline. A braced word is the only \
         faithful spelling, so the statement is dropped rather than written \
         down wrong."
    ));
    out.losses.push(Loss {
        key: key.to_owned(),
        reason: "the text has an unbalanced brace or a backslash-newline, which no \
                 Tcl word carries verbatim"
            .to_owned(),
    });
}

/// Emit `key` when it is set, as a one-value property statement.
fn scalar(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str, value: &str) {
    if ctx.set(draft, key) {
        out.line(&format!("{key} {value}"));
    }
}

/// Emit a boolean property in the DSL's bare-word / explicit-`no` spelling.
fn flag(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if !ctx.set(draft, key) {
        return;
    }
    if draft[key].as_bool() == Some(false) {
        out.line(&format!("{key} no"));
    } else {
        out.line(key);
    }
}

/// Emit a text / opt-text property.
fn text(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(value) = draft[key].as_str()
    {
        match word(value) {
            Some(spelling) => out.line(&format!("{key} {spelling}")),
            None => unwritable(out, key),
        }
    }
}

/// Emit an enum property, whose draft value is already the catalogue spelling.
///
/// A variant carrying a payload (`Rebinarifies { data_index: 2 }`) is *not*
/// one word, and no loader vocabulary lists it, so it becomes a `TODO` rather
/// than a statement the loader would drop half of.
fn enum_word(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(value) = draft[key].as_str()
    {
        if value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        {
            out.line(&format!("{key} {value}"));
        } else {
            todo(out, key);
        }
    }
}

/// Emit a flag-set property — a trait, taint-colour, or dialect set.
fn set_word(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    // Availability is written in the 2.0 algebra wherever it carries the
    // set; `dialects` stays for the bits it has no provider for, and for
    // `safe_on_uninit`, which holds a dialect set but is not an
    // availability question and has no `available` reader.
    if key == "dialects"
        && ctx.availability
        && ctx.set(draft, key)
        && draft[key].is_array()
        && let Some(rows) = available_statement(&draft[key])
    {
        out.line(&format!("available {rows}"));
        return;
    }
    if ctx.set(draft, key) && draft[key].is_array() {
        let spelling = if is_dialect_set(key) {
            dialect_set_word(&draft[key])
        } else {
            str_list_word(&draft[key])
        };
        match spelling {
            Some(spelling) => out.line(&format!("{key} {spelling}")),
            None => unwritable(out, key),
        }
    }
}

/// Emit the data form of the Tk geometry-manager descriptor.
fn tk_geometry_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "tk_geometry") {
        return;
    }
    let Some(fields) = draft["tk_geometry"].as_object() else {
        todo(out, "tk_geometry");
        return;
    };
    let Some(policy @ ("Independent" | "Exclusive")) =
        fields.get("container_policy").and_then(Value::as_str)
    else {
        todo(out, "tk_geometry");
        return;
    };
    let container_option = fields
        .get("container_option")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let direct_form = fields
        .get("direct_form")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let placement_subcommand = fields
        .get("placement_subcommand")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let release_subcommands: Vec<String> = fields
        .get("release_subcommands")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let mut row = vec!["tk_geometry".to_owned(), policy.to_owned()];
    let mut lost = false;
    if let Some(container_option) = container_option {
        row.push("-container-option".to_owned());
        push_word(&mut row, &mut lost, word(&container_option));
    }
    if direct_form {
        row.push("-direct-form".to_owned());
    }
    if let Some(placement_subcommand) = placement_subcommand {
        row.push("-placement-subcommand".to_owned());
        push_word(&mut row, &mut lost, word(&placement_subcommand));
    }
    if !release_subcommands.is_empty() {
        push_flag(
            &mut row,
            &mut lost,
            "-release-subcommands",
            list_word(&release_subcommands),
        );
    }
    emit_row(out, "tk_geometry", &row, lost, "");
}

/// Whether a flag-set field holds dialect members, which take the shorthands.
fn is_dialect_set(key: &str) -> bool {
    matches!(key, "dialects" | "safe_on_uninit")
}

/// Emit a `&'static [&'static str]` property.
fn text_list(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    set_word(out, ctx, draft, key);
}

/// Emit an index-list property, keeping the tri-state (`{}` is *declared
/// empty*, absent is unset).
fn index_list(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if !ctx.set(draft, key) {
        return;
    }
    let Some(items) = draft[key].as_array() else {
        return;
    };
    let indices: Vec<String> = items.iter().map(ToString::to_string).collect();
    out.line(&format!("{key} {{{}}}", indices.join(" ")));
}

/// Emit a count property.
fn count(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(n) = draft[key].as_u64()
    {
        out.line(&format!("{key} {n}"));
    }
}

/// Emit an expression-valued property whose DSL spelling is one word.
fn expr_word(
    out: &mut Out,
    ctx: &Ctx<'_>,
    draft: &Draft,
    key: &str,
    spell: impl Fn(&str) -> Option<String>,
) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        match draft[key].as_str().and_then(spell) {
            Some(spelling) => out.line(&format!("{key} {spelling}")),
            None => todo(out, key),
        }
    }
}

/// Emit an expression-valued property whose DSL spelling is a flag row.
fn expr_row(
    out: &mut Out,
    ctx: &Ctx<'_>,
    draft: &Draft,
    key: &str,
    spell: impl Fn(&str) -> Option<Vec<String>>,
) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        match draft[key].as_str().and_then(spell) {
            Some(row) => out.row(&row, "-description"),
            None => todo(out, key),
        }
    }
}

/// Emit a `RustExpr` field that the DSL cannot carry, as a `TODO`.
fn gap_todo(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key) || unrecovered(draft, key) {
        todo(out, key);
    }
}

/// Emit a hook field in the documented `-native ID` form.
///
/// Not a loss: the loader installs the family's abstention for a `-native`
/// hook exactly as it does for a Tcl body, and seeding records the reloaded
/// field the same way it recorded the shipped one — "set, expression not
/// recovered".
fn native_hook(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if unrecovered(draft, key) {
        out.line(&ctx.native(key).join(" "));
    }
}

/// Emit a closed-catalogue compiler hook, whose draft value *is* the variant
/// name the DSL takes.
fn catalogue_hook(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft, key: &str) {
    if ctx.set(draft, key)
        && let Some(id) = draft[key].as_str()
    {
        out.line(&format!("{key} -native {id}"));
    }
}

/// `arity N`, `N..M`, `N..`, `..M`, `..`, plus `-step` / `-also`.
fn arity_row(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "arity") {
        return;
    }
    let value = &draft["arity"];
    let min = value["min"].as_u64().unwrap_or(0);
    let max = value["max"].as_u64();
    let step = value["step"].as_u64().unwrap_or(0);
    let also = value["also_exact"].as_u64();
    let range = match (min, max) {
        (m, Some(x)) if m == x && step == 0 => m.to_string(),
        (0, None) => "..".to_owned(),
        (m, None) => format!("{m}.."),
        (0, Some(x)) => format!("..{x}"),
        (m, Some(x)) => format!("{m}..{x}"),
    };
    let mut row = format!("arity {range}");
    if step != 0 {
        let _ = write!(row, " -step {step}");
    }
    if let Some(also) = also {
        let _ = write!(row, " -also {also}");
    }
    out.line(&row);
}

/// The shape words of one arity value, shared by the plain row and every
/// window so a window's shape is spelled exactly as the plain row spells it.
fn arity_shape(value: &Value) -> String {
    let min = value["min"].as_u64().unwrap_or(0);
    let max = value["max"].as_u64();
    let step = value["step"].as_u64().unwrap_or(0);
    let also = value["also_exact"].as_u64();
    let range = match (min, max) {
        (m, Some(x)) if m == x && step == 0 => m.to_string(),
        (0, None) => "..".to_owned(),
        (m, None) => format!("{m}.."),
        (0, Some(x)) => format!("..{x}"),
        (m, Some(x)) => format!("{m}..{x}"),
    };
    let mut row = range;
    if step != 0 {
        let _ = write!(row, " -step {step}");
    }
    if let Some(also) = also {
        let _ = write!(row, " -also {also}");
    }
    row
}

/// `arity SHAPE -introduced V ?-deprecated V? ?-retired V?` — one row per
/// version window (`SpecTcl` 1.2, issue #1627).
///
/// Emitted after the plain `arity` row, matching the order the loader reads
/// them and the order `CommandSpec::DEFAULT` declares them. A spec with no
/// windows renders nothing, which is every spec whose signature never
/// changed.
fn arity_window_rows(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "arity_windows") {
        return;
    }
    let windows = as_array(&draft["arity_windows"]).to_vec();
    for window in &windows {
        let mut row = format!("arity {}", arity_shape(&window["arity"]));
        let lifecycle = &window["lifecycle"];
        for (flag, key) in [
            ("-introduced", "introduced"),
            ("-deprecated", "deprecated"),
            ("-retired", "retired"),
        ] {
            if let Some(at) = lifecycle.get(key).and_then(Value::as_str) {
                let _ = write!(row, " {flag} {at}");
            }
        }
        out.line(&row);
    }
}

/// The per-argument rows: six schema keys, one statement per index.
///
/// Indices are visited in the order the tables themselves list them — roles
/// first, then the tables that only ever accompany them — so a spec that
/// declares its roles out of index order keeps that order on the way back in.
fn arg_row_statements(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    let table =
        |key: &str| -> Vec<Value> { as_array(draft.get(key).unwrap_or(&Value::Null)).to_vec() };
    let roles = table("arg_roles");
    let types = table("arg_types");
    let values = table("arg_values");
    let presentation = table("arg_presentation");
    let prefixes = table("command_prefixes");
    let closed: Vec<u64> = as_array(draft.get("closed_value_args").unwrap_or(&Value::Null))
        .iter()
        .filter_map(Value::as_u64)
        .collect();

    let mut order: Vec<u64> = Vec::new();
    let note = |index: Option<u64>, order: &mut Vec<u64>| {
        if let Some(index) = index
            && !order.contains(&index)
        {
            order.push(index);
        }
    };
    for entry in roles
        .iter()
        .chain(&types)
        .chain(&values)
        .chain(&presentation)
        .chain(&prefixes)
    {
        note(entry["index"].as_u64(), &mut order);
    }
    for index in &closed {
        note(Some(*index), &mut order);
    }

    let at = |entries: &[Value], index: u64| -> Option<Value> {
        entries
            .iter()
            .find(|entry| entry["index"].as_u64() == Some(index))
            .cloned()
    };

    for index in order {
        let mut row = vec!["arg".to_owned(), index.to_string()];
        let mut lost = false;
        if let Some(entry) = at(&roles, index) {
            row.push("-role".to_owned());
            row.push(str_of(&entry["role"]).to_owned());
        }
        if let Some(entry) = at(&types, index) {
            if let Some(expected) = entry["expected"].as_str() {
                row.push("-type".to_owned());
                row.push(expected.to_owned());
            }
            if entry["shimmers"].as_bool() == Some(true) {
                row.push("-shimmers".to_owned());
            }
            let transparent = as_array(&entry["transparent_from"]);
            if !transparent.is_empty() {
                push_flag(
                    &mut row,
                    &mut lost,
                    "-transparent",
                    str_list_word(&entry["transparent_from"]),
                );
            }
        }
        if let Some(entry) = at(&values, index) {
            push_values(
                &mut row,
                &mut lost,
                ctx,
                &entry["values"],
                &format!("{}-arg{index}", ctx.scope),
            );
        }
        if closed.contains(&index) {
            row.push("-closed".to_owned());
        }
        if let Some(entry) = at(&presentation, index) {
            row.push("-layout".to_owned());
            row.push(str_of(&entry["presentation"]).to_owned());
        }
        if let Some(entry) = at(&prefixes, index) {
            row.push("-appends".to_owned());
            row.push(appended_arity_word(&entry["arity"]));
        }
        emit_row(out, "arg_values", &row, lost, "-detail");
    }
}

/// The registry-declared user-controlled substitutions for positional deferred
/// callbacks. This is deliberately separate from `arg` rows: it describes a
/// runtime injection into the stored callback, not a static syntax role.
fn callback_taint_input_table(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "callback_taint_inputs") {
        return;
    }
    let rows: Vec<String> = as_array(&draft["callback_taint_inputs"])
        .iter()
        .filter_map(|entry| {
            let index = entry["index"].as_u64()?;
            let inputs = as_array(&entry["inputs"]);
            (!inputs.is_empty())
                .then(|| str_list_word(&Value::Array(inputs.to_vec())))
                .flatten()
                .map(|inputs| format!("{{{index} {inputs}}}"))
        })
        .collect();
    if !rows.is_empty() {
        out.line(&format!("callback_taint_inputs {{{}}}", rows.join(" ")));
    }
}

/// `{Exactly N}` / `{AtLeast N}` / `Unknown`.
fn appended_arity_word(value: &Value) -> String {
    let kind = str_of(&value["kind"]);
    match value["n"].as_u64() {
        Some(n) => format!("{{{kind} {n}}}"),
        None => kind.to_owned(),
    }
}

/// Push `-values {…}` or `-values-from NAME` onto a row.
///
/// The inline form carries the value words and nothing else, so a table whose
/// rows have a detail, a version floor, or a completion code is hoisted to a
/// pack-level `values` declaration instead.
fn push_values(
    row: &mut Vec<String>,
    lost: &mut bool,
    ctx: &mut Ctx<'_>,
    values: &Value,
    hint: &str,
) {
    let entries = as_array(values);
    if entries.is_empty() {
        return;
    }
    // Anything the inline `-values {…}` word cannot carry — a detail, either
    // version axis, or a completion code — hoists the whole set to a
    // pack-level table, which has a row per value and room for all of it.
    let plain = entries.iter().all(|entry| {
        str_of(&entry["detail"]).is_empty()
            && entry["min_tcl"].is_null()
            && entry["code"].is_null()
            && !has_lifecycle(entry)
    });
    if plain {
        let words: Vec<&str> = entries
            .iter()
            .map(|entry| str_of(&entry["value"]))
            .collect();
        push_flag(row, lost, "-values", list_word(&words));
    } else {
        let name = ctx.tables.intern(hint, values);
        row.push("-values-from".to_owned());
        row.push(name);
    }
}

/// One `option NAME …` row.
#[allow(clippy::too_many_lines)]
fn option_row(out: &mut Out, ctx: &mut Ctx<'_>, option: &Value) {
    let name = str_of(&option["name"]);
    let mut row = vec!["option".to_owned(), name_word(name)];
    let mut lost = false;
    let value = &option["value"];
    if !value.is_null() {
        push_flag(&mut row, &mut lost, "-takes", word(str_of(&value["hint"])));

        let arity = &value["arity"];
        match str_of(&arity["kind"]) {
            "Fixed" => {
                row.push("-arity".to_owned());
                row.push(format!("{{Fixed {}}}", arity["n"].as_u64().unwrap_or(0)));
            }
            "Hook" => {
                row.push("-arity-hook".to_owned());
                row.push("-native".to_owned());
                row.push(format!(
                    "{}::{}::arity",
                    ctx.scope,
                    name.trim_start_matches('-')
                ));
            }
            _ => {}
        }
        let role = str_of(&value["role"]);
        if role != "Value" {
            row.push("-role".to_owned());
            row.push(role.to_owned());
        }
        let also_role = value["also_role"].as_str();
        if let Some(also) = also_role {
            row.push("-also-role".to_owned());
            row.push(also.to_owned());
        }
        let body_kind = str_of(&value["body_kind"]);
        if body_kind != "Plain" {
            row.push("-body-kind".to_owned());
            row.push(body_kind.to_owned());
        }
        let script_timing = str_of(&value["script_timing"]);
        if !script_timing.is_empty() && script_timing != "SameInvocation" {
            row.push("-script-timing".to_owned());
            row.push(script_timing.to_owned());
        }
        let callback_inputs = as_array(&value["callback_taint_inputs"]);
        if !callback_inputs.is_empty()
            && let Some(inputs) = str_list_word(&Value::Array(callback_inputs.to_vec()))
        {
            row.push("-callback-taint-inputs".to_owned());
            row.push(inputs);
        }
        if str_of(&value["variable_scope"]) == "Global" {
            row.push("-variable-scope".to_owned());
            row.push("Global".to_owned());
        }
        push_values(
            &mut row,
            &mut lost,
            ctx,
            &value["values"],
            &format!("{}-{}", ctx.scope, name.trim_start_matches('-')),
        );
        if value["closed"].as_bool() == Some(true) {
            row.push("-closed".to_owned());
        }
        if let Some(domain) = integer_domain_word(&value["integer"]) {
            row.push("-integer".to_owned());
            row.push(domain);
        }
        let appended = &value["appended_arity"];
        if str_of(&appended["kind"]) != "Unknown" {
            row.push("-appends".to_owned());
            row.push(appended_arity_word(appended));
        }
        if value["taints_var_write"].as_bool() == Some(true)
            && (role == "VarWrite" || also_role == Some("VarWrite"))
        {
            row.push("-taints-var-write".to_owned());
        }
    }
    if let Some(aliases) = option["aliases"].as_array()
        && !aliases.is_empty()
    {
        push_flag(
            &mut row,
            &mut lost,
            "-aliases",
            str_list_word(&option["aliases"]),
        );
    }
    if let Some(dialects) = option["dialects"].as_array() {
        push_availability_flag(
            &mut row,
            &mut lost,
            &Value::Array(dialects.clone()),
            ctx.availability,
        );
    }
    if let Some(n) = option["min_abbrev"].as_u64() {
        row.push("-min-abbrev".to_owned());
        row.push(n.to_string());
    }
    push_lifecycle_flags(&mut row, &mut lost, option);
    // The data form of the quick-fix hook: the same flags the command-level
    // `deprecation_fix` statement takes, wrapped in one block word. The
    // callback variant has no data to write, so it stays a reported loss.
    let mut unwritable_fix = option[draft::OPTION_DEPRECATION_FIX_UNRECOVERABLE_KEY]
        .as_bool()
        .unwrap_or(false);
    if let Some(expr) = option["deprecation_fix"].as_str() {
        match deprecation_fix_row(expr)
            .map(|words| words[1..].join(" "))
            .and_then(|flags| braced(&flags))
        {
            Some(block) => {
                row.push("-deprecation-fix".to_owned());
                row.push(block);
            }
            None => unwritable_fix = true,
        }
    }
    let detail = str_of(&option["detail"]);
    if !detail.is_empty() {
        push_flag(&mut row, &mut lost, "-detail", braced(detail));
    }
    emit_row(out, "options", &row, lost, "-detail");
    if unwritable_fix {
        out.indented(|out| {
            out.comment(&format!(
                "TODO(spectcl): `{name}` carries a contextual deprecation fix; \
                 `-deprecation-fix {{…}}` writes the data form only."
            ));
            out.losses.push(Loss {
                key: OPTION_DEPRECATION_FIX_HOOK_KEY.to_owned(),
                reason: "the fix is a registry callback, not data".to_owned(),
            });
        });
    }
}

/// `{Range lo hi}` / `Any` / `Port`, with the `max` / `min` sentinels the DSL
/// documents.
fn integer_domain_word(value: &Value) -> Option<String> {
    match str_of(&value["kind"]) {
        "Any" => Some("Any".to_owned()),
        "Port" => Some("Port".to_owned()),
        "Range" => {
            let bound = |n: Option<i64>, sentinel: &str| -> String {
                match n {
                    Some(i64::MAX) => "max".to_owned(),
                    Some(i64::MIN) => "min".to_owned(),
                    Some(n) => n.to_string(),
                    None => sentinel.to_owned(),
                }
            };
            Some(format!(
                "{{Range {} {}}}",
                bound(value["lo"].as_i64(), "min"),
                bound(value["hi"].as_i64(), "max"),
            ))
        }
        _ => None,
    }
}

/// The `hover { … }` block.
fn hover_block(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "hover") {
        return;
    }
    let hover = &draft["hover"];
    if hover.is_null() {
        return;
    }
    out.gap();
    out.line("hover {");
    out.indented(|out| {
        let scalar_row = |out: &mut Out, key: &str, word_name: &str| {
            let text = str_of(&hover[key]);
            if text.is_empty() {
                return;
            }
            match braced(text) {
                Some(spelling) => out.line(&format!("{word_name} {spelling}")),
                None => unwritable(out, "hover"),
            }
        };
        scalar_row(out, "summary", "summary");
        for synopsis in as_array(&hover["synopsis"]) {
            match braced(str_of(synopsis)) {
                Some(spelling) => out.line(&format!("synopsis {spelling}")),
                None => unwritable(out, "hover"),
            }
        }
        // Three words are renamed from their Rust field names, and nothing
        // else in the DSL renames a key.
        scalar_row(out, "snippet", "description");
        scalar_row(out, "source", "source");
        // ONE `example` block, never one row per example: repeated rows join
        // with a single newline, which would flatten an `examples` string that
        // separates its examples with a blank line.
        scalar_row(out, "examples", "example");
        scalar_row(out, "return_value", "returns");
    });
    out.line("}");
}

// ---------------------------------------------------------------------------
// Command and subcommand bodies
// ---------------------------------------------------------------------------

/// Render the body of one `command NAME { … }` block.
#[allow(clippy::too_many_lines)]
fn command_body(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    // --- identity and availability -----------------------------------------
    set_word(out, ctx, draft, "dialects");
    set_word(out, ctx, draft, "traits");
    arity_row(out, ctx, draft);
    arity_window_rows(out, ctx, draft);
    text(out, ctx, draft, "required_package");
    tk_geometry_row(out, ctx, draft);
    text(out, ctx, draft, "tcllib_package");
    text(out, ctx, draft, "implementation_namespace");
    text(out, ctx, draft, "introduced_version");
    text(out, ctx, draft, "deprecated_version");
    text(out, ctx, draft, "retired_version");
    flag(out, ctx, draft, "warn_missing_import");
    flag(out, ctx, draft, "is_namespace_exported");
    flag(out, ctx, draft, "unsafe_command");
    text_list(out, ctx, draft, "excluded_events");
    set_word(out, ctx, draft, "safe_on_uninit");
    expr_row(out, ctx, draft, "deprecation_fix", deprecation_fix_row);

    // --- types and shape ---------------------------------------------------
    out.gap();
    enum_word(out, ctx, draft, "return_type");
    if ctx.set(draft, "var_write_typing")
        && let Some(value) = draft["var_write_typing"].as_str()
    {
        match var_write_typing_word(value) {
            Some(spelling) => out.line(&format!("var_write_typing {spelling}")),
            None => todo(out, "var_write_typing"),
        }
    }
    expr_word(out, ctx, draft, "return_elements", return_elements_word);
    expr_word(
        out,
        ctx,
        draft,
        "var_elements_effect",
        var_elements_effect_word,
    );
    expr_word(
        out,
        ctx,
        draft,
        "representation_effect",
        representation_effect_word,
    );
    enum_word(out, ctx, draft, "inferred_storage_type");
    enum_word(out, ctx, draft, "body_kind");
    expr_word(out, ctx, draft, "byte_array_effect", byte_array_effect_word);
    enum_word(out, ctx, draft, "pattern_type");
    gap_todo(out, ctx, draft, "pattern_arg_resolver");
    enum_word(out, ctx, draft, "format_string_type");
    expr_row(
        out,
        ctx,
        draft,
        "byte_array_payload",
        byte_array_payload_row,
    );

    arg_row_statements(out, ctx, draft);
    callback_taint_input_table(out, ctx, draft);
    if ctx.set(draft, "repeated_args")
        && let Some(expr) = draft["repeated_args"].as_str()
    {
        match repeat_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "repeated_args"),
        }
    }
    count(out, ctx, draft, "reserved_trailing_words");
    count(out, ctx, draft, "body_arg_implicit_args");
    if ctx.set(draft, "assigns_variable_at") {
        scalar(
            out,
            ctx,
            draft,
            "assigns_variable_at",
            &draft["assigns_variable_at"].to_string(),
        );
    }
    if ctx.set(draft, "creates_instance_at") {
        scalar(
            out,
            ctx,
            draft,
            "creates_instance_at",
            &draft["creates_instance_at"].to_string(),
        );
    }
    if ctx.set(draft, "defines_command_at") {
        scalar(
            out,
            ctx,
            draft,
            "defines_command_at",
            &draft["defines_command_at"].to_string(),
        );
    }
    expr_row(out, ctx, draft, "defines_symbol", defines_symbol_row);

    // --- subcommand dispatch -----------------------------------------------
    flag(out, ctx, draft, "allow_unknown_subcommands");
    enum_word(out, ctx, draft, "prefix_matching");
    enum_word(out, ctx, draft, "default_form_first_word");
    text_list(out, ctx, draft, "self_receiver_words");

    // --- hooks -------------------------------------------------------------
    out.gap();
    native_hook(out, ctx, draft, "arg_role_resolver");
    native_hook(out, ctx, draft, "command_prefix_resolver");
    native_hook(out, ctx, draft, "script_timing_resolver");
    native_hook(out, ctx, draft, "clause_shape_check");
    native_hook(out, ctx, draft, "const_fold");
    native_hook(out, ctx, draft, "const_fold_versioned");
    native_hook(out, ctx, draft, "literal_argument_validator");
    native_hook(out, ctx, draft, "context_gate");
    catalogue_hook(out, ctx, draft, "lowering_hook");
    catalogue_hook(out, ctx, draft, "codegen_hook");
    catalogue_hook(out, ctx, draft, "inline_codegen_hook");
    catalogue_hook(out, ctx, draft, "analyser_hook");
    gap_todo(out, ctx, draft, "semantic_operation");
    gap_todo(out, ctx, draft, "bpf_op");
    gap_todo(out, ctx, draft, "completion");
    gap_todo(out, ctx, draft, "dispatch_dependencies");
    gap_todo(out, ctx, draft, "result_stability");
    gap_todo(out, ctx, draft, "command_forms");

    // --- effects -----------------------------------------------------------
    out.gap();
    enum_word(out, ctx, draft, "command_table_effect");
    side_effect_rows(out, draft, ctx.availability);
    gap_todo(out, ctx, draft, "frame_effect");
    gap_todo(out, ctx, draft, "world_effects");
    gap_todo(out, ctx, draft, "state_transitions");

    // --- taint and security ------------------------------------------------
    out.gap();
    text(out, ctx, draft, "taint_output_sink");
    text_list(out, ctx, draft, "taint_output_sink_subcommands");
    text(out, ctx, draft, "taint_log_sink");
    index_list(out, ctx, draft, "taint_network_sink_args");
    index_list(out, ctx, draft, "taint_code_sink_args");
    text_list(out, ctx, draft, "taint_interp_eval_subcommands");
    set_word(out, ctx, draft, "taint_source");
    set_word(out, ctx, draft, "taint_transform");
    set_word(out, ctx, draft, "taint_double_encode_colour");
    set_word(out, ctx, draft, "taint_sink_safe_colour");
    native_hook(out, ctx, draft, "taint_sink_gate");
    text_list(out, ctx, draft, "credential_options");
    text_list(out, ctx, draft, "sensitive_headers");
    if ctx.set(draft, "setter_constraints") {
        match setter_constraint_rows(&draft["setter_constraints"]) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "-message");
                }
            }
            None => todo(out, "setter_constraints"),
        }
    }

    // --- iRules ------------------------------------------------------------
    gap_todo(out, ctx, draft, "event_requires");
    gap_todo(out, ctx, draft, "event_requirement_forms");
    gap_todo(out, ctx, draft, "data_collection");
    gap_todo(out, ctx, draft, "side_switch_target");
    gap_todo(out, ctx, draft, "event_handler_priority");
    gap_todo(out, ctx, draft, "irules_top_level_effect");

    // --- translation -------------------------------------------------------
    out.gap();
    if ctx.set(draft, "xc_translatable")
        && let Some(value) = draft["xc_translatable"].as_bool()
    {
        out.line(if value {
            "xc_translatable yes"
        } else {
            "xc_translatable no"
        });
    }
    text(out, ctx, draft, "deprecated_replacement");
    flag(out, ctx, draft, "deprecated_replacement_drop_in");

    // --- descriptors -------------------------------------------------------
    out.gap();
    gap_todo(out, ctx, draft, "definition_body");
    manufacturer_rows(out, draft);
    gap_todo(out, ctx, draft, "case_list");
    object_class_block(out, ctx, draft);
    gap_todo(out, ctx, draft, "body_scope");
    if ctx.set(draft, "oo_context_facts")
        && let Some(expr) = draft["oo_context_facts"].as_str()
    {
        match oo_context_fact_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "oo_context_facts"),
        }
    }
    if ctx.set(draft, "binds_handle")
        && let Some(expr) = draft["binds_handle"].as_str()
    {
        match binds_handle_word(expr) {
            Some(spelling) => out.line(&format!("binds_handle {spelling}")),
            None => todo(out, "binds_handle"),
        }
    }

    // --- options -----------------------------------------------------------
    option_block(out, ctx, draft);
    versioned_arg_value_rows_for(out, ctx, draft);

    // --- documentation -----------------------------------------------------
    if ctx.set(draft, "forms") {
        out.gap();
        for form in as_array(&draft["forms"]) {
            let mut row = vec!["form".to_owned(), str_of(&form["kind"]).to_owned()];
            let mut lost = false;
            push_word(&mut row, &mut lost, braced(str_of(&form["synopsis"])));
            if let Some(dialects) = form["dialects"].as_array() {
                push_availability_flag(
                    &mut row,
                    &mut lost,
                    &Value::Array(dialects.clone()),
                    ctx.availability,
                );
            }
            push_lifecycle_flags(&mut row, &mut lost, form);
            emit_row(out, "forms", &row, lost, "");
        }
    }
    hover_block(out, ctx, draft);

    // --- subcommands -------------------------------------------------------
    for sub in as_array(draft.get("subcommands").unwrap_or(&Value::Null)) {
        let Some(body) = sub.as_object() else {
            continue;
        };
        subcommand_block(out, ctx, body, "subcommand");
    }
}

/// The `option …` / `option_conflict …` rows, which the ports keep together.
fn option_block(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    if ctx.set(draft, "options") {
        out.gap();
        for option in as_array(&draft["options"]).to_vec() {
            option_row(out, ctx, &option);
        }
    }
    enum_word(out, ctx, draft, "option_placement");
    if ctx.set(draft, "option_relations")
        && let Some(expr) = draft["option_relations"].as_str()
    {
        match option_conflict_rows(expr, ctx.availability) {
            Some(rows) => {
                for row in rows {
                    out.row(&row, "");
                }
            }
            None => todo(out, "option_relations"),
        }
    }
}

/// The `versioned_arg_value N VALUE …` rows of a command or subcommand body —
/// one spelling, both levels, since 1.1 made the statement legal at either.
fn versioned_arg_value_rows_for(out: &mut Out, ctx: &Ctx<'_>, draft: &Draft) {
    if !ctx.set(draft, "versioned_arg_values") {
        return;
    }
    let Some(expr) = draft["versioned_arg_values"].as_str() else {
        return;
    };
    match versioned_arg_value_rows(expr) {
        Some(rows) => {
            for row in rows {
                out.row(&row, "");
            }
        }
        None => todo(out, "versioned_arg_values"),
    }
}

fn side_effect_rows(out: &mut Out, draft: &Draft, availability: bool) {
    for effect in as_array(draft.get("side_effects").unwrap_or(&Value::Null)) {
        let mut row = vec![
            "side_effect".to_owned(),
            str_of(&effect["target"]).to_owned(),
        ];
        if effect["reads"].as_bool() == Some(true) {
            row.push("-reads".to_owned());
        }
        if effect["writes"].as_bool() == Some(true) {
            row.push("-writes".to_owned());
        }
        let side = str_of(&effect["connection_side"]);
        if side != "None" {
            row.push("-side".to_owned());
            row.push(side.to_owned());
        }
        let mut lost = false;
        if let Some(dialects) = effect["dialects"].as_array() {
            push_availability_flag(
                &mut row,
                &mut lost,
                &Value::Array(dialects.clone()),
                availability,
            );
        }
        push_lifecycle_flags(&mut row, &mut lost, effect);
        emit_row(out, "side_effects", &row, lost, "");
    }
}

fn manufacturer_rows(out: &mut Out, draft: &Draft) {
    for method in as_array(draft.get("manufacturer_methods").unwrap_or(&Value::Null)) {
        let mut row = vec![
            "manufacturer".to_owned(),
            name_word(str_of(&method["keyword"])),
        ];
        if str_of(&method["visibility"]) == "Unexported" {
            row.push("-unexported".to_owned());
        }
        for (key, flag) in [
            ("names_instance_at", "-names-instance-at"),
            ("definition_body_at", "-definition-body-at"),
        ] {
            if let Some(n) = method[key].as_u64() {
                row.push(flag.to_owned());
                row.push(n.to_string());
            }
        }
        if let Some(n) = method["constructor_args_from"].as_u64()
            && n != 0
        {
            row.push("-constructor-args-from".to_owned());
            row.push(n.to_string());
        }
        out.row(&row, "");
    }
}

/// `object_class NAME ?-superclass {…}? ?-allow-unknown?`
/// `?-method-prefix-matching Enabled|Strict? ?{ method … }?`.
///
/// The ratified spelling puts the class's three one-word facts on the
/// statement and only the method table in the block, so a class with no
/// methods of its own — `SpiceGenTcl`'s `R`, an empty subclass that inherits
/// everything — is one row. The `method` rows are ordinary subcommand bodies:
/// [`ObjectClassSpec::instance_methods`](tcl_registry::spec::ObjectClassSpec::instance_methods)
/// is `&[SubCommand]`, so the same emitter writes both.
fn object_class_block(out: &mut Out, ctx: &mut Ctx<'_>, draft: &Draft) {
    let Some(class) = draft.get("object_class").filter(|value| !value.is_null()) else {
        return;
    };
    let mut row = vec!["object_class".to_owned()];
    let mut lost = false;
    push_word(&mut row, &mut lost, word(str_of(&class["class_name"])));
    let superclasses: Vec<&str> = as_array(&class["superclasses"])
        .iter()
        .filter_map(Value::as_str)
        .collect();
    if !superclasses.is_empty() {
        push_flag(&mut row, &mut lost, "-superclass", list_word(&superclasses));
    }
    if class["allow_unknown_methods"].as_bool() == Some(true) {
        row.push("-allow-unknown".to_owned());
    }
    if str_of(&class["method_prefix_matching"]) == "Enabled" {
        row.push("-method-prefix-matching".to_owned());
        row.push("Enabled".to_owned());
    }
    let methods = as_array(&class["instance_methods"]);
    if lost {
        unwritable(out, "object_class");
        return;
    }
    out.gap();
    if methods.is_empty() {
        out.row(&row, "-superclass");
        return;
    }

    let mut scope = Ctx {
        defaults: ctx.defaults,
        scope: format!("{}::{}", ctx.scope, str_of(&class["class_name"])),
        tables: ctx.tables,
        availability: ctx.availability,
    };
    let mut body = Out::at(out.indent + 1);
    for method in methods {
        let Some(method) = method.as_object() else {
            continue;
        };
        subcommand_block(&mut body, &mut scope, method, "method");
    }
    row.push("{".to_owned());
    out.row(&row, "-superclass");
    out.block(&body);
    out.line("}");
}

/// One `subcommand NAME { … }` block, or the `method NAME { … }` row of an
/// `object_class` — `keyword` names which, since the DSL reuses the subcommand
/// body grammar unchanged for instance methods.
///
/// A subcommand saying only what its arity, detail, and synopsis are is
/// written on one line with `;` separators, the way
/// `irules-http-header.tclspec` writes fourteen of them.
#[allow(clippy::too_many_lines)]
fn subcommand_block(out: &mut Out, parent: &mut Ctx<'_>, sub: &Draft, keyword: &str) {
    let name = str_of(sub.get("name").unwrap_or(&Value::Null));
    let defaults = draft::default_subcommand_draft();
    let mut ctx = Ctx {
        defaults: &defaults,
        scope: format!("{}::{name}", parent.scope),
        tables: parent.tables,
        availability: parent.availability,
    };
    let ctx = &mut ctx;

    let mut body = Out::at(out.indent + 1);
    let out_body = &mut body;

    arity_row(out_body, ctx, sub);
    arity_window_rows(out_body, ctx, sub);
    if ctx.set(sub, "detail") {
        match braced(str_of(&sub["detail"])) {
            Some(spelling) => out_body.line(&format!("detail {spelling}")),
            None => unwritable(out_body, "detail"),
        }
    }
    if ctx.set(sub, "synopsis") {
        match braced(str_of(&sub["synopsis"])) {
            Some(spelling) => out_body.line(&format!("synopsis {spelling}")),
            None => unwritable(out_body, "synopsis"),
        }
    }
    set_word(out_body, ctx, sub, "traits");
    set_word(out_body, ctx, sub, "dialects");
    text(out_body, ctx, sub, "introduced_version");
    text(out_body, ctx, sub, "deprecated_version");
    text(out_body, ctx, sub, "retired_version");
    set_word(out_body, ctx, sub, "safe_on_uninit");
    expr_row(out_body, ctx, sub, "deprecation_fix", deprecation_fix_row);

    enum_word(out_body, ctx, sub, "return_type");
    if ctx.set(sub, "var_write_typing")
        && let Some(value) = sub["var_write_typing"].as_str()
    {
        match var_write_typing_word(value) {
            Some(spelling) => out_body.line(&format!("var_write_typing {spelling}")),
            None => todo(out_body, "var_write_typing"),
        }
    }
    expr_word(out_body, ctx, sub, "return_elements", return_elements_word);
    expr_word(
        out_body,
        ctx,
        sub,
        "var_elements_effect",
        var_elements_effect_word,
    );
    expr_word(
        out_body,
        ctx,
        sub,
        "representation_effect",
        representation_effect_word,
    );
    enum_word(out_body, ctx, sub, "inferred_storage_type");
    enum_word(out_body, ctx, sub, "body_kind");
    expr_word(
        out_body,
        ctx,
        sub,
        "byte_array_effect",
        byte_array_effect_word,
    );
    enum_word(out_body, ctx, sub, "pattern_type");
    enum_word(out_body, ctx, sub, "format_string_type");

    flag(out_body, ctx, sub, "pure");
    flag(out_body, ctx, sub, "mutator");
    flag(out_body, ctx, sub, "destructive");
    flag(out_body, ctx, sub, "returns_path");
    flag(out_body, ctx, sub, "is_unescape");
    flag(out_body, ctx, sub, "loop_list_header");
    flag(out_body, ctx, sub, "creates_scope_alias");
    flag(out_body, ctx, sub, "arg_values_accept_prefix");
    count(out_body, ctx, sub, "body_arg_implicit_args");
    if ctx.set(sub, "defines_command_at") {
        out_body.line(&format!("defines_command_at {}", sub["defines_command_at"]));
    }
    if ctx.set(sub, "max_leading_option_words") {
        out_body.line(&format!(
            "max_leading_option_words {}",
            sub["max_leading_option_words"]
        ));
    }
    if ctx.set(sub, "min_abbrev") {
        out_body.line(&format!("min_abbrev {}", sub["min_abbrev"]));
    }
    enum_word(out_body, ctx, sub, "prefix_matching");
    text(out_body, ctx, sub, "cfg_rewrite_name");

    arg_row_statements(out_body, ctx, sub);
    callback_taint_input_table(out_body, ctx, sub);
    if ctx.set(sub, "repeated_args")
        && let Some(expr) = sub["repeated_args"].as_str()
    {
        match repeat_rows(expr) {
            Some(rows) => {
                for row in rows {
                    out_body.row(&row, "");
                }
            }
            None => todo(out_body, "repeated_args"),
        }
    }

    native_hook(out_body, ctx, sub, "arg_role_resolver");
    native_hook(out_body, ctx, sub, "command_prefix_resolver");
    native_hook(out_body, ctx, sub, "script_timing_resolver");
    native_hook(out_body, ctx, sub, "const_fold");
    native_hook(out_body, ctx, sub, "const_fold_versioned");
    native_hook(out_body, ctx, sub, "literal_argument_validator");
    catalogue_hook(out_body, ctx, sub, "lowering_hook");
    catalogue_hook(out_body, ctx, sub, "codegen_hook");
    catalogue_hook(out_body, ctx, sub, "inline_codegen_hook");
    catalogue_hook(out_body, ctx, sub, "analyser_hook");
    gap_todo(out_body, ctx, sub, "semantic_operation");
    gap_todo(out_body, ctx, sub, "completion");
    gap_todo(out_body, ctx, sub, "dispatch_dependencies");
    gap_todo(out_body, ctx, sub, "result_stability");
    gap_todo(out_body, ctx, sub, "subcommand_forms");

    enum_word(out_body, ctx, sub, "command_table_effect");
    side_effect_rows(out_body, sub, ctx.availability);
    gap_todo(out_body, ctx, sub, "world_effects");
    gap_todo(out_body, ctx, sub, "state_transitions");

    text(out_body, ctx, sub, "taint_output_sink");
    set_word(out_body, ctx, sub, "taint_transform");
    set_word(out_body, ctx, sub, "taint_double_encode_colour");
    if ctx.set(sub, "credential_arg") {
        out_body.line(&format!("credential_arg {}", sub["credential_arg"]));
    }
    text_list(out_body, ctx, sub, "sensitive_headers");

    option_block(out_body, ctx, sub);
    versioned_arg_value_rows_for(out_body, ctx, sub);
    for row in as_array(sub.get("sub_subcommands").unwrap_or(&Value::Null)) {
        let mut words = vec!["sub_subcommand".to_owned(), name_word(str_of(&row["name"]))];
        let mut lost = false;
        let detail = str_of(&row["detail"]);
        if !detail.is_empty() {
            push_flag(&mut words, &mut lost, "-detail", braced(detail));
        }
        let synopsis = str_of(&row["synopsis"]);
        if !synopsis.is_empty() {
            push_flag(&mut words, &mut lost, "-synopsis", braced(synopsis));
        }
        if let Some(dialects) = row["dialects"].as_array() {
            push_availability_flag(
                &mut words,
                &mut lost,
                &Value::Array(dialects.clone()),
                ctx.availability,
            );
        }
        push_lifecycle_flags(&mut words, &mut lost, row);
        // A second-level operation with its own option table (`namespace
        // ensemble create` vs `configure`, issue #1610) takes a block body,
        // exactly as a subcommand does. An operation that declares *nothing*
        // about options — the JSON `null` — keeps the flag row and inherits;
        // an operation declaring an empty table gets an empty block, because
        // "no options here" and "ask the parent" are different claims and the
        // DSL has to be able to make both.
        let declared = row.get("options").filter(|v| !v.is_null());
        let mut ops = Out::at(out_body.indent + 1);
        for option in as_array(declared.unwrap_or(&Value::Null)).to_vec() {
            option_row(&mut ops, ctx, &option);
        }
        if lost || declared.is_none() {
            emit_row(out_body, "sub_subcommands", &words, lost, "-detail");
        } else if ops.text.is_empty() {
            out_body.line(&format!("{} {{}}", words.join(" ")));
        } else {
            out_body.line(&format!("{} {{", words.join(" ")));
            out_body.block(&ops);
            out_body.line("}");
        }
    }
    hover_block(out_body, ctx, sub);

    // A subcommand saying only its arity, detail, and synopsis goes on one
    // line — `;` separates statements exactly as a newline does.
    let one_liner = format!(
        "{keyword} {} {{ {} }}",
        name_word(name),
        body.statements.join(" ; ")
    );
    let simple = body.statements.len() <= 3
        && !body
            .statements
            .iter()
            .any(|line| line.ends_with('{') || line.contains('\n') || line.starts_with('#'));
    // Consecutive one-liners are a table and read as one; a block always gets
    // air around it. That is exactly how `irules-http-header.tclspec` lays its
    // fourteen subcommands out.
    let after_one_liner = out
        .statements
        .last()
        .is_some_and(|last| last.starts_with(&format!("{keyword} ")) && last.ends_with('}'));
    if simple && out.indent * 4 + one_liner.len() <= ONE_LINE_COLUMN {
        if !after_one_liner {
            out.gap();
        }
        out.line(&one_liner);
        out.losses.extend(body.losses);
        return;
    }

    out.gap();
    out.line(&format!("{keyword} {} {{", name_word(name)));
    out.block(&body);
    out.line("}");
}

// ---------------------------------------------------------------------------
// Entry points
// ---------------------------------------------------------------------------

/// Render one draft as a complete single-command `.tclspec` pack.
///
/// The pack takes its name from the dialect the draft was seeded from
/// ([`SOURCE_DIALECT_KEY`]) when it has one, which is what the ports do —
/// `speclib tcl 1.2`, `speclib f5-irules 1.2` — the newest vocabulary the
/// loader reads, per [`DSL_VERSION`].
#[must_use]
pub fn render(draft: &Draft) -> String {
    let pack = draft
        .get(SOURCE_DIALECT_KEY)
        .and_then(Value::as_str)
        .unwrap_or("pack")
        .to_owned();
    render_pack(std::slice::from_ref(draft), &pack)
}

/// Render a pack of drafts as one `.tclspec` file.
#[must_use]
pub fn render_pack(drafts: &[Draft], pack_name: &str) -> String {
    render_pack_with_version(drafts, pack_name, DSL_VERSION)
}

/// Render a pack of drafts with its declared `SpecTcl` vocabulary version.
///
/// A newly-created or exported pack uses [`DSL_VERSION`], while a document
/// loaded into the studio must retain the version its own `speclib` header
/// declared when a canonical re-render or write-back fallback rebuilds it.
#[must_use]
pub fn render_pack_with_version(drafts: &[Draft], pack_name: &str, dsl_version: &str) -> String {
    render_pack_reporting_with_version(drafts, pack_name, dsl_version).0
}

/// [`render_pack`], plus every field the render could not carry.
///
/// Each [`Loss`] corresponds to one `# TODO(spectcl):` line in the text, so a
/// caller can list what the pack does not say without re-reading its comments.
#[must_use]
pub fn render_pack_reporting(drafts: &[Draft], pack_name: &str) -> (String, Vec<Loss>) {
    render_pack_reporting_with_version(drafts, pack_name, DSL_VERSION)
}

/// [`render_pack_reporting`] with an explicit `speclib` vocabulary version.
///
/// Kept separate from the default entry point so exports continue to target
/// the newest vocabulary, while an already-authored document round-trips its
/// header byte-for-byte in meaning.
#[must_use]
pub fn render_pack_reporting_with_version(
    drafts: &[Draft],
    pack_name: &str,
    dsl_version: &str,
) -> (String, Vec<Loss>) {
    let mut tables = ValueTables::default();
    let mut bodies: Vec<(String, Out)> = Vec::new();
    // The header decides the vocabulary the body may use, never the other way
    // round: `available` is a 2.0 word, so a pack declaring less keeps the
    // legacy `dialects` spelling and stays internally consistent.
    let availability =
        !tcl_registry::version::compare(dsl_version, tcl_spectcl::NEWEST_VOCABULARY_VERSION)
            .is_lt();

    for draft in drafts {
        let name = str_of(draft.get("name").unwrap_or(&Value::Null)).to_owned();
        let defaults = draft::default_command_draft();
        // Rendered at its final indent: a braced word's continuation lines are
        // part of the value and must never be shifted afterwards.
        let mut body = Out::at(1);
        {
            let mut ctx = Ctx {
                defaults: &defaults,
                scope: name.clone(),
                tables: &mut tables,
                availability,
            };
            command_body(&mut body, &mut ctx, draft);
        }
        bodies.push((name, body));
    }

    let mut out = Out::default();
    out.comment(
        "Rendered from the tcl-lsp command registry by the spec studio's SpecTcl \
         renderer (rust/tcl-spec-studio/src/render_spectcl.rs).",
    );
    out.comment(
        "The syntax is docs/design/spec-dsl-examples/README.md. Every `TODO(spectcl)` \
         line below names a field the source spec sets that a pack cannot yet say.",
    );
    out.line("");
    out.line(&format!(
        "speclib {} {dsl_version} {{",
        name_word(pack_name)
    ));
    // The ports do not indent a pack's own declarations — a `.tclspec` is one
    // long file of `command` blocks, and a second level of indentation buys
    // nothing.
    tables.render(&mut out);
    for (name, body) in &bodies {
        out.gap();
        out.line(&format!("command {} {{", name_word(name)));
        out.block(body);
        out.line("}");
    }
    out.gap();
    out.line("}");
    (out.text, out.losses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn drafted(name: &str) -> Draft {
        let mut d = draft::default_command_draft();
        d.insert("name".into(), json!(name));
        d
    }

    #[test]
    fn invalid_taint_link_is_not_rendered_without_var_write_role() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let entry = registry.get("entry").expect("Tk entry spec");
        let seeded = draft::from_command_spec(entry);
        let mut option = seeded["options"]
            .as_array()
            .expect("entry options")
            .iter()
            .find(|option| str_of(&option["name"]) == "-textvariable")
            .expect("entry -textvariable")
            .clone();
        option["value"]["role"] = json!("Value");
        option["value"]["also_role"] = Value::Null;
        option["value"]["taints_var_write"] = json!(true);
        let mut draft = drafted("probe");
        draft.insert("options".into(), Value::Array(vec![option]));

        let text = render_pack(&[draft], "probe");
        assert!(!text.contains("-taints-var-write"), "{text}");
    }

    #[test]
    fn a_default_draft_renders_a_pack_that_says_nothing_else() {
        let text = render_pack(&[drafted("mycommand")], "probe");
        assert!(text.contains(&format!("speclib probe {DSL_VERSION} {{")));
        assert!(text.contains("command mycommand {"));
        let statements: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#') && !line.starts_with('}'))
            .collect();
        assert_eq!(
            statements,
            vec![
                format!("speclib probe {DSL_VERSION} {{").as_str(),
                "command mycommand {"
            ],
            "a default draft must emit only what the author actually set:\n{text}"
        );
    }

    /// The 2.0 availability algebra, spelled the way the loader reads it
    /// back — one row per contiguous run of releases, open at the top when
    /// the run reaches the newest.
    #[test]
    fn a_dialect_set_is_written_in_the_availability_algebra() {
        let rows = |members: &[&str]| availability_rows(members);
        assert_eq!(
            rows(&["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"]),
            Some(vec!["tcl 8.4-".to_owned()])
        );
        assert_eq!(
            rows(&["tcl8.6", "tcl9.0", "tcl9.1"]),
            Some(vec!["tcl 8.6-".to_owned()])
        );
        // The bound is exclusive, so a run stopping at 8.6 names 9.0.
        assert_eq!(
            rows(&["tcl8.4", "tcl8.5", "tcl8.6"]),
            Some(vec!["tcl 8.4-9.0".to_owned()])
        );
        assert_eq!(
            rows(&["tcl8.4", "tcl8.5"]),
            Some(vec!["tcl 8.4-8.6".to_owned()])
        );
        // A bare `X.Y` names that release line only — exactly one bit.
        assert_eq!(rows(&["tcl8.5"]), Some(vec!["tcl 8.5".to_owned()]));
        // A hole in the ladder is two rows, which the loader unions.
        assert_eq!(
            rows(&["tcl8.4", "tcl8.6"]),
            Some(vec!["tcl 8.4".to_owned(), "tcl 8.6".to_owned()])
        );
        assert_eq!(
            rows(&["tcl8.6", "tcl9.0", "tcl9.1", "f5-irules"]),
            Some(vec!["tcl 8.6-".to_owned(), "f5-irules".to_owned()])
        );
        // `tk` is a package pin, and the algebra says so.
        assert_eq!(rows(&["tk"]), Some(vec!["package Tk".to_owned()]));

        // A bit the closed provider list does not name keeps `dialects`, and
        // so does the empty (declared-unavailable) set.
        assert_eq!(rows(&["f5-iapps"]), None);
        assert_eq!(rows(&["tcl9.0", "bpf"]), None);
        assert_eq!(rows(&[]), None);

        // The flag form: one row bare, several wrapped into one list word.
        assert_eq!(
            available_flag(&["tcl8.6", "tcl9.0", "tcl9.1"]).as_deref(),
            Some("{tcl 8.6-}")
        );
        assert_eq!(
            available_flag(&["tcl8.6", "tcl9.0", "tcl9.1", "f5-irules"]).as_deref(),
            Some("{{tcl 8.6-} {f5-irules}}")
        );
    }

    /// The rendered rows, end to end, over a real shipped command: the
    /// command scope takes the statement spelling and an option takes the
    /// flag, and both load back to the very bits the draft held.
    #[test]
    fn availability_rows_reload_to_the_set_they_were_written_from() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let seeded = draft::from_command_spec(registry.get("lsort").expect("the lsort spec"));
        let text = render_pack(std::slice::from_ref(&seeded), "probe");
        // `{all-tcl f5-irules}` in the algebra, and `-nocase`'s 8.5+ gate.
        assert!(text.contains("available {tcl 8.4-} {f5-irules}"), "{text}");
        assert!(text.contains("-available {tcl 8.5-}"), "{text}");
        // …and not a legacy row anywhere: the prose may say the word
        // "dialects", so the check is on the row spellings themselves.
        assert!(!text.contains("dialects {all-tcl f5-irules}"), "{text}");
        assert!(!text.contains("-dialects tcl8.5+"), "{text}");

        let pack = crate::spectcl::load_pack(&text);
        let spec = pack.commands.first().expect("the command").spec;
        let reloaded = draft::from_command_spec(spec);
        assert_eq!(reloaded["dialects"], seeded["dialects"]);
        let gate = |draft: &Draft, option: &str| -> Value {
            as_array(&draft["options"])
                .iter()
                .find(|o| str_of(&o["name"]) == option)
                .map_or(Value::Null, |o| o["dialects"].clone())
        };
        for option in ["-nocase", "-indices", "-stride", "-ascii"] {
            assert_eq!(
                gate(&reloaded, option),
                gate(&seeded, option),
                "{option} lost its availability:\n{text}"
            );
        }
    }

    /// The header decides the spelling, and the two never disagree: a pack
    /// declaring the newest vocabulary takes `available`, and one rendered
    /// under an older declared vocabulary keeps the legacy word — so neither
    /// earns the loader's per-site "newer than this pack declares" notice
    /// (#1627, in both directions).
    #[test]
    fn the_header_and_the_body_agree_on_their_vocabulary() {
        assert_eq!(DSL_VERSION, tcl_spectcl::NEWEST_VOCABULARY_VERSION);
        let registry = tcl_registry::CommandRegistry::build_default();
        let seeded = draft::from_command_spec(registry.get("lsort").expect("the lsort spec"));

        let newest = render_pack(std::slice::from_ref(&seeded), "probe");
        assert!(
            newest.contains(&format!("speclib probe {DSL_VERSION} {{")),
            "{newest}"
        );
        assert!(newest.contains("available {tcl 8.4-}"), "{newest}");
        assert!(
            !crate::spectcl::load_pack(&newest)
                .notices
                .iter()
                .any(|notice| notice
                    .message
                    .contains("vocabulary, but this pack declares")),
            "{:#?}",
            crate::spectcl::load_pack(&newest).notices
        );

        let older = render_pack_with_version(&[seeded], "probe", "1.2");
        assert!(older.contains("speclib probe 1.2 {"), "{older}");
        assert!(older.contains("dialects {all-tcl f5-irules}"), "{older}");
        assert!(!older.contains("available {tcl"), "{older}");
        assert!(!older.contains("-available "), "{older}");
        assert!(
            !crate::spectcl::load_pack(&older)
                .notices
                .iter()
                .any(|notice| notice
                    .message
                    .contains("vocabulary, but this pack declares")),
            "{:#?}",
            crate::spectcl::load_pack(&older).notices
        );
    }

    #[test]
    fn arity_takes_its_six_documented_spellings() {
        for (min, max, step, also, expected) in [
            (3u64, Some(3u64), 0u64, None, "arity 3"),
            (1, Some(2), 0, None, "arity 1..2"),
            (1, None, 0, None, "arity 1.."),
            (0, Some(2), 0, None, "arity ..2"),
            // `arity ..` on its own is the default and is never emitted, so
            // the unbounded spelling is only visible with a modifier.
            (0, None, 2, None, "arity .. -step 2"),
            (3, None, 2, None, "arity 3.. -step 2"),
            (3, None, 2, Some(2), "arity 3.. -step 2 -also 2"),
        ] {
            let mut d = drafted("probe");
            d.insert(
                "arity".into(),
                json!({ "min": min, "max": max, "step": step, "also_exact": also }),
            );
            let text = render_pack(&[d], "probe");
            assert!(text.contains(expected), "expected `{expected}` in:\n{text}");
        }
    }

    /// A braced word is the only faithful spelling, so text braces cannot hold
    /// has none at all — the quoted form keeps its backslashes and normalises
    /// `$x` to `${x}` on the way through the CST.
    #[test]
    fn only_a_braced_word_carries_prose_verbatim() {
        assert_eq!(braced("plain text").as_deref(), Some("{plain text}"));
        assert_eq!(
            braced("a {nested} word").as_deref(),
            Some("{a {nested} word}")
        );
        // `$` and `[` are literal inside braces, so they need nothing.
        assert_eq!(braced("$x [y]").as_deref(), Some("{$x [y]}"));
        // A lone opening brace, a trailing backslash, and a backslash-newline
        // (the one substitution Tcl still performs inside braces) have no
        // faithful spelling at all.
        assert_eq!(braced("a { lone"), None);
        assert_eq!(braced("trailing\\"), None);
        assert_eq!(braced("wrapped \\\nline"), None);
    }

    #[test]
    fn a_native_hook_is_named_after_the_field_it_fills() {
        let mut d = drafted("probe");
        d.insert(
            draft::UNRENDERABLE_KEY.into(),
            json!(["arg_role_resolver", "script_timing_resolver", "const_fold"]),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains("arg_role_resolver -native probe::arg_role_resolver"));
        assert!(
            text.contains("script_timing_resolver -native probe::script_timing_resolver"),
            "{text}"
        );
        assert!(text.contains("const_fold -native probe::const_fold"));
    }

    #[test]
    fn a_field_the_dsl_cannot_carry_becomes_a_todo_naming_it() {
        let mut d = drafted("probe");
        d.insert(draft::UNRENDERABLE_KEY.into(), json!(["frame_effect"]));
        let text = render_pack(&[d], "probe");
        assert!(
            text.contains("TODO(spectcl): `frame_effect`"),
            "the TODO must name the field:\n{text}"
        );
        assert!(text.contains("frame_effect -level-word W -layout L"));
    }

    #[test]
    fn values_carrying_details_are_hoisted_to_a_shared_table() {
        let mut d = drafted("probe");
        d.insert(
            "arg_values".into(),
            json!([{
                "index": 0,
                "values": [
                    { "value": "alnum", "detail": "Any alphanumeric.", "min_tcl": null, "code": null },
                    { "value": "dict", "detail": "A dict.", "min_tcl": "V9_0", "code": null },
                ],
            }]),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains("values probe-arg0 {"), "{text}");
        assert!(
            text.contains("value alnum -detail {Any alphanumeric.}"),
            "{text}"
        );
        assert!(text.contains("value dict -min-tcl tcl9.0"), "{text}");
        assert!(text.contains("arg 0 -values-from probe-arg0"), "{text}");
    }

    #[test]
    fn a_plain_value_set_stays_on_the_row() {
        let mut d = drafted("probe");
        d.insert(
            "arg_values".into(),
            json!([{
                "index": 1,
                "values": [
                    { "value": "yes", "detail": "", "min_tcl": null, "code": null },
                    { "value": "no", "detail": "", "min_tcl": null, "code": null },
                ],
            }]),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains("arg 1 -values {yes no}"), "{text}");
        assert!(!text.contains("values probe"), "{text}");
    }

    #[test]
    fn a_repeated_arg_layout_round_trips_through_its_rust_expression() {
        let rows = repeat_rows(
            "&[RepeatedArgLayout { role: ArgRole::LoopVarList, start: 0, stride: 2, \
             exclude_trailing: 1, optional_leading_word: false, conditional_binding: false }]",
        )
        .expect("the layout parses");
        assert_eq!(
            rows[0].join(" "),
            "repeat LoopVarList -stride 2 -exclude-trailing 1"
        );
    }

    #[test]
    fn an_option_constraint_keeps_its_dialect_gate() {
        const EXPR: &str = "&[OptionRelation { kind: RelationKind::MutuallyExclusive, \
                            subject: None, terms: &[RelationTerm::Option(\"-glob\"), \
                            RelationTerm::Option(\"-regexp\")], \
                            dialects: Some(DialectSet::ALL_TCL.union(DialectSet::IRULES)) }]";
        // Under 2.0 the gate is written in the availability algebra …
        let rows = option_conflict_rows(EXPR, true).expect("the constraint parses");
        assert_eq!(
            rows[0].join(" "),
            "option_conflict {-glob -regexp} -available {{tcl 8.4-} {f5-irules}}"
        );
        // … and under an older declared vocabulary, in the legacy word.
        let rows = option_conflict_rows(EXPR, false).expect("the constraint parses");
        assert_eq!(
            rows[0].join(" "),
            "option_conflict {-glob -regexp} -dialects {all-tcl f5-irules}"
        );
    }

    /// **The E-R14 export gate.** Each of the three new relation kinds, and
    /// each of the four term shapes, must come back out as the statement an
    /// author would have written — otherwise a pack that round-trips through
    /// the studio silently loses a relation.
    #[test]
    fn every_relation_kind_and_term_shape_round_trips_to_its_statement() {
        for (expr, expected) in [
            (
                "&[OptionRelation { kind: RelationKind::Requires, \
                 subject: Some(RelationTerm::Option(\"-command\")), \
                 terms: &[RelationTerm::Option(\"-channel\")], dialects: None, \
                 message: None }]",
                "option_requires -command -channel",
            ),
            (
                "&[OptionRelation { kind: RelationKind::RequiresOneOf, subject: None, \
                 terms: &[RelationTerm::Option(\"-channel\"), RelationTerm::Argument(0)], \
                 dialects: None, message: Some(\"Neither -channel nor text specified\") }]",
                "option_requires_one_of {} {-channel {arg 0}} \
                 -message {Neither -channel nor text specified}",
            ),
            (
                "&[OptionRelation { kind: RelationKind::Forbids, \
                 subject: Some(RelationTerm::OptionValue(\"-order\", \"in\")), \
                 terms: &[RelationTerm::OptionValue(\"-type\", \"bfs\")], \
                 dialects: None, message: None }]",
                "option_forbids {-order in} {{-type bfs}}",
            ),
            (
                "&[OptionRelation { kind: RelationKind::Forbids, \
                 subject: Some(RelationTerm::Option(\"-channel\")), \
                 terms: &[RelationTerm::ArgumentValue(1, \"text\")], \
                 dialects: None, message: None }]",
                "option_forbids -channel {{arg 1 text}}",
            ),
        ] {
            let rows = option_conflict_rows(expr, true).expect("the relation parses");
            assert_eq!(rows[0].join(" "), expected, "{expr}");
        }
    }

    #[test]
    fn a_simple_subcommand_is_written_on_one_line() {
        let mut d = drafted("probe");
        let mut sub = draft::default_subcommand_draft();
        sub.insert("name".into(), json!("at"));
        sub.insert(
            "arity".into(),
            json!({ "min": 1, "max": 1, "step": 0, "also_exact": null }),
        );
        sub.insert("detail".into(), json!("Get header name by index."));
        d.insert("subcommands".into(), json!([Value::Object(sub)]));
        let text = render_pack(&[d], "probe");
        assert!(
            text.contains("subcommand at { arity 1 ; detail {Get header name by index.} }"),
            "{text}"
        );
    }

    /// The class's three one-word facts ride on the statement; only the
    /// method table goes in the block, and a method is written with the
    /// subcommand body grammar under a `method` keyword.
    #[test]
    fn an_object_class_writes_its_flags_on_the_row_and_its_methods_in_the_block() {
        let mut d = drafted("probe");
        let mut method = draft::default_subcommand_draft();
        method.insert("name".into(), json!("configure"));
        method.insert(
            "arity".into(),
            json!({ "min": 1, "max": 1, "step": 0, "also_exact": null }),
        );
        method.insert("detail".into(), json!("Reconfigure the widget."));
        d.insert(
            "object_class".into(),
            json!({
                "class_name": "::probe::Widget",
                "instance_methods": [Value::Object(method)],
                "superclasses": ["::probe::Base", "::probe::Mixin"],
                "allow_unknown_methods": true,
                "method_prefix_matching": "Enabled",
            }),
        );
        let text = render_pack(&[d], "probe");
        assert!(text.contains("object_class ::probe::Widget"), "{text}");
        assert!(
            text.contains("-superclass {::probe::Base ::probe::Mixin}"),
            "{text}"
        );
        assert!(text.contains("-allow-unknown"), "{text}");
        assert!(text.contains("-method-prefix-matching Enabled"), "{text}");
        assert!(
            text.contains("method configure { arity 1 ; detail {Reconfigure the widget.} }"),
            "{text}"
        );
    }

    /// A class with no methods of its own — `SpiceGenTcl`'s `R`, an empty
    /// subclass — is one row, not an empty block.
    #[test]
    fn an_object_class_with_no_methods_is_a_single_row() {
        let mut d = drafted("probe");
        d.insert(
            "object_class".into(),
            json!({
                "class_name": "::probe::R",
                "instance_methods": [],
                "superclasses": ["::probe::Resistor"],
                "allow_unknown_methods": false,
                "method_prefix_matching": "Strict",
            }),
        );
        let text = render_pack(&[d], "probe");
        assert!(
            text.contains("object_class ::probe::R -superclass ::probe::Resistor\n"),
            "{text}"
        );
        assert!(!text.contains("-allow-unknown"), "{text}");
    }

    /// The gate in miniature: a spec carrying an `object_class` renders,
    /// loads, and drafts back **equal** — no notices, no gap.
    #[test]
    fn an_object_class_survives_render_load_draft() {
        static METHODS: &[tcl_registry::spec::SubCommand] = &[tcl_registry::spec::SubCommand {
            name: "Add",
            arity: tcl_registry::arity::Arity::at_least(1),
            detail: "Add a series by literal type name.",
            synopsis: "$chart Add seriesType ?-opt value ...?",
            mutator: true,
            ..tcl_registry::spec::SubCommand::DEFAULT
        }];
        static CLASS: tcl_registry::spec::ObjectClassSpec = tcl_registry::spec::ObjectClassSpec {
            class_name: "probe::chart",
            instance_methods: METHODS,
            superclasses: &["probe::base"],
            allow_unknown_methods: true,
            method_prefix_matching: tcl_registry::abbrev::PrefixMatching::Enabled,
        };
        let spec = tcl_registry::spec::CommandSpec {
            name: "probe::chart",
            arity: tcl_registry::arity::Arity::at_least(1),
            object_class: Some(&CLASS),
            ..tcl_registry::spec::CommandSpec::DEFAULT
        };

        let before = draft::from_command_spec(&spec);
        let text = render_pack(std::slice::from_ref(&before), "probe");
        assert!(text.contains("-method-prefix-matching Enabled"), "{text}");
        let pack = crate::spectcl::load_pack(&text);
        assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
        let reloaded = pack.command("probe::chart").expect("the command reloads");
        let after = draft::from_command_spec(reloaded.spec);
        assert_eq!(
            after.get("object_class"),
            before.get("object_class"),
            "{text}"
        );
        assert_eq!(
            after.get(draft::UNRENDERABLE_KEY),
            before.get(draft::UNRENDERABLE_KEY),
            "{text}"
        );
        assert!(
            gap("object_class").is_none(),
            "`object_class` is no longer a renderer gap"
        );
    }

    #[test]
    fn option_script_timings_survive_render_load_draft() {
        static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[
            tcl_registry::hover::OptionSpec {
                name: "-command",
                value: tcl_registry::hover::OptionValue::deferred_script(),
                detail: "Run after activation.",
                ..tcl_registry::hover::OptionSpec::DEFAULT
            },
            tcl_registry::hover::OptionSpec {
                name: "-remove",
                value: tcl_registry::hover::OptionValue::Takes(tcl_registry::hover::OptionArg {
                    role: tcl_registry::ArgRole::CommandPrefix,
                    script_timing: tcl_registry::hover::ScriptTiming::ReferenceOnly,
                    hint: "prefix",
                    ..tcl_registry::hover::OptionArg::DEFAULT
                }),
                detail: "Match a prior registration without invoking it.",
                ..tcl_registry::hover::OptionSpec::DEFAULT
            },
        ];
        let spec = tcl_registry::spec::CommandSpec {
            name: "probe::button",
            options: OPTIONS,
            ..tcl_registry::spec::CommandSpec::DEFAULT
        };

        let before = draft::from_command_spec(&spec);
        let text = render_pack(std::slice::from_ref(&before), "probe");
        assert!(text.contains("-script-timing Deferred"), "{text}");
        assert!(text.contains("-script-timing ReferenceOnly"), "{text}");
        let pack = crate::spectcl::load_pack(&text);
        assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
        let reloaded = pack.command("probe::button").expect("the command reloads");
        let after = draft::from_command_spec(reloaded.spec);
        assert_eq!(after.get("options"), before.get("options"), "{text}");
    }

    #[test]
    fn callback_taint_inputs_survive_option_and_positional_round_trip() {
        static VALIDATION: &[tcl_registry::CallbackTaintInput] = &[
            tcl_registry::CallbackTaintInput::TK_PROPOSED_VALUE,
            tcl_registry::CallbackTaintInput::TK_EDIT_TEXT,
        ];
        static EVENT: &[tcl_registry::CallbackTaintInput] = &[
            tcl_registry::CallbackTaintInput::TK_EVENT_CHAR,
            tcl_registry::CallbackTaintInput::TK_EVENT_KEYSYM,
        ];
        static POSITIONAL: &[(u8, &[tcl_registry::CallbackTaintInput])] = &[(2, EVENT)];
        static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[tcl_registry::hover::OptionSpec {
            name: "-validatecommand",
            value: tcl_registry::hover::OptionValue::deferred_tainted_script(VALIDATION),
            ..tcl_registry::hover::OptionSpec::DEFAULT
        }];
        let spec = tcl_registry::spec::CommandSpec {
            name: "probe::callback",
            traits: tcl_registry::Traits::DEFERS_BODY,
            arity: tcl_registry::Arity::exact(3),
            arg_roles: &[(2, tcl_registry::ArgRole::Body)],
            callback_taint_inputs: POSITIONAL,
            options: OPTIONS,
            ..tcl_registry::spec::CommandSpec::DEFAULT
        };

        let before = draft::from_command_spec(&spec);
        let text = render_pack(std::slice::from_ref(&before), "probe");
        assert!(text.contains("-callback-taint-inputs {%P %S}"), "{text}");
        assert!(
            text.contains("callback_taint_inputs {{2 {%A %K}}}"),
            "{text}"
        );
        let pack = crate::spectcl::load_pack(&text);
        assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
        let after = draft::from_command_spec(
            pack.command("probe::callback")
                .expect("the command reloads")
                .spec,
        );
        assert_eq!(
            after.get("callback_taint_inputs"),
            before.get("callback_taint_inputs"),
            "{text}"
        );
        assert_eq!(after.get("options"), before.get("options"), "{text}");
    }

    #[test]
    fn an_option_variable_scope_survives_render_load_draft() {
        static OPTIONS: &[tcl_registry::hover::OptionSpec] = &[tcl_registry::hover::OptionSpec {
            name: "-textvariable",
            value: tcl_registry::hover::OptionValue::user_input_var(),
            detail: "Global linked input.",
            ..tcl_registry::hover::OptionSpec::DEFAULT
        }];
        let spec = tcl_registry::spec::CommandSpec {
            name: "probe::entry",
            options: OPTIONS,
            ..tcl_registry::spec::CommandSpec::DEFAULT
        };

        let before = draft::from_command_spec(&spec);
        let text = render_pack(std::slice::from_ref(&before), "probe");
        assert!(text.contains("-variable-scope Global"), "{text}");
        let pack = crate::spectcl::load_pack(&text);
        assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
        let reloaded = pack.command("probe::entry").expect("the command reloads");
        let after = draft::from_command_spec(reloaded.spec);
        assert_eq!(after.get("options"), before.get("options"), "{text}");
    }

    #[test]
    fn tk_geometry_preserves_a_custom_container_option() {
        let spec = tcl_registry::spec::CommandSpec {
            name: "probe::layout",
            tk_geometry: Some(tcl_registry::tk_geometry::TkGeometryManagerSpec {
                container_policy: tcl_registry::tk_geometry::TkGeometryContainerPolicy::Exclusive,
                container_option: Some("-inside"),
                direct_form: false,
                placement_subcommand: Some("arrange"),
                release_subcommands: &["release", "unmanage"],
            }),
            ..tcl_registry::spec::CommandSpec::DEFAULT
        };

        let before = draft::from_command_spec(&spec);
        let text = render_pack(std::slice::from_ref(&before), "probe");
        assert!(text.contains("tk_geometry Exclusive"), "{text}");
        assert!(text.contains("-container-option -inside"), "{text}");
        assert!(text.contains("-placement-subcommand arrange"), "{text}");
        assert!(
            text.contains("-release-subcommands {release unmanage}"),
            "{text}"
        );
        assert!(!text.contains("-container-option -in\n"), "{text}");

        let pack = crate::spectcl::load_pack(&text);
        assert!(pack.notices.is_empty(), "{:?}\n{text}", pack.notices);
        let reloaded = pack.command("probe::layout").expect("command reloads");
        assert_eq!(
            reloaded
                .spec
                .tk_geometry
                .and_then(|geometry| geometry.container_option),
            Some("-inside")
        );
        let geometry = reloaded.spec.tk_geometry.expect("geometry descriptor");
        assert!(!geometry.direct_form);
        assert_eq!(geometry.placement_subcommand, Some("arrange"));
        assert_eq!(geometry.release_subcommands, ["release", "unmanage"]);
    }
}
