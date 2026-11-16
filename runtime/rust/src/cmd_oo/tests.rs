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

use crate::interp::{obj_bytes, Code, Interp, Param};
use crate::{counters, list, obj};

fn leak_free(body: impl FnOnce(&mut Interp)) {
    counters::reset();
    {
        let mut interp = Interp::new();
        body(&mut interp);
    }
    assert_eq!(counters::finalize(), 0, "residual objs/bufs");
    assert_eq!(counters::double_free_count(), 0);
}

fn ok(i: &mut Interp, src: &[u8]) -> Vec<u8> {
    assert_eq!(
        i.eval_str(src),
        Code::Ok,
        "eval {:?} -> {:?}",
        String::from_utf8_lossy(src),
        String::from_utf8_lossy(&i.result_bytes())
    );
    i.result_bytes()
}

/// Whether `haystack` contains the byte subsequence `needle`.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

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

/// The `property` half of the option-table prefix matching, which needs
/// a configurable class.
#[test]
fn configure_property_word_abbreviates() {
    const MUST: &str = "must be -yellow or -zed";
    leak_free(|i| {
        let err_of = |i: &mut Interp, src: &[u8]| {
            assert_eq!(i.eval_str(src), Code::Error, "expected an error");
            String::from_utf8_lossy(&i.result_bytes()).into_owned()
        };
        ok(
            i,
            b"oo::configurable create Q { property yellow zed\n\
                   constructor {} { my configure -yellow 0 -zed 0 } }",
        );
        ok(i, b"Q create q");
        assert_eq!(
            err_of(i, b"q configure -zz"),
            format!("bad property \"-zz\": {MUST}")
        );
        assert_eq!(
            err_of(i, b"q configure {}"),
            format!("ambiguous property \"\": {MUST}")
        );
        assert_eq!(
            err_of(i, b"q configure -"),
            format!("ambiguous property \"-\": {MUST}")
        );
        // A unique prefix resolves — and writes the *canonical* property.
        ok(i, b"q configure -ye 1");
        assert_eq!(ok(i, b"q configure -yellow"), b"1");
        assert_eq!(ok(i, b"q configure -ye"), b"1");
        i.eval_str(b"Q destroy");
    });
}

#[test]
fn filters_wrap_method_calls() {
    leak_free(|i| {
        ok(i, b"set ::r {}");
        ok(i, b"oo::class create C");
        ok(
            i,
            b"oo::define C method foo args {global r; lappend r foo; return done}",
        );
        ok(i, b"oo::define C method log args {global r; lappend r [self method]; return [next {*}$args]}");
        ok(i, b"oo::define C filter log");
        ok(i, b"C create o");
        // The filter runs first (self method = the filter name), then `next`
        // reaches the real method.
        assert_eq!(ok(i, b"o foo"), b"done");
        assert_eq!(ok(i, b"set ::r"), b"log foo");
        i.eval_str(b"C destroy; unset -nocomplain ::r");
    });
}

#[test]
fn class_methods_via_self() {
    leak_free(|i| {
        // `oo::define … self method` defines a method on the class object.
        ok(i, b"oo::class create C");
        ok(
            i,
            b"oo::define C self method greet {} {return \"hi from [self]\"}",
        );
        assert_eq!(ok(i, b"C greet"), b"hi from ::C");
        // `self method` inside the class body, and `self` alone returns it.
        assert_eq!(
            ok(
                i,
                b"oo::class create D {self method who {} {return [self]}}; D who"
            ),
            b"::D"
        );
        // Instance methods still resolve (the class-as-object regression guard).
        ok(i, b"oo::class create E {method m {} {return inst}}");
        assert_eq!(ok(i, b"E create e; e m"), b"inst");
        i.eval_str(b"C destroy; D destroy; E destroy");
    });
}

#[test]
fn rename_to_empty_frees_object_and_class_names() {
    leak_free(|i| {
        // Deleting an object's command (rename to {}) frees the name so it
        // can be recreated — the OO registry tracks the command's lifetime.
        ok(i, b"oo::object create foo");
        ok(i, b"rename foo {}");
        assert_eq!(ok(i, b"oo::object create foo"), b"::foo");
        // Same for a class (registered in both the object and class maps).
        ok(i, b"oo::class create C");
        ok(i, b"rename C {}");
        assert_eq!(ok(i, b"oo::class create C"), b"::C");
        // A renamed object follows its command to the new name.
        ok(i, b"oo::object create a; rename a b");
        assert_eq!(ok(i, b"info object isa object b"), b"1");
        i.eval_str(b"rename foo {}; rename C {}; rename b {}");
    });
}

#[test]
fn precedence_ordering_c3_and_mixins() {
    leak_free(|i| {
        // oo-21.2: diamond + mixin precedence. Expected call order
        // `Fmix Emix o D B C A` (shared bases deferred via keep-last).
        ok(
            i,
            b"oo::class create A { method m {} {lappend ::result A} }",
        );
        ok(
            i,
            b"oo::class create B { superclass A; method m {} {lappend ::result B;next} }",
        );
        ok(
            i,
            b"oo::class create C { superclass A; method m {} {lappend ::result C;next} }",
        );
        ok(
            i,
            b"oo::class create D { superclass B C; method m {} {lappend ::result D;next} }",
        );
        ok(
            i,
            b"oo::class create Emix { superclass C; method m {} {lappend ::result Emix;next} }",
        );
        ok(
            i,
            b"oo::class create Fmix { superclass Emix; method m {} {lappend ::result Fmix;next} }",
        );
        ok(i, b"D create o");
        ok(
            i,
            b"oo::objdefine o { method m {} {lappend ::result o;next}; mixin Fmix }",
        );
        ok(i, b"set ::result {}; o m");
        assert_eq!(ok(i, b"set ::result"), b"Fmix Emix o D B C A");
        // A class mixin runs before the class itself (oo-14.8 ordering):
        // the mixin's method runs first and chains via `next` to the class.
        ok(
            i,
            b"oo::class create mix { method t {} {lappend ::r mix; next} }",
        );
        ok(
            i,
            b"oo::class create cls { mixin mix; method t {} {lappend ::r cls; return $::r} }",
        );
        ok(i, b"set ::r {}");
        assert_eq!(ok(i, b"[cls new] t"), b"mix cls");
        // A filter method defined by several providers wraps once per
        // implementation, in precedence order (oo-21.3): F.flt then G.flt.
        ok(
            i,
            b"oo::class create G { method flt {} {lappend ::r G-flt; next} }",
        );
        ok(i, b"oo::class create F { superclass G; method flt {} {lappend ::r F-flt; next}; method go {} {lappend ::r go} }");
        ok(i, b"F create gf");
        ok(i, b"oo::objdefine gf filter flt");
        ok(i, b"set ::r {}; gf go");
        assert_eq!(ok(i, b"set ::r"), b"F-flt G-flt go");
    });
}

#[test]
fn tip558_configurable_configure() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        ok(
            i,
            b"oo::configurable create Point { superclass parent; property x y; \
                  constructor args { my configure -x 0 -y 0 {*}$args } }",
        );
        ok(i, b"set pt [Point new -x 3]");
        assert_eq!(ok(i, b"$pt configure -x"), b"3");
        assert_eq!(ok(i, b"$pt configure -y"), b"0");
        ok(i, b"$pt configure -y 4");
        assert_eq!(ok(i, b"$pt configure"), b"-x 3 -y 4");
        assert_eq!(i.eval_str(b"$pt configure gorp"), Code::Error);
        assert_eq!(i.result_bytes(), b"bad property \"gorp\": must be -x or -y");
        assert_eq!(i.eval_str(b"$pt configure -x 1 -y"), Code::Error);
        // per-object property declaration (oo-2.3).
        ok(i, b"oo::objdefine $pt property z");
        ok(i, b"$pt configure -z 5");
        assert_eq!(ok(i, b"$pt configure -z"), b"5");
        // -kind controls direction.
        ok(
            i,
            b"oo::configurable create RO { superclass parent; property r -kind readable; \
                  constructor {} { my variable r; set r 7 } }",
        );
        ok(i, b"RO create ro");
        assert_eq!(ok(i, b"ro configure -r"), b"7");
        assert_eq!(i.eval_str(b"ro configure -r 9"), Code::Error);
        assert_eq!(i.result_bytes(), b"property \"-r\" is read only");
        // Error codes (ooProp-4.x).
        ok(i, b"oo::configurable create E { superclass parent }");
        assert_eq!(i.eval_str(b"oo::define E {property -x}"), Code::Error);
        assert_eq!(ok(i, b"set ::errorCode"), b"TCL OO PROPERTY_FORMAT");
        assert_eq!(
            i.eval_str(b"oo::define E {property q -kind gorp}"),
            Code::Error
        );
        assert_eq!(ok(i, b"set ::errorCode"), b"TCL LOOKUP INDEX kind gorp");
    });
}

#[test]
fn tip558_property_custom_accessors_and_kinds() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        // Custom -get/-set bodies; option/kind prefix abbreviation.
        ok(
            i,
            b"oo::configurable create Point { superclass parent; variable xyz; \
                  property x -g {return [list $xyz $xyz]} -s {set xyz $value} }",
        );
        ok(i, b"Point create p");
        ok(i, b"p configure -x 5");
        assert_eq!(ok(i, b"p configure -x"), b"5 5");
        // A getter doing break/continue is reported, not leaked as a loop error.
        ok(
                i,
                b"oo::configurable create B { superclass parent; property y -get {return -code break} }",
            );
        ok(i, b"B create b");
        assert_eq!(i.eval_str(b"b configure -y"), Code::Error);
        assert_eq!(i.result_bytes(), b"property getter for -y did a break");
        // A second declaration with a different -kind replaces membership.
        ok(
                i,
                b"oo::configurable create R { superclass parent; variable z; \
                  property z -kind readable -get {return $z}; property z -kind writable -set {set z $value} }",
            );
        ok(i, b"R create r");
        ok(i, b"r configure -z ok");
        assert_eq!(i.eval_str(b"r configure -z"), Code::Error);
        assert_eq!(i.result_bytes(), b"property \"-z\" is write only");
    });
}

