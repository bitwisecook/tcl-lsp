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

//! What each member subcommand records: slots, methods, constructors, forwards, variables, filters, exports and properties.

use super::*;

#[test]
fn superclass_subcommand_assigns_supers() {
    let mut cd = class();
    let texts: Vec<String> = ["superclass", "::A", "::B"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 10)), tok((11, 14)), tok((15, 18))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.superclasses, vec!["::A", "::B"]);
}

#[test]
fn mixin_subcommand_strips_dash_flags() {
    let mut cd = class();
    let texts: Vec<String> = ["mixin", "-append", "::M1", "::M2"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 5)), tok((6, 13)), tok((14, 18)), tok((19, 23))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.mixins, vec!["::M1", "::M2"]);
}

/// Apply a sequence of definition-body words to one class — the shape
/// slot folding is about: later words must fold into,
/// not overwrite, earlier slot state.
fn apply_words(cd: &mut super::super::ClassDef, calls: &[&[&str]]) {
    for words in calls {
        let texts: Vec<String> = words.iter().map(|s| (*s).to_string()).collect();
        let argv: Vec<tcl_lexer::Token> = words.iter().map(|_| tok((0, 1))).collect();
        apply_oo_subcommand(tcloo(), &texts, &argv, cd);
    }
}

// Slot semantics for filter / superclass / mixin / variable (defaults
// pinned against tclOODefineCmds.c 9.0.4 and confirmed live on
// tclsh 9.0.4).

// TP: `filter a ; filter b` appends — `info class filters` → `a b`,
// NOT `b` (which is what a last-writer-wins reading would give).
#[test]
fn filter_slot_appends_across_calls() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a"], &["filter", "b"]]);
    assert_eq!(cd.filters, vec!["a", "b"]);
}

// TP: the explicit operations fold — `-set` replaces, `-clear` empties,
// `-prepend` front-inserts (tclsh 9.0.4: `filter a; filter -prepend b`
// → `b a`), `-remove` deletes.
#[test]
fn filter_slot_explicit_operations_fold() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a", "b"], &["filter", "-set", "x"]]);
    assert_eq!(cd.filters, vec!["x"]);
    apply_words(&mut cd, &[&["filter", "-clear"]]);
    assert_eq!(cd.filters, [] as [std::string::String; 0]);
    apply_words(&mut cd, &[&["filter", "a"], &["filter", "-prepend", "b"]]);
    assert_eq!(cd.filters, vec!["b", "a"]);
    apply_words(&mut cd, &[&["filter", "-remove", "b"]]);
    assert_eq!(cd.filters, vec!["a"]);
}

// FP guard: an operation word is recognised at argument 0 only —
// `filter a -set b` appends three literal items (tclsh 9.0.4), it does
// not replace the slot with `b`.
#[test]
fn filter_slot_op_word_only_recognised_first() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a", "-set", "b"]]);
    assert_eq!(cd.filters, vec!["a", "-set", "b"]);
}

// TP: `superclass` defaults to `-set` (replace) — the common single
// declaration keeps its meaning — and `-append` extends (tclsh 9.0.4:
// `superclass A` after `superclass B` → `::B`; `superclass -append A`
// → `::B ::A`).
#[test]
fn superclass_slot_replaces_by_default_and_appends_explicitly() {
    let mut cd = class();
    apply_words(&mut cd, &[&["superclass", "::A"], &["superclass", "::B"]]);
    assert_eq!(cd.superclasses, vec!["::B"]);
    apply_words(&mut cd, &[&["superclass", "-append", "::A"]]);
    assert_eq!(cd.superclasses, vec!["::B", "::A"]);
}

// TP: `mixin` defaults to `-set` in 8.6 and 9.0 alike (both C sources
// forward `--default-operation` to `-set`; tclsh 9.0.4:
// `mixin M1; mixin M2` → `::M2`).
#[test]
fn mixin_slot_replaces_by_default() {
    let mut cd = class();
    apply_words(&mut cd, &[&["mixin", "::M1"], &["mixin", "::M2"]]);
    assert_eq!(cd.mixins, vec!["::M2"]);
}

