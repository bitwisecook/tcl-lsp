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

//! Generate the command-name alternations of the TextMate-family Tcl grammars
//! — VS Code's `tcl.tmLanguage.json`, its `JetBrains` copy, and the Sublime
//! Text `.sublime-syntax` port — from `tcl-registry`, the same way
//! [`crate::gen_zed_queries`] already does for Zed's tree-sitter queries.
//!
//! ## Why this exists
//!
//! Hand-maintaining three separate copies of the
//! `keyword.control` / `keyword.other` / `support.function`
//! command-name alternations lets them drift from each other and
//! from the registry: `lmap` binds
//! a loop variable like `foreach`, so the registry carries it with the
//! `LANGUAGE_KEYWORD` trait, but a hand-written list can miscategorise it as
//! a plain builtin, and a fix applied to one of the three files can leave the
//! other two silently unfixed. This generator makes `tcl-registry` the single
//! source of truth for
//! *which* ambient core-Tcl commands are keywords vs. plain builtins, so a
//! future addition/removal in the registry can never leave any grammar
//! behind — only the `--check` gate fails, the same day.
//!
//! ## What is and isn't generated
//!
//! The three command-name regex alternations and the shared lexical regex
//! bodies (numbers, backslash escapes, and variable names) are
//! generated. Number / escape behaviour is projected from the modern grammar
//! that [`tcl_dialect::NumberSyntax::of_profile`] /
//! [`tcl_dialect::EscapeSyntax::of_profile`] designate for a grammar without a
//! per-document profile; the `TextMate` formats cannot switch regexes when LSP
//! configuration changes. The variable body comes from
//! [`tcl_syntax::naming::textmate_variable_name_body`]. Thus the static
//! fallback grammars remain one cross-editor projection instead of three
//! hand-rolled lexical parsers.
//!
//! The braced-variable projection preserves the historical first-close display
//! rule, including empty names; Tcl 9 nested/escaped-close extents remain
//! approximate until semantic tokens arrive.
//!
//! Definition/member/class/regex rosters and historical alternative order
//! come from the Registry's descriptor-checked source-display recipe. Name
//! tokens and shallow parameter-list shapes come from Syntax owners. Target
//! capture scopes and repository layouts remain presentation templates; these
//! static rules do not select a handler or prove a member context. Comments,
//! strings, operators and punctuation remain target template text. Each target
//! file is edited by locating the `"match"` (JSON) / `match:` (YAML) value
//! that immediately follows each scope name and replacing only that value —
//! not a full-file reserialisation — so unrelated formatting (e.g. the
//! single-line `fileTypes` array in the `.tmLanguage.json` files) is left
//! byte-for-byte untouched.
//!
//! ## Scope: ambient core Tcl only, not iRules/iApps/Expect
//!
//! Unlike Zed (which emits a *separate* query file per dialect directory),
//! all three TextMate-family grammars here share **one** grammar file across
//! every registered language ID (`tcl`, `tcl-irule`, `tcl90`, …) — see
//! `editors/vscode/package.json`'s `contributes.grammars`, which points every
//! Tcl-family language at the same `syntaxes/tcl.tmLanguage.json`. Pulling in
//! the full iRules command surface (~1,270 commands, mostly `HTTP::`/`LB::`/…
//! namespaced) would bulk-highlight F5-only commands even in plain `.tcl`
//! files that will never see them. So the registry projection here is scoped
//! to [`SpecSurface::ALL_TCL`] (core Tcl 8.4-9.1 + Tk) only — the same
//! ambient ground the three hand-maintained lists actually covered — and the one
//! iRules-only word those lists carried (`when`) is kept as a small static
//! addition alongside the non-command clause words below, rather than
//! justifying the full dialect union for one word.
//!
//! ## Static residue: words with no `CommandSpec`, and words excluded on purpose
//!
//! A few real, in-scope words never come from the registry projection:
//!
//! * **Clause continuations** (`else`, `elseif`, `on`, `trap`, `finally` —
//!   [`tcl_registry::traits::clause_keywords_without_command_spec`], shared
//!   with `tcl_lsp_core::semantic_tokens`'s equivalent residue so the two
//!   never drift on which clause words are real keywords) and the iRules
//!   block keyword `when` have no standalone `CommandSpec` (or, for `when`,
//!   sit outside the `ALL_TCL` scope above) but are real Tcl keywords, so
//!   they are added statically.
//! * `itcl::class` is package-gated (`package require Itcl`) so the ambient
//!   projection below excludes it, but it is namespace-qualified — no
//!   collision risk with an ordinary bareword — so it is added statically too.
//! * `my`, `next`, `nextto`, `self` carry the registry's `LANGUAGE_KEYWORD`
//!   trait (`TclOO` intra-method dispatch) but are common enough as ordinary
//!   variable/proc names elsewhere that a context-free regex blanket-matching
//!   them would false-positive constantly — the semantic-token layer handles
//!   them correctly (it knows whether it is inside a method body); a static
//!   grammar cannot, so they are deliberately excluded here, matching
//!   [`crate::gen_zed_queries`]'s equivalent judgement call for its `my`-style
//!   words. The same reasoning covers the `TclOO` method-body helpers
//!   `callback` and `mymethod`: `proc callback {…}` is ordinary, common Tcl, and
//!   a context-free regex cannot tell that definition — or any call of it —
//!   from the 9.0 helper, so both bare words join the exclusion list. Their
//!   **qualified** `oo::Helpers::…` spellings stay in, on the same
//!   namespace-qualified/no-collision-risk criterion [`STATIC_OTHER_WORDS`]
//!   applies to `itcl::class`.
//!
//! ## Control vs. "other" keyword bucket
//!
//! The registry's `CONTROL_FLOW` trait exists for the compiler's dataflow
//! analysis, not editor colouring, and does not match how mainstream
//! language grammars split "control" keywords (most treat `return` /
//! `break` / `continue` / `throw` as control-flow, matching what a user
//! coming from JS/Python/Java would expect) from declaration/scoping
//! keywords (`proc`, `namespace`, `variable`, `global`, …). So the
//! control/other split here is a small explicit list
//! ([`CONTROL_STYLE`]) reviewed by a human when a new `LANGUAGE_KEYWORD`
//! command is added, not a mechanical trait lookup — the trait only decides
//! *whether* a command is a keyword at all (drift-protected), never *which
//! bucket* it colours as (a human judgement call, same spirit as the
//! clause-word residue above).
//!
//! Run `cargo xtask gen-tmlanguage-keywords` to (re)write the files;
//! `--check` verifies the committed files match, exiting non-zero on drift.

