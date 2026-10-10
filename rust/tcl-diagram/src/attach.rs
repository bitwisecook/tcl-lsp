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

//! Current readonly source candidates for pool, node and SNAT-pool attachment.
//! The guarded authored argv/schema and shared object-reference table select
//! each operand and kind. Literal source values remain exact candidates;
//! dynamic values stay unconstrained until independent value composition and
//! observer premises exist. These filters do not certify runtime reachability
//! or object deletion safety.

use serde_json::{Value, json};
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_dialect::DialectProfile;
use tcl_lexer::{LexerConfig, SourceMap};

/// Cap on patterns collected per object type per rule body — a runaway-input
/// backstop far above any real iRule.
const MAX_PATTERNS: usize = 128;

/// One segment of a reconstructed name: a fixed literal, or an unknown gap
/// produced by a `$var` / `[cmd]` substitution.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Seg {
    Lit(String),
    Wild,
}

/// Merge adjacent literals, drop empty literals, collapse runs of wildcards.
fn normalise(segs: Vec<Seg>) -> Vec<Seg> {
    let mut out: Vec<Seg> = Vec::new();
    for s in segs {
        match (&s, out.last_mut()) {
            (Seg::Lit(t), _) if t.is_empty() => {}
            (Seg::Lit(t), Some(Seg::Lit(prev))) => prev.push_str(t),
            (Seg::Wild, Some(Seg::Wild)) => {}
            _ => out.push(s),
        }
    }
    out
}

/// A reconstructed object-name pattern from a single dynamic attach expression.
///
/// Matching semantics are a glob where each [`Seg::Wild`] is `*` (matching zero
/// or more characters): the name must start with `prefix`, end with `suffix`,
/// and contain each `contains` fragment in order in between. An `exact` pattern
/// (the name resolved to a constant) matches only that literal; an
/// `unconstrained` pattern (no literal anchor) matches everything.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachPattern {
    /// The source text of the attach argument, for display.
    pub raw: String,
    /// Required leading literal (empty when the name starts with a substitution).
    pub prefix: String,
    /// Interior literals that must appear in order.
    pub contains: Vec<String>,
    /// Required trailing literal (empty when the name ends with a substitution).
    pub suffix: String,
    /// True when the source operand is one supported literal (`prefix` is the
    /// whole name and matching requires equality).
    pub exact: bool,
    /// True when the expression carries no literal anchor at all, so any object
    /// of the type could be the target.
    pub unconstrained: bool,
}

impl AttachPattern {
    fn from_segs(segs: Vec<Seg>, raw: String) -> Self {
        let segs = normalise(segs);
        let has_lit = segs.iter().any(|s| matches!(s, Seg::Lit(_)));
        let has_wild = segs.iter().any(|s| matches!(s, Seg::Wild));
        if !has_lit {
            return Self {
                raw,
                prefix: String::new(),
                contains: Vec::new(),
                suffix: String::new(),
                exact: false,
                unconstrained: true,
            };
        }
        if !has_wild {
            // Exact authored literal source value; runtime target contents remain separate.
            let lit: String = segs
                .into_iter()
                .map(|s| match s {
                    Seg::Lit(t) => t,
                    Seg::Wild => String::new(),
                })
                .collect();
            return Self {
                raw,
                prefix: lit,
                contains: Vec::new(),
                suffix: String::new(),
                exact: true,
                unconstrained: false,
            };
        }
        let mut prefix = String::new();
        let mut suffix = String::new();
        let mut contains: Vec<String> = Vec::new();
        let n = segs.len();
        for (i, s) in segs.into_iter().enumerate() {
            if let Seg::Lit(t) = s {
                if i == 0 {
                    prefix = t;
                } else if i == n - 1 {
                    suffix = t;
                } else {
                    contains.push(t);
                }
            }
        }
        Self {
            raw,
            prefix,
            contains,
            suffix,
            exact: false,
            unconstrained: false,
        }
    }

