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

//! Option-shape factory proc specialisation.
//!
//! Drives the tcltest `Option` pattern:
//!
//! ```tcl
//! proc Configure {name default description} {
//!     proc $name {{value {}}} [subst -nocommands {
//!         if {[string length $value] > 0} {
//!             set Option($name) $value
//!         }
//!         return $Option($name)
//!     }]
//! }
//!
//! Configure verbose 0 "verbose flag"
//! Configure skip {} "patterns to skip"
//! ```
//!
//! Each call to `Configure` with literal args creates a dedicated
//! helper proc (`verbose`, `skip`, …). Lowering passes lower the
//! *factory itself* well — the inner `proc $name` substitutes when
//! its name and body template are const-known. But the call sites
//! at top level still fire the factory through interpreted dispatch
//! on every invocation, which is what this pass fixes.

use std::collections::HashMap;

use tcl_lexer::TokenType;
use tcl_registry::CommandRegistry;

use crate::ir::{Module, Procedure, Script, Statement};
use crate::lowering::Lowerer;
use crate::subst_nocommands::subst_nocommands;

/// The extracted specialisation recipe for an Option-shape factory.
///
/// Populated by [`detect_factory_shape`]. Consumed by the call-site
/// rewriter in [`specialise_factories`] to synthesise per-call
/// helper procs.
#[derive(Debug, Clone)]
pub struct FactoryShape {
    /// The factory proc's qualified name (e.g. `::Configure`).
    pub qualified_name: String,
    /// All parameters of the factory, in declaration order. Used by
    /// the rewriter to build the `param -> literal-arg` map from a
    /// call-site's positional args.
    pub params: Vec<String>,
    /// Name of the factory parameter whose value becomes the
    /// synthesised child proc's command name.
    pub name_param: String,
    /// Literal params-spec of the child proc (second argument to the
    /// inner `proc`).
    pub child_params: String,
    /// Template body for the inner `proc`. Fed through
    /// [`subst_nocommands`] with a `param -> literal-arg` map per
    /// call site.
    pub child_body_template: String,
}

/// Default per-factory specialisation cap.
///
/// An upper bound on how many call sites of a single factory we
/// will specialise.
pub const DEFAULT_FACTORY_CAP: usize = 64;

/// Walk every procedure in *module* and rewrite call sites of any
/// detected Option-shape factory to synthesise a per-call child
/// proc. Mutates the module in place.
pub fn specialise_factories(module: &mut Module, registry: &CommandRegistry) {
    specialise_factories_with_cap(module, registry, DEFAULT_FACTORY_CAP);
}

/// Same as [`specialise_factories`] but with an explicit per-factory
/// specialisation cap.
pub fn specialise_factories_with_cap(module: &mut Module, registry: &CommandRegistry, cap: usize) {
    let mut factories: HashMap<String, FactoryShape> = HashMap::new();
    for (qname, proc) in &module.procedures {
        if module.redefined_procedures.contains(qname) {
            continue;
        }
        if let Some(shape) = detect_factory_shape(proc) {
            factories.insert(qname.clone(), shape);
        }
    }
    if factories.is_empty() {
        return;
    }

    let mut counts: HashMap<String, usize> = factories.keys().map(|k| (k.clone(), 0)).collect();

    let mut top = std::mem::take(&mut module.top_level);
    let synthesised_top = rewrite_script(&mut top, &factories, registry, "::", &mut counts, cap);
    module.top_level = top;
    for (name, params, body) in synthesised_top {
        register_synthesised(module, &name, &params, body);
    }

    let proc_qnames: Vec<String> = module.procedures.keys().cloned().collect();
    for qname in proc_qnames {
        let caller_ns = namespace_of(&qname);
        let proc = module.procedures.get_mut(&qname).expect("proc key present");
        let mut body = std::mem::take(&mut proc.body);
        let synthesised_proc = rewrite_script(
            &mut body,
            &factories,
            registry,
            &caller_ns,
            &mut counts,
            cap,
        );
        proc.body = body;
        for (n, p, b) in synthesised_proc {
            register_synthesised(module, &n, &p, b);
        }
    }
}

