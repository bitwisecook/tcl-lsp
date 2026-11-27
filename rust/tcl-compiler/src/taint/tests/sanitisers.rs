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

//! What clears a sink: sanitisers under their actual binding, inline quoters, CRLF-deleting maps and literal `list` wrappers.

use super::*;

#[test]
fn nested_sanitiser_requires_the_actual_point_binding() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    for (source, tainted) in [
        ("set value [string length [gets stdin]]", false),
        (
            "interp alias {} measure {} string length\nset value [measure [gets stdin]]",
            false,
        ),
        (
            "rename string core_string\nproc string args {return [lindex $args end]}\nset value [string length [gets stdin]]",
            true,
        ),
        (
            "interp alias {} measure {} string length\nrename string core_string\nproc string args {return [lindex $args end]}\nset value [measure [gets stdin]]",
            true,
        ),
        (
            "rename string core_string\nproc string args {return [lindex $args end]}\nset value \"[string length [gets stdin]]\"",
            true,
        ),
    ] {
        let module = crate::lowering::lower_to_ir(source, registry);
        let (value, tokens) = module
            .top_level
            .statements
            .iter()
            .rev()
            .find_map(|statement| {
                let tokens = statement.tokens()?;
                Some((tokens.argv_texts.last()?, tokens))
            })
            .expect("retained source value carrier");
        let cfg = crate::cfg::Function::new("::top", "entry");
        let ssa = crate::ssa::SsaFunction::trivial("::top", cfg.entry, cfg.block_names().to_vec());
        let ctx = TaintCtx {
            registry,
            ssa: &ssa,
            interproc: None,
            known_procs: None,
            dialect: registry.profile(),
            source: TaintSourceContext::standalone(registry, registry.profile()),
            taint_summaries: None,
            instance_classes: None,
            source_position: Some(tokens.argv[0].start()),
            source_tokens: Some(tokens),
            word_offset: tokens.argv.last().map(|span| span.start()),
            source_word: tokens.word_exprs.last(),
        };
        assert_eq!(
            word_taint(value, &HashMap::new(), &HashMap::new(), ctx).is_tainted(),
            tainted,
            "{source}"
        );
    }
}

#[test]
fn inline_sanitiser_mitigation_requires_its_retained_implementation() {
    let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    for (body, warn) in [
        ("puts [string length $x]", false),
        (
            "rename string saved; proc string args {return [lindex $args end]}; puts [string length $x]",
            true,
        ),
        (
            "interp alias {} measure {} string length; puts [measure $x]",
            false,
        ),
        (
            "interp alias {} measure {} string length; rename string saved; proc string args {return [lindex $args end]}; puts [measure $x]",
            true,
        ),
    ] {
        let source = format!("set x [gets stdin]; {body}");
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(native)),
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            native_compilation: crate::environment_ingress::authoring_native_compilation(),
            ..Default::default()
        };
        let unit = crate::compilation_unit::CompilationUnit::build_with_context_registry(
            &source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::for_file_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            Some(&entry),
            std::sync::Arc::clone(&current),
        )
        .with_interprocedural(registry, Some(profile));
        let warnings = find_taint_warnings_for_cu(&unit, registry, Some(profile));
        assert_eq!(
            warnings
                .iter()
                .any(|warning| warning.code == DiagCode::T101),
            warn,
            "{source}: {warnings:?}"
        );
    }
}

/// An *escaped* bracket writes a literal `[`, so the quoter never runs and
/// the raw value lands in the pattern.
///
/// tclsh 8.6.18 and 9.0.4 both leave `"\[regex::quote $p]"` as the text
/// `[regex::quote <value of p>]` — the command is not invoked — so the
/// wrapper the mitigation would credit does not exist. Segmentation is the
/// lexer's for exactly this reason; a bracket count reads a call here.
#[test]
fn t103_not_cleared_by_an_escaped_bracket_that_never_calls_the_quoter() {
    use crate::compilation_unit::CompilationUnit;
    for (release, registry) in release_registries() {
        let source = "proc f {} {\n  set p [gets stdin]\n  \
                      regexp -- \"\\[regex::quote $p]\" $line\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().any(|w| w.code == DiagCode::T103),
            "{release}: an escaped bracket is not a call, so T103 must stand: {warnings:?}",
        );
    }
}

