// SPDX-License-Identifier: AGPL-3.0-or-later
// Native C8.5 completions and target-side-effect observations.
pub type ExecutionCase = (&'static str, &'static [u8], i32, &'static [u8], bool);
pub const EXECUTIONS: &[ExecutionCase] = &[
    ("simple-result", b"namespace upvar ::N x local; set local", 0, b"X", false),
    ("original-namespace", b"namespace upvar [set ns ::N] [set name x] local; set local", 0, b"X", false),
    ("multiple-pairs", b"namespace upvar ::N x first y second; list $first $second", 0, b"X Y", false),
    ("empty-local", b"namespace upvar ::N x {}; set {}", 0, b"X", false),
    ("recreated-namespace", b"namespace upvar ::N x first [namespace delete ::N; namespace eval ::N {variable y NEW}; set name y] second; set second", 0, b"NEW", false),
    ("defined-local", b"set local BEFORE; namespace upvar ::N x local", 1, b"variable \"local\" already exists", false),
    ("array-local-generic", b"namespace upvar ::N x a(k)", 1, b"bad variable name \"a(k)\": can't create a scalar variable that looks like an array element", false),
    ("missing-namespace-after-target", b"namespace upvar ::MISSING [set ::visited YES; set name x] local", 1, b"namespace \"::MISSING\" not found", true),
];
