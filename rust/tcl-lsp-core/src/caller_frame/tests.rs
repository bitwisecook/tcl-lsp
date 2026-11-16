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

use super::*;
use tcl_compiler::analyser::Analyser;

/// The idx-58 shape, minimised: a call-by-name helper, a `TclOO` class with
/// an accessor **method** of the same name as the caller-frame variable,
/// and a constructor that reads the variable it never assigns.
///
/// tclsh 9.0.4 / 8.6.14 (identical) run the real thing to completion —
/// `dataset` is a live variable created purely by the callee's `upvar`.
const IDX58: &str = "\
proc gridlayoutHasDataSetObj {dts} {
    upvar 1 $dts dataset
    set dataset MY-SHARED-DATASET
}
oo::class create chart {
    constructor {} {
        gridlayoutHasDataSetObj dataset
        set _dataset $dataset
    }
    method dataset {} { return 1 }
}
";

fn analyse(source: &str) -> AnalysisResult {
    Analyser::new().analyse(source, "tcl9.0").clone()
}

fn reg() -> &'static tcl_registry::CommandRegistry {
    tcl_registry::model::ingress::static_context_for("tcl9.0").commands()
}

fn offset_of(source: &str, needle: &str) -> u32 {
    u32::try_from(source.find(needle).expect("needle present")).expect("offset fits u32")
}

#[test]
fn original_caller_templates_keep_full_input_bom_namespace_and_source_currency() {
    // Implementation contract: naming.core.original-caller-frame-navigation
    // docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md
    let source = "\u{feff}namespace eval N {\nproc setdef {d} {upvar 1 $d dst; set dst SET}\nproc caller {ok} {\nif {$ok} {setdef shared}\nputs $shared\n}\n}\n";
    for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
        let context = tcl_registry::model::ingress::context_for_profile(profile);
        let mut config = tcl_lexer::LexerConfig::for_file_grammar(profile.grammar);
        config.leading_bom = tcl_lexer::LeadingBom::Skip;
        let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            context.clone(),
            config,
        );
        let mut analysis = Analyser::new()
            .with_resolved_input(input.clone())
            .analyse(source, dialect);
        // UI name and scope maps cannot replace the immutable original
        // declaration and body/frame receipts used by this projection.
        analysis.all_procs.clear();
        analysis.all_classes.clear();
        analysis.global_scope.children.clear();
        let read = offset_of(source, "$shared") + 1;
        let resolution =
            crate::definition::CallResolution::document_only().with_registry(context.commands());
        let bindings =
            caller_frame_bindings(&analysis, source, profile, resolution, read, "shared");
        assert_eq!(bindings.len(), 1, "{dialect}: {bindings:?}");
        assert_eq!(bindings[0].callee, "::N::setdef");
        assert_eq!(
            caller_frame_bindings(
                &analysis.clone(),
                source,
                profile,
                resolution,
                read,
                "shared"
            ),
            bindings
        );
        let mut changed = analysis.clone();
        let mut body_config = config;
        body_config.leading_bom = tcl_lexer::LeadingBom::Content;
        changed.body_lexer_config = Some(body_config);
        assert_eq!(
            caller_frame_bindings(&changed, source, profile, resolution, read, "shared"),
            [] as [crate::caller_frame::CallerFrameBinding; 0]
        );
        assert_eq!(
            caller_frame_bindings(
                &analysis,
                &format!("{source}# changed"),
                profile,
                resolution,
                read,
                "shared"
            ),
            [] as [crate::caller_frame::CallerFrameBinding; 0]
        );
        let equivalent_generation = std::sync::Arc::new(
            context.with_command_store(context.commands().snapshot().shared_registry()),
        );
        let mut foreign = analysis.clone();
        foreign.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            equivalent_generation,
            config,
        ));
        assert_eq!(
            caller_frame_bindings(&foreign, source, profile, resolution, read, "shared"),
            [] as [crate::caller_frame::CallerFrameBinding; 0]
        );
    }
}

