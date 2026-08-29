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

//! Client-side syntax highlighting and hover for the `.tclspec` DSL itself —
//! not the Tcl the pack describes, the *pack file's own* grammar.
//!
//! ## Why this is bespoke rather than reused wholesale
//!
//! Two existing classifiers were weighed before writing this one:
//!
//! - `tcl_lsp_core::semantic_tokens::full` is what the language server paints
//!   a `.tclspec` file with (dialect `"spectcl"`, driven by the DSL's own
//!   command registry pack under `tcl-registry/src/commands/spectcl/`). It is
//!   the *authoritative* classification, but its public surface only returns
//!   [`tcl_lsp_core::semantic_tokens::SemanticTokens`] — the LSP wire format
//!   (line/character delta-encoded, UTF-16 columns). Decoding that back into
//!   the byte spans this module's contract promises would mean either
//!   depending on `tcl-lsp-core`'s private `Entry`/`TokenKind` types (not
//!   `pub`) or re-deriving a UTF-16-column-to-byte-offset map by hand —
//!   exactly the kind of subtle position math AGENTS.md's "Word-token closing
//!   delimiters" section warns is easy to get wrong, for a browser-only
//!   editing surface that Monaco (backed by the real LSP compiled to wasm,
//!   per the project's stated direction) will supersede.
//! - `tcl_lexer::highlight::{highlight_ranges, HlRange}` already returns
//!   absolute byte spans and already recurses correctly into braced/bracketed
//!   words (the exact `open`/`close` delimiter-trim this module's [`recurse`]
//!   mirrors). It is not spec-aware, though: it paints `comment` / `var` /
//!   `cmd`-or-`ns` / ALL-CAPS-`event`, nothing else, so it cannot tell a
//!   statement head (`arity`, `option`, `hook`) from an ordinary bare word, or
//!   a `-flag` from a value.
//!
//! What follows keeps `tcl_lexer::highlight`'s proven approach — walk the real
//! [`Lexer`]'s token stream, recurse into `Str`/`Cmd` words at the correct
//! inner-content offset — and layers `.tclspec`-specific statement tracking on
//! top, using [`tcl_spec_studio::schema`] to decide *which* braced bodies are
//! further DSL statements (a `command` body, an `options` list, a `hover`
//! block, …) versus a flat word list (a `traits` or `dialects` set) versus
//! prose/foreign code the DSL does not itself structure (a `hook` body, which
//! is real Tcl carried verbatim). That is the "lexer-token-class based
//! highlight" the brief allows as a first slice when full reuse of the
//! server's classifier is not cleanly reachable.
//!
//! ## Known imprecision (acceptable for a v1 overlay editor)
//!
//! - Double-quoted `"…"` words are not a distinct [`TokenType`] in `tcl-lexer`
//!   (quoting is carried as `in_quote` on plain `Esc` fragments), so a quoted
//!   string's pieces paint as ordinary words rather than one `string` run.
//!   `.tclspec` packs use `{…}` for every text body in practice (see
//!   `docs/design/spec-dsl-examples/`), so this is rarely visible.
//! - A `hook` body (real Tcl, carried verbatim per `tcl-spectcl`'s loader) is
//!   painted as flat prose — its own `$vars` and `#` comments still classify
//!   correctly, but it does not get full nested Tcl highlighting. Giving hook
//!   bodies real Tcl colouring is a reasonable follow-up once this overlay
//!   editor is not itself heading for replacement by the wasm-LSP-backed
//!   Monaco editor.
//! - An unrecognised statement head's trailing block defaults to prose. This
//!   under-classifies rather than over-classifies: worst case a structured
//!   block reads as plain text, never as invented structure.

use std::fmt::Write as _;

use serde_json::{Value, json};
use tcl_lexer::{Lexer, LexerConfig, SourceMap, TokenType};

/// Recursion guard, matching [`tcl_lexer::highlight`]'s own bound — a
/// maliciously (or accidentally) deeply nested `{{{{…}}}}` document must not
/// blow the stack.
const MAX_DEPTH: usize = 64;

/// One classified byte span. Kept separate from the JSON `Value` so the
/// native tests below can assert on typed fields instead of re-parsing JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct HighlightToken {
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) class: &'static str,
    /// The source text of the span, included alongside `start`/`end` so a
    /// browser caller never has to re-slice the original string by *byte*
    /// offset (a UTF-16 `String.slice` is the wrong tool for that). Not part
    /// of the documented `{start, end, class}` contract's minimum shape, but
    /// additive JSON fields are free and this is the one a caller always
    /// wants next.
    pub(crate) text: String,
}

