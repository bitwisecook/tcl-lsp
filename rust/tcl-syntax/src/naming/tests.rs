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

#[test]
fn namespace_hint_tail_is_one_literal_join_after_the_prefix() {
    use super::{authored_namespace_hint_tail, authored_namespace_hint_tail_bytes};
    let cases: [(&str, &str, Option<&str>); 14] = [
        ("a::b", "a", Some("b")),
        ("a::b::c", "a", Some("b::c")),
        ("a::b::c", "a::b", Some("c")),
        ("::a::b", "::a", Some("b")),
        // An empty tail is the empty-named member.
        ("a::", "a", Some("")),
        // Only one join is removed; a longer colon run stays in the tail.
        ("a:::b", "a", Some(":b")),
        ("a::::b", "a", Some("::b")),
        // The prefix is literal: no rooting, no shared-prefix match.
        ("::a::b", "a", None),
        ("a::b", "::a", None),
        ("ab::c", "a", None),
        ("a:b", "a", None),
        ("a", "a", None),
        // The root has no prefix to remove.
        ("::b", "", None),
        ("", "", None),
    ];
    for (name, namespace, tail) in cases {
        assert_eq!(
            authored_namespace_hint_tail(name, namespace),
            tail,
            "{name:?} under {namespace:?}"
        );
        assert_eq!(
            authored_namespace_hint_tail_bytes(name.as_bytes(), namespace.as_bytes()),
            tail.map(str::as_bytes),
            "{name:?} under {namespace:?}"
        );
    }
    // Counted bytes need not be UTF-8, and a NUL is ordinary data.
    assert_eq!(
        authored_namespace_hint_tail_bytes(b"\xff\0::\xfe", b"\xff\0"),
        Some(b"\xfe".as_slice())
    );
    // The string twin returns a slice of its input at a character boundary.
    assert_eq!(authored_namespace_hint_tail("é::ü", "é"), Some("ü"));
}

#[test]
fn unattributed_alias_hints_preserve_unavailable_value_boundaries() {
    let config = tcl_lexer::LexerConfig::default();
    for reference in ["$selector", "${selector}"] {
        assert_eq!(
            unattributed_variable_alias_hint(reference, config, |name| (name == "selector")
                .then_some("target")),
            Some("target")
        );
        for value in ["", "$literal", "[literal]", "has space"] {
            assert_eq!(
                unattributed_variable_alias_hint(reference, config, |_| Some(value)),
                None
            );
        }
    }
    for reference in [
        "plain",
        "$",
        "$array(key)",
        "${array(key)}",
        "${selector}suffix",
    ] {
        assert_eq!(
            unattributed_variable_alias_hint(reference, config, |_| Some("target")),
            None
        );
    }
    for (reference, name) in [
        ("${}", ""),
        ("${$literal}", "$literal"),
        ("${[literal]}", "[literal]"),
    ] {
        assert_eq!(
            unattributed_variable_alias_hint(reference, config, |read| (read == name)
                .then_some("target")),
            Some("target")
        );
        assert_eq!(
            unattributed_variable_alias_hint(reference, config, |_| None),
            None
        );
    }
}

#[test]
fn unattributed_rooted_candidates_preserve_data_and_refuse_lookup_authority() {
    let text = "literal {::old::member} [::other] $::array(index)\u{2003}\":::lone\" ; :relative ::empty::";
    assert_eq!(
        unattributed_rooted_name_candidates(text).collect::<Vec<_>>(),
        [
            "::old::member",
            "::other",
            "::array",
            ":::lone",
            "::empty::"
        ]
    );
    assert_eq!(
        unattributed_rooted_name_candidates("$${::old(index)}").collect::<Vec<_>>(),
        ["::old"]
    );
    assert_eq!(
        unattributed_rooted_name_candidates(":: :relative plain").collect::<Vec<_>>(),
        ["::"]
    );
}

#[test]
fn command_lookup_bases_visit_path_only_after_current() {
    use super::{CommandLookupBase, command_lookup_bases};
    let reached = std::cell::Cell::new(0);
    let path = [20, 30]
        .into_iter()
        .inspect(|_| reached.set(reached.get() + 1));
    let mut bases = command_lookup_bases(false, 10, path);
    assert_eq!(bases.next(), Some(CommandLookupBase::Context(10)));
    assert_eq!(reached.get(), 0);
    assert_eq!(bases.next(), Some(CommandLookupBase::Context(20)));
    assert_eq!(reached.get(), 1);
    assert_eq!(bases.next(), Some(CommandLookupBase::Context(30)));
    assert_eq!(bases.next(), Some(CommandLookupBase::Root));
    assert_eq!(bases.next(), None);
    assert_eq!(bases.next(), None);
}

