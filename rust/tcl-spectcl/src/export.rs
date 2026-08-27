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

//! **Canonical export** — a loaded snapshot back out as straight-line
//! `SpecTcl` source (design E, `docs/design/spectcl-design-e-deep-dive.md`
//! §15.1, E-R11).
//!
//! The canonical form is the *straight-line subset* of the language: literal
//! registration calls only, no `proc`, `foreach`, `set`, or computed
//! argument. Every pack shipped today is already canonical, so for those this
//! is a byte-stable round trip; for a **programmed** pack it is the
//! expansion — what the loops actually registered — which is the affordance
//! that makes a templated pack reviewable (`tcl spec export`, the MCP
//! `spectcl_expand`, and the studio's read-only expansion pane).
//!
//! ## Expansion is total; contraction is never attempted
//!
//! A program is not recovered from its snapshot, ever. Export writes the
//! registrations the evaluation actually made, in the order it made them,
//! and says nothing about the program that made them.
//!
//! ## What it renders from: the registration record
//!
//! Both loaders keep the calls they read as [`Pack::registrations`] — a
//! [`Registration`] per call, nested for `command` and `subcommand` bodies,
//! carrying each word exactly as the loader read it (verbatim braced text,
//! per-word braced-ness). That record *is* the canonical subset: rendering
//! it is a spelling exercise, not a re-derivation from `CommandSpec`, so
//! nothing a `CommandSpec` cannot hold (an inline descriptor block, a hook
//! body, a value table) can be lost on the way out. The studio's
//! draft-driven renderer (`tcl_spec_studio::render_spectcl`) answers the
//! different question of what a *form edit* means, and keeps its own gap
//! register for the fields a draft cannot carry.
//!
//! ## What is deliberately not written down
//!
//! Load-time facts — a notice, a line number, a `degraded` flag, a
//! provenance class — are **derived on reload**, never embedded. An exported
//! pack carries no comments and no provenance markers, so the reloaded
//! snapshot recomputes all of it from the same words. Line numbers therefore
//! change (a templated pack's rows move to where the expansion writes them),
//! and every other field of the snapshot does not: that is exactly what the
//! two round-trip gates in `tests/export.rs` assert.
//!
//! ## Layout
//!
//! The house style of the ports and of the studio's renderer: pack-level
//! declarations sit at the `speclib` body's own margin, a `command` body is
//! indented one level, a `subcommand` body two, and a blank line separates
//! block declarations from what precedes them. A multi-line braced word — a
//! `hover` block, a `detail` prose, a hook body — is written **verbatim**,
//! with only its first line taking the row's indent, because every byte
//! between its braces is the value.

use crate::loader::{Pack, Stmt, Word};

// ---------------------------------------------------------------------------
// The record
// ---------------------------------------------------------------------------

/// One registration call as the loader read it.
///
/// A row (`arity 1..`, `option -nocase …`) has no body; a `command` or
/// `subcommand` declaration carries its block as nested registrations, which
/// is the only nesting either loader descends into. Every other braced word
/// — a `hover` block, an inline descriptor, a hook body — stays a word of
/// its own row, because that is how the loader reads it back.
#[derive(Debug, Clone)]
pub struct Registration {
    /// The statement's words, minus a block body descended into.
    pub(crate) head: Vec<Word>,
    /// The block body, for the two declarations that have one.
    pub(crate) body: Option<Vec<Registration>>,
    /// The line the call was made on, as the loader saw it. Kept for the
    /// studio's "expanded from" labelling; never written into the export.
    pub(crate) line: u32,
}

impl Registration {
    /// A plain row, captured verbatim.
    pub(crate) fn row(stmt: &Stmt) -> Self {
        Self {
            head: stmt.words.clone(),
            body: None,
            line: stmt.line,
        }
    }

    /// A row built from words rather than read from a statement — the
    /// evaluation loader's capture of a declaration that never opened a
    /// block.
    pub(crate) fn row_words(head: Vec<Word>, line: u32) -> Self {
        Self {
            head,
            body: None,
            line,
        }
    }

    /// A `command` / `subcommand` declaration and its block.
    pub(crate) fn block(head: Vec<Word>, body: Vec<Registration>, line: u32) -> Self {
        Self {
            head,
            body: Some(body),
            line,
        }
    }

    /// The registration's own word (`command`, `option`, `arity`, …).
    #[must_use]
    pub fn word(&self) -> &str {
        self.head.first().map_or("", |word| word.text.as_str())
    }

    /// The word at `index`, or `""`.
    #[must_use]
    pub fn arg(&self, index: usize) -> &str {
        self.head.get(index).map_or("", |word| word.text.as_str())
    }

