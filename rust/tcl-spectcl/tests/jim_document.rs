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

//! What a `# tcl-dialect: jim` document sees, end to end: Jim's own commands
//! from the compiled-in pack, the analyser and the signature scan reading the
//! same registry, and the same text under another dialect keeping that
//! dialect's answers.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyser::types::AnalysisResult;
use tcl_compiler::signature_scan::extract_signatures;
use tcl_core_types::DiagCode;
use tcl_registry::model::resolve_environment;

/// One diagnostic: its code and the source text it covers.
type Finding = (DiagCode, String);

fn run(source: &str, per_item: bool) -> AnalysisResult {
    tcl_spectcl::core_surfaces::ensure();
    let dialect = tcl_registry::dialects::detect_dialect(source, None, "tcl");
    let mut analyser = Analyser::new();
    if per_item {
        analyser.analyse_per_item(source, dialect)
    } else {
        analyser.analyse(source, dialect)
    }
}

fn findings_of(result: &AnalysisResult, source: &str) -> Vec<Finding> {
    result
        .diagnostics
        .iter()
        .map(|d| {
            let end = (d.span.end() as usize).min(source.len());
            (d.code, source[d.span.start() as usize..end].to_owned())
        })
        .collect()
}

fn analyse_with(source: &str, per_item: bool) -> Vec<Finding> {
    findings_of(&run(source, per_item), source)
}

/// Whether `code` is a hint about a variable or parameter nothing reads.
fn is_unused_variable_hint(code: DiagCode) -> bool {
    matches!(code, DiagCode::W211 | DiagCode::W214 | DiagCode::W220)
}

/// The findings that are not a hint about an unused variable.
fn substantive(findings: Vec<Finding>) -> Vec<Finding> {
    findings
        .into_iter()
        .filter(|(code, _)| !is_unused_variable_hint(*code))
        .collect()
}

fn jim(body: &str) -> String {
    format!("# tcl-dialect: jim\n{body}")
}

fn tcl86(body: &str) -> String {
    format!("# tcl-dialect: tcl8.6\n{body}")
}

/// Both analyser tiers: the whole-file walk and the per-item shell walk that
/// defers proc bodies.
fn both_tiers(source: &str) -> [Vec<Finding>; 2] {
    [analyse_with(source, false), analyse_with(source, true)]
}

/// A static is declared in the body scope and is not an argument.
#[test]
fn a_static_is_a_local_of_the_body_and_no_argument() {
    let source = jim("proc counter {} {{n 0}} { return [incr n] }\nputs [counter]\n");
    for findings in both_tiers(&source) {
        assert_eq!(findings, vec![], "{source}");
    }
}

/// Each spelling of a static declares its name: `name`, `{name value}` and,
/// from 0.83, `&name`. A bare `name` and `&name` refer to the enclosing scope's
/// variable, so it is used there.
#[test]
fn every_static_spelling_declares_its_name() {
    let source = jim("set seed 5\nset shared 1\n\
         proc mix {a} {seed {n 0} &shared} { return [expr {$a + $seed + $n + $shared}] }\n\
         puts [mix 1]\n");
    for findings in both_tiers(&source) {
        assert_eq!(findings, vec![], "{source}");
    }
}

/// A bare `name` or `&name` static reads the enclosing scope's variable when the
/// procedure is defined; a `{name value}` static names its own initial value and
/// reads nothing there.
#[test]
fn only_a_bare_or_ampersand_static_reads_the_enclosing_variable() {
    let reads_of = |statics: &str| -> usize {
        let source = jim(&format!(
            "set seed 5\nproc p {{a}} {{{statics}}} {{ return $a }}\nputs $seed\n"
        ));
        [false, true]
            .map(|per_item| {
                let result = run(&source, per_item);
                result.global_scope.variables["seed"].references.len()
            })
            .into_iter()
            .reduce(|first, second| {
                assert_eq!(first, second, "both tiers count the same reads: {source}");
                first
            })
            .expect("two tiers")
    };
    assert_eq!(reads_of("seed"), 2, "the static and the `puts`");
    assert_eq!(reads_of("&seed"), 2, "the reference and the `puts`");
    assert_eq!(reads_of("{seed 0}"), 1, "only the `puts`");
}