#[test]
fn tip558_property_and_info_options_match_tcl_prefix_diagnostics() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        assert_eq!(
            i.eval_str(b"oo::configurable create P {property x -}"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"ambiguous option \"-\": must be -get, -kind, or -set"
        );
        assert_eq!(
            i.eval_str(b"oo::configurable create Q {property x -kind r}"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"ambiguous kind \"r\": must be readable, readwrite, or writable"
        );

        ok(
            i,
            b"oo::configurable create C {superclass parent; property x}",
        );
        ok(i, b"C create o");
        assert_eq!(ok(i, b"info class properties C -a"), b"-x");
        assert_eq!(ok(i, b"info object properties o -a"), b"-x");
        assert_eq!(i.eval_str(b"info class properties C {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"ambiguous option \"\": must be -all, -readable, or -writable"
        );
        assert_eq!(i.eval_str(b"info object properties o -"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"ambiguous option \"-\": must be -all, -readable, or -writable"
        );
    });
}

#[test]
fn tip558_property_storage_and_introspection() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        ok(i, b"oo::class create c {superclass parent}");
        // Empty to start; the slot stores uniqued, info sorts (oo-1.1).
        assert_eq!(ok(i, b"info class properties c"), b"");
        ok(
            i,
            b"oo::define c ::oo::configuresupport::readableproperties -set f e d",
        );
        assert_eq!(ok(i, b"info class properties c"), b"d e f");
        assert_eq!(ok(i, b"info class properties c -writable"), b"");
        ok(
            i,
            b"oo::define c ::oo::configuresupport::readableproperties -set a a a",
        );
        assert_eq!(ok(i, b"info class properties c"), b"a");
        // Writable slot is independent.
        ok(
            i,
            b"oo::define c ::oo::configuresupport::writableproperties -set w2 w1",
        );
        assert_eq!(ok(i, b"info class properties c -writable"), b"w1 w2");
        // -all merges the superclass chain (oo-1.5).
        ok(i, b"oo::class create d {superclass c}");
        ok(
            i,
            b"oo::define d ::oo::configuresupport::readableproperties -set x y z",
        );
        assert_eq!(ok(i, b"info class properties d -all"), b"a x y z");
        // Object properties + -all merges object, mixins, class chain (oo-1.9).
        ok(i, b"d create o");
        ok(
            i,
            b"oo::objdefine o ::oo::configuresupport::objreadableproperties -set m n",
        );
        assert_eq!(ok(i, b"info object properties o"), b"m n");
        assert_eq!(ok(i, b"info object properties o -all"), b"a m n x y z");
    });
}

#[test]
fn define_context_is_frame_scoped() {
    leak_free(|i| {
        // A user proc in ::oo::define is reachable as a definition command,
        // and runs with the definition context suspended (frame-scoped): its
        // `variable` is the ordinary command, and `uplevel 1` re-enters the
        // definition body's context (oo-36.9, oo-43.x).
        ok(i, b"oo::class create C");
        // A non-command in a define body routes through the global `unknown`
        // proc, whose `variable ::tcl::UnknownPending` must resolve to the
        // ordinary command (the define context is suspended in that nested
        // frame) — yielding the proper "invalid command name", not a spurious
        // declared-variable error.
        assert_eq!(i.eval_str(b"oo::define C nonesuch"), Code::Error);
        assert!(
            contains(&i.result_bytes(), b"invalid command name \"nonesuch\""),
            "got {:?}",
            i.result_bytes()
        );
        // A user proc in ::oo::define is reachable as a definition command;
        // direct `self` errors at the proc level, `uplevel 1 self` re-enters
        // the definition body's context and yields the class.
        ok(
                i,
                b"proc ::oo::objdefine::probe {} { list [catch {::oo::objdefine::self} m] [catch {uplevel 1 self} m2] $m2 }",
            );
        assert_eq!(ok(i, b"oo::objdefine C probe"), b"1 0 ::C");
    });
}

