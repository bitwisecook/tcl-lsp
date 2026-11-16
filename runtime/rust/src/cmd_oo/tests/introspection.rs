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

//! `info object` and `info class`: arity and option tables, `isa`, method and variable listings, call chains, creation ids and release gates.

use super::*;

/// TclOO's four option tables — `info object isa`'s `category`,
/// `configure`'s `property` (which C reaches through `tcl::prefix match
/// -message property`), `definitionnamespace`'s `kind`, and `method`'s
/// `export flag` — resolve by unambiguous prefix through the shared
/// option-table matcher, matching C's `Tcl_GetIndexFromObj`/`tcl::prefix
/// match` semantics rather than an exact match against a hand-joined
/// enumeration.
///
/// tclsh 9.0.4 (8.6.16 agrees on the `category` rows):
///   info object isa x o   -> bad category "x": must be class, metaclass,
///                            mixin, object, or typeof
///   info object isa {} o  -> ambiguous category "": must be <same>
///   info object isa m o   -> ambiguous category "m": must be <same>
///   info object isa cl o  -> 0  ; ob o -> 1 ; me o -> 0
///   q configure -zz       -> bad property "-zz": must be -yellow or -zed
///   q configure {}        -> ambiguous property "": must be <same>
///   q configure -ye 1     -> {}   ;  q configure -ye -> 1
///   oo::define D definitionnamespace {} ::foo
///                         -> ambiguous kind "": must be -class or -instance
///   info class definitionnamespace D -x
///                         -> bad kind "-x": must be -class or -instance
///   oo::define D {method n -x {} {}}
///                         -> bad export flag "-x": must be -export,
///                            -private, or -unexport
/// The other half of `info object isa`'s dispatch: once the category
/// resolves, C checks an **exact** argument count per category
/// (`InfoObjectIsACmd`, tclOOInfo.c) — two trailing words for
/// `class`/`metaclass`/`object`, three for `mixin`/`typeof`. Resolving the
/// category by prefix without that second stage answers `0` for
/// `info object isa t $obj` (a missing class name) and `1` for
/// `info object isa ob $obj extra` (a word C refuses).
///
/// The noun is C's `Tcl_WrongNumArgs(interp, 2, objv, …)`, so it carries
/// the *resolved* category word: an index-typed argument prints as its
/// table entry, which is why `isa t o` reports `typeof`. The
/// two-words-missing case is checked before the category is resolved at
/// all, so `info object isa object` is the generic message.
///
/// tclsh 8.6.16 and 9.0.4, byte-identical:
///   info object isa                -> wrong # args: should be "info
///                                     object isa category objName ?arg ...?"
///   info object isa object         -> <same>
///   info object isa t o            -> wrong # args: should be "info
///                                     object isa typeof objName className"
///   info object isa ob o extra     -> wrong # args: should be "info
///                                     object isa object objName"
///   info object isa cl o extra     -> … "info object isa class objName"
///   info object isa metaclass o e  -> … "info object isa metaclass objName"
///   info object isa mi o           -> … "info object isa mixin objName className"
///   info object isa mixin o C x    -> <same>
///   info object isa ty o C x       -> … "info object isa typeof objName className"
#[test]
fn info_object_isa_enforces_the_per_category_arity() {
    leak_free(|i| {
        let err_of = |i: &mut Interp, src: &[u8]| {
            assert_eq!(
                i.eval_str(src),
                Code::Error,
                "expected an error for {:?}",
                String::from_utf8_lossy(src)
            );
            String::from_utf8_lossy(&i.result_bytes()).into_owned()
        };
        ok(i, b"oo::class create C");
        ok(i, b"C create o");
        const GENERIC: &str =
            "wrong # args: should be \"info object isa category objName ?arg ...?\"";
        assert_eq!(err_of(i, b"info object isa"), GENERIC);
        assert_eq!(err_of(i, b"info object isa object"), GENERIC);
        for (src, want) in [
            (
                b"info object isa t o".as_slice(),
                "typeof objName className",
            ),
            (
                b"info object isa mi o".as_slice(),
                "mixin objName className",
            ),
            (
                b"info object isa mixin o C extra".as_slice(),
                "mixin objName className",
            ),
            (
                b"info object isa ty o C extra".as_slice(),
                "typeof objName className",
            ),
            (b"info object isa ob o extra".as_slice(), "object objName"),
            (b"info object isa cl o extra".as_slice(), "class objName"),
            (
                b"info object isa metaclass o extra".as_slice(),
                "metaclass objName",
            ),
        ] {
            assert_eq!(
                err_of(i, src),
                format!("wrong # args: should be \"info object isa {want}\"")
            );
        }
        // The exact counts still answer.
        assert_eq!(ok(i, b"info object isa object nosuchobj"), b"0");
        assert_eq!(ok(i, b"info object isa typeof o C"), b"1");
        assert_eq!(ok(i, b"info object isa mixin o C"), b"0");
    });
}