#[test]
fn absolute_command_lookup_never_visits_context_or_path() {
    use super::{CommandLookupBase, command_lookup_bases};
    let path = std::iter::once(20).inspect(|_| panic!("absolute lookup reached path"));
    let mut bases = command_lookup_bases(true, 10, path);
    assert_eq!(bases.next(), Some(CommandLookupBase::Root));
    assert_eq!(bases.next(), None);
    assert_eq!(bases.next(), None);
}

#[test]
fn scalar_parentheses_share_the_closed_element_partition() {
    // Implementation contract: naming.expression.selected-reference-root
    // docs/design/analysis/name-resolution-proofs/expression-selected-reference-root.md
    // Native341 public values establish the distinct scalar/element cases;
    // these assertions bind only the source-name projection, not a cell.
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        for (source, root) in [
            ("scalar(open", "scalar(open"),
            ("scalar(open)tail", "scalar(open)tail"),
            ("${scalar(open}", "scalar(open"),
            ("${scalar(open)tail}", "scalar(open)tail"),
            ("${arr(first(second)}", "arr"),
            ("${cash$name}", "cash$name"),
        ] {
            assert_eq!(normalise_var_name_for_style(source, style), root);
            assert_eq!(split_array_name_for_style(source, style).0, root);
        }
    }
}

#[test]
fn literal_reference_names_keep_combined_lookup_data_separate_from_indexes() {
    for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        let config = tcl_lexer::LexerConfig::for_dialect(profile);
        for (source, name) in [
            ("${arr(key)}", "arr(key)"),
            ("${}", ""),
            ("${scalar(open}", "scalar(open"),
        ] {
            assert_eq!(
                whole_variable_reference_literal_name(source, config),
                Some(name),
                "{profile}: {source}"
            );
        }
        for source in ["$arr(key)", "$arr($key)", "$a:b", "$a$b", "${missing"] {
            assert_eq!(
                whole_variable_reference_literal_name(source, config),
                None,
                "{profile}: {source}"
            );
        }
        assert_eq!(
            whole_scalar_variable_reference_name("${arr(key)}", config),
            None
        );
    }
}

#[test]
fn whole_reference_shapes_follow_original_provider_grammar() {
    let first = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::default()
    };
    let nested = tcl_lexer::LexerConfig::default();
    let jim = tcl_lexer::LexerConfig::for_dialect("jim");
    for config in [first, nested, jim] {
        for (source, name) in [("$a", "a"), ("${}", ""), ("${scalar(open}", "scalar(open")] {
            assert!(is_whole_variable_reference(source, config));
            assert_eq!(
                whole_scalar_variable_reference_name(source, config),
                Some(name)
            );
        }
        for source in ["$arr(key)", "${arr(key)}"] {
            assert!(is_whole_variable_reference(source, config));
            assert_eq!(whole_scalar_variable_reference_name(source, config), None);
        }
        for source in ["$a:b", "$arr(key)tail", "$a$b", "plain", "${missing"] {
            assert!(!is_whole_variable_reference(source, config));
            assert_eq!(whole_scalar_variable_reference_name(source, config), None);
        }
    }
    assert_eq!(whole_scalar_variable_reference_name("$café", first), None);
    assert_eq!(
        whole_scalar_variable_reference_name("$café", jim),
        Some("café")
    );
    assert_eq!(
        whole_scalar_variable_reference_name("${a{b}c}", first),
        None
    );
    assert_eq!(
        whole_scalar_variable_reference_name("${a{b}c}", nested),
        Some("a{b}c")
    );
    assert_eq!(whole_scalar_variable_reference_name("$(1+2)", jim), None);
}

