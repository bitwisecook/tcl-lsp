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

//! The dispatcher's own misses and the subcommands that walk the namespace tree, from `current` and `eval` to `path` and `upvar`.

use super::*;

/// `namespace` is a `TclMakeEnsemble` command, so its exact-then-unique-
/// prefix scan and its whole miss sentence belong to
/// `tcl_cmd_core::ensemble`, rather than a hand-rolled scan with the
/// 19-entry enumeration duplicated as a literal beside the table that
/// already owns it.
///
/// tclsh 8.6.16 / 9.0.4:
///   namespace cu -> ::
///   namespace {} -> unknown or ambiguous subcommand "": must be children,
///                   code, current, delete, ensemble, eval, exists, export,
///                   forget, import, inscope, origin, parent, path,
///                   qualifiers, tail, unknown, upvar, or which
///   namespace e  -> unknown or ambiguous subcommand "e": must be <same>
///                   (ensemble/eval/exists/export)
#[test]
fn namespace_ensemble_miss_comes_from_the_owner() {
    const MUST: &str = "must be children, code, current, delete, ensemble, eval, exists, \
                            export, forget, import, inscope, origin, parent, path, qualifiers, \
                            tail, unknown, upvar, or which";
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace cu"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::");
        assert_eq!(i.eval_str(b"namespace {}"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("unknown or ambiguous subcommand \"\": {MUST}").as_bytes()
        );
        assert_eq!(i.eval_str(b"namespace e"), Code::Error);
        assert_eq!(
            i.result_bytes(),
            format!("unknown or ambiguous subcommand \"e\": {MUST}").as_bytes()
        );
    });
}

#[test]
fn current_is_global_at_top_level() {
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace current"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::");
    });
}

