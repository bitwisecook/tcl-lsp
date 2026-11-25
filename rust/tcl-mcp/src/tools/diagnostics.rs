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

//! The diagnostics tools under the call's policy: `analyze`, `validate`, `review`, `find-legacy` and `code_actions`.

use super::editor_queries::SOURCE_URI;
use super::source_structure::{detect_events, doc_symbol_to_json, event_order_list};
use super::{
    Analysed, PolicyInputs, analyse_under, arg_str, arg_u32, lsp_range_json, resolve_dialect,
    span_range_json,
};
use serde_json::{Map, Value, json};
use tcl_lexer::SourceMap;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::diagnostic_policy::{Finding, Reason, Report, Shown};

/// Serialise one shown finding to `{code, severity, message, range,
/// category, fixes?}` (the `_facade_diagnostic_to_dict` wire shape), at the
/// severity the policy resolved.
pub(super) fn diag_to_json(shown: &Shown<'_>, sm: &SourceMap<'_>) -> Value {
    let d = shown.finding;
    let code = d.code.as_str();
    let mut obj = json!({
        "code": code,
        "severity": shown.severity.as_str(),
        "message": d.message,
        "range": span_range_json(sm, d.span),
        "category": crate::diag_meta::meta().categorise(code),
    });
    if let Some(subject) = d.structured_data() {
        obj["data"] = subject;
    }
    if !d.fixes.is_empty() {
        let fixes: Vec<Value> = d
            .fixes
            .iter()
            .map(|f| {
                json!({
                    "range": span_range_json(sm, f.span),
                    "new_text": f.new_text,
                    "description": f.description,
                })
            })
            .collect();
        obj.as_object_mut()
            .expect("json object")
            .insert("fixes".to_owned(), Value::Array(fixes));
    }
    obj
}

/// One suppressed finding as `{code, range, reason, message}`
/// (`docs/design/compiler/diagnostic-policy.md` § Adapters, MCP JSON).
/// `message` is added beside the page's three keys: without it a suppressed
/// W210 does not say which variable.
fn suppressed_to_json(finding: &Finding, reason: Reason, sm: &SourceMap<'_>) -> Value {
    json!({
        "code": finding.code.as_str(),
        "range": span_range_json(sm, finding.span),
        "reason": reason.to_string(),
        "message": finding.message,
    })
}

/// The `suppressed` array a diagnostics tool payload gains: every suppressed
/// finding `keep` allows through [`suppressed_to_json`], then every declared
/// gap but the default-off seed (`null` range and message), then one entry
/// per producer the tool did not run and reason, `{producer, codes, range,
/// reason, message}` — its codes listed together, not an entry each: like
/// the seed, they are the same on every call and would bury the answer.
/// `keep` restricts every part (a not-run entry keeps the codes that pass,
/// and goes when none does), so an agent can see that a finding exists and
/// was suppressed rather than concluding the code is clean.
fn suppressed_json(report: &Report, sm: &SourceMap<'_>, keep: impl Fn(&str) -> bool) -> Vec<Value> {
    let mut out: Vec<Value> = report
        .suppressed()
        .filter(|(finding, _)| keep(finding.code.as_str()))
        .map(|(finding, reason)| suppressed_to_json(finding, reason, sm))
        .collect();
    out.extend(
        report
            .gaps()
            .filter(|(code, reason)| !matches!(reason, Reason::DefaultOff) && keep(code.as_str()))
            .map(|(code, reason)| {
                json!({
                    "code": code.as_str(),
                    "range": Value::Null,
                    "reason": reason.to_string(),
                    "message": Value::Null,
                })
            }),
    );
    out.extend(report.not_run().into_iter().filter_map(|row| {
        let codes: Vec<&str> = row
            .codes
            .iter()
            .map(|code| code.as_str())
            .filter(|code| keep(code))
            .collect();
        (!codes.is_empty()).then(|| {
            json!({
                "producer": row.producer.as_str(),
                "codes": codes,
                "range": Value::Null,
                "reason": row.reason.to_string(),
                "message": row.message(),
            })
        })
    }));
    out
}

// Each diagnostics tool reads the report's shown set and keeps its own
// grouping — the `diag_meta` categories, the security / taint / thread
// split, the convertible-code table — which is presentation and stays here
// (`docs/design/compiler/diagnostic-policy.md` § Adapters). The `_with`
// forms take the layers explicitly, so a test can resolve under layers of
// its own rather than under the machine's `config.ini`.

pub(super) fn analyze(args: &Value) -> Value {
    analyze_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

pub(super) fn analyze_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(&analysed.analysis_text);
    let diagnostics: Vec<Value> = report.shown().map(|d| diag_to_json(&d, &sm)).collect();
    let symbols: Vec<Value> = tcl_lsp_core::document_symbols::document_symbols_from_analysis(
        &analysed.analysis_text,
        &analysed.findings.analysis,
    )
    .iter()
    .map(doc_symbol_to_json)
    .collect();
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "diagnostic_count": diagnostics.len(),
        "diagnostics": diagnostics,
        "symbols": symbols,
        "events": detect_events(&analysed.analysis_text, &analysed.findings.analysis),
        "event_order": event_order_list(&analysed.analysis_text, &analysed.findings.analysis),
        "suppressed": suppressed_json(&report, &sm, |_| true),
    })
}

