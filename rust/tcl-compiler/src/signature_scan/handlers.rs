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

//! Per-command handlers used by the `signature_scan` walker.
//!
//! Each handler is a small free function that takes the segmented
//! command pieces (head text, per-arg texts, per-arg representative
//! tokens, surrounding namespace prefix) and mutates a [`ScanCtx`].
//! Handlers do not recurse into bodies themselves — body recursion
//! lives in [`super::walker`] (`handle_if` / `handle_catch` /
//! `handle_try` / `maybe_recurse_body`) and the namespace-eval body
//! recursion lives inside `handle_namespace` here, which calls back
//! into `walker::maybe_recurse_body`.
//!
//! [`ScanCtx`]: super::ctx::ScanCtx

use tcl_lexer::{Token, TokenType};

use super::ctx::{FactoryCandidate, ProcBodyInfo, ScanCtx};
use super::params::parse_param_list;
use super::types::{
    SignatureAutoPathEntry, SignatureClass, SignatureCommandAlias, SignatureNamespaceImport,
    SignaturePackageRequire, SignatureProc, SignatureRename, SignatureScanResult, SignatureSource,
};

/// Fully qualify `name` within `ns_prefix` following Tcl scoping.
///
/// Absolute
/// names (those starting with `::`) ignore the prefix entirely so a
/// `proc ::foo::bar` declared inside `namespace eval baz` still
/// indexes as `::foo::bar`.
///
/// `ns_prefix` is expected to be the call-site namespace **without**
/// a leading `::` (the walker carries it that way by convention).
pub(super) fn qualify(ns_prefix: &str, name: &str) -> String {
    crate::naming::qualify(ns_prefix, name)
}

/// Resolve `word` against `parent`'s registry subcommand table, returning
/// the canonical subcommand name.
///
/// Resolution follows the registry's ensemble rule — exact match or unique
/// non-empty prefix, C Tcl's `Tcl_GetIndexFromObj` dispatch — so the
/// abbreviated spellings C Tcl accepts (`namespace ev`, `package req`)
/// resolve to their full names, and an ambiguous prefix (`interp al` —
/// `alias`/`aliases`) resolves to nothing, exactly as tclsh errors on it.
/// Falls back to the word as written when no registry is threaded (focused
/// unit tests) or the word doesn't resolve, preserving exact-name matching.
fn canonical_subcommand<'w>(
    registry: Option<&tcl_registry::CommandRegistry>,
    parent: &str,
    word: &'w str,
) -> &'w str {
    registry
        .and_then(|r| r.get(parent))
        .and_then(|spec| spec.resolve_subcommand(word))
        .map_or(word, |sub| sub.name)
}

/// Insert a class record under `result.classes`, computing the
/// qualified name + simple-name split.
///
/// Shared by `handle_oo_class` and `handle_itcl_class` so both
/// `oo::class create NAME ?BODY?` and `itcl::class NAME BODY`
/// produce identically-shaped records.
pub(super) fn emit_class(
    raw_name: &str,
    name_tok: Token,
    body_tok: Token,
    ns_prefix: &str,
    result: &mut SignatureScanResult,
) {
    let qualified = qualify(ns_prefix, raw_name);
    let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
    result.classes.insert(
        qualified.clone(),
        SignatureClass {
            name: simple,
            qualified_name: qualified,
            name_range: name_tok.span,
            body_range: body_tok.span,
        },
    );
}

/// Handler for `proc NAME PARAMS BODY`.
///
/// Records a `SignatureProc` in `ctx.result.procs` and a
/// `ProcBodyInfo` in `ctx.proc_bodies` so the second-pass factory
/// resolver can identify factory-wrapper procs by their
/// `proc $a $b $c` body shape.
///
/// Body recursion (`scan_factory_candidates`) is **not** wired in:
/// the proc is recorded, but its body is not walked.
pub(super) fn handle_proc(texts: &[String], argv: &[Token], ns_prefix: &str, ctx: &mut ScanCtx) {
    if texts.len() < 4 {
        return;
    }
    let raw_name = &texts[1];
    let qualified = qualify(ns_prefix, raw_name);
    let simple = qualified.rsplit("::").next().unwrap_or("").to_string();
    let name_range = argv[1].span;
    let body_range = argv[3].span;
    let params = parse_param_list(&texts[2]);
    let param_names: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
    ctx.result.procs.insert(
        qualified.clone(),
        SignatureProc {
            name: simple,
            qualified_name: qualified.clone(),
            params,
            name_range,
            body_range,
        },
    );
    let body_ns = match qualified.rsplit_once("::") {
        Some((parent, _)) => parent.trim_start_matches(':').to_string(),
        None => String::new(),
    };
    let body_text = texts[3].clone();
    ctx.proc_bodies.push(ProcBodyInfo {
        qname: qualified,
        params: param_names,
        body_text: body_text.clone(),
        ns_prefix: body_ns.clone(),
    });
    // Walk the proc body for factory-wrapper candidate calls.
    // Only braced bodies can be statically scanned; substituted
    // bodies (`$body`, `[gen_body]`) cannot be re-segmented.
    if argv[3].kind == TokenType::Str {
        super::walker::scan_factory_candidates(&body_text, argv[3], &body_ns, ctx);
    }
}

