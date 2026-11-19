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

//! Static mini-evaluator for `lappend auto_path` / `set auto_path`
//! arguments.
//!
//! A common Tcl idiom adds a sibling / parent directory to the package
//! search path using only a handful of side-effect-free built-ins:
//!
//! ```tcl
//! lappend auto_path [file join [file dirname [info script]] ..]
//! lappend auto_path [file dirname [file dirname [info script]]]
//! lappend auto_path /abs/path
//! lappend auto_path ~/lib
//! ```
//!
//! The supported subset is `[info script]`, `[file dirname …]`,
//! `[file join …]`, `[file normalize …]` (anchored arguments only — see
//! [`eval_file_normalize`]), the other `file` name operations the registry
//! routes (`tail`, `extension`, `rootname`, `split`) over names every
//! platform reads alike, literal words, and `~`-prefixed paths. A name
//! operation reads the registry's route where it answers, and the
//! host's slash-form reading below for the rest.
//! Variable references (`$dir`, `${dir}`, `pre_$dir`) are resolved through
//! whatever resolver the **caller** supplies — this module owns what a path
//! expression *means*, the caller owns what a variable *is worth*, and the
//! two compose through [`evaluate_auto_path_expr_with_resolver`].  A
//! variable the resolver cannot answer (including every variable, for the
//! resolver-less [`evaluate_auto_path_expr`]), an array element, or a
//! list-manipulation command returns `None` and the caller skips the
//! entry.  We never guess.
//!
//! The standalone evaluators consume explicit authoring expressions. A
//! consumer with retained analysis uses [`capture_source_path_expression`]:
//! it selects the authored [`tcl_registry::SourcePathOperation`] and effective
//! original arguments, including bound alias values. It does not infer an
//! operation from a written or reported head. The resulting plan retains the
//! actual source, configuration and context; it supplies conditional path
//! advice, not observed runtime values or filesystem state.
//!
//! # Path space
//!
//! All path arithmetic here happens in Tcl's **slash form**, on every host.
//! That is not a POSIX assumption — it is what Tcl itself does: `filename(n)`
//! specifies that the `file` commands accept either separator on Windows and
//! *return* `/`-separated paths, so `[file dirname C:/x/y]` is `C:/x` in a
//! Windows interpreter exactly as it is written here.  A native path handed in
//! as `info_script` (`C:\repo\user.tcl`, straight out of `Uri::to_file_path`)
//! is converted at the boundary by [`to_tcl_slash_form`], and the result is
//! returned in slash form too — which `PathBuf::from` accepts on Windows.
//!
//! Three absolute roots are recognised (see [`root_len`]): POSIX `/…`, a
//! Windows drive `C:/…`, and a UNC share `//server/share/…`.  A *drive-relative*
//! path (`C:` or `C:foo`, resolved against a per-drive working directory only
//! the running process knows) is outside the subset and abstains, like any
//! other unsupported shape.
//!
//! # A relative result stays relative
//!
//! `lappend auto_path lib` / `set v helper.tcl; source $v` fold to a path
//! with no root.  Tcl resolves such a path against the **interpreter's
//! working directory at run time** (tclsh 8.6.16 and 9.0.4:
//! `cd other; tclsh ../sub/main.tcl` where `main.tcl` does
//! `set v helper.tcl; source $v` loads `other/helper.tcl`, *not*
//! `sub/helper.tcl` — `[info script]` plays no part), which is a fact no
//! static analysis can know.
//!
//! This evaluator therefore returns such a result **relative**, normalised
//! but unanchored, and each caller applies the base it actually has — the
//! sourcing document's directory for the source graph
//! (`source_graph::resolve_source_target`), the workspace root for document
//! links (`document_links::resolve_path`), the analysed file's own directory
//! for the package database. It deliberately does **not** absolutise against
//! the *analysing process's* current directory: the language server's launch
//! directory is an editor artefact with no relationship to the user's
//! program, so anchoring on it makes the same file resolve differently
//! depending on how the editor happened to be started, and makes a computed
//! path disagree with the literal path right beside it (which every caller
//! already anchors on its own base).