#[test]
fn selected_reference_roots_keep_whole_scalar_and_combined_element_bytes() {
    // Implementation contract: naming.expression.selected-reference-root
    // docs/design/analysis/name-resolution-proofs/expression-selected-reference-root.md
    for style in [
        tcl_dialect::BracedVarStyle::FirstClose,
        tcl_dialect::BracedVarStyle::Tcl9Nesting,
    ] {
        let config = tcl_lexer::LexerConfig {
            braced_var: style,
            ..tcl_lexer::LexerConfig::default()
        };
        for (source, root) in [
            (b"${scalar(open}".as_slice(), b"scalar(open".as_slice()),
            (b"${scalar(open)tail}", b"scalar(open)tail"),
            (b"${arr(key)}", b"arr"),
            (b"$arr(key)", b"arr"),
            (b"${arr(inner)(key)}", b"arr"),
            (b"${cash$name}", b"cash$name"),
            (b"${\xff(key)}", b"\xff"),
            (b"${\xff(open}", b"\xff(open"),
            (b"${nul\0tail(key)}", b"nul\0tail"),
            (b"${}", b""),
        ] {
            assert_eq!(
                variable_reference_root_bytes(source, config),
                Ok(Some(root))
            );
        }
        for source in [b"plain".as_slice(), b"$", b"${arr}(key)", b"$arr(key)tail"] {
            assert_eq!(variable_reference_root_bytes(source, config), Ok(None));
        }
        assert!(variable_reference_root_bytes(b"${missing", config).is_err());
    }
    let first = tcl_lexer::LexerConfig {
        braced_var: tcl_dialect::BracedVarStyle::FirstClose,
        ..tcl_lexer::LexerConfig::default()
    };
    let nested = tcl_lexer::LexerConfig::default();
    assert_eq!(
        variable_reference_root_bytes(b"${a{b}", first),
        Ok(Some(b"a{b".as_slice()))
    );
    assert!(variable_reference_root_bytes(b"${a{b}", nested).is_err());
    assert_eq!(variable_reference_root_bytes(b"${a{b}c}", first), Ok(None));
    assert_eq!(
        variable_reference_root_bytes(b"${a{b}c}", nested),
        Ok(Some(b"a{b}c".as_slice()))
    );
}

#[test]
fn retained_namespace_candidates_preserve_unaddressable_contexts() {
    use super::command_resolution_candidates_from_namespace_keys;
    assert_eq!(
        command_resolution_candidates_from_namespace_keys(":::", &["::n", ":::"], "p"),
        [":::::p", "::n::p", "::p"]
    );
    assert_eq!(
        command_resolution_candidates_from_namespace_keys("::n", &[":::"], "p"),
        ["::n::p", ":::::p", "::p"]
    );
    assert_eq!(
        command_resolution_candidates_from_namespace_keys(":::", &[":::"], ":::::p"),
        ["::p"]
    );
}

#[test]
fn jim_global_keys_preserve_native_written_name_distinctions() {
    use super::{jim_global_variable_key, jim_global_variable_key_bytes, qualify_bytes};

    // Pinned Jim 0.84 resolves :::x as x, while :x is a different key.
    // Its flat keys also distinguish foo:::bar from foo::bar.
    for (namespace, name, expected) in [
        ("::", ":::x", "x"),
        ("::n", "::::x", "x"),
        ("::", ":x", ":x"),
        ("::n", ":x", "n:::x"),
        ("::", "foo:::bar", "foo:::bar"),
        ("::n", "foo:::bar", "n::foo:::bar"),
        ("::n", "::foo:::bar", "foo:::bar"),
        ("::n", ":::", ""),
    ] {
        assert_eq!(jim_global_variable_key(namespace, name), expected);
        assert_eq!(
            jim_global_variable_key_bytes(namespace.as_bytes(), name.as_bytes()),
            expected.as_bytes()
        );
    }
    assert_eq!(qualify_bytes(b"::", b"foo:::bar"), b"::foo::bar");
}

#[test]
fn jim_global_keys_keep_constructed_namespace_segments_and_raw_bytes() {
    use super::jim_global_variable_key_bytes;

    // Strip one root marker from an already constructed namespace key.
    // These segments are not absolute written names to canonicalise.
    assert_eq!(jim_global_variable_key_bytes(b":::::n", b"x"), b":::n::x");
    assert_eq!(jim_global_variable_key_bytes(b"::", b"A\0B"), b"A\0B");
    assert_eq!(jim_global_variable_key_bytes(b"::n", b":::\xff"), b"\xff");
    assert_eq!(
        jim_global_variable_key_bytes(b"::\xff", b"A\0B"),
        b"\xff::A\0B"
    );
}

