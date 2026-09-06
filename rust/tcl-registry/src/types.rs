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

//! Tcl internal representation types.
//!
//! Tcl values are always strings but may cache a typed internal
//! representation. This enum models the set of known intreps used
//! throughout the registry, compiler, and analyser.

use crate::documentation::{DocumentationAnnotation, DocumentationCarrier, DocumentationExample};

/// Known Tcl internal representation types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TclType {
    /// Pure string (no cached intrep).
    String,
    /// Integer.
    Int,
    /// Double-precision float.
    Double,
    /// Boolean.
    Boolean,
    /// Tcl list.
    List,
    /// Tcl dict.
    Dict,
    /// Byte array.
    ByteArray,
    /// Abstract join of `Int` and `Double`.
    Numeric,
    /// `TclOO` object instance.
    Object,
    /// I/O channel handle.
    Channel,
}

impl TclType {
    /// Every intrep, in declaration order.
    pub const ALL: &'static [Self] = &[
        Self::String,
        Self::Int,
        Self::Double,
        Self::Boolean,
        Self::List,
        Self::Dict,
        Self::ByteArray,
        Self::Numeric,
        Self::Object,
        Self::Channel,
    ];

    /// Registry-owned program showing a shipped command producing this intrep
    /// and what the type checker then knows: which later read is free, which
    /// re-represents the value (S100), which is rejected outright (W126 /
    /// W308). The carrier is the command whose `return_type` declares the
    /// intrep; `Object` has none — no shipped spec declares it, the class
    /// lattice infers it from a known class's constructor. This exhaustive
    /// match is the compile gate for intrep documentation.
    #[must_use]
    pub const fn example(self) -> DocumentationExample {
        macro_rules! typed {
            ($code:literal; carrier ($cline:literal, $cneedle:literal); $(($line:literal, $needle:literal, $label:literal)),+ $(,)?) => {
                {
                    const ANNOTATIONS: &[DocumentationAnnotation] =
                        &[$(DocumentationAnnotation::new($line, $needle, $label)),+];
                    DocumentationExample::with_carrier($code, DocumentationCarrier::new($cline, $cneedle), ANNOTATIONS)
                }
            };
            ($code:literal; $(($line:literal, $needle:literal, $label:literal)),+ $(,)?) => {
                {
                    const ANNOTATIONS: &[DocumentationAnnotation] =
                        &[$(DocumentationAnnotation::new($line, $needle, $label)),+];
                    DocumentationExample::new($code, ANNOTATIONS)
                }
            };
        }
        match self {
            Self::String => {
                typed!("set fields [string trim $line]\nset count [llength $fields]\nforeach field $fields { puts $field }"; carrier (0, "string trim"); (0, "string trim", "returns a pure string with no cached intrep"), (1, "llength $fields", "converts it to a list once, for free: a pure string's first conversion is not a shimmer"), (2, "foreach field $fields", "reuses the list rep it now carries"))
            }
            Self::Int => {
                typed!("set count [llength $items]\nincr count\nappend count \" items\""; carrier (0, "llength"); (0, "llength", "leaves count known to hold an int"), (1, "incr count", "reads that int with no conversion"), (2, "append count", "wants a string, which a numeric rep regenerates cheaply, so no S100 fires"))
            }
            Self::Double => {
                typed!("scale .volume -from 0 -to 1 -resolution 0.01\nset level [.volume get]\nincr level"; carrier (1, ".volume get"); (0, "scale .volume", "creates a slider widget"), (1, ".volume get", "returns its position as a double"), (2, "incr level", "wants an int, so the double result is the mismatch S100 reports"))
            }
            Self::Boolean => {
                typed!("set ok [string is integer -strict $port]\nif {$ok} { puts valid }\nset score [expr {$ok * 10}]"; carrier (0, "string is"); (0, "string is", "leaves ok known to hold a boolean"), (1, "if {$ok}", "reads it in boolean context with no conversion"), (2, "$ok * 10", "arithmetic accepts the boolean as 0 or 1"))
            }
            Self::List => {
                typed!("set parts [split $path /]\nset name [lindex $parts end]\nstring length $parts"; carrier (0, "split"); (0, "split", "leaves parts known to hold a list"), (1, "lindex $parts end", "reads the cached list structure directly"), (2, "string length $parts", "wants a string, so the list structure is discarded and S100 fires"))
            }
            Self::Dict => {
                typed!("set info [dict create host db1 port 5432]\nset port [dict get $info port]\nforeach {key value} $info { puts $key }"; carrier (0, "dict create"); (0, "dict create", "leaves info known to hold a dict"), (1, "dict get $info port", "looks the key up in the hash directly"), (2, "foreach {key value} $info", "wants a list, so the dict is re-represented and S100 fires"))
            }
            Self::ByteArray => {
                typed!("set bytes [binary decode base64 $blob]\nset text [encoding convertfrom utf-8 $bytes]\nputs $text"; carrier (0, "binary decode"); (0, "binary decode", "leaves bytes known to hold binary data, which S110 tracks"), (1, "encoding convertfrom", "decodes those bytes into characters, the legitimate direction"), (2, "$text", "is an ordinary string again"))
            }
            Self::Numeric => {
                typed!("set total [expr {$price * $qty}]\nif {$total > 100} { puts large }\nlindex $total 0"; carrier (0, "expr"); (0, "expr", "leaves total known to be a number, int or double undecided until run time"), (1, "$total > 100", "compares it with no conversion"), (2, "lindex $total 0", "wants a list, so the number is re-represented and S100 fires"))
            }
            Self::Object => {
                typed!("oo::class create Account { method balance {} { return 0 } }\nset acct [Account new]\n$acct balance\n$acct withdraw 5"; (1, "Account new", "yields an object whose class the analyser knows"), (2, "$acct balance", "dispatches a method Account defines"), (3, "withdraw", "is not a method of Account, so W308 fires"))
            }
            Self::Channel => {
                typed!("set count [llength $items]\nset log [open app.log a]\nputs $log $count\nputs $count done"; carrier (1, "open"); (1, "open app.log a", "leaves log known to hold a channel handle"), (2, "puts $log", "accepts it in channel position"), (3, "puts $count", "passes an int where a channel is expected, so W126 fires"))
            }
        }
    }
}

