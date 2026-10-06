// SPDX-License-Identifier: AGPL-3.0-or-later
// Exact source bodies from the native C8.5 compiler fixture.
pub const CASES: &[(&str, &[u8])] = &[
    ("simple", b"namespace upvar ::N x local"),
    ("namespace-child", b"namespace upvar [set ns ::N] x local"),
    ("other-child", b"namespace upvar ::N [set other x] local"),
    ("ordered-pairs", b"namespace upvar [set ns ::N] [set a x] left [set b y] right"),
    ("quoted-local", b"namespace upvar ::N x \"local\""),
    ("braced-local", b"namespace upvar ::N x {local}"),
    ("escaped-local", b"namespace upvar ::N x lo\\x63al"),
    ("empty-local", b"namespace upvar ::N x {}"),
    ("array-local", b"namespace upvar ::N x a(k)"),
    ("qualified-local", b"namespace upvar ::N x ::local"),
    ("partial-array", b"namespace upvar [set ns ::N] [set a x] good [set b y] bad(k)"),
    ("partial-dynamic", b"namespace upvar [set ns ::N] [set a x] good [set b y] $local"),
    ("braced-subcommand", b"namespace {upvar} ::N x local"),
    ("literal-expansion", b"namespace {*}{upvar ::N x local}"),
    ("dynamic-expansion", b"namespace upvar ::N {*}$pairs"),
];
