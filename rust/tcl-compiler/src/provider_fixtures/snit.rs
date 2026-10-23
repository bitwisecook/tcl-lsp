// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tcllib 2.0 Snit 2.3.4 fixture-driver installation receipt.
//!
//! The roster was inspected in Tcl 8.5, 8.6, 9.0 and 9.1. The generated
//! validation handles are retained as installed commands with unknown hooks;
//! only the inspected ordinary procedures receive absent compiler stamps.

use crate::command_binding::{TrustedCommandLookup, TrustedPackageLoader};
use tcl_runtime_api::native_compilation::NativeCompilerHookPresence;

const COMMANDS: &[&str] = &[
    "::snit::Capitalize",
    "::snit::CheckArgs",
    "::snit::Comp.CheckMethodName",
    "::snit::Comp.Compile",
    "::snit::Comp.Define",
    "::snit::Comp.DefineComponent",
    "::snit::Comp.DefineTypecomponent",
    "::snit::Comp.DelegatedMethod",
    "::snit::Comp.DelegatedOption",
    "::snit::Comp.DelegatedTypemethod",
    "::snit::Comp.Init",
    "::snit::Comp.OptionNameIsValid",
    "::snit::Comp.SaveOptionInfo",
    "::snit::Comp.statement.component",
    "::snit::Comp.statement.constructor",
    "::snit::Comp.statement.delegate",
    "::snit::Comp.statement.destructor",
    "::snit::Comp.statement.expose",
    "::snit::Comp.statement.hulltype",
    "::snit::Comp.statement.method",
    "::snit::Comp.statement.oncget",
    "::snit::Comp.statement.onconfigure",
    "::snit::Comp.statement.option",
    "::snit::Comp.statement.pragma",
    "::snit::Comp.statement.proc",
    "::snit::Comp.statement.typecomponent",
    "::snit::Comp.statement.typeconstructor",
    "::snit::Comp.statement.typemethod",
    "::snit::Comp.statement.typevariable",
    "::snit::Comp.statement.variable",
    "::snit::Comp.statement.widgetclass",
    "::snit::Expand",
    "::snit::Mappend",
    "::snit::RT.CacheCgetCommand",
    "::snit::RT.CacheConfigureCommand",
    "::snit::RT.CallInstance",
    "::snit::RT.ClearInstanceCaches",
    "::snit::RT.Component",
    "::snit::RT.ComponentTrace",
    "::snit::RT.ConstructInstance",
    "::snit::RT.DestroyObject",
    "::snit::RT.GetOptionDbSpec",
    "::snit::RT.InstanceTrace",
    "::snit::RT.MakeInstanceCommand",
    "::snit::RT.OptionDbGet",
    "::snit::RT.RemoveInstanceTrace",
    "::snit::RT.TypecomponentTrace",
    "::snit::RT.UniqueInstanceNamespace",
    "::snit::RT.UniqueName",
    "::snit::RT.UnknownMethod",
    "::snit::RT.UnknownTypemethod",
    "::snit::RT.body",
    "::snit::RT.codename",
    "::snit::RT.from",
    "::snit::RT.install",
    "::snit::RT.installhull",
    "::snit::RT.method.cget",
    "::snit::RT.method.configure",
    "::snit::RT.method.configurelist",
    "::snit::RT.method.destroy",
    "::snit::RT.method.info",
    "::snit::RT.method.info.args",
    "::snit::RT.method.info.body",
    "::snit::RT.method.info.default",
    "::snit::RT.method.info.methods",
    "::snit::RT.method.info.options",
    "::snit::RT.method.info.type",
    "::snit::RT.method.info.typemethods",
    "::snit::RT.method.info.typevars",
    "::snit::RT.method.info.vars",
    "::snit::RT.mymethod",
    "::snit::RT.myproc",
    "::snit::RT.mytypemethod",
    "::snit::RT.mytypevar",
    "::snit::RT.myvar",
    "::snit::RT.type.typemethod.create",
    "::snit::RT.typemethod.destroy",
    "::snit::RT.typemethod.info",
    "::snit::RT.typemethod.info.args",
    "::snit::RT.typemethod.info.body",
    "::snit::RT.typemethod.info.default",
    "::snit::RT.typemethod.info.instances",
    "::snit::RT.typemethod.info.typemethods",
    "::snit::RT.typemethod.info.typevars",
    "::snit::RT.variable",
    "::snit::RT.widget.typemethod.create",
    "::snit::boolean",
    "::snit::compile",
    "::snit::double",
    "::snit::enum",
    "::snit::fpixels",
    "::snit::integer",
    "::snit::listtype",
    "::snit::macro",
    "::snit::method",
    "::snit::pixels",
    "::snit::stringtype",
    "::snit::type",
    "::snit::typemethod",
    "::snit::widget",
    "::snit::widgetadaptor",
    "::snit::window",
];

const GENERATED_HANDLES: &[&str] = &[
    "::snit::stringtype",
    "::snit::window",
    "::snit::integer",
    "::snit::pixels",
    "::snit::fpixels",
    "::snit::boolean",
    "::snit::double",
    "::snit::enum",
    "::snit::listtype",
];

/// Supply the attested recipe and dependencies of the native fixture loader.
pub(super) fn attest(loader: &mut TrustedPackageLoader) {
    loader.command_surface = COMMANDS.iter().map(|name| (*name).to_owned()).collect();
    loader.compiler_hooks = COMMANDS
        .iter()
        .filter(|name| !GENERATED_HANDLES.contains(name))
        .map(|name| ((*name).to_owned(), NativeCompilerHookPresence::Absent))
        .collect();
    loader.definition_dispatchers.insert((
        "::snit::type".to_owned(),
        tcl_registry::definer::DefinitionDispatcher::SnitType,
    ));
    loader.installed_lookup_dependencies = COMMANDS
        .iter()
        .map(|name| TrustedCommandLookup {
            head: (*name).to_owned(),
            namespace: "::snit".to_owned(),
            registry_identity: (*name).to_owned(),
            required: true,
        })
        .collect();
    loader.lookup_dependencies = [
        "append",
        "array",
        "break",
        "catch",
        "close",
        "concat",
        "continue",
        "dict",
        "error",
        "eval",
        "expr",
        "for",
        "foreach",
        "format",
        "global",
        "if",
        "incr",
        "info",
        "interp",
        "join",
        "lappend",
        "lindex",
        "linsert",
        "list",
        "llength",
        "lrange",
        "lreplace",
        "lsearch",
        "lset",
        "lsort",
        "namespace",
        "package",
        "proc",
        "puts",
        "regexp",
        "regsub",
        "rename",
        "return",
        "set",
        "split",
        "string",
        "switch",
        "trace",
        "unset",
        "uplevel",
        "upvar",
        "variable",
        "while",
    ]
    .into_iter()
    .map(|name| TrustedCommandLookup {
        head: name.to_owned(),
        namespace: "::snit".to_owned(),
        registry_identity: format!("::{name}"),
        required: true,
    })
    .collect();
    loader.state_dependency_namespaces = vec!["::snit".to_owned()];
}