    /// Does `name` fall within the set of names this expression could build?
    #[must_use]
    pub fn matches(&self, name: &str) -> bool {
        if self.unconstrained {
            return true;
        }
        if self.exact {
            return name == self.prefix;
        }
        let mut hay = name;
        if !self.prefix.is_empty() {
            match hay.strip_prefix(self.prefix.as_str()) {
                Some(rest) => hay = rest,
                None => return false,
            }
        }
        if !self.suffix.is_empty() {
            match hay.strip_suffix(self.suffix.as_str()) {
                Some(rest) => hay = rest,
                None => return false,
            }
        }
        for frag in &self.contains {
            match hay.find(frag.as_str()) {
                Some(i) => hay = &hay[i + frag.len()..],
                None => return false,
            }
        }
        true
    }

    /// A human-readable glob (`web_pool`, `web_*`, `*_pool`, `a*b*c`, `*`).
    #[must_use]
    pub fn glob(&self) -> String {
        if self.unconstrained {
            return "*".to_string();
        }
        if self.exact {
            return self.prefix.clone();
        }
        let mut g = String::new();
        g.push_str(&self.prefix);
        g.push('*');
        for c in &self.contains {
            g.push_str(c);
            g.push('*');
        }
        g.push_str(&self.suffix);
        g
    }

    /// Serialise for the `PyO3` / JSON boundary.
    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "raw": self.raw,
            "prefix": self.prefix,
            "contains": self.contains,
            "suffix": self.suffix,
            "glob": self.glob(),
            "exact": self.exact,
            "unconstrained": self.unconstrained,
        })
    }
}

/// The dynamic-attachment reach of one iRule body: the name patterns it could
/// build for each attachable object type.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AttachReach {
    pub pools: Vec<AttachPattern>,
    pub nodes: Vec<AttachPattern>,
    pub snatpools: Vec<AttachPattern>,
}

impl AttachReach {
    /// Patterns for a device-model type key (`"pools"` / `"nodes"` /
    /// `"snatpools"`); empty slice for any other key.
    #[must_use]
    pub fn patterns_for(&self, type_key: &str) -> &[AttachPattern] {
        match type_key {
            "pools" => &self.pools,
            "nodes" => &self.nodes,
            "snatpools" => &self.snatpools,
            _ => &[],
        }
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pools.is_empty() && self.nodes.is_empty() && self.snatpools.is_empty()
    }

    fn bucket_mut(&mut self, type_key: &str) -> &mut Vec<AttachPattern> {
        match type_key {
            "pools" => &mut self.pools,
            "nodes" => &mut self.nodes,
            _ => &mut self.snatpools,
        }
    }

    fn push(&mut self, type_key: &str, pat: AttachPattern) {
        let bucket = self.bucket_mut(type_key);
        if bucket.len() < MAX_PATTERNS && !bucket.contains(&pat) {
            bucket.push(pat);
        }
    }

    /// Serialise for the `PyO3` / JSON boundary.
    #[must_use]
    pub fn to_json(&self) -> Value {
        let ser = |v: &[AttachPattern]| -> Value {
            Value::Array(v.iter().map(AttachPattern::to_json).collect())
        };
        json!({
            "applicability": "conditional-source",
            "obligations": ["handler-applicability", "evaluated-target", "runtime-reachability"],
            "pools": ser(&self.pools),
            "nodes": ser(&self.nodes),
            "snatpools": ser(&self.snatpools),
        })
    }
}