fn register_synthesised(module: &mut Module, name: &str, params: &str, body: Script) {
    let qualified = if name.starts_with("::") {
        name.to_string()
    } else {
        format!("::{name}")
    };
    let proc = Procedure {
        name: name.to_string(),
        qualified_name: qualified.clone(),
        params: parse_simple_params(params),
        span: tcl_lexer::Span::new(0, 0),
        body,
        params_raw: params.to_string(),
        body_source: None,
        namespace_scoped: false,
        base_priority: 500,
    };
    module.procedures.insert(qualified, proc);
}

fn parse_simple_params(text: &str) -> Vec<String> {
    text.split_whitespace().map(str::to_owned).collect()
}

fn namespace_of(qname: &str) -> String {
    if let Some(idx) = qname.rfind("::") {
        if idx == 0 {
            return "::".to_string();
        }
        return qname[..idx].to_string();
    }
    "::".to_string()
}

fn resolve_target<'a>(
    command: &str,
    caller_ns: &str,
    factories: &'a HashMap<String, FactoryShape>,
) -> Option<&'a String> {
    if command.starts_with("::") {
        return factories.get_key_value(command).map(|(k, _)| k);
    }
    if caller_ns != "::" {
        let candidate = format!("{caller_ns}::{command}");
        if let Some((k, _)) = factories.get_key_value(&candidate) {
            return Some(k);
        }
    }
    let root = format!("::{command}");
    factories.get_key_value(&root).map(|(k, _)| k)
}

/// Recognise the Option-shape factory pattern in *proc*.
///
/// Returns `Some(FactoryShape)` when the proc body is exactly one
/// `Statement::Barrier { reason: "dynamic proc name", command:
/// "proc", … }` whose tokens match the gate, or `None` otherwise.
#[must_use]
pub fn detect_factory_shape(proc: &Procedure) -> Option<FactoryShape> {
    let stmts = &proc.body.statements;
    if stmts.len() != 1 {
        return None;
    }
    let last = &stmts[0];
    let Statement::Barrier {
        reason,
        command,
        tokens,
        ..
    } = last
    else {
        return None;
    };
    if reason != "dynamic proc name" || command != "proc" {
        return None;
    }
    let tk = tokens.as_ref()?;
    if tk.argv.len() != 4 {
        return None;
    }
    if !tk.single_token_word.iter().take(4).all(|&b| b) {
        return None;
    }
    let argv_texts = &tk.argv_texts;
    if argv_texts.len() != 4 {
        return None;
    }
    // We need the original Token::kind to gate name/body — but
    // CommandTokens.argv stores Spans only. The lowering preserves
    // the kind information for the proc-name word via the original
    // ``$var`` text starting with ``$``. For the body we check the
    // text shape: a CMD token's text starts with ``[`` and ends
    // with ``]`` (the segmenter's content_offset stripping isn't
    // applied to argv_texts, so we see the original brackets).
    let name_text = &argv_texts[1];
    let params_text = &argv_texts[2];
    let body_text = &argv_texts[3];

    // Name must be a single ``$var`` / ``${var}`` reference matching
    // one of the factory's params.
    let name_param = if let Some(rest) = name_text.strip_prefix("${") {
        rest.strip_suffix('}')?
    } else {
        name_text.strip_prefix('$')?
    };
    if name_param.contains('$')
        || name_param.contains('[')
        || name_param.contains('(')
        || name_param.contains("::")
        || !name_param
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return None;
    }
    if !proc.params.iter().any(|p| p == name_param) {
        return None;
    }

    // Params spec must be a literal — accept brace-delimited
    // (starts/ends with `{}`) or a bareword without ``$`` / ``[``.
    let child_params = if params_text.starts_with('{') && params_text.ends_with('}') {
        params_text[1..params_text.len() - 1].to_string()
    } else if !params_text.contains('$') && !params_text.contains('[') {
        params_text.clone()
    } else {
        return None;
    };

    // Body is a CMD token: text begins with ``[`` and ends with
    // ``]``. Re-parse the inner text to extract the subst -nocommands
    // template.
    if !body_text.starts_with('[') || !body_text.ends_with(']') {
        return None;
    }
    let inner = &body_text[1..body_text.len() - 1];
    let template = extract_subst_nocommands_template(inner)?;

    Some(FactoryShape {
        qualified_name: proc.qualified_name.clone(),
        params: proc.params.clone(),
        name_param: name_param.to_string(),
        child_params,
        child_body_template: template,
    })
}

