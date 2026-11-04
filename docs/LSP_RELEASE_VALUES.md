# LSP value and diagnostic release audit

The source revisions and scope of each check are recorded below. Quiet
warnings are not treated as verification when a genuine positive control is
also silent.

| Issue | Current evidence and status |
| --- | --- |
| #940 | At `5755cd79d`, the reported conditional append and original font-size examples are quiet. A genuine int/string loop also produces no S102, so the result is inconclusive. See the replay below. |
| #2498, #2500, #2504 | At `5755cd79d`, each reported warning is quiet, but all three genuine shimmer controls below also produce no warning. The results are inconclusive. |
| #2499 | At `e19c0acaa`, the dynamic read is quiet and the dynamic write control emits W212. `lsp_external_validation::issue_2499_dynamic_variable_read`. |
| #2503 | At `e19c0acaa`, the quoted expression is quiet and a direct nested expression emits W114. `lsp_external_validation::issue_2503_nested_expr_inside_quoted_result`. |
| #2505 | At `e19c0acaa`, multiline and single-line list-built scripts are quiet and an interpolated script emits W301. `lsp_external_validation::issue_2505_multiline_list_uplevel`. |
| #2506 | At `0c7b97c9d`, the boolean comparison remains quiet and `set raw [gets stdin]; expr {$raw + 1}` emits T100 for `raw`. The exact control is committed in `lsp_external_taint_validation::issue_2506_boolean_comparison_taint`. |
| #2507 | Separate LSP report and action checks passed at `14b26b2e0`; see the optimiser validation record. |
| #2131 | `type_infer` uses the `tcl-syntax` `MathResultClass` owner; execution pending. |
| #2288 | At `e19c0acaa`, the prefixed list-built Tk callback is quiet and an interpolated Tk callback emits W105. `lsp_external_validation::issue_2288_prefixed_tk_bind_script`. |
| #2440 | Selected script-argument roles avoid scanning the containing body. The focused integration regression passed on `073e6e942`, covering proc literal and proven use, conditional and iRules bodies, and braced data. |
| #2436 | The earlier draft is superseded by the verified fix in `1eefbc317`. Two lexer tests and four production/reference tests passed against Tcl 8.4–9.1 and the verified current Jim build; see `docs/LSP_RELEASE_STATUS.md`. |
| #2387, #2437, #2438, #2451–#2454, #2427 | Awaiting reproduction against current branch. |

The initial compiler test build was stopped before completion to honour the
shared machine's compiler-build concurrency limit. The later `--lib` test
build failed on 43 existing test-compilation errors, including unavailable
sparse-checkout fixtures and private-module and ambiguous empty-array assertions
(`/tmp/lsp-values-2440.log`). The focused integration regression also passed on `073e6e942`. Full gates have not run
in this checkout.


## Shimmer reproduction at `5755cd79d`

The following standalone Cargo test uses the production analyser input,
captured source settings, and interprocedural phase. It explicitly checks that
`ShimmerContext::for_function` is available for each analysed function. The
seven checks yielded five passes and two failures: the reported examples were
quiet, while `issue_940_conditional_accumulators_with_oscillation_control` and
`shimmer_positive_control` failed because the genuine controls were quiet too.
The first-list-conversion check for `fontSizes` passed. The same run's #2506
boolean and positive taint cases passed. Source hashes were unchanged during
the run.

Tcl 8.6.18, 9.0.4 and 9.1.0 `tcl::unsupported::representation` confirms that
`set n [string length $s]; lindex $n 0` changes the same object from int to
list, and `set d [dict create k x]; llength $d` changes the same object from
dict to list. In the S102 control `set x [expr {$x+1}]; set x [string range
$x 0 end]`, each of those runtimes reports int after `expr` and string after
`string range` on two iterations. Tcl 8.4.20 through 9.1.0 and Jim 0.84 run
the reported #940 loop and return `ys={4 6}` and `pts={{3 4} {5 6}}`.

To replay, create a temporary Cargo package with dependencies on this
checkout's `rust/tcl-compiler`, `rust/tcl-registry`, `rust/tcl-dialect`, and
`rust/tcl-core-types` crates, put the code below in `src/lib.rs`, and run
`cargo test -- --nocapture --test-threads=1`. Use an isolated target directory
for the checkout. The two positive-control failures are intentional evidence
of the current gap, not expected passing tests.