#[test]
fn namespaced_define_command_context_error() {
    leak_free(|i| {
        // Outside a definition the namespaced define subcommands error.
        assert_eq!(i.eval_str(b"oo::define::private error xyz"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"this command may only be called from within the context of an \
                  ::oo::define or ::oo::objdefine command"
        );
        assert_eq!(i.eval_str(b"oo::objdefine::method m {} {}"), Code::Error);
        // Inside a definition they dispatch normally.
        ok(i, b"oo::class create C");
        ok(i, b"oo::define C { ::oo::define::method m {} {return ok} }");
        ok(i, b"C create o");
        assert_eq!(ok(i, b"o m"), b"ok");
    });
}

#[test]
fn private_variable_cross_class_mangling() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        // clsA/clsB private x, clsC public x — three independent storages
        // (oo-38.1).
        ok(
            i,
            b"oo::class create clsA { superclass parent; private variable x; \
                  constructor {} {set x 1}; method getA {} {return $x} }",
        );
        ok(
            i,
            b"oo::class create clsB { superclass clsA; private variable x; \
                  constructor {} {set x 2; next}; method getB {} {return $x} }",
        );
        ok(
            i,
            b"oo::class create clsC { superclass clsB; variable x; \
                  constructor {} {set x 3; next}; method getC {} {return $x} }",
        );
        ok(i, b"clsC create o");
        assert_eq!(ok(i, b"o getA"), b"1");
        assert_eq!(ok(i, b"o getB"), b"2");
        assert_eq!(ok(i, b"o getC"), b"3");
        // `my varname` of a private var returns its mangled storage (oo-38.3).
        ok(
            i,
            b"oo::class create V { superclass parent; private variable p; \
                  method name {} {my varname p} }",
        );
        ok(i, b"V create v");
        let n = ok(i, b"v name");
        assert!(n.ends_with(b" : p"), "mangled storage, got {n:?}");
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
fn object_deleted_in_constructor() {
    leak_free(|i| {
        // Destroying the object in its constructor makes `new`/`create` fail
        // (oo-30.1/30.2, Bug 2903011), via `[self] destroy` or `my destroy`.
        ok(
            i,
            b"oo::class create cls { constructor {} {[self] destroy} }",
        );
        assert_eq!(i.eval_str(b"cls new"), Code::Error);
        assert_eq!(i.result_bytes(), b"object deleted in constructor");
        ok(i, b"oo::class create cls2 { constructor {} {my destroy} }");
        assert_eq!(i.eval_str(b"cls2 create foo"), Code::Error);
        assert_eq!(i.result_bytes(), b"object deleted in constructor");
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
fn destructor_error_goes_to_bgerror() {
    leak_free(|i| {
        // A destructor erroring during implicit teardown (`rename {}`) is
        // queued as a background error and reported to the `interp bgerror`
        // handler at `update`, not propagated (oo-3.7/3.8).
        ok(
                i,
                b"interp bgerror {} [list apply {{var msg args} {upvar #0 $var v; lappend v $msg}} ::caught]",
            );
        ok(i, b"oo::class create cls { destructor {error boom} }");
        ok(i, b"set ::caught {}; cls create obj");
        ok(i, b"rename obj {}");
        ok(i, b"update idletasks");
        assert_eq!(ok(i, b"set ::caught"), b"boom");
        // An empty cmdPrefix is rejected (C's `ChildBgerror`: length >= 1),
        // so the handler is reset by overwriting it with a one-element list.
        ok(i, b"interp bgerror {} noop");
    });
}

#[test]
fn oo_copy_clones_namespace_contents() {
    leak_free(|i| {
        // oo::copy copies the instance namespace's procs (re-pointed to the
        // copy's namespace) and variable values (oo-15.6/15.8).
        ok(i, b"oo::class create C { export eval }");
        ok(i, b"C create a");
        ok(
            i,
            b"a eval {variable y 0; proc foo {} {variable y; incr y}}",
        );
        assert_eq!(ok(i, b"a eval foo"), b"1"); // a.y = 1
        ok(i, b"oo::copy a b");
        // b inherits y=1 and its own foo (operating on b's y).
        assert_eq!(ok(i, b"b eval foo"), b"2"); // b.y 1->2
        assert_eq!(ok(i, b"b eval foo"), b"3"); // b.y 2->3
                                                // a's y is untouched by b's foo.
        assert_eq!(ok(i, b"a eval {set y}"), b"1");
        assert_eq!(ok(i, b"a eval foo"), b"2"); // a.y 1->2
    });
}

#[test]
fn my_command_invoked_directly() {
    leak_free(|i| {
        // The per-object `my`, invoked directly (not from a method), still
        // dispatches on its own object (oo-16.13).
        ok(i, b"oo::object create foo");
        ok(i, b"oo::objdefine foo method Bar {} {return {ok in foo}}");
        assert_eq!(ok(i, b"[info object namespace foo]::my Bar"), b"ok in foo");
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
fn nextto_resumes_chain_at_class() {
    leak_free(|i| {
        ok(i, b"oo::class create root");
        ok(
            i,
            b"oo::class create A { superclass root; method x args {lappend ::r ==A==} }",
        );
        ok(
            i,
            b"oo::class create B { superclass A; method x args {lappend ::r ==B==; nextto A} }",
        );
        ok(
            i,
            b"oo::class create C { superclass A; method x args {lappend ::r ==C==} }",
        );
        ok(
            i,
            b"oo::class create D { superclass B C; method x args {lappend ::r ==D==; next} }",
        );
        ok(i, b"set ::r {}; [D new] x");
        // D -> B (next) -> B does `nextto A`, skipping C, to A.
        assert_eq!(ok(i, b"set ::r"), b"==D== ==B== ==A==");
        // nextto to an unrelated/behind class errors.
        ok(i, b"oo::class create Z { superclass root }");
        ok(
            i,
            b"oo::class create E { superclass root; method x {} {nextto Z} }",
        );
        ok(i, b"E create e");
        assert_eq!(i.eval_str(b"e x"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"method has no non-filter implementation by \"Z\""
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
fn mixin_of_mixin_methods_are_reachable() {
    leak_free(|i| {
        // Bug 1960703 (oo-14.6): a method from a mixin's own mixin is
        // reachable via the call chain.
        ok(i, b"oo::class create parent");
        ok(
            i,
            b"oo::class create A { superclass parent; method egg {} {return chicken} }",
        );
        ok(
            i,
            b"oo::class create B { superclass parent; mixin A; method bar {} {my egg} }",
        );
        ok(
            i,
            b"oo::class create C { superclass parent; mixin B; method foo {} {my bar} }",
        );
        assert_eq!(ok(i, b"[C new] foo"), b"chicken");
    });
}

#[test]
fn my_variable_rejects_array_element() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create C { method bar {} { my variable a(b) } }",
        );
        ok(i, b"C create o");
        assert_eq!(i.eval_str(b"o bar"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't define \"a(b)\": name refers to an element in an array"
        );
    });
}

#[test]
fn variable_declaration_introspection_retains_original_counted_object() {
    // Native proof naming.tcloo.original-counted-variable-header:
    // docs/design/analysis/name-resolution-proofs/tcloo-original-counted-variable-header.md
    // Direct declaration/query retains the original counted input object;
    // byte-equal reconstructed values cannot supply this header identity.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"oo::class create C"), Code::Ok);
        let original = obj::Owned::fresh(obj::new_string_bytes(b"a\0z"));
        let mut declaration = vec![
            obj::Owned::fresh(obj::new_string_bytes(b"oo::define")),
            obj::Owned::fresh(obj::new_string_bytes(b"C")),
            obj::Owned::fresh(obj::new_string_bytes(b"variable")),
            original.clone(),
        ];
        let words: Vec<_> = declaration.iter().map(obj::Owned::as_ptr).collect();
        assert_eq!(interp.dispatch(&words), Code::Ok);
        declaration.clear();
        let query = [
            obj::Owned::fresh(obj::new_string_bytes(b"info")),
            obj::Owned::fresh(obj::new_string_bytes(b"class")),
            obj::Owned::fresh(obj::new_string_bytes(b"variables")),
            obj::Owned::fresh(obj::new_string_bytes(b"C")),
        ];
        let words: Vec<_> = query.iter().map(obj::Owned::as_ptr).collect();
        assert_eq!(interp.dispatch(&words), Code::Ok);
        let listed = list::list_elements(interp.result_obj()).unwrap();
        assert_eq!(listed, vec![original.as_ptr()], "{version}");
        assert_eq!(obj_bytes(listed[0]), b"a\0z", "{version}");
    }
}

#[test]
fn unknown_method_preserves_original_selector_for_returned_slot_data() {
    // naming.tcloo.original-counted-variable-header
    // docs/design/analysis/name-resolution-proofs/tcloo-original-counted-variable-header.md
    // This Rust control isolates the dispatcher before the native proof's
    // direct declaration/query boundary; it claims no extra native observer.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(version),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        assert_eq!(interp.eval_str(b"oo::object create O; oo::objdefine O method unknown {requested args} {return $requested}"), Code::Ok);
        let selector = obj::Owned::fresh(obj::new_string_bytes(b"a\0z"));
        let head = obj::Owned::fresh(obj::new_string_bytes(b"O"));
        assert_eq!(
            interp.dispatch(&[head.as_ptr(), selector.as_ptr()]),
            Code::Ok,
            "{version}"
        );
        assert!(
            interp.result_obj() == selector.as_ptr(),
            "{version}: original selector header"
        );
        assert_eq!(interp.result_bytes(), b"a\0z", "{version}");
    }
}

#[test]
fn variable_slot_operations() {
    leak_free(|i| {
        // `variable` accumulates by default (-append), uniquely.
        ok(i, b"oo::class create A { variable a; variable b }");
        assert_eq!(ok(i, b"lsort [info class variables A]"), b"a b");
        // `-clear` empties; a following declaration starts fresh (oo-27.16).
        ok(
            i,
            b"oo::class create B { variable x; variable -clear; variable y }",
        );
        assert_eq!(ok(i, b"info class variables B"), b"y");
        // `-set` replaces (oo-27.17).
        ok(i, b"oo::class create C { variable x; variable -set y }");
        assert_eq!(ok(i, b"info class variables C"), b"y");
        // An unknown slot op errors with the C unknown-method message
        // (oo-27.18).
        assert_eq!(
            i.eval_str(b"oo::class create D { variable -? y }"),
            Code::Error
        );
        assert_eq!(
                i.result_bytes(),
                b"unknown method \"-?\": must be -append, -appendifnew, -clear, -prepend, -remove or -set"
            );
    });
}

#[test]
fn define_context_self_arity() {
    // naming.tcloo.original-definition-context-self
    // docs/design/analysis/name-resolution-proofs/tcloo-original-definition-context-self.md
    leak_free(|i| {
        ok(i, b"oo::class create Cls");
        ok(i, b"Cls create obj");
        // `oo::define`'s `self` accepts class-side directives; `self` alone
        // returns the class.
        assert_eq!(ok(i, b"oo::define Cls {set ::r [self]}; set ::r"), b"::Cls");
        ok(
            i,
            b"oo::define Cls { self { method cm {} {return classside} } }",
        );
        assert_eq!(ok(i, b"Cls cm"), b"classside");
        // `oo::objdefine`'s `self` takes no arguments (oo-36.8).
        assert_eq!(ok(i, b"oo::objdefine obj {self}"), b"::obj");
        assert_eq!(
            i.eval_str(b"oo::objdefine obj {self anything}"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"wrong # args: should be \"self\"");
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
fn class_method_constructor_instance_var() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create Animal {
                    variable sound
                    constructor {s} { set sound $s }
                    method speak {} { return \"I say $sound\" }
                    method describe {} { return \"[self] speaks\" }
                }",
        );
        ok(i, b"set a [Animal new woof]");
        assert_eq!(ok(i, b"$a speak"), b"I say woof");
        ok(i, b"Animal create ::bessie moo");
        assert_eq!(ok(i, b"bessie speak"), b"I say moo");
        ok(i, b"bessie destroy");
        assert_eq!(i.eval_str(b"bessie speak"), Code::Error);
    });
}

// Needs the numeric tower: `$obj eval` runs `expr`.
#[cfg(have_tommath)]
#[test]
fn object_builtin_variable_varname_eval() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create C {
                    method s {v} { my variable x; set x $v }
                    method g {} { my variable x; return $x }
                    method vn {} { my varname x }
                    method e {} { my eval {set y 9; return $y} }
                }",
        );
        ok(i, b"set o [C new]");
        ok(i, b"$o s hi");
        assert_eq!(ok(i, b"$o g"), b"hi");
        let mut variable = ok(i, b"info object namespace $o");
        variable.extend_from_slice(b"::x");
        assert_eq!(ok(i, b"$o vn"), variable);
        assert_eq!(ok(i, b"$o e"), b"9");
        // Built-ins are unexported: an external call is an unknown method.
        assert_eq!(i.eval_str(b"$o variable x"), Code::Error);
        // ... but `export` promotes a built-in to a public method.
        ok(i, b"oo::class create E { export eval }; set e [E new]");
        assert_eq!(ok(i, b"$e eval {expr 3+4}"), b"7");
        // `variable` rejects a namespace-qualified name.
        assert_eq!(
            i.eval_str(b"oo::class create D {method z {} {my variable a::b}}; [D new] z"),
            Code::Error,
        );
    });
}

#[test]
fn classvariable_shares_state_per_declaring_class() {
    leak_free(|i| {
        // A class variable is shared across all instances of the class, and
        // lives in the class object's namespace.
        ok(
            i,
            b"oo::class create Counter {
                    constructor {} { classvariable count; incr count }
                    method count {} { classvariable count; return $count }
                }",
        );
        ok(i, b"Counter create a; Counter create b; Counter create c");
        assert_eq!(ok(i, b"a count"), b"3");
        assert_eq!(ok(i, b"b count"), b"3");
        // It really lives in the class's own namespace.
        assert_eq!(ok(i, b"set [info object namespace Counter]::count"), b"3");
        // A method reached via `next` links to *its own* declaring class, not
        // the leaf object's — so Base and Derived keep separate stores.
        ok(
            i,
            b"oo::class create Base {
                    method tag {v} { classvariable t; set t $v }
                    method get {} { classvariable t; return $t }
                }",
        );
        ok(
            i,
            b"oo::class create Sub {
                    superclass Base
                    method tag {v} { classvariable t; set t sub-$v; next $v }
                }",
        );
        ok(i, b"Sub create s; s tag hi");
        assert_eq!(ok(i, b"set [info object namespace Base]::t"), b"hi");
        assert_eq!(ok(i, b"set [info object namespace Sub]::t"), b"sub-hi");
        assert_eq!(ok(i, b"s get"), b"hi");
    });
}

#[test]
fn classvariable_context_and_name_errors() {
    leak_free(|i| {
        // Outside a method it is unusable (helpers are call-stack-gated).
        assert_eq!(i.eval_str(b"classvariable foo"), Code::Error);
        assert!(contains(
            &i.result_bytes(),
            b"classvariable may only be called from inside a method"
        ));
        // A per-object / class-side method has no class namespace to share.
        ok(
            i,
            b"oo::object create solo; oo::objdefine solo {method m {} {classvariable x}}",
        );
        assert_eq!(i.eval_str(b"solo m"), Code::Error);
        assert_eq!(i.result_bytes(), b"method not defined by a class");
        // Name validation mirrors C: array look-alike, then namespace sep.
        ok(
            i,
            b"oo::class create C {
                    method arr {} { classvariable a(b) }
                    method ns {} { classvariable ::foo }
                    method none {} { classvariable }
                }",
        );
        ok(i, b"C create o");
        assert_eq!(i.eval_str(b"o arr"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"bad variable name \"a(b)\": can't create a scalar variable that looks like an array element"
            );
        assert_eq!(i.eval_str(b"o ns"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"bad variable name \"::foo\": can't create a local variable with a namespace separator in it"
            );
        assert_eq!(i.eval_str(b"o none"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"classvariable name ...\""
        );
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
fn define_subcommand_abbrev_and_extras() {
    leak_free(|i| {
        // Abbreviated definition subcommands (unique prefixes).
        ok(i, b"oo::class create A");
        ok(i, b"oo::class create C { meth foo {} {return X} }");
        assert_eq!(ok(i, b"[C new] foo"), b"X");
        ok(i, b"oo::define C { super A }");
        assert_eq!(ok(i, b"info class superclasses C"), b"::A");
        // An ambiguous prefix is not a define subcommand.
        assert_eq!(i.eval_str(b"oo::define C { m foo {} {} }"), Code::Error);
        // deletemethod / renamemethod.
        ok(
            i,
            b"oo::class create D { method foo {} {return F}; method bar {} {return B} }",
        );
        ok(i, b"oo::define D deletemethod foo");
        assert_eq!(i.eval_str(b"[D new] foo"), Code::Error);
        ok(i, b"oo::define D renamemethod bar baz");
        assert_eq!(ok(i, b"[D new] baz"), b"B");
        assert_eq!(i.eval_str(b"oo::define D deletemethod nope"), Code::Error);
        // objdefine class — change an object's class.
        ok(i, b"oo::class create P { method m {} {return P} }");
        ok(i, b"oo::class create Q { method m {} {return Q} }");
        ok(i, b"set o [P new]");
        ok(i, b"oo::objdefine $o class Q");
        assert_eq!(ok(i, b"$o m"), b"Q");
    });
}

#[test]
fn method_definition_retains_original_default_objects_without_rendering() {
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        leak_free(|interp| {
            interp.set_dialect_profile(crate::environment::profile_for_dialect(dialect));
            assert_eq!(interp.eval_str(b"oo::class create C"), Code::Ok);
            let default = obj::Owned::fresh(obj::new_wide_int_obj(17));
            let parameters = [Param {
                name: b"x".to_vec(),
                default: Some(default.clone()),
            }];
            let procedure = interp.create_native_callable_from_chosen(
                parameters.to_vec(),
                obj::Owned::fresh(obj::new_string_bytes(b"return $x")),
                crate::namespace::GLOBAL,
                None,
                0,
            );
            let definition =
                obj::Owned::fresh(super::list_procedure_body(interp, &procedure.declaration()));
            assert!(!obj::has_string_rep(definition.as_ptr()));
            let elements = list::native_list_backing(definition.as_ptr()).unwrap();
            let formals = list::native_list_backing(elements.elements().unwrap()[0]).unwrap();
            let parameter = list::native_list_backing(formals.elements().unwrap()[0]).unwrap();
            assert_eq!(parameter.elements().unwrap()[1], default.as_ptr());
            assert!(!obj::has_string_rep(default.as_ptr()));
            interp.set_result(definition.as_ptr());
            assert_eq!(interp.result_bytes(), b"{{x 17}} {return $x}");
        });
    }
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

#[test]
fn metaclass_instantiation() {
    leak_free(|i| {
        // A metaclass's instances are themselves classes.
        ok(i, b"oo::class create meta { superclass oo::class }");
        ok(i, b"meta create instance1");
        ok(i, b"instance1 create instance2");
        assert_eq!(ok(i, b"info object class instance1"), b"::meta");
        assert_eq!(ok(i, b"info object class instance2"), b"::instance1");
        assert_eq!(ok(i, b"info object isa class instance1"), b"1");
        assert_eq!(ok(i, b"info object isa metaclass meta"), b"1");
        assert_eq!(ok(i, b"info object isa metaclass instance1"), b"0");
        assert_eq!(ok(i, b"info object isa object instance2"), b"1");
        // A metaclass instance can carry a definition-script body.
        ok(i, b"set c [meta create C { method foo {} { return MF } }]");
        assert_eq!(ok(i, b"[C new] foo"), b"MF");
        // Destroying the metaclass cascades to its class-instances (both maps
        // freed), so the names can be recreated.
        ok(i, b"meta destroy");
        assert_eq!(i.eval_str(b"instance1 create x"), Code::Error);
        ok(i, b"oo::class create instance1");
    });
}

#[test]
fn oo_slot_base_class() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create SampleSlot {\n\
                    superclass oo::Slot\n\
                    constructor {} { variable contents {a b c} }\n\
                    method contents {} { variable contents; return $contents }\n\
                    method Get {} { variable contents; return $contents }\n\
                    method Set {lst} { variable contents $lst; return }\n\
                    method Resolve {x} { return $x }\n\
                }",
        );
        ok(i, b"SampleSlot create s");
        ok(i, b"s -append g h");
        assert_eq!(ok(i, b"s contents"), b"a b c g h");
        ok(i, b"s -set d e");
        assert_eq!(ok(i, b"s contents"), b"d e");
        ok(i, b"s -prepend x");
        assert_eq!(ok(i, b"s contents"), b"x d e");
        ok(i, b"s -remove d");
        assert_eq!(ok(i, b"s contents"), b"x e");
        ok(i, b"s -clear");
        assert_eq!(ok(i, b"s contents"), b"");
        // Defaulting: a bare value goes through `unknown` → -append.
        ok(i, b"s p q");
        assert_eq!(ok(i, b"s contents"), b"p q");
    });
}

#[test]
fn object_namespace_publication_uses_selected_relative_byte_geometry() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        leak_free(|i| {
            i.set_runtime_version(version);
            ok(
                i,
                b"namespace eval ::outer {oo::object create src; oo::copy src dst ns::::child}",
            );
            assert_eq!(
                ok(i, b"info object namespace ::outer::dst"),
                b"::outer::ns::child"
            );
            assert_eq!(
                i.eval_str(b"namespace eval ::outer {oo::copy src another ns::::child}"),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"ns::::child refers to an existing namespace"
            );
            ok(i, b"namespace eval ::outer {oo::class create C; oo::objdefine C export createWithNamespace; C createWithNamespace named allocated::::child}");
            assert_eq!(
                ok(i, b"info object namespace ::outer::named"),
                b"::outer::allocated::child"
            );
            assert_eq!(
                i.eval_str(
                    b"namespace eval ::outer {C createWithNamespace another allocated::::child}"
                ),
                Code::Error
            );
            assert_eq!(
                i.result_bytes(),
                b"can't create namespace \"allocated::::child\": already exists"
            );
        });
    }
}

#[test]
fn create_with_namespace_conflicts_use_the_shared_native_report() {
    for version in [
        tcl_dialect::TclVersion::V8_6,
        tcl_dialect::TclVersion::V9_0,
        tcl_dialect::TclVersion::V9_1,
    ] {
        for (seed, operand) in [
            (b"allocated".as_slice(), b"allocated".as_slice()),
            (b"allocated", b"::Outer::allocated"),
            (b"allocated", b"::::Outer::::allocated"),
            (b"allocated:tail", b"allocated:tail"),
        ] {
            let mut interpreter = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(version.dialect_profile_name()),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let source = [
                br#"namespace eval ::Outer {
    oo::class create C
    oo::objdefine C export createWithNamespace
    namespace eval "#
                    .as_slice(),
                seed,
                b" {}\nset operand ",
                operand,
                br#"
    set status [catch {C createWithNamespace object $operand} value]
    list $status $value $::errorCode [info commands object]
}"#,
            ]
            .concat();
            assert_eq!(interpreter.eval_str(&source), Code::Ok);
            let expected = [
                b"1 {can't create namespace \"".as_slice(),
                operand,
                b"\": already exists} {TCL OPERATION NAMESPACE CREATEEXISTING} {}",
            ]
            .concat();
            assert_eq!(interpreter.result_bytes(), expected, "{version:?}");
            assert!(interpreter.native_execution_refusal().is_none());
        }
    }
}