/// Whether a braced/bracketed body continues the DSL's statement grammar
/// (so its first word is a further `command-word`) or is a flat word list /
/// prose the DSL does not itself structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Statement,
    Flat,
}

/// The field-kind tags ([`tcl_spec_studio::schema::FieldKind::tag`]) that
/// mean "this field's value is itself a block of further DSL statements" —
/// the studio's schema drives the decision instead of a second hardcoded
/// keyword list, per AGENTS.md's registry-is-the-source-of-truth rule.
const CONTAINER_KIND_TAGS: &[&str] = &[
    "options",
    "forms",
    "hover",
    "sideEffects",
    "setterConstraints",
    "manufacturerMethods",
    "subCommands",
    "subSubCommands",
    "objectClass",
];

/// Statement heads that are pack/DSL grammar structure rather than
/// `CommandSpec`/`SubCommand` fields (so they are not in
/// [`tcl_spec_studio::schema`]), whose bodies are further statements. Mirrors
/// `tcl_spectcl::loader::evaluate_pack`'s and `descriptor`'s own dispatch.
const STRUCTURAL_STATEMENT_HEADS: &[&str] = &["speclib", "command", "descriptor", "values"];

/// Statement heads whose trailing brace(s) are foreign content (real Tcl, or
/// a plain word list) that the DSL does not parse as further statements even
/// though the head itself is structural. `hook NAME {params} {body}` carries
/// a parameter list and a Tcl body verbatim (`tcl_spectcl::loader::HookSource`);
/// neither is spec-dsl statement shaped.
const FLAT_STATEMENT_HEADS: &[&str] = &["hook"];

/// Decide how a statement's own trailing brace body should be walked, from
/// its head word (the statement's first bare word, e.g. `"command"`,
/// `"traits"`, `"hover"`, or `None` when the block has no head yet — a
/// dynamic/computed leading word, which is rare in practice for this DSL).
fn body_mode(head: Option<&str>) -> Mode {
    let Some(head) = head else {
        return Mode::Flat;
    };
    if STRUCTURAL_STATEMENT_HEADS.contains(&head) {
        return Mode::Statement;
    }
    if FLAT_STATEMENT_HEADS.contains(&head) {
        return Mode::Flat;
    }
    let kind_tag = tcl_spec_studio::schema::command_field(head)
        .or_else(|| tcl_spec_studio::schema::subcommand_field(head))
        .map(|field| field.kind.tag());
    match kind_tag {
        Some(tag) if CONTAINER_KIND_TAGS.contains(&tag) => Mode::Statement,
        _ => Mode::Flat,
    }
}

/// `true` for a bare word that reads as a number: an optional leading sign, a
/// digit, then any run of digits and `.` — covers `arity`'s `3..`, `3..5`,
/// plain counts, and version-shaped values like `8.5`. Deliberately loose:
/// this is a colour hint, not a validator.
fn looks_like_number(word: &str) -> bool {
    let unsigned = word.strip_prefix(['-', '+']).unwrap_or(word);
    let core = unsigned.strip_suffix('+').unwrap_or(unsigned);
    !core.is_empty()
        && core.starts_with(|c: char| c.is_ascii_digit())
        && core.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// `true` for a bare word that reads as a DSL property flag: `-introduced`,
/// `-detail`, `-step`, … — a leading `-` that is not itself a negative
/// number's sign.
fn looks_like_flag(word: &str) -> bool {
    word.len() > 1 && word.starts_with('-') && !looks_like_number(word)
}

/// Classify one bare (`Esc`-kind) word.
fn classify_bare_word(text: &str, mode: Mode, is_head: bool) -> &'static str {
    if is_head && mode == Mode::Statement {
        "command-word"
    } else if looks_like_flag(text) {
        "property-flag"
    } else if looks_like_number(text) {
        "number"
    } else {
        "value"
    }
}

