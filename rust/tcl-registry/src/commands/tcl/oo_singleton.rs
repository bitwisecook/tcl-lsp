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

//! `oo::singleton` — metaclass for classes that permit at most one instance.
use super::oo_class::oo_class_arg_roles;
use crate::prelude::*;

// `oo::singleton create` / `createWithNamespace` mint a new class as a
// command in the interpreter's command table; no file I/O, process, or
// channel involvement, matching every other `TclOO` metaclass spec
// (`oo::class`, `oo::abstract`, `oo::configurable`, `oo::object`).
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
    dialects: None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "oo::singleton method ?arg ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::singleton",
        // `oo::singleton create Name { … }` is a four-token `HEAD NAME
        // BRACED BRACED` call — the same shape `oo::class`, `oo::abstract`,
        // `oo::configurable`, and `oo::object` also match, which is why it
        // carries `NOT_PROC_FACTORY` too (see the rationale on
        // `oo_abstract.rs`): without it, a `create` call written inside a
        // `proc` body would be misfiled by the nested factory-wrapper
        // rescan (`scan_factory_candidates`, run from `handle_proc`) as a
        // tcllib-style factory-wrapper candidate, since that rescan never
        // consults the top-level `IS_OO_METACLASS` + `definition_body`
        // dispatch (`dispatch_definer` in
        // `tcl-compiler/src/signature_scan/walker.rs`) that correctly
        // classifies it everywhere else.
        traits: Traits::IS_OO_METACLASS
            | Traits::LANGUAGE_KEYWORD
            | Traits::DEFINES_PROCEDURE
            | Traits::NOT_PROC_FACTORY,
        // Verified absent (soft-404 "URL Not Found" page, HTTP 200) from
        // the tcl-lang.org manpage tree for 8.4, 8.5, and 8.6 (both the
        // `.html` and, for the 8.6 tree, the `.htm` extension) — `TclOO`
        // itself shipped built into the core in 8.6, but this metaclass
        // did not exist yet. Present with byte-identical NAME/SYNOPSIS/
        // CLASS HIERARCHY/DESCRIPTION/CONSTRUCTOR/DESTRUCTOR/EXPORTED
        // METHODS/NON-EXPORTED METHODS/EXAMPLE text in both the 9.0.4 and
        // 9.1b0 singleton.n pages (only the version banner and internal
        // anchor IDs differ) — a Tcl 9.0 addition, unchanged in 9.1.
        dialects: Some(DialectSet::TCL90_PLUS),
        arity: Arity::at_least(1),
        arg_role_resolver: Some(oo_class_arg_roles),
        return_type: Some(TclType::String),
        // Bodies of `oo::singleton create / new / createWithNamespace`
        // run in a TclOO definition context, exactly like `oo::class`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet {
            summary: "metaclass for singleton classes",
            synopsis: &[
                "oo::singleton method ?arg ...?",
                "oo::singleton create name ?definition?",
            ],
            snippet: "Singleton classes permit at most one instance of themselves. A class created via oo::singleton unexports its own create and createWithNamespace methods entirely, and overrides new so that it returns the existing instance once one has been made, constructing one — via oo::class's ordinary new — only the first time; arguments passed to new are used solely on that first call and ignored on every later call, which is why giving a singleton class's constructor any arguments is discouraged. destroy is overridden to always raise an error, discouraging destruction of the instance, though it remains possible by destroying the class itself or by renaming the instance away; the non-exported <cloned> method is likewise overridden to always error, so oo::copy of a singleton instance fails. Inheriting from a singleton class is possible but not recommended, since singleton-ness itself is not inherited by subclasses. oo::singleton defines no constructor or destructor of its own — both behave exactly as oo::class's. oo::singleton is part of tcl::oo (`package require tcl::oo`), a Tcl 9.0 addition; TclOO itself has shipped built into the core interpreter since Tcl 8.6.",
            source: "Tcl man page singleton.n",
            examples: "oo::singleton create Highlander {\n    method say {} {\n        puts \"there can be only one\"\n    }\n}\nset h1 [Highlander new]\nset h2 [Highlander new]\nif {$h1 eq $h2} {\n    puts \"equal objects\"\n}",
            return_value: "The fully qualified name of the newly created class for create/new/createWithNamespace; otherwise whatever the invoked oo::class-inherited method returns.",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        definition_body: Some(&crate::definer::TCLOO_GRAMMAR),
        manufacturer_methods: crate::definer::TCLOO_DERIVED_METACLASS_MANUFACTURERS,
        ..CommandSpec::DEFAULT
    }
}