/// The edges of a static list: an empty list declares nothing, a one-word list
/// declares one static, and an element of three fields is an error `jimsh`
/// raises while creating the procedure.
#[test]
fn the_edges_of_a_static_list() {
    let empty = jim("proc f {} {} { return 1 }\nputs [f]\n");
    let one_word = jim("set n 1\nproc f {} {n} { return $n }\nputs $n\nputs [f]\n");
    let three_fields = jim("proc f {} {{a b c}} { return 1 }\nputs [f]\n");
    for findings in both_tiers(&empty) {
        assert_eq!(findings, vec![], "{empty}");
    }
    for findings in both_tiers(&one_word) {
        assert_eq!(findings, vec![], "{one_word}");
    }
    for findings in both_tiers(&three_fields) {
        assert_eq!(
            findings,
            vec![(DiagCode::E006, "{{a b c}}".to_owned())],
            "{three_fields}"
        );
    }
}

/// The body of a proc with statics is still walked: an unknown command in it
/// is reported.
#[test]
fn the_body_of_a_proc_with_statics_is_walked() {
    let source = jim("proc counter {} {{n 0}} { not_a_command_anywhere $n }\ncounter\n");
    for findings in both_tiers(&source) {
        assert_eq!(
            findings,
            vec![(DiagCode::W123, "not_a_command_anywhere".to_owned())],
            "{source}"
        );
    }
}

/// A static never adds to the arity: the call supplies the parameters alone.
#[test]
fn a_static_adds_nothing_to_the_arity() {
    let source = jim("proc pair {a b} {{n 0}} { return $a$b$n }\npair 1 2\npair 1\npair 1 2 3\n");
    for findings in both_tiers(&source) {
        let codes: Vec<(DiagCode, &str)> = findings
            .iter()
            .map(|(code, text)| (*code, text.as_str()))
            .collect();
        assert_eq!(
            codes,
            vec![(DiagCode::E002, "pair 1"), (DiagCode::E003, "3")],
            "{source}"
        );
    }
}

/// Three words is the plain form under Jim as under Tcl.
#[test]
fn a_three_word_proc_is_the_plain_form() {
    let source = jim("proc plain {a b} { return $a$b }\nputs [plain 1 2]\n");
    for findings in both_tiers(&source) {
        assert_eq!(findings, vec![], "{source}");
    }
}

/// Tcl's `proc` has no static list: the same four words are an arity error,
/// and the error names the three-word usage.
#[test]
fn a_tcl_document_still_rejects_the_four_word_proc() {
    let source = tcl86("proc counter {} {{n 0}} { incr n }\n");
    for findings in both_tiers(&source) {
        assert!(
            findings.iter().any(|(code, _)| *code == DiagCode::E003),
            "{findings:?}"
        );
    }
}

/// The signature scan places the name, parameter list and body from the same
/// role data, so a cross-file consumer sees the four-word definition as the
/// analyser does.
#[test]
fn the_signature_scan_reads_the_four_word_definition() {
    tcl_spectcl::core_surfaces::ensure();
    let generation = resolve_environment("jim").default_context_registry();
    let source = "proc pair {a b} {{n 0}} { return $a$b$n }\nproc plain {x} { return $x }\n";
    let scan = extract_signatures(source, generation.commands());

    let pair = &scan.procs["::pair"];
    let params: Vec<&str> = pair.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(params, ["a", "b"]);
    let body = &source[pair.body_range.start() as usize..pair.body_range.end() as usize];
    assert!(body.contains("return $a$b$n"), "{body}");

    let plain = &scan.procs["::plain"];
    let body = &source[plain.body_range.start() as usize..plain.body_range.end() as usize];
    assert!(body.contains("return $x"), "{body}");
}

