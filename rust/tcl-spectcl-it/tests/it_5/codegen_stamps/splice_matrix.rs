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

//! Every shape of inlined body at every call site answers as its definition does.

use super::*;

/// The bodies a pack gives its commands, one for each shape the splice has a path
/// for: `(name, arity, definition)`.
const SPLICED_SHAPES: &[(&str, usize, &str)] = &[
    // A trailing `return`.
    ("vdouble", 1, "proc vdouble {x} {return [expr {$x * 2}]}"),
    // No `return`: the last command's value.
    ("vtail", 1, "proc vtail {x} {expr {$x * 2}}"),
    // A `return` before the end, and a trailing one.
    (
        "vabs",
        1,
        "proc vabs {x} {if {$x < 0} {return [expr {-$x}]}; return $x}",
    ),
    // A `return` before the end, and a last command's value.
    (
        "vsign",
        1,
        "proc vsign {x} {if {$x < 0} {return neg}; set r pos}",
    ),
    ("vconst", 0, "proc vconst {} {return fixed}"),
    (
        "vlocal",
        1,
        "proc vlocal {x} {set y [expr {$x + 1}]; return $y}",
    ),
    ("vinc", 1, "proc vinc {x} {set y $x; incr y; return $y}"),
    ("vnoop", 0, "proc vnoop {} {}"),
    ("vpair", 2, "proc vpair {a b} {return \"$a-$b\"}"),
    // A braced word, which is literal.
    ("vlit", 1, "proc vlit {x} {string length {$x}}"),
    ("vbrace", 1, "proc vbrace {x} {return {$x}}"),
    // A command a word substitutes.
    (
        "vsubst",
        1,
        "proc vsubst {x} {set n [string length $x]; return \"$n-$x\"}",
    ),
    // A value from a branch that has no `return`, and a loop the caller's own
    // loop would nest.
    (
        "vif",
        1,
        "proc vif {x} {if {$x < 0} {set r neg} else {set r pos}}",
    ),
    (
        "vloop",
        1,
        "proc vloop {x} {set t 0; foreach i $x {incr t $i}; set t}",
    ),
    // A braced word with a backslash, and one with brackets, which are literal
    // where the value is the last command's and where it is a `return`'s; the
    // `return {[…]}` form is `a_braced_literal_stays_literal_through_the_wrap`'s.
    (
        "vsetbs",
        1,
        "proc vsetbs {x} {if {$x < 0} {return neg}; set r {a\\tb}}",
    ),
    (
        "vretbs",
        1,
        "proc vretbs {x} {if {$x > 0} {return {a\\tb}}; return neg}",
    ),
    (
        "vsetbr",
        1,
        "proc vsetbr {x} {if {$x < 0} {return neg}; set r {[string length $x]}}",
    ),
    // A variable a command a word substitutes names: no `$` shows the read.
    ("vsetcmd", 1, "proc vsetcmd {x} {set y $x; return [set y]}"),
    (
        "vincrcmd",
        1,
        "proc vincrcmd {x} {set y $x; return [incr y]}",
    ),
    (
        "vfmtcmd",
        1,
        "proc vfmtcmd {x} {return [format %s [set x]]}",
    ),
    ("vexprcmd", 1, "proc vexprcmd {x} {expr {[set x] + 1}}"),
];

