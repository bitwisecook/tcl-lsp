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

//! Constant folds against tclsh: pure builtins, namespace string operations, `format`, `binary format` and the type a `string is` test proves.

use super::*;

/// The broad list / string / dict / subst fold matrix. The `string is` number
/// classes (integer / wideinteger / entier / double / dict) fold via the
/// version-aware folder and are pinned here under the `tcl9.0` dialect.
const FOLDABLE: &[Case] = &[
    // list ops
    ("list", None, &["a", "b", "c"]),
    ("list", None, &[]),
    ("list", None, &["a b", "c"]),
    ("llength", None, &["a b c"]),
    ("llength", None, &[""]),
    ("llength", None, &["a {b c} d"]),
    ("lindex", None, &["a b c", "1"]),
    ("lindex", None, &["a b c", "end"]),
    ("lindex", None, &["a b c", "end-1"]),
    ("lrange", None, &["a b c d", "1", "2"]),
    ("lrange", None, &["a b c d", "0", "end"]),
    ("lreverse", None, &["a b c"]),
    ("lreverse", None, &["a b", "c"]),
    ("lrepeat", None, &["3", "x"]),
    ("concat", None, &["a", "b", "c"]),
    ("concat", None, &["a b", "c d"]),
    // Backslash-bearing concat inputs must decline rather than trim
    // incorrectly; this case fails if the registry fold starts returning
    // the old approximate value again.
    ("concat", None, &["a", "b\\ "]),
    ("join", None, &["a b c", "-"]),
    ("join", None, &["a b c"]),
    ("join", None, &["a b c", ":"]),
    ("split", None, &["a,b,c", ","]),
    ("split", None, &["abc", ""]),
    // string ops
    ("string", Some("length"), &["hello"]),
    ("string", Some("index"), &["hello", "1"]),
    ("string", Some("index"), &["hello", "end"]),
    ("string", Some("range"), &["hello", "1", "3"]),
    ("string", Some("reverse"), &["hello"]),
    ("string", Some("toupper"), &["abc"]),
    ("string", Some("tolower"), &["ABC"]),
    ("string", Some("totitle"), &["abc"]),
    ("string", Some("cat"), &["a", "b", "c"]),
    ("string", Some("repeat"), &["ab", "3"]),
    ("string", Some("trim"), &["  x  "]),
    ("string", Some("map"), &["a X b Y", "abab"]),
    ("string", Some("first"), &["l", "hello"]),
    ("string", Some("first"), &["", "hello"]),
    ("string", Some("first"), &["", "hello", "1"]),
    ("string", Some("last"), &["l", "hello"]),
    ("string", Some("last"), &["", "hello"]),
    ("string", Some("last"), &["", "hello", "1"]),
    ("string", Some("equal"), &["abc", "abc"]),
    ("string", Some("equal"), &["abc", "abd"]),
    ("string", Some("compare"), &["a", "b"]),
    // string is — char / boolean / list classes + the version-aware number
    // classes (integer / wideinteger / entier / double / dict, here under 9.0)
    ("string", Some("is"), &["alpha", "abc"]),
    ("string", Some("is"), &["digit", "123"]),
    ("string", Some("is"), &["space", "   "]),
    ("string", Some("is"), &["integer", "42"]),
    ("string", Some("is"), &["integer", "-7"]),
    ("string", Some("is"), &["integer", "4294967295"]),
    ("string", Some("is"), &["integer", "abc"]),
    ("string", Some("is"), &["integer", "1.5"]),
    ("string", Some("is"), &["integer", "9999999999"]), // 9.0 unbounded → 1
    ("string", Some("is"), &["wideinteger", "42"]),
    (
        "string",
        Some("is"),
        &["wideinteger", "9223372036854775808"],
    ), // > 2^63-1 → 0 on 9.0
    (
        "string",
        Some("is"),
        &["entier", "99999999999999999999999999"],
    ),
    ("string", Some("is"), &["dict", "a 1 b 2"]),
    ("string", Some("is"), &["dict", "a 1 b"]),
    ("string", Some("is"), &["double", "3.14"]),
    ("string", Some("is"), &["double", "5.e3"]),
    ("string", Some("is"), &["double", "1E-10"]),
    ("string", Some("is"), &["double", "inf"]),
    ("string", Some("is"), &["double", "nan"]),
    ("string", Some("is"), &["double", "1e"]),
    ("string", Some("is"), &["double", "abc"]),
    ("string", Some("is"), &["double", "-2.5"]),
    // dict ops
    ("dict", Some("get"), &["a 1 b 2", "b"]),
    ("dict", Some("size"), &["a 1 b 2"]),
    ("dict", Some("keys"), &["a 1 b 2"]),
    ("dict", Some("values"), &["a 1 b 2"]),
    ("dict", Some("exists"), &["a 1 b 2", "a"]),
    // subst (literal single-arg form only)
    ("subst", None, &["hello world"]),
    ("subst", None, &["plain"]),
    // scan — inline form, integer / hex / octal / char / string conversions
    ("scan", None, &["42", "%d"]),
    ("scan", None, &["-5", "%d"]),
    ("scan", None, &["ff", "%x"]),
    ("scan", None, &["-ff", "%x"]),
    ("scan", None, &["7fffffff", "%x"]),
    ("scan", None, &["17", "%o"]),
    ("scan", None, &["A", "%c"]),
    ("scan", None, &["abc", "%s"]),
    ("scan", None, &["1 2 3", "%d %d %d"]),
    ("scan", None, &["x42", "x%d"]),
    ("scan", None, &["a(b)", "%s"]),
];