// TP: the class `variable` slot appends with dedup (tclsh 9.0.4:
// `variable a; variable a b` → `a b`) and folds `-set` / `-remove`.
#[test]
fn variable_slot_appends_dedups_and_folds_ops() {
    let mut cd = class();
    apply_words(&mut cd, &[&["variable", "a"], &["variable", "a", "b"]]);
    assert_eq!(cd.variables, vec!["a", "b"]);
    apply_words(&mut cd, &[&["variable", "-remove", "a"]]);
    assert_eq!(cd.variables, vec!["b"]);
    apply_words(&mut cd, &[&["variable", "-set", "c"]]);
    assert_eq!(cd.variables, vec!["c"]);
}

// TN: an unknown leading `-op` aborts the whole definition in real Tcl
// (`unknown method "-bogus"`), so the fold must leave the slot state
// unchanged rather than record `-bogus`/`x` as filters.
#[test]
fn unknown_slot_op_leaves_slot_untouched() {
    let mut cd = class();
    apply_words(&mut cd, &[&["filter", "a"], &["filter", "-bogus", "x"]]);
    assert_eq!(cd.filters, vec!["a"]);
}

// TP, full pipeline: the fold carries across definition blocks — the
// oracle transcript verbatim (tclsh 9.0.4 and 8.6.16, byte-identical):
//   oo::class create C { filter a; filter b } → info class filters → a b
//   oo::define C { filter -set x }            →                    → x
//   oo::define C { filter -clear }            →                    → (empty)
#[test]
fn filter_slot_folds_across_oo_define_blocks() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { filter a\nfilter b }\n\
             oo::define ::C { filter -set x }\n",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.filters, vec!["x"]);

    let mut a2 = Analyser::new();
    let r2 = a2.analyse(
        "oo::class create ::C { filter a\nfilter b }\n\
             oo::define ::C { filter -append c }\n",
        "tcl9.0",
    );
    let c2 = r2.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c2.filters, vec!["a", "b", "c"]);
}

// The class-object side folds through the same slot spec: `self filter`
// appends across calls, and never leaks into the instance slot.
#[test]
fn class_side_filter_slot_folds_and_stays_sided() {
    let mut a = Analyser::new();
    let r = a.analyse(
        "oo::class create ::C { self { filter a }\nself { filter b } }\n",
        "tcl9.0",
    );
    let c = r.all_classes.get("::C").expect("::C recorded");
    assert_eq!(c.class_filters, vec!["a", "b"]);
    assert_eq!(c.filters, [] as [std::string::String; 0]);
}

#[test]
fn method_subcommand_records_method_def() {
    let mut cd = class();
    let texts: Vec<String> = ["method", "greet", "name", "puts $name"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 6)), tok((7, 12)), tok((13, 17)), str_tok((18, 32))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.methods.contains_key("greet"));
    let md = &cd.methods["greet"];
    assert_eq!(md.kind, "method");
    assert_eq!(md.visibility, "public");
}

/// Tcl 9's declaration flag sits between a method name and parameter list.
/// The analyser receives only the generic registry layout and its selected
/// visibility—not a command-specific option parser.
#[test]
fn method_visibility_option_shifts_params_and_sets_visibility() {
    let mut cd = class();
    let texts: Vec<String> = ["method", "internal", "-private", "arg", "puts $arg"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 6)),
        tok((7, 15)),
        tok((16, 24)),
        tok((25, 28)),
        str_tok((29, 38)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let method = cd.methods.get("internal").expect("method recorded");
    assert_eq!(method.visibility, "private");
    assert_eq!(method.params[0].name, "arg");
    assert_eq!(method.body_span, tcl_lexer::Span::new(29, 38));
}

#[test]
fn method_visibility_option_is_profile_gated_without_old_layout_reinterpretation() {
    let src = "oo::class create C { method internal -private {} { return ok } }";
    let mut analyser = Analyser::new();
    let tcl86 = analyser.analyse(src, "tcl8.6").clone();
    assert!(
        tcl86
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code.as_str() == "W002"),
        "Tcl 8.6 must diagnose the Tcl 9 method option: {:?}",
        tcl86.diagnostics
    );
    assert!(
        tcl86
            .all_classes
            .get("::C")
            .is_none_or(|class| !class.methods.contains_key("internal")),
        "the invalid Tcl 8.6 member must not be reinterpreted using a shifted layout"
    );

    let mut analyser = Analyser::new();
    let tcl90 = analyser.analyse(src, "tcl9.0").clone();
    assert_eq!(
        tcl90.all_classes["::C"].methods["internal"].visibility,
        "private"
    );
}