/// A Tcl registry places the three-word definition at the name, the parameter
/// list and the body.
#[test]
fn the_signature_scan_reads_the_three_word_definition_under_tcl() {
    tcl_spectcl::core_surfaces::ensure();
    let generation = resolve_environment("tcl8.6").default_context_registry();
    let source = "proc pair {a b} { return $a$b }\n";
    let scan = extract_signatures(source, generation.commands());
    let pair = &scan.procs["::pair"];
    assert_eq!(pair.params.len(), 2);
    let body = &source[pair.body_range.start() as usize..pair.body_range.end() as usize];
    assert!(body.contains("return $a$b"), "{body}");
}

/// `class NAME {vars}` records the class, its metaclass and the instance
/// variables the dictionary names; the dictionary is read for its keys and
/// never walked as a script.
#[test]
fn a_class_records_the_variables_its_dictionary_names() {
    let source = jim("class Point {x 0 y 0}\nputs [Point new]\n");
    for per_item in [false, true] {
        let result = run(&source, per_item);
        assert_eq!(findings_of(&result, &source), vec![], "{source}");
        let class = &result.all_classes["::Point"];
        assert_eq!(class.variables, ["x", "y"]);
        assert_eq!(class.metaclass, "class");
        assert!(class.superclasses.is_empty());
    }
}

/// A derived class names its bases and inherits their instance variables: the
/// variable dictionary of a derived class is its bases' merged with its own.
#[test]
fn a_derived_class_inherits_its_bases_variables() {
    let source = jim("class Point {x 0 y 0}\nclass Point3 Point {z 0}\n");
    let result = run(&source, false);
    let derived = &result.all_classes["::Point3"];
    assert_eq!(derived.superclasses, ["Point"]);
    assert_eq!(derived.variables, ["x", "y", "z"]);
}

/// A computed base-class list leaves the class's inheritance unknown rather
/// than guessed; a computed variable dictionary names no variables and changes
/// no ancestry.
#[test]
fn only_a_computed_base_list_leaves_the_inheritance_unknown() {
    let source = jim("class Dyn $bases {x 0}\nclass Vars $vars\n");
    for per_item in [false, true] {
        let result = run(&source, per_item);
        assert!(result.all_classes["::Dyn"].inheritance_unknown);
        assert!(!result.all_classes["::Vars"].inheritance_unknown);
        assert!(result.all_classes["::Vars"].variables.is_empty());
    }
}

/// A variable dictionary with a key and no value is an error in `jimsh`; the
/// keys are still read, so a method body's reads of them add nothing.
#[test]
fn an_odd_variable_dictionary_is_an_argument_shape_error() {
    let source = jim("class P {x 0 y}\n");
    for per_item in [false, true] {
        let result = run(&source, per_item);
        assert_eq!(
            findings_of(&result, &source),
            vec![(DiagCode::E005, "{x 0 y}".to_owned())],
            "{source}"
        );
        assert_eq!(result.all_classes["::P"].variables, ["x", "y"]);
    }
}

/// The definer's own arity is enforced: `class` needs a name and a variable
/// dictionary.
#[test]
fn a_class_without_its_variable_dictionary_is_an_arity_error() {
    let source = jim("class Lonely\n");
    assert_eq!(
        analyse_with(&source, false),
        vec![(DiagCode::E002, "class Lonely".to_owned())]
    );
}

/// `CLASS method NAME ARGS BODY` adds a method whose body is walked with the
/// class variables, the inherited ones and `self` bound.
#[test]
fn a_method_body_sees_the_instance_variables_and_self() {
    let source = jim("class Point {x 0 y 0}\nclass Point3 Point {z 0}\n\
         Point method norm {} { return [expr {abs($x) + abs($y)}] }\n\
         Point3 method who {} { not_a_command_anywhere $self $x $z }\n");
    for per_item in [false, true] {
        let result = run(&source, per_item);
        assert!(
            findings_of(&result, &source)
                .contains(&(DiagCode::W123, "not_a_command_anywhere".to_owned())),
            "the body is walked: {source}"
        );
        assert!(result.all_classes["::Point"].methods.contains_key("norm"));
        assert!(result.all_classes["::Point3"].methods.contains_key("who"));
        for variable in [
            "::Point::norm::x",
            "::Point::norm::self",
            "::Point3::who::x",
        ] {
            assert!(
                result.all_variables.contains_key(variable),
                "{variable} is bound in the method body: {:?}",
                result.all_variables.keys().collect::<Vec<_>>()
            );
        }
    }
}

