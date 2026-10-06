// SPDX-License-Identifier: AGPL-3.0-or-later
// Same original source and observer geometry as the pinned native probe.
const CASES: &[&str] = &[
    "string equal $left $right",
    "string e $left $right",
    "string {equal} $left $right",
    "string equal -nocase $left $right",
    "string length $left",
    "string len $left",
    "string \"length\" $left",
    "llength $left",
    "llength",
    "llength $left $right",
    "string {*}{length} $left",
    "string {*}{equal} $left $right",
    "llength {*}{a b}",
    "string length",
    "string equal $left",
];
