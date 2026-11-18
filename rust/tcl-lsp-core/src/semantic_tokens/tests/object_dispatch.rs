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

//! Method words of object receivers, with the table of `TclOO` dispatch patterns that must resolve or abstain.

use super::*;

#[test]
fn object_method_options_resolve_via_registry() {
    // The ngspice / ticklecharts pattern, end-to-end through
    // the CompilationUnit: `set chart [ticklecharts::chart new]` binds the
    // handle's class, so `$chart Xaxis -name … -type value …` resolves the
    // method and its declared options through the registry's object-class
    // model — the precise path, not the shape-based fallback.
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "package require ticklecharts\nset chart [ticklecharts::chart new]\n$chart Xaxis -name {v(anode), V} -type value -min 0.4 -splitLine {show True}\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let toks = decode_semantic(&full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    ));
    let ks: Vec<u32> = toks.iter().map(|&(_, _, _, k, _)| k).collect();
    // `Xaxis` resolved as a callable method.
    assert!(
        ks.contains(&(TokenKind::Function as u32)),
        "expected Xaxis as a Function token; got {ks:?}"
    );
    // `-name` / `-type` / `-min` / `-splitLine` are decorators.
    assert!(
        ks.iter()
            .filter(|&&k| k == TokenKind::Decorator as u32)
            .count()
            >= 4,
        "expected the four axis options as decorators; got {ks:?}"
    );
    // `-type value` is a closed-set member; `-min 0.4` a generic value.
    assert!(
        ks.contains(&(TokenKind::EnumMember as u32)),
        "expected -type's `value` as an EnumMember; got {ks:?}"
    );
    assert!(
        ks.contains(&(TokenKind::OptionValue as u32)),
        "expected -min's `0.4` as an OptionValue; got {ks:?}"
    );
}

#[test]
fn direct_constructor_dispatch_resolves_method() {
    // A direct `[Class new] method …` dispatch (no intermediate variable)
    // resolves the method and its options through the registry too.
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "[ticklecharts::chart new] Yaxis -name Y -max 10\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let toks = decode_semantic(&full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    ));
    let ks: Vec<u32> = toks.iter().map(|&(_, _, _, k, _)| k).collect();
    assert!(
        ks.contains(&(TokenKind::Decorator as u32))
            && ks.contains(&(TokenKind::OptionValue as u32)),
        "expected -name/-max decorators + values on direct dispatch; got {ks:?}"
    );
}

#[test]
fn collection_dispatch_resolves_user_configurable_method() {
    // End-to-end: a `Pins` dict is filled with `[Pin new]`
    // handles in one method and an element is dispatched with
    // `[dict get $Pins $pin] configure -node …` in another.  The receiver
    // resolves to the user `oo::configurable` class, so `configure` is a
    // method and `-node` (a declared property) an option — not the plain
    // strings they read as without collection-element + user-class
    // resolution.
    // Root registry calls because TclOO selects the receiver's private
    // namespace at runtime, where a relative command may be shadowed.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {pin} { ::dict append Pins $pin [::Pin new] }\n\
                     method cfg {pin node} { [::dict get $Pins $pin] configure -node $node }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // The dispatch line carries `dict` (a `defaultLibrary` Function); the
    // resolved *method* is a plain Function (no `defaultLibrary`), which is
    // the signal that distinguishes resolution from the built-in.
    let user_method_on_dispatch_line = |toks: &[(u32, u32, u32, u32, u32)]| {
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0)
    };
    // Without analysis: `configure` stays an unresolved string — only
    // `dict` (defaultLibrary) is a Function on the line.
    let plain = decode_semantic(&full_with_cu(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
    ));
    assert!(
        !user_method_on_dispatch_line(&plain),
        "without analysis, no user method resolves; got {plain:?}"
    );
    // With analysis: the dynamic dispatch resolves `configure` as a method.
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        user_method_on_dispatch_line(&toks),
        "`configure` on the retrieved Pin should resolve to a method; got {toks:?}"
    );
}

