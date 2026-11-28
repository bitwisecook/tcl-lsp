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

mod support;

use serde_json::Value;
use tcl_explorer::render::{render_all, render_view, render_view_mapped};
use tcl_explorer::{run_pipeline, serialise_result};

fn data(src: &str) -> Value {
    serialise_result(&run_pipeline(src, "tcl8.6"))
}

/// The `sccp` view prints each cell update's route and the answer it
/// gave at the fixed point — the lines `tcl explore --show sccp --text`
/// prints — and a decline names its reason. `tk` declares no release, so
/// `incr` over `010` is ambiguous there; `f5-irules` evaluates under its
/// declared 8.4 base (ruling 8) and folds it to 9, as tclsh 8.4 does.
#[test]
fn sccp_text_prints_the_route_of_each_cell_update() {
    support::install_surfaces();
    let sccp = |src: &str, dialect: &str| {
        render_all(
            &serialise_result(&run_pipeline(src, dialect)),
            &["sccp".to_owned()],
            false,
        )
    };
    let text = sccp("proc p {} {set n 1; incr n; incr n 2; return $n}", "tcl8.6");
    assert!(text.contains("n#3 = const(4)"), "{text}");
    assert_eq!(
        text.matches("route incr: direct cell-increment (registry)")
            .count(),
        2,
        "{text}"
    );
    assert_eq!(text.matches("· answer: evaluated").count(), 2, "{text}");

    let text = sccp("proc p {} {set z 010; incr z}", "tk");
    assert!(
        text.contains("· answer: declined: release-ambiguous: numeral-grammar"),
        "{text}"
    );
    let text = sccp("proc p {} {set z 010; incr z}", "f5-irules");
    assert!(text.contains("z#2 = const(9)"), "{text}");

    let text = sccp("set r [llength {a b}]", "tcl8.6");
    assert!(
        text.contains("route llength: direct list-length (registry)"),
        "{text}"
    );
}

/// The `sccp` view prints each executable branch edge's refinement: the
/// nested equality program refines `x` to `a` on the outer true edge,
/// which decides the inner test, so the inner true edge and its
/// refinement to `b` never show; a numeric `==` refines the type and the
/// point, never the value; and the true edge of `[info exists x]` after
/// `unset x`, which the solver never takes, shows none either.
#[test]
fn sccp_text_prints_each_edge_refinement() {
    support::install_surfaces();
    let sccp = |src: &str| {
        render_all(
            &serialise_result(&run_pipeline(src, "tcl8.6")),
            &["sccp".to_owned()],
            false,
        )
    };
    let text = sccp("proc p {x} {if {$x eq \"a\"} {if {$x eq \"b\"} {puts never}}}");
    assert!(text.contains("refinement x = 'a'"), "{text}");
    assert!(text.contains("· edge: entry_1 → if_then_3"), "{text}");
    assert!(text.contains("branch if_then_3: False"), "{text}");
    assert!(!text.contains("refinement x = 'b'"), "{text}");
    let text = sccp("proc p {x} {if {$x == 1} {puts $x}}");
    assert!(text.contains("refinement x = type numeric"), "{text}");
    assert!(text.contains("refinement x = range [1, 1]"), "{text}");
    assert!(!text.contains("refinement x = '1'"), "{text}");
    let text = sccp("proc p {} {set x 1; unset x; if {[info exists x]} {puts yes}}");
    assert!(text.contains("refinement x = unbound"), "{text}");
    assert!(
        !text.contains("refinement x = bound"),
        "the untaken edge's refinement holds nowhere: {text}"
    );
}

/// The `sccp` text names each loop the solver ran to its exit — its
/// passes, how it left, the block it leaves to and each value it
/// published — the line `tcl explore --show sccp --text` prints; a loop
/// whose bound the analysis does not know is not run, and names none.
#[test]
fn sccp_text_prints_each_enumerated_loop() {
    support::install_surfaces();
    let sccp = |src: &str| {
        render_all(
            &serialise_result(&run_pipeline(src, "tcl8.6")),
            &["sccp".to_owned()],
            false,
        )
    };
    let text = sccp("for {set i 0} {$i < 5} {incr i} {}; if {$i == 5} {puts five}");
    assert!(
        text.contains("enumerated loop: 5 iterations, false condition"),
        "{text}"
    );
    assert!(text.contains("· exit block: for_end_"), "{text}");
    assert!(text.contains("· i: const(5)"), "{text}");
    let text = sccp("proc p {n} {for {set i 0} {$i < $n} {incr i} {}}");
    assert!(!text.contains("enumerated loop"), "{text}");
}

const SEALED_ADD: &str =
    "proc add {b c} {return [expr {$b + $c}]}\nset d 2\nset e 4\nputs [add $d $e]\n";

/// The native addition's part of the `aot` text: the last node of the plan.
fn native_add_lines(text: &str) -> &str {
    let start = text
        .find("    └── native i64 add")
        .expect("the native addition's node");
    text[start..].trim_end()
}

/// The `aot` view prints, for a sealed native addition the compile did not
/// select, each premise that rejected it — the lines `tcl explore --show
/// aot --text` prints. Untouched, no pass the addition consumes is
/// requested: the record is the hosted environment and each pass left off,
/// and no proof is built.
#[test]
fn aot_text_lists_every_rejected_native_premise() {
    support::install_surfaces();
    let text = render_all(&data(SEALED_ADD), &["aot".to_owned()], false);
    assert!(
        text.starts_with("=== aot ===\n└── plan: general\n"),
        "{text}"
    );
    assert_eq!(
        native_add_lines(&text),
        "    └── native i64 add: declined (6 premises)\n\
         \x20       ├── sealed-program: hosted-environment\n\
         \x20       ├── pass direct-proc: pass-disabled\n\
         \x20       ├── pass materialisable-slot: pass-disabled\n\
         \x20       ├── pass frame-elision: pass-disabled\n\
         \x20       ├── pass native-integer: pass-disabled\n\
         \x20       └── pass semantic-operation-specialisation: pass-disabled"
    );
}