/// Every `${…}` reader in this module resolves the closer
/// through the one owner, so they agree with each other *and* move
/// together with the release.
///
/// Without that shared owner this module could answer `${a{b}c}` two
/// ways at once: `normalise_var_name` stripping the **last** `}`
/// (`a{b}c`, the 9.x answer) while `split_array_name` /
/// `element_var_name_braced` take the **first** (`a{b`, the 8.x answer)
/// — neither of them consulting the dialect. Oracles:
/// `set {a{b}c} HIT; subst {${a{b}c}}` is `HIT` on
/// 9.0.4 and `can't read "a{b"` on 8.6.16
/// (`Tcl_ParseVarName`, `tclParse.c:1315` vs `:1398`).
#[test]
fn braced_var_readers_agree_and_follow_the_release_rule() {
    use super::{
        element_var_name_braced_for_style, normalise_var_name_for_style,
        split_array_name_for_style, split_braced_var_ref, var_reference_for_style,
    };
    use tcl_dialect::BracedVarStyle::{FirstClose, Tcl9Nesting};

    // 9.x: nesting and `\X` pairs are inside the name.
    for (word, name) in [
        ("${a{b}c}", "a{b}c"),
        (r"${a\}b}", r"a\}b"),
        ("${a{b}}", "a{b}"),
        ("${plain}", "plain"),
    ] {
        assert_eq!(split_braced_var_ref(word, Tcl9Nesting), Some((name, "")));
        assert_eq!(normalise_var_name_for_style(word, Tcl9Nesting), name);
        assert_eq!(var_reference_for_style(word, Tcl9Nesting), name);
        assert_eq!(
            element_var_name_braced_for_style(word, false, Tcl9Nesting),
            name
        );
        assert_eq!(split_array_name_for_style(word, Tcl9Nesting), (name, None));
    }

    // 8.x: the name stops at the first literal `}`; the rest is word text.
    for (word, name, rest) in [
        ("${a{b}c}", "a{b", "c}"),
        (r"${a\}b}", r"a\", "b}"),
        ("${a{b}}", "a{b", "}"),
    ] {
        assert_eq!(split_braced_var_ref(word, FirstClose), Some((name, rest)));
        assert_eq!(normalise_var_name_for_style(word, FirstClose), name);
        assert_eq!(
            element_var_name_braced_for_style(word, false, FirstClose),
            name
        );
        assert_eq!(split_array_name_for_style(word, FirstClose), (name, None));
    }

    // The nesting rule *widens* what is unterminated: `${a{b}` closes at
    // 8.x but runs off the end at 9.x, and an unterminated reference is an
    // error C names — never a name running to end-of-input.
    assert_eq!(
        split_braced_var_ref("${a{b}", FirstClose),
        Some(("a{b", ""))
    );
    assert_eq!(split_braced_var_ref("${a{b}", Tcl9Nesting), None);
    assert_eq!(split_braced_var_ref(r"${a\}", Tcl9Nesting), None);
    assert_eq!(split_braced_var_ref("${a", Tcl9Nesting), None);
    assert_eq!(split_braced_var_ref("$a", Tcl9Nesting), None);

    // A whole `${…}` word the caller's rule cannot close was delimited by
    // a lexer running the *other* release's rule, and that span is correct
    // by construction: the readers trust it rather than naming `{a{b}`,
    // which the source never spells. `split_braced_var_ref` itself stays
    // owner-pure — the span reading belongs to the whole-word readers only.
    assert_eq!(normalise_var_name_for_style("${a{b}", Tcl9Nesting), "a{b");
    assert_eq!(var_reference_for_style("${a{b}", Tcl9Nesting), "a{b");
    assert_eq!(
        element_var_name_braced_for_style("${a{b}", false, Tcl9Nesting),
        "a{b"
    );
    assert_eq!(
        split_array_name_for_style("${a{b}", Tcl9Nesting),
        ("a{b", None)
    );
    // With no closing `}` at all there is no span to trust either, and the
    // reader falls back to its non-braced reading.
    assert_eq!(normalise_var_name_for_style("${a{b", Tcl9Nesting), "{a{b");

    // An `(idx)` inside the braces is an element; one after the closer is
    // not — and that holds under both rules.
    for style in [Tcl9Nesting, FirstClose] {
        assert_eq!(
            split_array_name_for_style("${arr(foo)}", style),
            ("arr", Some("foo"))
        );
        assert_eq!(
            split_array_name_for_style("${arr}(foo)", style),
            ("arr", None)
        );
        assert_eq!(normalise_var_name_for_style("${arr}(foo)", style), "arr");
    }
}

#[test]
fn written_command_tail_follows_the_trailing_separator_rule() {
    use super::written_command_tail as tail;
    // Trailing separator run → the empty-string `{}` command
    // (tclsh 8.6/9.0-pinned: `proc x:: {} {}` defines `::x::`).
    assert_eq!(tail(b"x::"), b"");
    assert_eq!(tail(b"::x::"), b"");
    assert_eq!(tail(b"a::b:::"), b"");
    assert_eq!(tail(b"::"), b"");
    assert_eq!(tail(b":::"), b"");
    assert_eq!(tail(b""), b"");
    // Ordinary spellings keep the last colon-run segment.
    assert_eq!(tail(b"cmd"), b"cmd");
    assert_eq!(tail(b"::a::b"), b"b");
    assert_eq!(tail(b"a:::b"), b"b");
    // A lone colon is an ordinary name character.
    assert_eq!(tail(b":"), b":");
    assert_eq!(tail(b"a:b"), b"a:b");
}