#[test]
fn dict_for_loop_var_dispatch_resolves() {
    // `dict for {k pin} $Pins {$pin configure …}` — iterating an object
    // collection binds `pin` to an element, so the loop-body dispatch
    // resolves the user method just like the `[dict get …]` retrieval
    // (the SpiceGenTcl `allNodes` / `floating` shape).
    // Root registry calls because TclOO selects the receiver's private
    // namespace at runtime, where a relative command may be shadowed.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { ::dict append Pins $p [::Pin new] }\n\
                     method dump {} { ::dict for {k pin} $Pins { ::puts [$pin configure -node] } }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    // `configure` on the loop var resolves to a user method (plain Function,
    // no `defaultLibrary`) on the `dump` method's line.
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0),
        "expected `configure` on the dict-for value var to resolve; got {toks:?}"
    );
}

/// A method call through each kind of receiver is coloured as a method on
/// the stated line. A `false` row pins a receiver that states no class.
#[test]
fn receiver_dispatch_colours_the_method_word() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    for (receiver, src, line, resolves) in [
        (
            "`my` inside a class body",
            "oo::class create C {\n    method helper {} {}\n    method run {} { my helper }\n}\n",
            2,
            true,
        ),
        // tclsh 9.0.4: an `oo::configurable` instance answers `configure`,
        // an `oo::class` instance `unknown method "configure"`. The
        // metaclass decides, not a declared property.
        (
            "`configure` on an `oo::configurable` instance",
            "oo::configurable create Point {}\nset pt [Point new]\n$pt configure -x 1\n",
            2,
            true,
        ),
        (
            "`configure` on a plain `oo::class` instance",
            "oo::class create Plain { method m {} {} }\nset p [Plain new]\n$p configure -x 1\n",
            2,
            false,
        ),
        (
            "snit `$self`",
            "package require snit\nsnit::type C {\n    method helper {} {}\n    method run {} { $self helper }\n}\n",
            3,
            true,
        ),
        (
            "snit component installed with a type",
            "package require snit\nsnit::widget verticalAxis { method draw {} {} }\nsnit::widget chart {\n    component axis\n    constructor {args} {\n      install axis using verticalAxis $win.a\n      $axis draw\n    }\n}\n",
            6,
            true,
        ),
        // snit(n): `installhull using TYPE` creates the implicit hull;
        // `installhull $win` names an existing widget and no type.
        (
            "snit hull installed with a type",
            "package require snit\nsnit::widget frameish { method draw {} {} }\nsnit::widgetadaptor chart {\n    constructor {args} {\n      installhull using frameish\n      $hull draw\n    }\n}\n",
            5,
            true,
        ),
        (
            "snit hull installed from an existing widget",
            "package require snit\nsnit::widget frameish { method draw {} {} }\nsnit::widgetadaptor chart {\n    constructor {args} {\n      installhull $win\n      $hull draw\n    }\n}\n",
            5,
            false,
        ),
        (
            "snit bare-word constructor with a computed name",
            "package require snit\nsnit::type Engine { method run {} {} }\nsnit::type Wrapper {\n    variable eng\n    constructor {} { set eng [Engine ${selfns}::e] }\n    method go {} { $eng run }\n}\n",
            5,
            true,
        ),
        (
            "result of a factory procedure",
            "oo::class create C { method mrun {} {} }\nproc make {} { return [C new] }\nset o [make]\n$o mrun\n",
            3,
            true,
        ),
        (
            "object passed as a procedure parameter",
            "oo::configurable create Pin { property node }\nproc connect {dev t} { $dev configure -node $t }\nset p [Pin new]\nconnect $p n1\n",
            1,
            true,
        ),
        (
            "handle bound by `set`",
            "oo::configurable create Pin { property node }\nset p [Pin new]\n$p configure -node 5\n",
            2,
            true,
        ),
    ] {
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert_eq!(
            toks.iter()
                .any(|&(l, _, _, k, m)| l == line && k == TokenKind::Method as u32 && m == 0),
            resolves,
            "{receiver}: {toks:?}"
        );
    }
}