/// Extract the brace-string template from a `subst -nocommands
/// {template}` (or `subst -nocommands -nobackslashes {template}`)
/// command-substitution body. Returns `None` for any non-matching
/// shape. Used by [`detect_factory_shape`].
fn extract_subst_nocommands_template(inner: &str) -> Option<String> {
    let segments = crate::segmenter::segment_commands(inner);
    if segments.len() != 1 {
        return None;
    }
    let cmd = &segments[0];
    if cmd.texts.is_empty() || cmd.texts[0] != "subst" {
        return None;
    }
    let argv = cmd.arg_tokens();
    let texts = cmd.args();
    let single = &cmd.single_token_word;
    let mut saw_nocommands = false;
    let mut template: Option<String> = None;
    for (i, tok) in argv.iter().enumerate() {
        let text = &texts[i];
        if text == "-nocommands" {
            saw_nocommands = true;
            continue;
        }
        if text == "-nobackslashes" || text == "-novariables" {
            return None;
        }
        if text.starts_with('-') {
            return None;
        }
        if !single.get(i + 1).copied().unwrap_or(false) {
            return None;
        }
        if tok.kind != TokenType::Str {
            return None;
        }
        if template.is_some() {
            return None;
        }
        template = Some(text.clone());
    }
    if !saw_nocommands {
        return None;
    }
    template
}

/// Walk *script*, rewriting matching call sites and collecting the
/// synthesised child procs.
fn rewrite_script(
    script: &mut Script,
    factories: &HashMap<String, FactoryShape>,
    registry: &CommandRegistry,
    namespace: &str,
    counts: &mut HashMap<String, usize>,
    cap: usize,
) -> Vec<(String, String, Script)> {
    let mut synthesised: Vec<(String, String, Script)> = Vec::new();
    for stmt in &mut script.statements {
        // Recurse into Block bodies (other structured statements
        // are out of scope per main's rewriter).
        if let Statement::Block { body, .. } = stmt {
            let inner = rewrite_script(body, factories, registry, namespace, counts, cap);
            synthesised.extend(inner);
            continue;
        }
        if let Some(replacement) =
            try_specialise_call(stmt, factories, registry, namespace, counts, cap)
        {
            let (new_stmt, name, params, body) = replacement;
            synthesised.push((name, params, body));
            *stmt = new_stmt;
        }
    }
    synthesised
}

