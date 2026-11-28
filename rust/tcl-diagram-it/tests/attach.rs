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

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_diagram::{
    AttachPattern, attach_reach, attach_reach_for_source_context, irule_attach_patterns,
    proc_call_refs, proc_call_refs_for_analysis,
};
use tcl_dialect::DialectProfile;
use tcl_lexer::LexerConfig;

fn globs(v: &[AttachPattern]) -> Vec<String> {
    v.iter().map(AttachPattern::glob).collect()
}

fn install_surfaces() {
    tcl_registry::install_shipped().unwrap();
}

#[test]
fn original_attachment_reports_keep_dynamic_values_unconstrained() {
    install_surfaces();
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
fn proc_call_refs_cross_rule() {
    install_surfaces();
    let refs = proc_call_refs(
        r"when HTTP_REQUEST { call MyLib::helper $x; if {1} { call /Common/Other::doit 1 } }",
    );
    assert_eq!(refs, vec!["MyLib", "/Common/Other"]);
}

#[test]
fn proc_call_refs_ignores_local_and_dedups() {
    install_surfaces();
    let refs = proc_call_refs(r"when HTTP_REQUEST { call ::plain; call Lib::a; call Lib::b }");
    assert_eq!(refs, vec!["Lib"]);
}

#[test]
fn json_roundtrip_shape() {
    install_surfaces();
    let v = irule_attach_patterns(r#"when HTTP_REQUEST { pool "web_[HTTP::host]" }"#);
    let p = &v["pools"][0];
    assert_eq!(p["prefix"], "");
    assert_eq!(p["glob"], "*");
    assert_eq!(p["unconstrained"], true);
    assert_eq!(p["exact"], false);
}

#[test]
fn original_attachment_reports_require_actual_kind_ordinal_and_current_source() {
    install_surfaces();
    // Implementation contract: naming.consumer.original-source-attachment-candidates
    // docs/design/analysis/name-resolution-proofs/original-source-attachment-candidates.md
    let source =
        "when CLIENT_ACCEPTED {pool $p; node $n; snatpool $s; snat pool $chosen; snat $address}";
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
    assert!(attach_reach_for_source_context(&format!("# changed\n{source}"), &context).is_empty());
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
    assert_ne!(metadata.obligations(), []);
    let reach = attach_reach_for_source_context(source, &context);
    assert_eq!(reach.pools.len(), 1);
    assert!(
        reach.pools[0].unconstrained,
        "the variable has no evaluated object identity"
    );
}

#[test]
fn original_rule_references_require_guarded_metadata_and_current_owners() {
    install_surfaces();
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
    analyser.set_owned_cu_override(unit);
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
    assert_eq!(
        proc_call_refs_for_analysis(
            &format!("# different complete image\n{source}"),
            &analysis,
            &context
        ),
        [] as [String; 0]
    );
    let mut changed = analysis.clone();
    changed.body_lexer_config.as_mut().unwrap().strict_quoting =
        !analysis.body_lexer_config.unwrap().strict_quoting;
    assert_eq!(
        proc_call_refs_for_analysis(source, &changed, &context),
        [] as [String; 0]
    );
    let other = tcl_registry::model::ingress::context_for_profile(
        tcl_registry::model::ingress::resolve_environment("f5-iapps").analyser_profile(),
    );
    assert_eq!(
        proc_call_refs_for_analysis(source, &analysis, &other),
        [] as [String; 0]
    );
    assert_eq!(
        proc_call_refs("proc call {args} {}\nwhen HTTP_REQUEST {call Lib::one}"),
        [] as [String; 0]
    );
    assert_eq!(
        proc_call_refs(r"when HTTP_REQUEST {call L\uD800::one}"),
        [] as [String; 0]
    );
    assert_eq!(
        proc_call_refs("when HTTP_REQUEST {puts Lib::one; call $unknown}"),
        [] as [String; 0]
    );
}