/// Handler for the `namespace` command — dispatches on the
/// subcommand (`eval` vs `import`).
///
/// The `eval` arm computes the
/// inner namespace prefix (absolute names rebase via leading `::`,
/// otherwise nest under the current prefix); body recursion into
/// the eval body is a stub here.
pub(super) fn handle_namespace(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    conditional: bool,
    known_commands: &std::collections::HashSet<&str>,
    ctx: &mut ScanCtx,
) {
    if texts.len() < 2 {
        return;
    }
    // Resolve the subcommand word through the registry's ensemble rule so
    // the abbreviated spellings C Tcl accepts (`namespace ev` /
    // `namespace imp`) dispatch like their full names.
    let sub = canonical_subcommand(ctx.registry, "namespace", &texts[1]);
    if sub == "eval" && texts.len() >= 4 {
        let raw_ns = &texts[2];
        let inner_prefix = if let Some(rest) = raw_ns.strip_prefix("::") {
            rest.trim_start_matches(':').to_string()
        } else if !ns_prefix.is_empty() {
            format!("{ns_prefix}::{raw_ns}")
        } else {
            raw_ns.clone()
        };
        super::walker::maybe_recurse_body(
            &texts[3],
            argv[3],
            &inner_prefix,
            conditional,
            known_commands,
            ctx,
        );
        return;
    }
    if sub == "import" && texts.len() >= 3 {
        handle_namespace_import(texts, argv, ns_prefix, &mut ctx.result);
    }
}

/// Handler for `namespace import ?-force? PATTERN ?PATTERN…?`.
///
/// Records every static pattern;
/// patterns that still carry a `$` / `[` substitution or that lack
/// any `::` namespace segment are skipped (we cannot statically
/// resolve them to a source namespace). Patterns without a leading
/// `::` are resolved relative to the *current* namespace, mirroring
/// Tcl's own rule.
pub(super) fn handle_namespace_import(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    result: &mut SignatureScanResult,
) {
    let importing_ns = if ns_prefix.is_empty() {
        "::".to_string()
    } else {
        format!("::{ns_prefix}")
    };
    let mut i = 2;
    while i < texts.len() {
        let pattern_raw = &texts[i];
        if pattern_raw == "-force" {
            i += 1;
            continue;
        }
        if !pattern_raw.contains("::") || pattern_raw.contains('$') || pattern_raw.contains('[') {
            i += 1;
            continue;
        }
        let pattern = if pattern_raw.starts_with("::") {
            pattern_raw.clone()
        } else if importing_ns == "::" {
            format!("::{pattern_raw}")
        } else {
            format!("{importing_ns}::{pattern_raw}")
        };
        result.namespace_imports.push(SignatureNamespaceImport {
            ns: importing_ns.clone(),
            pattern,
            range: argv[i].span,
            conjectured: false,
        });
        i += 1;
    }
}

/// Handler for `package require ?-exact? NAME ?VERSION?`.
///
/// Records a
/// `SignaturePackageRequire`; the optional `-exact` flag is parsed
/// and skipped (we do not currently distinguish exact from
/// minimum-version requires); the optional `VERSION` is captured
/// when present.  The subcommand word resolves through the registry's
/// ensemble rule, so C Tcl's accepted abbreviation (`package req Tcl`)
/// records the requirement too.
pub(super) fn handle_package(
    texts: &[String],
    argv: &[Token],
    conditional: bool,
    registry: Option<&tcl_registry::CommandRegistry>,
    result: &mut SignatureScanResult,
) {
    if texts.len() < 3 || canonical_subcommand(registry, "package", &texts[1]) != "require" {
        return;
    }
    let mut idx = 2;
    if texts[idx] == "-exact" && texts.len() > idx + 1 {
        idx += 1;
    }
    let pkg_name = texts[idx].clone();
    let version = if texts.len() > idx + 1 {
        Some(texts[idx + 1].clone())
    } else {
        None
    };
    result.package_requires.push(SignaturePackageRequire {
        name: pkg_name,
        version,
        range: argv[idx].span,
        conditional,
    });
}

/// Handler for `source ?-encoding ENC? PATH`.
///
/// Walks past every leading `-FLAG` (consuming a follow-up word for
/// `-encoding`); the remaining word is recorded as the path. The
/// `is_literal` flag is set when the segmenter-reconstructed word
/// contains no `$` or `[` substitution markers.
pub(super) fn handle_source(texts: &[String], argv: &[Token], result: &mut SignatureScanResult) {
    let mut idx = 1;
    while idx < texts.len() && texts[idx].starts_with('-') {
        if texts[idx] == "-encoding" && idx + 1 < texts.len() {
            idx += 2;
        } else {
            idx += 1;
        }
    }
    if idx >= texts.len() {
        return;
    }
    let raw = texts[idx].clone();
    let is_literal = !raw.contains('$') && !raw.contains('[');
    result.source_targets.push(SignatureSource {
        raw_path: raw,
        range: argv[idx].span,
        is_literal,
    });
}