#[test]
fn oo_option_tables_resolve_like_tcl_get_index_from_obj() {
    const CATEGORY: &str = "must be class, metaclass, mixin, object, or typeof";
    const KIND: &str = "must be -class or -instance";
    leak_free(|i| {
        let err_of = |i: &mut Interp, src: &[u8]| {
            assert_eq!(
                i.eval_str(src),
                Code::Error,
                "expected an error for {:?}",
                String::from_utf8_lossy(src)
            );
            String::from_utf8_lossy(&i.result_bytes()).into_owned()
        };
        ok(i, b"oo::class create C");
        ok(i, b"oo::define C method m {} {return 1}");
        ok(i, b"C create o");
        assert_eq!(
            err_of(i, b"info object isa x o"),
            format!("bad category \"x\": {CATEGORY}")
        );
        assert_eq!(
            err_of(i, b"info object isa {} o"),
            format!("ambiguous category \"\": {CATEGORY}")
        );
        assert_eq!(
            err_of(i, b"info object isa m o"),
            format!("ambiguous category \"m\": {CATEGORY}")
        );
        assert_eq!(ok(i, b"info object isa cl o"), b"0");
        assert_eq!(ok(i, b"info object isa ob o"), b"1");
        assert_eq!(ok(i, b"info object isa me o"), b"0");
        // The method list keeps TclOO's own join, which drops the Oxford
        // comma the option tables above keep.
        assert_eq!(
            err_of(i, b"o x"),
            "unknown method \"x\": must be destroy or m"
        );
        ok(i, b"oo::class create C3");
        ok(
            i,
            b"oo::define C3 {method a {} {}; method b {} {}; method c {} {}}",
        );
        ok(i, b"C3 create o3");
        assert_eq!(
            err_of(i, b"o3 zz"),
            "unknown method \"zz\": must be a, b, c or destroy"
        );
        // `definitionnamespace`'s kind, both spellings.
        ok(i, b"oo::class create D");
        assert_eq!(
            err_of(i, b"oo::define D definitionnamespace {} ::foo"),
            format!("ambiguous kind \"\": {KIND}")
        );
        assert_eq!(
            err_of(i, b"info class definitionnamespace D -x"),
            format!("bad kind \"-x\": {KIND}")
        );
        assert_eq!(ok(i, b"info class definitionnamespace D -i"), b"");
        // `method`'s TIP 500 export flag.
        assert_eq!(
            err_of(i, b"oo::define D {method n -x {} {}}"),
            "bad export flag \"-x\": must be -export, -private, or -unexport"
        );
        ok(i, b"oo::define D {method n -e {} {}}");
        ok(i, b"oo::define D {method n2 -p {} {}}");
        i.eval_str(b"C destroy; D destroy");
    });
}