/// T103's `[regex::quote …]` mitigation reads the same whether the quote
/// wraps the value in place or through a variable.
///
/// The quoting is what makes the pattern trusted, and it happens either
/// way; only the spelling differs. Since wrapping in place is what T103's
/// quick fix writes, that spelling is the one an applied fix has to clear.
///
/// The scope is the pattern argument alone: `$p` used as the *subject*
/// too is not in a pattern position, so it cannot withdraw the proof the
/// wrapped pattern carries.
#[test]
fn t103_cleared_by_an_inline_regex_quote_in_the_pattern_slot() {
    use crate::compilation_unit::CompilationUnit;
    for (release, registry) in release_registries() {
        let cases: [(&str, bool); 5] = [
            ("regexp -- $p $line", true),
            ("set q [regex::quote $p]\n  regexp -- $q $line", false),
            ("regexp -- [regex::quote $p] $line", false),
            ("regexp -- [regex::quote $p] $p", false),
            // `string range` carries no transform, so it proves nothing.
            ("regexp -- [string range $p 0 5] $line", true),
        ];
        for (tail, expected) in cases {
            let source = format!("proc f {{}} {{\n  set p [gets stdin]\n  {tail}\n}}\n");
            let cu = CompilationUnit::build_for(&source, &registry, false)
                .with_interprocedural(&registry, None);
            let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
            let got = warnings.iter().any(|w| w.code == DiagCode::T103);
            assert_eq!(got, expected, "{release}: for {tail:?}, got {warnings:?}");
        }
    }
}

/// The T101 mitigation chain end to end: the CR/LF-stripping `string map`
/// its KCS page prescribes — and its quick fix writes — clears the
/// diagnostic, in both the wrap-a-variable and wrap-in-place spellings,
/// and no other mapping does.
///
/// tclsh 8.6.18 and 9.0.4 both give `string map {"\n" "" "\r" ""}
/// "a\nb\rc"` a length of 3 (`abc`), so the mapping really does delete
/// both characters — the element `"\n"` is list-parsed with its backslash
/// honoured, giving the newline itself as the key rather than a
/// two-character sequence. Dropping the `"\r"` pair leaves length 4 on
/// both, which is the half-pair case below.
#[test]
fn t101_cleared_only_by_a_mapping_that_deletes_crlf() {
    use crate::compilation_unit::CompilationUnit;
    for (release, registry) in release_registries() {
        let crlf = r#"{"\n" "" "\r" ""}"#;
        let cases: &[(&str, bool)] = &[
            ("puts $x", true),
            (&format!("puts [string map {crlf} $x]"), false),
            (&format!("set s [string map {crlf} $x]\n  puts $s"), false),
            ("puts [string map {a b} $x]", true),
            // Only half the pair: an LF is deleted but a CR still survives.
            (r#"puts [string map {"\n" ""} $x]"#, true),
            // The mapping is dynamic and the *subject* is the braced word, so
            // the rewriting is exactly what cannot be seen. tclsh 8.6.18 and
            // 9.0.4: `set m [list X "a\nb"]; string map $m {"\n" "" "\r" ""
            // X}` yields a string containing a newline.
            (&format!("puts [string map $x {crlf}]"), true),
            // A later `string map` puts a newline back, so the earlier proof
            // no longer describes what reaches `puts` — in either spelling.
            (
                &format!("set s [string map {crlf} $x]\n  puts [string map {{\"|\" \"\\n\"}} $s]"),
                true,
            ),
            (
                &format!(
                    "set s [string map {crlf} $x]\n  \
                     set t [string map {{\"|\" \"\\n\"}} $s]\n  puts $t"
                ),
                true,
            ),
        ];
        for &(tail, expected) in cases {
            let source = format!("proc f {{}} {{\n  set x [gets stdin]\n  {tail}\n}}\n");
            let cu = CompilationUnit::build_for(&source, &registry, false)
                .with_interprocedural(&registry, None);
            let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
            let got = warnings.iter().any(|w| w.code == DiagCode::T101);
            assert_eq!(got, expected, "{release}: for {tail:?}, got {warnings:?}");
        }
    }
}

#[test]
fn standalone_list_wrapper_uses_original_grouped_head_values_and_exact_extent() {
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    let dialect = registry
        .profile()
        .map(tcl_native_model::InvocationDialect::of_profile);
    for original in [
        "[list puts $value]",
        "[list {puts} $value]",
        "[::list \"puts\" $value]",
    ] {
        assert_eq!(
            standalone_list_wrapper_head(original, &registry, config, dialect).as_deref(),
            Some("puts")
        );
    }
    for original in [
        "[list $command $value]",
        "[list {$command} $value]",
        "[list {} $value]",
        "[list {*}{puts value}]",
        "[list puts $value; list puts $value]",
        "[list puts $value]suffix",
        "[list puts $value]; missing",
        "[list puts $value",
        "[string cat puts $value]",
    ] {
        assert!(
            standalone_list_wrapper_head(original, &registry, config, dialect).is_none(),
            "{original}"
        );
    }
    let args = vec!["[list {puts} $value]".to_owned()];
    let call = SinkCall {
        source: TaintSourceContext::standalone(&registry, registry.profile()),
        command: "eval",
        args: &args,
        registry: &registry,
        tokens: None,
        effective: None,
        source_read: None,
        argument_offset: 0,
    };
    assert!(list_wrapped_arg_command_is_literal(&call, "value"));
    let mut unavailable = call;
    unavailable.source = TaintSourceContext::for_input(&registry, None, config);
    assert!(!list_wrapped_arg_command_is_literal(&unavailable, "value"));
}