#[test]
fn definitionnamespace_semantics() {
    // naming.tcloo.original-definitionnamespace-source-control
    // docs/design/analysis/name-resolution-proofs/tcloo-original-definitionnamespace-source-control.md
    leak_free(|i| {
        // TIP 524: a metaclass's -class definition namespace becomes the
        // resolution scope when defining its instances (classes).
        ok(i, b"oo::class create parent");
        ok(
            i,
            b"namespace eval foodef { proc sparkle {} { return ok } }",
        );
        ok(
            i,
            b"oo::class create foocls { superclass oo::class parent; definitionnamespace foodef }",
        );
        ok(
            i,
            b"oo::class create foo { superclass parent; self class foocls }",
        );
        // `sparkle` (a proc in foodef) is now reachable in foo's definition.
        assert_eq!(ok(i, b"oo::define foo { sparkle }"), b"ok");
        // Round-trip introspection of an explicitly-set namespace.
        ok(i, b"namespace eval ::nd {}");
        ok(
            i,
            b"oo::class create D; oo::define D { definitionnamespace ::nd }",
        );
        assert_eq!(ok(i, b"info class definitionnamespace D"), b"::nd");
        assert_eq!(ok(i, b"info class definitionnamespace D -instance"), b"");
        // The root classes reject definition-namespace changes.
        assert_eq!(
            i.eval_str(b"oo::define oo::object { definitionnamespace ::nd }"),
            Code::Error,
        );
    });
}

