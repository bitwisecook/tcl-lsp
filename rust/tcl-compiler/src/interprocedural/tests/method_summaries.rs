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

//! Method purity and instance-variable writes of `TclOO` method bodies.

use super::*;

// method-purity summary tests

#[test]
fn pure_getter_method_is_summarised_pure() {
    let ia = build(
        "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method get {} { ::return $n }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::get").expect("method summary");
    assert!(m.base.pure, "read-only getter should be pure: {:?}", m.base);
    assert!(m.writes_instance_vars.is_empty());
    assert_eq!(m.class_name, "::C");
    assert_eq!(m.method_kind, crate::ir::MethodKind::Method);
}

#[test]
fn instance_var_write_makes_method_impure() {
    let ia = build(
        "oo::class create C {\n\
             \x20   variable n\n\
             \x20   method bump {} { incr n }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::bump").expect("method summary");
    assert!(!m.base.pure, "instance-var write must be impure");
    assert!(
        m.writes_instance_vars.contains("n"),
        "writes_instance_vars: {:?}",
        m.writes_instance_vars
    );
}

#[test]
fn array_element_instance_var_write_makes_method_impure() {
    // `set counter(0) 1` writes the instance var `counter` (compared
    // on the base scalar name, not the raw element name).
    let ia = build(
        "oo::class create C {\n\
             \x20   variable counter\n\
             \x20   method bump {} { set counter(0) 1 }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::bump").expect("method summary");
    assert!(!m.base.pure);
    assert!(m.writes_instance_vars.contains("counter"));
}

#[test]
fn my_dispatch_makes_method_impure() {
    // `my <other>` is a dynamic self-dispatch — unresolvable
    // statically, so the method can't be proven pure.
    let ia = build(
        "oo::class create C {\n\
             \x20   method a {} { my b }\n\
             \x20   method b {} { ::return 1 }\n\
             }",
    );
    let a = fixture_method(&ia, "::C::a").expect("method a");
    assert!(!a.base.pure, "my-dispatch caller must be impure");
    // The pure leaf method still summarises pure.
    let b = fixture_method(&ia, "::C::b").expect("method b");
    assert!(b.base.pure, "absolute literal return leaf: {:?}", b.base);
}

#[test]
fn redefined_method_joins_over_every_retained_body() {
    // The lowering retains replacement bodies, so the
    // summary is the JOIN over every body a dispatch may run —
    // pure when all bodies are pure, impure when any is.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { ::return 1 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { ::return 2 }\n\
             }",
    );
    let methods = ia
        .methods
        .values()
        .filter(|method| method.base.qualified_name == "::C::m")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    for m in methods {
        assert!(
            m.base.pure,
            "two pure bodies join to pure — abstention on the mere fact of \
             redefinition is the imprecision issue #1166 removes"
        );
        // Disagreeing constants join to no-constant-return.
        assert!(m.base.constant_return.is_none());
    }

    // FN-now-caught: an impure REPLACEMENT makes the join impure even
    // though the first (retained-in-`methods`) body is pure.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { puts side-effect\n\
             \x20       return 2 }\n\
             }",
    );
    let methods = ia
        .methods
        .values()
        .filter(|method| method.base.qualified_name == "::C::m")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    for m in methods {
        assert!(!m.base.pure, "an impure replacement body must join impure");
    }

    // Agreeing constants survive the join.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { ::return 7 }\n\
             }\n\
             oo::define C {\n\
             \x20   method m {} { ::return 7 }\n\
             }",
    );
    let methods = ia
        .methods
        .values()
        .filter(|method| method.base.qualified_name == "::C::m")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    for m in methods {
        assert!(m.base.pure);
        assert!(matches!(
            m.base.constant_return,
            Some(ConstantReturn::Int(7))
        ));
    }
}

#[test]
fn unreadable_oo_definitions_force_methods_impure() {
    // A dynamic member name may have redefined ANY method of the
    // class with an unknown body — every method of the class joins
    // impure.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             \x20   method $n {} { return 2 }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::m").expect("method summary");
    assert!(!m.base.pure, "unanalysable class member must widen");

    // A dynamic class word may have touched any class — module-wide.
    let ia = build(
        "oo::class create C {\n\
             \x20   method m {} { return 1 }\n\
             }\n\
             oo::define $cls { method m {} { puts hi } }",
    );
    let m = fixture_method(&ia, "::C::m").expect("method summary");
    assert!(!m.base.pure, "dynamic OO definition target must widen");
}

#[test]
fn variable_decl_is_not_counted_as_instance_var_write() {
    // A method-local `variable x` is a link/declaration, not a
    // write — its def must not land in `writes_instance_vars`
    // (soundness fix: a read-only instance-var method isn't
    // forced impure by *that* path).  Note: the registry models
    // `variable` itself as a scope-alias barrier, so `peek` is still
    // impure overall — but not via the instance-var-write channel.
    let ia = build(
        "oo::class create C {\n\
             \x20   method peek {} { variable x; return $x }\n\
             }",
    );
    let m = fixture_method(&ia, "::C::peek").expect("method summary");
    assert!(
        m.writes_instance_vars.is_empty(),
        "variable decl wrongly counted as write: {:?}",
        m.writes_instance_vars
    );
}

#[test]
fn instance_write_candidates_keep_literal_destinations_and_dynamic_unknowns() {
    let ivars: HashSet<_> = ["", "$field", "field", "${braced}", "$array"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let mut writes = HashSet::new();
    for name in ["", "$field", "${braced}", "$array(key)"] {
        check_ivar_write(name, &ivars, &mut writes);
    }
    assert_eq!(
        writes,
        HashSet::from_iter(
            ["", "$field", "${braced}", "$array"]
                .into_iter()
                .map(str::to_owned)
        )
    );
    assert!(!writes.contains("field"));
    let dynamic = crate::ir::Script::from_statements(vec![crate::ir::Statement::AssignConst {
        span: tcl_lexer::Span::new(0, 0),
        name: "$target".into(),
        name_braced: false,
        value: "1".into(),
        value_span: None,
    }]);
    collect_instance_var_writes(&dynamic, &ivars, &mut writes, 0);
    assert_eq!(writes, ivars);
}

/// `collect_instance_var_writes`
/// (the `TclOO` method-body instance-write scan) recurses once per
/// nested `if` body, so it needs a depth cap of its own.
/// Same transitively-bounded-today caveat and big-stack-thread
/// rationale as `deeply_nested_if_survives_interprocedural_scan`. 1000
/// levels is comfortably past the new cap; the assertion is that the
/// method summary is built at all, not what it contains.
#[test]
fn deeply_nested_if_survives_collect_instance_var_writes() {
    const DEPTH: usize = 1000;
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let mut src = "oo::class create C {\n    variable n\n    method bump {} {\n".to_owned();
    for _ in 0..DEPTH {
        src.push_str("if {1} {\n");
    }
    src.push_str("incr n\n");
    for _ in 0..DEPTH {
        src.push_str("}\n");
    }
    src.push_str("}\n}\n");
    std::thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            let ia = build(&src);
            assert!(
                ia.methods
                    .values()
                    .any(|method| method.base.qualified_name == "::C::bump")
            );
        })
        .unwrap()
        .join()
        .unwrap();
}