/// Handler for `interp alias SLAVE-PATH NAME TARGET-PATH TARGET ?ARG…?`.
///
/// Records only **local-interpreter** aliases (slave path and
/// target path both empty `{}`); cross-interpreter aliases install
/// commands inside child interpreters and are not visible to
/// workspace command resolution, so they are skipped.  The subcommand
/// word resolves through the registry's ensemble rule — which, matching
/// tclsh, rejects `interp al` / `interp alia` as ambiguous with
/// `aliases`, so in practice only the full spelling dispatches here.
pub(super) fn handle_interp(
    texts: &[String],
    registry: Option<&tcl_registry::CommandRegistry>,
    result: &mut SignatureScanResult,
) {
    if texts.len() < 6 || canonical_subcommand(registry, "interp", &texts[1]) != "alias" {
        return;
    }
    if !texts[2].is_empty() || !texts[4].is_empty() {
        return;
    }
    let alias_name = texts[3].clone();
    let target = texts[5].clone();
    let extras: Vec<String> = texts.iter().skip(6).cloned().collect();
    let qualified = qualify("", &alias_name);
    result.command_aliases.insert(
        qualified.clone(),
        SignatureCommandAlias {
            qualified_name: qualified,
            target,
            extras,
        },
    );
}

/// Handler for `rename OLD NEW`.
///
/// `NEW` becomes a callable command name subject to ordinary namespace
/// resolution (like `proc`, unlike `interp alias`'s always-global
/// aliasName), so it is qualified against `ns_prefix`. `rename OLD {}`
/// deletes `OLD` rather than introducing a new name, so an empty `NEW`
/// is skipped.
pub(super) fn handle_rename(texts: &[String], ns_prefix: &str, result: &mut SignatureScanResult) {
    if texts.len() < 3 {
        return;
    }
    let old_name = texts[1].clone();
    let new_name = &texts[2];
    if new_name.is_empty() {
        return;
    }
    let qualified = qualify(ns_prefix, new_name);
    result.renames.insert(
        qualified.clone(),
        SignatureRename {
            qualified_name: qualified,
            target: old_name,
        },
    );
}

/// Handler for `oo::class create NAME ?BODY?` (any `TclOO` metaclass —
/// the walker dispatches every registry `IS_OO_METACLASS` +
/// `definition_body` bearer here).
///
/// When `BODY` is absent the body span falls back to the name token.
///
/// The `create` word stays a literal match: the metaclass specs model
/// their `create` / `new` / `createWithNamespace` shapes via
/// `arg_role_resolver` (there is no `SubCommand` table to resolve
/// against), and `create` is a `TclOO` *method* word — method dispatch is
/// exact-match in C Tcl (`oo::class cr` errors "unknown method"), so an
/// ensemble-style prefix resolution would be wrong here.  `new` makes
/// anonymous classes (nothing to index) and `createWithNamespace` is not
/// modelled.
pub(super) fn handle_oo_class(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    result: &mut SignatureScanResult,
) {
    if texts.len() < 3 || texts[1] != "create" {
        return;
    }
    let body_tok = if argv.len() > 3 { argv[3] } else { argv[2] };
    emit_class(&texts[2], argv[2], body_tok, ns_prefix, result);
}

/// Record a potential factory-wrapper call for post-scan resolution.
///
/// A tcllib-style factory call
/// has the shape `HEAD NAME ARGS BODY` (four tokens total, last
/// braced). HEADs in `ctx.skip_heads` (the registry's
/// `NOT_PROC_FACTORY` heads plus the non-command residual) are
/// built-ins that happen to match the same shape; they are skipped.
/// Names with
/// `$` / `[` substitution markers cannot be statically resolved
/// and are skipped. The body must be a `Str` token (braced
/// literal); unbraced bodies cannot be re-scanned for the
/// `proc $a $b $c` shape that confirms a real factory.
pub(super) fn maybe_record_factory_candidate(
    head: &str,
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    ctx: &mut ScanCtx,
) {
    if texts.len() != 4 {
        return;
    }
    if ctx.skip_heads.contains(head) {
        return;
    }
    let name = &texts[1];
    if name.is_empty() || name.contains('$') || name.contains('[') {
        return;
    }
    let body_tok = argv[3];
    if body_tok.kind != TokenType::Str {
        return;
    }
    ctx.candidates.push(FactoryCandidate {
        head: head.to_string(),
        name: name.clone(),
        name_tok: argv[1],
        body_tok,
        ns_prefix: ns_prefix.to_string(),
    });
}

