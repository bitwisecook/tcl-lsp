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

//! The whole-source transformations: `format_source`, `optimize` and `compile_wasm`.

use super::{PolicyInputs, arg_str, registry, resolve_dialect};
use serde_json::{Value, json};
use tcl_lexer::{LexerConfig, LineIndex};
use tcl_lsp_core::diagnostic_report::optimise_under_policy;

pub(super) fn format_source(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    // The tool's resolved dialect drives the formatter as one fact:
    // lexer grammar, rewrite-candidate release, and forward range —
    // otherwise every dialect would format as modern Tcl.
    let mut config = tcl_lsp_core::formatting::FormatterConfig::for_dialect(&dialect);
    if let Some(n) = args.get("indent_size").and_then(Value::as_u64) {
        config.indent_size = usize::try_from(n).unwrap_or(config.indent_size);
    }
    if let Some(n) = args.get("max_line_length").and_then(Value::as_u64) {
        config.max_line_length = usize::try_from(n).unwrap_or(config.max_line_length);
    }
    if arg_str(args, "indent_style").eq_ignore_ascii_case("tabs") {
        config.indent_style = tcl_lsp_core::formatting::IndentStyle::Tabs;
    }
    let formatted =
        tcl_lsp_core::formatting::engine::format_tcl(source, &config, &registry(&dialect));
    json!({ "formatted": formatted, "changed": formatted != source })
}

pub(super) fn optimize(args: &Value) -> Value {
    optimize_with(args, &PolicyInputs::for_call(args, "optimiser"))
}

/// [`optimize`] under `inputs`: the rewrites the policy shows are applied,
/// on every pass, so a `# noqa`, a file-wide directive or a code the profile
/// or a layer turned off keeps a rewrite off exactly as it keeps a squiggle
/// off (`docs/design/compiler/diagnostic-policy.md` § Adapters).
///
/// A non-empty `profile` argument is the profile in force over the global
/// file's `[optimiser] profile`, which applies only when the call names none,
/// and `full` when neither does: the profile is a request parameter with a
/// project default (§ Configuration). The profile in force sets the passes.
///
/// The loop reads the analysis form of the source (a lone `\r` rewritten to
/// `\n`), as `tcl opt` does, so `optimized_source` and every range are on the
/// client's line model, and `changed` says whether a rewrite applied.
pub(super) fn optimize_with(args: &Value, inputs: &PolicyInputs) -> Value {
    use tcl_compiler::optimiser::profiles::OptimisationProfile;

    let raw = arg_str(args, "source");
    let dialect = resolve_dialect(args, raw);
    let analysis_text = tcl_lexer::normalise_lone_cr(raw);
    let source: &str = &analysis_text;
    let named = arg_str(args, "profile");
    let requested = (!named.is_empty()).then(|| OptimisationProfile::parse(named));
    let dialect_profile = crate::environment::profile_for_dialect(&dialect);
    let policy = inputs
        .builder()
        .requested_profile(requested)
        .default_profile(OptimisationProfile::Full)
        .dialect(dialect_profile)
        .build();
    let profile = policy.optimiser.profile;
    let registry = registry(&dialect);
    let overlay = tcl_spectcl::bundled::packs().key;
    let context = match tcl_registry::model::resolve_environment(dialect_profile.name)
        .context_registry(&tcl_registry::model::KeyedVersions::default(), overlay)
    {
        Ok(context) => context.with_command_store(std::sync::Arc::clone(&registry)),
        Err(miss) => return json!({ "error": miss.to_string() }),
    };
    let analysis_input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        dialect_profile,
        dialect_profile,
        std::sync::Arc::new(context),
        LexerConfig::for_file_grammar(dialect_profile.grammar),
    );
    let optimised = optimise_under_policy(
        source,
        &registry,
        Some(&analysis_input),
        profile.max_iterations(),
        &policy,
    );
    let (optimised, opts, iterations) = (optimised.text, optimised.applied, optimised.iterations);

    let line_index = LineIndex::new(source);
    let pos = |offset: u32| {
        let p = line_index.position_at_utf16(offset, source);
        json!({ "line": p.line, "character": p.character.get() })
    };
    let optimizations: Vec<Value> = opts
        .iter()
        .map(|o| {
            let mut item = json!({
                "code": o.code.to_string(),
                "message": o.message,
                "range": { "start": pos(o.span.start()), "end": pos(o.span.end()) },
                "replacement": o.replacement,
            });
            let map = item.as_object_mut().expect("json object");
            if let Some(g) = o.group {
                map.insert("group".to_owned(), json!(g));
            }
            if o.hint_only {
                map.insert("hintOnly".to_owned(), json!(true));
            }
            item
        })
        .collect();

    json!({
        "optimizations": optimizations,
        "total": optimizations.len(),
        "optimized_source": optimised,
        "changed": optimised != source,
        "profile": profile.name(),
        "iterations": iterations,
        "multi_pass": profile.is_multi_pass(),
    })
}

pub(super) fn compile_wasm(args: &Value) -> Value {
    let source = arg_str(args, "source");
    let dialect = resolve_dialect(args, source);
    let registry = registry(&dialect);
    let profile = crate::environment::profile_for_dialect(&dialect);
    let unit = tcl_compiler::compilation_unit::CompilationUnit::build_for_profile(
        source, &registry, false, profile,
    );
    let mut wasm = tcl_codegen::codegen::wasm::compile_wasm(
        &unit,
        &registry,
        tcl_codegen::codegen::wasm::WasmCompileOptions::hosted(),
    );
    let codegen_plan = wasm.plan.as_str();
    let semantic_decline = wasm
        .plan
        .semantic_decline()
        .map(tcl_codegen::codegen::wasm::WasmSemanticDecline::as_str);
    let semantic_decline_detail = wasm
        .plan
        .semantic_decline()
        .map(tcl_codegen::codegen::wasm::WasmSemanticDecline::detail_kind);
    let function_count = wasm.functions.len();
    let bytes = wasm.to_bytes();
    let wat = wasm.to_wat();
    json!({
        "pipeline": "canonical",
        "codegen_plan": codegen_plan,
        "semantic_decline": semantic_decline,
        "semantic_decline_detail": semantic_decline_detail,
        "wat": wat,
        "byte_length": bytes.len(),
        "function_count": function_count,
    })
}