#[test]
fn widget_method_tokens_follow_the_resolved_tk_lifecycle() {
    // `current` belongs to ttk::treeview only from Tk 9.1. The semantic
    // provider must consume the registry's filtered object-method table,
    // not the raw class descriptor that contains future methods.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;

    let src = "package require Tk\nttk::treeview .tree\n.tree current\n";
    for (dialect, expected) in [("tcl9.0", false), ("tcl9.1", true)] {
        let profile = tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile();
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let cu = CompilationUnit::build_for(src, registry, false);
        let analysis = Analyser::new().analyse(src, dialect);
        let tokens = decode_semantic(&full_with_cu_and_analysis(
            src,
            profile,
            registry,
            Some(&cu),
            Some(&analysis),
        ));
        assert_eq!(
            tokens
                .iter()
                .any(|&(line, _, _, kind, _)| { line == 2 && kind == TokenKind::Method as u32 }),
            expected,
            "ttk::treeview current semantic token under {dialect}: {tokens:?}"
        );
    }
}

/// A bare (namespace-less) `apply {{} {...}}` runs its body in a *fresh*
/// call frame in the global namespace by default — never the enclosing
/// method's object namespace — so `my helper` inside it is not actually a
/// call to the enclosing class's method at runtime (`my` isn't defined in
/// `::`); a class-definition-body scan would raise "invalid command name
/// my" here. The `enclosing_class` context must not leak into an
/// `apply`-lambda body the way it correctly persists into an ordinary
/// nested `if`/`foreach` body, or this gets painted as a resolved,
/// legitimate method call anyway — the same fresh-frame rule call-graph
/// namespace resolution, param traits and declarations apply.
#[test]
fn my_call_inside_apply_lambda_body_does_not_resolve() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "oo::class create C {\n\
                   \x20   method helper {} {}\n\
                   \x20   method run {} { apply {{} {my helper}} }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        !toks
            .iter()
            .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
        "a bare apply lambda's `my helper` must not resolve as the \
             enclosing class's method — it isn't one at runtime; got {toks:?}"
    );
}

#[test]
fn snit_typemethod_call_does_not_type_handle() {
    // `set x [Engine spawn]` is a *typemethod* call, not a construction —
    // its result is not an Engine, so `$x run` must NOT resolve (soundness:
    // the bare-constructor scan excludes declared typemethods).
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "snit::type Engine { method run {} {} \n\
                   \x20   typemethod spawn {} {} }\n\
                   set x [Engine spawn]\n\
                   $x run\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    // `$x run` on line 3 must stay unresolved (`run` not a Function token).
    assert!(
        !toks
            .iter()
            .any(|&(l, _, _, k, _)| l == 3 && k == TokenKind::Function as u32),
        "expected `$x run` on a typemethod result to abstain; got {toks:?}"
    );
}

#[test]
fn my_configure_property_options_resolve() {
    // `my configure -prop` inside an oo::configurable body colours the
    // property option too.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "oo::configurable create C {\n\
                   \x20   property node\n\
                   \x20   constructor {} { my configure -node 0 }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    // `configure` resolves (Function) and `-node` is a decorator on line 2.
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 2 && k == TokenKind::Method as u32 && m == 0),
        "expected `my configure` to resolve; got {toks:?}"
    );
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, _)| l == 2 && k == TokenKind::Decorator as u32),
        "expected `-node` property option; got {toks:?}"
    );
}

/// Expected outcome for a `tcloo_dispatch_cases` fixture row.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Expect {
    Resolve,
    Abstain,
}