/// `qualify` joins by the canonical rule. tclsh 8.6 and 9.0 pinned the
/// lone-colon and trailing-separator rows; the rule is invariant from C
/// Tcl 8.4 to 9.1 (`TclGetNamespaceForQualName`).
#[test]
fn qualify_joins_by_the_canonical_rule() {
    for (prefix, name, key) in [
        // A relative name resolves under a rooted or unrooted prefix.
        ("::ns", "C", "::ns::C"),
        ("ns", "C", "::ns::C"),
        ("::a::b", "c::D", "::a::b::c::D"),
        ("", "C", "::C"),
        ("::", "C", "::C"),
        // An absolute name is never re-prefixed.
        ("::ns", "::other::C", "::other::C"),
        ("ns", "::C", "::C"),
        // `proc {} {} {}`: the empty name is the entity a trailing
        // separator denotes.
        ("", "", "::"),
        ("::ns", "", "::ns::"),
        // `proc :` at the root is `"::" + ":"`, never the empty-name key,
        // and likewise inside a namespace named `:` (key `":::"`).
        ("::", ":", ":::"),
        (":::", ":", "::::::"),
        // Written runs in the name collapse (`a:::b` names `a::b`); a
        // written trailing run names the `{}` command there.
        ("::ns", "a:::b", "::ns::a::b"),
        ("::", "x::", "::x::"),
        ("::", "x:::", "::x::"),
    ] {
        assert_eq!(qualify(prefix, name), key, "{prefix:?} {name:?}");
    }
}

#[test]
fn canonical_written_command_forms() {
    assert_eq!(canonical_written_command("::a:::b"), "::a::b");
    assert_eq!(canonical_written_command(":::"), "::");
    assert_eq!(canonical_written_command("::"), "::");
    assert_eq!(canonical_written_command("::x::"), "::x::");
    assert_eq!(canonical_written_command("x:::"), "x::");
    assert_eq!(canonical_written_command(":"), ":");
    assert_eq!(canonical_written_command(":x"), ":x");
    assert_eq!(canonical_written_command("a:b"), "a:b");
    assert_eq!(canonical_written_command(""), "");
}

#[test]
fn key_tail_inverts_the_key_construction() {
    assert_eq!(key_tail("::a::b"), "b");
    assert_eq!(key_tail("::a"), "a");
    assert_eq!(key_tail(":::"), ":");
    assert_eq!(key_tail(":::::p"), "p");
    assert_eq!(key_tail("::a:::::p"), "p");
    assert_eq!(key_tail("::a::::::::p"), "p");
    assert_eq!(key_tail("::::p"), "::::p");
    assert_eq!(key_tail("::::::"), ":");
    assert_eq!(key_tail("::a:::"), ":");
    assert_eq!(key_tail("::a::::x"), ":x");
    assert_eq!(key_tail("::x::"), "");
    assert_eq!(key_tail("::"), "");
    assert_eq!(key_tail(":"), ":");
    assert_eq!(key_tail("a:b"), "a:b");
    assert_eq!(key_tail("cmd"), "cmd");
    assert_eq!(key_tail("a::b"), "b");
    assert_eq!(key_tail(""), "");
}

#[test]
fn constructed_key_split_preserves_nested_literal_colon_namespaces() {
    assert_eq!(key_holder_and_tail(":::::p"), (":::", "p"));
    assert_eq!(key_holder_and_tail("::a:::::p"), ("::a:::", "p"));
    assert_eq!(key_holder_and_tail("::a:::::"), ("::a:::", ""));
    assert_eq!(key_holder_and_tail("::a::::::::p"), ("::a::::::", "p"));
    assert_eq!(key_segments("::a:::::p"), ["a", ":", "p"]);
}

#[test]
fn colon_names_resolve_by_construction_symmetry() {
    // A bare `:` call from the global scope must produce the same key
    // `proc :` is registered under.
    assert_eq!(bareword_resolution_candidates("::", ":"), vec![":::"]);
    // The written `:::` is the *global `{}` command* (`proc {} {} {}`),
    // never the `:` proc — tclsh-pinned: with `proc {}` defined, `::` and
    // `:::` both dispatch to it; without it they are invalid.
    assert_eq!(bareword_resolution_candidates("::", ":::"), vec!["::"]);
    // From inside the namespace named `:`, a bare `:` reaches the nested
    // proc first, then the global `:`.
    assert_eq!(
        bareword_resolution_candidates(":::", ":"),
        vec!["::::::", ":::"],
    );
    // `x:::` names the `{}` command in `::x` (tclsh-pinned).
    assert_eq!(bareword_resolution_candidates("::", "x:::"), vec!["::x::"]);
}