pub(super) fn validate(args: &Value) -> Value {
    validate_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

pub(super) fn validate_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(&analysed.analysis_text);
    let meta = crate::diag_meta::meta();
    let mut categories = Map::new();
    for (key, label) in &meta.category_order {
        let items: Vec<Value> = report
            .shown()
            .filter(|d| meta.categorise(d.finding.code.as_str()) == key)
            .map(|d| diag_to_json(&d, &sm))
            .collect();
        if !items.is_empty() {
            categories.insert(key.clone(), json!({ "label": label, "items": items }));
        }
    }
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "categories": Value::Object(categories),
        "total": report.shown().count(),
        "suppressed": suppressed_json(&report, &sm, |_| true),
    })
}

pub(super) fn review(args: &Value) -> Value {
    review_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

pub(super) fn review_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(&analysed.analysis_text);
    let meta = crate::diag_meta::meta();
    let filt = |set: &std::collections::HashSet<String>| -> Vec<Value> {
        report
            .shown()
            .filter(|d| set.contains(d.finding.code.as_str()))
            .map(|d| diag_to_json(&d, &sm))
            .collect()
    };
    let security = filt(&meta.security_codes);
    let taint = filt(&meta.taint_codes);
    let thread = filt(&meta.thread_codes);
    let total = security.len() + taint.len() + thread.len();
    let suppressed = suppressed_json(&report, &sm, |code| {
        meta.security_codes.contains(code)
            || meta.taint_codes.contains(code)
            || meta.thread_codes.contains(code)
    });
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "security": security,
        "taint": taint,
        "thread_safety": thread,
        "total": total,
        "suppressed": suppressed,
    })
}

pub(super) fn find_legacy(args: &Value) -> Value {
    find_legacy_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

pub(super) fn find_legacy_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    let report = analysed.diagnostics_report();
    let sm = SourceMap::new(&analysed.analysis_text);
    // Shared with `tcl-cli`'s `find-legacy` verb (`tcl_cli::CONVERTIBLE_CODES`/
    // `conversion_for`) rather than a second hand-duplicated copy of the same
    // 6-code table.
    let patterns: Vec<Value> = report
        .shown()
        .filter(|d| tcl_cli::CONVERTIBLE_CODES.contains(&d.finding.code.as_str()))
        .map(|d| {
            let mut obj = diag_to_json(&d, &sm);
            let conversion = tcl_cli::conversion_for(d.finding.code.as_str());
            obj.as_object_mut()
                .expect("json object")
                .insert("conversion".to_owned(), json!(conversion));
            obj
        })
        .collect();
    let suppressed = suppressed_json(&report, &sm, |code| {
        tcl_cli::CONVERTIBLE_CODES.contains(&code)
    });
    json!({
        "analysis_context_unavailable": report.analysis_context_status_data(),
        "total": patterns.len(),
        "patterns": patterns,
        "suppressed": suppressed
    })
}

pub(super) fn code_actions(args: &Value) -> Value {
    code_actions_with(args, &PolicyInputs::for_call(args, "diagnostics"))
}

/// [`code_actions`] under `inputs`: one report over the analyser's
/// findings, the compiler checks and the optimiser's rewrites, so a fix is
/// offered for a finding the document shows and for no other — and a shown
/// rewrite is an offerable action, not only a diagnostic payload.
pub(super) fn code_actions_with(args: &Value, inputs: &PolicyInputs) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let analysed = analyse_under(source, &dialect, inputs);
    code_actions_from_analysis(args, &analysed)
}

/// Render actions from the already supplied producer. Optimisation reads its
/// retained unit and complete current input rather than rebuilding a world.
pub(super) fn code_actions_from_analysis(args: &Value, analysed: &Analysed) -> Value {
    let range = LspRange {
        start_line: arg_u32(args, "start_line"),
        start_character: arg_u32(args, "start_character"),
        end_line: arg_u32(args, "end_line"),
        end_character: arg_u32(args, "end_character"),
    };
    // The MCP tool analyses one standalone source string with no workspace
    // behind it, so the analysed set — the analyser's and the compiler
    // checks' findings — is the published set; the rewrites join it under
    // the same policy, whose optimiser switch stays the layers'. Both run
    // over the analysis form, as the editor's lightbulb does, so a lone-`\r`
    // source's ranges are on the client's line model.
    let rewrites: Vec<Finding> = analysed
        .findings
        .optimisation_candidates(&analysed.analysis_text)
        .into_iter()
        .map(Finding::from)
        .collect();
    let report = analysed.actions_report(rewrites);
    let actions: Vec<Value> = tcl_lsp_core::code_actions::code_actions(
        &analysed.analysis_text,
        range,
        Some(&analysed.findings.analysis),
        &report,
    )
    .iter()
        .map(|a| {
            let mut obj = json!({ "title": a.title, "kind": a.kind.as_str() });
            let m = obj.as_object_mut().expect("json object");
            if !a.edits.is_empty() {
                let edits: Vec<Value> = a
                    .edits
                    .iter()
                    .map(|e| json!({ "uri": SOURCE_URI, "range": lsp_range_json(&e.range), "new_text": e.new_text }))
                    .collect();
                m.insert("edits".to_owned(), Value::Array(edits));
            }
            if let Some(cmd) = &a.command {
                m.insert(
                    "command".to_owned(),
                    json!({ "command": cmd.command, "args": cmd.args, "string_args": cmd.string_args }),
                );
            }
            if let Some(dg) = &a.data_group_definition {
                m.insert("data_group_definition".to_owned(), json!(dg));
            }
            if let Some(reason) = &a.disabled {
                m.insert("refused".to_owned(), json!({ "reason": reason }));
            }
            obj
        })
        .collect();
    json!({ "actions": actions })
}