```rust
use tcl_compiler::analyser::Analyser;
use tcl_core_types::DiagCode;

fn with_unit<T>(source: &str, run: impl FnOnce(&tcl_compiler::compilation_unit::CompilationUnit, &tcl_registry::CommandRegistry, &'static tcl_dialect::DialectProfile) -> T) -> T {
    use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
    use tcl_compiler::shimmer::ShimmerContext;
    let mut analyser = Analyser::new();
    let _analysis = analyser.analyse(source, "tcl8.6");
    let input = analyser.resolved_analysis_input();
    let settings = analyser.source_compilation_settings();
    let registry = input.borrowed_context_registry().commands();
    let profile = input.unit_profile();
    let options = UnitBuildOptions {registry, defer_top_level: false, config: input.lexer_config(), dialect: Some(profile), external_call_sites: None, declared_commands: None};
    let unit = CompilationUnit::build_with_analysis_input_and_source_settings(source, options, None, &input, &settings).with_interprocedural(registry, Some(profile));
    assert!(unit.analysable_functions().next().is_some());
    assert!(unit.analysable_functions().all(|f| ShimmerContext::for_function(f, registry).is_some()));
    run(&unit, registry, profile)
}

fn shimmer(source: &str) -> Vec<(DiagCode, String)> {
    with_unit(source, |unit, registry, _| {
        tcl_compiler::shimmer::find_shimmer_warnings_for_cu(unit, registry).into_iter().map(|warning| (warning.code, warning.variable)).collect()
    })
}

#[test]
fn issue_2498_list_element_arithmetic() {
    let source = "set steps {0.001 0.002 0.005 0.01 0.02 0.05 0.1 0.2 0.5 1 2 5 10 15 30 60 120 300 600 900 1800 3600 7200 10800 21600 43200 86400}\nset hi [lindex $steps end]\nset pow [expr {pow(10, floor(log10(5)))}]\nset e [expr {5 / $pow}]\nset mult [expr {$e >= 5 ? 5 : ($e >= 2 ? 2 : 1)}]\nset step [expr {$mult * $pow * $hi}]";
    let warnings = shimmer(source);
    assert!(!warnings.iter().any(|(code, var)| *code == DiagCode::S100 && var == "hi"), "{warnings:?}");
}

#[test]
fn issue_2500_signed_infinity() {
    let source = "set x0 Inf\nset x1 -Inf\nforeach child {1 2 3 4} {lassign {10 20 30 40} x y w h; set x0 [expr {min($x0, $x)}]; set x1 [expr {max($x1, $x + $w)}]}";
    with_unit(source, |unit, registry, _| {
        let shimmers = tcl_compiler::shimmer::find_shimmer_warnings_for_cu(unit, registry);
        let thunks = tcl_compiler::shimmer::find_thunking_warnings_for_cu(unit, registry);
        assert!(!shimmers.iter().any(|w| w.code == DiagCode::S100 && w.variable == "x1"), "{shimmers:?}");
        assert!(!thunks.iter().any(|w| w.variable == "x1"), "{thunks:?}");
    });
}

#[test]
fn issue_2504_list_iteration_arithmetic() {
    let source = "proc rgbToLab {{r 255} {g 255} {b 255}} { foreach c [list $r $g $b] {set cs [expr {$c / 255.0}]} }; rgbToLab 200 150 90";
    let warnings = shimmer(source);
    assert!(!warnings.iter().any(|(code, var)| *code == DiagCode::S101 && var == "c"), "{warnings:?}");
}

#[test]
fn shimmer_positive_control() {
    let sources = [
        "proc computed {} {set s abc; set n [string length $s]; lindex $n 0}; computed",
        "proc counted {} {set i 0; incr i; lindex $i 0}; counted",
        "proc f {l} {foreach x $l {set d [dict create k $x]; set n [llength $d]}}; f {1 2}",
    ];
    let found: Vec<_> = sources.iter().map(|source| (source, shimmer(source))).collect();
    eprintln!("valid shimmer positive controls: {found:?}");
    assert!(found.iter().any(|(_, warnings)| !warnings.is_empty()), "no valid shimmer positive control warned: {found:?}");
}

fn taint(source: &str) -> Vec<String> {
    with_unit(source, |unit, registry, profile| {
        tcl_compiler::taint::find_taint_warnings_for_cu(unit, registry, Some(profile)).into_iter().filter(|w| w.code == DiagCode::T100).map(|w| w.variable).collect()
    })
}

#[test]
fn issue_2506_boolean_comparison_taint() {
    let source = "set line [gets stdin]\nset trimmed [string trimleft $line]\nset is_empty [expr {$trimmed eq \"\"}]\nset is_comment [expr {!$is_empty && [string index $trimmed 0] eq \"#\"}]\nset is_code [expr {!$is_empty && !$is_comment}]";
    let warnings = taint(source);
    assert!(!warnings.iter().any(|v| v == "is_empty" || v == "is_comment"), "{warnings:?}");
    let control = taint("set n [gets stdin]\nset m [expr {$n + 1}]\nif {$m} {puts y}");
    assert!(control.iter().any(|v| v == "m"), "taint analysis failed positive control: {control:?}");
}

#[test]
fn issue_940_conditional_accumulators_with_oscillation_control() {
    use tcl_compiler::shimmer::find_thunking_warnings_for_cu;
    let source = "set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; if {$y > 2} {lappend ys $y; lappend pts $pt}}";
    let warnings = with_unit(source, |unit, registry, _| find_thunking_warnings_for_cu(unit, registry));
    assert!(!warnings.iter().any(|w| w.variable == "ys" || w.variable == "pts"), "{warnings:?}");
    let control = "proc f {n} { set x 0; while {$n} { set x [expr {$x + 1}]; set x [string range $x 0 end]; incr n -1 }; return $x }; f 3";
    let warnings = with_unit(control, |unit, registry, _| find_thunking_warnings_for_cu(unit, registry));
    assert!(warnings.iter().any(|w| w.variable == "x"), "genuine int/string control has no S102: {warnings:?}");
}

#[test]
fn issue_940_font_sizes_first_list_conversion() {
    for source in [
        "set fontSizes {10.0 12.0 16.0 24.0}\nforeach size $fontSizes { puts $size }",
        "set fontSizes [list 10.0 12.0 16.0 24.0]\nforeach size $fontSizes { puts $size }",
    ] {
        let warnings = shimmer(source);
        assert!(!warnings.iter().any(|(code, var)| *code == DiagCode::S100 && var == "fontSizes"), "{warnings:?}");
    }
}
```