/// `namespace qualifiers` / `namespace tail` — pure string
/// splitting at the last `::`, kept as its own matrix because it is also
/// replayed against 8.6 by [`namespace_string_op_folds_match_tcl86`] to pin
/// the version-invariant (`const_fold`, not `const_fold_versioned`)
/// registration.
const NAMESPACE_STRING_OPS: &[Case] = &[
    ("namespace", Some("qualifiers"), &["::a::b::c"]),
    ("namespace", Some("qualifiers"), &["a::b"]),
    ("namespace", Some("qualifiers"), &["c"]),
    ("namespace", Some("qualifiers"), &[""]),
    ("namespace", Some("qualifiers"), &["::"]),
    ("namespace", Some("qualifiers"), &[":::"]),
    ("namespace", Some("qualifiers"), &["a:::b"]),
    ("namespace", Some("qualifiers"), &["::a::b::"]),
    ("namespace", Some("qualifiers"), &["::x:y"]),
    ("namespace", Some("qualifiers"), &["::foo"]),
    ("namespace", Some("qualifiers"), &["foo::"]),
    ("namespace", Some("qualifiers"), &["::a::b::c::"]),
    ("namespace", Some("qualifiers"), &["a"]),
    ("namespace", Some("qualifiers"), &[":"]),
    ("namespace", Some("qualifiers"), &["::::"]),
    ("namespace", Some("qualifiers"), &["x::y::z"]),
    ("namespace", Some("qualifiers"), &["::ticklecharts::Gauge"]),
    ("namespace", Some("qualifiers"), &["a::b c::d"]),
    ("namespace", Some("qualifiers"), &["a::"]),
    ("namespace", Some("qualifiers"), &["::a"]),
    // A name for a namespace that does not exist splits identically: the
    // subcommands are string operations, never namespace lookups.
    ("namespace", Some("qualifiers"), &["::never::here::x"]),
    ("namespace", Some("tail"), &["::a::b::c"]),
    ("namespace", Some("tail"), &["a::b"]),
    ("namespace", Some("tail"), &["c"]),
    ("namespace", Some("tail"), &[""]),
    ("namespace", Some("tail"), &["::"]),
    ("namespace", Some("tail"), &[":::"]),
    ("namespace", Some("tail"), &["a:::b"]),
    ("namespace", Some("tail"), &["::a::b::"]),
    ("namespace", Some("tail"), &["::x:y"]),
    ("namespace", Some("tail"), &["::foo"]),
    ("namespace", Some("tail"), &["foo::"]),
    ("namespace", Some("tail"), &["::a::b::c::"]),
    ("namespace", Some("tail"), &["a"]),
    ("namespace", Some("tail"), &[":"]),
    ("namespace", Some("tail"), &["::::"]),
    ("namespace", Some("tail"), &["x::y::z"]),
    ("namespace", Some("tail"), &["::ticklecharts::Gauge"]),
    ("namespace", Some("tail"), &["a::b c::d"]),
    ("namespace", Some("tail"), &["a::"]),
    ("namespace", Some("tail"), &["::a"]),
    ("namespace", Some("tail"), &["::never::here::x"]),
];

