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

//! T100, T101 and T103 to T106 sinks: evaluation, channels, sockets, interpreters, double encoding, `open`, `apply` and `subst`.

use super::*;

#[test]
fn original_cross_interp_script_reads_keep_the_tainted_command_word() {
    let context = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = context.commands();
    let profile = registry.profile().unwrap();
    for (source, expected) in [
        (
            "set raw [gets stdin]\ninterp eval $child [list $raw]\n",
            true,
        ),
        (
            "set raw [gets stdin]\ninterp eval $child [list puts $raw]\n",
            false,
        ),
        (
            "set raw [gets stdin]\nset child [interp create]\ninterp eval $child [list $raw]\n",
            true,
        ),
        (
            "set child [interp create]\nset raw [gets stdin]\ninterp eval $child [list $raw]\n",
            true,
        ),
        (
            "set child [interp create]\nset raw [gets stdin]\ninterp eval $child [list puts $raw]\n",
            false,
        ),
        (
            "set child [gets stdin]\ninterp eval $child {puts SAFE}\n",
            false,
        ),
        (
            "set child [interp create]\nset raw [gets stdin]\ninterp eval $child [list puts $raw] $raw\n",
            true,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source, registry, false, profile,
        );
        let warnings = find_taint_warnings_for_cu(&unit, registry, Some(profile));
        let cross_interp: Vec<_> = warnings
            .iter()
            .filter(|warning| warning.code == DiagCode::T105)
            .collect();
        assert_eq!(
            !cross_interp.is_empty(),
            expected,
            "{source:?}: {warnings:?}"
        );
        assert!(
            cross_interp.iter().all(|warning| warning.variable == "raw"),
            "the interpreter path must not impersonate script data: {warnings:?}"
        );
    }
}

fn source_regexp_warnings(
    unit: &crate::compilation_unit::CompilationUnit,
    registry: &CommandRegistry,
    source: TaintSourceContext<'_>,
) -> Vec<TaintWarning> {
    let statement = unit.ir_module.top_level.statements.last().unwrap();
    let tokens = statement.tokens().unwrap();
    let uses = HashMap::new();
    let taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
    let scan = TaintScan {
        uses: &uses,
        taints: &taints,
        ssa: &unit.top_level.ssa,
        quoted_uses: None,
        source_read: None,
        source,
    };
    let mut warnings = Vec::new();
    emit_regexp_pattern_warnings(
        &scan,
        statement.span(),
        registry,
        &mut warnings,
        Some(tokens),
        source,
    );
    warnings
}

#[test]
fn actual_regexp_warning_refuses_missing_and_foreign_source_metadata() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    // A retained literal terminator establishes source pattern topology;
    // the original variable read remains unknown, with no observed value.
    let current =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let registry = current.commands();
    let (_owner, unit) = retained_warning_unit("regexp -- $payload haystack", &current);
    let source =
        TaintSourceContext::for_module_function(registry, &unit.ir_module, &unit.top_level);
    assert!(
        source_regexp_warnings(&unit, registry, source)
            .iter()
            .any(|warning| warning.code == DiagCode::T103 && warning.variable == "payload")
    );
    let missing = TaintSourceContext::for_input(registry, None, source.config);
    assert_eq!(
        source_regexp_warnings(&unit, registry, missing),
        [] as [crate::taint::TaintWarning; 0]
    );
    let input = unit.top_level.source_metadata_input().unwrap();
    let foreign = crate::analyser::ResolvedAnalysisInput::new(
        input.analyser_profile(),
        input.unit_profile(),
        tcl_registry::model::ingress::resolve_environment("tcl9.1").default_context_registry(),
        input.lexer_config(),
    );
    let foreign = TaintSourceContext::for_input(registry, Some(&foreign), source.config);
    assert_eq!(
        source_regexp_warnings(&unit, registry, foreign),
        [] as [crate::taint::TaintWarning; 0]
    );
    let mut stale = source.config;
    stale.strict_quoting = !stale.strict_quoting;
    let stale = TaintSourceContext::for_input(registry, Some(input), stale);
    assert_eq!(
        source_regexp_warnings(&unit, registry, stale),
        [] as [crate::taint::TaintWarning; 0]
    );
    let standalone = TaintSourceContext::standalone(registry, registry.profile());
    assert_ne!(
        source_regexp_warnings(&unit, registry, standalone),
        [] as [crate::taint::TaintWarning; 0]
    );
}