    /// Whether the call carries `flag` as one of its words — how a
    /// declaration's `-override` is read back out of the record.
    #[must_use]
    pub fn has_flag(&self, flag: &str) -> bool {
        self.head.iter().any(|word| word.text == flag)
    }

    /// The line the call was made on.
    #[must_use]
    pub const fn line(&self) -> u32 {
        self.line
    }

    /// The nested registrations of a `command` / `subcommand` block.
    #[must_use]
    pub fn body(&self) -> &[Registration] {
        self.body.as_deref().unwrap_or(&[])
    }
}

/// A word the evaluation captured rather than read from the file.
///
/// Evaluation delivers argument *values*, so the per-word braced-ness the
/// CST carries has to be reconstructed from the one property that decides a
/// spelling: a name is one whitespace-free word, a block is not — the same
/// rule the evaluation loader's own capture uses.
pub(crate) fn synth_word(text: &str, line: u32) -> Word {
    Word {
        braced: text.is_empty() || text.contains(char::is_whitespace),
        text: text.to_owned(),
        line,
    }
}

// ---------------------------------------------------------------------------
// Words
// ---------------------------------------------------------------------------

/// Whether `text` can be written as a bare word.
///
/// The same rule the studio's renderer applies, and for the same reason:
/// anything a bare word would let the lexer substitute (`$`, `[`, a quote, a
/// brace, whitespace, a leading `#`) has to take the braced form instead.
fn bare_safe(text: &str) -> bool {
    !text.is_empty()
        && !text.starts_with('#')
        && text.chars().all(|c| {
            c.is_ascii_alphanumeric() || "-_.:/+@=,<>*?!%^~|'`".contains(c) && c.is_ascii()
        })
}

/// Whether `{text}` is a well-formed braced word the loader reads back as
/// `text` byte for byte.
///
/// The scan is the brace counting Tcl's lexer does: a backslash escapes the
/// next character (so `\{` and `\}` do not nest), the depth never goes
/// negative, and a trailing backslash would escape the closing brace rather
/// than be part of the value.
///
/// **A backslash-newline is fine here**, where the studio's otherwise
/// identical check refuses it. The difference is what the two are rendering:
/// the studio writes a `&'static str` out of the registry and must be sure
/// Tcl's *reader* returns it unchanged, while this renders a word the
/// loader itself read — and the loader keeps every byte between the braces,
/// continuation included (`Word`). The shipped EDA packs write
/// backslash-continued examples inside `hover` blocks, and refusing them
/// here would report a loss on a word that round-trips exactly.
fn brace_safe(text: &str) -> bool {
    let mut depth = 0i32;
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if chars.next().is_none() {
                    return false;
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

/// One word this export could not write down faithfully.
///
/// The only shape that reaches here is a word whose text no *verbatim*
/// spelling carries — unbalanced braces, or a trailing backslash — which the
/// loader itself can only produce from a quoted word, since a braced word is
/// every byte between its braces. The quoted fallback keeps the file
/// parseable and the loss is reported rather than hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportLoss {
    /// The registration word the loss occurred under (`option`, `hover`, …).
    pub word: String,
    /// The line the registration was read from, in the *source* snapshot.
    pub line: u32,
    /// The text that has no verbatim spelling.
    pub text: String,
}

/// `text` as a Tcl word carrying it verbatim, or `None` when nothing does.
fn spelling(word: &Word) -> Option<String> {
    if word.braced || !bare_safe(&word.text) {
        return brace_safe(&word.text).then(|| format!("{{{}}}", word.text));
    }
    Some(word.text.clone())
}

/// The quoted last resort for a word no verbatim spelling carries.
fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if matches!(c, '"' | '\\' | '$' | '[' | ']') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------

/// The rendering buffer: text, the current indent, and the losses so far.
#[derive(Default)]
struct Out {
    text: String,
    indent: usize,
    losses: Vec<ExportLoss>,
    /// A blank line is owed before the next line, unless nothing follows.
    pending_blank: bool,
}

impl Out {
    /// One physical line at the current indent.
    ///
    /// Only the *first* line is indented: a braced word carrying newlines is
    /// its own value byte for byte, so shifting its continuation lines would
    /// edit pack content rather than format it.
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
        }
        self.text.push('\n');
    }

    /// Ask for one blank line before whatever comes next.
    fn gap(&mut self) {
        if !self.text.is_empty() {
            self.pending_blank = true;
        }
    }

    fn indented(&mut self, body: impl FnOnce(&mut Self)) {
        self.indent += 1;
        body(self);
        self.indent -= 1;
    }

    /// The spelling of one word, recording a loss when none is faithful.
    fn spell(&mut self, word: &Word, under: &str, line: u32) -> String {
        spelling(word).unwrap_or_else(|| {
            self.losses.push(ExportLoss {
                word: under.to_owned(),
                line,
                text: word.text.clone(),
            });
            quoted(&word.text)
        })
    }

    fn registration(&mut self, reg: &Registration) {
        let under = reg.word().to_owned();
        let words: Vec<String> = reg
            .head
            .iter()
            .map(|word| self.spell(word, &under, reg.line))
            .collect();
        let Some(body) = &reg.body else {
            self.line(&words.join(" "));
            return;
        };
        self.gap();
        self.line(&format!("{} {{", words.join(" ")));
        self.indented(|out| out.body(body));
        self.line("}");
    }

    fn body(&mut self, registrations: &[Registration]) {
        for reg in registrations {
            self.registration(reg);
        }
    }
}