#[test]
fn private_variable_introspection() {
    leak_free(|i| {
        // TIP 500 (oo-38.2): `private variable` is tracked separately from
        // the public declared variables; `-private` lists it.
        ok(i, b"oo::class create parent");
        ok(
                i,
                b"oo::class create cls { superclass parent; private { variable x1; variable x2 }; variable y1 y2 }",
            );
        ok(i, b"cls create obj");
        ok(
            i,
            b"oo::objdefine obj { private variable a1 a2; variable b1 b2 }",
        );
        assert_eq!(ok(i, b"lsort [info class variables cls]"), b"y1 y2");
        assert_eq!(
            ok(i, b"lsort [info class variables cls -private]"),
            b"x1 x2"
        );
        assert_eq!(ok(i, b"lsort [info object variables obj]"), b"b1 b2");
        assert_eq!(
            ok(i, b"lsort [info object variables obj -private]"),
            b"a1 a2"
        );
    });
}

#[test]
fn info_methods_all_and_object_vars_pattern() {
    leak_free(|i| {
        // `info object vars obj ?pattern?` filters by the glob (oo-16.10).
        ok(i, b"oo::class create K");
        ok(i, b"K create foo");
        ok(i, b"oo::objdefine foo export eval");
        ok(i, b"foo eval {variable c 3 a 1 b 2 ddd 4}");
        assert_eq!(ok(i, b"lsort [info object vars foo ?]"), b"a b c");
        // `info class methods -all` includes destroy; `-all -private` adds
        // the oo::object built-ins (oo-17.9).
        ok(i, b"oo::class create C { method bar {} {} }");
        assert_eq!(ok(i, b"lsort [info class methods C -all]"), b"bar destroy");
        assert_eq!(
            ok(i, b"lsort [info class methods C -all -private]"),
            b"<cloned> bar destroy eval unknown variable varname"
        );
        // Unexporting destroy hides it from `-all` (oo-17.10).
        ok(i, b"oo::define C unexport {*}[info class methods C -all]");
        assert_eq!(ok(i, b"info class methods C -all"), b"");
    });
}

#[test]
fn info_methods_scope_and_export_reclassifies() {
    leak_free(|i| {
        ok(i, b"oo::class create cls { private method foo {} {} }");
        assert_eq!(ok(i, b"info class methods cls -scope public"), b"");
        assert_eq!(ok(i, b"info class methods cls -scope unexported"), b"");
        assert_eq!(ok(i, b"info class methods cls -scope private"), b"foo");
        // export reclassifies private -> public.
        ok(i, b"oo::define cls { export foo }");
        assert_eq!(ok(i, b"info class methods cls -scope public"), b"foo");
        assert_eq!(ok(i, b"info class methods cls -scope private"), b"");
        // unexport reclassifies private -> unexported.
        ok(i, b"oo::class create cls2 { private method bar {} {} }");
        ok(i, b"oo::define cls2 { unexport bar }");
        assert_eq!(ok(i, b"info class methods cls2 -scope unexported"), b"bar");
        assert_eq!(ok(i, b"info class methods cls2 -scope private"), b"");
    });
}