/// Recognise the tcllib `<NS>::import <ALIAS>` wrapper idiom and
/// emit a conjectural `namespace import` record.
///
/// Statically detecting all
/// such wrappers from their bodies is out of scope, but the *call*
/// is unambiguous: head ends with `::import`, single argument, no
/// substitution markers in the alias. The conjectured import is
/// recorded with `conjectured = true` so consumers can weight it
/// less heavily.
pub(super) fn maybe_handle_import_wrapper(
    head: &str,
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    result: &mut SignatureScanResult,
) {
    if !head.ends_with("::import") {
        return;
    }
    if texts.len() < 2 {
        return;
    }
    let alias = &texts[1];
    if alias.is_empty() || alias.contains('$') || alias.contains('[') {
        return;
    }
    if texts.len() > 2 {
        return;
    }
    let source_ns_raw = &head[..head.len() - "::import".len()];
    let source_ns = if source_ns_raw.starts_with("::") {
        source_ns_raw.to_string()
    } else {
        format!("::{source_ns_raw}")
    };
    let alias_ns = if alias.starts_with("::") {
        alias.clone()
    } else if !ns_prefix.is_empty() {
        format!("::{ns_prefix}::{alias}")
    } else {
        format!("::{alias}")
    };
    result.namespace_imports.push(SignatureNamespaceImport {
        ns: alias_ns,
        pattern: format!("{source_ns}::*"),
        range: argv[0].span,
        conjectured: true,
    });
}

/// Handler for `lappend auto_path …` / `set auto_path …`.
///
/// Both forms are accepted; every
/// word after `auto_path` is recorded as a `SignatureAutoPathEntry`.
/// Path-resolution to absolute filesystem paths happens later in
/// the analyser pipeline.
///
/// `auto_path` here is a *variable name*, not a subcommand — the
/// registry's special-variable table (`tcl_registry::special_vars`)
/// declares the variable and its interpreter write-effect, but "is the
/// package auto-load path the workspace indexer must resolve" is this
/// consumer's own question, so the name match is deliberate, not a
/// registry bypass.
pub(super) fn handle_auto_path(texts: &[String], argv: &[Token], result: &mut SignatureScanResult) {
    if texts.len() < 3 || texts[1] != "auto_path" {
        return;
    }
    for i in 2..texts.len() {
        let raw = &texts[i];
        if raw.is_empty() {
            continue;
        }
        result.auto_path_entries.push(SignatureAutoPathEntry {
            raw: raw.clone(),
            range: argv[i].span,
        });
    }
}

/// Handler for `itcl::class NAME BODY` (or `::itcl::class NAME BODY`).
///
/// Always requires a body
/// argument — itcl's class statement is not optional like
/// `oo::class create`.
pub(super) fn handle_itcl_class(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    result: &mut SignatureScanResult,
) {
    if texts.len() < 3 {
        return;
    }
    emit_class(&texts[1], argv[1], argv[2], ns_prefix, result);
}

