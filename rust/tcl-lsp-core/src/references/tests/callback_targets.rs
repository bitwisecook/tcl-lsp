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

//! Method words captured into callbacks: stored prefixes, list-built self callbacks, visibility and inherited receivers.

use super::*;

#[test]
fn references_for_method_includes_one_hop_stored_callback_prefixes() {
    let src = "package require Tk\noo::class create C {\n    method tick {} {}\n    method setup {} {\n        set external [list [self] tick]\n        bind .w <Button-1> $external\n        set internal [namespace code [list my tick]]\n        after 0 $internal\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&2), "declaration missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "stored [self] callback missing: {refs:?}"
    );
    assert!(
        lines.contains(&6),
        "stored namespace-code callback missing: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_abstain_on_reassignment_and_dynamic_methods() {
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {method} {\n        set cb [list my tick]\n        lappend cb suffix\n        set cb [list my $method]\n        bind .w <Button-1> $cb\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "ambiguous stored callback must abstain: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_count_nested_same_frame_reassignment() {
    let src = "oo::class create C {\n    method tick {} {}\n    method other {} {}\n    method setup {} {\n        set cb [list my tick]\n        if 1 {\n            set cb [list my other]\n            after 0 $cb\n        }\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "a nested reassignment makes the stored callback ambiguous: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_abstain_on_non_set_varwrite_reassignment() {
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set cb [list my tick]\n        lappend cb suffix\n        after 0 $cb\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "non-Set VarWrite reassignment must abstain: {refs:?}"
    );
}

#[test]
fn stored_callback_prefixes_abstain_on_qualified_and_aliased_variables() {
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set ::cb [list my tick]\n        bind .w <Button-1> $::cb\n        global local\n        set local [list my tick]\n        after 0 $local\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(
        lines,
        vec![1],
        "qualified/aliased storage must abstain: {refs:?}"
    );
}

#[test]
fn stored_callback_prefix_cursor_resolves_to_method_declaration() {
    // tclsh 8.6.18/9.1.0: `after` runs a bare [list my tick] at global level
    // (invalid command name "my"); namespace code runs it in the object.
    let src = "oo::class create C {\n    method tick {} {}\n    method setup {} {\n        set cb [namespace code [list my tick]]\n        after 0 $cb\n    }\n}\n[C new] setup\n";
    let analysis = analyse(src);
    let col = u32::try_from(src.lines().nth(3).unwrap().find("tick").unwrap()).unwrap();
    let locations = crate::definition::definition(src, 3, col, &analysis);
    assert_eq!(
        locations.len(),
        1,
        "stored callback cursor must resolve: {locations:?}"
    );
    assert_eq!(
        locations[0].start_line, 1,
        "wrong stored callback target: {locations:?}"
    );
}

#[test]
fn references_from_decl_reach_a_list_built_self_callback() {
    // `bind` receives a deferred script built as a Tcl list.
    // `[self]` is substituted while the method frame is live, leaving an
    // object-command prefix that later dispatches `animTick` externally.
    let src = "package require Tk\noo::class create C {\n    method animTick {} { return 1 }\n    method anim {wl} {\n        bind $wl <ButtonPress-1> [list [self] animTick %x %y]\n    }\n}\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&2), "decl missing: {refs:?}");
    assert!(
        lines.contains(&4),
        "the list-built bind callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_a_list_built_self_object_callback() {
    let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self object] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 4),
        "the `[self object]` callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_a_list_built_self_after_callback() {
    // A real corpus shape: Pave and Zesty both schedule methods this
    // way. `after` exposes its script through the same registry Body role
    // as `bind`, so no scheduler-specific branch belongs in this scan.
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the list-built after callback is missing: {refs:?}"
    );
}