#[test]
fn registry_folds_match_tcl9() {
    let Some(tclsh) = find_tclsh("9.0") else {
        return;
    };
    let reg = CommandRegistry::build_default();
    let folded = check_matrix(&tclsh, &reg, FOLDABLE);
    assert!(
        folded > 0,
        "no FOLDABLE case folded — the differential harness is not wired to the registry"
    );
    let ns_folded = check_matrix(&tclsh, &reg, NAMESPACE_STRING_OPS);
    assert_eq!(
        ns_folded,
        NAMESPACE_STRING_OPS.len(),
        "every `namespace qualifiers`/`tail` row must fold (issue #1096)"
    );
}

/// The same matrix replayed against `tclsh8.6`.  `namespace qualifiers` /
/// `tail` are registered as *version-invariant* folds, which is only sound if
/// 8.6 and 9.0 agree on every row — this is the test that pins it, so a future
/// divergence turns into a failure here rather than a wrong fold under an 8.6
/// dialect.
#[test]
fn namespace_string_op_folds_match_tcl86() {
    let Some(tclsh) = find_tclsh("8.6") else {
        return;
    };
    let reg = CommandRegistry::build_default();
    let folded = check_matrix(&tclsh, &reg, NAMESPACE_STRING_OPS);
    assert_eq!(
        folded,
        NAMESPACE_STRING_OPS.len(),
        "every `namespace qualifiers`/`tail` row must fold identically on 8.6"
    );
}

/// The `format` matrix. The fold now covers the whole
/// conversion set — every integer verb (`%d`/`%i`/`%x`/`%X`/`%o`/`%b`/`%u`,
/// version-aware: leading-zero octal/decimal, 8.x↔9.0 width/wrap), `%c`, the
/// float family (`%f`/`%e`/`%g`), and `%s` — with the full flag/width/precision
/// grid.  Pinned here under `tcl9.0`; the genuinely version-divergent forms a
/// single fold can't be sound for (`%#o`, `%#X`) simply bail (a miss).
const FORMATS: &[Case] = &[
    ("format", None, &["%d", "42"]),
    ("format", None, &["%d", "010"]), // 9.0: leading-zero decimal -> 10
    ("format", None, &["%d", "08"]),  // 9.0: decimal 8 (octal-invalid on 8.x)
    ("format", None, &["%d", "2147483648"]), // 9.0: 32-bit wrap -> -2147483648
    ("format", None, &["%d", "5000000000"]), // 9.0: wrap -> 705032704
    ("format", None, &["%d", "-2147483649"]),
    ("format", None, &["%d", "-7"]),
    ("format", None, &["%5d", "42"]),
    ("format", None, &["%-5d", "42"]),
    ("format", None, &["%05d", "7"]),
    ("format", None, &["%05d", "-7"]),
    ("format", None, &["%+d", "42"]),
    ("format", None, &["%+05d", "42"]),
    ("format", None, &["% d", "42"]),
    ("format", None, &["% d", "-7"]),
    ("format", None, &["%i", "42"]),
    ("format", None, &["%.5d", "42"]),
    ("format", None, &["%.3d", "-4"]),
    ("format", None, &["%5.3d", "42"]),
    ("format", None, &["%05.3d", "42"]),
    ("format", None, &["%.0d", "0"]), // 8.4: no digit; 8.5 on: `0`
    ("format", None, &["%.0x", "0"]),
    ("format", None, &["%.0o", "0"]),
    ("format", None, &["%.0u", "0"]),
    ("format", None, &["%5.0d", "0"]),
    ("format", None, &["%+.0d", "0"]),
    ("format", None, &["%-3.0d|", "0"]),
    ("format", None, &["%d", "2147483647"]),
    ("format", None, &["%d", "-2147483648"]),
    ("format", None, &["%x", "255"]),
    ("format", None, &["%x", "-1"]), // 9.0: 32-bit two's complement
    ("format", None, &["%x", "-255"]),
    ("format", None, &["%x", "010"]), // 9.0: decimal 10 -> hex a
    ("format", None, &["%x", "5000000000"]), // 9.0: wrap
    ("format", None, &["%o", "-1"]),
    ("format", None, &["%x", "0"]),
    ("format", None, &["%#x", "255"]),
    ("format", None, &["%#08x", "255"]),
    ("format", None, &["%X", "255"]),
    ("format", None, &["%o", "8"]),
    ("format", None, &["%08x", "255"]),
    ("format", None, &["%-8x", "255"]),
    ("format", None, &["%5x", "255"]),
    ("format", None, &["%.4x", "255"]),
    ("format", None, &["%s", "hi"]),
    ("format", None, &["%10s", "hi"]),
    ("format", None, &["%-10s", "hi"]),
    ("format", None, &["%.3s", "hello"]),
    ("format", None, &["%5.3s", "hello"]),
    ("format", None, &["%.0s", "hi"]),
    ("format", None, &["%c", "65"]),
    ("format", None, &["%5c", "65"]),
    ("format", None, &["%-5c", "65"]),
    // fixed-point float
    ("format", None, &["%f", "3.14"]),
    ("format", None, &["%.2f", "3.14159"]),
    ("format", None, &["%.0f", "2.5"]),
    ("format", None, &["%.0f", "3.5"]),
    ("format", None, &["%.2f", "0.125"]),
    ("format", None, &["%f", "42"]),
    ("format", None, &["%8.2f", "3.14159"]),
    ("format", None, &["%-8.2f", "3.14159"]),
    ("format", None, &["%08.3f", "2.5"]),
    ("format", None, &["%+.2f", "3.14"]),
    ("format", None, &["%.2f", "-2.675"]),
    ("format", None, &["%.10f", "0.1"]),
    // scientific / general float
    ("format", None, &["%e", "31400.0"]),
    ("format", None, &["%.3e", "31400.0"]),
    ("format", None, &["%E", "31400.0"]),
    ("format", None, &["%e", "0.0314"]),
    ("format", None, &["%.2e", "9.999"]),
    ("format", None, &["%e", "1e100"]),
    ("format", None, &["%015.3e", "31400.0"]),
    ("format", None, &["%g", "3.14"]),
    ("format", None, &["%g", "1000000.0"]),
    ("format", None, &["%g", "100000.0"]),
    ("format", None, &["%g", "0.00001"]),
    ("format", None, &["%g", "1.0"]),
    ("format", None, &["%G", "1234567.0"]),
    ("format", None, &["%.3g", "0.000123456"]),
    ("format", None, &["%u", "42"]),
    ("format", None, &["%u", "-1"]),         // 9.0: u32 -> 4294967295
    ("format", None, &["%u", "4294967296"]), // 9.0: wrap -> 0
    ("format", None, &["%u", "010"]),        // 9.0: decimal 10
    ("format", None, &["%05u", "42"]),
    ("format", None, &["%u", "2147483648"]),
    ("format", None, &["%b", "5"]),
    ("format", None, &["%b", "255"]),
    ("format", None, &["%08b", "5"]),
    ("format", None, &["%#b", "5"]),
    ("format", None, &["%c", "65"]),
    ("format", None, &["%5c", "65"]),
    ("format", None, &["%%"]),
];

