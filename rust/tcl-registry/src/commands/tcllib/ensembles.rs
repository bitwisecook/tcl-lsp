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

//! tcllib ensemble packages — `generator`, `debug`, `hook` — plus the flat
//! `websocket` package.
//!
//! `generator`, `debug`, and `hook` are ensemble commands: a single command
//! word dispatches on a sub-command (`generator define`, `debug on`,
//! `hook bind`).  Their sub-command tables (names, arity bounds, synopses,
//! and summaries) are derived from the upstream tcllib 2.0 manual pages.
//! Requires Tcl 8.5+.

use crate::prelude::*;

fn deferred_first_arg(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (!args.is_empty())
        .then_some((0, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}

/// The `generator` ensemble's sub-commands.
const GENERATOR_SUBS: &[SubCommand] = &[
    SubCommand {
        name: "define",
        arity: Arity::exact(3),
        detail: "Creates a new generator procedure.",
        synopsis: "generator define name params body",
        // `generator define name params body` is proc-shaped: mark the
        // parameter list and body so the generic registry-driven body
        // recursion (semantic tokens, folding, diagnostics) descends into the
        // generator body, as it does for `proc` / `lambda`.
        arg_roles: &[(1, ArgRole::ParamList), (2, ArgRole::Body)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "yield",
        arity: Arity::at_least(1),
        detail: "Used in the definition of a generator, this command returns the next set of values to the consumer.",
        synopsis: "generator yield arg",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foreach",
        // `generator foreach varList generator ?varList generator ...? body`:
        // the single-generator form is three words (one pair plus the body).
        arity: Arity::at_least(3),
        detail: "Loops through one or more generators, assigning the next values to variables and then executing the loop body.",
        synopsis: "generator foreach varList generator ?varList generator ...? body",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "next",
        arity: Arity::at_least(1),
        detail: "Manually retrieves the next values from a generator.",
        synopsis: "generator next generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Returns 1 if the generator (still) exists, or 0 otherwise.",
        synopsis: "generator exists generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "Returns a list of all currently existing generator commands.",
        synopsis: "generator names",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "destroy",
        arity: Arity::at_least(0),
        detail: "Destroys one or more generators, freeing any associated resources.",
        synopsis: "generator destroy",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "finally",
        arity: Arity::at_least(1),
        detail: "Used in the definition of a generator procedure, this command arranges for a resource to be cleaned up whenever the",
        synopsis: "generator finally cmd",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "from",
        arity: Arity::exact(2),
        detail: "Creates a generator from a data structure.",
        synopsis: "generator from format value",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "to",
        arity: Arity::exact(2),
        detail: "Converts a generator into a data structure.",
        synopsis: "generator to format generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "map",
        arity: Arity::exact(2),
        detail: "Apply a function to every element of a generator, returning a new generator of the results.",
        synopsis: "generator map function generator",
        // `function` (index 0) is applied per element.  The generator may yield
        // multiple values per step, so the appended count is not statically
        // fixed ⇒ Unknown (a reference, not arity-checked).
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "filter",
        arity: Arity::exact(2),
        detail: "Another classic functional programming gem.",
        synopsis: "generator filter predicate generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "reduce",
        arity: Arity::exact(3),
        detail: "This is the classic left-fold operation.",
        synopsis: "generator reduce function zero generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foldl",
        arity: Arity::exact(3),
        detail: "This is an alias for the reduce command.",
        synopsis: "generator foldl function zero generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foldr",
        arity: Arity::exact(3),
        detail: "This is the right-associative version of reduce.",
        synopsis: "generator foldr function zero generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "all",
        arity: Arity::exact(2),
        detail: "Returns true if all elements of the generator satisfy the given predicate.",
        synopsis: "generator all predicate generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "and",
        arity: Arity::exact(1),
        detail: "Returns true if all elements of the generator are true (i.e., takes the logical conjunction of the elements).",
        synopsis: "generator and generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "any",
        arity: Arity::exact(1),
        detail: "Returns true if any of the elements of the generator are true (i.e., logical disjunction).",
        synopsis: "generator any generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "concat",
        arity: Arity::at_least(1),
        detail: "Returns a generator which is the concatenation of each of the argument generators.",
        synopsis: "generator concat generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "concatMap",
        arity: Arity::exact(2),
        detail: "Given a function which maps a value to a series of values, and a generator of values of that type, returns a genera",
        synopsis: "generator concatMap function generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "drop",
        arity: Arity::exact(2),
        detail: "Removes the given number of elements from the front of the generator and returns the resulting generator with those",
        synopsis: "generator drop n generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dropWhile",
        arity: Arity::exact(2),
        detail: "Removes all elements from the front of the generator that satisfy the predicate.",
        synopsis: "generator dropWhile predicate generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "contains",
        arity: Arity::exact(2),
        detail: "Returns true if the generator contains the given element.",
        synopsis: "generator contains element generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foldl1",
        arity: Arity::exact(2),
        detail: "A version of foldl that takes the zero argument from the first element of the generator.",
        synopsis: "generator foldl1 function generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foldli",
        arity: Arity::exact(3),
        detail: "A version of foldl that supplies the integer index of each element as the first argument to the function.",
        synopsis: "generator foldli function zero generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "foldri",
        arity: Arity::exact(3),
        detail: "Right-associative version of foldli.",
        synopsis: "generator foldri function zero generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "head",
        arity: Arity::exact(1),
        detail: "Returns the first element of the generator.",
        synopsis: "generator head generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tail",
        arity: Arity::exact(1),
        detail: "Removes the first element of the generator, returning the rest.",
        synopsis: "generator tail generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "init",
        arity: Arity::exact(1),
        detail: "Returns a new generator consisting of all elements except the last of the argument generator.",
        synopsis: "generator init generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "takeList",
        arity: Arity::exact(2),
        detail: "Returns the next n elements of the generator as a list.",
        synopsis: "generator takeList n generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "take",
        arity: Arity::exact(2),
        detail: "Returns the next n elements of the generator as a new generator.",
        synopsis: "generator take n generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "iterate",
        arity: Arity::exact(2),
        detail: "Returns an infinite generator formed by repeatedly applying the function to the initial argument.",
        synopsis: "generator iterate function init",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "last",
        arity: Arity::exact(1),
        detail: "Returns the last element of the generator (if it exists).",
        synopsis: "generator last generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "length",
        arity: Arity::exact(1),
        detail: "Returns the length of the generator, destroying it in the process.",
        synopsis: "generator length generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "or",
        arity: Arity::exact(2),
        detail: "Returns 1 if any of the elements of the generator satisfy the predicate.",
        synopsis: "generator or predicate generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "product",
        arity: Arity::exact(1),
        detail: "Returns the product of the numbers in a generator.",
        synopsis: "generator product generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "repeat",
        // `generator repeat n value ?value ...?` — one or more repeated values.
        arity: Arity::at_least(2),
        detail: "Returns a generator that consists of n copies of the given elements.",
        synopsis: "generator repeat n value ?value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sum",
        arity: Arity::exact(1),
        detail: "Returns the sum of the values in the generator.",
        synopsis: "generator sum generator",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "takeWhile",
        arity: Arity::exact(2),
        detail: "Returns a generator of the first elements in the argument generator that satisfy the predicate.",
        synopsis: "generator takeWhile predicate generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "splitWhen",
        arity: Arity::exact(2),
        detail: "Splits the generator into lists of elements using the predicate to identify delimiters.",
        synopsis: "generator splitWhen predicate generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "scanl",
        arity: Arity::exact(3),
        detail: "Similar to foldl, but returns a generator of all of the intermediate values for the accumulator argument.",
        synopsis: "generator scanl function zero generator",
        command_prefixes: &[(0, AppendedArity::Unknown)],
        script_timing_resolver: Some(deferred_first_arg),
        ..SubCommand::DEFAULT
    },
];

/// The `generator` ensemble command.
fn generator_spec() -> CommandSpec {
    CommandSpec {
        name: "generator",
        arity: Arity::at_least(1),
        subcommands: GENERATOR_SUBS,
        hover: Some(HoverSnippet::brief(
            "Create and drive lazy generator procedures (streams).",
            &["generator subcommand ?arg ...?"],
            "tcllib generator package",
        )),
        tcllib_package: Some("generator"),
        required_package: Some("generator"),
        ..CommandSpec::DEFAULT
    }
}

/// The `debug` ensemble's sub-commands.
const DEBUG_SUBS: &[SubCommand] = &[
    SubCommand {
        name: "2array",
        arity: Arity::exact(0),
        detail: "This method returns a dictionary mapping the names of all debug tags currently known to the package to their state ",
        synopsis: "debug 2array",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "define",
        arity: Arity::exact(1),
        detail: "This method registers the named tag with the package.",
        synopsis: "debug define tag",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "header",
        arity: Arity::exact(1),
        detail: "This method defines a global substable Tcl script which provides a text printed before each line of output.",
        synopsis: "debug header text",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "level",
        arity: Arity::at_least(1),
        detail: "This method sets the detail-level for the tag, and the channel fd to write the tags narration into.",
        synopsis: "debug level tag",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::exact(0),
        detail: "This method returns a list containing the names of all debug tags currently known to the package.",
        synopsis: "debug names",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "off",
        arity: Arity::exact(1),
        detail: "This method registers the named tag with the package and sets it inactive.",
        synopsis: "debug off tag",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "on",
        arity: Arity::exact(1),
        detail: "This method registers the named tag with the package, as active.",
        synopsis: "debug on tag",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "parray",
        arity: Arity::exact(1),
        detail: "This is a convenience method formatting the named array like the builtin command parray, except it returns the resu",
        synopsis: "debug parray arrayvarname",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pdict",
        arity: Arity::exact(1),
        detail: "This is a convenience method formatting the dictionary similarly to how the builtin command parray does for array, ",
        synopsis: "debug pdict dict",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hexl",
        arity: Arity::at_least(1),
        detail: "This is a convenience method formatting arbitrary data into a hex-dump and returns the resulting string.",
        synopsis: "debug hexl data",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nl",
        arity: Arity::exact(0),
        detail: "This is a convenience method to insert a linefeed character (ASCII 0x0a) into a debug message.",
        synopsis: "debug nl",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tab",
        arity: Arity::exact(0),
        detail: "This is a convenience method to insert a TAB character (ASCII 0x09) into a debug message.",
        synopsis: "debug tab",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "prefix",
        arity: Arity::at_least(1),
        detail: "This method is similar to the method header above, in that it defines substable Tcl script which provides more text",
        synopsis: "debug prefix tag",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "setting",
        arity: Arity::at_least(2),
        detail: "This method is a multi-tag variant of method level above, with the functionality of methods on, and off also folded",
        synopsis: "debug setting (tag level) ...",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "suffix",
        arity: Arity::at_least(1),
        detail: "This method is similar to the method trailer below, in that it defines substable Tcl script which provides more tex",
        synopsis: "debug suffix tag",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "trailer",
        arity: Arity::exact(1),
        detail: "This method defines a global substable Tcl script which provides a text printed after each line of output (before t",
        synopsis: "debug trailer text",
        ..SubCommand::DEFAULT
    },
];

/// The `debug` ensemble command.
fn debug_spec() -> CommandSpec {
    CommandSpec {
        name: "debug",
        arity: Arity::at_least(1),
        subcommands: DEBUG_SUBS,
        hover: Some(HoverSnippet::brief(
            "Conditional, tag-scoped debug narration.",
            &["debug subcommand ?arg ...?"],
            "tcllib debug package",
        )),
        tcllib_package: Some("debug"),
        required_package: Some("debug"),
        ..CommandSpec::DEFAULT
    }
}

/// `hook bind subject hook observer binding` (hook.tcl 97-137) sets `binding`
/// — a command prefix fired via `uplevel #0 [list {*}$binding {*}$args]` when
/// the hook is called, so the appended count is whatever the matching
/// `hook call subject hook ?arg…?` passes ⇒ `Unknown`.  The 1/2/3-arg forms
/// are query variants (and an empty 4th arg is a delete), so only the full
/// four-word set form names a callback.
fn hook_bind_command_prefixes(args: CommandPrefixArguments<'_>) -> Vec<(u8, AppendedArity)> {
    if args.len() == 4 {
        vec![(3, AppendedArity::Unknown)]
    } else {
        Vec::new()
    }
}

fn hook_bind_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    (args.len() == 4)
        .then_some((3, ScriptTiming::Deferred))
        .into_iter()
        .collect()
}

/// The `hook` ensemble's sub-commands.
const HOOK_SUBS: &[SubCommand] = &[
    SubCommand {
        name: "bind",
        arity: Arity::at_least(0),
        detail: "This subcommand is used to create, update, delete, and query hook bindings.",
        synopsis: "hook bind ?subject? ?hook? ?observer? ?binding?",
        command_prefix_resolver: Some(hook_bind_command_prefixes),
        script_timing_resolver: Some(hook_bind_script_timing),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "call",
        arity: Arity::at_least(2),
        detail: "This command is called when the named subject wishes to call the named hook.",
        synopsis: "hook call subject hook",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "forget",
        arity: Arity::exact(1),
        detail: "This command deletes any existing bindings in which the named object appears as either the subject or the observer.",
        synopsis: "hook forget object",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "cget",
        arity: Arity::exact(1),
        detail: "This command returns the value of one of the hook command's configuration options.",
        synopsis: "hook cget option",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "This command sets the value of one or more of the hook command's configuration options: If the value of this option",
        synopsis: "hook configure option value ...",
        ..SubCommand::DEFAULT
    },
];

/// The `hook` ensemble command.
fn hook_spec() -> CommandSpec {
    CommandSpec {
        name: "hook",
        arity: Arity::at_least(1),
        subcommands: HOOK_SUBS,
        hover: Some(HoverSnippet::brief(
            "Publish/subscribe hook points between decoupled components.",
            &["hook subcommand ?arg ...?"],
            "tcllib hook package",
        )),
        tcllib_package: Some("hook"),
        required_package: Some("hook"),
        ..CommandSpec::DEFAULT
    }
}

/// A row in the flat `websocket` table: name, arity, synopsis, summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// The flat `websocket` package.
const WEBSOCKET_CMDS: &[Row] = &[
    (
        "websocket::open",
        Arity::at_least(2),
        &["websocket::open url handler"],
        "This command is used in clients to open a WebSocket to a remote websocket-enabled HTTP server.",
    ),
    (
        "websocket::send",
        Arity::at_least(2),
        &["websocket::send sock type"],
        "This command will send a fragment or a control message to the remote end of the WebSocket identified by sock.",
    ),
    (
        "websocket::server",
        Arity::exact(1),
        &["websocket::server sock"],
        "This command registers the (accept) socket sock as the identifier fo an HTTP server that is capable of doing.",
    ),
    (
        "websocket::live",
        Arity::at_least(3),
        &["websocket::live sock path cb"],
        "This procedure registers callbacks that will be performed on a WebSocket compliant server registered with.",
    ),
    (
        "websocket::test",
        Arity::at_least(3),
        &["websocket::test srvSock cliSock path"],
        "This procedure will test if the connection from an incoming client on socket cliSock and on the path path is the.",
    ),
    (
        "websocket::upgrade",
        Arity::exact(1),
        &["websocket::upgrade sock"],
        "Upgrade the socket sock that had been deemed by ::websocket::test to be a WebSocket connection request to a true.",
    ),
    (
        "websocket::takeover",
        Arity::at_least(2),
        &["websocket::takeover sock handler"],
        "Take over the existing opened socket sock to implement sending and receiving WebSocket framing on top of the.",
    ),
    (
        "websocket::conninfo",
        Arity::exact(2),
        &["websocket::conninfo sock what"],
        "Provides callers with some introspection facilities in order to get some semi-internal information about an.",
    ),
    (
        "websocket::find",
        Arity::at_least(0),
        &["websocket::find"],
        "Look among existing websocket connections for the ones that match the hostname and port number filters passed as.",
    ),
    (
        "websocket::configure",
        Arity::at_least(2),
        &["websocket::configure sock args"],
        "This command takes a number of dash-led options (and their values) to configure the behaviour of an existing.",
    ),
    (
        "websocket::loglevel",
        Arity::at_least(0),
        &["websocket::loglevel"],
        "Set or query the log level of the library, which defaults to error.",
    ),
    (
        "websocket::close",
        Arity::at_least(1),
        &["websocket::close sock"],
        "Gracefully close a websocket that was directly or indirectly passed to ::websocket::takeover.",
    ),
];

/// All ensemble + `websocket` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = vec![generator_spec(), debug_spec(), hook_spec()];
    specs.extend(
        WEBSOCKET_CMDS
            .iter()
            .map(|&(name, arity, synopsis, summary)| CommandSpec {
                name,
                arity,
                hover: Some(HoverSnippet::brief(
                    summary,
                    synopsis,
                    "tcllib websocket package",
                )),
                tcllib_package: Some("websocket"),
                required_package: Some("websocket"),
                ..CommandSpec::DEFAULT
            }),
    );
    specs
}