/// Selecting passes changes what the record says: with every pass but the
/// integer one requested, the premises they answer are evaluated, and the
/// ones the missing pass takes down are named with the call they concern.
#[test]
fn aot_text_follows_the_pass_selection() {
    support::install_surfaces();
    use tcl_explorer::{SemanticOptimisationConfig, SemanticOptimisationPassId};
    let mut config = SemanticOptimisationConfig::new();
    for pass in [
        SemanticOptimisationPassId::DirectProc,
        SemanticOptimisationPassId::MaterialisableSlot,
        SemanticOptimisationPassId::FrameElision,
        SemanticOptimisationPassId::SemanticOperationSpecialisation,
    ] {
        config.enable(pass);
    }
    let result = tcl_explorer::run_pipeline(SEALED_ADD, "tcl8.6");
    let data = tcl_explorer::serialise::serialise_result_with_optimisations(&result, config);
    let text = render_all(&data, &["aot".to_owned()], false);
    assert_eq!(
        native_add_lines(&text),
        "    └── native i64 add: declined (7 premises)\n\
         \x20       ├── sealed-program: hosted-environment\n\
         \x20       ├── pass native-integer: pass-disabled\n\
         \x20       ├── closed-program: hosted-environment\n\
         \x20       ├── direct-body: native-integer-pass-disabled\n\
         \x20       │   · site: ::top block 0 statement 3, argument 0\n\
         \x20       ├── frame: frame-not-elidable\n\
         \x20       │   · site: ::top block 0 statement 3, argument 0\n\
         \x20       ├── actuals: hosted-top-level-observable\n\
         \x20       │   · site: ::top block 0 statement 3, argument 0\n\
         \x20       └── native-integer: pass-disabled\n\
         \x20           · site: ::top block 0 statement 3, argument 0"
    );
}

#[test]
fn render_view_draws_a_tree() {
    support::install_surfaces();
    let d = data("set x 1\nset y 2");
    let text = render_view("ir", &d, false);
    assert!(text.contains("└── ") || text.contains("├── "));
    assert!(!text.contains('\x1b'), "no colour when disabled");
}

#[test]
fn render_all_includes_headers_and_asm() {
    support::install_surfaces();
    let d = data("interp create child");
    let text = render_all(&d, &[], false);
    assert!(text.contains("=== ir ==="));
    assert!(text.contains("=== worldSsa ==="));
    assert!(text.contains("=== asm ==="));
    assert!(text.contains("=== wasm ==="));
}

#[test]
fn show_filter_limits_views() {
    support::install_surfaces();
    let d = data("set x 1");
    let text = render_all(&d, &["ir".to_owned()], false);
    assert!(text.contains("=== ir ==="));
    assert!(!text.contains("=== cfg ==="));
}

#[test]
fn colour_emits_ansi() {
    support::install_surfaces();
    let d = data("set x 1");
    let text = render_all(&d, &["ir".to_owned()], true);
    assert!(text.contains('\x1b'), "colour should emit ANSI codes");
}

#[test]
fn wasm_renders_wat_module() {
    support::install_surfaces();
    // The eval-fallback emitter produces a real WASM module; the view shows
    // its WAT (a `(module …)` with at least the `$::top` function).
    let d = data("set x 1\nputs $x");
    let wat = render_all(&d, &["wasm".to_owned()], false);
    assert!(wat.contains("(module"), "{wat}");
    assert!(wat.contains("$::top"), "{wat}");
}

/// The map has exactly one entry per rendered line.
///
/// This is the property an editor pane depends on: it turns a caret line
/// into an index into `lines`, so a map that drifted by one would send
/// every click to the wrong statement.
#[test]
fn the_line_map_is_aligned_with_the_rendered_text() {
    support::install_surfaces();
    let d = data("proc greet {who} {\n    puts \"hi $who\"\n}\ngreet world\n");
    for view in tcl_explorer::views::tree_view_ids() {
        let rendered = render_view_mapped(view, &d, false);
        assert_eq!(
            rendered.text.lines().count(),
            rendered.lines.len(),
            "{view}: one map entry per rendered line"
        );
        assert_eq!(
            rendered.text,
            render_view(view, &d, false),
            "{view}: the mapped render is the same text"
        );
    }
}

/// A mapped line points at the span its row displays.
#[test]
fn a_mapped_line_points_at_the_span_its_row_shows() {
    support::install_surfaces();
    let source = "set x 1\n";
    let d = data(source);
    let rendered = render_view_mapped("ir", &d, false);
    let (index, range) = rendered
        .lines
        .iter()
        .enumerate()
        .find_map(|(i, r)| r.map(|r| (i, r)))
        .expect("the IR view maps at least one line");
    let line = rendered.text.lines().nth(index).unwrap();
    assert!(
        line.contains("set") || line.contains("assign") || line.contains("range"),
        "line {index} ({line:?}) should be a statement row"
    );
    assert!(
        usize::try_from(range.end_offset).unwrap() <= source.len(),
        "the span must index the source it came from"
    );
}