fn try_specialise_call(
    stmt: &Statement,
    factories: &HashMap<String, FactoryShape>,
    registry: &CommandRegistry,
    namespace: &str,
    counts: &mut HashMap<String, usize>,
    cap: usize,
) -> Option<(Statement, String, String, Script)> {
    let Statement::Call {
        command,
        args,
        span,
        tokens,
        ..
    } = stmt
    else {
        return None;
    };
    let target = resolve_target(command, namespace, factories)?;
    let target = target.clone();
    let shape = factories.get(&target)?;
    if args.len() != shape.params.len() {
        return None;
    }
    // per-factory cap.
    let count = counts.entry(target.clone()).or_insert(0);
    if *count >= cap {
        return None;
    }
    // Each arg must be a literal token (single STR or single ESC
    // without substitutions).
    let tk = tokens.as_ref()?;
    if tk.argv_texts.len() != args.len() + 1 {
        return None;
    }
    let mut bindings: HashMap<String, String> = HashMap::new();
    for (i, param) in shape.params.iter().enumerate() {
        let single = tk.single_token_word.get(i + 1).copied().unwrap_or(false);
        if !single {
            return None;
        }
        if tk
            .expand_word
            .as_ref()
            .is_some_and(|ew| ew.get(i + 1).copied().unwrap_or(false))
        {
            return None;
        }
        let arg_text = &args[i];
        // Inspect the original argv_texts to determine literal-ness.
        // STR literals are passed as their unbraced content; ESC
        // bareword literals must not contain `$` / `[`.
        if arg_text.contains('$') || arg_text.contains('[') {
            return None;
        }
        bindings.insert(param.clone(), arg_text.clone());
    }
    let materialised = subst_nocommands(&shape.child_body_template, &bindings)?;
    // The child proc's name comes from the bound name_param.
    let child_name = bindings.get(&shape.name_param)?.clone();
    if child_name.is_empty() || child_name.contains(' ') {
        return None;
    }
    // Lower the materialised body as a fresh script.
    let mut lowerer = Lowerer::new(registry);
    let body = lowerer.lower_into_script(&materialised, "::");
    *count += 1;
    // Replace the call site with a no-op Block (the synthesised
    // proc registration happens out-of-band via the caller).
    let replacement = Statement::Block {
        span: *span,
        body: Script::default(),
        namespace: namespace.to_string(),
        tokens: tokens.clone(),
        error_context: None,
    };
    Some((replacement, child_name, shape.child_params.clone(), body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn detects_canonical_factory_shape() {
        let m = lower_to_ir(
            "proc Configure {name default description} {\n  proc $name {x} [subst -nocommands {return $default}]\n}",
            &reg(),
        );
        let proc = m.procedures.get("::Configure").expect("registered");
        let shape = detect_factory_shape(proc).expect("expected factory");
        assert_eq!(shape.qualified_name, "::Configure");
        assert_eq!(shape.name_param, "name");
        assert_eq!(shape.child_params, "x");
        assert!(shape.child_body_template.contains("return $default"));
    }

    #[test]
    fn rejects_factory_with_multiple_statements() {
        // Two statements in body — current detector only matches the
        // single-statement shape.
        let m = lower_to_ir(
            "proc Configure {name default} {\n  set foo 1\n  proc $name {x} [subst -nocommands {return $default}]\n}",
            &reg(),
        );
        let proc = m.procedures.get("::Configure").expect("registered");
        assert!(detect_factory_shape(proc).is_none());
    }

    #[test]
    fn rejects_factory_with_non_param_name() {
        // Inner ``\$other`` doesn't match any of the factory's
        // parameters — refuse.
        let m = lower_to_ir(
            "proc Configure {name default} {\n  proc $other {x} [subst -nocommands {return $default}]\n}",
            &reg(),
        );
        let proc = m.procedures.get("::Configure").expect("registered");
        assert!(detect_factory_shape(proc).is_none());
    }

    #[test]
    fn specialiser_synthesises_per_call_proc() {
        let mut m = lower_to_ir(
            "proc Configure {name default description} {\n  proc $name {x} [subst -nocommands {return $default}]\n}\nConfigure verbose 0 \"verbose flag\"",
            &reg(),
        );
        specialise_factories(&mut m, &reg());
        assert!(
            m.procedures.contains_key("::verbose"),
            "expected ::verbose synthesised, got {:?}",
            m.procedures.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn specialiser_skips_calls_with_dynamic_args() {
        let mut m = lower_to_ir(
            "proc Configure {name default description} {\n  proc $name {x} [subst -nocommands {return $default}]\n}\nConfigure $dyn 0 desc",
            &reg(),
        );
        specialise_factories(&mut m, &reg());
        // ``\$dyn`` arg made literal-resolution fail; no synthesis.
        assert!(!m.procedures.contains_key("::verbose"));
    }

    #[test]
    fn specialiser_respects_per_factory_cap() {
        let mut m = lower_to_ir(
            "proc F {name default} {\n  proc $name {x} [subst -nocommands {return $default}]\n}\nF a 1\nF b 2\nF c 3",
            &reg(),
        );
        specialise_factories_with_cap(&mut m, &reg(), 2);
        assert!(m.procedures.contains_key("::a"));
        assert!(m.procedures.contains_key("::b"));
        assert!(
            !m.procedures.contains_key("::c"),
            "third call past cap should not synthesise"
        );
    }
}