/// Walk `src` (a full document on the first call, a braced/bracketed body's
/// inner text on a recursive one) and append every classified span, with
/// `base` translating `src`-relative offsets back to document-absolute ones.
fn collect(src: &str, depth: usize, base: usize, mode: Mode, out: &mut Vec<HighlightToken>) {
    if src.is_empty() || depth >= MAX_DEPTH {
        return;
    }
    let Ok(tokens) =
        Lexer::with_source_map(SourceMap::new(src), LexerConfig::default()).tokenise_all()
    else {
        return;
    };

    let mut at_stmt_start = true;
    let mut after_flag = false;
    let mut head: Option<&str> = None;

    for tok in &tokens {
        let s = (tok.span.start() as usize).min(src.len());
        let e = (tok.span.end() as usize).min(src.len());
        let text = src.get(s..e).unwrap_or("");

        match tok.kind {
            TokenType::Comment => {
                push(out, base + s, base + e, "comment", text);
            }
            TokenType::Var => {
                push(out, base + s, base + e, "variable", text);
                after_flag = false;
                at_stmt_start = false;
            }
            TokenType::Str | TokenType::Cmd => {
                let (open, close) = if tok.kind == TokenType::Str {
                    ('{', '}')
                } else {
                    ('[', ']')
                };
                let child_mode = if after_flag {
                    Mode::Flat
                } else {
                    body_mode(head)
                };
                recurse(text, open, close, depth, base + s, child_mode, out);
                after_flag = false;
                at_stmt_start = false;
            }
            TokenType::Expand => {
                push(out, base + s, base + e, "punctuation", text);
                at_stmt_start = false;
            }
            TokenType::Esc => {
                let is_head = mode == Mode::Statement && at_stmt_start;
                let class = classify_bare_word(text, mode, is_head);
                if is_head {
                    head = Some(text);
                }
                push(out, base + s, base + e, class, text);
                after_flag = mode == Mode::Statement && looks_like_flag(text);
                at_stmt_start = false;
            }
            TokenType::Eol => {
                if text == ";" {
                    push(out, base + s, base + e, "punctuation", text);
                }
                at_stmt_start = mode == Mode::Statement;
                after_flag = false;
                head = None;
            }
            TokenType::Sep | TokenType::Eof => {}
        }
    }
}

fn push(out: &mut Vec<HighlightToken>, start: usize, end: usize, class: &'static str, text: &str) {
    if end > start {
        out.push(HighlightToken {
            start,
            end,
            class,
            text: text.to_owned(),
        });
    }
}

/// Strip `open`/`close` from a `Str`/`Cmd` token's raw text and recurse into
/// the inner content at the right absolute offset.
///
/// Mirrors `tcl_lexer::highlight::collect_recurse` exactly (a private
/// function of that module, so re-derived here rather than imported) — the
/// same inner-end span convention AGENTS.md's "Word-token closing
/// delimiters" section documents: `text` already omits the closing
/// delimiter except when the word is the degenerate empty `{}`/`[]`, where it
/// is present and must be trimmed by hand.
fn recurse(
    text: &str,
    open: char,
    close: char,
    depth: usize,
    text_base: usize,
    mode: Mode,
    out: &mut Vec<HighlightToken>,
) {
    let mut inner = text;
    let mut inner_base = text_base;
    if inner.starts_with(open) {
        inner = &inner[open.len_utf8()..];
        inner_base += open.len_utf8();
    }
    if inner.len() == close.len_utf8() && inner.starts_with(close) {
        inner = &inner[..inner.len() - close.len_utf8()];
    }
    collect(inner, depth + 1, inner_base, mode, out);
}

/// The full token list for `source`, in source order.
pub(crate) fn highlight_tokens(source: &str) -> Vec<HighlightToken> {
    let mut out = Vec::new();
    collect(source, 0, 0, Mode::Statement, &mut out);
    out.sort_by_key(|t| (t.start, t.end));
    out
}

/// `dsl_highlight`'s JSON body: `[{start, end, class, text}, …]`.
pub(crate) fn highlight_json(source: &str) -> Value {
    Value::Array(
        highlight_tokens(source)
            .iter()
            .map(|t| json!({ "start": t.start, "end": t.end, "class": t.class, "text": t.text }))
            .collect(),
    )
}

// Hover