#[test]
fn classmethod_subcommand_records_class_method() {
    let mut cd = class();
    let texts: Vec<String> = ["classmethod", "build", "args", "return $args"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 11)),
        tok((12, 17)),
        tok((18, 22)),
        str_tok((23, 38)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.class_methods.contains_key("build"));
    assert!(!cd.methods.contains_key("build"));
}

#[test]
fn constructor_appends_to_constructors_list() {
    let mut cd = class();
    let texts: Vec<String> = ["constructor", "args", "puts ctor"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 11)), tok((12, 16)), str_tok((17, 28))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.constructors.len(), 1);
    assert_eq!(cd.constructors[0].kind, "constructor");
    assert_eq!(cd.constructors[0].name, "<constructor>");
    // Name span anchors on the `constructor` keyword token
    // (argv[0] = 0..11), not the default (0, 0).
    assert_eq!(cd.constructors[0].name_span, tcl_lexer::Span::new(0, 11));
    // Constructors are not mirrored into the methods map.
    assert!(!cd.methods.contains_key("<constructor>"));
}

#[test]
fn destructor_sets_destructor_field() {
    let mut cd = class();
    let texts: Vec<String> = ["destructor", "puts dtor"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 10)), str_tok((11, 22))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let dtor = cd.destructor.as_ref().expect("destructor recorded");
    assert_eq!(dtor.kind, "destructor");
    assert_eq!(dtor.name, "<destructor>");
    // Name span anchors on the `destructor` keyword token
    // (argv[0] = 0..10).
    assert_eq!(dtor.name_span, tcl_lexer::Span::new(0, 10));
    assert!(!cd.methods.contains_key("<destructor>"));
}

#[test]
fn forward_records_method_with_forward_kind() {
    let mut cd = class();
    let texts: Vec<String> = ["forward", "delegate", "::other::cmd"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 7)), tok((8, 16)), tok((17, 29))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.methods.contains_key("delegate"));
    assert_eq!(cd.methods["delegate"].kind, "forward");
}

#[test]
fn private_method_records_with_private_visibility() {
    let mut cd = class();
    let texts: Vec<String> = ["private", "method", "internal", "args", "puts hi"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 7)),
        tok((8, 14)),
        tok((15, 23)),
        tok((24, 28)),
        str_tok((29, 37)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(cd.methods.contains_key("internal"));
    assert_eq!(cd.methods["internal"].visibility, "private");
}

#[test]
fn self_method_subcommand_records_class_method_tagged_is_self_method() {
    // TP: `self method NAME ARGS BODY` (TclOO's own spelling of a
    // class-level method, the stock counterpart to ooutil's `classmethod`
    // keyword) must land by its row's type-object receiver, or
    // `class_methods` gains no entry for it. Recorded with
    // `kind: "classmethod"` (both
    // spellings mean "dispatched via the class's own bound command")
    // but tagged `is_self_method` so the class-command MRO walk knows
    // NOT to treat it as inherited the way ooutil's `classmethod` is.
    let mut cd = class();
    let texts: Vec<String> = ["self", "method", "make", "n", "return made"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 4)),
        tok((5, 11)),
        tok((12, 16)),
        tok((17, 18)),
        str_tok((19, 30)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert!(!cd.methods.contains_key("make"));
    let md = cd
        .class_methods
        .get("make")
        .expect("recorded as a classmethod");
    assert_eq!(md.kind, "classmethod");
    assert!(md.is_self_method);
    assert_eq!(md.visibility, "public");
}

#[test]
fn self_classmethod_subcommand_also_records_into_class_methods() {
    // TP — the `self classmethod` inner spelling (rarer, but valid:
    // `self` always retargets to the class object regardless of which
    // inner member follows) must resolve identically to `self method`.
    let mut cd = class();
    let texts: Vec<String> = ["self", "classmethod", "build", "args", "return $args"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 4)),
        tok((5, 16)),
        tok((17, 22)),
        tok((23, 27)),
        str_tok((28, 43)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let md = cd
        .class_methods
        .get("build")
        .expect("recorded as a classmethod");
    assert!(md.is_self_method);
}

