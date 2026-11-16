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

//! `TclOO` and megawidget class metadata and the method-frame facts an introspection keyword answers with.

use super::subcommand::SubCommand;
use crate::abbrev::{Keyword, KeywordTable, PrefixMatching};

/// A compile-time-constant fact about the **enclosing `TclOO` method frame**
/// that an introspection keyword answers with.
///
/// Declared per keyword word on [`CommandSpec::oo_context_facts`], so a
/// consumer folding `[self class]` reads registry data rather than matching
/// the command name or the subcommand word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OoContextFact {
    /// The class that **defines** the currently executing method
    /// implementation — what `self class` returns.
    ///
    /// The source fixes it: it is the class whose definition body lexically
    /// encloses the method, and it stays that class through inheritance,
    /// mixins, and `next`.  Oracle, byte-identical on tclsh 9.0.4 and 8.6.16:
    ///
    /// ```tcl
    /// oo::class create ::A { method m {} { self class } }
    /// [::A new] m                       ;# -> ::A
    /// oo::class create ::B { superclass ::A }
    /// [::B new] m                       ;# -> ::A   (the definer, not the receiver)
    /// oo::define ::B { method n {} { self class } }
    /// [::B new] n                       ;# -> ::B
    /// oo::class create ::M { method mx {} { self class } }
    /// oo::define ::B { mixin ::M } ; [::B new] mx
    ///                                   ;# -> ::M   (a mixin still reports its definer)
    /// oo::class create ::S { method q {} { list S:[self class] } }
    /// oo::class create ::T { superclass ::S
    ///                        method q {} { list T:[self class] {*}[next] } }
    /// [::T new] q                       ;# -> T:::T S:::S  (per chain link)
    /// oo::class create ::E { constructor {} { self class } }   ;# ctor -> ::E
    /// oo::class create ::G { method real {} {return real}
    ///                        method filt args { list [self class] {*}[next {*}$args] }
    ///                        filter filt }
    /// [::G new] real                    ;# -> ::G real       (a filter is a method)
    /// namespace eval ::N { oo::class create C { method m {} { self class } } }
    /// [::N::C new] m                    ;# -> ::N::C
    /// ```
    ///
    /// Two shapes make it **not** a source constant.  A consumer must abstain
    /// on both — the first is not even a value:
    ///
    /// ```tcl
    /// # (1) The implementation is not defined by a class: an `oo::objdefine`
    /// #     instance method, or a method on the class object itself
    /// #     (`self method` / `classmethod`).  `self class` raises.
    /// oo::objdefine $obj { method p {} { self class } }
    /// $obj p                ;# -> error: method not defined by a class
    /// oo::class create ::U { self method cm {} { self class } }
    /// ::U cm                ;# -> error: method not defined by a class
    ///
    /// # (2) The class command was renamed: `self class` answers with the
    /// #     class's *current* name, not the one written at definition —
    /// #     the same rename-captures-identity rule `indirection.rs` applies.
    /// oo::class create ::R { method r {} { self class } }
    /// set r [::R new] ; rename ::R ::R2
    /// $r r                  ;# -> ::R2
    /// ```
    ///
    /// **Downstream of the fold.**  The folded class name is an ordinary
    /// constant word, so it feeds the ordinary `const_fold` callbacks: this is
    /// what makes the real-corpus ticklecharts chain
    /// `set ns [namespace qualifiers [self class]] ; ${ns}::setdef …` resolve
    /// in one step.  `namespace qualifiers` / `namespace tail`
    /// are pure string operations — they split a string at its last `::` and
    /// never consult the interpreter's namespace table — so no namespace has
    /// to exist for the chain to be sound.
    DefiningClass,
}

/// Metadata for a `TclOO` / megawidget class whose instances are dispatched as
/// `$obj <method> …`.
///
/// Attached to the class *command* spec (the factory — e.g.
/// `ticklecharts::chart`, created by `oo::class create`) via
/// [`CommandSpec::object_class`].  For a `TclOO` class the class name **is** the
/// factory command name, so the registry resolves a class spec by looking the
/// name up in the ordinary command table (no separate index) — see
/// [`crate::CommandRegistry::object_class`].
///
/// The class's `new` / `create` constructor returns an object handle of
/// `class_name`; a later `$handle method …` dispatch resolves `method` against
/// [`Self::instance_methods`], which reuse the [`SubCommand`] shape so option /
/// enum / arg-value highlighting, arity, and hover work identically to an
/// ensemble subcommand.  This is the registry half of the object-method
/// pattern: knowing the class of an object handle (via the compiler's
/// object-type tracking) plus the class's methods lets `$chart Xaxis -name …`
/// light up its options precisely rather than by shape alone.
#[derive(Debug)]
pub struct ObjectClassSpec {
    /// Fully-qualified class name — equal to the factory command name for a
    /// `TclOO` class (`"ticklecharts::chart"`).
    pub class_name: &'static str,

    /// Instance methods dispatched on an object handle (`Xaxis`, `Add`,
    /// `SetOptions`, …), in declaration order.  Reuses [`SubCommand`].
    pub instance_methods: &'static [SubCommand],

    /// Direct superclass names, for inherited-method resolution.  Each is
    /// itself a class command name resolvable via
    /// [`crate::CommandRegistry::object_class`].  Empty = none.
    pub superclasses: &'static [&'static str],

    /// Whether an unrecognised instance method is accepted without complaint
    /// (a class with a dynamic `unknown` handler or runtime-generated
    /// methods).  Highlighting-only today; reserved for a future
    /// unknown-method diagnostic.
    pub allow_unknown_methods: bool,

    /// Whether the instance-method command table accepts unique prefixes.
    ///
    /// Object commands are not ensembles by default: `TclOO` and most
    /// extension object APIs dispatch the method word exactly.  Tk widget
    /// command tables are the deliberately documented exception and opt in
    /// where the Tk source proves `Tk_GetIndexFromObj`-style dispatch.
    /// Keeping this strict by default prevents a guessed object method from
    /// becoming a completion, hover, or taint fact.
    pub method_prefix_matching: PrefixMatching,
}

impl ObjectClassSpec {
    /// Look up an instance method by name (this class only — no superclass
    /// walk; the registry's [`crate::CommandRegistry::instance_method`] does
    /// the inherited resolution). Lookup follows this class's declared
    /// [`Self::method_prefix_matching`] policy: strict classes require the
    /// complete spelling, while an enabled table accepts a unique non-empty
    /// prefix. An exact spelling wins and an ambiguous prefix abstains.
    #[must_use]
    pub fn instance_method(&self, name: &str) -> Option<&SubCommand> {
        let table = KeywordTable::from_keywords(
            self.instance_methods.iter().map(|method| Keyword {
                name: method.name,
                min_abbrev: method.min_abbrev,
            }),
            self.method_prefix_matching,
        );
        let canonical = table.resolve(name).unique()?;
        self.instance_methods
            .iter()
            .find(|method| method.name == canonical)
    }
}
