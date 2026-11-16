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

//! How a command name resolves through the tree: qualified names, the path, the unknown handler, braced defining words and byte-valued names.

use super::*;

#[test]
fn namespace_unknown_retains_the_original_root_and_validates_before_replacement() {
    for version in tcl_dialect::TclVersion::ALL
        .into_iter()
        .filter(|version| *version >= tcl_dialect::TclVersion::V8_5)
    {
        leak_free(|interp| {
            interp.set_runtime_version(version);
            let command = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"namespace"));
            let subcommand = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"unknown"));
            let original = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"::unknown tag"));
            let prefix = [command.as_ptr(), subcommand.as_ptr()];
            assert_eq!(
                super::super::ns_unknown(interp, &[prefix[0], prefix[1], original.as_ptr()]),
                Code::Ok
            );
            assert_eq!(interp.result_obj(), original.as_ptr());
            assert_eq!(super::super::ns_unknown(interp, &prefix), Code::Ok);
            assert_eq!(interp.result_obj(), original.as_ptr());
            let malformed = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b"{"));
            assert_eq!(
                super::super::ns_unknown(interp, &[prefix[0], prefix[1], malformed.as_ptr()]),
                Code::Error
            );
            assert_eq!(super::super::ns_unknown(interp, &prefix), Code::Ok);
            assert_eq!(interp.result_obj(), original.as_ptr());
            let empty = crate::obj::Owned::fresh(crate::obj::new_string_bytes(b" \t "));
            assert_eq!(
                super::super::ns_unknown(interp, &[prefix[0], prefix[1], empty.as_ptr()]),
                Code::Ok
            );
            assert_eq!(interp.result_obj(), empty.as_ptr());
            assert_eq!(super::super::ns_unknown(interp, &prefix), Code::Ok);
            assert_ne!(interp.result_obj(), original.as_ptr());
            assert_eq!(interp.result_bytes(), b"::unknown");
        });
    }
}

#[test]
fn namespace_unknown_keeps_reached_prefix_members_after_handler_reset() {
    pins(
        br#"namespace eval N {
            proc handler {tag cmd args} {namespace unknown {}; return [list $tag $cmd $args]}
            namespace unknown {::N::handler retained}
            missing arg
        }"#,
        b"retained missing arg",
    );
}

#[test]
fn path_fallback_resolves_unqualified() {
    leak_free(|i| {
        // A command in ::lib (the alias creates ::lib), then ::lib on ::app's path.
        i.eval_str(b"interp alias {} ::lib::ping {} set pinged");
        assert_eq!(
            i.eval_str(b"namespace eval app { namespace path ::lib }"),
            Code::Ok
        );
        // Now from ::app, bare `ping` resolves through the path. Its body
        // `set pinged` runs with the current namespace = ::app, so the
        // variable lands in ::app's table, not the global frame.
        assert_eq!(i.eval_str(b"namespace eval app { ping yes }"), Code::Ok);
        assert_eq!(i.result_bytes(), b"yes");
        assert_eq!(i.eval_str(b"set ::app::pinged"), Code::Ok);
        assert_eq!(i.result_bytes(), b"yes");
        // …and it is NOT visible as a bare global.
        assert_eq!(i.eval_str(b"set pinged"), Code::Error);
    });
}