A bounded source-owner probe at `5755cd79d` separately queried
`source_registry_words_at` with the retained analysis and obtained source
schemas for each #940 loop-body `set`, `lindex`, and both `lappend` sites, as
well as the positive proc's `set`, `expr`, `string range`, and `incr` sites.
Those source schemas are advisory facts; they do not prove runtime command
selection or successful execution.

The bounded probe uses the same temporary Cargo package and its retained
analysis:

```rust
use tcl_compiler::analyser::Analyser;
use tcl_compiler::registry_invocation::source_structure::source_registry_words_at;

#[test]
fn original_body_source_schemas() {
    for (source, needles) in [
        ("set ys {}; set pts {}; foreach pt {{1 2} {3 4} {5 6}} {set y [lindex $pt 1]; if {$y > 2} {lappend ys $y; lappend pts $pt}}", &["set y ", "lindex ", "lappend ys", "lappend pts"][..]),
        ("proc f {n} { set x 0; while {$n} { set x [expr {$x + 1}]; set x [string range $x 0 end]; incr n -1 }; return $x }; f 3", &["set x [expr", "expr {", "set x [string", "string range", "incr n"][..]),
    ] {
        let mut analyser = Analyser::new();
        let analysis = analyser.analyse(source, "tcl8.6");
        let input = analyser.resolved_analysis_input();
        for needle in needles {
            let offset = u32::try_from(source.find(needle).unwrap()).unwrap();
            let words = source_registry_words_at(source, &analysis, offset);
            let schema = words.as_ref().and_then(|words| words.with_source_schema(input.borrowed_context_registry(), |schema| (schema.semantics.lowering_hook, schema.authored_source_argument_roles())));
            println!("{needle:?} offset={offset} words={} schema={schema:?}", words.is_some());
        }
    }
}
```