#[test]
fn definitionnamespace_uses_original_namespace_object_in_each_outer_context() {
    for version in [tcl_dialect::TclVersion::V9_0, tcl_dialect::TclVersion::V9_1] {
        leak_free(|i| {
            i.set_runtime_version(version);
            ok(i, b"namespace eval ::dest {}; namespace eval ::outer {namespace eval dest {}; oo::class create C}; namespace eval ::other {namespace eval dest {}; oo::class create C}; set name dest");
            ok(
                i,
                b"namespace eval ::outer {oo::define C definitionnamespace $::name}",
            );
            assert_eq!(
                ok(i, b"info class definitionnamespace ::outer::C"),
                b"::outer::dest"
            );
            ok(i, b"set name");
            assert!(matches!(
                obj::native_object_snapshot(i.result_obj()).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::NamespaceName {
                    resolved: true,
                    ..
                }
            ));
            ok(
                i,
                b"namespace eval ::other {oo::define C definitionnamespace $::name}",
            );
            assert_eq!(
                ok(i, b"info class definitionnamespace ::other::C"),
                b"::other::dest"
            );
            ok(
                i,
                b"namespace eval ::outer {oo::define C definitionnamespace {}}",
            );
            assert_eq!(ok(i, b"info class definitionnamespace ::outer::C"), b"");
        });
    }
}

#[test]
fn destructors_fire_on_rename_destroy_and_namespace_delete() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create cls { destructor { lappend ::result died } }",
        );
        // `obj destroy`
        ok(i, b"set ::result {}; cls create a; a destroy");
        assert_eq!(ok(i, b"set ::result"), b"died");
        // `rename obj {}`
        ok(i, b"set ::result {}; cls create b; rename b {}");
        assert_eq!(ok(i, b"set ::result"), b"died");
        // `namespace delete <object-namespace>`
        ok(
            i,
            b"set ::result {}; cls create c; namespace delete [info object namespace c]",
        );
        assert_eq!(ok(i, b"set ::result"), b"died");
        // Re-entrancy guard: a destructor that re-destroys the object runs once.
        ok(
            i,
            b"oo::class create g { destructor { lappend ::result x; catch {[self] destroy} } }",
        );
        ok(i, b"set ::result {}; g create o; o destroy");
        assert_eq!(ok(i, b"set ::result"), b"x");
        // A2: the dup-name error uses the as-written name and `object`.
        ok(i, b"oo::object create foo");
        assert_eq!(i.eval_str(b"oo::object create foo"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't create object \"foo\": command already exists with that name"
        );
        ok(i, b"oo::class create K");
        assert_eq!(i.eval_str(b"oo::class create K"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"can't create object \"K\": command already exists with that name"
        );
    });
}

/// TclOO command lifetime is the stable owner identity, not the current
/// table/name. Explicit destruction must retire a hidden object token and
/// preserve the destructor's successful result.
#[test]
fn hidden_object_destruction_retires_identity_and_preserves_result() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [self object]}
                        }
                        C create x
                        interp hide {} x held
                        set a [interp invokehidden {} held destroy]
                        list $a $log [interp hidden {}] \
                             [catch {interp invokehidden {} held destroy} m] $m"#,
            ),
            br#"::x ::x {} 1 {invalid hidden command name "held"}"#,
        );
    });
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"namespace eval N {
                            oo::class create C
                            C create o
                            namespace export o
                        }
                        namespace import N::o
                        set a [o destroy]
                        list $a [info commands o] [info commands ::N::o]"#,
            ),
            b"{} {} {}",
        );
    });
}

/// Class commands and the private dispatchers share the same retirement
/// seam, including after independent rename and hide operations.
#[test]
fn hidden_class_and_private_dispatchers_retire_by_owner_identity() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C
                        C create x
                        interp hide {} C heldC
                        interp invokehidden {} heldC destroy
                        list [interp hidden {}] [info commands x] \
                             [catch {interp invokehidden {} heldC new} m] $m"#,
            ),
            br#"{} {} 1 {invalid hidden command name "heldC"}"#,
        );
    });
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C {method who {} {self object}}
                        C create x
                        set ns [info object namespace x]
                        rename ${ns}::my myx
                        interp hide {} myx heldmy
                        x destroy
                        list [interp hidden {}] \
                             [catch {interp invokehidden {} heldmy who} m] $m"#,
            ),
            br#"{} 1 {invalid hidden command name "heldmy"}"#,
        );
    });
}

/// TclOO teardown removes the old command token's trace list, not traces a
/// delete callback attaches to a same-named replacement (Tcl 9.0.4).
#[test]
fn object_retirement_preserves_visible_replacement_traces() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [list d [self object]]}
                        }
                        C create x
                        proc cb {old new op} {
                            lappend ::log [list t $old $new $op]
                            proc x {} {return P}
                            trace add command x delete cbnew
                        }
                        proc cbnew {old new op} {
                            lappend ::log [list r $old $new $op]
                        }
                        trace add command x delete cb
                        x destroy
                        set before [list [x] [trace info command x] $log]
                        rename x {}
                        list $before $log"#,
                ),
                br#"{P {{delete cbnew}} {{d ::x} {t ::x {} delete}}} {{d ::x} {t ::x {} delete} {r ::x {} delete}}"#,
            );
    });
}

/// A hidden token keeps its generation, so its delete trace does not own a
/// visible replacement and trace at the same display spelling.
#[test]
fn object_retirement_preserves_hidden_name_replacement_traces() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [list d [self object]]}
                        }
                        C create x
                        proc cb {old new op} {
                            lappend ::log [list t $old $new $op]
                            proc held {} {return H}
                            trace add command held delete cbnew
                        }
                        proc cbnew {old new op} {
                            lappend ::log [list r $old $new $op]
                        }
                        trace add command x delete cb
                        interp hide {} x held
                        interp invokehidden {} held destroy
                        set before [list [held] [trace info command held] $log \
                                         [interp hidden {}]]
                        rename held {}
                        list $before $log"#,
                ),
                br#"{H {{delete cbnew}} {{d ::x} {t ::held {} delete}} {}} {{d ::x} {t ::held {} delete} {r ::held {} delete}}"#,
            );
    });
}

/// Renaming an object's private dispatcher does not turn its mutable name
/// into ownership: a callback replacement at that name remains independent.
#[test]
fn object_retirement_preserves_private_dispatcher_replacement_traces() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [list d [self object]]}
                        }
                        C create x
                        set ns [info object namespace x]
                        rename ${ns}::my myx
                        proc cb {old new op} {
                            lappend ::log [list t $old $new $op]
                            proc myx {} {return M}
                            trace add command myx delete cbnew
                        }
                        proc cbnew {old new op} {
                            lappend ::log [list r $old $new $op]
                        }
                        trace add command myx delete cb
                        x destroy
                        set before [list [myx] [trace info command myx] $log]
                        rename myx {}
                        list $before $log"#,
                ),
                br#"{M {{delete cbnew}} {{d ::x} {t ::myx {} delete}}} {{d ::x} {t ::myx {} delete} {r ::myx {} delete}}"#,
            );
    });
}

/// The public object's delete callback runs before private dispatchers. A
/// dispatcher it moves is found again by identity and retired at its new
/// location, including a trace attached there by the callback.
#[test]
fn public_delete_callback_can_relocate_a_private_dispatcher() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        proc cb {ns old new op} {
                            lappend ::log [list move \
                                [catch {rename ${ns}::my ::movedmy} msg] \
                                $msg [info commands ::movedmy]]
                            trace add command ::movedmy delete mygone
                        }
                        proc mygone {old new op} {
                            lappend ::log [list mygone $old $op]
                        }
                        trace add command x delete [list cb $ns]
                        x destroy
                        list $log [info commands ::movedmy]"#,
            ),
            br#"{{move 0 {} ::movedmy} {mygone ::movedmy delete}} {}"#,
        );
    });
}

/// Tcl retires an object's three command roles in semantic order rather
/// than their table-generation order, even after both private dispatchers
/// have moved out of the instance namespace.
#[test]
fn object_command_roles_retire_public_myclass_my() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        rename ${ns}::my ::mm
                        rename ${ns}::myclass ::mc
                        proc t {tag old new op} {lappend ::log $tag}
                        trace add command x delete [list t public]
                        trace add command ::mm delete [list t my]
                        trace add command ::mc delete [list t myclass]
                        x destroy
                        set log"#,
            ),
            b"public myclass my",
        );
    });
}