/// A tcllib-shaped fixture read from the real files rather than a
/// hand-written imitation.
///
/// The original fixed upstream bytes are required; missing or changed
/// corpus setup cannot count as observed callback semantics.
#[test]
fn the_real_tcllib_callback_shapes_resolve_to_their_methods() {
    let corpus = std::env::var_os("TCLLIB_2_0_DIR").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/tcllib-2.0"),
        std::path::PathBuf::from,
    );
    // (fixture, method declared in the file, 0-based lines of the
    // callback prefixes that name it). `cat.tcl` is the `after …
    // [namespace code [list my Post $c]]` shape; `httpd.tcl` (Clay classes,
    // Tcl 8.6) is the `socket -server [namespace code [list my connect]]
    // ?-myaddr $ip? $port` shape (#1956).
    for (fixture, method, callbacks) in [
        (
            &tcl_test_support::fixed_corpus::TCLLIB_2_0_CHANNEL_CAT,
            "Post",
            &[][..],
        ),
        (
            &tcl_test_support::fixed_corpus::TCLLIB_2_0_HTTPD,
            "connect",
            &[1010_u32, 1013][..],
        ),
    ] {
        let relative = fixture.relative_path;
        let original = tcl_test_support::fixed_corpus::FixedCorpusSource::read(&corpus, fixture)
            .expect("fixed Tcllib 2.0 source setup must be available and unchanged");
        let src = String::from_utf8(original.bytes().to_vec())
            .expect("the original Tcllib callback control is UTF-8");
        let analysis = analyse(&src);
        let declaration = src
            .find(&format!("method {method} "))
            .unwrap_or_else(|| panic!("{relative} declares `method {method}`"));
        let line = u32::try_from(src[..declaration].lines().count() - 1).expect("line fits");
        let column = u32::try_from(
            src[..declaration]
                .rsplit('\n')
                .next()
                .unwrap_or_default()
                .len()
                + "method ".len(),
        )
        .expect("column fits");
        let refs = references(
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            line,
            column,
            &analysis,
            true,
        );
        assert!(
            refs.len() > 1,
            "{relative}: `{method}` must reach its callback prefix as well as its \
                 declaration, got {refs:?}"
        );
        for callback in callbacks {
            assert!(
                refs.iter().any(|r| r.start_line == *callback),
                "{relative}: `{method}` must reach the callback on line {callback}: {refs:?}"
            );
        }
    }
}

/// A `clay::define` class is a `TclOO` class: its lowercase `method` names a
/// `TclOO` method and its `superclass` a `TclOO` relation, with Clay's
/// `::clay::object` root implicit as `::oo::object` is, so a `my` call
/// reaches the method a class or its superclass declares (#1956). Tcl 8.6.
#[test]
fn references_reach_a_my_call_in_a_clay_class() {
    for src in [
        "clay::define ::C {\n    method connect {sock} {}\n    method run {} { my connect x }\n}\n",
        "clay::define ::B {\n    method connect {sock} {}\n}\nclay::define ::C {\n    superclass ::B\n    method run {} { my connect x }\n}\n",
    ] {
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line != 1),
            "the `my connect` call is missing: {src}: {refs:?}"
        );
    }
}

/// `socket` scans options while a word starts with `-` and keeps the last
/// word as the port (`Tcl_SocketObjCmd`, 8.4.20-9.1.0), so the accept
/// callback is found with a dynamic port and later options. Tcl 8.6.
#[test]
fn references_reach_a_socket_server_callback_with_a_dynamic_port() {
    for call in [
        "socket -server [namespace code [list my connect]] $port",
        "socket -server [namespace code [list my connect]] -myaddr $ip $port",
    ] {
        let src = format!(
            "oo::class create C {{\n    method connect {{sock ip port}} {{}}\n    method start {{ip port}} {{\n        {call}\n    }}\n}}\n"
        );
        let refs = references(
            &src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(&src),
            true,
        );
        assert!(
            refs.iter().any(|r| r.start_line == 3),
            "the socket accept callback is missing: {call}: {refs:?}"
        );
    }
}