#[test]
fn eval_switches_current_and_defines_in_ns() {
    leak_free(|i| {
        // A command defined inside `namespace eval` lands in that ns.
        assert_eq!(
            i.eval_str(b"namespace eval foo { set ::probe [namespace current] }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"set ::probe"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::foo");
        // Back at top level current is global again.
        assert_eq!(i.eval_str(b"namespace current"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::");
    });
}

#[test]
fn exists_and_parent_and_children() {
    leak_free(|i| {
        i.eval_str(b"namespace eval a { namespace eval b {} }");
        assert_eq!(i.eval_str(b"namespace exists ::a::b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"namespace exists ::a::nope"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
        assert_eq!(i.eval_str(b"namespace parent ::a::b"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::a");
        assert_eq!(i.eval_str(b"namespace children ::a"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::a::b");

        assert_eq!(
            i.eval_str(
                b"namespace eval ::a {
                          catch {namespace parent {not here}} pm po
                          catch {namespace children {also not here}} cm co
                          list $pm [dict get $po -errorcode] \
                               $cm [dict get $co -errorcode]
                      }"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"{namespace \"not here\" not found in \"::a\"} \
                  {TCL LOOKUP NAMESPACE {not here}} \
                  {namespace \"also not here\" not found in \"::a\"} \
                  {TCL LOOKUP NAMESPACE {also not here}}"
        );
    });
}

#[test]
fn qualifiers_and_tail() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(b"namespace qualifiers ::foo::bar::baz"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::foo::bar");
        assert_eq!(i.eval_str(b"namespace tail ::foo::bar::baz"), Code::Ok);
        assert_eq!(i.result_bytes(), b"baz");
        assert_eq!(i.eval_str(b"namespace qualifiers plain"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        assert_eq!(i.eval_str(b"namespace tail plain"), Code::Ok);
        assert_eq!(i.result_bytes(), b"plain");
    });
}

#[test]
fn subcommand_prefix_abbreviation() {
    leak_free(|i| {
        // `namespace` subcommands accept unambiguous prefixes.
        assert_eq!(i.eval_str(b"namespace exist ::nope"), Code::Ok);
        assert_eq!(i.result_bytes(), b"0");
        assert_eq!(i.eval_str(b"namespace cur"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::");
        // An ambiguous prefix still errors.
        assert_eq!(i.eval_str(b"namespace ex foo"), Code::Error);
    });
}

/// Vectors measured against tclsh 9.0.4. These exercise the runtime
/// adapter through the same shared namespace grammar as the bytecode VM.
#[test]
fn namespace_issue_1584_oracle_vectors() {
    leak_free(|i| {
        assert_eq!(
            i.eval_str(
                b"namespace eval declared {variable only
                      list [namespace which -variable only] [info vars] [info exists only]}"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::declared::only only 0");

        for script in [
            &b"namespace which -zork puts"[..],
            b"namespace which -command puts extra",
        ] {
            assert_eq!(i.eval_str(script), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"namespace which ?-command? ?-variable? name\""
            );
        }

        assert_eq!(
            i.eval_str(b"namespace eval self {namespace import ::self::*}"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"import pattern \"::self::*\" tries to import from namespace \"self\" into itself"
        );
        assert_eq!(
            i.eval_str(b"namespace eval dest {namespace import ::nosuch::*}"),
            Code::Error
        );
        assert_eq!(
            i.result_bytes(),
            b"unknown namespace in import pattern \"::nosuch::*\""
        );

        for script in [&b"namespace origin"[..], b"namespace origin set extra"] {
            assert_eq!(i.eval_str(script), Code::Error);
            assert_eq!(
                i.result_bytes(),
                b"wrong # args: should be \"namespace origin name\""
            );
        }

        assert_eq!(
            i.eval_str(
                b"namespace eval order {}
                      foreach n {one two three four five six seven eight nine ten} {
                          namespace eval ::order::$n {}
                      }
                      namespace children ::order"
            ),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"::order::six ::order::four ::order::three ::order::eight \
                  ::order::seven ::order::nine ::order::five ::order::two \
                  ::order::one ::order::ten"
        );

        assert_eq!(
            i.eval_str(
                b"namespace eval p {}
                      foreach n {a0 a1 a2 a3 a4 a5 a6 a7 a8 a9 a10 a11} {
                          namespace eval ::p::$n {}
                      }
                      foreach i {1 2 4 5 6 7 8 9 10 11} {namespace delete ::p::a$i}
                      namespace children ::p"
            ),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::p::a0 ::p::a3");
    });

    // The active command survives long enough to return even though the
    // global namespace and command table have been torn down.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"namespace delete ::"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
        assert_eq!(i.eval_str(b"puts hi"), Code::Error);
        assert_eq!(i.result_bytes(), b"invalid command name \"puts\"");
    });
}

#[test]
fn path_query_returns_list() {
    leak_free(|i| {
        i.eval_str(b"namespace eval lib {}");
        i.eval_str(b"namespace eval app { namespace path ::lib }");
        assert_eq!(
            i.eval_str(b"namespace eval app { namespace path }"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"::lib");
    });
}

// Needs the numeric tower: the linked var is bumped via `expr`.
#[cfg(have_tommath)]
#[test]
fn namespace_upvar_links_local_to_ns_var() {
    leak_free(|i| {
        i.eval_str(b"namespace eval a { variable x 42 }");
        // `namespace upvar a x lx` links a frame-local `lx` to `::a::x`.
        assert_eq!(
            i.eval_str(b"proc p {} { namespace upvar a x lx; set lx [expr {$lx+1}]; return $lx }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"p"), Code::Ok);
        assert_eq!(i.result_bytes(), b"43");
        // The write went through to the namespace variable.
        assert_eq!(i.eval_str(b"set ::a::x"), Code::Ok);
        assert_eq!(i.result_bytes(), b"43");
        // A missing namespace is an error.
        assert_eq!(i.eval_str(b"namespace upvar nope v lv"), Code::Error);
    });
}

/// An embedder can supply a plain string whose bytes are not UTF-8.
/// Drive the real `namespace` adapter with such objects rather than using a
/// script-created byte array (whose string shimmer is valid UTF-8), so any
/// lossy `&str` hop makes these two distinct namespace names disappear or
/// collide.
#[test]
fn byte_valued_namespace_navigation_uses_the_embedder_bytes_verbatim() {
    fn invoke(interp: &mut Interp, words: &[&[u8]]) -> Code {
        let argv: Vec<*mut crate::obj::TclObj> = words
            .iter()
            .map(|word| crate::obj::new_string_bytes(word))
            .collect();
        let code = super::super::namespace_cmd(interp, &argv);
        for object in argv {
            super::super::drop_fresh(object);
        }
        code
    }

    leak_free(|interp| {
        let parent = b"::raw\xff";
        let sibling = b"::raw\xfe";
        let child = b"::raw\xff::child\xfd";
        {
            let mut namespaces = interp.namespaces_mut();
            namespaces.ensure_namespace(crate::namespace::GLOBAL, parent);
            namespaces.ensure_namespace(crate::namespace::GLOBAL, sibling);
            namespaces.ensure_namespace(crate::namespace::GLOBAL, child);
        }

        assert_eq!(invoke(interp, &[b"namespace", b"exists", parent]), Code::Ok);
        assert_eq!(interp.result_bytes(), b"1");
        assert_eq!(
            invoke(interp, &[b"namespace", b"exists", sibling]),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"1");

        assert_eq!(invoke(interp, &[b"namespace", b"parent", child]), Code::Ok);
        assert_eq!(interp.result_bytes(), parent);

        assert_eq!(
            invoke(interp, &[b"namespace", b"children", parent]),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), child);
        assert_eq!(
            invoke(interp, &[b"namespace", b"children", parent, child]),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), child);

        // Native namespace child matching scans opaque CString bytes.
        assert_eq!(
            invoke(interp, &[b"namespace", b"children", parent, b"*"]),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), child);
        assert_eq!(
            invoke(interp, &[b"namespace", b"children", parent, b"*other*"]),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"");

        let missing = b"::missing\xff";
        assert_eq!(
            invoke(interp, &[b"namespace", b"parent", missing]),
            Code::Error
        );
        let mut message = b"namespace \"".to_vec();
        message.extend_from_slice(missing);
        message.extend_from_slice(b"\" not found");
        assert_eq!(interp.result_bytes(), message);
    });
}