#[test]
fn self_block_form_is_a_silent_noop_not_a_crash() {
    // FN guard — this helper does not read the `self { method NAME ARGS
    // BODY }` *block* form (nor the symmetric `private` one). It must
    // decline cleanly, never panic on the whole braced blob standing in
    // for an inner keyword.
    let mut cd = class();
    let texts: Vec<String> = ["self", "{ method make {n} { return made } }"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 4)), str_tok((5, 42))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.class_methods.len(), 0);
    assert_eq!(cd.methods.len(), 0);
}

#[test]
fn unrecognised_subcommand_is_silent_noop() {
    let mut cd = class();
    let texts: Vec<String> = ["whatever", "x"].iter().map(|s| (*s).to_string()).collect();
    let argv = [tok((0, 8)), tok((9, 10))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    // No fields populated; no panic.
    assert_eq!(cd.methods.len(), 0);
    assert_eq!(cd.superclasses, [] as [std::string::String; 0]);
    assert_eq!(cd.mixins, [] as [std::string::String; 0]);
}

#[test]
fn variable_declaration_spans_use_selected_original_literal_name_operands() {
    let source = "variable {$literal} {${braced}} {}; variable $computed; variable {*}{shifted}";
    let profile = tcl_dialect::DialectProfile::find("tcl9.1").unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let image = tcl_lexer::SourceImage::document(source);
    let commands = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
    let known = [
        "$literal",
        "${braced}",
        "",
        "computed",
        "$computed",
        "shifted",
    ]
    .map(str::to_owned);
    let spans = collect_var_decl_spans(tcloo(), &commands, &known, &image, config, None);
    for name in ["$literal", "${braced}", ""] {
        assert!(spans.contains_key(name), "{name:?}: {spans:?}");
    }
    assert!(!spans.contains_key("literal") && !spans.contains_key("braced"));
    assert!(
        !spans.contains_key("computed")
            && !spans.contains_key("$computed")
            && !spans.contains_key("shifted")
    );
}

#[test]
fn variable_subcommand_records_class_variables() {
    let mut cd = class();
    let texts: Vec<String> = ["variable", "x", "y"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 8)), tok((9, 10)), tok((11, 12))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.variables, vec!["x", "y"]);
}

#[test]
fn a_second_variable_subcommand_accumulates_rather_than_replacing() {
    // TP — the real corpus shape
    // (georgtree_tclopt's ::tclopt::Mpfit) has TWO separate `variable`
    // statements in the same class body (`variable funct m ftol ...`
    // then, separately, `variable Pars`). tclsh9.0-verified: both
    // statements' names are live, simultaneous instance variables —
    // `variable` inside a class body is additive, never a reset (the
    // same "always present in every method" declaration a `variable`
    // command inside a method body itself would make, just issued
    // once for the whole class). A second statement must not silently
    // discard the first's names.
    let mut cd = class();
    apply_oo_subcommand(
        tcloo(),
        &["variable".to_string(), "funct".to_string(), "m".to_string()],
        &[tok((0, 8)), tok((9, 14)), tok((15, 16))],
        &mut cd,
    );
    apply_oo_subcommand(
        tcloo(),
        &["variable".to_string(), "Pars".to_string()],
        &[tok((20, 28)), tok((29, 33))],
        &mut cd,
    );
    assert_eq!(cd.variables, vec!["funct", "m", "Pars"]);
}