/// A `{*}`-expanded callback word must abstain.
///
/// `{*}` splices the word's value into the argument list, so the registry's
/// role indices no longer describe where anything landed. Reading the word
/// as if it were the callback slot is not merely unjustified, it invents
/// references: after expansion `lsort -command {*}[list [self] compare]
/// $items` runs the bare *object command* as the comparator and passes
/// `compare` as a separate argument, so `compare` is not dispatched at all
/// — and Rename would have rewritten it.
#[test]
fn an_expanded_callback_word_invents_no_reference() {
    for src in [
        // The comparator is the object command; `compare` is an argument.
        "oo::class create C {\n    method compare {a b} { return 0 }\n    method sort {items} {\n        lsort -command {*}[list [self] compare] $items\n    }\n}\n",
        // Same rule for a Body slot, where the arity happens to work out.
        "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle {*}[list [self] tick]\n    }\n}\n",
        // Inside a `WRAPS_COMMAND_PREFIX` wrapper:
        // `namespace code` then has two arguments and errors rather than
        // dispatching anything, so there is nothing to reference.
        "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [namespace code {*}[list my tick]]\n    }\n}\n",
        // And inside the builder itself: the receiver is no longer word 1.
        "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list {*}$prefix tick]\n    }\n}\n",
    ] {
        let refs = references(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
            1,
            11,
            &analyse(src),
            true,
        );
        assert!(
            !refs.iter().any(|r| r.start_line == 3),
            "an expanded word is not a callback slot: {src:?} gave {refs:?}"
        );
    }
}

/// The gate is about expansion, not about the shape around it: the same
/// prefix written as a sole substitution still resolves.
#[test]
fn references_reach_a_list_built_self_command_prefix() {
    // CommandPrefix is distinct from Body: the registry says this word is
    // a partial command and the consumer appends its own arguments. The
    // object-method reference is nevertheless the same external dispatch.
    let src = "oo::class create C {\n    method compare {a b} { return 0 }\n    method sort {items} {\n        lsort -command [list [self] compare] $items\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the list-built CommandPrefix callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_namespace_wrapped_my_callback_even_when_private() {
    // tcllib's virtchannel_base shape: the registry declares
    // `namespace code` as a WRAPS_COMMAND_PREFIX and `list` as a
    // BUILDS_COMMAND_PREFIX.  `my` keeps private current-object dispatch,
    // unlike a captured `[self]` object command.
    let src = "oo::class create C {\n    method read {} { return 1 }\n    unexport read\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my read]]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 4),
        "the wrapped private `my read` callback is missing: {refs:?}"
    );
}

#[test]
fn references_reach_namespace_wrapped_self_callback_only_when_public() {
    let src = "oo::class create C {\n    method changed {} { return 1 }\n    method wire {} {\n        trace add variable v write [namespace code [list [self] changed]]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.0").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the wrapped public `[self] changed` callback is missing: {refs:?}"
    );
}