mod source_json;

use std::collections::BTreeSet;
use std::fs;
use std::process::ExitCode;
use tcl_dialect::model::{Family, SurfaceQuery};

use anyhow::{Context, Result, bail};
use tcl_dialect::{EscapeSyntax, NumberSyntax};
use tcl_registry::CommandRegistry;
use tcl_registry::traits::{Traits, clause_keywords_without_command_spec};
use tcl_syntax::naming::{textmate_braced_variable_reference, textmate_variable_name_body};

use crate::util::{self, repo_root};

/// The iRules block keyword `when`, added alongside
/// [`clause_keywords_without_command_spec`] (see module docs for why it
/// isn't covered by the `ALL_TCL` dialect scope this generator otherwise
/// uses).
const WHEN: &str = "when";

/// Package-gated but namespace-qualified, so safe to blanket-match (see
/// module docs).
const STATIC_OTHER_WORDS: &[&str] = &["itcl::class"];

/// Registry `LANGUAGE_KEYWORD` commands deliberately excluded — too generic /
/// collision-prone for a context-free grammar (see module docs).
const CONTEXT_SENSITIVE_EXCLUDE: &[&str] =
    &["callback", "my", "mymethod", "next", "nextto", "self"];

/// The human-curated control/other split for registry-sourced keywords —
/// mainstream branch/loop/jump/exception-style words go to [`Buckets::control`];
/// every other `LANGUAGE_KEYWORD` command falls through to [`Buckets::other`].
/// See the module docs for why this isn't derived from the `CONTROL_FLOW` trait.
const CONTROL_STYLE: &[&str] = &[
    "break",
    "catch",
    "continue",
    // `error` sits here with `catch`/`throw`/`try`: it is a non-local exit, not
    // a computation. Leaving it off `LANGUAGE_KEYWORD` would colour
    // `catch { error boom }`'s two halves differently.
    "error",
    "for",
    "foreach",
    "foreachLine",
    "if",
    "lfilter",
    "lmap",
    "return",
    "switch",
    "tailcall",
    "throw",
    "try",
    "while",
    "yield",
    "yieldto",
];