#[test]
fn format_folds_match_tcl9() {
    let Some(tclsh) = find_tclsh("9.0") else {
        return;
    };
    let reg = CommandRegistry::build_default();
    // No `folded > 0` floor here: the matrix mixes folds with a few cases that
    // intentionally bail (e.g. `%#o`). Most fire, so the harness is exercised;
    // `check_matrix` asserts every fold that fires matches tclsh9.0.
    check_matrix(&tclsh, &reg, FORMATS);
}

/// The version-aware folds under iRules, which embeds an 8.4 fork whose release
/// was measured (`DialectProfile::evaluation_point`), against the real
/// `tclsh8.4`: where both answer they agree, and the rows that depend on the
/// release answer under the point where, with no release, they decline.
#[test]
fn versioned_folds_under_irules_match_tclsh84() {
    const DEPENDENT: &[Case] = &[
        ("string", Some("is"), &["integer", "4294967296"]),
        ("string", Some("is"), &["integer", "4294967295"]),
        ("string", Some("is"), &["integer", "-4294967296"]),
        ("string", Some("is"), &["wideinteger", "4294967296"]),
        ("string", Some("is"), &["entier", "5"]),
        ("format", None, &["%d", "010"]),
        ("format", None, &["%x", "-1"]),
    ];
    let Some(tclsh) = find_tclsh("8.4") else {
        return;
    };
    let reg = CommandRegistry::build_default();
    let point = tcl_dialect::DialectProfile::irules().evaluation_point();
    assert_eq!(point, Some(tcl_dialect::TclVersion::V8_4));

    let folded = check_matrix_under(&tclsh, &reg, FOLDABLE, point)
        + check_matrix_under(&tclsh, &reg, FORMATS, point)
        + check_matrix_under(&tclsh, &reg, DEPENDENT, point);
    assert!(
        folded > 0,
        "no row folded: the harness is not wired to the registry"
    );

    let mut moved = 0usize;
    for &(head, sub, args) in DEPENDENT {
        let under_point = registry_fold_under(&reg, head, sub, args, point);
        let invariant = registry_fold_under(&reg, head, sub, args, None);
        if under_point.is_some() && invariant.is_none() {
            moved += 1;
        }
        assert!(
            invariant.is_none() || invariant == under_point,
            "[{}] the invariant answer {invariant:?} is not the 8.4 one {under_point:?}",
            tcl_command(head, sub, args)
        );
    }
    assert!(
        moved >= 3,
        "the point answers where no release does for only {moved} of the dependent rows"
    );
}