#[test]
fn original_caller_reference_geometry_does_not_cross_a_nested_frame() {
    // Implementation contract: naming.core.original-caller-frame-navigation
    // docs/design/analysis/name-resolution-proofs/original-caller-frame-navigation.md
    let source = "proc setdef {d} {upvar 1 $d dst; set dst SET}\nproc caller {ok} {\nsetdef shared\nif {$ok} {puts $shared}\nproc unrelated {} {puts $shared}\nputs $shared\n}\n";
    let analysis = analyse(source);
    let read = u32::try_from(source.rfind("$shared").unwrap() + 1).unwrap();
    let resolution = crate::definition::CallResolution::document_only().with_registry(reg());
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let spans =
        caller_frame_reference_spans(&analysis, source, profile, resolution, read, "shared");
    let expected = [
        source.find("setdef shared").unwrap() + "setdef ".len(),
        source.find("if {$ok} {puts $shared}").unwrap() + "if {$ok} {puts $".len(),
        source.rfind("$shared").unwrap() + 1,
    ]
    .map(|start| {
        Span::new(
            u32::try_from(start).unwrap(),
            u32::try_from(start + "shared".len()).unwrap(),
        )
    });
    assert_eq!(spans, expected);
}

#[test]
fn call_site_argument_binds_the_caller_frame_variable() {
    let analysis = analyse(IDX58);
    let read = offset_of(IDX58, "$dataset") + 1;
    let bindings = caller_frame_bindings(
        &analysis,
        IDX58,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        crate::definition::CallResolution::document_only().with_registry(reg()),
        read,
        "dataset",
    );
    assert_eq!(bindings.len(), 1, "one binding call site: {bindings:?}");
    assert_eq!(bindings[0].param.as_deref(), Some("dts"));
    assert!(!bindings[0].read_only);
    assert_eq!(
        &IDX58[bindings[0].arg_span.as_range()],
        "dataset",
        "the binding span must cover the call-site word"
    );
}

#[test]
fn conditional_caller_template_maps_original_arguments_after_alias_prefixes() {
    let source = "proc setter {ignored target} {upvar 1 $target value; set value SET}\n\
                      interp alias {} writer {} setter prefix\n\
                      proc caller {} {writer dataset; puts $dataset}\n";
    let analysis = analyse(source);
    let read = offset_of(source, "$dataset") + 1;
    let bindings = caller_frame_bindings(
        &analysis,
        source,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        crate::definition::CallResolution::document_only().with_registry(reg()),
        read,
        "dataset",
    );
    assert_eq!(bindings.len(), 1, "{bindings:?}");
    assert_eq!(bindings[0].param.as_deref(), Some("target"));
    assert_eq!(&source[bindings[0].arg_span.as_range()], "dataset");
    assert!(!bindings[0].read_only);
}

#[test]
fn caller_templates_decode_original_literal_arguments() {
    for word in ["{data set}", "\"data set\"", r"data\ set"] {
        let source = format!(
            "proc setter {{target}} {{upvar 1 $target value; set value SET}}\nproc caller {{}} {{setter {word}; puts ${{data set}}}}\n"
        );
        let analysis = analyse(&source);
        let bindings = caller_frame_bindings(
            &analysis,
            &source,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            offset_of(&source, "${data set}") + 2,
            "data set",
        );
        assert_eq!(bindings.len(), 1, "{word}: {bindings:?}");
        assert_eq!(bindings[0].param.as_deref(), Some("target"));
    }
}

#[test]
fn caller_templates_do_not_borrow_final_or_unrelated_namespace_definitions() {
    for source in [
        "namespace eval foreign {proc setter {target} {upvar 1 $target value; set value SET}}\n\
             proc caller {} {setter dataset; puts $dataset}\n",
        "proc setter {target} {upvar 1 $target value; set value SET}\n\
             rename setter saved; proc setter args {return ordinary}\n\
             proc caller {} {setter dataset; puts $dataset}\n",
    ] {
        let analysis = analyse(source);
        assert!(
            caller_frame_bindings(
                &analysis,
                source,
                tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
                crate::definition::CallResolution::document_only().with_registry(reg()),
                offset_of(source, "$dataset") + 1,
                "dataset",
            )
            .is_empty(),
            "{source}"
        );
    }
}