/// The places a call can stand, `@CALL@` the call: where its value is the
/// procedure's, where nothing reads it, and where a command around it does.
///
/// An `if` inside an `if` is not among them: the plain compile of a procedure
/// whose value comes from the inner one answers the empty string where Tcl
/// answers the value, so it is no reference for the splice.
const SPLICE_SITES: &[(&str, &str)] = &[
    ("terminal", "proc caller {n} {@CALL@}"),
    ("after a set", "proc caller {n} {set a 5; @CALL@}"),
    (
        "dropped before a return",
        "proc caller {n} {@CALL@; set k 1; return $k}",
    ),
    (
        "dropped before a last set",
        "proc caller {n} {@CALL@; set k 1}",
    ),
    (
        "then arm",
        "proc caller {n} {if {$n != 0} {@CALL@} else {return zero}}",
    ),
    (
        "else arm",
        "proc caller {n} {if {$n == 0} {return zero} else {@CALL@}}",
    ),
    (
        "switch arm",
        "proc caller {n} {switch -- $n {0 {return zero} default {@CALL@}}}",
    ),
    ("static eval", "proc caller {n} {eval {@CALL@}}"),
    (
        "static eval, then more",
        "proc caller {n} {eval {@CALL@}; return after}",
    ),
    (
        "catch result",
        "proc caller {n} {catch {@CALL@} r; return $r}",
    ),
    (
        "catch code",
        "proc caller {n} {set c [catch {@CALL@}]; return $c}",
    ),
    (
        "try ok",
        "proc caller {n} {try {@CALL@} on ok {r} {return $r}}",
    ),
    (
        "try terminal",
        "proc caller {n} {try {@CALL@} on error {e} {return err}}",
    ),
    (
        "try handler",
        "proc caller {n} {try {error x} on error {e} {@CALL@}}",
    ),
    ("uplevel", "proc caller {n} {uplevel 0 {@CALL@}}"),
    (
        "loop body",
        "proc caller {n} {foreach i {1 2} {@CALL@}; return done}",
    ),
    (
        "while body",
        "proc caller {n} {while {1} {@CALL@; break}; return done}",
    ),
    ("twice", "proc caller {n} {@CALL@; @CALL@}"),
    (
        "in a substitution",
        "proc caller {n} {set y [@CALL@]; return $y}",
    ),
    // A body whose value somebody reads: each iteration of an `lmap` and of a
    // `dict map` gives its body's value to the list it builds.
    ("lmap body", "proc caller {n} {lmap i {1 2 3} {@CALL@}}"),
    (
        "lmap body after a set",
        "proc caller {n} {lmap i {1 2} {set a 5; @CALL@}}",
    ),
    (
        "dict map body",
        "proc caller {n} {dict map {k v} {a 1 b 2} {@CALL@}}",
    ),
    // A caller that holds a variable of each spelling the bodies use.
    (
        "caller with the bodies' names",
        "proc caller {n} {set x 42; set y 43; set r 44; @CALL@}",
    ),
];

/// Every body, at every site, answers what its definition answers when it is
/// called as a procedure — for a value, a zero and a negative — and the VM takes
/// the function as compiled. The matrix is a comparison, so it needs no list of
/// expected answers; it asks only that the splice is the call it replaced, and
/// that the plain shapes are spliced at all, so the comparison is not of a
/// call with itself.
#[test]
fn every_shape_of_inlined_body_answers_as_its_definition_does_at_every_site() {
    crate::install_surfaces();
    let pack = Spliced::new("splice-matrix", SPLICED_SHAPES);
    let mut disagreements = Vec::new();
    let mut spliced = 0usize;
    for (name, arity, _) in SPLICED_SHAPES {
        let call = if *arity == 0 {
            (*name).to_owned()
        } else {
            format!("{name} {}", vec!["$n"; *arity].join(" "))
        };
        for (site, template) in SPLICE_SITES {
            let source = template.replace("@CALL@", &call);
            spliced += pack.bindings(&source, "::caller");
            for n in ["-5", "0", "7"] {
                let script = format!("caller {n}");
                let want = pack.reference(&[&source], &script);
                let got = pack.inlined(&[&source], &script);
                if got.code != want.code || got.answer != want.answer || got.plain != 0 {
                    disagreements.push(format!(
                        "{name} at `{site}` ({script}): the definition answers {:?} {:?}, the \
                         splice {:?} {:?} after {} plain recompile(s)\n    {source}",
                        want.code, want.answer, got.code, got.answer, got.plain
                    ));
                }
            }
        }
    }
    assert!(
        disagreements.is_empty(),
        "{} disagreement(s):\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
    assert!(
        spliced >= SPLICED_SHAPES.len() * 6,
        "the comparison is of a call with itself: only {spliced} splice(s) were made"
    );
}