/// A raw `proc {CLASS M} …` is a method too, and — not being wrapped by the
/// class command's `method` — sees neither the class variables nor `self`.
#[test]
fn a_raw_two_word_proc_is_a_method_without_the_class_variables() {
    let source = jim("class Point {x 0}\nproc {Point move} {dx} { return $dx }\n");
    let result = run(&source, false);
    assert_eq!(findings_of(&result, &source), vec![], "{source}");
    assert!(result.all_classes["::Point"].methods.contains_key("move"));
    assert!(!result.all_variables.contains_key("Point move::x"));
    assert!(!result.all_variables.contains_key("Point move::self"));
}

/// A `proc {CLASS M}` written before `class CLASS …` is a method of it too: the
/// class command dispatches through the commands named `CLASS *`.
#[test]
fn a_two_word_proc_written_before_its_class_is_a_method() {
    let source = jim("proc {Point move} {dx} { return $dx }\nclass Point {x 0}\n");
    for per_item in [false, true] {
        let result = run(&source, per_item);
        assert_eq!(findings_of(&result, &source), vec![], "{source}");
        assert!(
            result.all_classes["::Point"].methods.contains_key("move"),
            "per_item={per_item}"
        );
    }
}

/// A method body that names its own class finds it, on either analyser tier:
/// the class stays indexed while the body is walked, so the object the body
/// constructs is typed and `$copy clone` is a dispatch on the class.
#[test]
fn a_method_body_can_construct_its_own_class() {
    let source = jim("class Point {x 0}\n\
         Point method clone {} { set copy [Point new]\n return [$copy clone] }\n");
    for per_item in [false, true] {
        let result = run(&source, per_item);
        assert_eq!(findings_of(&result, &source), vec![], "{source}");
        assert!(result.all_classes["::Point"].methods.contains_key("clone"));
        assert_eq!(
            result.instance_classes["copy"], "::Point",
            "per_item={per_item}"
        );
    }
}

/// `set obj [CLASS new]` types the variable, so `$obj method` is a dispatch on
/// the class rather than a non-literal command head, and the members the
/// class has not declared abstain: any `{CLASS word}` command is a method.
#[test]
fn an_object_is_typed_by_its_class_and_its_member_set_is_open() {
    let source = jim("class Point {x 0}\nproc {Point move} {dx} { return $dx }\n\
         set p [Point new {x 5}]\nputs [$p move 1]\nputs [$p get x]\nputs [$p undeclared]\n");
    for per_item in [false, true] {
        let result = run(&source, per_item);
        assert_eq!(findings_of(&result, &source), vec![], "{source}");
        assert_eq!(result.instance_classes["p"], "::Point");
    }
}

/// A `proc` named by a two-element list defines that two-word command in every
/// dialect: a call whose head is the same list resolves, another list does not,
/// and the arity is the procedure's.
#[test]
fn a_two_element_proc_name_defines_a_two_word_command_in_every_dialect() {
    for dialect in ["tcl8.4", "tcl8.6", "tcl9.0", "jim"] {
        let source = format!(
            "# tcl-dialect: {dialect}\nproc {{a b}} {{x}} {{ return $x }}\n{{a b}} 1\n{{a c}} 1\n{{a b}}\n"
        );
        let found: Vec<Finding> = substantive(analyse_with(&source, false));
        let codes: Vec<DiagCode> = found.iter().map(|(code, _)| *code).collect();
        assert_eq!(
            codes,
            [DiagCode::W123, DiagCode::E002],
            "{dialect}: {found:?}"
        );
    }
}