#[test]
fn a_mixin_branch_decides_its_own_providers_visibility() {
    // A mixin that inherits the member from its
    // own superclass and unexports the name empties *its* branch only —
    // the spine still answers.  tclsh 8.6.16 / 9.0.4 both run `A`'s body:
    //   oo::class create MChild { superclass MBase } ; unexport m
    //   oo::class create D { superclass A ; mixin MChild }
    //   [D new] m  ->  A-m   (not MBase-m)
    let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MChild {\n    superclass MBase\n    unexport m\n}\noo::class create D {\n    superclass A\n    mixin MChild\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n[D new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
        Some(("::A".to_owned(), false)),
        "the suppressed mixin branch must not provide the capture"
    );
    assert!(
        !references(src, dialect, 4, 11, &analysis, true)
            .iter()
            .any(|range| range.start_line == 14),
        "`MBase::m` must not collect a capture its branch cannot dispatch"
    );
    assert!(
        references(src, dialect, 1, 11, &analysis, true)
            .iter()
            .any(|range| range.start_line == 14),
        "`A::m` — the provider the receiver actually reaches — must collect it"
    );
}

#[test]
fn a_live_mixin_branch_still_outranks_the_spine() {
    // The control for the case above: with the branch left public, the
    // mixin's inherited `MBase::m` is what `[D new] m` enters (tclsh
    // 8.6.16 / 9.0.4 -> MBase-m), so the capture belongs to *its* family.
    let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MPub {\n    superclass MBase\n}\noo::class create D {\n    superclass A\n    mixin MPub\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n[D new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
        Some(("::MBase".to_owned(), false)),
        "a live mixin branch provides the capture ahead of the spine"
    );
}

#[test]
fn a_mixin_branch_can_revive_its_own_bases_unexported_member() {
    // The mirror direction inside a branch: `NBase` unexports its own `n`,
    // the mixin `NChild` exports the inherited name, and tclsh 8.6.16 /
    // 9.0.4 run `NBase`'s body for `[D new] n` — ahead of the spine's
    // public `A2::n`.
    let src = "oo::class create A2 {\n    method n {} { return 1 }\n}\noo::class create NBase {\n    method n {} { return 2 }\n    unexport n\n}\noo::class create NChild {\n    superclass NBase\n    export n\n}\noo::class create D {\n    superclass A2\n    mixin NChild\n    method wire {} {\n        after idle [list [self] n]\n    }\n}\n[D new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] n]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "n", cursor),
        Some(("::NBase".to_owned(), false)),
        "the branch's own export revives its base's body"
    );
}

#[test]
fn a_mixin_declared_on_a_superclass_governs_its_branch_too() {
    // Branch roots are collected transitively, so a `mixin` written on a
    // superclass behaves exactly as one written on the receiver: tclsh
    // 8.6.16 / 9.0.4 answer `[D new] m` with `A-m`.
    let src = "oo::class create A {\n    method m {} { return 1 }\n}\noo::class create MBase {\n    method m {} { return 2 }\n}\noo::class create MChild {\n    superclass MBase\n    unexport m\n}\noo::class create S {\n    superclass A\n    mixin MChild\n}\noo::class create D {\n    superclass S\n    method wire {} {\n        after idle [list [self] m]\n    }\n}\n[D new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] m]").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "m", cursor),
        Some(("::A".to_owned(), false)),
        "a superclass-declared mixin's suppression empties that branch only"
    );
}

#[test]
fn inert_list_built_self_value_is_not_a_method_reference() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {} {\n        set inert [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "an inert list value must not become a call site: {refs:?}"
    );
}

#[test]
fn dynamic_list_built_self_method_remains_unresolved() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {methodName} {\n        after idle [list [self] $methodName]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "a dynamic method word must remain an abstention: {refs:?}"
    );
}

#[test]
fn compound_list_built_self_word_is_not_the_spelled_method() {
    // The list substitution produces an object command ending in `tick`,
    // then Tcl concatenates `Suffix` onto that word. The invoked method is
    // `tickSuffix`; rewriting the inner `tick` token would be incorrect.
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method build {} {\n        after idle [list [self] tick]Suffix\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "a method fragment inside a compound word is not an exact reference: {refs:?}"
    );
}

#[test]
fn deferred_braced_self_script_is_not_a_method_reference() {
    // The braces delay `[self]` itself until Tk runs the binding, after
    // the defining method frame has gone.  This script cannot resolve the
    // object and must not be credited as a method call.
    let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> {[self] tick}\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 4),
        "a later, out-of-frame `self` must not become a call site: {refs:?}"
    );
}

#[test]
fn list_built_self_callback_cannot_reach_an_unexported_method() {
    // A captured `[self]` result is an object command.  Its later call is
    // therefore external and cannot dispatch an unexported method.
    let src = "package require Tk\noo::class create C {\n    method tick {} { return 1 }\n    unexport tick\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 5),
        "an unexported method is not callable through the captured object: {refs:?}"
    );
}

#[test]
fn wrapped_my_callback_cursor_resolves_private_dispatch() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    unexport tick\n    method wire {chan} {\n        fileevent $chan readable [namespace code [list my tick]]\n    }\n}\n[C new] wire stdin\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let cursor = u32::try_from(src.rfind("tick]").unwrap()).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            &analysis,
            "tick",
            cursor,
        ),
        Some(("::C".to_owned(), false)),
    );
}