## External report language oracles

Run each script below under Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0,
and Jim 0.84. The first script returned `+widget destroy`, `{12 34}`, `25%`,
and `hello` on all six. Its `bind` stub examines Tcl substitution and list
construction; it does not exercise Tk GUI dispatch.

```tcl
proc self {} {return widget}
proc bind {name event script} {return $script}
set frameName .f
set result2288 [bind $frameName <Destroy> +[list [self] destroy]]
set sides {left right}; set left 12; set right 34; set result2499 {}
foreach side $sides {lappend result2499 [set $side]}
set progress 0.25
set result2503 [expr {$progress eq "" ? "" : "[expr {round($progress * 100)}]%"}]
proc sink {x} {return $x}
set result2505 [uplevel #0 [list \
    sink hello \
]]
puts [list [info patchlevel] $result2288 $result2499 $result2503 $result2505]
```

The next script returned `432000.0` for #2498 and `{0 1 0}` for #2506 on
all six. #2504 returned the expected ratios, with Tcl 8.4 and Jim displaying
fewer digits. #2500 returned `{10 40}` on Tcl 8.5–9.1; Tcl 8.4 rejected
`min`, and Jim rejected that expression.

```tcl
set script2498 {
 set steps {0.001 0.002 0.005 0.01 0.02 0.05 0.1 0.2 0.5 1 2 5 10 15 30 60 120 300 600 900 1800 3600 7200 10800 21600 43200 86400}
 set hi [lindex $steps end]; set pow [expr {pow(10, floor(log10(5)))}]; set e [expr {5/$pow}]; set mult [expr {$e>=5 ? 5 : ($e>=2 ? 2 : 1)}]; expr {$mult*$pow*$hi}
}
set script2500 {
 set x0 Inf; set x1 -Inf
 foreach child {1 2 3 4} {lassign {10 20 30 40} x y w h; set x0 [expr {min($x0,$x)}]; set x1 [expr {max($x1,$x+$w)}]}
 list $x0 $x1
}
set script2504 {
 proc rgbToLab {{r 255} {g 255} {b 255}} {set out {}; foreach c [list $r $g $b] {lappend out [expr {$c / 255.0}]}; return $out}
 rgbToLab 200 150 90
}
set script2506 {
 set trimmed [string trimleft " # comment"]; set is_empty [expr {$trimmed eq ""}]; set is_comment [expr {!$is_empty && [string index $trimmed 0] eq "#"}]; set is_code [expr {!$is_empty && !$is_comment}]; list $is_empty $is_comment $is_code
}
foreach number {2498 2500 2504 2506} {
 set script [set script$number]
 set status [catch {eval $script} result]
 puts [list [info patchlevel] $number $status $result]
}
```


## Analysis-only source-owner experiment

At `942a957d0`, a local candidate routed a checked conditional Native source
schema from the current retained inventory through the existing analysis-only
structured invocation. Its frozen full-pipeline run still left both the
Tcl-confirmed S102 positive control and all three int/dict-to-list shimmer
positive controls silent. The reported #940, #2498, #2500 and #2504 cases
remained quiet, so the candidate does not establish a fix. The owner route
requires additional tracing; no candidate production changes have been
committed. Native execution and Normal-completion proof paths were not changed.

## Authored source authority follow-up

At `1da87f395`, the actual full-pipeline #940 fixture retained the
`AuthoredSimulation` purpose for the affected loop-body commands. The existing
resolved-handler, owned-body, original-declaration and inline-control selectors
all declined those sites before the authored static-word gate. The top-level
`foreach` children had no deferred procedure-body entry; the proc control's
conditional invocation did not own the declared frame. These refusals were
observed with bounded boolean traces and unchanged source hashes.