#[test]
fn filter_subcommand_records_filters() {
    let mut cd = class();
    let texts: Vec<String> = ["filter", "log", "trace"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [tok((0, 6)), tok((7, 10)), tok((11, 16))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.filters, vec!["log", "trace"]);
    // …and only the instance slot: the class object stays unfiltered.
    assert_eq!(cd.class_filters.len(), 0, "{:?}", cd.class_filters);
}

#[test]
fn export_and_unexport_record_sets() {
    let mut cd = class();
    let texts1: Vec<String> = ["export", "foo", "bar"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv1 = [tok((0, 6)), tok((7, 10)), tok((11, 14))];
    apply_oo_subcommand(tcloo(), &texts1, &argv1, &mut cd);
    assert!(cd.exports.contains("foo"));
    assert!(cd.exports.contains("bar"));

    let texts2: Vec<String> = ["unexport", "baz"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv2 = [tok((0, 8)), tok((9, 12))];
    apply_oo_subcommand(tcloo(), &texts2, &argv2, &mut cd);
    assert!(cd.unexports.contains("baz"));
}

#[test]
fn property_subcommand_records_property_def() {
    let mut cd = class();
    let texts: Vec<String> = [
        "property",
        "colour",
        "-kind",
        "readable",
        "-get",
        "return red",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect();
    let argv = [
        tok((0, 8)),
        tok((9, 15)),
        tok((16, 21)),
        tok((22, 30)),
        tok((31, 35)),
        str_tok((36, 47)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let pd = cd.properties.get("colour").expect("colour recorded");
    assert_eq!(pd.kind, "readable");
    assert!(pd.has_getter);
    assert!(!pd.has_setter);
}

/// `property` is a 9.0 `TclOO` member: under 8.6 it is flagged
/// disabled-in-dialect (W002), but from 9.0 on it is accepted silently.
///
/// Either way the member is **recorded** — reporting "this needs Tcl
/// 9.0" must not also break go-to-definition, references, rename, and
/// document symbols over the code the user wrote, and the whole-command
/// W002 behaves the same way (it reports a dialect-unavailable command
/// while the analyser goes on modelling the call).
#[test]
fn property_member_is_gated_to_9_0() {
    let src = "oo::class create C {\n    property color\n}\n";
    let mut a86 = Analyser::new();
    let r86 = a86.analyse(src, "tcl8.6");
    assert!(
        r86.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W002),
        "property should be W002 under 8.6",
    );
    assert!(
        r86.all_classes
            .get("::C")
            .is_some_and(|c| c.properties.contains_key("color")),
        "the member is still recorded, so editor features keep working",
    );

    let mut a90 = Analyser::new();
    let r90 = a90.analyse(src, "tcl9.0");
    assert!(
        !r90.diagnostics
            .iter()
            .any(|d| d.code == tcl_core_types::DiagCode::W002),
        "property is available in 9.0",
    );
    assert!(
        r90.all_classes
            .get("::C")
            .is_some_and(|c| c.properties.contains_key("color")),
        "9.0 records the property",
    );
}

/// A configurable class answers `configure` for its properties, so that
/// generated accessor is folded into its known methods even though no
/// `method` body defines it.  `cget` is **not** generated
/// (`configurable.n` documents none in 9.0 or 9.1), so it must not be.
#[test]
fn configurable_class_knows_configure_but_not_cget() {
    let mut a = Analyser::new();
    let r = a
        .analyse(
            "oo::configurable create C {\n    property color\n}\n",
            "tcl9.0",
        )
        .clone();
    let known = r.class_hierarchy().known_methods(
        Some(tcl_registry::model::ingress::static_context_for("tcl9.0").commands()),
        "::C",
    );
    assert!(known.contains(&"configure".to_owned()), "{known:?}");
    assert!(!known.contains(&"cget".to_owned()), "{known:?}");
}

#[test]
fn property_subcommand_with_no_kind_defaults_to_readwrite() {
    let mut cd = class();
    let texts: Vec<String> = ["property", "x"].iter().map(|s| (*s).to_string()).collect();
    let argv = [tok((0, 8)), tok((9, 10))];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    let pd = cd.properties.get("x").expect("x recorded");
    assert_eq!(pd.kind, "readwrite");
    assert!(!pd.has_getter);
    assert!(!pd.has_setter);
}

#[test]
fn property_subcommand_records_multiple_names() {
    let mut cd = class();
    let texts: Vec<String> = ["property", "x", "y", "-kind", "writable"]
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let argv = [
        tok((0, 8)),
        tok((9, 10)),
        tok((11, 12)),
        tok((13, 18)),
        tok((19, 27)),
    ];
    apply_oo_subcommand(tcloo(), &texts, &argv, &mut cd);
    assert_eq!(cd.properties.len(), 2);
    assert_eq!(cd.properties["x"].kind, "writable");
    assert_eq!(cd.properties["y"].kind, "writable");
}

#[test]
fn extract_method_def_too_few_args_returns_none() {
    // ``method`` with only 1 arg (just the name) — needs 3 (name, params,
    // body), so the grammar's `Body` word at index 2 is absent.
    let member = tcloo().member("method").expect("method is a member");
    let args: Vec<String> = vec!["foo".to_string()];
    let arg_tokens: Vec<Token> = vec![tok((0, 3))];
    let md = extract_method_def(member, &args, &arg_tokens, "method", "public", "");
    assert!(md.is_none());
}