/// TclOO's semantic role order also precedes instance-variable teardown.
/// Moving only `myclass` from the public callback is the asymmetric case:
/// table order would otherwise delete the remaining `my` first.
#[test]
fn object_command_roles_precede_variables_after_asymmetric_move() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        set ${ns}::v 1
                        proc public {ns old new op} {
                            lappend ::log public
                            rename ${ns}::myclass ::mc
                        }
                        proc t {tag args} {lappend ::log $tag}
                        proc var {name1 name2 op} {
                            set c [catch {trace add command ::mc delete late} m]
                            lappend ::log [list var [info commands ::mc] $c $m \
                                                [info commands ::x]]
                        }
                        proc late {args} {lappend ::log late}
                        trace add command x delete [list public $ns]
                        trace add command ${ns}::my delete [list t my]
                        trace add command ${ns}::myclass delete [list t myclass]
                        trace add variable ${ns}::v unset var
                        x destroy
                        set log"#,
            ),
            b"public myclass my {var {} 1 {unknown command \"::mc\"} ::x}",
        );
    });
}

/// A later role may attach a trace to an earlier role while every command
/// token is still visible. Tcl discards that late trace at final removal;
/// it does not restart the semantic role walk (Tcl 9.0.4).
#[test]
fn object_command_role_walk_does_not_revisit_earlier_roles() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C
                        C create x
                        set ns [info object namespace x]
                        proc t {tag args} {lappend ::log $tag}
                        proc mc {old new op} {
                            set c [catch {trace add command ::x delete late} m]
                            lappend ::log [list mc ::x $c $m]
                        }
                        proc late {old new op} {lappend ::log late}
                        trace add command x delete [list t public]
                        trace add command ${ns}::myclass delete mc
                        trace add command ${ns}::my delete [list t my]
                        x destroy
                        set log"#,
            ),
            b"public {mc ::x 0 {}} my",
        );
    });
}

/// Namespace imports retain the source command token across `interp hide`.
/// They keep dispatching the hidden OO object, report its hidden origin,
/// and retire with it.
#[test]
fn hidden_object_imports_follow_and_retire_with_the_source_token() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C {method ping {} {return ok}}
                        C create x
                        namespace export x
                        namespace eval dst {namespace import ::x}
                        set before [dst::x ping]
                        interp hide {} x hx
                        set hiddenCall [dst::x ping]
                        set hiddenOrigin [namespace eval dst {namespace origin x}]
                        interp invokehidden {} hx destroy
                        list $before $hiddenCall $hiddenOrigin \
                             [namespace eval dst {info commands x}]"#,
            ),
            b"ok ok ::hx {}",
        );
    });
}

/// Installing an ordinary command over an object command retires that
/// exact TclOO owner—including destructors and class descendant cascades—
/// while leaving the newly-installed command intact.
#[test]
fn ordinary_command_replacement_runs_displaced_oo_lifecycle() {
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"set log {}
                        oo::class create C {
                            destructor {lappend ::log [self object]}
                        }
                        C create x
                        proc x {} {return P}
                        list $log [info object isa object x] [x]"#,
            ),
            b"::x 0 P",
        );
    });
    leak_free(|i| {
        assert_eq!(
            ok(
                i,
                br#"oo::class create C
                        C create x
                        proc C {} {return P}
                        list [info commands x] [info object isa object x] [C]"#,
            ),
            b"{} 0 P",
        );
    });
}

/// The declaring scope of an unknown-method miss is stable OO identity.
/// Renaming the provider inside its user `unknown` method therefore does
/// not hide that provider's private methods from the final diagnostic.
#[test]
fn unknown_scope_survives_provider_rename() {
    leak_free(|i| {
        assert_eq!(
                ok(
                    i,
                    br#"oo::class create C {
                            private method secret {} {return SECRET}
                            method probe {} {my missing}
                            method unknown args {
                                rename ::C ::D
                                next {*}$args
                            }
                        }
                        C create o
                        catch {o probe} m
                        list $m [info object class o]"#,
                ),
                br#"{unknown method "missing": must be <cloned>, destroy, eval, probe, secret, unknown, variable or varname} ::D"#,
            );
    });
}