/// The signature scan records a Jim class, whether or not it names a base, so
/// a cross-file `NAME new` types its receiver.
#[test]
fn the_signature_scan_records_a_jim_class_with_or_without_bases() {
    tcl_spectcl::core_surfaces::ensure();
    let generation = resolve_environment("jim").default_context_registry();
    let source = "class Point {x 0}\nclass Point3 Point {z 0}\n";
    let scan = extract_signatures(source, generation.commands());
    let mut classes: Vec<&str> = scan.classes.keys().map(String::as_str).collect();
    classes.sort_unstable();
    assert_eq!(classes, ["::Point", "::Point3"]);
    // The body is the variable dictionary, not the base-class word before it.
    let derived = &scan.classes["::Point3"];
    let dictionary = source.find("{z 0}").expect("the dictionary is written");
    assert_eq!(derived.body_range.start() as usize, dictionary);
}

/// A Jim program that defines a class, gives it a two-word method with a
/// static, and uses the class, a `loop` and `sleep` — the commands a `jimsh`
/// script writes that `tclsh` has no word for.
const JIM_PROGRAM: &str = "class system {model \"\"}
proc {system model} {} {{model \"\"}} {
    return $model
}
loop i 0 3 { puts $i }
sleep 0.1
set s [system new]
puts [$s model]
";

/// The whole program is accepted by a `jim` document, on both analyser tiers.
#[test]
fn a_jim_program_draws_no_findings() {
    let source = jim(JIM_PROGRAM);
    for findings in both_tiers(&source) {
        assert_eq!(findings, vec![], "{source}");
    }
}

/// The same text under `tcl8.6` is a different program: Tcl's `proc` takes
/// three words, so the four-word definition is an arity error, and `loop` and
/// `sleep` are Jim's commands, disabled in a Tcl document.
#[test]
fn the_same_program_under_tcl_is_rejected() {
    let source = tcl86(JIM_PROGRAM);
    for findings in both_tiers(&source) {
        let on =
            |code: DiagCode, text: &str| findings.iter().any(|f| *f == (code, text.to_owned()));
        assert!(
            findings
                .iter()
                .any(|(code, text)| *code == DiagCode::E003 && text.contains("return $model")),
            "the four-word proc: {findings:?}"
        );
        assert!(
            findings
                .iter()
                .any(|(code, text)| *code == DiagCode::W002 && text == "loop"),
            "loop is Jim's, not Tcl's: {findings:?}"
        );
        assert!(on(DiagCode::W002, "sleep"), "{findings:?}");
    }
}

/// `loop i 0 3 {…}` is flagged under `tcl8.6`, as Jim's and disabled there,
/// and accepted under `jim`, where its body is walked as the loop body it is.
#[test]
fn loop_is_flagged_under_tcl_and_accepted_under_jim() {
    let program = "loop i 0 3 { puts $i }\nloop k 2 { puts $k }\n";
    let jim_findings = analyse_with(&jim(program), false);
    assert_eq!(jim_findings, vec![], "{jim_findings:?}");
    let tcl_findings = analyse_with(&tcl86(program), false);
    assert_eq!(
        tcl_findings
            .iter()
            .filter(|(code, text)| *code == DiagCode::W002 && text == "loop")
            .count(),
        2,
        "{tcl_findings:?}"
    );

    let unknown_in_body = jim("loop i 0 3 { not_a_command_anywhere $i }\n");
    assert_eq!(
        analyse_with(&unknown_in_body, false),
        vec![(DiagCode::W123, "not_a_command_anywhere".to_owned())],
        "the body is a script: {unknown_in_body}"
    );
}

/// Under another dialect `class` is not Jim's: nothing is recorded as a class,
/// `class` is reported as a Jim command disabled there, and the class name it
/// would have defined is unknown.
#[test]
fn a_tcl_document_has_no_jim_class() {
    let source = tcl86("class Point {x 0 y 0}\nset p [Point new]\n");
    let result = run(&source, false);
    assert!(
        result.all_classes.is_empty(),
        "{:?}",
        result.all_classes.keys()
    );
    let findings = findings_of(&result, &source);
    assert!(
        result.diagnostics.iter().any(|d| d.code == DiagCode::W002
            && &source[d.span.start() as usize..d.span.end() as usize] == "class"
            && d.message.contains("jim")),
        "{findings:?}"
    );
    assert!(
        findings.contains(&(DiagCode::W123, "Point".to_owned())),
        "{findings:?}"
    );
}