The source-registry query still found conditional schemas at the original
sites. Top-level loop children were catalogue `Conditional` facts; the proc
control's `SourceTransitions` retained `UnavailableActualLookup`,
`UnknownEarlierMutation`, and conditional procedure/body/namespace
applicability, with two to six uncertain earlier operations. These are not
closed handler or Normal-result proofs. An analysis-only experiment that
required exact original owner, source, config, registry, original words and
Native Execution purpose did not restore the genuine S102 or S100 controls and
was withdrawn. It did not alter Native execution or Normal receipts.

A shadowing control renames `lappend`, installs a no-op procedure under that
name, and runs the conditional append loop. Tcl 8.4–9.1 and Jim 0.84 all leave
`ys` empty. A source spelling or conditional catalogue role therefore cannot
alone prove a list write. No source-owner or warning-suppression change was
committed as a fix.

A follow-up source-catalogue probe retained stronger detail for those
obligations: each top-level loop-body source schema carried
`UnknownCurrentLookup`, `UnknownLookupCoordinates`, and an `UnretainedHolder`
for its original head. The proc control retained unavailable actual lookup,
unknown earlier mutations, and conditional body/namespace applicability.
The existing `declaration_flow` source-only graph is a possible home for a
future obligation-preserving diagnostic projection; its current root-script
selection and representation transfer do not yet provide this proof. No
conditional source fact was promoted to a handler, Normal result or SSA fact.

```tcl
rename lappend saved_lappend
proc lappend args {return}
set ys {}
foreach pt {{1 2} {3 4} {5 6}} {
    set y [lindex $pt 1]
    if {$y > 2} {lappend ys $y}
}
puts [list [info patchlevel] $ys]
```

The first guarded root-flow prototype was tested at `c80aa092d` with its uncommitted source changes. Its production library compiled, but `conditional_root_source_scope_uses_its_original_parent_and_input` failed: the existing child layout row was refused at `scopes.row-current`. The prototype has not established a safe root source owner and #940 remains open. The frozen build and direct test logs are `/tmp/lsp-940-root-test2-build.log` and `/tmp/lsp-940-root-test2-run.log`; the test build required five exact fixture blobs restored from the same commit because the checkout was sparse.

The follow-up source-owner trace at `7372b0864` confirmed the `lappend` child has a real `ScriptActivation` (body base 37, length 14) rather than the root entry. Both original rows retain the same frame, namespace, configuration, original words and source-origin allocation as the root. The existing read-flow refusal is appropriate; a distinct representation-only relation is being reviewed and is not yet validated. The trace and frozen build are `/tmp/lsp-940-root-test5-run.log` and `/tmp/lsp-940-root-test5-build.log`.

A genuine repeated-conversion control is `set x 0; for {set n 0} {$n < 3} {incr n} {incr x; lindex $x 0}`. On Tcl 8.6.18, 9.0.4 and 9.1.0 the native representation inspector confirms that the same object alternates between integer and list each iteration. All five Tcl versions and the current Jim build return `3`; older Tcl and Jim were not checked for internal representation. The exact script and output are `/tmp/lsp-940-incr-list-oracle.tcl` and `/tmp/lsp-940-incr-list-oracle.log`.

The subsequent related-root prototype, built against `99a98eb66` plus uncommitted changes, passed `conditional_root_source_scope_uses_its_original_parent_and_input` (1/1) and five existing read/ancestry controls. The positive verifies the conditional `lappend` accumulator effect and a selected graph handler; the controls reject changed source ancestry, frames, namespaces, configurations, inputs, registry, argv and synthetic tokens. The prototype remains uncommitted and the complete #940 warning is not yet fixed. Logs: `/tmp/lsp-940-related-test2-build.log`, `/tmp/lsp-940-related-test2-run.log`, `/tmp/lsp-940-related-ancestry.log`, and `/tmp/lsp-940-related-read-control.log`.

The complete reported #940 loop also passed source-graph coverage at `cdf71efee` plus the still-uncommitted candidate: both focused tests passed, including selection of `foreach`, the nested `lindex`, `if`, and both `lappend` handlers. This verifies only conditional source coverage, not S100/S102 behaviour. The compiler build and test logs are `/tmp/lsp-940-complete-graph-build.log` and `/tmp/lsp-940-complete-graph-run.log`; their source and fixture hashes matched before and after. Static `string length abc` must not serve as an integer-representation positive: native Tcl compilers can emit a pooled literal for that result.