/// Grammar keywords and option-row flags that are DSL structure rather than
/// `CommandSpec`/`SubCommand` fields, so [`tcl_spec_studio::help::field_help`]
/// does not carry them. Hand-written, matching `tcl_spectcl::loader`'s own
/// dispatch tables (`evaluate_pack`, `hover_block`, `option_row`).
const KEYWORD_HELP: &[(&str, &str, &str)] = &[
    (
        "speclib",
        "speclib — pack header",
        "Opens a pack: `speclib NAME DSL-VERSION { … }`. Every `.tclspec` file has exactly one, wrapping every `command`, `values`, `descriptor`, `hook`, and `default` declaration.",
    ),
    (
        "command",
        "command — a command declaration",
        "`command NAME { … }` declares one `CommandSpec`. The body is a block of statements, one per `CommandSpec` field (`arity`, `traits`, `dialects`, `option`, `hover`, …). Add `-override` after NAME to claim a name the shipped registry already defines.",
    ),
    (
        "option",
        "option — one flag/switch row",
        "`option NAME ?flags…?` declares one entry of the command's `options` table (an `OptionSpec`) — a `-flag` the command itself accepts. Common row flags: `-detail`, `-takes`, `-dialects`, `-aliases`, `-arity`, `-arity-hook`, `-introduced`, `-deprecated`, `-retired`.",
    ),
    (
        "form",
        "form — an invocation form",
        "`form NAME {synopsis}` declares one entry of the command's `forms` table — a distinct callable shape, most often a getter/setter pair.",
    ),
    (
        "descriptor",
        "descriptor — a shared block descriptor",
        "`descriptor KEY NAME { … }` declares a reusable block (an `EventRequires`, `CaseListSpec`, `DefinitionBodyGrammar`, …) that a command can reference by NAME instead of repeating inline.",
    ),
    (
        "hook",
        "hook — a shared hook body",
        "`hook NAME {params} { body }` declares a reusable Tcl-shaped hook body (carried as text, never evaluated by the loader) that a command or option can attach by NAME.",
    ),
    (
        "values",
        "values — a shared value catalogue",
        "`values NAME { value V ?-detail {…}? … }` declares a reusable list of accepted literal values that an option or argument can reference by NAME.",
    ),
    (
        "value",
        "value — one catalogue entry",
        "One row of a `values` block: `value V ?-detail {…}? ?-min-tcl VER? ?-code N?` — a single accepted literal and its documentation.",
    ),
    (
        "default",
        "default — a pack-wide default",
        "`default KEY VALUE` sets a pack-wide fallback (`dialects`, `required_package`, `introduced_version`, …) that every `command` in the pack inherits unless it states its own.",
    ),
    (
        "summary",
        "hover summary",
        "The one-line summary shown at the top of the command's hover — write it like the first line of a man page.",
    ),
    (
        "synopsis",
        "hover synopsis",
        "One invocation shape as the hover shows it, e.g. `lappend varName ?value …?`. Repeatable — one row per form worth showing.",
    ),
    (
        "description",
        "hover description",
        "The prose paragraph(s) of the command's hover, below the summary and synopsis.",
    ),
    (
        "source",
        "hover source",
        "Where the documentation comes from, e.g. `Tcl lsort(n)` — shown as the hover's attribution line.",
    ),
    (
        "example",
        "hover example",
        "One example call, shown verbatim in the hover. Repeatable — one row per example line.",
    ),
    (
        "returns",
        "hover return value",
        "Prose describing what the command's result means — the hover's closing line.",
    ),
    (
        "-detail",
        "-detail — row documentation",
        "Long-form prose for the option/value row it appears on — what a hover shows for that one flag or literal.",
    ),
    (
        "-takes",
        "-takes — option argument hint",
        "The placeholder name shown for the value an option consumes, e.g. `-index -takes indexList`.",
    ),
    (
        "-aliases",
        "-aliases — option spelling aliases",
        "Alternate spellings for the same option row, as a Tcl list.",
    ),
    (
        "-dialects",
        "-dialects — row availability",
        "Restricts the option/value row to a dialect set, the same vocabulary as the command-level `dialects` field (`all-tcl`, `tcl8.5+`, `f5-irules`, …).",
    ),
    (
        "-min-abbrev",
        "-min-abbrev — minimum unique abbreviation",
        "The fewest leading characters of this option's name that still resolve to it uniquely.",
    ),
    (
        "-arity",
        "-arity — option value arity",
        "How many value words this option consumes: `One` or `{Fixed N}`. For an arity that depends on the call, use `-arity-hook` instead.",
    ),
    (
        "-arity-hook",
        "-arity-hook — dynamic option arity",
        "A hook body (or `-native ID`) that decides how many value words this option consumes at the call site, for options whose value shape is not a fixed count.",
    ),
    (
        "-role",
        "-role — option value argument role",
        "The `ArgRole` the option's value word plays, e.g. `CommandPrefix` for a callback-taking option like `lsort -command`.",
    ),
    (
        "-appends",
        "-appends — command-prefix appended arity",
        "For a `-role CommandPrefix` option: how many arguments the command appends when it invokes the callback — `{Exactly N}`, `{AtLeast N}`, or `Unknown`.",
    ),
    (
        "-native",
        "-native — engine-provided hook",
        "Names a hook implementation that already exists in the engine by id, instead of supplying a Tcl-shaped body inline.",
    ),
    (
        "-override",
        "-override — claim a shipped name",
        "On a `command` declaration: this pack's definition replaces the shipped registry's command of the same name, instead of losing to it.",
    ),
    (
        "-step",
        "-step — arity step",
        "On `arity`: the repeat stride once the minimum argument count is reached (e.g. `array set`'s trailing `name value` pairs).",
    ),
    (
        "-also",
        "-also — arity extra exact count",
        "On `arity`: one extra exact total argument count that is valid in addition to the stated range.",
    ),
    (
        "-min-tcl",
        "-min-tcl — value's minimum Tcl version",
        "On a `values` row: the earliest Tcl version this literal value is accepted from.",
    ),
    (
        "-code",
        "-code — value's numeric code",
        "On a `values` row: the integer code this literal value corresponds to, when the underlying API is code-based.",
    ),
    (
        "-inputs",
        "-inputs — hook cacheability shape",
        "Declares what a hook body's result depends on, so the hook host can cache by call shape instead of treating the hook as fully dynamic.",
    ),
];