#[test]
fn isa_follows_import_and_unknown_list_excludes_bare_export() {
    leak_free(|i| {
        // info object isa follows a namespace-import alias to the object.
        ok(
                i,
                b"namespace eval foo { namespace eval bar { oo::object create o; namespace export o }; namespace import bar::o }",
            );
        assert_eq!(ok(i, b"info object isa object foo::o"), b"1");
        assert_eq!(ok(i, b"info object isa object foo::bar::o"), b"1");
        // An `export`ed name with no implementation is not in the unknown list.
        ok(i, b"oo::class create tc");
        ok(i, b"oo::objdefine tc export Bad");
        assert_eq!(i.eval_str(b"tc Bad"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown method \"Bad\": must be create, destroy or new"
        );
    });
}

#[test]
fn info_call_chain_rendering() {
    leak_free(|i| {
        ok(i, b"oo::class create root");
        // declarer is `object` for a per-object method, the class FQN else.
        ok(
            i,
            b"oo::class create ::A { superclass root; method x {} {} }",
        );
        ok(i, b"A create y");
        ok(i, b"oo::objdefine y method x {} {}");
        assert_eq!(
            ok(i, b"info object call y x"),
            b"{method x object method} {method x ::A method}"
        );
        // info class call follows the full precedence, mixins first.
        ok(i, b"oo::class create ::B { superclass A; method x {} {} }");
        ok(i, b"oo::class create ::C { superclass A; method x {} {} }");
        ok(
            i,
            b"oo::class create ::D { superclass C; mixin B; method x {} {} }",
        );
        assert_eq!(
                ok(i, b"info class call D x"),
                b"{method x ::B method} {method x ::D method} {method x ::C method} {method x ::A method}"
            );
        // A missing method renders the unknown chain, ending at the core
        // oo::object handler with a `core method:` type.
        ok(
            i,
            b"oo::class create ::U { superclass root; method unknown args {} }",
        );
        ok(i, b"U create u");
        assert_eq!(
                ok(i, b"info object call u nosuch"),
                b"{unknown unknown ::U method} {unknown unknown ::oo::object {core method: \"unknown\"}}"
            );
    });
}

#[test]
fn isa_typeof_traverses_mixins_and_subclass_pattern() {
    leak_free(|i| {
        // `isa typeof` follows mixin links, not just the class MRO (oo-16.9).
        ok(i, b"oo::class create Ac");
        ok(i, b"oo::class create Bc { superclass Ac }");
        ok(i, b"oo::class create Cc { superclass Bc }");
        ok(i, b"oo::class create Dc { mixin Cc }");
        ok(i, b"Dc create F");
        assert_eq!(ok(i, b"info object isa typeof F Bc"), b"1");
        assert_eq!(ok(i, b"info object isa typeof F Cc"), b"1");
        assert_eq!(ok(i, b"info object isa typeof F oo::class"), b"0");
        // `info class subclasses class ?pattern?` filters by the FQN glob.
        assert_eq!(ok(i, b"lsort [info class subclasses Ac]"), b"::Bc");
        assert_eq!(ok(i, b"info class subclasses Ac ::C*"), b"");
    });
}

#[test]
fn object_creationid_and_definitionnamespace() {
    leak_free(|i| {
        ok(i, b"oo::class create cls");
        // Distinct objects get distinct creation IDs.
        let a = ok(i, b"info object creationid [cls new]");
        let b = ok(i, b"info object creationid [cls new]");
        assert_ne!(a, b);
        // The ID is stable across rename.
        ok(
            i,
            b"set o [cls new]; set id [info object creationid $o]; rename $o gorp",
        );
        assert_eq!(ok(i, b"info object creationid gorp"), ok(i, b"set id"));
        // TIP 524 definition-namespace introspection (built-in defaults).
        assert_eq!(
            ok(i, b"info class definitionnamespace oo::object -instance"),
            b"::oo::objdefine"
        );
        assert_eq!(ok(i, b"info class definitionnamespace oo::object"), b"");
        assert_eq!(
            ok(i, b"info class definitionnamespace oo::class -class"),
            b"::oo::define"
        );
        assert_eq!(
            ok(i, b"info class definitionnamespace oo::class -instance"),
            b""
        );
        // A non-object reports an error (without surrounding quotes).
        assert_eq!(i.eval_str(b"info object creationid nosuch"), Code::Error);
    });
}

#[test]
fn info_oo_ensemble_resolution_and_release_gates_match_tcl() {
    leak_free(|i| {
        use tcl_dialect::TclVersion;

        i.set_runtime_version(TclVersion::V9_0);
        ok(i, b"oo::class create C; set o [C new]");
        assert_eq!(ok(i, b"info object cl $o"), b"::C");
        assert_eq!(i.eval_str(b"info object bogus $o"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"bogus\": must be call, class, creationid, definition, filters, forward, isa, methods, methodtype, mixins, namespace, properties, variables, or vars"
            );
        assert_eq!(i.eval_str(b"info class def C nope"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"unknown or ambiguous subcommand \"def\": must be call, constructor, definition, definitionnamespace, destructor, filters, forward, instances, methods, methodtype, mixins, properties, subclasses, superclasses, or variables"
            );
        assert_eq!(ok(i, b"info class definitionnamespace C"), b"");

        i.set_runtime_version(TclVersion::V8_6);
        for (script, expected) in [
                (
                    &b"info object creationid $o"[..],
                    &b"unknown or ambiguous subcommand \"creationid\": must be call, class, definition, filters, forward, isa, methods, methodtype, mixins, namespace, variables, or vars"[..],
                ),
                (
                    &b"info object properties $o"[..],
                    &b"unknown or ambiguous subcommand \"properties\": must be call, class, definition, filters, forward, isa, methods, methodtype, mixins, namespace, variables, or vars"[..],
                ),
                (
                    &b"info class definitionnamespace C"[..],
                    &b"unknown or ambiguous subcommand \"definitionnamespace\": must be call, constructor, definition, destructor, filters, forward, instances, methods, methodtype, mixins, subclasses, superclasses, or variables"[..],
                ),
                (
                    &b"info class properties C"[..],
                    &b"unknown or ambiguous subcommand \"properties\": must be call, constructor, definition, destructor, filters, forward, instances, methods, methodtype, mixins, subclasses, superclasses, or variables"[..],
                ),
            ] {
                assert_eq!(i.eval_str(script), Code::Error);
                assert_eq!(i.result_bytes(), expected);
            }
        // With the 9.0-only definitionnamespace row absent, `def` is once
        // again the unique prefix of `definition`.
        assert_eq!(i.eval_str(b"info class def C nope"), Code::Error);
        assert_eq!(i.result_bytes(), b"unknown method \"nope\"");
    });
}

#[test]
fn info_class_introspection_and_abbrev() {
    leak_free(|i| {
        ok(
                i,
                b"oo::class create C { method foo {a b} {return x}; forward fwd ::tcl::mathop::+ 1; filter foo }",
            );
        assert_eq!(ok(i, b"info class methodtype C foo"), b"method");
        assert_eq!(ok(i, b"info class methodtype C fwd"), b"forward");
        assert_eq!(ok(i, b"info class definition C foo"), b"{a b} {return x}");
        assert_eq!(ok(i, b"info class forward C fwd"), b"::tcl::mathop::+ 1");
        assert_eq!(ok(i, b"info class filters C"), b"foo");
        // Abbreviated subcommand (unique prefix).
        assert_eq!(ok(i, b"info class methodt C foo"), b"method");
        // Ambiguous / unknown subcommand errors.
        assert_eq!(i.eval_str(b"info class gorp C"), Code::Error);
        assert_eq!(i.eval_str(b"info class methodtype C nope"), Code::Error);
        // `forward` on a non-forward method.
        assert_eq!(i.eval_str(b"info class forward C foo"), Code::Error);
    });
}

#[test]
fn info_call_and_self_call() {
    leak_free(|i| {
        ok(i, b"oo::class create A { method foo {} {} }");
        ok(
            i,
            b"oo::class create B { superclass A; method foo {} { next } }",
        );
        assert_eq!(
            ok(i, b"info class call B foo"),
            b"{method foo ::B method} {method foo ::A method}"
        );
        // `self call` reports the live chain + index; a private method shows
        // as `private` and is visible to a same-object `[self]` dispatch.
        ok(
            i,
            b"oo::class create P { method chain {} { return [self call] } }",
        );
        ok(
                i,
                b"oo::class create Q { superclass P; private method chain {} { next }; method viapub {} { [self] chain } }",
            );
        ok(i, b"set q [Q new]");
        assert_eq!(
            ok(i, b"$q viapub"),
            b"{{private chain ::Q method} {method chain ::P method}} 1"
        );
        // An external call from outside the object skips Q's private `chain`
        // and runs P's public one.
        assert_eq!(ok(i, b"$q chain"), b"{{method chain ::P method}} 0");
    });
}