/// The three command buckets projected from the registry, plus the static
/// residue folded in.
#[derive(Default)]
struct Buckets {
    control: BTreeSet<String>,
    other: BTreeSet<String>,
    builtin: BTreeSet<String>,
    source_display: Option<tcl_registry::source_navigation::StaticTclSourceDisplay>,
}

/// Lexical patterns shared by the three TextMate-family formats. All strings
/// are logical regexes (one backslash in memory); the JSON renderer escapes
/// them while the YAML renderer keeps them literal in a single-quoted scalar.
struct LexicalRegexes {
    variable: String,
    braced_variable: String,
    hex: String,
    octal: String,
    binary: String,
    decimal: String,
    escape: String,
}

/// A digit run under the shared numeral grammar. Tcl 9's `_` separator is
/// allowed only *between* digits, with one-or-more underscores (the lexer
/// accepts `1__000`); older grammars receive the bare digit class.
fn digit_run(class: &str, syntax: NumberSyntax) -> String {
    if syntax.allows_digit_separators() {
        format!("{class}(?:{class}|_+{class})*")
    } else {
        format!("{class}+")
    }
}

/// Render lexical regex bodies from the shared syntax owners. The grammar
/// files have no dynamic hook for the LSP document's configured profile, so
/// their common fallback follows the same permissive modern default as
/// `LexerConfig::default`; dialect-specific analysis remains the LSP's job.
fn lexical_regexes() -> LexicalRegexes {
    let numbers = NumberSyntax::of_profile(None);
    let escapes = EscapeSyntax::of_profile(None);
    let hex_digits = digit_run("[0-9a-fA-F]", numbers);
    let octal_digits = digit_run("[0-7]", numbers);
    let binary_digits = digit_run("[01]", numbers);
    let decimal_digits = digit_run("[0-9]", numbers);

    let hex = format!(r"\b0[xX]{hex_digits}\b");
    let octal = if numbers.has_binary_octal_prefix() {
        format!(r"\b0[oO]{octal_digits}\b")
    } else {
        // A future oldest grammar could remove the explicit prefix. Keep the
        // generated target syntactically valid rather than emitting an empty
        // alternative, while the owner method remains the decision point.
        r"(?!)".to_owned()
    };
    let binary = if numbers.has_binary_octal_prefix() {
        format!(r"\b0[bB]{binary_digits}\b")
    } else {
        r"(?!)".to_owned()
    };
    let decimal_prefix = numbers
        .has_decimal_prefix()
        .then(|| format!(r"0[dD]{decimal_digits}"));
    // Explicit `0d` is integer-only; ordinary decimal spelling may contain a
    // fractional part and/or exponent. Keeping them separate prevents the
    // grammar from colouring invalid `0d1.5` as a single numeric literal.
    let ordinary_decimal =
        format!(r"{decimal_digits}(?:\.{decimal_digits})?(?:[eE][+-]?{decimal_digits})?");
    let decimal = match decimal_prefix {
        Some(prefix) => format!(r"\b(?:{prefix}|{ordinary_decimal})\b"),
        None => format!(r"\b{ordinary_decimal}\b"),
    };

    let hex_escape = match escapes.hex_escape_digits() {
        Some(width) => format!(r"x[0-9a-fA-F]{{1,{width}}}"),
        None => r"x[0-9a-fA-F]+".to_owned(),
    };
    // 8.6+ takes a third octal digit only when the first digit is 0–3. The
    // `octal_takes_third_digit` owner says this directly, avoiding an
    // unsound flat `[0-7]{1,3}` grammar.
    let octal_escape = if escapes.octal_takes_third_digit(0o37) {
        if escapes.octal_takes_third_digit(0o40) {
            r"[0-7]{1,3}".to_owned()
        } else {
            r"(?:[0-3][0-7]{0,2}|[4-7][0-7]?)".to_owned()
        }
    } else {
        r"[0-7]{1,2}".to_owned()
    };
    // `hex_run` stops before the next digit once the accumulated prefix is
    // above Tcl's `ParseHex` cap (0x10fff).  The generated grammars need the
    // same boundary, not merely the same eight-digit maximum.  Since the cap
    // is five hex digits wide, these three longer branches encode the finite
    // states where the first over-cap digit is the sixth, seventh, or eighth;
    // the final branch handles runs of five digits or fewer.  Branches are
    // ordered longest-first so a still-valid prefix consumes the available
    // next digit before the shorter fallback can win.
    let wide = if escapes.has_wide_unicode() {
        r"|U(?:(?:000[0-9a-fA-F]{4}|0010[0-9a-fA-F]{3})[0-9a-fA-F]|(?:00[0-9a-fA-F]{4}|010[0-9a-fA-F]{3})[0-9a-fA-F]|(?:0[0-9a-fA-F]{4}|10[0-9a-fA-F]{3})[0-9a-fA-F]|[0-9a-fA-F]{1,5})"
    } else {
        ""
    };
    let escape = format!(
        r#"\\(?:[abfnrtv\\"{{}}()\[\]\$]|{octal_escape}|{hex_escape}|u[0-9a-fA-F]{{1,4}}{wide}|\n)"#
    );

    LexicalRegexes {
        braced_variable: textmate_braced_variable_reference().to_owned(),
        variable: format!(r"\${}(?:\([^)]*\))?", textmate_variable_name_body()),
        hex,
        octal,
        binary,
        decimal,
        escape,
    }
}