/// The `TclOO` object-method dispatch fixture rows:
/// `(name, source, method word, 0-based dispatch line, expectation)`.
const TCLOO_DISPATCH_CASES: &[(&str, &str, &str, u32, Expect)] = &[
    // ---- statically determinable → resolve ----
    (
        "var_new",
        "oo::class create C { method mrun {} {} }\nset o [C new]\n$o mrun\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "direct_new",
        "oo::class create C { method mrun {} {} }\n[C new] mrun\n",
        "mrun",
        1,
        Resolve,
    ),
    (
        // TclOO's own same-object dispatch idiom — `self`
        // called with no argument returns the current object's own
        // command name, and dispatching through it (`[self] m`) reaches
        // the enclosing class exactly like `my m`.
        "bare_self_receiver",
        "oo::class create C {\n    method mrun {} {}\n    method call {} { [self] mrun }\n}\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        // The explicit spelling `self object` — documented as
        // equivalent to a bare `self`.
        "self_object_receiver",
        "oo::class create C {\n    method mrun {} {}\n    method call {} { [self object] mrun }\n}\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "configurable_property",
        "oo::configurable create C { property node }\nset o [C new]\n$o configure -node 1\n",
        "configure",
        2,
        Resolve,
    ),
    (
        "inherited_method",
        "oo::class create B { method base {} {} }\noo::class create D { superclass B }\nset o [D new]\n$o base\n",
        "base",
        3,
        Resolve,
    ),
    (
        "mixin_method",
        "oo::class create M { method mixm {} {} }\noo::class create C { mixin M }\nset o [C new]\n$o mixm\n",
        "mixm",
        3,
        Resolve,
    ),
    (
        "dict_collection",
        "oo::class create C { method mrun {} {} }\ndict set d k [C new]\n[dict get $d k] mrun\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "foreach_loopvar",
        "oo::class create C { method mrun {} {} }\nlappend objs [C new]\nforeach o $objs { $o mrun }\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        "interproc_param",
        "oo::class create C { method mrun {} {} }\nproc f {o} { $o mrun }\nset p [C new]\nf $p\n",
        "mrun",
        1,
        Resolve,
    ),
    (
        "proc_return",
        "oo::class create C { method mrun {} {} }\nproc make {} { return [C new] }\nset o [make]\n$o mrun\n",
        "mrun",
        3,
        Resolve,
    ),
    (
        "my_self_call",
        "oo::class create C {\n  method helper {} {}\n  method run {} { my helper }\n}\n",
        "helper",
        2,
        Resolve,
    ),
    (
        "snit_self_call",
        "snit::type C {\n  method helper {} {}\n  method run {} { $self helper }\n}\n",
        "helper",
        2,
        Resolve,
    ),
    (
        "itcl_this_call",
        "itcl::class C {\n  method helper {} {}\n  method run {} { $this helper }\n}\n",
        "helper",
        2,
        Resolve,
    ),
    (
        "snit_install_component",
        "snit::widget Ax { method draw {} {} }\nsnit::widget C {\n  constructor {} { install ax using Ax $win.a\n    $ax draw }\n}\n",
        "draw",
        3,
        Resolve,
    ),
    (
        "snit_bare_constructor",
        "snit::type Eng { method run {} {} }\nsnit::type C {\n  variable e\n  constructor {} { set e [Eng ${selfns}::x] }\n  method go {} { $e run }\n}\n",
        "run",
        4,
        Resolve,
    ),
    (
        "oo_define_added",
        "oo::class create C {}\noo::define C { method added {} {} }\nset o [C new]\n$o added\n",
        "added",
        3,
        Resolve,
    ),
    (
        "registry_class",
        "package require ticklecharts\nset c [ticklecharts::chart new]\n$c Xaxis -name x\n",
        "Xaxis",
        2,
        Resolve,
    ),
    (
        // Tk widget instance dispatch, bareword receiver:
        // `ttk::treeview .t` names a widget path exactly like a registry
        // naming factory (`struct::graph g`) — `.t instate …` resolves
        // through the same self-referential `object_class`.
        "widget_bareword",
        "package require Tk\nttk::treeview .t\n.t instate {selected} {}\n",
        "instate",
        2,
        Resolve,
    ),
    (
        // Tk widget instance dispatch, `$var`-captured constructor
        // return value (`set lb [listbox .l]`).
        "widget_var_captured",
        "package require Tk\nset lb [listbox .l]\n$lb curselection\n",
        "curselection",
        2,
        Resolve,
    ),
    // ---- genuinely dynamic → must abstain (soundness) ----
    (
        "introspection_class",
        "oo::class create C { method mrun {} {} }\nset o [C new]\nset cls [info object class $o]\n[$cls new] mrun\n",
        "mrun",
        3,
        Abstain,
    ),
    (
        "oo_copy",
        "oo::class create C { method mrun {} {} }\nset a [C new]\nset b [oo::copy $a]\n$b mrun\n",
        "mrun",
        3,
        Abstain,
    ),
    (
        "unknown_param",
        "proc f {o} { $o mrun }\n",
        "mrun",
        0,
        Abstain,
    ),
    (
        // `CLASS create NAME` — resolved via
        // `instance_classes` gated on `created_instance_commands`,
        // merged into the object-class map `insert_object_method_overrides`'s
        // bareword branch already reads (see `NamedInstanceMap`).
        "named_object",
        "oo::class create C { method mrun {} {} }\nC create obj\nobj mrun\n",
        "mrun",
        2,
        Resolve,
    ),
    (
        // The snit *named-constructor* shape: `$o` bound by `foo create
        // x` types as `foo` (the signature scan records snit types as
        // classes), so the dispatch resolves like any handle.
        "snit_named_object",
        "snit::type foo { method smeth {} {} }\nset o [foo create x]\n$o smeth\n",
        "smeth",
        2,
        Resolve,
    ),
];