#[test]
fn a_shadowing_list_command_does_not_build_a_callback_reference() {
    let src = "package require Tk\nproc list args { return not-a-command-prefix }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 5),
        "a user command shadowing the registry builder must not inherit its semantics: {refs:?}"
    );
}

#[test]
fn a_global_self_command_does_not_shadow_the_tcloo_method_helper() {
    let src = "package require Tk\nproc self args { return ::notTheCurrentObject }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {wl} {\n        bind $wl <Expose> [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        3,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 5),
        "::oo::Helpers precedes global fallback on a method frame: {refs:?}"
    );
}

#[test]
fn malformed_self_object_receiver_is_not_a_callback_reference() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [self object junk] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 3),
        "a receiver call that errors cannot build a callback: {refs:?}"
    );
}

#[test]
fn explicitly_global_self_is_not_the_tcloo_method_helper() {
    let src = "proc ::self {} { return ::notTheCurrentObject }\noo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [::self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        2,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 4),
        "an absolute global receiver must not inherit method-path semantics: {refs:?}"
    );
}

#[test]
fn qualified_tcloo_self_helper_builds_the_same_callback() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [list [::oo::Helpers::self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        refs.iter().any(|r| r.start_line == 3),
        "the registry-declared qualified helper must remain valid: {refs:?}"
    );
}

#[test]
fn callback_cursor_classifier_reaches_constructor_and_destructor_bodies() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    constructor {} {\n        after idle [list [self] tick]\n    }\n    destructor {\n        after idle [list [self] tick]\n    }\n}\n[C new] destroy\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let first = src.find("] tick").unwrap() + 2;
    let second = src[first + 1..].find("] tick").unwrap() + first + 3;
    for cursor in [first, second] {
        assert_eq!(
            list_built_self_method_target_at_cursor(
                src,
                dialect,
                &analysis,
                "tick",
                u32::try_from(cursor).unwrap(),
            ),
            Some(("::C".to_owned(), false)),
        );
    }
}

#[test]
fn callback_cursor_classifier_rejects_ordinary_method_dispatches() {
    let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        my tick\n        [self] tick\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let my_cursor = u32::try_from(src.find("my tick").unwrap() + 3).unwrap();
    let self_cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", my_cursor,)
            .is_none()
    );
    assert!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", self_cursor,)
            .is_none()
    );
}

#[test]
fn inherited_callback_abstains_without_effective_receiver_visibility() {
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    unexport tick\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analyse(src),
        true,
    );
    assert!(
        !refs.iter().any(|r| r.start_line == 7),
        "provider visibility cannot be reused for the child receiver: {refs:?}"
    );
}

#[test]
fn inherited_callback_joins_the_providers_reference_set() {
    // A captured `[self]` object command in an inheriting class
    // reaches the provider's exported implementation — tclsh 8.6.16 /
    // 9.0.4 both run `Base`'s body for `[list [Child new] tick]`.  Both
    // directions must agree on that, or rename would edit one and not the
    // other.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n[Child new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
        Some(("::Base".to_owned(), false)),
        "the inherited callback resolves to its effective provider"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        refs.iter().any(|range| range.start_line == 6),
        "declaration-origin references must collect the same site: {refs:?}"
    );
}

#[test]
fn a_receiver_that_unexports_the_inherited_name_abstains_both_ways() {
    // The receiver's own `unexport` decides, not the provider's
    // declaration: tclsh 8.6.16 / 9.0.4 answer `[Child new] tick` with
    // `unknown method "tick"` even though `Base` exports it, so the
    // capture is not a call site of `Base::tick` at all.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    unexport tick\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor).is_none(),
        "an unexported receiver name must not resolve a captured [self] callback"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        !refs.iter().any(|range| range.start_line == 7),
        "declaration-origin references must make the same abstention: {refs:?}"
    );
}