/// How a command types the variable(s) it *writes* as a side effect — its
/// [`ArgRole::VarWrite`](crate::arg_role::ArgRole::VarWrite) / IR `defs`
/// targets — as distinct from the value the command *returns*
/// ([`CommandSpec::return_type`](crate::CommandSpec::return_type)).
///
/// A variable a command writes does not always receive the command's return
/// value.  `append` / `lappend` store exactly what they return, so the return
/// type describes both.  But a destructuring command returns one thing while
/// writing another: `lassign` returns the *leftover* list yet writes list
/// *elements*; `scan` / `regexp` / `binary scan` return a match/convert
/// *count* yet write parsed pieces; `gets chan line` returns the character
/// count yet writes the *line*.  Broadcasting the return type onto those
/// targets is the S100 / W126 false-positive source (issue #867): a `lassign`
/// target wrongly typed `List`, a `regexp` capture wrongly typed `Int`.
///
/// The compiler's type-inference pass reads this per command / subcommand so
/// it never keys on the command name — the distinction lives in the registry
/// as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum VarWriteTyping {
    /// The written variable's new value *is* the command's return value, so
    /// type it from [`CommandSpec::return_type`](crate::CommandSpec::return_type).
    /// The default — matches `append`, `lappend`, `ledit`, `lset`, `dict set`,
    /// and every writer whose stored value is its result.
    #[default]
    ReturnValue,
    /// The written variable receives a fixed intrep, independent of the
    /// command's return value.  `gets chan line` stores a text `String` line
    /// (while returning the character count); `lpop listVar` leaves a `List`
    /// (while returning the popped element).
    Fixed(TclType),
    /// The written variables receive destructured elements / parsed pieces
    /// whose static intrep is unknown and unrelated to the return value —
    /// `scan` / `binary scan` (format-dependent conversions), `regexp` /
    /// `regsub` (matched substrings).  Each target widens to *overdefined*
    /// so no downstream type check reads a bogus intrep.
    Destructured,
    /// The written variables receive the container argument's elements
    /// **positionally**: target `i` takes element `i` of the container at
    /// `container_arg` (0-based, after the command / subcommand word).
    /// `lassign $l a b` types `a` / `b` from `$l`'s tracked per-position
    /// element shapes when known, and widens each target to *overdefined*
    /// otherwise (the pre-element-tracking `Destructured` behaviour).
    /// `foreach` / `lmap` element variables share this semantic; their
    /// group→container mapping rides the lowered statement's `foreach_groups`
    /// metadata rather than a single argument index.
    ElementsOf {
        /// 0-based index of the container argument.
        container_arg: u8,
    },
}

/// How a command's *result value* relates to container element structure —
/// the registry fact behind per-element type inference
/// (`docs/design/compiler/type-tracking.md`, P3). Read by the compiler's
/// type-propagation pass; never keyed on command names in the compiler.
///
/// Faithful to the runtime: container elements are shared `Tcl_Obj`s, so a
/// builder's computed argument keeps its intrep inside the container, and a
/// retrieval hands back the same object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReturnElements {
    /// The result is a list of exactly the argument words from `from`
    /// onward, one element per word in order (`list ?value …?`).
    ListOfArgs {
        /// 0-based index of the first element word.
        from: u8,
    },
    /// The result is a dict built from alternating key/value words from
    /// `from` onward (`dict create ?key value …?`); element facts describe
    /// the **values**.
    DictOfPairs {
        /// 0-based index of the first key word.
        from: u8,
    },
    /// The result is one element of the container argument — `lindex $l $i`
    /// (single-index form) / `dict get $d $k` (single-key form). Multi-level
    /// paths are outside this fact; the compiler applies it only to the
    /// single-step call shape.
    ElementOf {
        /// 0-based index of the container argument.
        container_arg: u8,
    },
    /// The result is a sub-list of the container argument (`lrange $l a b`):
    /// a list whose every element is bounded by the source's uniform element
    /// shape.
    SubListOf {
        /// 0-based index of the container argument.
        container_arg: u8,
    },
}