/// TN — a call whose callee does *not* alias the argument binds nothing,
/// so navigation keeps abstaining instead of inventing a variable.
#[test]
fn a_plain_value_argument_binds_nothing() {
    let src = "proc plain {v} { return $v }\nproc caller {} { plain thing\n puts $thing }\n";
    let analysis = analyse(src);
    let read = offset_of(src, "$thing") + 1;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "thing"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// TN — a substituted call-site word names a variable this analysis
/// cannot identify, so it contributes no binding.
#[test]
fn a_substituted_call_site_word_binds_nothing() {
    let src = "proc setdef {d} { upvar 1 $d dst; set dst 1 }\n\
                   proc caller {n} { setdef $n\n puts $thing }\n";
    let analysis = analyse(src);
    let read = offset_of(src, "$thing") + 1;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "thing"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// The search never leaves the frame: a binding call in a *different*
/// proc creates the variable in that proc's frame, not in this one.
#[test]
fn a_binding_in_another_frame_is_not_visible_here() {
    let src = "proc setdef {d} { upvar 1 $d dst; set dst 1 }\n\
                   proc other {} { setdef shared }\n\
                   proc caller {} { puts $shared }\n";
    let analysis = analyse(src);
    let read = offset_of(src, "$shared") + 1;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "shared"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// Both halves of the idiom are one variable: the bare call-site word and
/// every `$name` read it feeds.
#[test]
fn reference_spans_cover_the_call_site_word_and_the_reads() {
    let analysis = analyse(IDX58);
    let read = offset_of(IDX58, "$dataset") + 1;
    let spans = caller_frame_reference_spans(
        &analysis,
        IDX58,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        crate::definition::CallResolution::document_only().with_registry(reg()),
        read,
        "dataset",
    );
    assert_eq!(spans.len(), 2, "call-site word + one read: {spans:?}");
    for span in &spans {
        assert_eq!(&IDX58[span.as_range()], "dataset");
    }
    assert!(spans[0].start() < offset_of(IDX58, "$dataset"));
}

/// **Finding 1.** The `VarRead` / `VarWrite`
/// trait says a parameter's value is used as a variable *name* through an
/// `upvar`; it does not say which frame the alias lands in.  Only
/// `upvar 1` lands in the caller's, so every other level must bind
/// nothing here.
///
/// tclsh 9.0.4 / 8.6.14, byte-identical — with `proc q {n} {upvar L $n a;
/// set a 1}` called as `q y` from inside a proc, `info exists y` in the
/// caller afterwards:
///
/// | `L` | caller's `y` exists |
/// |---|---|
/// | `1` | **1** |
/// | `0` | 0 — the alias is the *callee's own* local |
/// | `#0` | 0 — the alias is the global `::y` |
/// | `2` | 0 — the alias is the caller's *caller* |
#[test]
fn only_a_caller_frame_upvar_level_binds_here() {
    for level in ["0", "#0", "2", "$lvl", "bogus"] {
        let src = format!(
            "proc setdef {{d}} {{ upvar {level} $d dst; set dst 1 }}\n\
                 proc caller {{}} {{ setdef shared\n puts $shared }}\n"
        );
        let analysis = analyse(&src);
        let read = offset_of(&src, "$shared") + 1;
        let bindings = caller_frame_bindings(
            &analysis,
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "shared",
        );
        assert!(
            bindings.is_empty(),
            "`upvar {level}` does not bind the caller's frame, so navigation \
                 must abstain; got {bindings:?}"
        );
    }
    // TN — the plain `upvar 1 $n a` idiom, and its default-level
    // spelling, keep working.
    for level in ["1 ", ""] {
        let src = format!(
            "proc setdef {{d}} {{ upvar {level}$d dst; set dst 1 }}\n\
                 proc caller {{}} {{ setdef shared\n puts $shared }}\n"
        );
        let analysis = analyse(&src);
        let read = offset_of(&src, "$shared") + 1;
        let bindings = caller_frame_bindings(
            &analysis,
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "shared",
        );
        assert_eq!(
            bindings.len(),
            1,
            "`upvar {level}$d dst` binds the caller's frame: {bindings:?}"
        );
    }
}

/// `namespace upvar ns $token local` aliases a **namespace** variable, so
/// a call site passing the token creates nothing in the calling frame
/// either — even though the parameter carries the same `VarRead` trait.
#[test]
fn namespace_upvar_binds_nothing_in_the_calling_frame() {
    let src = "namespace eval ::cfg { variable shared 1 }\n\
                   proc getcfg {t} { namespace upvar ::cfg $t local; return $local }\n\
                   proc caller {} { getcfg shared\n puts $shared }\n";
    let analysis = analyse(src);
    let read = offset_of(src, "puts $shared") + 6;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "shared"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// **Finding 3.** A call in a same-frame script — `if` / `while` /
/// `catch` / `foreach` body, a `switch` clause — runs in the very frame
/// the construct is written in, so the variable it creates belongs here.
///
/// tclsh 9.0.4 / 8.6.14, identical: with `proc setdef {d} {upvar 1 $d
/// dst; set dst SET}`, each of `if {$ok} {setdef x; return $x}`,
/// `while … {setdef y…}`, `catch {setdef z}`, `switch a {a {setdef s}}`
/// and `foreach i {1} {setdef f}` returns `SET` from the enclosing proc.
#[test]
fn a_binding_inside_a_same_frame_body_is_visible_here() {
    const SETDEF: &str = "proc setdef {d} { upvar 1 $d dst; set dst SET }\n";
    for body in [
        "if {$ok} { setdef shared }\n puts $shared",
        "while {$ok} { setdef shared }\n puts $shared",
        "catch { setdef shared }\n puts $shared",
        "foreach i {1} { setdef shared }\n puts $shared",
        "switch a { a { setdef shared } }\n puts $shared",
        "if {$ok} { if {$ok} { setdef shared } }\n puts $shared",
        "set r [expr {1}]\n if {$ok} { setdef shared }\n puts $shared",
    ] {
        let src = format!("{SETDEF}proc caller {{ok}} {{\n {body}\n}}\n");
        let analysis = analyse(&src);
        let read = offset_of(&src, "puts $shared") + 6;
        let bindings = caller_frame_bindings(
            &analysis,
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "shared",
        );
        assert_eq!(
            bindings.len(),
            1,
            "a same-frame nested body still binds this frame — {body:?}: {bindings:?}"
        );
        assert_eq!(&src[bindings[0].arg_span.as_range()], "shared");
    }
}

/// TN for finding 3 — the fresh-frame boundary. A `setdef` written inside
/// a nested `proc` body, an `apply` lambda, or a `namespace eval` body
/// creates *that* frame's variable, never the enclosing one.
///
/// tclsh 9.0.4 / 8.6.14, identical: all three report `info exists` = 0 in
/// the enclosing proc afterwards.
#[test]
fn a_binding_inside_a_fresh_frame_body_does_not_leak_out() {
    const SETDEF: &str = "proc setdef {d} { upvar 1 $d dst; set dst SET }\n";
    for body in [
        "proc inner {} { setdef shared }\n inner\n puts $shared",
        "apply {{} { setdef shared }}\n puts $shared",
        "namespace eval ::zz { setdef shared }\n puts $shared",
        "uplevel 1 { setdef shared }\n puts $shared",
    ] {
        let src = format!("{SETDEF}proc caller {{}} {{\n {body}\n}}\n");
        let analysis = analyse(&src);
        let read = offset_of(&src, "puts $shared") + 6;
        let bindings = caller_frame_bindings(
            &analysis,
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "shared",
        );
        assert!(
            bindings.is_empty(),
            "a fresh-frame body's binding must not leak out — {body:?}: {bindings:?}"
        );
    }
}

/// **Finding 2.** Tcl's backslash prevents substitution, and the rule is
/// the backslash *run's parity*, not "is the previous byte a backslash".
///
/// tclsh 9.0.4 / 8.6.14, identical, with `name` upvar-set to `SET`:
/// `"\$name"` → `$name` (no substitution), `"\\$name"` → `\SET`
/// (**substitution**), `"\\\$name"` → `\$name`, `"\\\\$name"` → `\\SET`,
/// `"\${name}"` → `${name}`, `"\\${name}"` → `\SET`.
#[test]
fn an_escaped_dollar_is_not_a_read_but_an_escaped_backslash_leaves_one() {
    // Rust `\\` is one source backslash.
    for (fragment, extra_reads) in [
        ("puts \"\\$shared\"", 0),   // \$shared   — escaped
        ("puts \"\\\\$shared\"", 1), // \\$shared  — real substitution
        ("puts \"\\${shared}\"", 0), // \${shared} — escaped, braced form
        ("puts \"\\\\${shared}\"", 1),
        ("puts \"\\\\\\$shared\"", 0), // \\\$shared — escaped
        ("puts \"\\\\\\\\$shared\"", 1),
    ] {
        let src = format!(
            "proc setdef {{d}} {{ upvar 1 $d dst; set dst 1 }}\n\
                 proc caller {{}} {{\n setdef shared\n {fragment}\n}}\n"
        );
        let analysis = analyse(&src);
        let call = offset_of(&src, "setdef shared") + 7;
        let spans = caller_frame_reference_spans(
            &analysis,
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            call,
            "shared",
        );
        assert_eq!(
            spans.len(),
            1 + extra_reads,
            "call-site word + {extra_reads} read for {fragment:?}: {spans:?}"
        );
    }
}

/// The literal-target shape,
/// proc form: the callee spells the caller-frame name in its **own**
/// body (`upvar name name`), so no call-site word carries it.
///
/// tclsh 9.0.4: `proc NameProcess {arguments object} {upvar name name;
/// set name W1}` then `proc build {} {NameProcess x obj; return $name}`
/// — `build` returns `W1`.
const IDX22: &str = "\
proc NameProcess {arguments object} {
    upvar name name
    set name W1
}
proc build {} {
    NameProcess x obj
    set out $name
}
";

#[test]
fn a_literal_upvar_target_binds_at_the_call_head() {
    let analysis = analyse(IDX22);
    let read = offset_of(IDX22, "$name") + 1;
    let bindings = caller_frame_bindings(
        &analysis,
        IDX22,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        crate::definition::CallResolution::document_only().with_registry(reg()),
        read,
        "name",
    );
    assert_eq!(bindings.len(), 1, "one binding call site: {bindings:?}");
    assert_eq!(bindings[0].param, None, "no call-site word carries it");
    assert!(!bindings[0].read_only, "the alias is written through");
    assert_eq!(
        &IDX22[bindings[0].arg_span.as_range()],
        "NameProcess",
        "the binding span is the call-head word"
    );
}

/// TN — the literal fact is level-gated exactly like the param fact:
/// only `upvar 1` (or the omitted level) binds this frame.
#[test]
fn a_literal_target_at_a_non_caller_level_binds_nothing() {
    for level in ["0", "#0", "2", "$lvl"] {
        let src = format!(
            "proc np {{}} {{ upvar {level} name name; set name 1 }}\n\
                 proc build {{}} {{ np\n puts $name }}\n"
        );
        let analysis = analyse(&src);
        let read = offset_of(&src, "$name") + 1;
        let bindings = caller_frame_bindings(
            &analysis,
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "name",
        );
        assert!(
            bindings.is_empty(),
            "`upvar {level}` does not bind the caller's frame: {bindings:?}"
        );
    }
}

// `my <method>` dispatch, through a mixin.

/// The `SpiceGenTcl` shape: the callee is a
/// method of a **mixin**, reached by `my NameProcess …` from a
/// constructor that never assigns `name` itself.
///
/// tclsh 9.0.4 / 8.6.16, identical: `Widget new {-base 1}` prints
/// `name=::oo::Obj24 params=-base 1` — the mixin's `upvar name name`
/// creates the constructor's `name`.
const MIXIN_SRC: &str = "\
oo::class create Utility {
    method NameProcess {arguments object} {
        upvar name name
        set name $object
    }
}
oo::class create Widget {
    mixin Utility
    constructor {arguments} {
        my NameProcess $arguments [self object]
        puts \"name=$name\"
    }
}
";

#[test]
fn a_mixin_method_reached_by_my_dispatch_binds_its_literal_target() {
    // naming.tcloo.original-declared-self-method-caller-template
    // docs/design/analysis/name-resolution-proofs/tcloo-original-declared-self-method-caller-template.md
    let analysis = analyse(MIXIN_SRC);
    let read = offset_of(MIXIN_SRC, "$name\"") + 1;
    if std::env::var_os("TCL_LSP_TRACE_CALLER_FRAME").is_some() {
        let config = analysis.body_lexer_config.unwrap();
        let realm = analysis.retained_command_realm().unwrap();
        let registry = analysis.resolved_registry().unwrap();
        let structure =
            crate::source_structure::SourceStructure::capture(MIXIN_SRC, Some(&analysis), config)
                .unwrap();
        eprintln!(
            "CALLER_METHOD_SOURCE heads={:?}",
            structure
                .commands
                .iter()
                .map(|command| (command.span.start(), command.name()))
                .collect::<Vec<_>>()
        );
        for command in &structure.commands {
            if command.name() != "my" {
                continue;
            }
            let mut tokens = tcl_compiler::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(MIXIN_SRC),
                config,
                command,
            );
            realm.stamp_original_tokens(&mut tokens);
            eprintln!(
                "CALLER_METHOD_BINDING offset={} binding={} info={:?} template={}",
                command.span.start(),
                tokens.source_binding.is_some(),
                tokens.source_binding.as_ref().map(|binding| (
                    &binding.variable_frame,
                    binding.variable_context.dynamic_bindings,
                    binding.unknown,
                    binding
                        .targets
                        .iter()
                        .map(|target| (&target.command, target.registry_backed, target.kind))
                        .collect::<Vec<_>>(),
                    binding
                        .receiver_self_method_entry(registry)
                        .map(|(_, entry, _)| (entry.name(), entry.declaration().offset)),
                )),
                realm
                    .caller_frame_invocation_template_at(&tokens, read, registry)
                    .is_some()
            );
        }
    }
    let bindings = caller_frame_bindings(
        &analysis,
        MIXIN_SRC,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        crate::definition::CallResolution::document_only().with_registry(reg()),
        read,
        "name",
    );
    assert_eq!(bindings.len(), 1, "{bindings:?}");
    assert!(
        bindings[0].callee.contains("NameProcess"),
        "the card must name the resolved method: {bindings:?}"
    );
    assert!(
        !bindings[0].read_only,
        "the method writes through the alias"
    );
    // The binding span is the dispatch head — the point where the
    // variable comes to exist in this frame.
    assert_eq!(&MIXIN_SRC[bindings[0].arg_span.as_range()], "my");
}

/// TP control — the abstention is per-name: a name the resolved method
/// never binds gets no binding, so the read keeps abstaining.
#[test]
fn a_mixin_method_binds_only_the_name_its_upvar_spells() {
    let analysis = analyse(MIXIN_SRC);
    let read = offset_of(MIXIN_SRC, "$name\"") + 1;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            MIXIN_SRC,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "other"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// TN — a method with no caller-frame `upvar` binds nothing, exactly
/// like the plain-proc control.  tclsh 9.0.4: the constructor's `$name`
/// really does raise `can't read "name"`.
#[test]
fn a_mixin_method_without_an_upvar_binds_nothing() {
    let src = "\
oo::class create Utility {
    method NameProcess {arguments object} { set name $object }
}
oo::class create Widget {
    mixin Utility
    constructor {arguments} { my NameProcess $arguments [self object]
        puts \"name=$name\" }
}
";
    let analysis = analyse(src);
    let read = offset_of(src, "$name\"") + 1;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "name"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// TN — `next` names no callee statically (it dispatches to whatever
/// follows *this* implementation in the MRO), so it stays abstaining.
///
/// And abstaining is the *right* answer here, not merely the safe one:
/// tclsh 9.0.4 and 8.6.16 both raise `can't read "name": no such
/// variable` for the source below.  `next`'s frame does not nest inside
/// the method that issued it, so `Base::init`'s `upvar 1` reaches
/// **past** `Derived::init` — claiming a binding here would have been a
/// wrong answer, not just an over-eager one.
#[test]
fn a_next_dispatch_binds_nothing() {
    let src = "\
oo::class create Base {
    method init {} { upvar name name; set name B }
}
oo::class create Derived {
    superclass Base
    method init {} { next
        puts \"name=$name\" }
}
";
    let analysis = analyse(src);
    let read = offset_of(src, "$name\"") + 1;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "name"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// TN — a `::`-qualified target names a fixed global/namespace cell,
/// not a caller-frame variable; the analyser's `otherVar` link owns it.
#[test]
fn a_qualified_literal_target_is_not_a_caller_frame_binding() {
    let src = "proc np {} { upvar 1 ::tk::FocusGrab(x) data; set data 1 }\n\
                   proc build {} { np\n puts $FocusGrab }\n";
    let analysis = analyse(src);
    let read = offset_of(src, "$FocusGrab") + 1;
    assert_eq!(
        caller_frame_bindings(
            &analysis,
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            crate::definition::CallResolution::document_only().with_registry(reg()),
            read,
            "FocusGrab"
        ),
        [] as [crate::caller_frame::CallerFrameBinding; 0]
    );
}

/// A callee that only *reads* through the literal alias references the
/// variable without creating it (tclsh 9.0.4: `proc peekname {} {upvar
/// name name; return [info exists name]}` creates nothing).
#[test]
fn a_read_only_literal_target_is_a_reference_not_a_creation() {
    let src = "proc peekname {} { upvar name name; return [string length $name] }\n\
                   proc build {} { set name N0\n peekname }\n";
    let analysis = analyse(src);
    let call = offset_of(src, "peekname }");
    let bindings = caller_frame_bindings(
        &analysis,
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        crate::definition::CallResolution::document_only().with_registry(reg()),
        call,
        "name",
    );
    assert_eq!(bindings.len(), 1, "{bindings:?}");
    assert!(bindings[0].read_only, "{bindings:?}");
}

/// The parity helper itself, over the pinned table.
#[test]
fn dollar_escape_parity_matches_c_tcl() {
    for (text, escaped) in [
        ("$n", false),
        ("\\$n", true),
        ("\\\\$n", false),
        ("\\\\\\$n", true),
        ("\\\\\\\\$n", false),
    ] {
        let at = text.find('$').expect("a dollar");
        assert_eq!(dollar_is_escaped(text.as_bytes(), at), escaped, "{text:?}");
    }
}

/// A `$name`-shaped run inside a comment is not a read, so it must not
/// enter the reference set.
#[test]
fn a_commented_read_is_not_a_reference() {
    let src = "proc setdef {d} { upvar 1 $d dst; set dst 1 }\n\
                   proc caller {} {\n setdef shared\n # mentions $shared but inertly\n }\n";
    let analysis = analyse(src);
    let call = offset_of(src, "setdef shared") + 7;
    let spans = caller_frame_reference_spans(
        &analysis,
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        crate::definition::CallResolution::document_only().with_registry(reg()),
        call,
        "shared",
    );
    assert_eq!(spans.len(), 1, "only the call-site word: {spans:?}");
}