/// A vendor profile whose base nothing measured has no evaluation point, and a
/// version-aware fold under it declines where the releases differ: `expect`
/// models an 8.6 base, and `string is integer 4294967296` is 0 on 8.x and 1 on
/// 9.x.
#[test]
fn an_unmeasured_vendor_base_leaves_a_versioned_fold_to_the_invariant_subset() {
    let reg = CommandRegistry::build_default();
    let expect = tcl_dialect::DialectProfile::find("expect").expect("catalogue profile");
    assert_eq!(expect.runtime_base, Some(tcl_dialect::TclVersion::V8_6));
    assert_eq!(expect.evaluation_point(), None);
    for args in [&["integer", "4294967296"], &["integer", "-4294967296"]] {
        assert_eq!(
            registry_fold_under(
                &reg,
                "string",
                Some("is"),
                args,
                expect.const_fold_version()
            ),
            None,
            "{args:?}: no point, no fold where 8.x and 9.x differ"
        );
    }
    assert_eq!(
        registry_fold_under(
            &reg,
            "string",
            Some("is"),
            &["integer", "4294967295"],
            expect.const_fold_version()
        )
        .as_deref(),
        Some("1"),
        "every release agrees on the largest 32-bit integer"
    );
}

/// `format` on its registry-owned route, per release found on `PATH`,
/// against the real `tclsh`: every case of the format matrix and the
/// release-gated conversions (`%b` from 8.6; `%p`, `%llu` and the `0d` /
/// `0o` prefixes from 9.0). When `tclsh` raises the route must decline, and
/// when the route answers it must print what `tclsh` prints. The iRules
/// profile is held to `tclsh8.4` as well: it is a fork of that release, and the
/// route answers under it as 8.4 does.
#[test]
fn format_witnesses_match_every_release_on_path() {
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValueOrUnavailable, LiteralInputs, resolve_semantics,
    };
    const GATED: &[Case] = &[
        ("format", None, &["%b", "5"]),
        ("format", None, &["%p", "255"]),
        ("format", None, &["%llu", "5"]),
        ("format", None, &["%#o", "8"]),
        ("format", None, &["%#d", "5"]),
    ];
    let profiles = tcl_dialect::TclVersion::ALL
        .map(|version| (version, version.dialect_profile_name()))
        .into_iter()
        .chain([(tcl_dialect::TclVersion::V8_4, "f5-irules")]);
    for (version, dialect) in profiles {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        let semantics = resolve_semantics(reg.get("format").expect("format"), None, None);
        let semantics = semantics.semantics().expect("the format route");
        let mut answered = 0usize;
        for &(head, sub, args) in FORMATS.iter().chain(GATED) {
            let inputs = LiteralInputs::new(head, sub, args, profile);
            let route = match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
                EvalAnswer::Evaluated(outcome) => match outcome.result {
                    ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).ok(),
                    ExactValueOrUnavailable::Unavailable(_) => None,
                },
                EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
            };
            let oracle = tcl_value(&tclsh, &tcl_command(head, sub, args));
            if let Some(route) = route {
                assert_eq!(
                    Some(route),
                    oracle,
                    "tclsh{} ({dialect}) {args:?}",
                    version.version_string()
                );
                answered += 1;
            }
        }
        assert!(
            answered * 2 > FORMATS.len(),
            "tclsh{} ({dialect}): the route answered only {answered} cases",
            version.version_string()
        );
    }
}