// Needs the numeric tower: method bodies compute via `expr`.
#[cfg(have_tommath)]
#[test]
fn private_method_scope_and_unknown_listing() {
    leak_free(|i| {
        ok(i, b"oo::class create parent");
        // A class's private method is invisible to a different scope's `my`
        // call: the object's private `step` does not shadow the class's
        // public `step` when the caller is a class method (oo-39.4/39.5).
        ok(
            i,
            b"oo::class create clsA { superclass parent; variable x; \
                  constructor {} {set x 1}; method act {} {my step; return}; \
                  method step {} {incr x}; method x {} {return $x} }",
        );
        ok(i, b"clsA create obj; obj act");
        assert_eq!(ok(i, b"obj x"), b"2");
        ok(
            i,
            b"oo::objdefine obj { variable x; private { method step {} {incr x 2} } }",
        );
        ok(i, b"obj act"); // class act -> class step (public), not obj private
        assert_eq!(ok(i, b"obj x"), b"3");
        // The unknown-method error lists in-scope private methods (oo-39.6).
        ok(
            i,
            b"oo::class create cls { superclass parent; variable x; \
                  constructor {val} {set x $val}; private method x {} {return $x}; \
                  method equal {other} {expr {$x == [$other y]}} }",
        );
        ok(i, b"cls create a 1; cls create b 2");
        assert_eq!(i.eval_str(b"a equal b"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown method \"y\": must be destroy, equal or x"
        );
        // An internal (`my`) miss also lists the unexported oo::object
        // built-ins and unexported methods (oo-39.8).
        ok(i, b"oo::class create P { method m {} { my nonesuch } }");
        ok(i, b"P create p");
        assert_eq!(i.eval_str(b"p m"), Code::Error);
        assert_eq!(
                i.result_bytes(),
                b"unknown method \"nonesuch\": must be <cloned>, destroy, eval, m, unknown, variable or varname"
            );
    });
}

#[test]
fn method_body_errorinfo_frames() {
    leak_free(|i| {
        ok(i, b"oo::class create cls { constructor {} {} }");
        ok(i, b"cls create obj");
        // A per-object method: `(object "::obj" method "bar" line N)`.
        ok(i, b"oo::objdefine obj method bar {} {error foo}");
        assert_eq!(i.eval_str(b"obj bar"), Code::Error);
        let ei = ok(i, b"set errorInfo");
        assert!(
            contains(&ei, b"(object \"::obj\" method \"bar\" line 1)"),
            "got {ei:?}"
        );
        // A class method: `(class "::cls" method "cm" line N)`.
        ok(i, b"oo::define cls method cm {} {error boom}");
        assert_eq!(i.eval_str(b"obj cm"), Code::Error);
        let ei = ok(i, b"set errorInfo");
        assert!(
            contains(&ei, b"(class \"::cls\" method \"cm\" line 1)"),
            "got {ei:?}"
        );
        // `my eval {…}` adds `(in "my eval" script line N)`.
        ok(i, b"oo::objdefine obj method ev {} {my eval {error e}}");
        assert_eq!(i.eval_str(b"obj ev"), Code::Error);
        let ei = ok(i, b"set errorInfo");
        assert!(
            contains(&ei, b"(in \"my eval\" script line 1)"),
            "got {ei:?}"
        );

        // The shared proc runner uses the full external invocation for
        // both `info level 0` and TIP 348's procedure-boundary `CALL`.
        ok(i, b"oo::define cls method stack {arg} {error stacked}");
        ok(i, b"catch {obj stack value} m o");
        assert_eq!(
            ok(i, b"lindex [dict get $o -errorstack] end"),
            b"obj stack value"
        );
        ok(i, b"catch {::obj stack rooted} m o");
        assert_eq!(
            ok(i, b"lindex [dict get $o -errorstack] end"),
            b"::obj stack rooted"
        );
    });
}

#[test]
fn original_renamed_definition_matches_native_source_and_raw_return_boundaries() {
    // naming.class.original-renamed-definition-error-context
    // docs/design/analysis/name-resolution-proofs/class-original-renamed-definition-error-context.md
    // Whole public completion/errorInfo only; no private class/header
    // identity follows. C8.4/8.5 stock TclOO is unavailable: their C API
    // settles top-level return to OK, while this raw evaluator retains RETURN.
    macro_rules! provider {
        ($engine:literal, $version:literal, $raw_return:literal) => {
            (
                $engine,
                $raw_return,
                include_str!(concat!(
                    "../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/",
                    $version,
                    "/renamed-definition/stdout"
                )),
            )
        };
    }
    let source = include_bytes!(
        "../../../../rust/tcl-registry/tests/data/native_oo_renamed_definition321/source.tcl"
    );
    for (engine, raw_return, stdout) in [
        provider!("tcl8.4", "8.4.20", true),
        provider!("tcl8.5", "8.5.19", true),
        provider!("tcl8.6", "8.6.18", false),
        provider!("tcl9.0", "9.0.4", false),
        provider!("tcl9.1", "9.1.0", false),
        provider!("jim", "jim", false),
    ] {
        let (expected_code, original) = stdout
            .lines()
            .find_map(|line| line.strip_prefix("ORIGINAL|"))
            .unwrap()
            .split_once('|')
            .unwrap();
        let expected: Vec<u8> = original
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        counters::reset();
        {
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            // The native provider starts with its distribution extensions loaded.
            crate::cmd_proc::install_stock_scripted_wrappers(&mut interp);
            let code = interp.eval_str(source);
            assert!(
                !interp.host_refusal_pending(),
                "{engine}: {:?}",
                interp.native_access_refusal()
            );
            if raw_return {
                assert_eq!(expected_code, "0", "{engine}: native embedding boundary");
                assert_eq!(code, Code::Return, "{engine}: raw evaluator boundary");
            } else {
                assert_eq!(code.as_int().to_string(), expected_code, "{engine}");
            }
            assert_eq!(interp.result_bytes(), expected, "{engine}");
        }
        assert_eq!(counters::finalize(), 0, "{engine}");
        assert_eq!(counters::double_free_count(), 0, "{engine}");
    }
}

#[test]
fn define_script_errorinfo_and_super_mixin_messages() {
    // naming.tcloo.original-nonclass-superclass-mixin-errors
    // docs/design/analysis/name-resolution-proofs/tcloo-original-nonclass-superclass-mixin-errors.md
    leak_free(|i| {
        // superclass/mixin: not-an-object vs not-a-class messages (as-written
        // name; matches C's Tcl_GetObjectFromObj + ClassSuperSet/MixinSet).
        ok(i, b"oo::object create anobj");
        assert_eq!(
            i.eval_str(b"oo::class create c1 { superclass nosuch }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"nosuch does not refer to an object");
        assert_eq!(
            i.eval_str(b"oo::class create c2 { superclass anobj }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"only a class can be a superclass");
        assert_eq!(
            i.eval_str(b"oo::class create c3 { mixin nosuch }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"nosuch does not refer to an object");
        assert_eq!(
            i.eval_str(b"oo::class create c4 { mixin anobj }"),
            Code::Error
        );
        assert_eq!(i.result_bytes(), b"only a class can be a mixin");
        // naming.class.original-renamed-definition-error-context
        // docs/design/analysis/name-resolution-proofs/class-original-renamed-definition-error-context.md
        // The define-script errorInfo frame uses the *current* (renamed) name.
        ok(i, b"oo::class create base");
        assert_eq!(
            i.eval_str(b"oo::class create abc { superclass base; rename abc def; error foo }"),
            Code::Error
        );
        let ei = ok(i, b"set errorInfo");
        assert!(
            ei.windows(b"class \"::oo::define::def\"".len())
                .any(|w| w == b"class \"::oo::define::def\""),
            "errorInfo should name the renamed class ::oo::define::def, got {ei:?}"
        );
    });
}

#[test]
fn define_single_command_wrong_args_names_whole_command() {
    leak_free(|i| {
        ok(i, b"oo::class create Foo");
        // Single-command form: the message names the whole original command.
        assert_eq!(
            i.eval_str(b"oo::define Foo method missingArgs"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"oo::define Foo method name ?option? args body\""
        );
        assert_eq!(
            i.eval_str(b"oo::objdefine Foo method missingArgs"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"oo::objdefine Foo method name ?option? args body\""
        );
        // Body form keeps the bare usage (no rewrite prefix).
        assert_eq!(
            i.eval_str(b"oo::define Foo { method missingArgs }"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"wrong # args: should be \"method name ?option? args body\""
        );
    });
}

#[test]
fn destructor_error_propagation_and_renamed_my_teardown() {
    leak_free(|i| {
        // Explicit `obj destroy` propagates a destructor error to its caller
        // (oo-3.6); the object command is still removed.
        ok(i, b"oo::class create cls { destructor { error foo } }");
        assert_eq!(
            ok(
                i,
                b"list [catch {[cls create obj] destroy} m] $m [info commands obj]"
            ),
            b"1 foo {}"
        );
        // Implicit teardown (rename {}) swallows the destructor error.
        ok(i, b"cls create obj2");
        ok(i, b"rename obj2 {}");
        // A `my` renamed out of the instance namespace is deleted when the
        // object is destroyed (oo-3.4): tie its lifetime to the object.
        ok(i, b"oo::class create plain { constructor {} {} }");
        ok(i, b"plain create p");
        ok(i, b"rename [info object namespace p]::my ::pmy");
        assert_eq!(ok(i, b"info commands ::pmy"), b"::pmy");
        ok(i, b"p destroy");
        assert_eq!(ok(i, b"info commands ::pmy"), b"");
    });
}

#[test]
fn create_with_namespace_and_copy_targetns() {
    leak_free(|i| {
        // `createWithNamespace` is unexported by default; once exported it
        // creates the object in a fresh (must-not-exist) namespace.
        ok(i, b"oo::class create AC");
        assert_eq!(
            i.eval_str(b"AC createWithNamespace inst ::ns1"),
            Code::Error
        );
        ok(i, b"oo::objdefine AC export createWithNamespace");
        ok(i, b"AC createWithNamespace inst ::ns1");
        assert_eq!(ok(i, b"info object namespace inst"), b"::ns1");
        // An already-existing namespace is rejected.
        ok(i, b"namespace eval ::taken {}");
        assert_eq!(
            i.eval_str(b"AC createWithNamespace inst2 ::taken"),
            Code::Error
        );
        // `oo::copy src tgt targetNamespace` places the copy's vars there.
        ok(i, b"oo::class create C { method m {} {} }");
        ok(i, b"C create src");
        ok(i, b"oo::copy src dst ::dstns");
        assert_eq!(ok(i, b"info object namespace dst"), b"::dstns");
    });
}

#[test]
fn oo_copy_clones_classes() {
    leak_free(|i| {
        // `oo::copy` of a class clones the class facet, so the copy is a
        // working class whose instances run the cloned methods.
        ok(
            i,
            b"oo::class create foo { method testme {} { return [self class] } }",
        );
        ok(i, b"oo::copy foo bar");
        assert_eq!(ok(i, b"[bar new] testme"), b"::bar");
        // Cloned superclasses + declared variables come across too.
        ok(i, b"oo::class create AC");
        ok(i, b"oo::class create Foo { superclass AC; variable a b c }");
        ok(i, b"oo::copy Foo Bar");
        assert_eq!(ok(i, b"info class variable Bar"), b"a b c");
        assert_eq!(ok(i, b"info class superclasses Bar"), b"::AC");
    });
}

#[test]
fn inheritance_and_next() {
    leak_free(|i| {
        ok(i, b"oo::class create Animal { variable s; constructor {x} {set s $x}; method speak {} {return \"I say $s\"} }");
        ok(i, b"oo::class create Dog { superclass Animal; constructor {} {next bark}; method speak {} {return \"Dog: [next]\"} }");
        ok(i, b"set d [Dog new]");
        assert_eq!(ok(i, b"$d speak"), b"Dog: I say bark");
        ok(i, b"oo::define Dog method legs {} { return 4 }");
        assert_eq!(ok(i, b"$d legs"), b"4");
    });
}

// Needs the numeric tower: `forward` targets `::tcl::mathop::+`.
#[cfg(have_tommath)]
#[test]
fn objdefine_forward_mixin_unexport_copy() {
    leak_free(|i| {
        ok(i, b"oo::class create Base { method m {} {return base}; method hide {} {return H}; unexport hide }");
        ok(i, b"set o [Base new]");
        // unexported method: external call fails, `my` works.
        assert_eq!(i.eval_str(b"$o hide"), Code::Error);
        // per-object method via objdefine
        ok(i, b"oo::objdefine $o { method extra {} {return per-obj} }");
        assert_eq!(ok(i, b"$o extra"), b"per-obj");
        // forward
        ok(i, b"oo::class create F { forward add ::tcl::mathop::+ 10 }");
        assert_eq!(ok(i, b"[F new] add 5"), b"15");
        // mixin
        ok(
            i,
            b"oo::class create Mix { method mixed {} {return MIXED} }",
        );
        ok(i, b"oo::objdefine $o { mixin Mix }");
        assert_eq!(ok(i, b"$o mixed"), b"MIXED");
        // info introspection
        assert_eq!(ok(i, b"info object class $o"), b"::Base");
        assert_eq!(ok(i, b"info object isa object $o"), b"1");
        let object = ok(i, b"set o");
        assert_eq!(ok(i, b"info class instances Base"), object);
    });
}

#[test]
fn oo_package_is_provided() {
    leak_free(|i| {
        assert_eq!(ok(i, b"package require tcl::oo"), b"1.3.1");
    });
}

#[test]
fn info_frame_method_body_lines_are_relative() {
    leak_free(|i| {
        // A method body defined while sourcing reports file-absolute lines,
        // so a body command's `info frame` line tracks the source — even
        // when the body opens on a later line than the `method` command.
        let script = b"oo::class create C {}\n\
                oo::define C {\n\
                  method a {} {info frame 0}\n\
                  method b {\n\
                  } {info frame 0}\n\
                }\n\
                oo::define C method c {} {info frame 0}\n\
                C create o\n\
                list [dict get [o a] line] [dict get [o b] line] \
                     [dict get [o c] line] [dict get [o a] type] \
                     [dict get [o a] file]";
        let r = i.eval_sourced(script, b"/tmp/x.tcl");
        assert_eq!(r, Code::Ok);
        // `method a` body line 3; `method b` body line 5; `method c` line 7.
        assert_eq!(i.result_bytes(), b"3 5 7 source /tmp/x.tcl");
    });
}

#[test]
fn info_frame_reports_method_context() {
    leak_free(|i| {
        // `info frame 0` in a class-defined method reports method + class;
        // an object-defined method reports method + object (not `proc`).
        ok(i, b"oo::class create c");
        ok(i, b"c create i");
        ok(i, b"oo::define c method m {} { info frame 0 }");
        ok(i, b"oo::objdefine i method n {} { info frame 0 }");
        let cm = ok(i, b"c create o; o m");
        assert!(cm.windows(13).any(|w| w == b"method m clas"));
        assert!(cm.ends_with(b"class ::c level 0"));
        let inf = ok(i, b"i n");
        assert!(inf.ends_with(b"object ::i level 0"));
        // A plain proc still reports `proc`, not method/class.
        ok(i, b"proc p {} { info frame 0 }");
        let pf = ok(i, b"p");
        assert!(pf.windows(7).any(|w| w == b"proc ::"));
        assert!(!pf.windows(6).any(|w| w == b"method"));
    });
}

#[test]
fn class_destroy_cascades_to_mixin_users() {
    leak_free(|i| {
        // Destroying a class destroys objects that mix it in; an object
        // whose mixin was cleared survives.
        ok(i, b"oo::class create A");
        ok(i, b"oo::class create B");
        ok(i, b"oo::objdefine [A create keep] mixin B");
        ok(i, b"oo::objdefine [A create gone] mixin B");
        ok(i, b"oo::objdefine keep mixin");
        ok(i, b"B destroy");
        assert_eq!(ok(i, b"info object isa object gone"), b"0");
        assert_eq!(ok(i, b"info object isa object keep"), b"1");
    });
}

#[test]
fn self_class_in_object_method_errors() {
    leak_free(|i| {
        // `self class` in a method declared directly on the object has no
        // declaring class.
        ok(i, b"oo::object create obj");
        ok(i, b"oo::objdefine obj method demo {} { self class }");
        assert_eq!(i.eval_str(b"obj demo"), Code::Error);
        assert_eq!(i.result_bytes(), b"method not defined by a class");
        // A class instance method still reports its declaring class.
        ok(i, b"oo::class create C { method d {} { self class } }");
        assert_eq!(ok(i, b"[C new] d"), b"::C");
    });
}

#[test]
fn no_method_forces_user_unknown() {
    leak_free(|i| {
        // A no-method invocation `$obj` with a user `unknown` runs it with
        // empty args (C's FORCE_UNKNOWN); a plain object reports usage.
        ok(i, b"set o [oo::object new]");
        assert_eq!(i.eval_str(b"$o"), Code::Error);
        assert!(i.result_bytes().starts_with(b"wrong # args: should be"));
        ok(
            i,
            b"oo::objdefine $o method unknown args { return \"u:>>$args<<\" }",
        );
        assert_eq!(ok(i, b"$o"), b"u:>><<");
        assert_eq!(ok(i, b"$o foo bar"), b"u:>>foo bar<<");
    });
}

#[test]
fn my_create_reaches_class_factory() {
    leak_free(|i| {
        // A metaclass method can instantiate via `my create`, even when the
        // class unexports `create` for external callers (TclOO factory).
        ok(
            i,
            b"oo::class create meta { superclass oo::class; self { unexport create new; \
                   method make {x} { my create $x } } }",
        );
        // External create is hidden (unknown method, not listing create/new).
        assert_eq!(i.eval_str(b"meta create foo"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            b"unknown method \"create\": must be destroy or make"
        );
        // The internal `my create` still works.
        assert_eq!(ok(i, b"meta make ::bar"), b"::bar");
        assert_eq!(ok(i, b"info object class ::bar"), b"::meta");
    });
}

#[test]
fn next_shares_caller_frame_level() {
    leak_free(|i| {
        // Every method in a `next` chain runs at the level of the original
        // invocation, so `info level` is the same and `upvar 1` from a
        // next-invoked method reaches the original caller's frame.
        ok(
                i,
                b"oo::class create A { method incr {var step} { upvar 1 $var v; ::incr v $step; return [info level] } }",
            );
        ok(
                i,
                b"oo::class create B { superclass A; method incr {var {step 1}} { list [info level] [next $var $step] } }",
            );
        ok(i, b"B create b");
        ok(i, b"set x 10");
        // Both methods report level 1; A's upvar bumps the global x to 11.
        assert_eq!(ok(i, b"b incr x"), b"1 1");
        assert_eq!(ok(i, b"set x"), b"11");
    });
}

#[test]
fn builtin_slot_objects_present() {
    leak_free(|i| {
        // The eleven built-in slots are ::oo::Slot instances.
        assert_eq!(
            ok(i, b"lsort [info class instances ::oo::Slot]"),
            b"::oo::configuresupport::objreadableproperties \
                  ::oo::configuresupport::objwritableproperties \
                  ::oo::configuresupport::readableproperties \
                  ::oo::configuresupport::writableproperties ::oo::define::filter \
                  ::oo::define::mixin ::oo::define::superclass ::oo::define::variable \
                  ::oo::objdefine::filter ::oo::objdefine::mixin ::oo::objdefine::variable"
        );
        // filter slot: public ops from the class, Get/Set private.
        assert_eq!(
            ok(i, b"lsort [info object methods ::oo::define::filter -all]"),
            b"-append -appendifnew -clear -prepend -remove -set"
        );
        assert_eq!(
            ok(
                i,
                b"lsort [info object methods ::oo::define::filter -private]"
            ),
            b"Get Set"
        );
        // mixin slot also has a Resolve + a --default-operation forward.
        assert_eq!(
            ok(
                i,
                b"lsort [info object methods ::oo::define::mixin -private]"
            ),
            b"--default-operation Get Resolve Set"
        );
    });
}

#[test]
fn foundation_abstract_singleton_classes() {
    leak_free(|i| {
        // The three foundation metaclasses exist in a fresh interp.
        assert_eq!(ok(i, b"info object class ::oo::singleton"), b"::oo::class");
        assert_eq!(ok(i, b"info object class ::oo::abstract"), b"::oo::class");
        assert_eq!(
            ok(i, b"info object class ::oo::SingletonInstance"),
            b"::oo::class"
        );
        // singleton/abstract are metaclasses (superclass oo::class).
        assert_eq!(
            ok(i, b"info class superclasses ::oo::singleton"),
            b"::oo::class"
        );
        assert_eq!(
            ok(i, b"info class superclasses ::oo::abstract"),
            b"::oo::class"
        );
        // SingletonInstance is a plain subclass of oo::object.
        assert_eq!(
            ok(i, b"info class superclasses ::oo::SingletonInstance"),
            b"::oo::object"
        );
        assert_eq!(
            ok(i, b"lsort [info class subclasses ::oo::class]"),
            b"::oo::abstract ::oo::configurable ::oo::singleton"
        );
    });
}

/// Regression coverage for the native-stack recursion hazard
/// `MAX_MRO_DEPTH`/`MAX_MRO_VISITS` guard against: `linearize_class`
/// (backing `method_chain_faceted` — the hot path for every ordinary
/// method dispatch — and `class_precedence`, `info class call`) and
/// `gather_class_props` (`info class properties -all`) recurse once
/// per mixin/superclass level, with only a same-branch cycle guard
/// bounding a malformed hierarchy — an unguarded deep mixin chain
/// overflows the native stack. Confirmed crash reproduction: a deep
/// `mixin` chain (`oo::class create C$i { mixin C[i-1] }`, no `{*}`
/// needed) SIGABRTs between depth 100-150 on a 256 KiB stack, and still
/// crashes at depth 2000 on a 1 MiB stack (a plain `superclass` chain
/// hits the same recursion but is masked by superclass cycle-check cost, which makes naive *construction* slow
/// before reaching crash depth — mixins avoid that and reproduce
/// cleanly). This builds a 2000-deep mixin chain (matching that
/// confirmed-still-crashing depth) and drives both guarded functions
/// over the whole thing via `info class call` and `info class
/// properties -all`; the assertion is that both complete at all, not
/// what they return — `MAX_MRO_DEPTH` (64) means the reported
/// precedence/property set is legitimately truncated for a hierarchy
/// this deep, the same graceful "just stop descending" degradation the
/// `path`/`seen` cycle guards already apply to a malformed hierarchy,
/// rather than erroring the whole dispatch out or crashing.
#[test]
fn deeply_nested_mixin_chain_survives_linearisation_and_property_gathering() {
    leak_free(|i| {
        const DEPTH: usize = 2000;
        ok(i, b"oo::class create C0");
        for n in 1..DEPTH {
            ok(
                i,
                format!("oo::class create C{n} {{ mixin C{} }}", n - 1).as_bytes(),
            );
        }
        let last = DEPTH - 1;
        let _ = i.eval_str(format!("info class call C{last} foo").as_bytes());
        let _ = i.eval_str(format!("info class properties C{last} -all").as_bytes());
        for n in (0..DEPTH).rev() {
            let _ = i.eval_str(format!("C{n} destroy").as_bytes());
        }
    });
}

/// A moderately nested mixin chain (well under `MAX_MRO_DEPTH`) still
/// linearises completely and correctly — the safety net must not fire,
/// let alone truncate anything, on realistic nesting depths. Checks both
/// fixed functions: method dispatch still resolves through the whole
/// mixin chain to the base class's implementation (`linearize_class` via
/// `method_chain_faceted`), and `-all` property gathering still unions
/// every level's declared properties (`gather_class_props`) — exactly
/// the behaviour before this fix, not merely "does not crash".
#[test]
fn moderately_nested_mixin_chain_still_behaves_identically() {
    leak_free(|i| {
        ok(
            i,
            b"oo::class create C0 { method foo {} { return from-c0 } }",
        );
        ok(i, b"oo::class create C1 { mixin C0 }");
        ok(i, b"oo::class create C2 { mixin C1 }");
        ok(i, b"oo::class create C3 { mixin C2 }");
        ok(i, b"C3 create obj");
        assert_eq!(ok(i, b"obj foo"), b"from-c0");
        ok(i, b"obj destroy");

        ok(
            i,
            b"oo::configurable create Base { property p0 -kind readable }",
        );
        ok(
            i,
            b"oo::configurable create P1 { mixin Base; property p1 -kind readable }",
        );
        ok(
            i,
            b"oo::configurable create P2 { mixin P1; property p2 -kind readable }",
        );
        assert_eq!(
            ok(i, b"lsort [info class properties P2 -all]"),
            b"-p0 -p1 -p2"
        );

        ok(i, b"C3 destroy; C2 destroy; C1 destroy; C0 destroy");
        ok(i, b"P2 destroy; P1 destroy; Base destroy");
    });
}
