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

use crate::interprocedural::test_body_identity::fixture_id;

use super::body_cache_eligible;

#[test]
fn isolated_body_profile_keeps_the_original_standalone_source_purpose() {
    let environment = crate::environment_ingress::resolve_environment("tcl9.0");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let profile = environment.unit_profile();
    let config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
    let source = "upvar 1 ${odd name} local";
    let body = super::lower_proc_body_isolated(source, "::", registry, config, Some(profile));
    let statement = body.statements.first().expect("original statement");
    let tokens = body
        .retained_source_tokens_for_statement(statement)
        .expect("original source tokens");
    let binding = tokens.source_binding.as_ref().expect("original binding");
    let metadata = tcl_registry::model::semantic::SemanticContext::for_profile(profile).into();
    assert_eq!(
        binding.original_source_advice_purpose(tokens, metadata, registry),
        Some(crate::command_binding::OriginalInvocationSourcePurpose::Standalone)
    );
    assert!(
        binding
            .original_invocation_source_purpose(tokens, metadata, registry)
            .is_none()
    );
    let retained = body
        .executed_source
        .as_ref()
        .expect("original isolated body source");
    assert_eq!(retained.try_text().unwrap(), source);
    assert_eq!(retained.base(), 0);
    assert!(body.native_compilation_admission.is_none());
    assert!(
        crate::registry_invocation::admitted_native_compiler_invocation(registry, None, tokens)
            .is_none()
    );
    let info = crate::cfg_builder::upvar_info::collect_upvar_targets_with_registry(
        &body,
        &["odd name".to_owned()],
        registry,
    );
    assert_eq!(
        info.param_targets.get("local").map(String::as_str),
        Some("odd name")
    );
}

#[test]
fn plain_bodies_are_eligible() {
    assert!(body_cache_eligible("set x 1"));
    assert!(body_cache_eligible("puts \"hi $name\"\nreturn $x"));
    assert!(body_cache_eligible("if {$x} { foo } else { bar }"));
    assert!(body_cache_eligible(""));
}

#[test]
fn cross_item_constructs_disqualify() {
    for body in [
        "namespace eval x { }",
        "namespace import ::ns::*",
        "interp alias {} x {} y",
        "rename set myset",
        "apply {{} { }}",
        "when HTTP_REQUEST { }",
        "oo::class create C { }",
        "itcl::class C { }",
        "proc inner {} { }",
    ] {
        assert!(!body_cache_eligible(body), "should disqualify: {body}");
    }
}

#[test]
fn tab_separated_command_still_disqualifies() {
    assert!(!body_cache_eligible("interp\talias {} x {} y"));
    assert!(!body_cache_eligible("namespace\timport ::ns::*"));
}

#[test]
fn substring_only_use_is_safe() {
    assert!(body_cache_eligible("set myproclist {a b c}"));
    assert!(body_cache_eligible("set renamed 1"));
}

#[test]
fn source_may_alias_commands_flags_interp() {
    use super::source_may_alias_commands;
    // `interp alias` establishes an alias a cached body cannot see.
    assert!(source_may_alias_commands(
        "interp alias {} = {} expr\nproc f {} { = 1 }\n"
    ));
    assert!(source_may_alias_commands("interp\talias {} = {} expr"));
    // No `interp` -> safe to cache.
    assert!(!source_may_alias_commands(
        "proc f {x} { set y $x }\nproc g {} { return 1 }\n"
    ));
    // `interp` as a substring of a bareword does not trip it.
    assert!(!source_may_alias_commands("set interpreter 1"));
}

// Proves *why* `source_may_alias_commands` must guard the file: with a
// top-level `interp alias {} = {} expr` in scope, lowering `f`'s
// body through the isolated body cache (empty alias table) resolves `=` as an
// unknown command, whereas the in-place whole-file lowering resolves it to
// `expr` — so the module IRs differ. The db therefore must not install the
// cache for such a file.
fn assert_scope_withdraws_cache(source: &str, expected_target: &str) {
    let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
    let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let cache_calls = std::cell::Cell::new(0);
    let cache = |body: &str, namespace: &str| {
        cache_calls.set(cache_calls.get() + 1);
        super::lower_proc_body_isolated(body, namespace, &registry, config, Some(profile))
    };
    let cached =
        super::lower_to_ir_with_body_cache(source, &registry, config, Some(profile), &cache);
    let fresh = super::lower_to_ir_with_config(source, &registry, config);
    assert_eq!(
        cache_calls.get(),
        0,
        "an isolated cache lacks this dispatch context"
    );
    // Separate analyses have distinct activation/source identities. Test
    // the actual withdrawal and retained dispatch instead of equating the
    // debug serialization of two different physical worlds.
    for module in [&cached, &fresh] {
        let procedure = module
            .procedures
            .get(&fixture_id(&module.procedures, "::f"))
            .expect("authored fixture body");
        assert_eq!(procedure.params, ["x"]);
        let mut retained_target = false;
        crate::ir::for_each_statement(&procedure.body, &mut |statement| {
            if let Some(tokens) = statement.tokens() {
                retained_target |= tokens
                    .source_binding
                    .iter()
                    .chain(tokens.nested_bindings.iter().map(|(_, binding)| binding))
                    .filter_map(|binding| binding.proved_execution_target())
                    .any(|target| target.command == expected_target && target.registry_backed);
            }
        });
        assert!(
            retained_target,
            "the original invocation must retain {expected_target}"
        );
    }
}

#[test]
fn alias_in_scope_withdraws_isolated_cache_proof() {
    assert_scope_withdraws_cache(
        "interp alias {} = {} expr\nproc f {x} { return [= {$x + 1}] }\n",
        "::expr",
    );
}

#[test]
fn source_may_alias_commands_flags_rename() {
    use super::source_may_alias_commands;
    // A static `rename` establishes an alias a cached body cannot see —
    // same hazard as `interp alias`, see `detect_rename`.
    assert!(source_may_alias_commands(
        "rename puts myputs\nproc f {x} { myputs $x }\n"
    ));
    assert!(source_may_alias_commands("rename\tputs myputs"));
    // `rename` as a substring of a bareword does not trip it.
    assert!(!source_may_alias_commands("set renamed 1"));
}

// Same hazard as `alias_in_scope_makes_body_cache_diverge`, for a static
// `rename`: `rename puts myputs` at the top level must make `f`'s body
// resolve `myputs` to `puts`, which the isolated (empty-alias-table) body
// cache cannot see.
#[test]
fn rename_in_scope_withdraws_isolated_cache_proof() {
    assert_scope_withdraws_cache("rename puts myputs\nproc f {x} { myputs $x }\n", "::puts");
}