/// Conditional source attachments from an independently retained context.
/// Changed whole source refuses. Actual object templates supply the kind and
/// argument ordinal; general SNAT effects cannot manufacture a pool operand.
#[must_use]
pub fn attach_reach_for_source_context(
    source: &str,
    context: &tcl_irules::OriginalIrulesSourceContext,
) -> AttachReach {
    // Implementation contract: naming.consumer.original-source-attachment-candidates
    // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
    let mut reach = AttachReach::default();
    if !context.matches_source(source) {
        return reach;
    }
    for command in context.commands() {
        let Some(operands) = command
            .words()
            .with_source_schema(context.context_registry(), |schema| {
                tcl_irules::original_source_object_operands(schema, None)
            })
        else {
            continue;
        };
        for operand in operands {
            let Some(kind) = operand.attachment() else {
                continue;
            };
            let ordinal = operand.argument();
            let raw = command
                .words()
                .operands()
                .get(ordinal)
                .and_then(Option::as_ref)
                .and_then(|operand| source.get(operand.span().as_range()))
                .unwrap_or("")
                .to_owned();
            let literal = command
                .words()
                .arguments()
                .get(ordinal)
                .and_then(|word| word.literal_bytes())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
                .filter(|name| !name.contains('\0'));
            let pattern = AttachPattern::from_segs(
                literal.map_or_else(|| vec![Seg::Wild], |name| vec![Seg::Lit(name.to_owned())]),
                raw,
            );
            let bucket = match kind {
                tcl_irules::IrulesSourceAttachmentKind::Pool => "pools",
                tcl_irules::IrulesSourceAttachmentKind::Node => "nodes",
                tcl_irules::IrulesSourceAttachmentKind::SnatPool => "snatpools",
            };
            reach.push(bucket, pattern);
        }
    }
    reach
}

/// Current source-only attachment candidates under the explicit iRules ingress.
/// A pattern supplies neither executed selection nor complete runtime coverage.
#[must_use]
pub fn attach_reach(source: &str) -> AttachReach {
    let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
    tcl_irules::OriginalIrulesSourceContext::capture(source, registry)
        .map_or_else(AttachReach::default, |context| {
            attach_reach_for_source_context(source, &context)
        })
}

/// Analyse one iRule body and serialise its attach reach as JSON — the entry
/// point the `PyO3` facade exposes to the Python report layer.
#[must_use]
pub fn irule_attach_patterns(source: &str) -> Value {
    attach_reach(source).to_json()
}

/// Relative or absolute rule spellings supplied by guarded authored source
/// metadata. Each returned string is a report candidate for the caller's
/// independent configuration-object resolver, not an owning rule or call edge.
#[must_use]
pub fn proc_call_refs(source: &str) -> Vec<String> {
    let profile = DialectProfile::irules();
    let context = tcl_registry::model::ingress::context_for_profile(profile);
    let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
        profile,
        profile,
        context.clone(),
        LexerConfig::from_grammar(profile.grammar),
    );
    let unit = CompilationUnit::build_for_profile(source, context.commands(), false, profile);
    let mut analyser = tcl_compiler::analyser::Analyser::new().with_resolved_input(input);
    analyser.set_cu_override(std::sync::Arc::new(unit));
    let analysis = analyser.analyse(source, profile.name);
    proc_call_refs_for_analysis(source, &analysis, &context)
}