#[test]
fn a_receiver_that_reexports_an_unexported_provider_resolves_both_ways() {
    // The mirror case, and the one a provider-only reading gets wrong in
    // the other direction: `Base` unexports its own `tock`, `Child`
    // exports the inherited name, and tclsh 8.6.16 / 9.0.4 run `Base`'s
    // body for `[Child new] tock`.
    let src = "oo::class create Base {\n    method tock {} { return 1 }\n    unexport tock\n}\noo::class create Child {\n    superclass Base\n    export tock\n    method wire {} {\n        after idle [list [self] tock]\n    }\n}\n[Child new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tock").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tock", cursor),
        Some(("::Base".to_owned(), false)),
        "a re-exporting receiver revives the inherited callback target"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        refs.iter().any(|range| range.start_line == 8),
        "declaration-origin references must collect the same site: {refs:?}"
    );
}

#[test]
fn an_overriding_receiver_keeps_its_callback_in_its_own_family() {
    // `Child` declares its own `tick`, so the capture dispatches there.
    // `Base::tick`'s family must not claim it (renaming `Base::tick`
    // would otherwise rewrite a word that never called it).
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Child {\n    superclass Base\n    method tick {} { return 2 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n[Child new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
        Some(("::Child".to_owned(), false)),
        "the override, not the base, provides this capture"
    );

    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        !refs.iter().any(|range| range.start_line == 7),
        "the base declaration must not collect the override's capture: {refs:?}"
    );
}

#[test]
fn an_unrelated_class_does_not_collect_a_same_named_callback() {
    // No `superclass` edge, so nothing links the two `tick`s.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Other {\n    method tick {} { return 2 }\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n";
    let analysis = analyse(src);
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let refs = references(src, dialect, 1, 11, &analysis, true);
    assert!(
        !refs.iter().any(|range| range.start_line == 6),
        "an unrelated class's capture is not a reference: {refs:?}"
    );
}

#[test]
fn a_mixins_unexport_does_not_suppress_the_superclass_capture() {
    // C enters each mixin with a fresh copy of the dispatch flags, so a
    // mixin that unexports the name empties only its own branch: tclsh
    // 8.6.16 / 9.0.4 still run `Base`'s `tick` for `[Child new] tick`.
    let src = "oo::class create Base {\n    method tick {} { return 1 }\n}\noo::class create Mix {\n    unexport tick\n}\noo::class create Child {\n    superclass Base\n    mixin Mix\n    method wire {} {\n        after idle [list [self] tick]\n    }\n}\n[Child new] wire\n";
    let analysis = Analyser::new().analyse(src, "tcl").clone();
    let dialect = tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile();
    let cursor = u32::try_from(src.find("] tick").unwrap() + 2).unwrap();
    assert_eq!(
        list_built_self_method_target_at_cursor(src, dialect, &analysis, "tick", cursor),
        Some(("::Base".to_owned(), false)),
        "a mixin's unexport must not suppress the spine's provider"
    );
}

#[test]
fn callback_source_roles_keep_actual_availability_and_original_builder_geometry() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Conditional source reference spans; no Native callback, receiver or edit grant.
    let store = callback_source_store("dict");
    for source in [
        "source_callback [list source_dispatch selected]",
        "set cb [list source_dispatch selected]; source_callback $cb",
        "source_callback [namespace code [list source_dispatch selected]]",
        "interp alias {} install {} source_callback; install [list source_dispatch selected]",
        "interp alias {} make {} list source_dispatch; source_callback [make selected]",
        "interp alias {} target {} source_dispatch selected; source_callback [list target]",
    ] {
        let mut analysis = callback_source_analysis_with_store(source, "tcl8.6", store.clone());
        let spans = callback_source_spans(source, &analysis, "selected");
        assert_eq!(spans.len(), 1, "{source}: {spans:?}");
        assert_eq!(source_span_text(source, spans[0]), "selected");
        assert!(
            callback_source_spans(
                source,
                &callback_source_analysis_with_store(source, "tcl8.4", store.clone()),
                "selected"
            )
            .is_empty(),
            "{source}"
        );
        let config = analysis.body_lexer_config.unwrap();
        let mut missing_point = AnalysisResult::default();
        missing_point.resolved_input = analysis.resolved_input.clone();
        missing_point.body_lexer_config = Some(config);
        assert_eq!(
            callback_source_spans(source, &missing_point, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax = !config.expand_syntax;
        assert_eq!(
            callback_source_spans(source, &analysis, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.body_lexer_config = Some(config);
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            tcl_dialect::DialectProfile::plain_tcl(),
            tcl_dialect::DialectProfile::plain_tcl(),
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry(),
            config,
        ));
        assert_eq!(
            callback_source_spans(source, &analysis, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.resolved_input = None;
        assert_eq!(
            callback_source_spans(source, &analysis, "selected"),
            [] as [tcl_lexer::Span; 0]
        );
    }
}

#[test]
fn callback_source_roles_keep_actual_package_availability_in_the_same_store() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "set cb [list source_dispatch selected]; source_callback $cb";
    let store = callback_source_store("bind");
    let available = callback_source_analysis_with_store(source, "tk", store.clone());
    let unavailable = callback_source_analysis_with_store(source, "tcl8.6", store);
    assert_eq!(
        callback_source_spans(source, &available, "selected").len(),
        1
    );
    assert_eq!(
        callback_source_spans(source, &unavailable, "selected"),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn callback_source_targets_withdraw_known_shadows_and_incomplete_stored_writes() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    for source in [
        "proc source_callback args {}; source_callback [list source_dispatch selected]",
        "proc list args {}; source_callback [list source_dispatch selected]",
        "proc source_dispatch args {}; source_callback [list source_dispatch selected]",
        "rename source_dispatch {}; source_callback [list source_dispatch selected]",
        "set cb [list source_dispatch selected]; opaque $cb; source_callback $cb",
        "set cb [list source_dispatch selected]; set $unknown other; source_callback $cb",
        "source_callback [list $unknown selected]",
        "interp alias {} target {} source_dispatch selected; rename target {}; source_callback [list target]",
        "source_callback {*}[list source_dispatch selected]",
        "source_callback [list source_dispatch selected]Suffix",
    ] {
        let analysis = callback_source_analysis(source, "tcl8.6");
        assert!(
            callback_source_spans(source, &analysis, "selected").is_empty(),
            "{source}"
        );
    }
}

/// A callback selector in a member body the analysis never entered is a
/// read-only reference to the method it names, never a rename edit: rename
/// keeps it as the whole-rename hazard. A rebound builder or a computed
/// selector reports nothing (#1956). Tcl 8.6 and 9.0.
#[test]
fn an_unentered_callback_selector_is_a_reference_but_never_a_rename_edit() {
    for release in ["tcl8.6", "tcl9.0"] {
        let environment = tcl_registry::model::ingress::resolve_environment(release);
        let profile = environment.analyser_profile();
        let registry = environment.default_context_registry();
        let src = "oo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [namespace code [list my tick]]\n    }\n}\n";
        let analysis = Analyser::new().analyse(src, release).clone();
        let refs = references(src, profile, 1, 11, &analysis, true);
        assert!(
            refs.iter().any(|r| r.start_line == 3),
            "{release}: the unentered callback is missing: {refs:?}"
        );
        let edits = crate::rename::rename(
            src,
            profile,
            1,
            11,
            "tock",
            &analysis,
            Some(registry.commands()),
        );
        assert!(
            edits.iter().all(|edit| edit.range.start_line != 3),
            "{release}: rename edited the unentered callback: {edits:?}"
        );
        for abstains in [
            "proc list args {}\noo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [namespace code [list my tick]]\n    }\n}\n",
            "rename my __my\noo::class create C {\n    method tick {} { return 1 }\n    method wire {} {\n        after idle [namespace code [list my tick]]\n    }\n}\n",
            "oo::class create C {\n    method tick {} { return 1 }\n    method wire {n} {\n        after idle [namespace code [list my $n]]\n    }\n}\n",
        ] {
            let analysis = Analyser::new().analyse(abstains, release).clone();
            let line = u32::from(!abstains.starts_with("oo::class"));
            let refs = references(abstains, profile, line + 1, 11, &analysis, true);
            assert!(
                refs.iter().all(|r| r.start_line != line + 3),
                "{release}: {abstains}: {refs:?}"
            );
        }
    }
}