#[test]
fn is_absolutely_addressable_cases() {
    assert!(is_absolutely_addressable(&[], "cmd"));
    assert!(is_absolutely_addressable(&["a", "b"], "cmd"));
    assert!(is_absolutely_addressable(&[], "a:b"));
    // The empty name IS addressable (`::` / `::x::`).
    assert!(is_absolutely_addressable(&["x"], ""));
    // A lone-colon simple name or namespace segment is not.
    assert!(!is_absolutely_addressable(&[], ":"));
    assert!(!is_absolutely_addressable(&[":"], "cmd"));
    assert!(!is_absolutely_addressable(&["a", ":"], "cmd"));
}

#[test]
fn is_absolute_reads_a_leading_colon_run_of_two_or_more() {
    for (name, absolute) in [
        ("::a", true),
        (":::a", true),
        ("::a::b", true),
        ("::", true),
        (":::", true),
        ("::a::", true),
        (":a", false),
        (":", false),
        ("a::b", false),
        ("a:::b", false),
        ("a::", false),
        ("a", false),
        ("", false),
    ] {
        assert_eq!(is_absolute(name), absolute, "{name:?}");
        assert_eq!(is_absolute_bytes(name.as_bytes()), absolute, "{name:?}");
        assert_eq!(
            written_command_parts(name.as_bytes()).absolute,
            absolute,
            "{name:?}"
        );
    }
    // Counted bytes: NUL and non-UTF-8 bytes are name data, not a boundary.
    assert!(is_absolute_bytes(b"::\0a"));
    assert!(is_absolute_bytes(b"::\xff"));
    assert!(!is_absolute_bytes(b"\0::a"));
    assert!(!is_absolute_bytes(b"\xff::a"));
}

#[test]
fn unrooted_member_key_roots_to_the_rooted_member_key() {
    for (namespace, member) in [
        ("", "v"),
        ("", ""),
        ("", ":"),
        ("a", "b"),
        ("a::b", "c"),
        ("a", ""),
        ("a", ":b"),
        (":", ":"),
        ("a:", "b"),
    ] {
        assert_eq!(
            root_unrooted_key(&unrooted_member_key(namespace, member)),
            rooted_member_key(&root_unrooted_key(namespace), member),
            "{namespace:?} {member:?}"
        );
    }
    assert_eq!(unrooted_member_key("", "v"), "v");
    assert_eq!(unrooted_member_key("oo::Obj1", "m"), "oo::Obj1::m");
}
use super::*;

/// The variable a written reference names: the `$`, a `${…}` wrapper and
/// an array index are spelling, not name. Release-invariant.
#[test]
fn normalise_var_name_strips_the_sigil_braces_and_index() {
    for (written, name) in [
        ("$foo", "foo"),
        ("${bar}", "bar"),
        ("$arr(idx)", "arr"),
        ("${arr(idx)}", "arr"),
        ("plain", "plain"),
        ("$ns::var", "ns::var"),
        ("", ""),
        ("$", ""),
    ] {
        assert_eq!(normalise_var_name(written), name, "{written:?}");
    }
}