use crate::analyser::types::{AutoPathEntry, AutoPathForm};
use crate::segmenter::segment_commands_with_offset_and_config;
use std::collections::HashMap;

/// Every directory one recorded [`AutoPathEntry`] adds to `auto_path`.
///
/// This is the supported way to read the fact, because the two mutation forms
/// differ in arity — see [`AutoPathForm`].  An [`AutoPathForm::Assign`] record
/// holds a Tcl *list*, so it is split with the shared list grammar
/// ([`tcl_syntax::list::split_list`]) before each element is folded; an
/// [`AutoPathForm::Append`] record is one directory and is folded whole, so a
/// braced `lappend auto_path {p q}` stays the single directory `p q`.
///
/// A *computed* right-hand side is never list-split.  The segmenter
/// reconstructs a substituted word with its `${var}` / `[cmd]` markers intact,
/// so their absence is reliable evidence the word is a plain literal (the same
/// test `SignatureSource::is_literal` uses).  `set auto_path [file dirname
/// [info script]]` is one computed directory, not three list elements, and is
/// handed whole to [`evaluate_auto_path_expr`]; `set auto_path [list /a /b]`
/// is outside the supported subset and contributes nothing.
///
/// A literal word, by contrast, is already a path *value* and is used as one —
/// re-parsing it would split `lappend auto_path {/opt/p q}` into two words and
/// lose a directory Tcl treats as a single element.
///
/// Entries outside the supported subset contribute nothing — we never guess.
///
/// `profile` is the document's resolved dialect profile, whose list grammar
/// divides an [`AutoPathForm::Assign`] record's word.
#[must_use]
pub fn evaluate_auto_path_entry(
    entry: &AutoPathEntry,
    info_script: Option<&str>,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<String> {
    evaluate_auto_path_entry_with_constants(entry, info_script, &HashMap::new(), profile)
}

/// [`evaluate_auto_path_entry`], with the document's single-assignment
/// constants ([`constant_path_assignments`], folded) answering variable
/// references — which is what carries the corpus idiom
/// `set libDir [file join $dir lib]; lappend auto_path $libDir` — the shape
/// a package uses to register its own `lib` directory.
#[must_use]
pub fn evaluate_auto_path_entry_with_constants<C: PathConstantLookup + ?Sized>(
    entry: &AutoPathEntry,
    info_script: Option<&str>,
    constants: &C,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> Vec<String> {
    let raw = entry.raw_path.as_str();
    if raw.contains('$') || raw.contains('[') {
        // Computed: fold it as an expression, as one directory.
        return evaluate_auto_path_expr_with_constants(raw, info_script, constants, profile)
            .into_iter()
            .collect();
    }
    if entry.form == AutoPathForm::Append {
        return fold_path_value(raw).into_iter().collect();
    }
    let rules = tcl_syntax::word_rules::WordValueRules::of_profile(profile);
    let Ok(elements) = rules.split_list(raw) else {
        return Vec::new();
    };
    elements
        .iter()
        .filter_map(|el| fold_path_value(el))
        .collect()
}

/// The commands a document of `dialect` reads, or of no named dialect
/// ([`tcl_dialect::DialectProfile::plain_tcl`]).
fn commands_of(
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> &'static tcl_registry::CommandRegistry {
    tcl_registry::model::static_context_for_profile(
        dialect.unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl),
    )
    .commands()
}

/// Fold one already-substituted path *value* (a `~` expansion plus lexical
/// normalisation), abstaining on a drive-relative path.  The shared tail of
/// [`evaluate_auto_path_expr`] and the per-element path of
/// [`evaluate_auto_path_entry`].
///
/// A rootless value stays rootless — see the module docs' "A relative result
/// stays relative".
fn fold_path_value(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let expanded = expanduser(&to_tcl_slash_form(value));
    if is_drive_relative(&expanded) {
        // `C:` / `C:foo` names a per-drive working directory only the running
        // process knows.  Outside the subset — abstain rather than guess.
        return None;
    }
    Some(normalise_path_value(&expanded))
}

/// Resolve `raw` (a `lappend auto_path` argument) to a normalised path,
/// substituting `info_script` for `[info script]` (the filesystem path
/// of the file being analysed — what Tcl itself would report).
///
/// The result is absolute whenever the expression is anchored — by
/// `[info script]`, by a literal root, or by `~` — and **relative**
/// otherwise, for the caller to anchor (see the module docs' "A relative
/// result stays relative").
///
/// Returns `None` when any part of the expression is outside the
/// supported subset. It reads the commands of a document of no named dialect
/// ([`tcl_dialect::DialectProfile::plain_tcl`]).
#[must_use]
pub fn evaluate_auto_path_expr(raw: &str, info_script: Option<&str>) -> Option<String> {
    evaluate_auto_path_expr_with_constants(raw, info_script, &HashMap::new(), None)
}

/// Whether `text` still carries an unevaluated substitution — a `$` variable
/// reference or a `[` command substitution.
///
/// The segmenter reconstructs a word with its `${var}` / `[cmd]` markers
/// intact, so their absence is reliable evidence the word is a plain literal
/// (the same test `SignatureSource::is_literal` uses).  A word that still
/// carries one is *not* a path, however plausible it looks spelled out.
#[must_use]
pub fn carries_substitution(text: &str) -> bool {
    text.contains('$') || text.contains('[')
}

/// [`evaluate_auto_path_expr_with_resolver`] over a retained constant view — the
/// convenience form for callers whose variable facts are already folded.
/// A plain `HashMap` is the explicit authored C naming abstraction; live
/// consumers retain [`FoldedPathConstants`] and select an original site with
/// [`FoldedPathConstants::at`]. Under the plain-map abstraction, qualified
/// spellings canonicalise on lookup, so `$::snit::library`, `$snit::library`
/// (relative, read from the global context), and `$::dir` (a
/// globally-qualified reference to a top-level global) all reach their key.
///
/// Names absent from `constants` stay unresolved, so the fold abstains on
/// them exactly as it does on any other unevaluated substitution — passing an
/// empty map is precisely [`evaluate_auto_path_expr`].
#[must_use]
pub fn evaluate_auto_path_expr_with_constants<C: PathConstantLookup + ?Sized>(
    raw: &str,
    info_script: Option<&str>,
    constants: &C,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<String> {
    evaluate_auto_path_expr_with_resolver(
        raw,
        info_script,
        &|name| constants.path_constant(name),
        dialect,
    )
}

/// The full evaluator: fold `raw` with variable references answered by
/// `resolve_var` — the provenance/semantics split that makes this a shared
/// lower layer rather than one consumer's helper.
///
/// This module owns what a path expression **means** (`[file join …]`,
/// `[file dirname …]`, `[file normalize …]`, `[info script]`, `~`); the
/// caller owns what a variable **is worth** at the point being analysed, and
/// says so through `resolve_var`.  A simple consumer passes a single-
/// assignment constant map ([`evaluate_auto_path_expr_with_constants`]); a
/// consumer with real flow information (the analyser's constant-string
/// lattice, an SCCP-backed value table) passes its own resolver and gets the
/// same path semantics with better provenance — the evaluator does not care
/// where the values came from, only that the caller vouches for them.
///
/// Resolution happens on the parsed expression, **never** by splicing values
/// into the raw text and re-lexing: a value containing spaces is still one
/// word (`$d` = `my dir` in `[file join $d x]` folds to `my dir/x`, where
/// textual substitution would re-parse it as two words and answer
/// `my/dir/x`), and a value containing `[` or `$` is data, not syntax to be
/// re-interpreted.
///
/// An unresolved variable — `resolve_var` answering `None` — abstains the
/// whole fold, the module-wide rule: we never guess.
#[must_use]
pub fn evaluate_auto_path_expr_with_resolver(
    raw: &str,
    info_script: Option<&str>,
    resolve_var: &dyn Fn(&str) -> Option<String>,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<String> {
    evaluate_auto_path_expr_with_resolver_and_braced_vars(
        raw,
        info_script,
        resolve_var,
        dialect.map_or_else(tcl_dialect::BracedVarStyle::default, |profile| {
            profile.grammar.braced_var
        }),
        dialect,
    )
}

/// The source path algebra with an explicitly retained variable-delimiter
/// grammar. Other source syntax and command premises belong to the caller's
/// checked original words; this helper supplies no invocation authority.
#[must_use]
pub fn evaluate_auto_path_expr_with_resolver_and_braced_vars(
    raw: &str,
    info_script: Option<&str>,
    resolve_var: &dyn Fn(&str) -> Option<String>,
    braced_vars: tcl_dialect::BracedVarStyle,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<String> {
    let config = tcl_lexer::LexerConfig {
        braced_var: braced_vars,
        ..tcl_lexer::LexerConfig::for_profile(dialect)
    };
    evaluate_path_source(raw, info_script, resolve_var, config, dialect)
}

fn evaluate_path_source(
    raw: &str,
    info_script: Option<&str>,
    resolve_var: &dyn Fn(&str) -> Option<String>,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Option<String> {
    let image = tcl_lexer::SourceImage::document(raw);
    let span = tcl_lexer::Span::new(0, u32::try_from(raw.len()).ok()?);
    let plan = tcl_lexer::native_script_words_in_with_purpose(
        image,
        span,
        config,
        tcl_lexer::SourceSegmentationPurpose::NestedScript,
    )
    .ok()?;
    let [word] = single_command(&plan)? else {
        return None;
    };
    // Preserve the standalone API's native-path input once its full extent
    // is known to be a single word. Computed words follow source grammar.
    if word.bytes() == raw.as_bytes() && is_windows_shaped(raw) && !carries_substitution(raw) {
        return fold_path_value(raw);
    }
    // `info script` reports the path as Tcl sees it, which is slash form on
    // every host, so the native path is converted before it enters the fold.
    // A drive-relative script path would make every `file dirname` of it
    // meaningless, so it is dropped rather than folded.
    let info_script = info_script
        .map(to_tcl_slash_form)
        .filter(|p| !is_drive_relative(p));
    let registry = commands_of(dialect);
    let result = path_word(&plan, word, info_script.as_deref(), resolve_var, registry)?;
    // Expand `~` and normalise (collapsing `..`) so the parent-dir idiom
    // resolves to a real directory.  A rootless result is left for the
    // caller to anchor.
    fold_path_value(&result)
}

/// Explicit standalone path-source simulation under a supplied dialect.
/// Top-level writes and supported namespace declarations retain lexical homes;
/// ordinary procedure/control bodies contribute no entered-scope facts. Values
/// fold in source order, and multiply written homes remain unavailable.
///
/// Consumers with an actual analysis use
/// [`constant_path_assignments_from_analysis`] or its retained inventory,
/// whose original words, selected operations, full context and configuration
/// remain attached. This wrapper cannot restore missing analysis premises.
#[must_use]
pub fn constant_path_vars(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    info_script: Option<&str>,
) -> FoldedPathConstants {
    fold_constant_assignments(&constant_path_assignments(source, dialect), info_script)
}

mod source_expression;
pub use source_expression::{
    OriginalSourceAutoPathExpression, OriginalSourceFileRewrite, OriginalSourcePathExpression,
    capture_source_auto_path_expression, capture_source_path_expression,
    capture_source_path_expression_at_analysis, capture_source_target_path_expression,
};

mod path_constants;
pub use path_constants::{
    FoldedPathConstants, PathConstantAssignments, PathConstantImports, PathConstantLookup,
    PathConstantValue, PathConstantView, PathConstantWrite, constant_path_assignments,
    constant_path_assignments_from_analysis, constant_path_assignments_from_commands,
    constant_path_assignments_in_namespace, constant_path_assignments_with_naming_policy,
    fold_constant_assignments, fold_constant_assignments_with_imports,
    path_source_word_is_available,
};
pub(crate) use path_constants::{
    extend_path_constant_assignments_from_analysis_commands,
    path_constant_prefix_for_source_analysis,
};

/// A plain scalar variable word: no substitution markers, no array element.
fn is_plain_scalar_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('$') && !name.contains('[') && !name.contains('(')
}

/// Preserve variable tails with the shared written-name grammar.
fn rooted_variable_key(name: &str) -> String {
    crate::naming::qualify("::", name)
}

/// Keep the compatibility map's bare spelling for unqualified global variables.
fn qualified_key(ns_prefix: &str, name: &str) -> String {
    if ns_prefix.is_empty() && !crate::naming::is_qualified(name.as_bytes()) {
        return name.to_owned();
    }
    crate::naming::qualify(ns_prefix, name)
}

/// The `PathConstantValue::Raw` for the word at `word_index` (an index into
/// `seg.texts`, i.e. counting the command word).
fn raw_value(seg: &crate::segmenter::SegmentedCommand, word_index: usize) -> PathConstantValue {
    PathConstantValue::Raw {
        text: seg.texts[word_index].clone(),
        literal: seg
            .argv
            .get(word_index)
            .is_some_and(|tok| tok.kind == tcl_lexer::TokenType::Str)
            && seg.single_token_word.get(word_index) == Some(&true),
    }
}

/// The argument index of the body of a *namespace-declaring* invocation
/// (`namespace eval NAME { … }`), or `None` when this call is not one.
///
/// Registry-driven: the invocation carries
/// [`tcl_registry::Traits::DECLARES_NAMESPACE`] — the trait whose sole
/// carrier today is `namespace eval`, and whose doc records why `namespace
/// inscope` (identical argument layout, same lowering hook) must *not* count
/// — and the words are found through [`tcl_registry::ArgRole::NamespaceName`]
/// and [`tcl_registry::ArgRole::Body`] rather than the fixed indices 2 and 3
/// with a `words[1] == "eval"` test.
///
/// The body must be the last word, keeping the previous restriction to the
/// single-braced-word form: `namespace eval ns a b` concatenates its trailing
/// words into a script, which this tier does not model.
fn namespace_body_index(
    registry: &tcl_registry::CommandRegistry,
    head: &str,
    args: &[&str],
) -> Option<usize> {
    tcl_registry::model::resolve_invocation_in_context(registry, None, head, args)?
        .authored_source_namespace_body_arguments()
        .map(|(_, body)| body)
}

/// The `(name index, value index)` argument pairs a load-time command
/// *assigns* — the registry's answer, rather than recognising `set` and
/// `variable` by name.
///
/// Two shapes, both spec-declared:
///
/// * a **repeating** name/value tail — a
///   [`tcl_registry::repeated::RepeatedArgLayout`] of
///   [`tcl_registry::ArgRole::VarWrite`] at stride 2 anchored on
///   [`tcl_registry::CommandSpec::assigns_variable_at`], which is `variable n
///   v n v`'s declaration;
/// * a **single** assignment whose value is the one word after the name —
///   `assigns_variable_at` with exactly that write and exactly two arguments,
///   which is `set name value` (and, for free, Tcl 9's `const name value`).
///
/// Read-modify-write commands are excluded by
/// [`tcl_registry::Traits::READS_BEFORE_WRITE`] (`append` / `incr` /
/// `lappend` do not assign the word after the name), and whole-array writers
/// by [`tcl_registry::Traits::WHOLE_ARRAY_ARG`] (`$name` is not the value of
/// an `array set`).  Everything else falls through to
/// the selected mutation inventory, which is the safe direction.
fn assigned_name_value_indices(
    registry: &tcl_registry::CommandRegistry,
    head: &str,
    args: &[&str],
) -> Vec<(usize, Option<usize>)> {
    tcl_registry::model::resolve_invocation_in_context(registry, None, head, args)
        .and_then(|schema| schema.authored_source_assignment_arguments())
        .unwrap_or_default()
}

/// A native filesystem path as Tcl's slash form.
///
/// Backslashes are separators **only** in a Windows-shaped path — one opening
/// with a `X:` drive prefix or a `\\server` UNC prefix.  On a POSIX host a
/// backslash is an ordinary filename character, so an unshaped path is
/// returned untouched and `/tmp/a\b` keeps its literal `\`.
#[must_use]
pub fn to_tcl_slash_form(p: &str) -> String {
    if is_windows_shaped(p) {
        p.replace('\\', "/")
    } else {
        p.to_owned()
    }
}

/// Does `p` open with a Windows volume prefix — `X:` or `\\` / `//`-plus-host?
fn is_windows_shaped(p: &str) -> bool {
    if p.starts_with("\\\\") {
        return true;
    }
    let mut chars = p.chars();
    matches!(
        (chars.next(), chars.next()),
        (Some(c), Some(':')) if c.is_ascii_alphabetic()
    )
}

/// Is `p` (slash form) drive-relative — a `X:` prefix *not* followed by `/`?
///
/// `C:foo` resolves against drive C's own working directory, which a static
/// analysis cannot know, so callers abstain on it.
fn is_drive_relative(p: &str) -> bool {
    is_windows_shaped(p) && !p.starts_with("//") && !p.starts_with("\\\\") && {
        // Windows-shaped and not UNC ⇒ a `X:` drive prefix.
        !p[2..].starts_with('/')
    }
}

/// Byte length of `p`'s absolute root, or `None` when `p` is relative.
///
/// The three roots, all in slash form:
///
/// | Shape | Root | Example |
/// |-------|------|---------|
/// | POSIX / Windows volume-relative | `/` | `/a/b` → 1 |
/// | Windows drive | `X:/` | `C:/a/b` → 3 |
/// | UNC share | `//server/share` | `//srv/sh/a` → 7 |
///
/// A UNC root spans *both* the host and the share, because `//server` alone is
/// not a directory: `file dirname //server/share` is `//server/share` itself.
/// Path arithmetic never rewrites or ascends past the root.
fn root_len(p: &str) -> Option<usize> {
    if is_windows_shaped(p) && !p.starts_with("//") && p[2..].starts_with('/') {
        return Some(3); // `C:/`
    }
    if p.starts_with("//") && !p.starts_with("///") {
        // `//server/share` — take two components; short of that, the whole
        // string is (an incomplete) root.
        let rest = &p[2..];
        let host = rest.find('/').unwrap_or(rest.len());
        let after_host = &rest[host..];
        if after_host.is_empty() {
            return Some(p.len());
        }
        let share = after_host[1..].find('/').unwrap_or(after_host.len() - 1);
        return Some(2 + host + 1 + share);
    }
    p.starts_with('/').then_some(1)
}

/// Only one complete command can supply a supported path operation.
fn single_command(plan: &tcl_lexer::NativeScriptWordsPlan) -> Option<&[tcl_lexer::NativeWord]> {
    let [command] = plan.commands.as_slice() else {
        return None;
    };
    plan.fatal_tail
        .is_none()
        .then_some(command.words.as_slice())
}

fn path_word(
    plan: &tcl_lexer::NativeScriptWordsPlan,
    word: &tcl_lexer::NativeWord,
    info_script: Option<&str>,
    resolve_var: &dyn Fn(&str) -> Option<String>,
    registry: &tcl_registry::CommandRegistry,
) -> Option<String> {
    use tcl_lexer::ExecutablePart;
    use tcl_registry::value_transfer::{EvalRoute, NativeEvalId};
    let arena = word.executable_parts();
    if word.group().expand {
        return None;
    }
    if let [component] = arena.list(arena.root())
        && let ExecutablePart::Command { body } = component.part
    {
        let child = plan.reborrow_command_substitution_plan(word, body).ok()?;
        let (head, args) = single_command(&child)?.split_first()?;
        let name = tcl_syntax::word_rules::original_static_word_unicode_value(head)?;
        let args = args
            .iter()
            .map(|arg| path_word(&child, arg, info_script, resolve_var, registry))
            .collect::<Option<Vec<_>>>()?;
        let head = std::str::from_utf8(&name).ok()?;
        let values = args.iter().map(String::as_str).collect::<Vec<_>>();
        let schema =
            tcl_registry::model::resolve_invocation_in_context(registry, None, head, &values)?;
        if let Some(selected) = schema.authored_source_path_operation() {
            return source_expression::evaluate_path_operation(
                selected.operation,
                args.get(selected.arguments)?,
                info_script,
            );
        }
        // These file-name operations have a portable value route but require
        // no filesystem/host fallback. Only the selected path recipes qualify.
        let semantics = schema.semantics.value.semantics()?;
        let EvalRoute::Direct {
            id:
                NativeEvalId::PathTail
                | NativeEvalId::PathExtension
                | NativeEvalId::PathRootname
                | NativeEvalId::PathSplit,
        } = semantics.route()
        else {
            return None;
        };
        return tcl_registry::value_transfer::evaluate_literal(
            semantics,
            schema.canonical_command,
            Some(schema.subcommand.resolved()?.canonical_name),
            values.get(schema.semantics.argument_offset..)?,
            registry.runtime_version(),
        );
    }
    let value = tcl_syntax::word_rules::original_word_unicode_value_with_variables(
        word,
        |arena, component| {
            let ExecutablePart::Variable { name, index: None } = component.part else {
                return None;
            };
            resolve_var(std::str::from_utf8(arena.bytes(name)?).ok()?).map(String::into_bytes)
        },
    )?;
    if word.group().kind == tcl_lexer::WordKind::Braced && value.contains(&b'$') {
        return None;
    }
    Some(to_tcl_slash_form(std::str::from_utf8(&value).ok()?))
}

/// `file normalize` in slash form, or `None` when the argument is not already
/// anchored.
///
/// `file normalize` always *returns* an absolute path: given a relative one it
/// resolves against the interpreter's run-time working directory, which is
/// exactly the fact this module refuses to guess at (module docs, "A relative
/// result stays relative").  So only an anchored argument folds — by a literal
/// root, or by `~`, which [`expanduser`] resolves here for the same reason
/// [`fold_path_value`] does at the top level.
///
/// For an anchored path the command's remaining work is collapsing `.` and
/// `..`, which [`normpath`] already does.  Symbolic links are *not* resolved:
/// tclsh follows them, and this cannot without touching the filesystem — the
/// gap is deliberate, and it only ever leaves a path a reader would recognise
/// rather than one pointing somewhere else.
///
/// This is what makes the corpus idiom fold — `set dir [file normalize [file
/// dirname [info script]]]`, and the equivalent `[file dirname [file normalize
/// [info script]]]`, both of which are anchored by `[info script]`.  Without
/// it every `source [file join $dir …]` in a file written that way abstains.
/// A drive-relative `C:foo` is rejected by the same test, since [`root_len`]
/// only counts `X:/` as a root — it names a per-drive working directory, which
/// is no more knowable than the process-wide one.
fn eval_file_normalize(path: &str) -> Option<String> {
    let expanded = expanduser(&to_tcl_slash_form(path));
    root_len(&expanded)?;
    Some(normpath(&expanded))
}

/// `file dirname` in slash form — everything up to (not including) the last
/// `/`, collapsing trailing slashes on the head, but never shortening the path
/// past its root ([`root_len`]).
///
/// The root clamp is what makes the Windows shapes come out right:
/// `C:/x` → `C:/` (not the drive-relative `C:`), and
/// `//server/share/dir` → `//server/share` (not `//server`).
///
/// A rootless single-component path has `.` for its directory, exactly as
/// tclsh 8.6.16 / 9.0.4 answer `file dirname helper.tcl` — not the empty
/// string, and emphatically not the analysing process's working directory.
fn path_dirname(p: &str) -> String {
    let root = root_len(p).unwrap_or(0);
    let head_or_dot = |head: &str| {
        if head.is_empty() {
            ".".to_owned()
        } else {
            head.to_owned()
        }
    };
    if p.len() <= root {
        return head_or_dot(&p[..root]);
    }
    let Some(idx) = p[root..].rfind('/') else {
        // No separator past the root: the head is the root, or `.` for a
        // single-component relative path.
        return head_or_dot(&p[..root]);
    };
    let head = &p[..root + idx];
    if head.len() <= root {
        return head_or_dot(&p[..root]);
    }
    head_or_dot(head.trim_end_matches('/'))
}

/// `file join` in slash form — a later *absolute* component resets the
/// accumulator; otherwise components are joined with a single `/`.
///
/// One Windows subtlety from `filename(n)`: a `/foo` component is
/// *volumerelative*, not absolute, so joined onto a drive- or UNC-rooted
/// accumulator it replaces the tail while keeping the volume —
/// `file join C:/a /b` is `C:/b`, not `/b`.
fn path_join(parts: &[String]) -> String {
    let mut result = parts[0].clone();
    for p in &parts[1..] {
        match (root_len(p), root_len(&result)) {
            // A volume-relative `/foo` onto a volume-rooted accumulator keeps
            // the volume and replaces everything after it.
            (Some(1), Some(vol)) if vol > 1 && !p.starts_with("//") => {
                result.truncate(vol);
                if !result.ends_with('/') {
                    result.push('/');
                }
                result.push_str(p.trim_start_matches('/'));
            }
            (Some(_), _) => result.clone_from(p),
            (None, _) => {
                if result.is_empty() || result.ends_with('/') {
                    result.push_str(p);
                } else {
                    result.push('/');
                    result.push_str(p);
                }
            }
        }
    }
    result
}

/// POSIX path tilde-expansion for the bare `~` and `~/…` forms; a
/// `~user` form is left unchanged (no passwd lookup).  `~` expands to
/// `$HOME` when set.
fn expanduser(p: &str) -> String {
    if !p.starts_with('~') {
        return p.to_owned();
    }
    // Only the bare `~` (followed by `/` or end) is expanded.
    let after = &p[1..];
    if (after.is_empty() || after.starts_with('/'))
        && let Ok(home) = std::env::var("HOME")
    {
        let home = home.trim_end_matches('/');
        return format!("{home}{after}");
    }
    p.to_owned()
}

/// Normalise `p` (slash form) — collapse `.`, `..` and repeated separators
/// — **without** anchoring a rootless path anywhere.
///
/// An unanchored relative result is Tcl's own "resolve against the
/// interpreter's run-time working directory" (module docs), which is not
/// statically knowable; absolutising it against the *analysing process's*
/// working directory would substitute an unrelated editor artefact for it.
/// The caller — which knows the sourcing document, the workspace root, or
/// both — anchors it instead.
fn normalise_path_value(p: &str) -> String {
    normpath(p)
}

/// Path normalisation in slash form — collapse `.`, `..` and repeated
/// separators, keeping the root ([`root_len`]) verbatim so `..` can never
/// ascend past `/`, a drive (`C:/`) or a UNC share (`//server/share`).
fn normpath(p: &str) -> String {
    if p.is_empty() {
        return ".".to_owned();
    }
    let root = root_len(p);
    let (prefix, tail) = match root {
        Some(n) => (&p[..n], &p[n..]),
        None => ("", p),
    };
    let mut out: Vec<&str> = Vec::new();
    for comp in tail.split('/') {
        if comp.is_empty() || comp == "." {
            continue;
        }
        if comp != ".." || (root.is_none() && out.is_empty()) || out.last() == Some(&"..") {
            out.push(comp);
        } else if !out.is_empty() {
            out.pop();
        }
    }
    // The root in canonical form: `/`, `C:/`, or `//server/share` (which
    // already ends in a component, so it takes a separator before the tail).
    let mut result = prefix.trim_end_matches('/').to_owned();
    if root.is_some() && (result.is_empty() || result.ends_with(':')) {
        result.push('/');
    }
    let tail = out.join("/");
    if !tail.is_empty() {
        if !result.is_empty() && !result.ends_with('/') {
            result.push('/');
        }
        result.push_str(&tail);
    }
    if result.is_empty() {
        ".".to_owned()
    } else {
        result
    }
}

#[cfg(test)]
mod tests;