/// Render a loaded snapshot as canonical `SpecTcl` source.
///
/// The `speclib` header declares the pack's **own** vocabulary word, not the
/// newest one: raising a 1.x pack's declared vocabulary is `tcl spec
/// upgrade`'s job (its U1 rule), and doing it here would make an export
/// change what the reloaded pack means — a 1.1 word carries a per-site
/// notice under a 1.0 declaration and none under a 2.0 one.
#[must_use]
pub fn export_pack(pack: &Pack) -> String {
    export_pack_reporting(pack).0
}

/// [`export_pack`], plus every word that had no verbatim spelling.
#[must_use]
pub fn export_pack_reporting(pack: &Pack) -> (String, Vec<ExportLoss>) {
    let mut out = Out::default();
    let name = Word {
        text: pack.name.clone(),
        braced: false,
        line: 1,
    };
    let version = Word {
        text: pack.dsl_version.clone(),
        braced: false,
        line: 1,
    };
    let name = out.spell(&name, "speclib", 1);
    let version = out.spell(&version, "speclib", 1);
    out.line(&format!("speclib {name} {version} {{"));
    // The ports do not indent a pack's own declarations — a `.tclspec` is one
    // long file of `command` blocks, and a second level of indentation buys
    // nothing.
    out.body(&pack.registrations);
    out.gap();
    out.line("}");
    (out.text, out.losses)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::evaluate_pack;

    #[test]
    fn a_row_and_a_block_render_at_the_ports_margins() {
        let pack = evaluate_pack(
            "speclib demo 2.0 {\ncommand greet {\narity 1\nsubcommand loud {\narity 0\n}\n}\n}\n",
        );
        assert_eq!(
            export_pack(&pack),
            "speclib demo 2.0 {\n\
             \ncommand greet {\n\
             \x20   arity 1\n\
             \n\
             \x20   subcommand loud {\n\
             \x20       arity 0\n\
             \x20   }\n\
             }\n\
             \n\
             }\n"
        );
    }

    #[test]
    fn a_multi_line_braced_word_keeps_every_byte_between_its_braces() {
        let source = "speclib demo 2.0 {\ncommand greet {\n    hover {\n        summary {Say hi.}\n    }\n}\n}\n";
        let pack = evaluate_pack(source);
        let exported = export_pack(&pack);
        assert!(
            exported.contains("hover {\n        summary {Say hi.}\n    }"),
            "{exported}"
        );
        assert_eq!(
            evaluate_pack(&exported).commands[0]
                .spec
                .hover
                .map(|h| h.summary),
            pack.commands[0].spec.hover.map(|h| h.summary)
        );
    }

    #[test]
    fn a_word_takes_the_narrowest_verbatim_spelling_that_carries_it() {
        let spell = |text: &str, braced: bool| {
            spelling(&Word {
                text: text.to_owned(),
                braced,
                line: 1,
            })
        };
        assert_eq!(spell("-nocase", false).as_deref(), Some("-nocase"));
        // Anything the lexer would substitute takes the braced form, even
        // though the source wrote it bare or quoted.
        assert_eq!(spell("${x}", false).as_deref(), Some("{${x}}"));
        assert_eq!(spell("a b", false).as_deref(), Some("{a b}"));
        // A word the source braced stays braced, so its bytes are the value.
        assert_eq!(spell("PURE", true).as_deref(), Some("{PURE}"));
        // No verbatim spelling at all: an unbalanced brace, or a trailing
        // backslash that would escape the closing one.
        assert_eq!(spell("a { brace", false), None);
        assert_eq!(spell("trailing\\", false), None);
        // A backslash-continued line inside a braced word is verbatim to the
        // loader, so it keeps the braced spelling.
        assert_eq!(
            spell("one \\\n  two", true).as_deref(),
            Some("{one \\\n  two}")
        );
        assert_eq!(quoted("a $x [b]"), r#""a \$x \[b\]""#);
    }
}
