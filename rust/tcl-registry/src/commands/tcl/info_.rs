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

//! `info` — information about the state of the Tcl interpreter.

use crate::hooks::InlineCodegenHookId;
use crate::prelude::*;
use tcl_dialect::TclVersion;
use tcl_dialect::model::SpecSurface;

// C 8.5.19 defaultInfoMap has NULL hooks for these members. C 8.6.18,
// 9.0.4 and 9.1.0 register TclCompileBasic*ArgCmd, which captures the private
// command name through TclCompileInvocation; the handler is looked up later.
const INFO_NAMED_LOOKUPS: &[crate::native_compilation::NativeCompilerImplementationLookup] = &[
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "args",
        slot: "::tcl::info::args",
        command: "info",
        prepended: &["args"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "body",
        slot: "::tcl::info::body",
        command: "info",
        prepended: &["body"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "cmdcount",
        slot: "::tcl::info::cmdcount",
        command: "info",
        prepended: &["cmdcount"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "complete",
        slot: "::tcl::info::complete",
        command: "info",
        prepended: &["complete"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "default",
        slot: "::tcl::info::default",
        command: "info",
        prepended: &["default"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "frame",
        slot: "::tcl::info::frame",
        command: "info",
        prepended: &["frame"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "functions",
        slot: "::tcl::info::functions",
        command: "info",
        prepended: &["functions"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "globals",
        slot: "::tcl::info::globals",
        command: "info",
        prepended: &["globals"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "hostname",
        slot: "::tcl::info::hostname",
        command: "info",
        prepended: &["hostname"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "library",
        slot: "::tcl::info::library",
        command: "info",
        prepended: &["library"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "loaded",
        slot: "::tcl::info::loaded",
        command: "info",
        prepended: &["loaded"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "locals",
        slot: "::tcl::info::locals",
        command: "info",
        prepended: &["locals"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "nameofexecutable",
        slot: "::tcl::info::nameofexecutable",
        command: "info",
        prepended: &["nameofexecutable"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "patchlevel",
        slot: "::tcl::info::patchlevel",
        command: "info",
        prepended: &["patchlevel"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "procs",
        slot: "::tcl::info::procs",
        command: "info",
        prepended: &["procs"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "script",
        slot: "::tcl::info::script",
        command: "info",
        prepended: &["script"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "sharedlibextension",
        slot: "::tcl::info::sharedlibextension",
        command: "info",
        prepended: &["sharedlibextension"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "tclversion",
        slot: "::tcl::info::tclversion",
        command: "info",
        prepended: &["tclversion"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "vars",
        slot: "::tcl::info::vars",
        command: "info",
        prepended: &["vars"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "cmdtype",
        slot: "::tcl::info::cmdtype",
        command: "info",
        prepended: &["cmdtype"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "constant",
        slot: "::tcl::info::constant",
        command: "info",
        prepended: &["constant"],
    },
    crate::native_compilation::NativeCompilerImplementationLookup {
        ensemble: "::info",
        member: "consts",
        slot: "::tcl::info::consts",
        command: "info",
        prepended: &["consts"],
    },
];

const fn named_member_compilation(
    lookup: &'static crate::native_compilation::NativeCompilerImplementationLookup,
    arity: Arity,
) -> crate::native_compilation::NativeCompilationSpec {
    crate::native_compilation::NativeCompilationSpec {
        grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
            lookup,
            implementation_from: TclVersion::V8_5,
            hook_from: TclVersion::V8_6,
            arity,
        },
        operation: crate::SemanticOperationId::Invoke,
        body: crate::native_compilation::NativeBodyCompilation::Inherit,
    }
}

// Source proof: naming.info.original-named-member-source-hooks
// docs/design/analysis/name-resolution-proofs/info-original-named-member-source-hooks.md
// These Basic* workers first exist in C9's original defaultInfoMap. Their
// named compiler does not issue TclOO storage or callable-binding authority.
const fn named_c9_member_compilation(
    lookup: &'static crate::native_compilation::NativeCompilerImplementationLookup,
    arity: Arity,
) -> crate::native_compilation::NativeCompilationSpec {
    crate::native_compilation::NativeCompilationSpec {
        grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
            lookup,
            implementation_from: TclVersion::V9_0,
            hook_from: TclVersion::V9_0,
            arity,
        },
        operation: crate::SemanticOperationId::Invoke,
        body: crate::native_compilation::NativeBodyCompilation::Inherit,
    }
}

// TclOO installs these original ensemble/worker tokens in tclOOInfo.c.
// Specialized object opcodes retain their hook without claiming a Basic recipe.
macro_rules! oo_info_lookup {
    ($kind:literal, $parent:literal, $member:literal) => {
        crate::native_compilation::NativeCompilerImplementationLookup {
            ensemble: concat!("::oo::Info", $kind),
            member: $member,
            slot: concat!("::oo::Info", $kind, "::", $member),
            command: "info",
            prepended: &[$parent, $member],
        }
    };
}
macro_rules! oo_info_compiler {
    ($kind:literal, $parent:literal, $member:literal, specialized $helper:ident) => {
        crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::WithImplementationPath {
                compiler: &crate::native_compilation::NativeCompilationSpec {
                    grammar: crate::native_compilation::NativeCompilationGrammar::TclOoHelper(
                        crate::native_tcloo_compilation::NativeTclOoHelper::$helper,
                    ),
                    operation: crate::SemanticOperationId::Invoke,
                    body: crate::native_compilation::NativeBodyCompilation::Inherit,
                },
                lookups: &[oo_info_lookup!($kind, $parent, $member)],
                implementation_from: TclVersion::V8_6,
                monolithic_no_hook_before: false,
            },
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }
    };
    ($kind:literal, $parent:literal, $member:literal, $arity:expr) => {
        crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup: &oo_info_lookup!($kind, $parent, $member),
                implementation_from: TclVersion::V8_6,
                hook_from: TclVersion::V8_6,
                arity: $arity,
            },
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }
    };
}
macro_rules! oo_info_ensemble_compiler {
    ($parent:literal, $kind:literal) => {
        crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::WithImplementationPath {
                compiler: &crate::native_compilation::NativeCompilationSpec {
                    grammar: crate::native_compilation::NativeCompilationGrammar::HookFrom(
                        TclVersion::V8_6,
                    ),
                    operation: crate::SemanticOperationId::Invoke,
                    body: crate::native_compilation::NativeBodyCompilation::Inherit,
                },
                lookups: &[
                    crate::native_compilation::NativeCompilerImplementationLookup {
                        ensemble: "::info",
                        member: $parent,
                        slot: concat!("::oo::Info", $kind),
                        command: "info",
                        prepended: &[$parent],
                    },
                ],
                implementation_from: TclVersion::V8_6,
                monolithic_no_hook_before: false,
            },
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }
    };
}

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "info option ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

/// A concise `SubSubCommand` — most `info object`/`info class` operations are
/// available since 8.6 (dialect `None`, inheriting the parent subcommand).
///
/// Every `9.0`-gated fact below (`sub_since` call sites) was cross-checked
/// against the Tcl 9.1b0 and 9.1.0 manpages as well: `info.n` in 9.1 differs
/// from 9.0 only in the version banner and synopsis typesetting, so a
/// `TCL90_PLUS` gate is exact for both releases — there is no 9.1-only delta to
/// model separately.
const fn sub(
    native_compilation: crate::native_compilation::NativeCompilationSpec,
    name: &'static str,
    detail: &'static str,
    synopsis: &'static str,
) -> SubSubCommand {
    SubSubCommand {
        native_compilation: Some(native_compilation),
        name,
        detail,
        synopsis,
        ..SubSubCommand::DEFAULT
    }
}

/// A `SubSubCommand` gated to a later dialect (e.g. the 9.0 `TclOO`
/// introspection additions below, TIP 500/524/558).
///
/// `since` states the same fact on the *version* axis that `dialects` states
/// on the dialect-bit axis: `info` is a core command with no owning package,
/// so its lifecycle axis is the Tcl core release the file targets. The two
/// are kept as separate arguments rather than derived from one another
/// because a dialect set is a membership mask, not an ordered release.
const fn sub_in_surface(
    mut subcommand: SubSubCommand,
    surface: &'static [SpecSurface],
) -> SubSubCommand {
    subcommand.surface = Some(surface);
    subcommand
}

const fn sub_since(
    native_compilation: crate::native_compilation::NativeCompilationSpec,
    name: &'static str,
    detail: &'static str,
    synopsis: &'static str,
    surface: &'static [SpecSurface],
    since: &'static str,
) -> SubSubCommand {
    SubSubCommand {
        native_compilation: Some(native_compilation),
        name,
        detail,
        synopsis,
        surface: Some(surface),
        lifecycle: Lifecycle::introduced_in(since),
        ..SubSubCommand::DEFAULT
    }
}

const INFO_PROPERTIES_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-all",
        detail: "Include inherited properties.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-readable",
        detail: "List readable properties (the default).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-writable",
        detail: "List writable properties.",
        ..OptionSpec::DEFAULT
    },
];

const fn properties_sub(
    native_compilation: crate::native_compilation::NativeCompilationSpec,
    detail: &'static str,
    synopsis: &'static str,
) -> SubSubCommand {
    SubSubCommand {
        name: "properties",
        detail,
        synopsis,
        options: Some(INFO_PROPERTIES_OPTIONS),
        option_prefix_words: 1,
        surface: Some(SpecSurface::TCL90_PLUS),
        lifecycle: Lifecycle::introduced_in("9.0"),
        native_compilation: Some(native_compilation),
    }
}

const fn methods_sub(
    native_compilation: crate::native_compilation::NativeCompilationSpec,
    detail: &'static str,
    synopsis: &'static str,
) -> SubSubCommand {
    SubSubCommand {
        name: "methods",
        detail,
        synopsis,
        options: Some(&crate::native_tcloo_info::METHOD_INFO_OPTIONS),
        option_prefix_words: 1,
        surface: Some(SpecSurface::TCL86_PLUS),
        lifecycle: Lifecycle::UNSPECIFIED,
        native_compilation: Some(native_compilation),
    }
}

/// Second-level subcommands of `info object` (the OBJECT INTROSPECTION
/// operations from the `info` man page). Base set is Tcl 8.6; `creationid`
/// is 9.0 (TIP 500) and `properties` is 9.0 (TIP 558).
const INFO_OBJECT_SUBS: &[SubSubCommand] = &[
    sub(
        oo_info_compiler!("Object", "object", "call", Arity::exact(2)),
        "call",
        "Report the method-call chain for a method.",
        "info object call object methodName",
    ),
    sub(
        oo_info_compiler!("Object", "object", "class", specialized ObjectClass),
        "class",
        "Report the class of an object (or test membership).",
        "info object class object ?className?",
    ),
    sub_since(
        oo_info_compiler!("Object", "object", "creationid", Arity::exact(1)),
        "creationid",
        "Report the object's unique creation id, fixed for its lifetime.",
        "info object creationid object",
        &[SpecSurface::core_in(
            tcl_dialect::model::Family::Tcl,
            &[("9.0", Some("9.1"))],
        )],
        "9.0",
    ),
    sub_since(
        oo_info_compiler!("Object", "object", "creationid", specialized ObjectCreationId),
        "creationid",
        "Report the object's unique creation id, fixed for its lifetime.",
        "info object creationid object",
        &[SpecSurface::core_in(
            tcl_dialect::model::Family::Tcl,
            &[("9.1", None)],
        )],
        "9.0",
    ),
    sub(
        oo_info_compiler!("Object", "object", "definition", Arity::exact(2)),
        "definition",
        "Report how a method was defined.",
        "info object definition object methodName",
    ),
    sub(
        oo_info_compiler!("Object", "object", "filters", Arity::exact(1)),
        "filters",
        "List the filter methods of an object.",
        "info object filters object",
    ),
    sub(
        oo_info_compiler!("Object", "object", "forward", Arity::exact(2)),
        "forward",
        "Report the target of a forwarded method.",
        "info object forward object methodName",
    ),
    sub(
        oo_info_compiler!("Object", "object", "isa", specialized ObjectIsObject),
        "isa",
        "Test whether an object belongs to a category: class, metaclass, mixin, object, or typeof.",
        "info object isa category object ?arg?",
    ),
    methods_sub(
        oo_info_compiler!("Object", "object", "methods", Arity::at_least(1)),
        "List the methods of an object.",
        "info object methods object ?option...?",
    ),
    sub(
        oo_info_compiler!("Object", "object", "methodtype", Arity::exact(2)),
        "methodtype",
        "Report the type of a method.",
        "info object methodtype object methodName",
    ),
    sub(
        oo_info_compiler!("Object", "object", "mixins", Arity::exact(1)),
        "mixins",
        "List the classes mixed into an object.",
        "info object mixins object",
    ),
    sub(
        oo_info_compiler!("Object", "object", "namespace", specialized ObjectNamespace),
        "namespace",
        "Report the private namespace of an object.",
        "info object namespace object",
    ),
    properties_sub(
        oo_info_compiler!("Object", "object", "properties", Arity::at_least(1)),
        "List the declared properties of an object.",
        "info object properties object ?options...?",
    ),
    sub_in_surface(
        sub(
            oo_info_compiler!("Object", "object", "variables", Arity::exact(1)),
            "variables",
            "List the declared instance variables of an object. Tcl 9.0+ accepts an optional -private flag to list private variables instead.",
            "info object variables object",
        ),
        &[SpecSurface::core_in(
            tcl_dialect::model::Family::Tcl,
            &[("8.6", Some("9.0"))],
        )],
    ),
    sub_in_surface(
        sub(
            oo_info_compiler!("Object", "object", "variables", Arity::new(1, 2)),
            "variables",
            "List the declared instance variables of an object. Tcl 9.0+ accepts an optional -private flag to list private variables instead.",
            "info object variables object",
        ),
        SpecSurface::TCL90_PLUS,
    ),
    sub(
        oo_info_compiler!("Object", "object", "vars", Arity::new(1, 2)),
        "vars",
        "List the visible variables in an object's namespace.",
        "info object vars object ?pattern?",
    ),
];

/// Second-level subcommands of `info class` (the CLASS INTROSPECTION
/// operations from the `info` man page). Base set is Tcl 8.6;
/// `definitionnamespace` is 9.0 (TIP 524) and `properties` is 9.0 (TIP 558).
const INFO_CLASS_SUBS: &[SubSubCommand] = &[
    sub(
        oo_info_compiler!("Class", "class", "call", Arity::exact(2)),
        "call",
        "Report the method-call chain for a class method.",
        "info class call class methodName",
    ),
    sub(
        oo_info_compiler!("Class", "class", "constructor", Arity::exact(1)),
        "constructor",
        "Report the definition of a class constructor.",
        "info class constructor class",
    ),
    sub(
        oo_info_compiler!("Class", "class", "definition", Arity::exact(2)),
        "definition",
        "Report how a class method was defined.",
        "info class definition class methodName",
    ),
    sub_since(
        oo_info_compiler!("Class", "class", "definitionnamespace", Arity::new(1, 2)),
        "definitionnamespace",
        "Report the definition namespace used for kind definitions of the class: -class (the default) or -instance.",
        "info class definitionnamespace class ?kind?",
        SpecSurface::TCL90_PLUS,
        "9.0",
    ),
    sub(
        oo_info_compiler!("Class", "class", "destructor", Arity::exact(1)),
        "destructor",
        "Report the definition of a class destructor.",
        "info class destructor class",
    ),
    sub(
        oo_info_compiler!("Class", "class", "filters", Arity::exact(1)),
        "filters",
        "List the filter methods of a class.",
        "info class filters class",
    ),
    sub(
        oo_info_compiler!("Class", "class", "forward", Arity::exact(2)),
        "forward",
        "Report the target of a forwarded class method.",
        "info class forward class methodName",
    ),
    sub(
        oo_info_compiler!("Class", "class", "instances", Arity::new(1, 2)),
        "instances",
        "List the instances of a class.",
        "info class instances class ?pattern?",
    ),
    methods_sub(
        oo_info_compiler!("Class", "class", "methods", Arity::at_least(1)),
        "List the methods of a class.",
        "info class methods class ?options...?",
    ),
    sub(
        oo_info_compiler!("Class", "class", "methodtype", Arity::exact(2)),
        "methodtype",
        "Report the type of a class method.",
        "info class methodtype class methodName",
    ),
    sub(
        oo_info_compiler!("Class", "class", "mixins", Arity::exact(1)),
        "mixins",
        "List the classes mixed into a class.",
        "info class mixins class",
    ),
    properties_sub(
        oo_info_compiler!("Class", "class", "properties", Arity::at_least(1)),
        "List the declared properties of a class.",
        "info class properties class ?options...?",
    ),
    sub(
        oo_info_compiler!("Class", "class", "subclasses", Arity::new(1, 2)),
        "subclasses",
        "List the subclasses of a class.",
        "info class subclasses class ?pattern?",
    ),
    sub(
        oo_info_compiler!("Class", "class", "superclasses", Arity::exact(1)),
        "superclasses",
        "List the superclasses of a class.",
        "info class superclasses class",
    ),
    sub_in_surface(
        sub(
            oo_info_compiler!("Class", "class", "variables", Arity::exact(1)),
            "variables",
            "List the declared instance variables of a class. Tcl 9.0+ accepts an optional -private flag to list private variables instead.",
            "info class variables class",
        ),
        &[SpecSurface::core_in(
            tcl_dialect::model::Family::Tcl,
            &[("8.6", Some("9.0"))],
        )],
    ),
    sub_in_surface(
        sub(
            oo_info_compiler!("Class", "class", "variables", Arity::new(1, 2)),
            "variables",
            "List the declared instance variables of a class. Tcl 9.0+ accepts an optional -private flag to list private variables instead.",
            "info class variables class",
        ),
        SpecSurface::TCL90_PLUS,
    ),
];

/// Which `TclOO` second-level `info` ensemble is being dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoOoEnsembleKind {
    /// `info object subcommand ...`
    Object,
    /// `info class subcommand ...`
    Class,
}

/// One option accepted by Tcl 9.0's `info object properties` and
/// `info class properties` operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoOoPropertiesOption {
    /// Include inherited properties.
    All,
    /// Select readable properties.
    Readable,
    /// Select writable properties.
    Writable,
}

static INFO_PROPERTIES_NAMES: [&str; 3] = [
    INFO_PROPERTIES_OPTIONS[0].name,
    INFO_PROPERTIES_OPTIONS[1].name,
    INFO_PROPERTIES_OPTIONS[2].name,
];

fn info_properties_option_at(index: usize) -> InfoOoPropertiesOption {
    match index {
        0 => InfoOoPropertiesOption::All,
        1 => InfoOoPropertiesOption::Readable,
        2 => InfoOoPropertiesOption::Writable,
        _ => unreachable!("the registry declares exactly three info properties options"),
    }
}

/// Resolve an executable option through its original object and the static
/// Registry table. The adapter independently selects native index/cache and
/// failure-publication protocols; the returned option supplies no OO target.
///
/// # Errors
/// An original getter/index capability refusal or the native option failure.
pub fn resolve_info_oo_properties_option_original<O: tcl_syntax::value::ValueOps>(
    ops: &mut O,
    original: &O::Value,
) -> Result<InfoOoPropertiesOption, tcl_cmd_core::error::CmdError> {
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", &INFO_PROPERTIES_NAMES);
    Ok(info_properties_option_at(
        table.index_of_original(ops, original)?,
    ))
}

/// Resolve an `info ... properties` option through the option rows attached
/// to the registry's `properties` operation.
///
/// # Errors
/// Tcl's byte-exact bad/ambiguous option message.
pub fn resolve_info_oo_properties_option(word: &[u8]) -> Result<InfoOoPropertiesOption, Vec<u8>> {
    let table = tcl_cmd_core::prefix::OptionTable::abbreviating("option", &INFO_PROPERTIES_NAMES);
    Ok(info_properties_option_at(table.index_of(word)?))
}

impl InfoOoEnsembleKind {
    const fn parent_name(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Class => "class",
        }
    }
}

/// The registry-derived, release-filtered operation table for one `TclOO`
/// `info` ensemble.
///
/// The registry owns which operations exist; `tcl-cmd-core::ensemble` owns
/// exact/unique-prefix resolution and the ensemble-specific `, or` rendering.
/// Runtime consumers therefore receive one ready-to-dispatch table and never
/// copy either the operation names or the miss-message list.
#[derive(Debug, Clone)]
pub struct InfoOoSubcommands {
    names: Vec<&'static str>,
}

impl InfoOoSubcommands {
    /// Canonical operation names in Tcl's listing order.
    #[must_use]
    pub fn names(&self) -> &[&'static str] {
        &self.names
    }

    /// Resolve an exact name or unique prefix, returning the canonical name.
    ///
    /// # Errors
    /// The Tcl ensemble miss message, including the release-filtered choices.
    pub fn resolve(&self, word: &[u8]) -> Result<&'static str, Vec<u8>> {
        tcl_cmd_core::ensemble::resolve_subcommand(&self.names, word, true)
            .map(|index| self.names[index])
            .ok_or_else(|| {
                tcl_cmd_core::ensemble::unknown_subcommand_message(&self.names, word, true, b"")
            })
    }
}

/// Build the authoritative operation table for `info object` or `info class`
/// at `version`.
///
/// This projects the same [`SubSubCommand`] rows used by diagnostics,
/// completion, and hover. In particular, Tcl 9.0's `creationid`,
/// `definitionnamespace`, and `properties` rows disappear on an 8.6 target
/// before prefix uniqueness or error rendering is decided.
#[must_use]
pub fn info_oo_subcommands(kind: InfoOoEnsembleKind, version: TclVersion) -> InfoOoSubcommands {
    let dialect =
        Some(crate::model::static_document_context_for(version.dialect_name()).authoring_query());
    let info = spec();
    let parent = info
        .subcommands
        .iter()
        .find(|subcommand| subcommand.name == kind.parent_name())
        .expect("the info spec declares both TclOO ensembles");
    InfoOoSubcommands {
        names: parent
            .available_sub_subcommands(dialect, None)
            .into_iter()
            .map(|subcommand| subcommand.name)
            .collect(),
    }
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "version",
        surface: Some(tcl_dialect::surface![SpecSurface::core_in(
            tcl_dialect::model::Family::Jim,
            &[("0.84", None)]
        )]),
        arity: Arity::exact(0),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        detail: "Returns the major and minor version of the Jim interpreter.",
        synopsis: "info version",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "stacktrace",
        surface: Some(tcl_dialect::surface![SpecSurface::core_in(
            tcl_dialect::model::Family::Jim,
            &[("0.84", None)]
        )]),
        arity: Arity::exact(0),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "args",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[0],
            Arity::exact(1),
        )),
        // Reflects another proc's parameter names, looked up by the proc's
        // spelled name — observable identity for both symbol kinds.
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(1),
        detail: "Returns the names of the parameters to the procedure named procname.",
        synopsis: "info args procname",
        pure: true,
        return_type: Some(TclType::List),
        // `procname` names a proc introspected (not called), so it is a
        // command reference navigation follows.
        arg_roles: &[(0, ArgRole::CommandName)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "body",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[1],
            Arity::exact(1),
        )),
        // Reflects a proc's source (including its local-variable spellings)
        // by the proc's spelled name.
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::REFLECTS_COMMAND_NAMES),
        arity: Arity::exact(1),
        detail: "Returns the body of the procedure named procname.",
        synopsis: "info body procname",
        pure: true,
        return_type: Some(TclType::String),
        arg_roles: &[(0, ArgRole::CommandName)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "class",
        native_compilation: Some(oo_info_ensemble_compiler!("class", "Class")),
        arity: Arity::at_least(2),
        detail: "Returns information about the class.",
        synopsis: "info class subcommand class ?arg ...?",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL86_PLUS),
        // `info class` is itself an ensemble: the word after `class` selects a
        // CLASS INTROSPECTION operation.
        sub_subcommands: INFO_CLASS_SUBS,
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cmdcount",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[2],
            Arity::exact(0),
        )),
        arity: Arity::exact(0),
        detail: "Returns the total number of commands evaluated in this interpreter.",
        synopsis: "info cmdcount",
        pure: true,
        return_type: Some(TclType::Int),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cmdtype",
        native_compilation: Some(named_c9_member_compilation(
            &INFO_NAMED_LOOKUPS[19],
            Arity::exact(1),
        )),
        // Introspects a command by its spelled name.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::exact(1),
        detail: "Returns the type of the command named commandName: alias, coroutine, ensemble, import, native, object, privateObject, proc, interp, or zlibStream.",
        synopsis: "info cmdtype commandName",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL90_PLUS),
        // `commandName` is an introspected command (a command reference),
        // matching `info args`/`info body`'s treatment of `procname`.
        arg_roles: &[(0, ArgRole::CommandName)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "commands",
        // Tcl's command-name enumeration performs no Tcl callback. The
        // independently mutable C ensemble worker remains part of this proof.
        successful_handler: Some(
            crate::native_compilation::SuccessfulHandlerSpec::EnsembleLeaf {
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::info",
                    member: "commands",
                    slot: "::tcl::info::commands",
                    command: "info",
                    prepended: &["commands"],
                },
                implementation_from: tcl_dialect::TclVersion::V8_5,
                direct_provider: None,
            },
        ),
        completion: Some(crate::completion::CompletionDescriptor::exact(&[
            crate::completion::CompletionCode::Ok,
            crate::completion::CompletionCode::Error,
        ])),
        semantic_operation: Some(SemanticOperationId::Invoke),
        inline_codegen_hook: Some(InlineCodegenHookId::InfoCommandsResolve),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::InfoCommands,
            operation: crate::SemanticOperationId::Intrinsic(
                crate::IntrinsicId::InfoCommandsResolve,
            ),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // Enumerates command names — reflection over the command table.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::new(0, 1),
        detail: "Returns the names of all commands visible in the current namespace.",
        synopsis: "info commands ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        // The optional argument is a glob *pattern*, but the common exact
        // spelling (`info commands foo`) probes one specific command — a
        // navigable reference that asserts nothing about existence (an
        // absent name returns an empty list).  The analyser's probe
        // recorder abstains on any word with glob metacharacters, so a
        // real pattern contributes no reference.
        arg_roles: &[(0, ArgRole::CommandNameProbe)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "complete",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[3],
            Arity::exact(1),
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if command is a complete command, and 0 otherwise.",
        synopsis: "info complete command",
        pure: true,
        return_type: Some(TclType::Boolean),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "constant",
        native_compilation: Some(named_c9_member_compilation(
            &INFO_NAMED_LOOKUPS[20],
            Arity::exact(1),
        )),
        traits: Traits::INTROSPECTS_BY_NAME,
        arity: Arity::exact(1),
        detail: "Returns 1 if varName is a constant variable and 0 otherwise.",
        synopsis: "info constant varName",
        pure: true,
        return_type: Some(TclType::Boolean),
        surface: Some(SpecSurface::TCL90_PLUS),
        arg_roles: &[(0, ArgRole::VarRead)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "consts",
        native_compilation: Some(named_c9_member_compilation(
            &INFO_NAMED_LOOKUPS[21],
            Arity::new(0, 1),
        )),
        arity: Arity::new(0, 1),
        detail: "Returns the list of constant variables in the current scope.",
        synopsis: "info consts ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        surface: Some(SpecSurface::TCL90_PLUS),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "coroutine",
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::InfoCoroutine,
            operation: SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        traits: Traits::CURRENT_FRAME_INTROSPECTION,
        arity: Arity::exact(0),
        detail: "Returns the name of the current coroutine, or the empty string if there is no current coroutine.",
        synopsis: "info coroutine",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL86_PLUS),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "default",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[4],
            Arity::exact(3),
        )),
        // Reflects a named proc's parameter defaults by the proc's spelled
        // name — observable identity for both symbol kinds.
        // `varname` is written either way: the default, or `""` without one.
        traits: Traits::INTROSPECTS_BY_NAME
            .union(Traits::REFLECTS_COMMAND_NAMES)
            .union(Traits::UNCONDITIONAL_VARIABLE_WRITE),
        arity: Arity::exact(3),
        detail: "If the parameter has a default value, stores that value in varname and returns 1; otherwise returns 0.",
        synopsis: "info default procname parameter varname",
        return_type: Some(TclType::Boolean),
        // `procname` is an introspected proc (a command reference); `varname`
        // is written.
        arg_roles: &[(0, ArgRole::CommandName), (2, ArgRole::VarWrite)],
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::scope_alias::INFO_DEFAULT,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "errorstack",
        traits: Traits::CURRENT_FRAME_INTROSPECTION,
        arity: Arity::new(0, 1),
        detail: "Returns an even-sized list of CALL/UP/INNER tokens and parameters describing the active command at each level from the call stack of the last error. Also available as the -errorstack entry of a 3-argument catch's options dictionary.",
        synopsis: "info errorstack ?interp?",
        pure: true,
        return_type: Some(TclType::List),
        surface: Some(SpecSurface::TCL86_PLUS),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        native_result: Some(
            crate::native_result::NativeResultContract::VariableExistence { variable_at: 0 },
        ),
        world_effects: Some(crate::WorldEffectDescriptor::VARIABLE_READ),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::InfoExists,
            operation: SemanticOperationId::Intrinsic(IntrinsicId::InfoExists),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        semantic_operation: Some(SemanticOperationId::Intrinsic(IntrinsicId::InfoExists)),
        traits: Traits::INTROSPECTS_BY_NAME,
        arity: Arity::exact(1),
        detail: "Returns 1 if a variable named varName is visible and has been defined, and 0 otherwise.",
        synopsis: "info exists varName",
        pure: true,
        return_type: Some(TclType::Boolean),
        arg_roles: &[(0, ArgRole::VarRead)],
        inline_codegen_hook: Some(InlineCodegenHookId::InfoExists),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "frame",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[5],
            Arity::new(0, 1),
        )),
        // Reflects the active command words (including proc names) of any
        // stack frame.
        traits: Traits::REFLECTS_COMMAND_NAMES.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the depth of the call to info frame itself when depth is omitted; otherwise returns a dictionary describing the active command at that depth (keys: type — one of source/proc/eval/precompiled — plus line, file, cmd, proc, lambda, level as applicable). Reports every stack frame, including eval/uplevel/source frames that info level does not see.",
        synopsis: "info frame ?depth?",
        pure: true,
        return_type: Some(TclType::Dict),
        // Introduced in Tcl 8.5 (TIP 280) — not available in 8.4.
        surface: Some(SpecSurface::TCL85_PLUS),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "functions",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[6],
            Arity::new(0, 1),
        )),
        arity: Arity::new(0, 1),
        detail: "Returns a list of all the math functions currently defined.",
        synopsis: "info functions ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "globals",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[7],
            Arity::new(0, 1),
        )),
        // Enumerates global variable names — the global-scope counterpart
        // of `info vars` / `info locals`.
        traits: Traits::INTROSPECTS_BY_NAME,
        arity: Arity::new(0, 1),
        detail: "Returns a list of all the names of currently-defined global variables.",
        synopsis: "info globals ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hostname",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[8],
            Arity::exact(0),
        )),
        arity: Arity::exact(0),
        detail: "Returns the name of the current host.",
        synopsis: "info hostname",
        pure: true,
        return_type: Some(TclType::String),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "level",
        inline_codegen_hook: Some(InlineCodegenHookId::InfoLevel),
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::InfoLevel,
            operation: crate::SemanticOperationId::Intrinsic(crate::IntrinsicId::InfoLevel),
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // `info level N` reflects the full command (name + arguments) at
        // that level, so proc names are observable data.
        traits: Traits::REFLECTS_COMMAND_NAMES.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the current stack level (0 at top level) when level is omitted; otherwise returns the complete command (name and arguments, as a list) active at that level. A positive level is absolute (1 = outermost active procedure); zero or a negative level is relative to the current one.",
        synopsis: "info level ?level?",
        pure: true,
        return_type: Some(TclType::Int),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "library",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[9],
            Arity::exact(0),
        )),
        arity: Arity::exact(0),
        detail: "Returns the name of the library directory in which standard Tcl scripts are stored.",
        synopsis: "info library",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "loaded",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[10],
            Arity::new(0, 1),
        )),
        arity: Arity::new(0, 2),
        detail: "Returns the name of each file loaded in interp by the load command, paired with the package prefix it was loaded under. From Tcl 9.0, an optional trailing prefix argument restricts the results to that prefix; Tcl 8.4-8.6 accept only the interp argument.",
        synopsis: "info loaded ?interp? ?prefix?",
        pure: true,
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "locals",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[11],
            Arity::new(0, 1),
        )),
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the name of each local variable matching pattern.",
        synopsis: "info locals ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nameofexecutable",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[12],
            Arity::exact(0),
        )),
        arity: Arity::exact(0),
        detail: "Returns the absolute pathname of the program for the current interpreter.",
        synopsis: "info nameofexecutable",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "object",
        native_compilation: Some(oo_info_ensemble_compiler!("object", "Object")),
        arity: Arity::at_least(2),
        detail: "Returns information about the object.",
        synopsis: "info object subcommand object ?arg ...?",
        pure: true,
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL86_PLUS),
        // `info object` is itself an ensemble: the word after `object` selects
        // an OBJECT INTROSPECTION operation.
        sub_subcommands: INFO_OBJECT_SUBS,
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "patchlevel",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[13],
            Arity::exact(0),
        )),
        arity: Arity::exact(0),
        detail: "Returns the live Tcl global tcl_patchLevel, or the independently selected Jim build/core version report.",
        synopsis: "info patchlevel",
        pure: true,
        return_type: Some(TclType::String),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "procs",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[14],
            Arity::new(0, 1),
        )),
        // Enumerates procedure names — reflection over the command table.
        traits: Traits::REFLECTS_COMMAND_NAMES,
        arity: Arity::new(0, 1),
        detail: "Returns the names of all visible procedures.",
        synopsis: "info procs ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        // Same probe semantics as `info commands` above: an exact spelling
        // (`info procs helper`) is a navigable reference to an existing
        // proc, a glob pattern contributes none.
        arg_roles: &[(0, ArgRole::CommandNameProbe)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "script",
        source_path_operation: Some(SourcePathOperation::ScriptPath),
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[15],
            Arity::new(0, 1),
        )),
        arity: Arity::new(0, 1),
        detail: "Returns the pathname of the innermost script currently being evaluated, or the empty string if none. With filename, overrides the return value of this command for the remainder of the active invocation — useful in virtual filesystem applications.",
        synopsis: "info script ?filename?",
        pure: true,
        mutator: true,
        return_type: Some(TclType::String),
        returns_path: true,
        // Read-only in its common bare form; the optional `filename`
        // argument writes interpreter-level state (the active script-path
        // override), unchanged across 8.4-9.1. `mutator: true` is required
        // alongside `pure: true` here (mirrors the getter/setter pattern
        // used e.g. by `DNS::header cd ?value?`) — without it,
        // `tcl_compiler::side_effects::classify_side_effects`'s
        // `sub.pure && !sub.mutator` short-circuit returns an empty,
        // effect-free classification for *every* call before the
        // `side_effects` below is ever consulted, silently making
        // `info script $path` look pure/dead-code-eliminable even though
        // it exists only for its side effect.
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sharedlibextension",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[16],
            Arity::exact(0),
        )),
        arity: Arity::exact(0),
        detail: "Returns the extension used on this platform for shared libraries.",
        synopsis: "info sharedlibextension",
        pure: true,
        return_type: Some(TclType::String),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tclversion",
        surface: Some(tcl_dialect::surface![SpecSurface::core_in(
            tcl_dialect::model::Family::Tcl,
            &[("8.4", None)]
        )]),
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[17],
            Arity::exact(0),
        )),
        arity: Arity::exact(0),
        detail: "Returns the major and minor version of the Tcl library.",
        synopsis: "info tclversion",
        pure: true,
        return_type: Some(TclType::String),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::ROUTE_UNAUTHORED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "vars",
        native_compilation: Some(named_member_compilation(
            &INFO_NAMED_LOOKUPS[18],
            Arity::new(0, 1),
        )),
        traits: Traits::INTROSPECTS_BY_NAME.union(Traits::CURRENT_FRAME_INTROSPECTION),
        arity: Arity::new(0, 1),
        detail: "Returns the names of all visible variables.",
        synopsis: "info vars ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::STATE_DECIDED),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `info`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "info",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::HookFrom(
                tcl_dialect::TclVersion::V8_5,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("info"),
        surface: Some(SpecSurface::ALL_TCL_AND_IRULES),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::InterpState,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Information about the state of the Tcl interpreter",
            synopsis: &["info option ?arg arg ...?"],
            snippet: "A large introspection ensemble; option names may be abbreviated to any unambiguous prefix. Covers procedure metadata (args, body, default, procs), variable metadata (exists, vars, globals, locals), and command/interpreter/environment metadata (commands, cmdcount, complete, level, loaded, hostname, library, nameofexecutable, patchlevel, script, sharedlibextension, tclversion). Tcl 8.5 adds stack-frame introspection (frame). Tcl 8.6 adds coroutine, errorstack, and the TclOO info class/info object introspection subcommands. Tcl 9.0 adds cmdtype, constant, consts, an optional prefix argument to loaded, and extends the TclOO introspection with creationid, definitionnamespace, and properties. Tcl 9.1 makes no further changes to info.",
            source: "Tcl info(n)",
            examples: "if {[info exists myVar]} {\n    puts $myVar\n}\nforeach p [info procs helper*] {\n    puts \"found $p\"\n}\nputs \"Tcl [info tclversion] (patchlevel [info patchlevel])\"",
            return_value: "Varies by subcommand: most return a string, list, or boolean. info level and info frame return an integer when called with no argument, and a list or dictionary respectively when given one. See each subcommand's own description for its exact shape.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJECT_86: &[&str] = &[
        "call",
        "class",
        "definition",
        "filters",
        "forward",
        "isa",
        "methods",
        "methodtype",
        "mixins",
        "namespace",
        "variables",
        "vars",
    ];
    const OBJECT_90: &[&str] = &[
        "call",
        "class",
        "creationid",
        "definition",
        "filters",
        "forward",
        "isa",
        "methods",
        "methodtype",
        "mixins",
        "namespace",
        "properties",
        "variables",
        "vars",
    ];
    const CLASS_86: &[&str] = &[
        "call",
        "constructor",
        "definition",
        "destructor",
        "filters",
        "forward",
        "instances",
        "methods",
        "methodtype",
        "mixins",
        "subclasses",
        "superclasses",
        "variables",
    ];
    const CLASS_90: &[&str] = &[
        "call",
        "constructor",
        "definition",
        "definitionnamespace",
        "destructor",
        "filters",
        "forward",
        "instances",
        "methods",
        "methodtype",
        "mixins",
        "properties",
        "subclasses",
        "superclasses",
        "variables",
    ];

    #[test]
    fn original_c9_basic_info_members_keep_their_selected_source_hooks() {
        // Source proof: naming.info.original-named-member-source-hooks
        // docs/design/analysis/name-resolution-proofs/info-original-named-member-source-hooks.md
        // Source rows prove declarations; actual installed tokens remain separate.
        let sources = [
            include_str!(
                "../../../tests/data/native_info_inventory_original/sources/8.4.20/tclCmdIL.c"
            ),
            include_str!(
                "../../../tests/data/native_info_inventory_original/sources/8.5.19/tclCmdIL.c"
            ),
            include_str!(
                "../../../tests/data/native_info_inventory_original/sources/8.6.18/tclCmdIL.c"
            ),
            include_str!(
                "../../../tests/data/native_info_inventory_original/sources/9.0.4/tclCmdIL.c"
            ),
            include_str!(
                "../../../tests/data/native_info_inventory_original/sources/9.1.0/tclCmdIL.c"
            ),
        ];
        let registry = crate::default_registry();
        for (version, source) in TclVersion::ALL.into_iter().zip(sources) {
            for (name, hook) in [
                ("cmdtype", "TclCompileBasic1ArgCmd"),
                ("constant", "TclCompileBasic1ArgCmd"),
                ("consts", "TclCompileBasic0Or1ArgCmd"),
            ] {
                let row = source
                    .lines()
                    .find(|line| line.trim_start().starts_with(&format!("{{\"{name}\",")));
                let descriptor = registry.native_compilation_for_registration(
                    &format!("::tcl::info::{name}"),
                    crate::InvocationDialect::for_version(version),
                );
                if version < TclVersion::V9_0 {
                    assert!(row.is_none(), "{version:?}/{name}");
                    assert!(descriptor.is_none(), "{version:?}/{name}");
                    continue;
                }
                assert_eq!(row.unwrap().split(',').nth(2).unwrap().trim(), hook);
                let crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                    lookup,
                    implementation_from,
                    hook_from,
                    ..
                } = descriptor.unwrap().grammar
                else {
                    panic!("{version:?}/{name}: original Basic named compiler");
                };
                assert_eq!(lookup.member, name);
                assert_eq!(implementation_from, TclVersion::V9_0);
                assert_eq!(hook_from, TclVersion::V9_0);
            }
        }
    }

    #[test]
    fn tcloo_info_tables_are_release_filtered_from_registry_rows() {
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Object, TclVersion::V8_6).names(),
            OBJECT_86
        );
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Object, TclVersion::V9_0).names(),
            OBJECT_90
        );
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V8_6).names(),
            CLASS_86
        );
        assert_eq!(
            info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V9_0).names(),
            CLASS_90
        );
    }

    #[test]
    fn tcloo_info_resolution_and_choices_match_tcl_ensembles() {
        let object = info_oo_subcommands(InfoOoEnsembleKind::Object, TclVersion::V9_0);
        assert_eq!(object.resolve(b"cl"), Ok("class"));
        assert_eq!(
            object.resolve(b"bogus").unwrap_err(),
            b"unknown or ambiguous subcommand \"bogus\": must be call, class, creationid, definition, filters, forward, isa, methods, methodtype, mixins, namespace, properties, variables, or vars"
        );

        let class_86 = info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V8_6);
        let class_90 = info_oo_subcommands(InfoOoEnsembleKind::Class, TclVersion::V9_0);
        assert_eq!(class_86.resolve(b"def"), Ok("definition"));
        assert_eq!(
            class_90.resolve(b"def").unwrap_err(),
            b"unknown or ambiguous subcommand \"def\": must be call, constructor, definition, definitionnamespace, destructor, filters, forward, instances, methods, methodtype, mixins, properties, subclasses, superclasses, or variables"
        );
    }

    #[test]
    fn tcloo_info_properties_options_use_registry_rows_and_shared_prefixes() {
        assert_eq!(
            resolve_info_oo_properties_option(b"-a"),
            Ok(InfoOoPropertiesOption::All)
        );
        assert_eq!(
            resolve_info_oo_properties_option(b"-r"),
            Ok(InfoOoPropertiesOption::Readable)
        );
        assert_eq!(
            resolve_info_oo_properties_option(b"-").unwrap_err(),
            b"ambiguous option \"-\": must be -all, -readable, or -writable"
        );
        assert_eq!(
            resolve_info_oo_properties_option(b"-bogus").unwrap_err(),
            b"bad option \"-bogus\": must be -all, -readable, or -writable"
        );

        for subs in [INFO_OBJECT_SUBS, INFO_CLASS_SUBS] {
            let properties = subs
                .iter()
                .find(|subcommand| subcommand.name == "properties")
                .expect("properties operation");
            assert_eq!(
                properties
                    .options
                    .expect("registry properties options")
                    .iter()
                    .map(|option| option.name)
                    .collect::<Vec<_>>(),
                ["-all", "-readable", "-writable"]
            );
        }
    }
}
