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
use crate::analyser::types::MethodDef;
use tcl_lexer::Span;

fn span() -> Span {
    Span::new(0, 0)
}

fn cls(qname: &str, supers: &[&str], mixins: &[&str], methods: &[&str]) -> ClassDef {
    let mut cd = ClassDef {
        name: crate::naming::key_tail(qname).to_string(),
        qualified_name: qname.to_string(),
        name_span: span(),
        body_span: span(),
        superclasses: supers.iter().map(|s| (*s).to_string()).collect(),
        mixins: mixins.iter().map(|s| (*s).to_string()).collect(),
        ..Default::default()
    };
    for m in methods {
        cd.methods.insert(
            (*m).to_string(),
            MethodDef {
                name: (*m).to_string(),
                params: Vec::new(),
                params_computed: false,
                formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
                    tcl_dialect::ParameterGrammar::Tcl,
                ),
                name_span: span(),
                body_span: span(),
                kind: "method".to_string(),
                is_self_method: false,
                visibility: "public".to_string(),
                doc: String::new(),
                forward_target: None,
            },
        );
    }
    cd
}

fn map(classes: Vec<ClassDef>) -> HashMap<String, ClassDef> {
    classes
        .into_iter()
        .map(|c| (c.qualified_name.clone(), c))
        .collect()
}

#[test]
fn declared_member_keeps_reported_side_and_declaring_class_separate() {
    let mut class = cls("::C", &[], &[], &["same"]);
    let mut class_method = class.methods["same"].clone();
    class_method.name_span = Span::new(20, 24);
    class_method.is_self_method = true;
    class.class_methods.insert("same".to_owned(), class_method);
    let hierarchy = build_class_hierarchy(map(vec![class, cls("::Sub", &["::C"], &[], &[])]));
    let (_, instance) = hierarchy
        .declared_member("::C", "same", super::super::types::MemberSide::Instance)
        .unwrap();
    let (_, class_object) = hierarchy
        .declared_member("::C", "same", super::super::types::MemberSide::ClassObject)
        .unwrap();
    assert_ne!(instance.name_span, class_object.name_span);
    assert_eq!(hierarchy.method_target("::Sub", "same"), Some("::C"));
    assert!(
        hierarchy
            .declared_member("::Sub", "same", super::super::types::MemberSide::Instance)
            .is_none()
    );
}

#[test]
fn empty_input_yields_empty_hierarchy() {
    let h = build_class_hierarchy(HashMap::new());
    assert_eq!(h.classes.len(), 0);
    assert_eq!(h.mro_map.len(), 0);
    assert_eq!(h.subclasses.len(), 0);
    assert_eq!(h.errors, [] as [crate::analyser::MroError; 0]);
}

#[test]
fn single_class_self_mro() {
    let classes = map(vec![cls("::A", &[], &[], &["m"])]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.mro_map["::A"], vec!["::A"]);
    assert_eq!(h.method_target("::A", "m"), Some("::A"));
}