#[test]
fn find_taint_warnings_eval_with_tainted_var() {
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let registry = CommandRegistry::build_default();

    // Statements: set x [gets stdin]; eval $x
    let assign = Statement::AssignValue {
        span: Span::new(0, 12),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let eval_call = Statement::Call {
        span: Span::new(13, 20),
        command: "eval".into(),
        canonical_command: None,
        args: vec!["$x".into()],
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        tokens: None,
        foreach_groups: None,
    };

    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(assign.clone());
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(eval_call.clone());
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });

    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let x = ssa.intern_var("x");
    let ssa_assign = ssa_stmt(assign, HashMap::new(), [(x, 1u32)].into_iter().collect());
    let ssa_eval = ssa_stmt(eval_call, [(x, 1u32)].into_iter().collect(), HashMap::new());

    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_assign, ssa_eval],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );

    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    let warnings = find_taint_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        None,
        &HashSet::new(),
    );

    assert!(
        warnings
            .iter()
            .any(|w| w.code == DiagCode::T100 && w.variable == "x"),
        "expected T100 for tainted $x passed to eval, got {warnings:?}"
    );
}

#[test]
fn t104_ssrf_for_tainted_socket_address() {
    // `socket $x 80` with tainted `$x` → SSRF (T104).
    let sink = call_stmt("socket", &["$x", "80"]);
    let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
    assert!(
        w.iter()
            .any(|w| w.code == DiagCode::T104 && w.variable == "x"),
        "expected T104 for tainted socket address; got {w:?}"
    );
}

#[test]
fn t105_cross_interp_for_tainted_interp_eval() {
    // `interp eval $child $x` with tainted `$x` → cross-interp (T105).
    let sink = call_stmt("interp", &["eval", "$child", "$x"]);
    let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
    let t105 = w
        .iter()
        .find(|w| w.code == DiagCode::T105 && w.variable == "x");
    assert!(t105.is_some(), "expected T105 for interp eval; got {w:?}");
    assert_eq!(t105.unwrap().sink_command, "interp eval");
}

#[test]
fn t105_cross_interp_for_tainted_console_eval() {
    // `console eval $x` with tainted `$x` → cross-interp (T105): the
    // script runs in the separate console interpreter, the same
    // category as `interp eval`.
    let sink = call_stmt("console", &["eval", "$x"]);
    let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
    let t105 = w
        .iter()
        .find(|w| w.code == DiagCode::T105 && w.variable == "x");
    assert!(t105.is_some(), "expected T105 for console eval; got {w:?}");
    assert_eq!(t105.unwrap().sink_command, "console eval");
}

#[test]
fn t105_cross_interp_for_tainted_consoleinterp_eval_and_record() {
    // `consoleinterp eval $x` / `consoleinterp record $x` with tainted
    // `$x` → cross-interp (T105): both cross back into the interpreter
    // the console is attached to.
    for sub in ["eval", "record"] {
        let sink = call_stmt("consoleinterp", &[sub, "$x"]);
        let w = warnings_for_tainted_sink(sink, &[("x", 1)]);
        let t105 = w
            .iter()
            .find(|w| w.code == DiagCode::T105 && w.variable == "x");
        assert!(
            t105.is_some(),
            "expected T105 for consoleinterp {sub}; got {w:?}"
        );
        assert_eq!(t105.unwrap().sink_command, format!("consoleinterp {sub}"));
    }
}