#[test]
fn qualified_command_falls_back_to_global() {
    // A relative qualified command name resolves against the current
    // namespace, then the global one (C's `TclGetNamespaceForQualName`): so
    // `foo::bar` from inside `::a::b` finds `::foo::bar` when `::a::b::foo`
    // doesn't exist. (The bug: `tcl::build-info` failing inside a namespace.)
    leak_free(|i| {
        i.eval_str(b"namespace eval foo { proc bar {} {return BAR} }");
        assert_eq!(
            i.eval_str(b"namespace eval ::a::b { set ::r [foo::bar] }"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"BAR");
        // A relative qualifier that *does* exist locally still wins.
        i.eval_str(b"namespace eval x::foo { proc bar {} {return LOCAL} }");
        assert_eq!(i.eval_str(b"namespace eval x { foo::bar }"), Code::Ok);
        assert_eq!(i.result_bytes(), b"LOCAL");
        i.eval_str(b"unset -nocomplain ::r");
    });
}

#[test]
fn qualifier_namespace_existing_does_not_commit_resolution() {
    // tclsh 8.6/9.0: the fallback is *command*-existence-checked, not
    // namespace-existence-checked.  `inner::p` from
    // `::outer` must dispatch `::inner::p` even though the namespace
    // `::outer::inner` exists — it merely holds no `p`.
    leak_free(|i| {
        i.eval_str(b"namespace eval inner { proc p {} {return GLOBAL} }");
        i.eval_str(b"namespace eval outer { namespace eval inner { proc other {} {} } }");
        assert_eq!(
            i.eval_str(b"namespace eval outer { inner::p }"),
            Code::Ok,
            "resolution must fall through to ::inner::p: {:?}",
            String::from_utf8_lossy(&i.result_bytes()),
        );
        assert_eq!(i.result_bytes(), b"GLOBAL");
    });
}

/// Every shared command-resolution vector
/// (`tcl_syntax::naming::conformance`, pinned against real tclsh by
/// the `tcl-syntax` conformance test) must dispatch identically
/// through this runtime's namespace tree — the anti-drift gate for
/// `Namespaces::home_of`.
///
/// `builtins::install` only registers `if`/`while`/`for` under
/// `have_tommath`, so capturing the call the way the shared
/// `vector_script` renderer normally does — `if {[catch {…} __r]} {…}`
/// — would report `invalid command name "if"` on the first vector when
/// libtommath is not vendored. That is a *capture-script* dependency,
/// not a dispatch one: nothing about command resolution needs the
/// numeric tower.
///
/// So the capture is composed here from the tower-free half of the
/// renderer (`vector_setup` + `vector_call`) with `set` and `catch`
/// standing in for `if` — `set __r -` then `catch {set __r [call]}`
/// leaves the dispatched name in `__r`, or `-` when the call raised,
/// which is exactly what the `if` form computes. This runs the vectors
/// in **every** build of this crate, tower or no tower.
#[test]
fn dispatch_matches_every_conformance_vector() {
    use tcl_syntax::naming::conformance::{vector_call, vector_setup, vectors};
    for v in vectors() {
        let script = format!(
            "{}set __r -\ncatch {{set __r [{}]}}\n",
            vector_setup(&v),
            vector_call(&v),
        );
        let body = script.clone();
        counters::reset();
        {
            let mut i = Interp::new();
            let code = i.eval_str(body.as_bytes());
            assert_eq!(
                code,
                Code::Ok,
                "vector line {}: runtime errored on script:\n{script}\nerror: {}",
                v.line,
                String::from_utf8_lossy(&i.result_bytes()),
            );
            assert_eq!(i.eval_str(b"set __r"), Code::Ok);
            let got = String::from_utf8_lossy(&i.result_bytes()).to_string();
            let want = v.want().unwrap_or_else(|| "-".to_string());
            assert_eq!(
                got, want,
                "vector line {} (ns={} path={:?} defs={:?} call={}): runtime dispatch \
                     disagrees with C Tcl\nscript:\n{script}",
                v.line, v.ns, v.path, v.defs, v.call,
            );
        }
        assert_eq!(counters::finalize(), 0, "vector line {}: leak", v.line);
        assert_eq!(counters::double_free_count(), 0);
    }
}

#[test]
fn build_info_queries() {
    // `tcl::build-info` (the tcltest constraint source): version/patchlevel
    // parse the build string; feature flags we don't set report 0.
    leak_free(|i| {
        assert_eq!(i.eval_str(b"tcl::build-info version"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9.0");
        assert_eq!(i.eval_str(b"tcl::build-info patchlevel"), Code::Ok);
        assert_eq!(i.result_bytes(), b"9.0.4");
        for feat in [&b"debug"[..], b"purify", b"memdebug", b"no-deprecate"] {
            let mut cmd = b"tcl::build-info ".to_vec();
            cmd.extend_from_slice(feat);
            assert_eq!(i.eval_str(&cmd), Code::Ok);
            assert_eq!(i.result_bytes(), b"0", "feature {feat:?} should be absent");
        }
    });
}

// Braced defining word.
//
// Defining a proc via a braced name word (`proc {p} {} {…}`) does not
// corrupt the command table: the braces are word quoting, the parser
// strips them, and `define_proc` never sees them.
//
// These pin the real behaviour directly, in tower-free command shapes,
// so the claim stays verifiable in *every* build rather than only where
// the numeric tower happens to be linked.

#[test]
fn braced_defining_word_defines_the_unbraced_name() {
    leak_free(|i| {
        // Real Tcl defines a command literally named `p`: the braces quote
        // the word, they are not part of the name.
        assert_eq!(i.eval_str(b"proc {p} {} { return {::p} }"), Code::Ok);
        assert_eq!(i.eval_str(b"::p"), Code::Ok, "absolute call must dispatch");
        assert_eq!(i.result_bytes(), b"::p");
        assert_eq!(i.eval_str(b"p"), Code::Ok, "bare call must dispatch");
        assert_eq!(i.result_bytes(), b"::p");
        assert_eq!(i.eval_str(b"namespace which -command ::p"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::p");
        // ... and no command named with the braces survives anywhere.
        assert_eq!(i.eval_str(b"namespace which -command {{p}}"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
    });
}

#[test]
fn braced_defining_word_leaves_builtin_dispatch_intact() {
    leak_free(|i| {
        // The strongest form of the claim: after defining a proc through
        // a braced name word, *unrelated builtins* must still resolve.
        // Only tower-free builtins are exercised so this holds in a
        // libtommath-less build too.
        assert_eq!(i.eval_str(b"proc {p} {} { return {::p} }"), Code::Ok);
        assert_eq!(i.eval_str(b"set __x 1"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"catch {::nosuch} __e"), Code::Ok);
        assert_eq!(i.result_bytes(), b"1");
        assert_eq!(i.eval_str(b"llength [list a b c]"), Code::Ok);
        assert_eq!(i.result_bytes(), b"3");
        assert_eq!(i.eval_str(b"string length abcd"), Code::Ok);
        assert_eq!(i.result_bytes(), b"4");
        // The global command table still holds the builtins by name.
        assert_eq!(i.eval_str(b"namespace which -command ::set"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::set");
        i.eval_str(b"unset __x");
        i.eval_str(b"unset __e");
    });
}

#[test]
fn braced_defining_word_binds_in_the_current_namespace() {
    leak_free(|i| {
        // The same quoting rule inside a namespace: `proc {q}` in `::ns`
        // binds `::ns::q`, resolvable bare from inside and absolutely from
        // outside, with the global table untouched.
        assert_eq!(
            i.eval_str(b"namespace eval ns { proc {q} {} { return {::ns::q} } }"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"::ns::q"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::ns::q");
        assert_eq!(i.eval_str(b"namespace eval ns { q }"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::ns::q");
        assert_eq!(i.eval_str(b"namespace which -command ::q"), Code::Ok);
        assert_eq!(i.result_bytes(), b"");
    });
}

/// TclOO's root object commands are engine-installed on the registry's
/// behalf, so they follow their introducing release; a script-created
/// object command does not.
#[test]
fn tcloo_roots_follow_their_introducing_release() {
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V8_4);
        assert_eq!(
            i.eval_str(b"catch {oo::class create C {}} m; set m"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"invalid command name \"oo::class\"");
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            i.eval_str(b"catch {oo::configurable create C {}} m; set m"),
            Code::Ok
        );
        assert_eq!(
            i.result_bytes(),
            b"invalid command name \"oo::configurable\""
        );
        // A user object named after a 9.0-only builtin stays callable.
        assert_eq!(
            i.eval_str(b"oo::class create lpop {method m {} {return M}}; [lpop new] m"),
            Code::Ok
        );
        assert_eq!(i.result_bytes(), b"M");
        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
        i.eval_str(b"unset -nocomplain m");
    });
}

/// Taking a gate-hidden root's name must work through *every* registration
/// verb, not just `create`. The root marking is an identity on the entry,
/// so it is cleared in the one registration funnel (`ns_register`); these
/// two vectors are the funnels that do not go through `create`.
#[test]
fn taking_a_hidden_root_name_works_through_copy_and_rename() {
    // `oo::copy` onto the hidden name: the copy must be callable and listed.
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            i.eval_str(b"oo::class create Src {method m {} {return SRC}}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"oo::copy ::Src ::oo::configurable"), Code::Ok);
        assert_eq!(i.eval_str(b"[oo::configurable new] m"), Code::Ok);
        assert_eq!(i.result_bytes(), b"SRC");
        assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::oo::configurable");
        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
    });
    // `rename` onto the hidden name: same contract.
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(i.eval_str(b"oo::object create ::mysrc"), Code::Ok);
        assert_eq!(i.eval_str(b"rename ::mysrc ::oo::configurable"), Code::Ok);
        assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::oo::configurable");
        assert_eq!(i.eval_str(b"::oo::configurable destroy"), Code::Ok);
        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
    });
}

/// Taking a gate-hidden root's name via `rename` keeps working — the root
/// marking is cleared on the rename path itself, not only by the OO
/// re-registration that follows it.
#[test]
fn renaming_onto_a_hidden_root_name_keeps_the_replacement_live() {
    leak_free(|i| {
        i.set_runtime_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            i.eval_str(b"oo::class create ::Src {method m {} {return SRC}}"),
            Code::Ok
        );
        assert_eq!(i.eval_str(b"rename ::Src ::oo::configurable"), Code::Ok);
        assert_eq!(i.eval_str(b"info commands ::oo::configurable"), Code::Ok);
        assert_eq!(i.result_bytes(), b"::oo::configurable");
        assert_eq!(i.eval_str(b"[::oo::configurable new] m"), Code::Ok);
        assert_eq!(i.result_bytes(), b"SRC");
        i.set_runtime_version(tcl_dialect::TclVersion::V9_0);
    });
}

/// The byte-valued `Namespaces` spellings resolve a name the `&str` ones
/// cannot express at all.
///
/// Everything reachable from a *script* arrives here as valid UTF-8 — a
/// byte array's string rep is one U+00XX per byte (`bytearray.rs`), so the
/// old `from_utf8_lossy` hop was a no-op on every scripted path, which is
/// why the surface test above passes either way. The hole it leaves is the
/// one `binary_bytes` already documents: an embedder may hand the C ABI a
/// plain string that is not UTF-8. This exercises that seam directly, so it
/// fails if the byte-valued entry points are ever routed back through
/// `str`.
#[test]
fn byte_valued_command_names_resolve_without_a_utf8_round_trip() {
    use tcl_runtime_api::Namespaces;

    leak_free(|i| {
        // A command name that is not valid UTF-8 in any encoding.
        let raw: &[u8] = b"::raw\xff\xfename";
        i.ns_register(
            raw,
            crate::interp::Command::Builtin(|interp, _| {
                interp.set_result_bytes(b"RAW");
                Code::Ok
            }),
        );
        let global = tcl_runtime_api::NsId(crate::namespace::GLOBAL as u32);
        let id = i
            .find_command_bytes(global, raw)
            .expect("a byte-valued name resolves through the byte-valued spelling");
        assert_eq!(i.command_name_bytes(id).as_deref(), Some(raw));
        // The shared cores reach the same answer.
        assert_eq!(
            tcl_cmd_core::namespace::origin_bytes_checked(i, raw)
                .unwrap()
                .as_deref(),
            Some(raw)
        );
        assert_eq!(
            tcl_cmd_core::namespace::which_command_bytes(i, raw).as_deref(),
            Some(raw)
        );
        // The lossy `&str` spelling genuinely cannot: the replacement
        // characters name a different (absent) command. This is the
        // divergence the byte-valued entry points exist to remove.
        assert_eq!(
            tcl_cmd_core::namespace::origin_bytes_checked(
                i,
                String::from_utf8_lossy(raw).as_bytes()
            )
            .unwrap(),
            None
        );
    });
}