#[test]
fn brace_substitutable_cases_tcl9_nesting() {
    // Reachable names under the Tcl 9 rule (nested braces + `\X` pairs).
    assert!(is_brace_substitutable("", true));
    assert!(is_brace_substitutable("foo", true));
    assert!(is_brace_substitutable("a(b)", true)); // `)` is fine in brace form
    assert!(is_brace_substitutable("a{b}c", true)); // balanced inner braces
    assert!(is_brace_substitutable(r"a\}b", true)); // `\}` consumes 2, kept
    // Unreachable names.
    assert!(!is_brace_substitutable("a}b", true)); // `}` at depth 0 ends span early
    assert!(!is_brace_substitutable(r"trail\", true)); // trailing lone backslash
    assert!(!is_brace_substitutable("a{b", true)); // unbalanced `{`
}

#[test]
fn brace_substitutable_cases_tcl8_first_close() {
    // The 8.x family closes at the FIRST literal `}` (8.6.14
    // `tclParse.c:1466`, tclsh-verified: `${a{b}}` reads variable `a{b`).
    assert!(is_brace_substitutable("", false));
    assert!(is_brace_substitutable("foo", false));
    assert!(is_brace_substitutable("a(b)", false));
    // `{` and `\` are ordinary name characters — no pairing, no escapes.
    assert!(is_brace_substitutable("a{b", false));
    assert!(is_brace_substitutable(r"trail\", false));
    // ANY `}` in the name is unreachable — nesting/escapes don't help.
    assert!(!is_brace_substitutable("a{b}c", false));
    assert!(!is_brace_substitutable(r"a\}b", false));
    assert!(!is_brace_substitutable("a}b", false));
}

#[test]
fn split_array_name_forms() {
    assert_eq!(split_array_name("arr"), ("arr", None));
    assert_eq!(split_array_name("arr(foo)"), ("arr", Some("foo")));
    assert_eq!(split_array_name("$arr(foo)"), ("arr", Some("foo")));
    assert_eq!(split_array_name("${arr(foo)}"), ("arr", Some("foo")));
    // `${arr}(foo)` is scalar `arr` then literal `(foo)` — not an element.
    assert_eq!(split_array_name("${arr}(foo)"), ("arr", None));
    assert_eq!(split_array_name("${arr}"), ("arr", None));
    // dynamic index text is preserved verbatim for later classification.
    assert_eq!(split_array_name("a($i)"), ("a", Some("$i")));
    // a `)` with no `(` is not an element.
    assert_eq!(split_array_name("weird)"), ("weird)", None));
}

/// The full brace-literal spelling matrix, pinned against
/// tclsh 9.0.4 and 8.6.14 (byte-identical transcripts):
///
/// ```text
/// set {$n} v ; set {$n}          -> v
/// info exists {$n} / n           -> 1 / 0      (distinct variables)
/// set n other ; set {$n}         -> v          (unaffected)
/// set ${$n}                      -> can't read "v"   (`${$n}` read `$n`)
/// set {a b} v2 ; set x ${a b}    -> v2         (space in a legal name)
/// set i 5 ; set {arr($i)} 1
///   array names arr              -> {$i}       (literal key)
///   info exists arr(5)           -> 0
///   set arr($i) 2 ; array names arr -> {$i} 5  (two distinct elements)
/// unset {$n} ; info exists {$n} / n -> 0 / 1
/// ```
#[test]
fn brace_literal_names_keep_their_sigil() {
    // Element-qualified (SSA / def-use) naming.
    assert_eq!(element_var_name_braced("$n", true), "$n");
    assert_eq!(element_var_name_braced("${n}", true), "${n}");
    assert_eq!(element_var_name_braced("a b", true), "a b");
    assert_eq!(element_var_name_braced("arr($i)", true), "arr($i)");
    assert_eq!(element_var_name_braced("$arr(k)", true), "$arr(k)");
    assert_eq!(element_var_name_braced("[gen]", true), "[gen]");
    // Base-name (scope / analyser) naming: the element suffix still comes
    // off, the sigil does not.
    assert_eq!(normalise_var_name_braced("$n", true), "$n");
    assert_eq!(normalise_var_name_braced("${n}", true), "${n}");
    assert_eq!(normalise_var_name_braced("a b", true), "a b");
    assert_eq!(normalise_var_name_braced("arr($i)", true), "arr");
    assert_eq!(normalise_var_name_braced("$arr(k)", true), "$arr");
    assert_eq!(
        split_array_name_braced("$arr(k)", true),
        ("$arr", Some("k"))
    );
    assert_eq!(split_array_name_braced("$n", true), ("$n", None));
}

/// TN control for [`brace_literal_names_keep_their_sigil`]: the *unbraced*
/// spellings are genuine substitutions and must keep normalising, or the
/// fix would blind every ordinary `$x` read.
#[test]
fn unbraced_names_still_normalise() {
    assert_eq!(element_var_name_braced("$n", false), "n");
    assert_eq!(element_var_name_braced("${n}", false), "n");
    assert_eq!(element_var_name_braced("$arr($i)", false), "arr");
    assert_eq!(element_var_name_braced("$arr(k)", false), "arr(k)");
    assert_eq!(normalise_var_name_braced("$n", false), "n");
    assert_eq!(normalise_var_name_braced("$arr(k)", false), "arr");
    assert_eq!(
        split_array_name_braced("$arr(k)", false),
        ("arr", Some("k"))
    );
    // The `${…}` *reference* form already carried the rule: its content is
    // the literal name whichever flag the caller passes.
    assert_eq!(element_var_name("${$n}"), "$n");
    assert_eq!(element_var_name("${arr($i)}"), "arr($i)");
    assert_eq!(normalise_var_name("${$n}"), "$n");
}

/// The rename gate's predicate: a name that can only be
/// written quoted is not renameable by span substitution.
#[test]
fn quoting_requirement_matches_the_writable_spellings() {
    for needs in [
        "$n", "${n}", "a b", "[gen]", "a\tb", "x;y", "q\"r", "", "a\\b",
    ] {
        assert!(
            var_name_requires_quoting(needs),
            "{needs:?} can only be written quoted"
        );
    }
    for plain in ["n", "::ns::v", "arr", "arr(k)", "_x1", "caf\u{e9}"] {
        assert!(
            !var_name_requires_quoting(plain),
            "{plain:?} is writable bare"
        );
    }
}

// normalise_qualified_name tests

/// A reporting key is rooted once and its written colon runs collapse;
/// the empty name stays empty. Release-invariant.
#[test]
fn normalise_qualified_name_roots_once_and_collapses_colon_runs() {
    for (written, key) in [
        ("foo", "::foo"),
        ("::bar", "::bar"),
        ("ns::sub", "::ns::sub"),
        ("", ""),
        ("::", "::"),
        ("::::x", "::x"),
    ] {
        assert_eq!(normalise_qualified_name(written), key, "{written:?}");
    }
}

// qualifier_segments / is_qualified

#[test]
fn qualifier_segments_cases() {
    assert_eq!(
        qualifier_segments(b"::a::b::cmd"),
        vec![&b"a"[..], b"b", b"cmd"]
    );
    assert_eq!(qualifier_segments(b"::cmd"), vec![&b"cmd"[..]]);
    assert_eq!(qualifier_segments(b"cmd"), vec![&b"cmd"[..]]);
    assert_eq!(qualifier_segments(b"a::b"), vec![&b"a"[..], b"b"]);
    assert_eq!(qualifier_segments(b"::"), [] as [&[u8]; 0]);
    // a trailing separator drops the empty tail; a lone interior colon stays.
    assert_eq!(qualifier_segments(b"a::b::"), vec![&b"a"[..], b"b"]);
    // a run of >=2 colons is one separator (all consecutive colons consumed).
    assert_eq!(qualifier_segments(b"a:::b"), vec![&b"a"[..], b"b"]);
    assert_eq!(qualifier_segments(b"a::::b"), vec![&b"a"[..], b"b"]);
    assert_eq!(
        qualifier_segments(b":::test_ns_1:::::test_ns_2:::"),
        vec![&b"test_ns_1"[..], b"test_ns_2"]
    );
    // a lone interior colon is an ordinary name character.
    assert_eq!(qualifier_segments(b"a:b"), vec![&b"a:b"[..]]);
    assert!(ends_with_separator(b"a::b::"));
    assert!(!ends_with_separator(b"a::b"));
}

#[test]
fn textmate_variable_body_keeps_colon_runs_together() {
    assert!(textmate_variable_name_body().contains("[:]{2,}"));
    assert_eq!(qualifier_segments(b"foo:::bar"), vec![&b"foo"[..], b"bar"]);
    assert_eq!(
        qualifier_segments(b"::foo::::bar"),
        vec![&b"foo"[..], b"bar"]
    );
    assert_eq!(qualifier_segments(b"foo:bar"), vec![&b"foo:bar"[..]]);
}

#[test]
fn is_qualified_cases() {
    assert!(is_qualified(b"::a"));
    assert!(is_qualified(b"a::b"));
    assert!(is_qualified(b"::"));
    assert!(!is_qualified(b"plain"));
    assert!(!is_qualified(b"a:b"));
    assert!(!is_qualified(b""));
}

#[test]
fn written_component_extents_share_segmentation_and_preserve_original_byte_geometry() {
    // naming.editor.logical-written-component-geometry
    // docs/design/analysis/name-resolution-proofs/logical-written-component-geometry.md
    // Written segmentation only; no native input/namespace or edit admission.
    for (written, wanted) in [
        (&b"::a::b"[..], vec![2..3, 5..6]),
        (&b":::a:::b"[..], vec![3..4, 7..8]),
        (&b"::::a::::b"[..], vec![4..5, 9..10]),
        (&b":::::a:::::b:::::"[..], vec![5..6, 11..12]),
        (&b"a:b"[..], std::iter::once(0..3).collect()),
        (&b"::"[..], vec![]),
        (&b":::::"[..], vec![]),
        (&b""[..], vec![]),
        (&b"a\0x:::b"[..], vec![0..3, 6..7]),
        ("::::é:::旧::".as_bytes(), vec![4..6, 9..12]),
    ] {
        let extents = qualifier_component_extents(written).collect::<Vec<_>>();
        assert_eq!(extents, wanted, "{written:?}");
        assert_eq!(
            qualifier_segments(written),
            extents
                .iter()
                .map(|extent| &written[extent.clone()])
                .collect::<Vec<_>>()
        );
    }
}