/// Classify every ambient (no `required_package`), `ALL_TCL`-dialect command
/// in `reg`, fold in the static residue, and split `LANGUAGE_KEYWORD`
/// commands into `control` / `other` via [`CONTROL_STYLE`].
///
/// A command name must be built only of ASCII alphanumerics, `_`, or `:` —
/// the character class the generated regex alternations (and Zed's
/// `#any-of?` lists) assume is inherently safe to embed literally.
fn classify(reg: &CommandRegistry) -> Buckets {
    let mut b = Buckets::default();
    let dialects = Some(SurfaceQuery::any_release(Family::Tcl));

    for name in reg.command_names() {
        if name == util::DISABLED_SENTINEL || CONTEXT_SENSITIVE_EXCLUDE.contains(&name) {
            continue;
        }
        if !util::is_safe_command_name(name) || util::is_internal_or_test_harness_noise(name) {
            continue;
        }
        // Any *ambient* spec under this grammar union decides the bucket —
        // see the same change in `gen_zed_queries`. A single-spec lookup
        // judges a duplicate-named command (`link`, `oo::Helpers::link`) by
        // its `package require`-gated 8.6 `ooutil` twin and drops the
        // ambient 9.0 core keyword.
        let Some(spec) = reg
            .specs(name)
            .iter()
            .find(|spec| spec.supports_dialect(dialects) && spec.required_package.is_none())
        else {
            continue;
        };

        if spec.traits.contains(Traits::LANGUAGE_KEYWORD) {
            if CONTROL_STYLE.contains(&name) {
                b.control.insert(name.to_owned());
            } else {
                b.other.insert(name.to_owned());
            }
        } else {
            b.builtin.insert(name.to_owned());
        }
    }

    b.control.extend(
        clause_keywords_without_command_spec()
            .iter()
            .chain([&WHEN])
            .map(|s| (*s).to_owned()),
    );
    b.other
        .extend(STATIC_OTHER_WORDS.iter().map(|s| (*s).to_owned()));
    b.source_display = tcl_registry::source_navigation::static_tcl_source_display(reg);
    b
}

/// Render a sorted word set as a `TextMate` alternation body:
/// `word1|word2|word3` (no surrounding `(?:…)` — callers wrap it).
fn render_alternation(words: &BTreeSet<String>) -> String {
    words.iter().cloned().collect::<Vec<_>>().join("|")
}

/// A `(?<=…)(?:alternation)(?=…)` command-boundary regex, in its *logical*
/// form (single backslashes) — callers escape per their target format.
fn boundary_regex(words: &BTreeSet<String>) -> String {
    format!(r"(?<=^|\s|;|\[)(?:{})(?=\s|$|;)", render_alternation(words))
}

/// Carve out the substring between `start_marker` and the next
/// `end_marker` after it — the scope names this generator targets
/// (`support.function.tcl` in particular) recur elsewhere in each grammar
/// (e.g. the `regexp`/`regsub` command highlighting also tags itself
/// `support.function.tcl`), so every splice below must be confined to the
/// `keyword` block specifically, never a whole-file search.
fn bounded_block<'a>(
    text: &'a str,
    start_marker: &str,
    end_marker: &str,
) -> Result<(&'a str, usize)> {
    let start = text
        .find(start_marker)
        .with_context(|| format!("{start_marker:?} not found"))?;
    let rel_end = text[start..]
        .find(end_marker)
        .with_context(|| format!("{end_marker:?} not found after {start_marker:?}"))?;
    Ok((&text[start..start + rel_end], start))
}