#[test]
fn single_inheritance_method_resolution() {
    // B inherits from A; A defines ``m``; B's lookup of
    // ``m`` resolves to A.
    let classes = map(vec![
        cls("::A", &[], &[], &["m"]),
        cls("::B", &["::A"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.mro_map["::B"], vec!["::B", "::A"]);
    assert_eq!(h.method_target("::B", "m"), Some("::A"));
    assert!(h.is_subtype("::B", "::A"));
    assert!(!h.is_subtype("::A", "::B"));
}

#[test]
fn diamond_method_resolution_with_late_placement() {
    // D → B, C → A.  TclOO late-placement puts A after C.
    // B defines ``m`` so D's ``m`` resolves to B.
    let classes = map(vec![
        cls("::A", &[], &[], &[]),
        cls("::B", &["::A"], &[], &["m"]),
        cls("::C", &["::A"], &[], &[]),
        cls("::D", &["::B", "::C"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.mro_map["::D"], vec!["::D", "::B", "::C", "::A"]);
    assert_eq!(h.method_target("::D", "m"), Some("::B"));
}

#[test]
fn mixin_provider_takes_precedence() {
    // B mixes M; M defines ``m``; B's ``m`` resolves to M.
    let classes = map(vec![
        cls("::A", &[], &[], &[]),
        cls("::M", &[], &[], &["m"]),
        cls("::B", &["::A"], &["::M"], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.mro_map["::B"], vec!["::M", "::B", "::A"]);
    assert_eq!(h.method_target("::B", "m"), Some("::M"));
}

#[test]
fn direct_subclasses_recorded() {
    let classes = map(vec![
        cls("::A", &[], &[], &[]),
        cls("::B", &["::A"], &[], &[]),
        cls("::C", &["::A"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    let a_subs = &h.subclasses["::A"];
    assert!(a_subs.contains("::B"));
    assert!(a_subs.contains("::C"));
    assert_eq!(a_subs.len(), 2);
    // Leaves have empty subclass sets, never None.
    assert_eq!(h.subclasses["::B"].len(), 0);
}

#[test]
fn mixin_recorded_as_subclass_edge() {
    // `B` mixes `M`.  `is_subtype(B, M)` is true (the MRO places the
    // mixin as a supertype), so the subclass map must list `B` under
    // `M` too — otherwise the two views disagree.
    let classes = map(vec![
        cls("::M", &[], &[], &[]),
        cls("::A", &[], &[], &[]),
        cls("::B", &["::A"], &["::M"], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert!(h.is_subtype("::B", "::M"));
    assert!(
        h.subclasses["::M"].contains("::B"),
        "{:?}",
        h.subclasses["::M"]
    );
    assert!(h.subclasses["::A"].contains("::B"));
    // Transitive closure agrees.
    assert!(h.transitive_subtypes["::M"].contains("::B"));
}

#[test]
fn transitive_subtypes_via_bfs() {
    let classes = map(vec![
        cls("::A", &[], &[], &[]),
        cls("::B", &["::A"], &[], &[]),
        cls("::C", &["::B"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    let a_descendants = &h.transitive_subtypes["::A"];
    assert!(a_descendants.contains("::B"));
    assert!(a_descendants.contains("::C"));
    assert_eq!(a_descendants.len(), 2);
}

#[test]
fn cycle_collected_as_error_not_panic() {
    let classes = map(vec![
        cls("::A", &["::B"], &[], &[]),
        cls("::B", &["::A"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_ne!(h.errors, [] as [crate::analyser::MroError; 0]);
    // Cycle classes get single-element MRO fallback.
    assert_eq!(h.mro_map["::A"], vec!["::A"]);
}

#[test]
fn relation_cycle_errors_distinguish_a_mixin_cycle_from_a_diamond() {
    let cyclic = build_class_hierarchy(map(vec![
        cls("::A", &[], &["::B"], &[]),
        cls("::B", &[], &["::A"], &[]),
        cls("::C", &[], &["::A"], &[]),
    ]));
    let relation = crate::analyser::mro::MroErrorKind::RelationCycle;
    let mut cyclic_classes: Vec<_> = cyclic
        .errors
        .iter()
        .filter(|error| error.kind == relation)
        .map(|error| error.class.as_str())
        .collect();
    cyclic_classes.sort_unstable();
    assert_eq!(cyclic_classes, ["::A", "::B", "::C"]);

    let diamond = build_class_hierarchy(map(vec![
        cls("::Root", &[], &[], &[]),
        cls("::Left", &["::Root"], &[], &[]),
        cls("::Right", &["::Root"], &[], &[]),
        cls("::Leaf", &["::Left", "::Right"], &[], &[]),
    ]));
    assert_eq!(diamond.errors, [] as [crate::analyser::MroError; 0]);
}

#[test]
fn unqualified_parent_normalised_when_match_exists() {
    // ``::B`` lists ``A`` (no ``::``) as a superclass; the
    // normalisation step adds the leading ``::`` because
    // ``::A`` is in the class map.
    let classes = map(vec![
        cls("::A", &[], &[], &["m"]),
        cls("::B", &["A"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.mro_map["::B"], vec!["::B", "::A"]);
    assert_eq!(h.method_target("::B", "m"), Some("::A"));
}

#[test]
fn method_target_returns_none_for_unknown_method() {
    let classes = map(vec![cls("::A", &[], &[], &[])]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.method_target("::A", "nope"), None);
}

/// `constructor_provider` mirrors `method_target` but over
/// `ClassDef::constructors` rather than `methods`.
///
/// Real source text `is_empty_method_body` slices into: byte 0-1 is
/// `"{}"` (an empty body), byte 2-4 is `"{x"` (`is_empty_method_body`'s
/// documented non-empty-content slice shape — the span covers the
/// opener through the last inner byte, excluding the closer).
const CTOR_SRC: &str = "{}{x}";
const EMPTY_BODY: Span = Span::new(0, 2);
const NON_EMPTY_BODY: Span = Span::new(2, 4);

/// A real (non-empty-bodied) constructor.
fn cls_with_ctor(qname: &str, supers: &[&str]) -> ClassDef {
    cls_with_ctor_body(qname, supers, NON_EMPTY_BODY)
}

fn cls_with_ctor_body(qname: &str, supers: &[&str], body_span: Span) -> ClassDef {
    let mut cd = cls(qname, supers, &[], &[]);
    cd.constructors.push(MethodDef {
        name: "<constructor>".to_string(),
        params: Vec::new(),
        params_computed: false,
        formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
            tcl_dialect::ParameterGrammar::Tcl,
        ),
        name_span: span(),
        body_span,
        kind: "constructor".to_string(),
        is_self_method: false,
        visibility: "public".to_string(),
        doc: String::new(),
        forward_target: None,
    });
    cd
}

#[test]
fn constructor_provider_none_when_no_class_declares_one() {
    let classes = map(vec![cls("::A", &[], &[], &[])]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_provider("::A", CTOR_SRC), None);
}

#[test]
fn constructor_provider_own_class() {
    let classes = map(vec![cls_with_ctor("::A", &[])]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_provider("::A", CTOR_SRC), Some("::A"));
}

#[test]
fn constructor_provider_inherited_from_superclass() {
    // ::B has no constructor of its own; ::A's is inherited.
    let classes = map(vec![
        cls_with_ctor("::A", &[]),
        cls("::B", &["::A"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_provider("::B", CTOR_SRC), Some("::A"));
}

#[test]
fn constructor_provider_own_overrides_inherited() {
    let classes = map(vec![
        cls_with_ctor("::A", &[]),
        cls_with_ctor("::B", &["::A"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_provider("::B", CTOR_SRC), Some("::B"));
}

#[test]
fn constructor_provider_none_for_empty_body_constructor() {
    // `constructor {a b} {}` (a literally empty body) is `TclOO`'s way
    // of writing "no constructor" — confirmed against tclsh 9.0.4:
    // `info class constructor` returns empty and `new` accepts any
    // argument count.
    let classes = map(vec![cls_with_ctor_body("::A", &[], EMPTY_BODY)]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_provider("::A", CTOR_SRC), None);
}

#[test]
fn constructor_provider_falls_back_to_superclass_when_own_is_empty_bodied() {
    let classes = map(vec![
        cls_with_ctor("::A", &[]),
        cls_with_ctor_body("::B", &["::A"], EMPTY_BODY),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_provider("::B", CTOR_SRC), Some("::A"));
}

#[test]
fn is_empty_method_body_matches_only_the_exact_delimiter_pair() {
    assert!(is_empty_method_body(CTOR_SRC, EMPTY_BODY));
    assert!(!is_empty_method_body(CTOR_SRC, NON_EMPTY_BODY));
    assert!(is_empty_method_body("[]", Span::new(0, 2)));
    assert!(is_empty_method_body("\"\"", Span::new(0, 2)));
    // Out-of-range span — must not panic, must not report empty.
    assert!(!is_empty_method_body("{}", Span::new(0, 5)));
}

// `constructor_next_provider` / `destructor_next_provider` — the
// constructor/destructor next-chain.

#[test]
fn constructor_next_provider_none_when_class_unknown() {
    let classes = map(vec![cls_with_ctor("::A", &[])]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_next_provider("::Nope", None, CTOR_SRC), None);
}

#[test]
fn constructor_next_provider_direct_superclass() {
    // `Sub`'s own `next` (no explicit target) resolves one past `Sub`
    // itself in `Sub`'s own MRO — its direct superclass `Base`.
    let classes = map(vec![
        cls_with_ctor("::Base", &[]),
        cls_with_ctor("::Sub", &["::Base"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(
        h.constructor_next_provider("::Sub", None, CTOR_SRC),
        Some("::Base")
    );
}

#[test]
fn constructor_next_provider_skips_ancestor_with_no_effective_constructor() {
    // `Mid` declares no constructor of its own (a pure pass-through), so
    // `Sub`'s `next` must skip past it and reach `Base`'s — the same
    // "effective provider" skip `constructor_provider` already performs,
    // just starting one class later.
    let classes = map(vec![
        cls_with_ctor("::Base", &[]),
        cls("::Mid", &["::Base"], &[], &[]),
        cls_with_ctor("::Sub", &["::Mid"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(
        h.constructor_next_provider("::Sub", None, CTOR_SRC),
        Some("::Base")
    );
}

#[test]
fn constructor_next_provider_skips_ancestor_with_empty_bodied_constructor() {
    // `Mid` declares an explicit but empty-bodied constructor — `TclOO`
    // treats that as "no constructor", so `next` from `Sub` must skip
    // past it too, exactly like `constructor_provider` does.
    let classes = map(vec![
        cls_with_ctor("::Base", &[]),
        cls_with_ctor_body("::Mid", &["::Base"], EMPTY_BODY),
        cls_with_ctor("::Sub", &["::Mid"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(
        h.constructor_next_provider("::Sub", None, CTOR_SRC),
        Some("::Base")
    );
}

#[test]
fn constructor_next_provider_none_when_chain_exhausted() {
    let classes = map(vec![cls_with_ctor("::Sub", &[])]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.constructor_next_provider("::Sub", None, CTOR_SRC), None);
}

#[test]
fn constructor_next_provider_nextto_explicit_target() {
    // `nextto Grandparent` jumps straight to the named class, skipping
    // `Base` even though `Base` also has an effective constructor.
    let classes = map(vec![
        cls_with_ctor("::Grandparent", &[]),
        cls_with_ctor("::Base", &["::Grandparent"]),
        cls_with_ctor("::Sub", &["::Base"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(
        h.constructor_next_provider("::Sub", Some("::Grandparent"), CTOR_SRC),
        Some("::Grandparent")
    );
}

/// A class with a real (non-empty) destructor.
fn cls_with_dtor(qname: &str, supers: &[&str]) -> ClassDef {
    let mut cd = cls(qname, supers, &[], &[]);
    cd.destructor = Some(MethodDef {
        name: "<destructor>".to_string(),
        params: Vec::new(),
        params_computed: false,
        formal_count: crate::signature_scan::formal_count::SourceFormalCount::Authored(
            tcl_dialect::ParameterGrammar::Tcl,
        ),
        name_span: span(),
        body_span: NON_EMPTY_BODY,
        kind: "destructor".to_string(),
        visibility: "public".to_string(),
        doc: String::new(),
        forward_target: None,
        is_self_method: false,
    });
    cd
}

#[test]
fn destructor_next_provider_direct_superclass() {
    let classes = map(vec![
        cls_with_dtor("::Base", &[]),
        cls_with_dtor("::Sub", &["::Base"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.destructor_next_provider("::Sub", None), Some("::Base"));
}

#[test]
fn destructor_next_provider_skips_ancestor_with_no_destructor() {
    let classes = map(vec![
        cls_with_dtor("::Base", &[]),
        cls("::Mid", &["::Base"], &[], &[]),
        cls_with_dtor("::Sub", &["::Mid"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.destructor_next_provider("::Sub", None), Some("::Base"));
}

#[test]
fn destructor_next_provider_none_when_chain_exhausted() {
    let classes = map(vec![cls_with_dtor("::Sub", &[])]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.destructor_next_provider("::Sub", None), None);
}

#[test]
fn all_implementations_lists_all_classes_with_method() {
    let classes = map(vec![
        cls("::A", &[], &[], &["m"]),
        cls("::B", &["::A"], &[], &["m"]),
    ]);
    let h = build_class_hierarchy(classes);
    let mut impls = h.all_implementations("m");
    impls.sort();
    assert_eq!(impls.len(), 2);
    // ``::A`` provides its own m.
    assert!(impls.contains(&("::A".to_string(), "::A".to_string())));
    // ``::B`` overrides; B's m provider is ::B.
    assert!(impls.contains(&("::B".to_string(), "::B".to_string())));
}

#[test]
fn bare_superclass_links_via_namespace_ancestry() {
    // A subclass in `::Ns::Sub` names its base bare (`Base`); the base
    // lives at `::Ns::Base`. Ancestry resolution links them so the
    // inherited method resolves.
    let classes = map(vec![
        cls("::Ns::Base", &[], &[], &["inherited"]),
        cls("::Ns::Sub", &["Base"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.mro_map["::Ns::Sub"], vec!["::Ns::Sub", "::Ns::Base"]);
    assert_eq!(
        h.method_target("::Ns::Sub", "inherited"),
        Some("::Ns::Base")
    );
}

#[test]
fn bare_superclass_links_via_unique_tail_across_namespaces() {
    // C Tcl 8.6/9.0/9.1 requires the actual import; a unique tail is not
    // a lookup owner. Analyse the exported/imported original source.
    let source = r"namespace eval ::SpiceGenTcl {
    namespace export Device
    oo::class create Device {
        method genSPICEString {} {return OK}
    }
}
rename auto_import savedPrelude
namespace import ::SpiceGenTcl::Device
oo::class create ::Core {superclass Device}
";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
    let h = build_class_hierarchy(analysis.all_classes);
    assert_eq!(
        h.method_target("::Core", "genSPICEString"),
        Some("::SpiceGenTcl::Device")
    );
}

#[test]
fn bare_superclass_unique_tail_without_import_remains_unresolved() {
    let classes = map(vec![
        cls("::SpiceGenTcl::Device", &[], &[], &["genSPICEString"]),
        cls("::Core", &["Device"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.method_target("::Core", "genSPICEString"), None);
}

#[test]
fn next_provider_walks_and_nextto_restarts() {
    // C -> B -> A, all define `m`.
    let classes = map(vec![
        cls("::A", &[], &[], &["m"]),
        cls("::B", &["::A"], &[], &["m"]),
        cls("::C", &["::B"], &[], &["m"]),
    ]);
    let h = build_class_hierarchy(classes);
    // `next` from C's m → B, then A, then exhausted.
    assert_eq!(h.next_provider("::C", "m", "::C", None), Some("::B"));
    assert_eq!(h.next_provider("::C", "m", "::B", None), Some("::A"));
    assert_eq!(h.next_provider("::C", "m", "::A", None), None);
    // `nextto ::A` jumps straight to A.
    assert_eq!(h.next_provider("::C", "m", "::C", Some("::A")), Some("::A"));
}

#[test]
fn next_provider_skips_classes_without_the_method() {
    // C -> B -> A; only C and A define `m`. `next` from C skips B → A.
    let classes = map(vec![
        cls("::A", &[], &[], &["m"]),
        cls("::B", &["::A"], &[], &[]),
        cls("::C", &["::B"], &[], &["m"]),
    ]);
    let h = build_class_hierarchy(classes);
    assert_eq!(h.next_provider("::C", "m", "::C", None), Some("::A"));
}

#[test]
fn ambiguous_bare_superclass_stays_unlinked() {
    // Two classes share the tail `Base`; a bare `superclass Base` from an
    // unrelated namespace must NOT be linked to either (no wrong guess).
    let classes = map(vec![
        cls("::A::Base", &[], &[], &["m"]),
        cls("::B::Base", &[], &[], &["m"]),
        cls("::C::Sub", &["Base"], &[], &[]),
    ]);
    let h = build_class_hierarchy(classes);
    // Sub's MRO contains only itself + the unresolved bare `Base` leaf;
    // the method does not resolve to either candidate.
    assert_eq!(h.method_target("::C::Sub", "m"), None);
}

// Class-name resolution follows C Tcl's one-hop rule (current ns, then
// global), never an ancestor walk.  `resolve_from` mirrors the analyser's
// real call.
fn resolve_from(name: &str, owner: &str, keys: &[&str]) -> Option<String> {
    let key_strings: Vec<String> = keys.iter().copied().map(String::from).collect();
    let known: std::collections::HashSet<String> = key_strings.iter().cloned().collect();
    let tail_index = build_tail_index(key_strings.iter());
    resolve_class_name(name, owner, |q| known.contains(q), &tail_index)
}

/// FP guard: a bare `superclass Base` in `::a::b::Sub` where `Base` exists
/// only at `::a::Base` (an *ancestor*, not the current ns or global) and the
/// tail is ambiguous must NOT link — real Tcl errors there, and an ancestor
/// walk would wrongly return `::a::Base`.
#[test]
fn superclass_resolution_abstains_on_ancestor_only_base() {
    let got = resolve_from(
        "Base",
        "::a::b::Sub",
        &["::a::Base", "::x::Base", "::a::b::Sub"],
    );
    assert_eq!(
        got, None,
        "must abstain, not climb to an ancestor namespace"
    );
}

/// TP: a base in the class's *own* namespace resolves one-hop.
#[test]
fn superclass_same_namespace_base_resolves() {
    let got = resolve_from("Base", "::a::Sub", &["::a::Base", "::a::Sub"]);
    assert_eq!(got.as_deref(), Some("::a::Base"));
}

/// TP: a global base resolves (current-ns miss falls through to global).
#[test]
fn superclass_global_base_resolves() {
    let got = resolve_from("Base", "::a::b::Sub", &["::Base", "::a::b::Sub"]);
    assert_eq!(got.as_deref(), Some("::Base"));
}

/// A globally unique tail supplies no retained import or caller receipt.
#[test]
fn superclass_cross_file_unique_tail_withdraws() {
    let got = resolve_from(
        "Device",
        "::spice::sub::Sub",
        &["::spice::Device", "::spice::sub::Sub"],
    );
    assert_eq!(got, None);
}

/// TN: an absolute `::`-qualified name is taken exactly.
#[test]
fn superclass_absolute_name_is_exact() {
    let got = resolve_from("::a::Base", "::x::Sub", &["::a::Base", "::x::Sub"]);
    assert_eq!(got.as_deref(), Some("::a::Base"));
}