/// Golden fixture for `TclOO` object-method dispatch resolution — validated
/// against the Tcler's-wiki pattern catalogue and a real corpus (tcllib,
/// tklib, `SpiceGenTcl`).  Two guarantees:
///
/// * **`Resolve`** — a statically-determinable dispatch colours its method a
///   callable, for every form the resolver supports.
/// * **`Abstain`** — a genuinely-dynamic dispatch (or a form we do not model)
///   leaves its method a plain string, never a *mis-highlighted* callable
///   (soundness guard: no false positives).
///
/// Adding a pattern here is how object-dispatch support is expanded and
/// measured; flip an `Abstain` to `Resolve` when a form becomes supported.
#[test]
fn tcloo_dispatch_pattern_fixture() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let cases = TCLOO_DISPATCH_CASES;
    let registry = release_reg("tcl9.0");
    let mut failures = Vec::new();
    for &(name, src, method, line, expect) in cases {
        let cu = CompilationUnit::build_for(src, &registry, false);
        let analysis = Analyser::new().analyse(src, "tcl9.0");
        let toks = decode_semantic(&full_with_cu_and_analysis(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
            &registry,
            Some(&cu),
            Some(&analysis),
        ));
        // Column of the method word on the dispatch line (word-boundary,
        // first occurrence — unambiguous in these snippets).  ASCII, so byte
        // == UTF-16 column.
        let src_line = src.lines().nth(line as usize).unwrap_or("");
        let mcol = src_line.match_indices(method).find_map(|(i, _)| {
            let before = src_line.as_bytes().get(i.wrapping_sub(1)).copied();
            let after = src_line.as_bytes().get(i + method.len()).copied();
            let boundary =
                |b: Option<u8>| b.is_none_or(|b| !b.is_ascii_alphanumeric() && b != b'_');
            (boundary(before) && boundary(after))
                .then(|| u32::try_from(i).expect("column fits u32"))
        });
        // The method resolves iff *its own* token is a callable `Function`.
        let resolved = mcol.is_some_and(|c| {
            toks.iter()
                .any(|&(l, tc, _, k, _)| l == line && tc == c && k == TokenKind::Method as u32)
        });
        let ok = match expect {
            Resolve => resolved,
            Abstain => !resolved,
        };
        if !ok {
            let want = if expect == Resolve {
                "resolve"
            } else {
                "abstain"
            };
            failures.push(format!(
                "  {name}: `{method}` expected to {want} but did not"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "TclOO dispatch fixture regressions:\n{}",
        failures.join("\n")
    );
}

#[test]
fn cross_file_constructor_dispatch_resolves() {
    // `[::Pin new] configure -node 5` where `Pin` is defined in another
    // file: the project's class cards supply the class, as the project
    // token query does. Without a provider the selector is not a method.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards;
    let library = Analyser::new().analyse(
        "oo::configurable create ::Pin { property node }\n",
        "tcl9.0",
    );
    let providers = [OriginalProjectClassCards::capture(&library).unwrap()];
    let user = "[::Pin new] configure -node 5\n";
    let caller = Analyser::new().analyse(user, "tcl9.0");
    let method = |project_classes| {
        let tokens = decode_semantic(&full_with_cu_and_facts(
            user,
            caller.resolved_profile().unwrap(),
            caller.resolved_registry().unwrap(),
            None,
            WorkspaceTokenFacts {
                classes: Some(library.class_hierarchy()),
                project_classes,
                analysis: Some(&caller),
                ..WorkspaceTokenFacts::default()
            },
        ));
        tokens
            .iter()
            .any(|&(l, c, _, k, m)| l == 0 && c == 12 && k == TokenKind::Method as u32 && m == 0)
    };
    assert!(
        method(Some(&providers[..])),
        "the provider's class resolves"
    );
    assert!(!method(None), "no provider, no method");
}

#[test]
fn dict_map_in_return_dispatch_resolves() {
    // `return [dict map {k v} $coll {$v method …}]` — the loop is nested in
    // a command substitution, so the IR never surfaces it as a loop; the
    // syntactic scan still binds `v` to the collection element (SpiceGenTcl
    // `getPinsNodes` / `getParams` shape).
    // Root registry calls because TclOO selects the receiver's private
    // namespace at runtime, where a relative command may be shadowed.
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = release_reg("tcl9.0");
    let src = "oo::configurable create Pin { property node }\n\
                   oo::class create Device {\n\
                     variable Pins\n\
                     method add {p} { ::dict append Pins $p [::Pin new] }\n\
                     method nodes {} { ::return [::dict map {k pin} $Pins {$pin configure -node}] }\n\
                   }\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        toks.iter()
            .any(|&(l, _, _, k, m)| l == 4 && k == TokenKind::Method as u32 && m == 0),
        "expected `configure` in the return-nested dict map to resolve; got {toks:?}"
    );
}

#[test]
fn malformed_braced_object_handle_does_not_resolve_a_method() {
    use tcl_compiler::analyser::Analyser;
    use tcl_compiler::compilation_unit::CompilationUnit;
    let registry = reg();
    let src = "oo::configurable create Pin { property node }\n\
                   set p [Pin new]\n\
                   ${p configure -node 5\n";
    let cu = CompilationUnit::build_for(src, &registry, false);
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    let toks = decode_semantic(&full_with_cu_and_analysis(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        &registry,
        Some(&cu),
        Some(&analysis),
    ));
    assert!(
        !toks
            .iter()
            .any(|&(l, _, _, k, _)| { l == 2 && k == TokenKind::Method as u32 }),
        "malformed `${{p` must not dispatch to Pin: {toks:?}"
    );
    assert_eq!(object_handle_name("${x}", None), Some("x"));
    assert_eq!(object_handle_name("$x", None), Some("x"));
    assert_eq!(object_handle_name("$arr(idx)", None), Some("arr(idx)"));
    assert_eq!(object_handle_name("$::a:::b", None), Some("::a:::b"));
    assert_eq!(object_handle_name("$foo:::", None), Some("foo:::"));
    assert_eq!(object_handle_name("${x", None), Some("{x"));
}