/// Replace the JSON string value of the first `"match"` key at or after
/// `anchor` *within `block`* with `new_value` (a *logical*, unescaped regex
/// — this JSON-escapes it), then splice `block` back into `text` at
/// `block_offset`.
fn splice_json_match(
    text: &str,
    block: &str,
    block_offset: usize,
    anchor: &str,
    new_value: &str,
) -> Result<String> {
    let anchor_pos = block
        .find(anchor)
        .with_context(|| format!("anchor {anchor:?} not found in keyword block"))?;
    let key_pat = "\"match\": \"";
    let key_off = block[anchor_pos..]
        .find(key_pat)
        .with_context(|| format!("no \"match\" key after anchor {anchor:?}"))?;
    let val_start = anchor_pos + key_off + key_pat.len();
    let bytes = block.as_bytes();
    let mut end = val_start;
    loop {
        match bytes.get(end) {
            Some(b'"') => break,
            Some(b'\\') => end += 2,
            Some(_) => end += 1,
            None => bail!("unterminated JSON string after anchor {anchor:?}"),
        }
    }
    // `new_value` is a logical regex. JSON needs both its backslashes and
    // the literal double quote accepted by Tcl's `\\\"` escape class quoted.
    let escaped = new_value.replace('\\', "\\\\").replace('"', "\\\"");
    let new_block = format!("{}{escaped}{}", &block[..val_start], &block[end..]);
    Ok(format!(
        "{}{new_block}{}",
        &text[..block_offset],
        &text[block_offset + block.len()..]
    ))
}

/// Replace exactly the parser-retained string at this target schema path.
fn splice_source_rule(
    text: &str,
    repository: &str,
    pattern: usize,
    key: &str,
    replacement: &str,
) -> Result<String> {
    use source_json::PathPart::{Index, Key};
    source_json::replace_string(
        text,
        &[
            Key("repository"),
            Key(repository),
            Key("patterns"),
            Index(pattern),
            Key(key),
        ],
        replacement,
    )
}