The drafted registry-only conditional source contracts passed two focused tests plus the nine-test existing representation suite at `12e8175bb` with the uncommitted prototype and matching frozen hashes. They distinguish dynamic string-length results from compile-time pooled literals, preserve CellWrite value forwarding, and reject unsupported indexed-list shapes. The graph consumer and S100/S102 diagnostics are still being implemented; no issue is marked fixed. Logs: `/tmp/lsp-940-registry2-build.log`, `/tmp/lsp-940-registry2-run.log`, and `/tmp/lsp-940-registry-controls.log`.

The next local draft connects the tested conditional source graph and registry contracts to an S100-only representation projection. It preserves captured operand ordering, retained source premises and conservative barriers; it is under review and has not yet passed an integrated compiler build or full-pipeline diagnostic tests. #940 remains open.

A clean-target full-pipeline build of the integrated S100 draft succeeded at `e19409705` plus the uncommitted candidate, but its genuine dynamic string-length → list-index positive and forwarded-copy control both returned no warnings. Pure/static, nested-result, shadow/rename and trace negatives passed, but are inconclusive while the positive is silent. Bounded tracing showed the retained module and exact source/analysis owner were present, while every representation report lookup refused. The draft now uses the existing original-token stamping adapter for the recovered original command; that correction remains untested. Logs: `/tmp/lsp-940-joint-tests-build.log`, `/tmp/lsp-940-joint-positive.log`, `/tmp/lsp-940-joint-controls.log`, and `/tmp/lsp-940-ingress-run.log`.

The stamping correction was built at `09cf51556` with frozen source and restored the conditional report, but the genuine S100 positive remained silent: its root commands had no CompletePrefix transition advice. The next local draft uses the existing retained-module realm for observed root commands, retaining the actual original source-word variant and requiring agreement with the original layout. That fallback remains under review and untested; no source fix is committed. Evidence: `/tmp/lsp-940-stamped-build.log` and `/tmp/lsp-940-stamped-positive.log`.

The retained-module fallback was built against `c753d2bc6` plus the uncommitted draft with matching frozen hashes. It recovered original Registry words at each positive-control command, but S100 remained silent because the observed and source flows disagreed on effective argv; all other reported flow facets agreed. The exact head/argument/origin/prefix mismatch is being traced before any comparison is relaxed. Logs: `/tmp/lsp-940-observed-build.log` and `/tmp/lsp-940-observed-positive.log`.

At `05b682b66` plus the uncommitted prototype, the focused test compiled after the shared branch corrected an unrelated visibility error. Frozen source hashes matched. The dynamic-result S100 positive and forwarded-alias control still failed; the literal, unsupported nested-result, shadow/rename and trace controls passed (3/5 overall, with those negatives inconclusive until the positive works). A bounded trace shows the source and observed descriptions differ only at the command head: post-head arguments, origins, binding prefixes and all other flow fields agree at all four positive-control sites. No correspondence or diagnostic fix is claimed. Logs: `/tmp/lsp-940-argv2-build.log`, `/tmp/lsp-940-argv2-positive.log`, and `/tmp/lsp-940-argv2-controls.log`.

The reviewed head-correspondence candidate's combined unit/integration no-run build at `f0000ed78` plus local changes failed because four existing unit tests still referred to the origin guard's previous name. The guard's original name has been restored with its implementation unchanged; the corrected candidate has not yet been rebuilt. No runtime result is claimed for that candidate. Build log: `/tmp/lsp-940-head-proof-build.log`.

At `273b170b6` plus the local prototype, the combined compiler unit/integration no-run build succeeded with unchanged source hashes. The full production pipeline now reports the genuine dynamic `string length` → `lindex` S100, with the asserted integer-to-list types and original `$n` span. Four of five focused full-pipeline tests passed; the forwarded-copy positive still emitted no warning, so the candidate remains local. The exact head-correspondence ownership test passed, both original loop/root-custody tests passed, and five existing read/ancestry controls passed. Bounded trace shows the alias fixture has no root naming scope even at its earlier selected sites; the specific failed row guard is being investigated. Logs: `/tmp/lsp-940-head-proof2-build.log`, `/tmp/lsp-940-head-proof2-positive.log`, `/tmp/lsp-940-head-proof2-controls.log`, `/tmp/lsp-940-head-proof2-owner.log`, `/tmp/lsp-940-head-proof2-loop.log`, `/tmp/lsp-940-head-proof2-ancestry.log`, `/tmp/lsp-940-head-proof2-read.log`, and `/tmp/lsp-940-head-proof2-alias.log`. This is S100 evidence only; #940's repeated-conversion S102 remains unresolved.