#[test]
fn t106_double_encode_through_uri_encode() {
    // `set x [URI::encode $tainted]` stamps URL_ENCODED on x; passing
    // x back through `URI::encode` double-encodes → T106.
    use crate::cfg::{Function, Terminator};
    use crate::ssa::{SsaBlock, SsaFunction};
    use tcl_lexer::Span;

    let mut registry = CommandRegistry::build_default();
    registry.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));

    // set x [gets stdin]      (taint source)
    // set y [URI::encode $x]  (x tainted → y URL_ENCODED)
    // URI::encode $y          (y already URL_ENCODED → T106)
    let s0 = Statement::AssignValue {
        span: Span::new(0, 12),
        name: "x".into(),
        name_braced: false,
        value: "[gets stdin]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let s1 = Statement::AssignValue {
        span: Span::new(13, 35),
        name: "y".into(),
        name_braced: false,
        value: "[URI::encode $x]".into(),
        value_needs_backsubst: false,
        tokens: None,
    };
    let s2 = call_stmt("URI::encode", &["$y"]);

    let mut cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    {
        let b = cfg.blocks.get_mut(&entry).unwrap();
        b.statements.push(s0.clone());
        b.statements.push(s1.clone());
        b.statements.push(s2.clone());
        b.terminator = Some(Terminator::Return {
            expr_base: None,
            tokens: None,
            value: None,
            value_word: None,
            span: None,
            expr: None,
            braced: false,
        });
    }
    let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let x = ssa.intern_var("x");
    let y = ssa.intern_var("y");
    let ssa_s0 = ssa_stmt(s0, HashMap::new(), [(x, 1u32)].into_iter().collect());
    let ssa_s1 = ssa_stmt(
        s1,
        [(x, 1u32)].into_iter().collect(),
        [(y, 1u32)].into_iter().collect(),
    );
    let ssa_s2 = ssa_stmt(s2, [(y, 1u32)].into_iter().collect(), HashMap::new());
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: vec![ssa_s0, ssa_s1, ssa_s2],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    let sccp = simple_sccp(&[entry]);
    let taints = plain_taints(&cfg, &ssa, &sccp, &registry);
    // The transform colour must have propagated to y.
    assert!(
        taints
            .get(&(y, 1))
            .is_some_and(|t| t.colours.intersects(TaintColour::URL_ENCODED)),
        "y should carry URL_ENCODED after URI::encode; got {:?}",
        taints.get(&(y, 1))
    );
    let warnings = find_taint_warnings(
        &cfg,
        &ssa,
        &taints,
        &sccp.executable_blocks,
        &registry,
        Some(tcl_dialect::DialectProfile::irules()),
        &HashSet::new(),
    );
    let t106 = warnings
        .iter()
        .find(|w| w.code == DiagCode::T106 && w.variable == "y");
    assert!(
        t106.is_some(),
        "expected T106 double-encode; got {warnings:?}"
    );
    assert!(
        t106.unwrap().message.contains("already URL-encoded"),
        "{:?}",
        t106.unwrap().message
    );
}

/// TP: `puts $tainted` still raises T101 (not T100) end-to-end, now via
/// the registry-driven `taint_output_sink` dispatch in `classify_sink`
/// rather than a hardcoded `command == "puts"` match.
#[test]
fn t101_fires_for_puts_via_registry_driven_dispatch() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set x [gets stdin]\nputs $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T101),
        "expected T101 for tainted puts, got {warnings:?}",
    );
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "puts must not also raise T100, got {warnings:?}",
    );
}

/// TP: `open $tainted` raises T100. A `fileName` argument whose first
/// character is `|` runs the rest of the word as a command pipeline —
/// the same command-injection hazard `exec` carries — so tainted data
/// reaching `open`'s first argument is a code-execution sink exactly
/// like `exec`, not merely a file-I/O one.
#[test]
fn t100_fires_for_open_tainted_filename() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert!(
        registry
            .get("open")
            .expect("open in registry")
            .traits
            .contains(tcl_registry::Traits::TAINT_SINK),
        "open must carry TAINT_SINK in the registry",
    );
    let source = "set x [gets stdin]\nopen $x r\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for tainted open fileName, got {warnings:?}",
    );
}

/// FP fix / TN: `open /tmp/out $mode` — a *literal* safe fileName with a
/// tainted access-mode argument must not raise T100. Only arg 0 selects a
/// `|`-pipeline; a tainted access mode or octal permissions can never
/// turn a literal path into a command, so `open`'s `taint_code_sink_args
/// = Some(&[0])` gates the sink to the fileName slot alone.
#[test]
fn t100_silent_for_open_tainted_access_mode() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set mode [gets stdin]\nopen /tmp/out $mode\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "a tainted open access mode with a literal fileName must not raise T100, got {warnings:?}",
    );
}

/// TP: `apply $lambda ...` with a tainted lambda (arg 0) raises T100 — the
/// lambda body is executed verbatim, so tainted code in the `func` slot
/// is a genuine code-injection sink.
#[test]
fn t100_fires_for_apply_tainted_lambda() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set lambda [gets stdin]\napply $lambda 1 2\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for a tainted apply lambda (arg 0), got {warnings:?}",
    );
}