#[allow(clippy::too_many_lines)]
fn static_source_roles(original: &str, b: &Buckets) -> Result<String> {
    use source_json::PathPart::{Index, Key};
    let display = b
        .source_display
        .as_ref()
        .context("Registry source-display recipe unavailable")?;
    let words = |names: &[&str]| {
        names
            .iter()
            .map(|name| regex::escape(name))
            .collect::<Vec<_>>()
            .join("|")
    };
    let name = tcl_syntax::naming::textmate_unquoted_entity_name();
    let named = words(&display.named_callables);
    let prefix = r"(?<=^|\s|;|\[)";
    let named_match = format!(r"{prefix}({named})\s+({name})");
    let json: serde_json::Value = serde_json::from_str(original)?;
    let rich_definitions = json["repository"].get("parameter-list").is_some();
    let mut text = original.to_owned();
    if rich_definitions {
        text = splice_source_rule(
            &text,
            "definitions",
            0,
            "begin",
            &format!(r"{named_match}\s*(\{{)"),
        )?;
        text = splice_source_rule(
            &text,
            "definitions",
            1,
            "begin",
            &format!(r"{prefix}({})\s*(\{{)", words(&display.constructors)),
        )?;
        let classes = display
            .classes
            .iter()
            .map(|(command, subcommand)| {
                subcommand.map_or_else(
                    || regex::escape(command),
                    |sub| format!(r"{}\s+{}", regex::escape(command), regex::escape(sub)),
                )
            })
            .collect::<Vec<_>>()
            .join("|");
        text = splice_source_rule(
            &text,
            "definitions",
            2,
            "match",
            &format!(r"{prefix}({classes})\s+({name})"),
        )?;
        text = splice_source_rule(
            &text,
            "definitions",
            3,
            "match",
            &format!(r"{prefix}({})\s+({name})", words(&display.class_references)),
        )?;
        text = splice_source_rule(&text, "definitions", 4, "match", &named_match)?;
        let parameters = tcl_syntax::formal_params::static_formal_parameter_display();
        text = splice_source_rule(&text, "parameter-list", 0, "match", parameters.defaulted)?;
        text = splice_source_rule(&text, "parameter-list", 1, "match", parameters.plain)?;
    } else {
        text = splice_source_rule(&text, "definitions", 0, "match", &named_match)?;
    }
    for (index, closer) in [(0, r"\{"), (1, r#"""#)] {
        let begin = format!(
            r"{prefix}({})\s+((?:-[a-z]+\s+)*(?:--\s+)?)({closer})",
            words(&display.regex_commands)
        );
        text = splice_source_rule(&text, "regexp-command", index, "begin", &begin)?;
    }
    let options = display
        .regex_options
        .iter()
        .map(|option| {
            option
                .strip_prefix('-')
                .map(regex::escape)
                .context("source option lacks prefix")
        })
        .collect::<Result<Vec<_>>>()?
        .join("|");
    let option_regex = format!(r"-(?:{options})\b");
    for index in 0..2 {
        text = source_json::replace_string(
            &text,
            &[
                Key("repository"),
                Key("regexp-command"),
                Key("patterns"),
                Index(index),
                Key("beginCaptures"),
                Key("2"),
                Key("patterns"),
                Index(0),
                Key("match"),
            ],
            &option_regex,
        )?;
    }
    Ok(text)
}

/// Patch the two (byte-for-byte identical in this section) `.tmLanguage.json`
/// grammars: VS Code's and the `JetBrains` copy. The `keyword` repository
/// entry is immediately followed by `operator`, which bounds the block.
fn render_tmlanguage_json(original: &str, b: &Buckets) -> Result<String> {
    let mut text = static_source_roles(original, b)?;
    for (anchor, words) in [
        ("\"keyword.control.tcl\"", &b.control),
        ("\"keyword.other.tcl\"", &b.other),
        ("\"support.function.tcl\"", &b.builtin),
    ] {
        let (block, offset) = bounded_block(&text, "\"keyword\": {", "\"operator\": {")?;
        let (block, offset) = (block.to_owned(), offset);
        text = splice_json_match(&text, &block, offset, anchor, &boundary_regex(words))?;
    }
    let lexical = lexical_regexes();
    text = splice_source_rule(&text, "variable", 0, "match", &lexical.braced_variable)?;
    for (start, end, anchor, regex) in [
        (
            "\"variable\": {",
            "\"string-quoted\": {",
            "\"variable.other.tcl\"",
            lexical.variable.as_str(),
        ),
        (
            "\"number\": {",
            "\"escape\": {",
            "\"constant.numeric.hex.tcl\"",
            lexical.hex.as_str(),
        ),
        (
            "\"number\": {",
            "\"escape\": {",
            "\"constant.numeric.octal.tcl\"",
            lexical.octal.as_str(),
        ),
        (
            "\"number\": {",
            "\"escape\": {",
            "\"constant.numeric.binary.tcl\"",
            lexical.binary.as_str(),
        ),
        (
            "\"number\": {",
            "\"escape\": {",
            "\"constant.numeric.tcl\"",
            lexical.decimal.as_str(),
        ),
        (
            "\"escape\": {",
            "\n  }\n}",
            "\"constant.character.escape.tcl\"",
            lexical.escape.as_str(),
        ),
    ] {
        let (block, offset) = bounded_block(&text, start, end)?;
        text = splice_json_match(&text, block, offset, anchor, regex)?;
    }
    Ok(text)
}

const VSCODE_GRAMMAR: &str = "editors/vscode/syntaxes/tcl.tmLanguage.json";
const JETBRAINS_GRAMMAR: &str = "editors/jetbrains/src/main/resources/syntaxes/tcl.tmLanguage.json";

/// A per-target grammar renderer: original file content + the classified
/// buckets in, patched content out.
type RenderFn = fn(&str, &Buckets) -> Result<String>;

/// Write (or, with `check`, verify) the two `TextMate` grammar files and print a
/// coverage report to stderr.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let reg = CommandRegistry::build_default();
    let b = classify(&reg);

    eprintln!(
        "  tcl-family  {:>4} commands ({} control, {} other-keyword, {} builtin)",
        b.control.len() + b.other.len() + b.builtin.len(),
        b.control.len(),
        b.other.len(),
        b.builtin.len(),
    );

    let targets: &[(&str, RenderFn)] = &[
        (VSCODE_GRAMMAR, render_tmlanguage_json),
        (JETBRAINS_GRAMMAR, render_tmlanguage_json),
    ];

    let mut drift = Vec::new();
    for &(rel, render) in targets {
        let path = root.join(rel);
        let original =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let content = render(&original, &b).with_context(|| format!("rendering {rel}"))?;
        if check {
            if content != original {
                drift.push(rel);
            }
        } else if content != original {
            fs::write(&path, &content).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("wrote {rel}");
        }
    }

    if check && !drift.is_empty() {
        eprintln!(
            "{} TextMate-family grammar file(s) are stale — run `cargo xtask gen-tmlanguage-keywords`:",
            drift.len()
        );
        for rel in &drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: all TextMate-family grammar keyword lists are in sync with the registry.");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_disjoint_and_nonempty() {
        let reg = CommandRegistry::build_default();
        let b = classify(&reg);
        assert!(b.control.is_disjoint(&b.other));
        assert!(b.control.is_disjoint(&b.builtin));
        assert!(b.other.is_disjoint(&b.builtin));
        // Bigger than the ~61-entry old hand list (it was missing genuine
        // ambient builtins, not just over-curated), but stays well under the
        // raw ambient surface (~265) once test-harness/internal-namespace
        // noise is filtered — see `builtin_bucket_excludes_test_harness_noise`.
        assert!(b.builtin.len() > 100, "got {}", b.builtin.len());
        for w in CONTEXT_SENSITIVE_EXCLUDE {
            assert!(!b.control.contains(*w) && !b.other.contains(*w) && !b.builtin.contains(*w));
        }
    }

    #[test]
    fn builtin_bucket_excludes_test_harness_noise() {
        let reg = CommandRegistry::build_default();
        let b = classify(&reg);
        // `tclUnixTest.c`-only debug commands — never present in a normal
        // interpreter, so not "common" by any definition.
        for w in ["testalarm", "testaccessproc", "testchannel"] {
            assert!(
                !b.builtin.contains(w),
                "{w} should be filtered as test-harness noise"
            );
        }
        // Tcl's own reserved internal-implementation namespace.
        for w in ["::tcl::mathop::eq", "tcl::process"] {
            assert!(
                !b.builtin.contains(w),
                "{w} should be filtered as internal-namespace noise"
            );
        }
        // A same-named *real* ambient command must survive the "test" prefix
        // filter — it only excludes `test` + more, not the bare word.
        assert!(
            reg.get("time").is_some(),
            "sanity: `time` is a real command"
        );
    }

    #[test]
    fn lmap_is_control_not_builtin() {
        // The miscategorisation this generator exists to prevent.
        let reg = CommandRegistry::build_default();
        let b = classify(&reg);
        assert!(b.control.contains("lmap"));
        assert!(!b.builtin.contains("lmap"));
        assert!(!b.other.contains("lmap"));
    }

    #[test]
    fn lexical_projection_uses_shared_modern_number_escape_and_name_owners() {
        let lexical = lexical_regexes();
        // Tcl 9's explicit decimal prefix and separator runs must reach every
        // static fallback grammar.
        assert!(lexical.decimal.contains("0[dD]"));
        assert!(lexical.decimal.contains("_+"));
        // TIP 388's wide unicode form and its first-octal-digit cap are both
        // lexical width rules, not merely decoded-value rules.
        assert!(
            lexical
                .escape
                .contains("U(?:(?:000[0-9a-fA-F]{4}|0010[0-9a-fA-F]{3})")
        );
        assert!(lexical.escape.contains("[0-3][0-7]{0,2}"));
        assert!(lexical.escape.contains("[4-7][0-7]?"));
        assert!(lexical.escape.contains("{}()"));
        // Namespace separator runs use the shared naming fragment, matching a
        // colon run of any length, not only exactly two.
        assert!(lexical.variable.contains("[:]{2,}"));
    }

    #[test]
    fn wide_unicode_projection_stops_at_parse_hex_cap() {
        let lexical = lexical_regexes();
        let re = regex::Regex::new(&lexical.escape).expect("generated escape regex is valid");
        for (source, expected) in [
            (r"\UFFFFFFFF", r"\UFFFFF"),
            (r"\U00110000", r"\U0011000"),
            (r"\U0010FFFF", r"\U0010FFFF"),
        ] {
            assert_eq!(
                re.find(source).map(|m| m.as_str()),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    fn braced_variable_display_matches_original_first_close_geometry() {
        let lexical = lexical_regexes();
        let pattern = regex::Regex::new(&lexical.braced_variable).unwrap();
        for source in [
            "${}",
            "${space name}",
            "${café}",
            "${a{b}tail}",
            r"${a\}tail}",
        ] {
            let (_, rest) = tcl_syntax::naming::split_braced_var_ref(
                source,
                tcl_dialect::BracedVarStyle::FirstClose,
            )
            .expect("original lexer owns a closed reference");
            assert_eq!(
                pattern.find(source).unwrap().end(),
                source.len() - rest.len()
            );
        }
        for source in ["$ordinary", "${unterminated", "${"] {
            assert!(pattern.find(source).is_none());
        }
        let nested = "${a{b}tail}";
        let (_, modern_rest) = tcl_syntax::naming::split_braced_var_ref(
            nested,
            tcl_dialect::BracedVarStyle::Tcl9Nesting,
        )
        .unwrap();
        assert!(modern_rest.is_empty());
        assert!(pattern.find(nested).unwrap().end() < nested.len());
    }

    #[test]
    fn committed_grammars_match_generated() {
        let root = repo_root();
        let reg = CommandRegistry::build_default();
        let b = classify(&reg);
        for (rel, render) in [
            (VSCODE_GRAMMAR, render_tmlanguage_json as RenderFn),
            (JETBRAINS_GRAMMAR, render_tmlanguage_json),
        ] {
            let path = root.join(rel);
            let original = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            let content = render(&original, &b).unwrap_or_else(|e| panic!("rendering {rel}: {e}"));
            assert_eq!(
                content, original,
                "{rel} is stale — run `cargo xtask gen-tmlanguage-keywords`"
            );
        }
    }

    /// Splicing must be confined to the `keyword` block's own three `match`
    /// values, never the *other* `"support.function.tcl"` occurrence inside
    /// `regexp-command` (its capture there tags `regexp`/`regsub` itself, and
    /// sits right before an unrelated `keyword.operator.option.tcl` pattern's
    /// `"match"` key — an early, unbounded version of this generator spliced
    /// the registry's command list into *that* regex by mistake). Since the
    /// committed grammar is itself already generated, re-rendering it must be
    /// a byte-for-byte no-op — the strongest form of "touched nothing else".
    #[test]
    fn json_splice_is_confined_and_idempotent() {
        let root = repo_root();
        let original = fs::read_to_string(root.join(VSCODE_GRAMMAR)).unwrap();
        let reg = CommandRegistry::build_default();
        let b = classify(&reg);
        let content = render_tmlanguage_json(&original, &b).unwrap();
        assert_eq!(
            content, original,
            "re-rendering the committed grammar must be a no-op"
        );
        // The unrelated `regexp`/`regsub` option-word regex (anchored via its
        // own distinctive scope name) must still be intact and untouched.
        assert!(content.contains("\"name\": \"keyword.operator.option.tcl\""));
        assert!(content.contains("-(?:nocase|expanded|line|linestop|lineanchor"));
        // The compact single-line `fileTypes` array — the reason a full-file
        // JSON reserialisation isn't used — must survive untouched. Matched
        // by shape against the committed grammar rather than by a pinned
        // list: `tclspec` joined it when SpecTcl packs became editable, and
        // what this pins is the single-line form surviving the splice, not
        // which extensions happen to be in it.
        let file_types = original
            .lines()
            .find(|line| line.trim_start().starts_with("\"fileTypes\""))
            .expect("the grammar declares fileTypes");
        assert!(
            file_types.trim_end().ends_with("],"),
            "fileTypes must stay on one line, or the splice guard is moot: {file_types}"
        );
        assert!(content.contains(file_types));
    }

    #[test]
    fn lexical_splices_are_shared_by_both_textmate_grammars() {
        let root = repo_root();
        let b = classify(&CommandRegistry::build_default());
        let lexical = lexical_regexes();
        for (rel, render) in [
            (VSCODE_GRAMMAR, render_tmlanguage_json as RenderFn),
            (JETBRAINS_GRAMMAR, render_tmlanguage_json),
        ] {
            let original = fs::read_to_string(root.join(rel)).unwrap();
            let rendered = render(&original, &b).unwrap();
            assert_eq!(rendered, original, "{rel} is stale");
            // The logical TextMate fragment is present directly in YAML, but
            // JSON doubles every backslash. Check owner-specific bodies whose
            // punctuation is invariant under that serialisation difference.
            assert!(rendered.contains("0[dD]"), "{rel} lacks 0d");
            assert!(rendered.contains("[:]{2,}"), "{rel} lacks colon runs");
            assert!(
                rendered.contains("U(?:(?:000[0-9a-fA-F]{4}|0010[0-9a-fA-F]{3})"),
                "{rel} lacks capped \\U"
            );
            assert!(rendered.contains("[4-7][0-7]?"), "{rel} lacks octal cap");
            assert!(
                rendered.contains(&lexical.decimal.replace('\\', "\\\\"))
                    || rendered.contains(&lexical.decimal)
            );
        }
    }
}