/// `binary format` witnesses: the format and its arguments. Program (2)
/// first, then the fields every release packs alike, then the spellings and
/// fields the releases part on, which the route declines.
const BINARY_FORMAT_WITNESSES: &[&[&str]] = &[
    &["H*", "414243444546"],
    &["a3 c", "foo", "65"],
    &["A5", "ab"],
    &["a*", "hello"],
    &["a0", "x"],
    &["b8 B8", "10100000", "10100000"],
    &["h2 H3", "4a", "4a5"],
    &["b*", "101"],
    &[
        "c s S i I w W",
        "-1",
        "70000",
        "258",
        "4294967297",
        "1",
        "-2",
        "3",
    ],
    &["c* c2", "1 2 3", "4 5 6"],
    &["c", " 5 "],
    &["c", "+5"],
    &["c", "300"],
    &["f d", "1.5", "0.1"],
    &["d*", "1.5 2 .5 5. 1e3"],
    &["d", "-0.0"],
    &["f", "1e-40"],
    &["c x2 X c", "1", "2"],
    &["@5 c", "1"],
    &["c @0 c", "1", "2"],
    &["c @* c", "1", "2"],
    &["c X* c", "1", "2"],
    &["", ""],
    &["c", "1", "2"],
    &["a4", "\u{ff}"],
    &["c", "010"],
    &["c", "0b101"],
    &["c", "0o17"],
    &["c", "1_0"],
    &["w", "18446744073709551616"],
    &["c* c2", "1 2 010", "4 5 010"],
    &["d", "010"],
    &["d", "-0"],
    &["d", "1e400"],
    &["d", "1e-310"],
    &["d", "0x1p3"],
    &["d", "1.5 2"],
    &["f", "1e40"],
    &["f", "3.4028235677973366e38"],
    &["t n m", "5", "5", "5"],
    &["r R q Q", "1.5", "1.5", "1.5", "1.5"],
    &["iu", "42"],
    &["x*"],
    &["c @ c", "1", "2"],
    &["c", "1 2"],
    &["c4", "1 2 3"],
    &["Z", "1"],
    &["c"],
    &["b4", "102"],
];

/// What the `binary format` route answers for `witness` under `profile`,
/// as the hex of its bytes, or `None` when it declines.
fn binary_format_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    witness: &[&str],
) -> Option<String> {
    use std::fmt::Write as _;
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValueOrUnavailable, LiteralInputs, RepresentationEvidence,
        resolve_semantics,
    };
    let spec = reg.get("binary").expect("binary");
    let semantics = resolve_semantics(spec, Some(spec.subcommand("format").expect("format")), None);
    let semantics = semantics.semantics().expect("the binary format route");
    let inputs = LiteralInputs::new("binary", Some("format"), witness, profile);
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    let ExactValueOrUnavailable::Exact(value) = outcome.result else {
        return None;
    };
    assert_eq!(
        value.representation,
        RepresentationEvidence::Constructed(tcl_registry::TclType::ByteArray),
        "{witness:?}"
    );
    let text = String::from_utf8(value.bytes).expect("text");
    let mut hex = String::new();
    for c in text.chars() {
        let byte = u8::try_from(u32::from(c)).expect("a byte");
        let _ = write!(hex, "{byte:02x}");
    }
    Some(hex)
}

/// What `tclsh` packs for `witness`, as hex, or `None` when it raises.
fn binary_format_oracle(tclsh: &str, witness: &[&str]) -> Option<String> {
    let mut script = String::from("if {[catch {binary format");
    for word in witness {
        script.push(' ');
        script.push_str(&tcl_quoted_word(word));
    }
    script.push_str(
        "} __packed]} {exit 1}\nbinary scan $__packed H* __hex\nputs -nonewline $__hex\n",
    );
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// Program (2) of the interface page and the `binary format` witnesses,
/// per release found on `PATH`, each under that release's profile
/// against the real `tclsh`: when the route answers it packs the bytes
/// `tclsh` packs, a byte array by construction; when `tclsh` raises the
/// route declines; a decline where `tclsh` answers is allowed only past the
/// shared fields — the spellings, fields and ranges the releases part on.
#[test]
fn binary_format_witnesses_match_every_release_on_path() {
    /// The witnesses every release packs alike, which the route must answer.
    const SHARED: usize = 25;
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        for (index, &witness) in BINARY_FORMAT_WITNESSES.iter().enumerate() {
            let want = binary_format_oracle(&tclsh, witness);
            let got = binary_format_route(&reg, profile, witness);
            match (&want, &got) {
                (Some(want), Some(got)) => assert_eq!(
                    got,
                    want,
                    "tclsh{}: binary format {witness:?}",
                    version.version_string()
                ),
                (None, Some(got)) => panic!(
                    "tclsh{} raises on binary format {witness:?}, the route answered {got}",
                    version.version_string()
                ),
                (Some(_), None) => assert!(
                    index >= SHARED
                        || (witness[0] == "a4" && version < tcl_dialect::TclVersion::V9_0),
                    "tclsh{}: the route declined binary format {witness:?}",
                    version.version_string()
                ),
                (None, None) => {}
            }
        }
    }
}