/// One catalogue entry match: which catalogue, and the `{key, doc}` pair.
fn catalogue_lookup(word: &str) -> Option<(String, String)> {
    let catalogues = tcl_spec_studio::schema::catalogues();
    let object = catalogues.as_object()?;
    for (catalogue_id, entries) in object {
        let Some(list) = entries.as_array() else {
            continue;
        };
        for entry in list {
            if entry.get("key").and_then(Value::as_str) == Some(word) {
                let doc = entry.get("doc").and_then(Value::as_str).unwrap_or("");
                return Some((format!("{catalogue_id} · {word}"), doc.to_owned()));
            }
        }
    }
    None
}

/// A `CommandSpec`/`SubCommand` field lookup, tolerating the DSL's occasional
/// singular/plural mismatch against the Rust field name (`option` vs.
/// `options`, `form` vs. `forms`).
fn field_lookup(word: &str) -> Option<(String, String)> {
    let candidates = [
        word.to_owned(),
        format!("{word}s"),
        word.strip_suffix('s').unwrap_or(word).to_owned(),
    ];
    for key in &candidates {
        if let Some(field) = tcl_spec_studio::schema::command_field(key)
            .or_else(|| tcl_spec_studio::schema::subcommand_field(key))
        {
            let body = tcl_spec_studio::help::field_help(field.key).unwrap_or(field.doc);
            return Some((field.label.to_owned(), body.to_owned()));
        }
    }
    None
}

/// [`field_lookup`] for a `-flag`-spelled word: `-introduced` needs
/// `introduced_version`, `-detail` needs `detail` — strip the dash and, if
/// that alone misses, try the `_version` lifecycle-field convention
/// `tcl_spectcl::loader` uses for `-introduced`/`-deprecated`/`-retired`.
fn flag_field_lookup(word: &str) -> Option<(String, String)> {
    let bare = word.trim_start_matches('-');
    field_lookup(bare).or_else(|| field_lookup(&format!("{bare}_version")))
}

/// Look up hover content for `word`, tried in the order most likely to hit:
/// a catalogue variant spelling, a schema field, a `-flag` field alias, then
/// the hand-written DSL grammar table.
fn hover_lookup(word: &str) -> Option<(String, String)> {
    catalogue_lookup(word)
        .or_else(|| field_lookup(word))
        .or_else(|| {
            if word.starts_with('-') {
                flag_field_lookup(word)
            } else {
                None
            }
        })
        .or_else(|| {
            KEYWORD_HELP
                .iter()
                .find(|(key, _, _)| *key == word)
                .map(|(_, title, body)| ((*title).to_owned(), (*body).to_owned()))
        })
}