/// FP fix / TN: `apply {{x} {return $x}} $tainted` — a tainted value
/// passed as an ordinary lambda *argument* (bound to a formal parameter,
/// never re-evaluated as script) must not raise T100. `apply`'s
/// `taint_code_sink_args = Some(&[0])` gates the sink to the `func` slot.
#[test]
fn t100_silent_for_apply_tainted_argument() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set x [gets stdin]\napply {{a} {return $a}} $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "a tainted apply argument (not the lambda) must not raise T100, got {warnings:?}",
    );
}

/// FP fix / TN: `subst -nocommands $tainted` — the exact mitigation
/// `subst`'s own hover snippet recommends — must not raise T100, since
/// `-nocommands` disables the only hazard (command substitution) the
/// code names.
#[test]
fn t100_silent_for_subst_nocommands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set x [gets stdin]\nsubst -nocommands $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "subst -nocommands must not raise T100, got {warnings:?}",
    );
}

/// TP regression: plain `subst $tainted` (no flags) keeps raising T100 —
/// the new sink gate must not suppress the common, genuinely dangerous
/// default-options call.
#[test]
fn t100_fires_for_subst_without_nocommands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set x [gets stdin]\nsubst $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for bare `subst $x`, got {warnings:?}",
    );
}

/// TN: the Tcl 9.1 positive option form without `-commands` is equally
/// safe — `-variables` alone never runs command substitution.
#[test]
fn t100_silent_for_subst_positive_form_without_commands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default();
    let source = "set x [gets stdin]\nsubst -variables $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().all(|w| w.code != DiagCode::T100),
        "subst -variables (Tcl 9.1 positive form) must not raise T100, got {warnings:?}",
    );
}

/// TP: the Tcl 9.1 positive form *with* `-commands` present stays a sink.
#[test]
fn t100_fires_for_subst_positive_form_with_commands() {
    use crate::compilation_unit::CompilationUnit;
    let registry = CommandRegistry::build_default().project_for_profile(
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let source = "set x [gets stdin]\nsubst -variables -commands $x\n";
    let cu =
        CompilationUnit::build_for(source, &registry, false).with_interprocedural(&registry, None);
    let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
    assert!(
        warnings.iter().any(|w| w.code == DiagCode::T100),
        "expected T100 for `subst -variables -commands $x`, got {warnings:?}",
    );
}

/// Precision: `eval "prefix" $x "suffix"` underlines only the `$x`
/// argument word (2 characters), not the whole `eval "prefix" $x
/// "suffix"` statement — `tight_arg_span` narrows the highlight to the
/// one argument word that actually carries the tainted value, using the
/// real per-word source spans a compiled `Statement::Call` carries.
///
/// `eval` concatenates its words into one script, so a `;` in `$x` runs a
/// command of its own: with `x` = `a; puts INJECTED; p`, `eval "p" $x "s"`
/// prints `INJECTED` on tclsh 8.4.20 / 8.5.19 / 8.6.18 / 9.0.4 / 9.1.0,
/// while the single `[list puts $x]` operand prints the text once.
#[test]
fn t100_span_is_the_tainted_argument_word_not_the_whole_statement() {
    use crate::compilation_unit::CompilationUnit;
    for (release, registry) in release_registries() {
        let source = "proc f {} {\n  set x [gets stdin]\n  eval \"prefix\" $x \"suffix\"\n}\n";
        let cu = CompilationUnit::build_for(source, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        let hit = warnings
            .iter()
            .find(|w| w.code == DiagCode::T100)
            .unwrap_or_else(|| panic!("{release}: expected T100, got {warnings:?}"));
        let text = &source[hit.span.start() as usize..hit.span.end() as usize];
        assert_eq!(
            text, "$x",
            "{release}: expected the tight `$x` argument span, got {text:?} from {hit:?}",
        );
        let quoted = "proc f {} {\n  set x [gets stdin]\n  eval [list puts $x]\n}\n";
        let cu = CompilationUnit::build_for(quoted, &registry, false)
            .with_interprocedural(&registry, None);
        let warnings = find_taint_warnings_for_cu(&cu, &registry, None);
        assert!(
            warnings.iter().all(|w| w.code != DiagCode::T100),
            "{release}: {warnings:?}",
        );
    }
}