/// The type `string is` proves of a value is the representation its test
/// leaves it with (`tcl_registry::commands::tcl::string_is_member_type`),
/// read back with `tcl::unsupported::representation` under every release
/// from 8.6 on path: each member a typed class accepts reads as that type —
/// an integer class as `int`, or `bignum` past the wide range; `double` as
/// `double` or `int`; `dict` as `dict`, `{}` included, from 9.0 — and a
/// class left untyped accepts members of more than one: `boolean` keeps the
/// word `yes` and makes `1` an integer, `list` leaves `{}` a pure string.
#[test]
fn a_string_is_type_is_the_representation_its_test_leaves() {
    use tcl_registry::TclType;
    use tcl_registry::commands::tcl::string_is_member_type;
    // (class, its option words, members, the first release with the class)
    let classes: [(&str, &[&str], &[&str], &str); 7] = [
        (
            "integer",
            &["-strict"],
            &["12", " 12 ", "0x10", "-5"],
            "8.6",
        ),
        (
            "wideinteger",
            &["-strict"],
            &["12", "9223372036854775807"],
            "8.6",
        ),
        (
            "entier",
            &["-strict"],
            &["12", "123456789012345678901234567890"],
            "8.6",
        ),
        ("double", &["-strict"], &["1.5", "12", "1e3"], "8.6"),
        ("dict", &[], &["a b", "", "k v k2 v2"], "9.0"),
        ("boolean", &["-strict"], &["1", "yes", "true", "0"], "8.6"),
        ("list", &["-strict"], &["", "a b"], "8.6"),
    ];
    let type_of = |representation: &str| match representation {
        "int" | "wideInt" | "bignum" => Some(TclType::Int),
        "double" => Some(TclType::Double),
        "dict" => Some(TclType::Dict),
        "list" => Some(TclType::List),
        "pure" => Some(TclType::String),
        "boolean" | "booleanString" => Some(TclType::Boolean),
        _ => None,
    };
    for series in ["8.6", "9.0", "9.1"] {
        let Some(tclsh) = find_tclsh(series) else {
            continue;
        };
        for &(class, options, members, from) in &classes {
            if series < from {
                continue;
            }
            let words: Vec<String> = members
                .iter()
                .map(|member| format!("{{{member}}}"))
                .collect();
            let script = format!(
                "foreach m {{{}}} {{\n\
                 set v [string range \"x$m\" 1 end]\n\
                 if {{![string is {class} {} $v]}} {{puts reject; continue}}\n\
                 puts [lindex [tcl::unsupported::representation $v] 3]\n\
                 }}\n",
                words.join(" "),
                options.join(" ")
            );
            let (ok, out) = run_tcl(&tclsh, &script).expect("tclsh runs");
            assert!(ok, "tclsh{series}: {class}");
            let read: Vec<Option<TclType>> = out.lines().map(type_of).collect();
            assert_eq!(read.len(), members.len(), "tclsh{series}: {class}: {out}");
            let mut args: Vec<&str> = vec![class];
            args.extend_from_slice(options);
            args.push("$v");
            if let Some(proved) = string_is_member_type(&args) {
                for (member, ty) in members.iter().zip(&read) {
                    let within = ty.is_some_and(|ty| {
                        ty == proved
                            || (proved == TclType::Numeric
                                && matches!(ty, TclType::Int | TclType::Double))
                    });
                    assert!(
                        within,
                        "tclsh{series}: string is {class} {member:?}: {ty:?}"
                    );
                }
            } else {
                let first = read.first().copied().flatten();
                assert!(
                    read.iter().any(|ty| *ty != first),
                    "tclsh{series}: {class} leaves one representation, {first:?}: {out}"
                );
            }
        }
    }
}