/// `dsl_hover`'s JSON body: `{found, title, body}`.
pub(crate) fn hover_json(source: &str, offset: usize) -> Value {
    let Some(token) = highlight_tokens(source)
        .into_iter()
        .find(|t| t.start <= offset && offset < t.end)
    else {
        return json!({ "found": false, "title": "", "body": "" });
    };
    // Only these two classes name a word from the DSL's own vocabulary — a
    // `value`/`number`/`variable`/`string` is call-site data the pack author
    // wrote, not a keyword the studio has anything to say about.
    if !matches!(token.class, "command-word" | "property-flag") {
        return json!({ "found": false, "title": "", "body": "" });
    }
    match hover_lookup(&token.text) {
        Some((title, body)) => json!({ "found": true, "title": title, "body": body }),
        None => {
            let mut title = String::new();
            let _ = write!(title, "`{}`", token.text);
            json!({ "found": false, "title": title, "body": "" })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(source: &str) -> Vec<(&'static str, String)> {
        highlight_tokens(source)
            .into_iter()
            .map(|t| (t.class, t.text))
            .collect()
    }

    #[test]
    fn comment_is_classified_and_positioned() {
        let src = "# a comment\ncommand foo {\n}\n";
        let toks = highlight_tokens(src);
        let comment = toks
            .iter()
            .find(|t| t.class == "comment")
            .expect("comment token");
        assert_eq!(&src[comment.start..comment.end], "# a comment");
    }

    #[test]
    fn top_level_and_nested_statement_heads_are_command_words() {
        let src =
            "speclib mylib 1.0 {\n\ncommand foo {\n    arity 1..\n    traits {PURE}\n}\n\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("command-word", "speclib".to_owned())));
        assert!(cs.contains(&("command-word", "command".to_owned())));
        assert!(cs.contains(&("command-word", "arity".to_owned())));
        // `traits` is itself a command-word (a CommandSpec field, statement
        // head)…
        assert!(cs.contains(&("command-word", "traits".to_owned())));
        // …but its own block is a flat FlagSet, so `PURE` is a value, not a
        // nested command-word.
        assert!(cs.contains(&("value", "PURE".to_owned())));
        assert!(!cs.contains(&("command-word", "PURE".to_owned())));
    }

    #[test]
    fn arity_range_is_a_number_and_flags_are_property_flags() {
        let src = "command foo {\n    arity 3.. -step 2 -also 4\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("number", "3..".to_owned())));
        assert!(cs.contains(&("property-flag", "-step".to_owned())));
        assert!(cs.contains(&("number", "2".to_owned())));
        assert!(cs.contains(&("property-flag", "-also".to_owned())));
        assert!(cs.contains(&("number", "4".to_owned())));
    }

    #[test]
    fn negative_number_after_a_flag_is_not_a_flag() {
        // `-also` is a flag; its value `-1` starts with `-` too but must not
        // be painted as a second flag.
        let src = "command foo {\n    arity 1.. -also -1\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("property-flag", "-also".to_owned())));
        assert!(cs.contains(&("number", "-1".to_owned())));
        assert!(!cs.iter().any(|(c, t)| *c == "property-flag" && t == "-1"));
    }

    #[test]
    fn dollar_variable_inside_a_hook_body_is_still_classified() {
        let src = "hook myhook {words ctx} {\n    set x $words\n}\n";
        let cs = classes(src);
        // The raw token span includes the sigil (`$name`, not `name`) —
        // `content_offset` is what a `SourceMap` would use to strip it, and
        // this module deliberately keeps the sigil in `text` so a rendered
        // span reproduces the source byte-for-byte when concatenated.
        assert!(cs.contains(&("variable", "$words".to_owned())));
        // `set` sits in a flat (hook-body) block, so it is a value — not
        // promoted to command-word — matching the module's documented
        // "hook bodies are prose, not DSL statements" limitation.
        assert!(!cs.contains(&("command-word", "set".to_owned())));
    }

    #[test]
    fn option_row_flags_are_property_flags_and_detail_is_flat() {
        let src = "command foo {\n    option -ascii -detail {Compare using $x order.}\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("command-word", "option".to_owned())));
        // `-ascii` is the option row's own name — spelled as a flag in the
        // DSL, so it paints as one (there is no positional/shape difference
        // a lexer-level classifier can use to tell "the row's name" from
        // "a switch this row declares").
        assert!(cs.contains(&("property-flag", "-ascii".to_owned())));
        assert!(cs.contains(&("property-flag", "-detail".to_owned())));
        // Inside the `-detail` value, `Compare`/`using`/`order.` are flat
        // prose words, not a nested command-word — and the `$x` inside it
        // still highlights as a variable.
        assert!(cs.contains(&("value", "Compare".to_owned())));
        assert!(!cs.contains(&("command-word", "Compare".to_owned())));
        assert!(cs.contains(&("variable", "$x".to_owned())));
    }

    #[test]
    fn hover_block_summary_is_a_command_word_but_its_body_is_flat() {
        let src =
            "command foo {\n    hover {\n        summary {Sort a list; keep going.}\n    }\n}\n";
        let cs = classes(src);
        assert!(cs.contains(&("command-word", "hover".to_owned())));
        assert!(cs.contains(&("command-word", "summary".to_owned())));
        // The embedded `;` must not turn "keep" into a second command-word —
        // `summary`'s own body is prose (`field_lookup("summary")` misses,
        // since `summary` is a `HoverSnippet` field, not a `CommandSpec`
        // one, so `body_mode` falls back to `Flat`).
        assert!(!cs.contains(&("command-word", "keep".to_owned())));
        assert!(cs.contains(&("value", "keep".to_owned())));
    }

    #[test]
    fn tokens_are_non_overlapping_and_sorted() {
        let src = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/design/spec-dsl-examples/lsort.tclspec"
        ))
        .expect("fixture reads");
        let toks = highlight_tokens(&src);
        assert!(!toks.is_empty());
        for pair in toks.windows(2) {
            assert!(
                pair[0].end <= pair[1].start,
                "overlap: {:?} vs {:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn highlight_json_has_the_documented_shape() {
        let value = highlight_json("command foo {\n    arity 1..\n}\n");
        let array = value.as_array().expect("array");
        assert!(!array.is_empty());
        for entry in array {
            assert!(entry.get("start").is_some_and(Value::is_u64));
            assert!(entry.get("end").is_some_and(Value::is_u64));
            assert!(entry.get("class").is_some_and(Value::is_string));
        }
    }

    #[test]
    fn hover_finds_a_command_field_by_dsl_keyword() {
        let src = "command foo {\n    arity 1..\n}\n";
        let offset = src.find("arity").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        assert_eq!(hover["title"], json!("Arity"));
        assert!(hover["body"].as_str().unwrap().contains("wrong # args"));
    }

    #[test]
    fn hover_finds_options_field_from_the_singular_option_keyword() {
        let src = "command foo {\n    option -ascii -detail {x}\n}\n";
        let offset = src.find("option ").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        // `option` (DSL, singular) resolves via the plural `options` field.
        assert!(
            hover["body"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("flag")
        );
    }

    #[test]
    fn hover_finds_a_flag_via_the_version_field_alias() {
        let src = "command foo {\n    option -x -introduced 8.5\n}\n";
        let offset = src.find("-introduced").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        assert!(
            hover["body"]
                .as_str()
                .unwrap()
                .to_lowercase()
                .contains("version")
        );
    }

    #[test]
    fn hover_finds_a_grammar_keyword_from_the_static_table() {
        let src = "speclib mylib 1.0 {\n}\n";
        let offset = 0;
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(true));
        assert!(hover["title"].as_str().unwrap().starts_with("speclib"));
    }

    #[test]
    fn hover_finds_a_trait_catalogue_variant() {
        let src = "command foo {\n    traits {PURE}\n}\n";
        let offset = src.find("PURE").unwrap();
        // `PURE` classifies as `value`, not `command-word`/`property-flag`,
        // so hover intentionally has nothing to say about it — it is call
        // site (well, pack-author-site) data, matching the doc comment on
        // `hover_json`.
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(false));
    }

    #[test]
    fn hover_on_whitespace_is_not_found() {
        let src = "command foo {\n}\n";
        let hover = hover_json(src, "command".len());
        assert_eq!(hover["found"], json!(false));
    }

    #[test]
    fn hover_on_an_unknown_word_reports_not_found_with_the_word_quoted() {
        let src = "command foo {\n    zzz-not-a-real-field 1\n}\n";
        let offset = src.find("zzz").unwrap();
        let hover = hover_json(src, offset);
        assert_eq!(hover["found"], json!(false));
        assert_eq!(hover["title"], json!("`zzz-not-a-real-field`"));
    }
}