/// How a command evolves the container *elements* of the variable it writes
/// in place — the registry fact that generalises the old object-only
/// element-class harvesting to every element shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VarElementsEffect {
    /// `lappend var ?value …?` — appends each value word (from `values_from`
    /// onward) as one new element of the variable's list.
    AppendsListElements {
        /// 0-based index of the first appended value word.
        values_from: u8,
    },
    /// `dict set var ?key …? value` — stores the final word as one value of
    /// the variable's dict. Only the single-key form carries the leaf's
    /// shape; a nested path (`dict set d outer inner v`) stores a *dict*
    /// under the first key (tclsh 8.6/9.0: `dict get $d outer` is a dict),
    /// so the consumer records a value-shape of `Dict` with unknown
    /// structure for multi-key writes.
    SetsDictValue,
    /// `dict append var key ?value …?` — the stored value is a string
    /// *concatenation*, so value **intreps do not survive** (tclsh 8.6:
    /// string for both a missing and an existing key; 9.0 keeps the
    /// argument's intrep only on the missing-key fast path). What does
    /// survive is an object's *dispatch identity* — the objref text — so
    /// only object-class facts flow into the value bound (the
    /// collection-of-objects pattern, issue #797); every other shape
    /// contributes nothing.
    ExtendsDictValuesByName {
        /// 0-based index of the first appended value word.
        values_from: u8,
    },
    /// `dict lappend var key ?value …?` — the key's value becomes a *list*
    /// (tclsh: `dict get` after `dict lappend` has list intrep; a prior
    /// scalar value becomes element 0). The list wrapper is the fact; the
    /// element shapes are not tracked (the prior value's shape is part of
    /// the elements and is not statically known).
    ListifiesDictValue,
}

/// Shared checks for a closed vocabulary's worked examples, used by every
/// registry enum that owns an `example()` table.
#[cfg(test)]
pub(crate) mod example_checks {
    use crate::documentation::DocumentationExample;
    use std::collections::HashSet;

    /// Every arrow and the carrier point at text that really occurs on their
    /// line, every label is a tight phrase, each example has two to four
    /// arrows that say at least two different things, and no two variants
    /// share a program.
    pub(crate) fn assert_examples_valid(
        vocabulary: &str,
        examples: &[(String, DocumentationExample)],
    ) {
        let mut programs = HashSet::new();
        for (variant, example) in examples {
            let owner = format!("{vocabulary}::{variant}");
            let lines: Vec<&str> = example.code.lines().collect();
            assert!(
                programs.insert(example.code),
                "{owner} reuses another variant's worked example"
            );
            assert!(
                (2..=4).contains(&example.annotations.len()),
                "{owner} needs two to four arrows"
            );
            let labels: HashSet<&str> = example.annotations.iter().map(|a| a.label).collect();
            assert!(labels.len() >= 2, "{owner} has boilerplate-only arrows");
            for annotation in example.annotations {
                assert!(!annotation.needle.is_empty(), "{owner} has an empty needle");
                assert!(!annotation.label.is_empty(), "{owner} has an empty label");
                assert!(
                    !annotation.label.ends_with('.'),
                    "{owner} label {:?} ends with a full stop",
                    annotation.label
                );
                assert!(
                    lines
                        .get(annotation.line)
                        .is_some_and(|line| line.contains(annotation.needle)),
                    "{owner}: line {} does not contain {:?} in {:?}",
                    annotation.line,
                    annotation.needle,
                    example.code
                );
            }
            if let Some(carrier) = example.carrier {
                assert!(!carrier.needle.is_empty(), "{owner} has an empty carrier");
                assert!(
                    lines
                        .get(carrier.line)
                        .is_some_and(|line| line.contains(carrier.needle)),
                    "{owner}: carrier line {} does not contain {:?}",
                    carrier.line,
                    carrier.needle
                );
                assert!(
                    example.annotations.iter().any(|annotation| {
                        annotation.line == carrier.line
                            && annotation.needle.contains(carrier.needle)
                    }),
                    "{owner}: carrier is not explained by an arrow"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_intrep_has_a_distinct_source_aligned_example() {
        let examples: Vec<_> = TclType::ALL
            .iter()
            .map(|&t| (format!("{t:?}"), t.example()))
            .collect();
        example_checks::assert_examples_valid("TclType", &examples);
    }

    /// The carrier is the shipped command that declares the intrep; only
    /// `Object`, which no shipped spec declares, is a carrier-less flow.
    #[test]
    fn only_object_lacks_a_declaring_carrier() {
        for &t in TclType::ALL {
            assert_eq!(
                t.example().carrier.is_none(),
                t == TclType::Object,
                "{t:?} carrier presence"
            );
        }
    }
}