/// Rule-reference candidates under the caller's actual current source,
/// configuration and full hosted Registry context. Native command recipes
/// remain independent and are not manufactured from hosted source words.
#[must_use]
pub fn proc_call_refs_for_analysis(
    source: &str,
    analysis: &tcl_compiler::analyser::AnalysisResult,
    context: &tcl_registry::model::ContextRegistry,
) -> Vec<String> {
    // Implementation contract: naming.consumer.original-rule-reference-candidates
    // docs/design/analysis/name-resolution-proofs/original-rule-reference-candidates.md
    let Some((image, config)) = tcl_compiler::source_graph::current_analysis(source, analysis)
    else {
        return Vec::new();
    };
    let Some(retained) = analysis.resolved_input.as_ref() else {
        return Vec::new();
    };
    if retained.context_registry().context() != context.context()
        || retained
            .context_registry()
            .commands()
            .snapshot()
            .semantic_key()
            != context.commands().snapshot().semantic_key()
        || !analysis.has_original_vendor_source_names()
    {
        return Vec::new();
    }
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let mut occurrences = analysis.original_vendor_source_names().collect::<Vec<_>>();
    occurrences.sort_by_key(|occurrence| occurrence.site().offset);
    let mut out = Vec::new();
    for occurrence in occurrences {
        let words = occurrence.original_words();
        let (Some(first), Some(last)) = (words.first(), words.last()) else {
            continue;
        };
        if first != occurrence.name_input().original_word() {
            continue;
        }
        let start = first.span().start();
        let end = last.span().end();
        let Some(text) = source.get(start as usize..end as usize) else {
            continue;
        };
        let mut commands =
            tcl_compiler::segmenter::segment_commands_with_offset_and_config(text, start, config);
        if commands.len() != 1 {
            continue;
        }
        let command = commands.remove(0);
        let mut tokens = tcl_compiler::ir::CommandTokens::from_segmented(
            &SourceMap::new(source),
            config,
            &command,
        );
        realm.stamp_original_tokens(&mut tokens);
        let Some(selected) =
            tcl_compiler::registry_invocation::original_conditional_vendor_registry_metadata(
                context,
                &tokens,
                occurrence.name_input(),
                words,
            )
        else {
            continue;
        };
        if !selected.matches_source(&image, config)
            || !selected.matches_registry(context.commands())
        {
            continue;
        }
        let Some(argument) = selected.shape().source_rule_procedure_operand() else {
            continue;
        };
        let Some(word) = words.get(argument + 1) else {
            continue;
        };
        let Some(units) = tcl_syntax::naming::vendor_source_literal_units(
            occurrence.name_input().policy(),
            word,
            tcl_syntax::naming::VendorSourceNamePurpose::SourceName,
        ) else {
            continue;
        };
        let Ok(target) = std::str::from_utf8(units) else {
            continue;
        };
        let Some(rule) = tcl_registry::f5::RuleProcedureTarget::referenced_rule_spelling(target)
        else {
            continue;
        };
        if !out.iter().any(|prior| prior == rule) {
            out.push(rule.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn globs(v: &[AttachPattern]) -> Vec<String> {
        v.iter().map(AttachPattern::glob).collect()
    }

    #[test]
    fn original_attachment_reports_keep_dynamic_values_unconstrained() {
        // Implementation contract: naming.consumer.original-source-attachment-candidates
        // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
        for source in [
            "when HTTP_REQUEST {pool \"web_[HTTP::host]\"}",
            "when HTTP_REQUEST {set p web_pool; pool $p}",
            "when HTTP_REQUEST {set prefix svc_; pool \"${prefix}[HTTP::host]\"}",
            "when HTTP_REQUEST {set p web_pool; unknown; pool $p}",
        ] {
            let reach = attach_reach(source);
            assert_eq!(globs(&reach.pools), ["*"]);
            assert!(reach.pools[0].unconstrained);
            assert!(reach.pools[0].matches("independent_target"));
            assert!(!reach.pools[0].exact);
        }
        let literal = attach_reach("when HTTP_REQUEST {pool /Common/literal}");
        assert_eq!(globs(&literal.pools), ["/Common/literal"]);
        assert!(literal.pools[0].exact);
        assert!(!literal.pools[0].matches("/Common/sibling"));
    }

    #[test]
    fn original_attachment_reports_require_actual_kind_ordinal_and_current_source() {
        // Implementation contract: naming.consumer.original-source-attachment-candidates
        // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
        let source = "when CLIENT_ACCEPTED {pool $p; node $n; snatpool $s; snat pool $chosen; snat $address}";
        let registry = tcl_registry::model::ingress::static_context_for("f5-irules").commands();
        let context = tcl_irules::OriginalIrulesSourceContext::capture(source, registry).unwrap();
        let reach = attach_reach_for_source_context(source, &context);
        assert_eq!(reach.pools.len(), 1);
        assert_eq!(reach.nodes.len(), 1);
        assert_eq!(
            reach.snatpools.len(),
            2,
            "bare SNAT address is not a SNAT-pool operand"
        );
        assert!(
            reach
                .snatpools
                .iter()
                .any(|pattern| pattern.raw == "$chosen")
        );
        assert!(
            !reach
                .snatpools
                .iter()
                .any(|pattern| pattern.raw == "$address")
        );
        assert!(
            attach_reach_for_source_context(&format!("# changed\n{source}"), &context).is_empty()
        );
        for source in [
            "proc pool args {}; when HTTP_REQUEST {pool $target}",
            "proc when args {}; when HTTP_REQUEST {pool $target}",
        ] {
            assert!(attach_reach(source).pools.is_empty(), "{source}");
        }
        // The selected TMM context excludes rename. Its source spelling leaves
        // unknown transitions and MAY candidates, rather than a definite move.
        let source = "rename pool moved; when HTTP_REQUEST {pool $target}";
        let context = tcl_irules::OriginalIrulesSourceContext::capture(source, registry).unwrap();
        let (_, words) = context
            .source_vectors()
            .iter()
            .find(|(_, words)| words.command() == "pool")
            .expect("conditional source candidate after unavailable TMM transition");
        let tcl_compiler::registry_invocation::source_structure::OriginalRegistrySource::Vendor(
            metadata,
        ) = words.source()
        else {
            panic!("actual hosted source purpose");
        };
        assert!(metadata.authored_barriers().has_unknown_transitions());
        assert!(!metadata.obligations().is_empty());
        let reach = attach_reach_for_source_context(source, &context);
        assert_eq!(reach.pools.len(), 1);
        assert!(
            reach.pools[0].unconstrained,
            "the variable has no evaluated object identity"
        );
    }

    #[test]
    fn proc_call_refs_cross_rule() {
        let refs = proc_call_refs(
            r"when HTTP_REQUEST { call MyLib::helper $x; if {1} { call /Common/Other::doit 1 } }",
        );
        assert_eq!(refs, vec!["MyLib", "/Common/Other"]);
    }

    #[test]
    fn proc_call_refs_ignores_local_and_dedups() {
        let refs = proc_call_refs(r"when HTTP_REQUEST { call ::plain; call Lib::a; call Lib::b }");
        assert_eq!(refs, vec!["Lib"]);
    }

    #[test]
    fn json_roundtrip_shape() {
        let v = irule_attach_patterns(r#"when HTTP_REQUEST { pool "web_[HTTP::host]" }"#);
        let p = &v["pools"][0];
        assert_eq!(p["prefix"], "");
        assert_eq!(p["glob"], "*");
        assert_eq!(p["unconstrained"], true);
        assert_eq!(p["exact"], false);
    }
    #[test]
    fn original_rule_references_require_guarded_metadata_and_current_owners() {
        // Implementation contract: naming.consumer.original-rule-reference-candidates
        // docs/design/analysis/name-resolution-proofs/original-rule-reference-candidates.md
        let profile = DialectProfile::irules();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let source =
            "when HTTP_REQUEST {call Lib::one; call -debug /Common/Other::two 1; call Lib::three}";
        let unit = CompilationUnit::build_for_profile(source, context.commands(), false, profile);
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            LexerConfig::from_grammar(profile.grammar),
        );
        let mut analyser = tcl_compiler::analyser::Analyser::new().with_resolved_input(input);
        analyser.set_cu_override(std::sync::Arc::new(unit));
        let mut analysis = analyser.analyse(source, profile.name);
        assert_eq!(
            proc_call_refs_for_analysis(source, &analysis, &context),
            vec!["Lib", "/Common/Other"]
        );
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        for invocation in &mut analysis.command_invocations {
            invocation.name = "counterfactual".to_owned();
        }
        assert_eq!(
            proc_call_refs_for_analysis(source, &analysis, &context),
            vec!["Lib", "/Common/Other"]
        );
        assert!(
            proc_call_refs_for_analysis(
                &format!("# different complete image\n{source}"),
                &analysis,
                &context
            )
            .is_empty()
        );
        let mut changed = analysis.clone();
        changed.body_lexer_config.as_mut().unwrap().strict_quoting =
            !analysis.body_lexer_config.unwrap().strict_quoting;
        assert!(proc_call_refs_for_analysis(source, &changed, &context).is_empty());
        let other = tcl_registry::model::ingress::context_for_profile(
            tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
        );
        assert!(proc_call_refs_for_analysis(source, &analysis, &other).is_empty());
        assert!(
            proc_call_refs("proc call {args} {}\nwhen HTTP_REQUEST {call Lib::one}").is_empty()
        );
        assert!(proc_call_refs(r"when HTTP_REQUEST {call L\uD800::one}").is_empty());
        assert!(proc_call_refs("when HTTP_REQUEST {puts Lib::one; call $unknown}").is_empty());
    }
}