The copied-value refusal was traced at `0fd47058b` plus the uncommitted candidate: all source, input, frame, purpose, native-word and namespace geometry guards pass at the earlier commands, but the later `lindex` at offset 60 lacks `original_namespace_geometry`. The previous whole-graph cache let that later refusal erase earlier observations. A per-command candidate retains the same guard and exact source-owner checks at each site; it is not yet built or verified. The later command continues to refuse rather than supplying a namespace claim. Logs: `/tmp/lsp-940-root-row-build.log` and `/tmp/lsp-940-root-row-alias.log`.

The per-command candidate passed at `c6dab4041` plus uncommitted changes with matching frozen source hashes: all five full-pipeline integration tests, including the genuine S100 positive and literal/pure refusals across every `TclVersion::ALL` profile; the copied-value warning at `$m` with the later `$n` namespace refusal; three root naming/loop-custody tests; the exact head-correspondence test; and five existing read/ancestry controls. The later namespace refusal does not prove two-consumer de-duplication. The current per-command representation scope also declines nested `ScriptActivation` rows, so loop-handler coverage does not establish S102 representation coverage. Temporary traces have since been removed; the trace-free candidate still requires its own focused rerun and lint. Logs: `/tmp/lsp-940-per-site-build.log`, `/tmp/lsp-940-per-site-integration.log`, `/tmp/lsp-940-per-site-root.log`, `/tmp/lsp-940-per-site-owner.log`, `/tmp/lsp-940-per-site-ancestry.log`, and `/tmp/lsp-940-per-site-read.log`. The native oracle `/tmp/lsp-940-dynamic-length-oracle.log` returns 3 on all five Tcl releases and Jim; Tcl 8.6, 9.0 and 9.1 additionally show the same object changing from int to list.

The first trace-free production Clippy attempt at `dd0d2a58e` plus the local candidate failed with fourteen lint errors, including three in newly integrated unrelated files and eleven in the candidate. The follow-on test build was skipped. The unrelated mechanical lint corrections were published separately; the candidate lint corrections are local and await a fresh frozen Clippy and test run. No S100 product commit or #940 fix is claimed. Log: `/tmp/lsp-940-final-clippy.log`.

The local candidate's lint corrections have been reviewed for unchanged source-owner guards and graph event ordering, and `cargo fmt --all --check` passes. The prior Clippy failure remains the latest completed lint result until the corrected source is rebuilt. No product changes are included in this status checkpoint.

Review found that the tested source-only shimmer projection was absent from `run_all_checks` and the LSP database's shared module-check path. The local candidate now wires it into that shared path with conditional no-action provenance and includes a same-compilation-unit positive/negative test; the first final Clippy attempt stopped on a missing explicit vector type in this new projection before tests ran. That mechanical type annotation has been added locally and still needs a fresh run. Log: `/tmp/lsp-940-published-path-clippy.log`. Native LSP delivery and #940 S102 remain unverified.

The trace-free source candidate at `ff1ff34bb` plus the sealed local patch passed production compiler/registry Clippy with `-D warnings`, combined compiler unit/integration build, six integration tests, three root scope/loop tests, one exact head-custody test, and five existing read/ancestry controls. The integration suite checks the public `compiler_checks::run_all_checks` path on the same unit: one conditional S100 at the original use with no code-action authority for the computed integer, zero for the pure-list control; the direct warning/literal matrix covers every supported Tcl release. Build and runtime source hashes matched. An independent same-unit admission/Normal/provider comparison is pending; no native LSP delivery or S102 loop fix is claimed. Logs: `/tmp/lsp-940-report-final-{clippy,build,integration,root,owner,ancestry,read}.log`.