/// Record a snit type/widget/widgetadaptor as a class so its instance-creating
/// constructor (`Name create obj` / `Name %AUTO%`) types the receiving variable
/// `OBJECT(Name)`.  Same `DEFINER Name Body` shape as itcl.
pub(super) fn handle_snit_type(
    texts: &[String],
    argv: &[Token],
    ns_prefix: &str,
    result: &mut SignatureScanResult,
) {
    if texts.len() < 3 {
        return;
    }
    emit_class(&texts[1], argv[1], argv[2], ns_prefix, result);
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{Span, TokenType};

    fn token(start: u32, end: u32) -> Token {
        Token::new(TokenType::Esc, Span::new(start, end))
    }

    fn str_token(start: u32, end: u32) -> Token {
        Token::with_content_offset(TokenType::Str, Span::new(start, end), 1)
    }

    #[test]
    fn handle_proc_with_braced_body_scans_factory_candidates() {
        // Direct call into handle_proc with a synthetic INIT-proc
        // shape: body is `DEFC bar args {body}` which should
        // register one factory candidate.
        let body_text = "DEFC bar args {body}";
        let texts = vec![
            "proc".to_string(),
            "INIT".to_string(),
            String::new(),
            body_text.to_string(),
        ];
        let argv = vec![token(0, 4), token(5, 9), token(10, 12), str_token(13, 35)];
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "", &mut ctx);
        assert_eq!(ctx.candidates.len(), 1);
    }

    #[test]
    fn handle_proc_with_unbraced_body_does_not_scan() {
        let texts = vec![
            "proc".to_string(),
            "INIT".to_string(),
            String::new(),
            "${dynamic}".to_string(),
        ];
        // Body is plain Esc (not Str), so factory walker should be
        // skipped.
        let argv = vec![token(0, 4), token(5, 9), token(10, 12), token(13, 23)];
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "", &mut ctx);
        assert!(ctx.candidates.is_empty());
    }

    #[test]
    fn absolute_name_ignores_prefix() {
        assert_eq!(qualify("foo::bar", "::baz"), "::baz");
        assert_eq!(qualify("", "::top"), "::top");
        assert_eq!(qualify("nested", "::ns::deep"), "::ns::deep");
    }

    #[test]
    fn relative_name_under_nested_ns() {
        assert_eq!(qualify("foo", "bar"), "::foo::bar");
        assert_eq!(qualify("foo::baz", "qux"), "::foo::baz::qux");
    }

    #[test]
    fn empty_prefix_promotes_to_global() {
        assert_eq!(qualify("", "bar"), "::bar");
    }

    #[test]
    fn emit_class_under_namespace_indexes_bare_name() {
        let mut result = SignatureScanResult::default();
        emit_class("MyCls", token(10, 15), token(20, 30), "ns", &mut result);
        let cls = result.classes.get("::ns::MyCls").expect("inserted");
        assert_eq!(cls.name, "MyCls");
        assert_eq!(cls.qualified_name, "::ns::MyCls");
        assert_eq!(cls.name_range, Span::new(10, 15));
        assert_eq!(cls.body_range, Span::new(20, 30));
    }

    #[test]
    fn emit_class_with_absolute_name_preserves_indexing() {
        let mut result = SignatureScanResult::default();
        emit_class("::Top", token(0, 5), token(6, 8), "ns", &mut result);
        let cls = result
            .classes
            .get("::Top")
            .expect("absolute name preserved");
        assert_eq!(cls.name, "Top");
        assert_eq!(cls.qualified_name, "::Top");
    }

    fn proc_inputs(name: &str) -> (Vec<String>, Vec<Token>) {
        let texts = vec![
            "proc".to_string(),
            name.to_string(),
            "a b".to_string(),
            "set x 1".to_string(),
        ];
        let argv = vec![token(0, 4), token(5, 8), token(10, 13), token(15, 22)];
        (texts, argv)
    }

    #[test]
    fn handle_proc_records_bare_proc() {
        let (texts, argv) = proc_inputs("foo");
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "", &mut ctx);
        let proc = ctx.result.procs.get("::foo").expect("inserted");
        assert_eq!(proc.name, "foo");
        assert_eq!(proc.qualified_name, "::foo");
        assert_eq!(proc.params.len(), 2);
        assert_eq!(proc.params[0].name, "a");
        assert_eq!(proc.name_range, Span::new(5, 8));
        assert_eq!(proc.body_range, Span::new(15, 22));
        assert_eq!(ctx.proc_bodies.len(), 1);
        assert_eq!(ctx.proc_bodies[0].qname, "::foo");
        assert_eq!(ctx.proc_bodies[0].body_text, "set x 1");
        assert_eq!(ctx.proc_bodies[0].ns_prefix, "");
    }

    #[test]
    fn handle_proc_under_namespace_indexes_qualified() {
        let (texts, argv) = proc_inputs("bar");
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "ns::deep", &mut ctx);
        let proc = ctx.result.procs.get("::ns::deep::bar").expect("inserted");
        assert_eq!(proc.name, "bar");
        assert_eq!(ctx.proc_bodies[0].ns_prefix, "ns::deep");
    }

    #[test]
    fn handle_proc_with_absolute_name_records_at_global() {
        let (texts, argv) = proc_inputs("::top::baz");
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "outer", &mut ctx);
        let proc = ctx.result.procs.get("::top::baz").expect("inserted");
        assert_eq!(proc.name, "baz");
        assert_eq!(proc.qualified_name, "::top::baz");
        // body_ns drops the leading colon and trailing simple name.
        assert_eq!(ctx.proc_bodies[0].ns_prefix, "top");
    }

    #[test]
    fn handle_proc_too_few_args_no_op() {
        let texts = vec!["proc".to_string(), "name".to_string()];
        let argv = vec![token(0, 4), token(5, 9)];
        let mut ctx = ScanCtx::default();
        handle_proc(&texts, &argv, "", &mut ctx);
        assert!(ctx.result.procs.is_empty());
        assert!(ctx.proc_bodies.is_empty());
    }

    #[test]
    fn handle_namespace_import_absolute_pattern() {
        let texts = vec![
            "namespace".to_string(),
            "import".to_string(),
            "::foo::*".to_string(),
        ];
        let argv = vec![token(0, 9), token(10, 16), token(17, 25)];
        let mut result = SignatureScanResult::default();
        handle_namespace_import(&texts, &argv, "", &mut result);
        assert_eq!(result.namespace_imports.len(), 1);
        let imp = &result.namespace_imports[0];
        assert_eq!(imp.ns, "::");
        assert_eq!(imp.pattern, "::foo::*");
        assert!(!imp.conjectured);
    }

    #[test]
    fn handle_namespace_import_force_flag_skipped() {
        let texts = vec![
            "namespace".to_string(),
            "import".to_string(),
            "-force".to_string(),
            "::foo::*".to_string(),
        ];
        let argv = vec![token(0, 9), token(10, 16), token(17, 23), token(24, 32)];
        let mut result = SignatureScanResult::default();
        handle_namespace_import(&texts, &argv, "", &mut result);
        assert_eq!(result.namespace_imports.len(), 1);
        assert_eq!(result.namespace_imports[0].pattern, "::foo::*");
    }

    #[test]
    fn handle_namespace_import_relative_under_nested_ns() {
        let texts = vec![
            "namespace".to_string(),
            "import".to_string(),
            "bar::*".to_string(),
        ];
        let argv = vec![token(0, 9), token(10, 16), token(17, 23)];
        let mut result = SignatureScanResult::default();
        handle_namespace_import(&texts, &argv, "foo", &mut result);
        assert_eq!(result.namespace_imports.len(), 1);
        let imp = &result.namespace_imports[0];
        assert_eq!(imp.ns, "::foo");
        assert_eq!(imp.pattern, "::foo::bar::*");
    }

    #[test]
    fn handle_namespace_import_substituted_pattern_skipped() {
        let texts = vec![
            "namespace".to_string(),
            "import".to_string(),
            "${ns}::*".to_string(),
        ];
        let argv = vec![token(0, 9), token(10, 16), token(17, 25)];
        let mut result = SignatureScanResult::default();
        handle_namespace_import(&texts, &argv, "", &mut result);
        assert!(result.namespace_imports.is_empty());
    }

    #[test]
    fn handle_package_with_version() {
        let texts = vec![
            "package".to_string(),
            "require".to_string(),
            "Tcl".to_string(),
            "8.6".to_string(),
        ];
        let argv = vec![token(0, 7), token(8, 15), token(16, 19), token(20, 23)];
        let mut result = SignatureScanResult::default();
        handle_package(&texts, &argv, false, None, &mut result);
        assert_eq!(result.package_requires.len(), 1);
        let req = &result.package_requires[0];
        assert_eq!(req.name, "Tcl");
        assert_eq!(req.version.as_deref(), Some("8.6"));
        assert_eq!(req.range, Span::new(16, 19));
        assert!(!req.conditional);
    }

    #[test]
    fn handle_package_without_version() {
        let texts = vec![
            "package".to_string(),
            "require".to_string(),
            "Tcl".to_string(),
        ];
        let argv = vec![token(0, 7), token(8, 15), token(16, 19)];
        let mut result = SignatureScanResult::default();
        handle_package(&texts, &argv, true, None, &mut result);
        assert_eq!(result.package_requires.len(), 1);
        let req = &result.package_requires[0];
        assert_eq!(req.name, "Tcl");
        assert!(req.version.is_none());
        assert!(req.conditional);
    }

    #[test]
    fn handle_source_literal_path() {
        let texts = vec!["source".to_string(), "/abs/path.tcl".to_string()];
        let argv = vec![token(0, 6), token(7, 20)];
        let mut result = SignatureScanResult::default();
        handle_source(&texts, &argv, &mut result);
        assert_eq!(result.source_targets.len(), 1);
        let st = &result.source_targets[0];
        assert_eq!(st.raw_path, "/abs/path.tcl");
        assert!(st.is_literal);
        assert_eq!(st.range, Span::new(7, 20));
    }

    #[test]
    fn handle_source_substituted_path() {
        let texts = vec!["source".to_string(), "${dir}/x.tcl".to_string()];
        let argv = vec![token(0, 6), token(7, 19)];
        let mut result = SignatureScanResult::default();
        handle_source(&texts, &argv, &mut result);
        assert_eq!(result.source_targets.len(), 1);
        assert!(!result.source_targets[0].is_literal);
    }

    #[test]
    fn handle_source_skips_encoding_option() {
        let texts = vec![
            "source".to_string(),
            "-encoding".to_string(),
            "utf-8".to_string(),
            "/abs/path.tcl".to_string(),
        ];
        let argv = vec![token(0, 6), token(7, 16), token(17, 22), token(23, 36)];
        let mut result = SignatureScanResult::default();
        handle_source(&texts, &argv, &mut result);
        assert_eq!(result.source_targets.len(), 1);
        let st = &result.source_targets[0];
        assert_eq!(st.raw_path, "/abs/path.tcl");
        assert!(st.is_literal);
        assert_eq!(st.range, Span::new(23, 36));
    }

    #[test]
    fn handle_package_with_exact_flag() {
        let texts = vec![
            "package".to_string(),
            "require".to_string(),
            "-exact".to_string(),
            "Tcl".to_string(),
            "8.6".to_string(),
        ];
        let argv = vec![
            token(0, 7),
            token(8, 15),
            token(16, 22),
            token(23, 26),
            token(27, 30),
        ];
        let mut result = SignatureScanResult::default();
        handle_package(&texts, &argv, false, None, &mut result);
        assert_eq!(result.package_requires.len(), 1);
        let req = &result.package_requires[0];
        assert_eq!(req.name, "Tcl");
        assert_eq!(req.version.as_deref(), Some("8.6"));
        assert_eq!(req.range, Span::new(23, 26));
    }

    #[test]
    fn handle_interp_local_alias_emitted() {
        let texts = vec![
            "interp".to_string(),
            "alias".to_string(),
            String::new(),
            "myalias".to_string(),
            String::new(),
            "puts".to_string(),
            "hello".to_string(),
        ];
        let mut result = SignatureScanResult::default();
        handle_interp(&texts, None, &mut result);
        assert_eq!(result.command_aliases.len(), 1);
        let alias = result.command_aliases.get("::myalias").expect("inserted");
        assert_eq!(alias.target, "puts");
        assert_eq!(alias.extras, ["hello".to_string()]);
    }

    #[test]
    fn handle_interp_cross_interp_alias_skipped() {
        let texts = vec![
            "interp".to_string(),
            "alias".to_string(),
            "child".to_string(),
            "foo".to_string(),
            String::new(),
            "puts".to_string(),
        ];
        let mut result = SignatureScanResult::default();
        handle_interp(&texts, None, &mut result);
        assert!(result.command_aliases.is_empty());
    }

    #[test]
    fn handle_rename_records_qualified_target() {
        let texts = vec![
            "rename".to_string(),
            "puts".to_string(),
            "my_puts".to_string(),
        ];
        let mut result = SignatureScanResult::default();
        handle_rename(&texts, "", &mut result);
        assert_eq!(result.renames.len(), 1);
        let rename = result.renames.get("::my_puts").expect("inserted");
        assert_eq!(rename.target, "puts");
        assert_eq!(rename.qualified_name, "::my_puts");
    }

    #[test]
    fn handle_rename_qualifies_against_ns_prefix() {
        let texts = vec![
            "rename".to_string(),
            "puts".to_string(),
            "loud_puts".to_string(),
        ];
        let mut result = SignatureScanResult::default();
        handle_rename(&texts, "myns", &mut result);
        assert!(result.renames.contains_key("::myns::loud_puts"));
    }

    #[test]
    fn handle_rename_absolute_new_name_ignores_ns_prefix() {
        let texts = vec![
            "rename".to_string(),
            "puts".to_string(),
            "::top_puts".to_string(),
        ];
        let mut result = SignatureScanResult::default();
        handle_rename(&texts, "myns", &mut result);
        assert!(result.renames.contains_key("::top_puts"));
    }

    #[test]
    fn handle_rename_to_empty_string_deletes_and_is_skipped() {
        let texts = vec!["rename".to_string(), "puts".to_string(), String::new()];
        let mut result = SignatureScanResult::default();
        handle_rename(&texts, "", &mut result);
        assert!(result.renames.is_empty());
    }

    #[test]
    fn handle_rename_too_few_args_is_a_no_op() {
        let texts = vec!["rename".to_string(), "puts".to_string()];
        let mut result = SignatureScanResult::default();
        handle_rename(&texts, "", &mut result);
        assert!(result.renames.is_empty());
    }

    #[test]
    fn handle_oo_class_with_body() {
        let texts = vec![
            "oo::class".to_string(),
            "create".to_string(),
            "MyCls".to_string(),
            "method foo {} {}".to_string(),
        ];
        let argv = vec![token(0, 9), token(10, 16), token(17, 22), token(23, 39)];
        let mut result = SignatureScanResult::default();
        handle_oo_class(&texts, &argv, "ns", &mut result);
        let cls = result.classes.get("::ns::MyCls").expect("inserted");
        assert_eq!(cls.name, "MyCls");
        assert_eq!(cls.body_range, Span::new(23, 39));
    }

    #[test]
    fn handle_oo_class_with_absolute_name() {
        let texts = vec![
            "oo::class".to_string(),
            "create".to_string(),
            "::Top".to_string(),
        ];
        let argv = vec![token(0, 9), token(10, 16), token(17, 22)];
        let mut result = SignatureScanResult::default();
        handle_oo_class(&texts, &argv, "ns", &mut result);
        let cls = result.classes.get("::Top").expect("absolute preserved");
        assert_eq!(cls.name, "Top");
        // Body falls back to the name token when omitted.
        assert_eq!(cls.body_range, Span::new(17, 22));
    }

    #[test]
    fn handle_itcl_class_records_class() {
        let texts = vec![
            "itcl::class".to_string(),
            "Foo".to_string(),
            "constructor {} {}".to_string(),
        ];
        let argv = vec![token(0, 11), token(12, 15), token(16, 33)];
        let mut result = SignatureScanResult::default();
        handle_itcl_class(&texts, &argv, "", &mut result);
        let cls = result.classes.get("::Foo").expect("inserted");
        assert_eq!(cls.name, "Foo");
        assert_eq!(cls.body_range, Span::new(16, 33));
    }

    #[test]
    fn handle_snit_type_records_class() {
        let texts = vec![
            "snit::type".to_string(),
            "Foo".to_string(),
            "method m {} {}".to_string(),
        ];
        let argv = vec![token(0, 10), token(11, 14), token(15, 29)];
        let mut result = SignatureScanResult::default();
        handle_snit_type(&texts, &argv, "", &mut result);
        let cls = result.classes.get("::Foo").expect("inserted");
        assert_eq!(cls.name, "Foo");
        assert_eq!(cls.body_range, Span::new(15, 29));
    }

    #[test]
    fn handle_auto_path_lappend_form() {
        let texts = vec![
            "lappend".to_string(),
            "auto_path".to_string(),
            "/usr/local/lib".to_string(),
            "/opt/foo".to_string(),
        ];
        let argv = vec![token(0, 7), token(8, 17), token(18, 32), token(33, 41)];
        let mut result = SignatureScanResult::default();
        handle_auto_path(&texts, &argv, &mut result);
        assert_eq!(result.auto_path_entries.len(), 2);
        assert_eq!(result.auto_path_entries[0].raw, "/usr/local/lib");
        assert_eq!(result.auto_path_entries[0].range, Span::new(18, 32));
        assert_eq!(result.auto_path_entries[1].raw, "/opt/foo");
    }

    #[test]
    fn handle_auto_path_set_form() {
        let texts = vec![
            "set".to_string(),
            "auto_path".to_string(),
            "/var/lib/tcl".to_string(),
        ];
        let argv = vec![token(0, 3), token(4, 13), token(14, 26)];
        let mut result = SignatureScanResult::default();
        handle_auto_path(&texts, &argv, &mut result);
        assert_eq!(result.auto_path_entries.len(), 1);
        assert_eq!(result.auto_path_entries[0].raw, "/var/lib/tcl");
    }

    #[test]
    fn maybe_import_wrapper_happy_path() {
        let texts = vec!["term::ansi::send::import".to_string(), "vt".to_string()];
        let argv = vec![token(0, 24), token(25, 27)];
        let mut result = SignatureScanResult::default();
        maybe_handle_import_wrapper("term::ansi::send::import", &texts, &argv, "", &mut result);
        assert_eq!(result.namespace_imports.len(), 1);
        let imp = &result.namespace_imports[0];
        assert_eq!(imp.ns, "::vt");
        assert_eq!(imp.pattern, "::term::ansi::send::*");
        assert!(imp.conjectured);
        assert_eq!(imp.range, Span::new(0, 24));
    }

    #[test]
    fn maybe_import_wrapper_multi_arg_rejected() {
        let texts = vec![
            "foo::import".to_string(),
            "vt".to_string(),
            "extra".to_string(),
        ];
        let argv = vec![token(0, 11), token(12, 14), token(15, 20)];
        let mut result = SignatureScanResult::default();
        maybe_handle_import_wrapper("foo::import", &texts, &argv, "", &mut result);
        assert!(result.namespace_imports.is_empty());
    }

    #[test]
    fn maybe_import_wrapper_dynamic_alias_rejected() {
        let texts = vec!["foo::import".to_string(), "${dyn}".to_string()];
        let argv = vec![token(0, 11), token(12, 18)];
        let mut result = SignatureScanResult::default();
        maybe_handle_import_wrapper("foo::import", &texts, &argv, "", &mut result);
        assert!(result.namespace_imports.is_empty());
    }

    #[test]
    fn factory_candidate_happy_path() {
        let texts = vec![
            "DEFC".to_string(),
            "foo".to_string(),
            "args".to_string(),
            "body".to_string(),
        ];
        let argv = vec![token(0, 4), token(5, 8), token(9, 13), str_token(14, 20)];
        let mut ctx = ScanCtx::default();
        maybe_record_factory_candidate("DEFC", &texts, &argv, "ns", &mut ctx);
        assert_eq!(ctx.candidates.len(), 1);
    }

    #[test]
    fn factory_candidate_skip_head_filtered() {
        let texts = vec![
            "proc".to_string(),
            "foo".to_string(),
            "args".to_string(),
            "body".to_string(),
        ];
        let argv = vec![token(0, 4), token(5, 8), token(9, 13), str_token(14, 20)];
        let mut ctx = ScanCtx::default();
        ctx.skip_heads.insert("proc".to_string());
        maybe_record_factory_candidate("proc", &texts, &argv, "", &mut ctx);
        assert!(ctx.candidates.is_empty());
    }

    #[test]
    fn factory_candidate_dynamic_name_skipped() {
        let texts = vec![
            "DEFC".to_string(),
            "${name}".to_string(),
            "args".to_string(),
            "body".to_string(),
        ];
        let argv = vec![token(0, 4), token(5, 12), token(13, 17), str_token(18, 24)];
        let mut ctx = ScanCtx::default();
        maybe_record_factory_candidate("DEFC", &texts, &argv, "", &mut ctx);
        assert!(ctx.candidates.is_empty());
    }

    #[test]
    fn factory_candidate_unbraced_body_skipped() {
        let texts = vec![
            "DEFC".to_string(),
            "foo".to_string(),
            "args".to_string(),
            "body".to_string(),
        ];
        // body argv[3] is plain ESC, not Str (not braced) — should skip.
        let argv = vec![token(0, 4), token(5, 8), token(9, 13), token(14, 20)];
        let mut ctx = ScanCtx::default();
        maybe_record_factory_candidate("DEFC", &texts, &argv, "", &mut ctx);
        assert!(ctx.candidates.is_empty());
    }
}
