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

//! tcllib packages — `comm` (inter-interp communication ensemble),
//! `profiler`, and `bench`.
//!
//! Command names, arity bounds, synopses, and summaries are derived from the
//! upstream tcllib 2.0 manual pages.  Requires Tcl 8.5+.

use crate::prelude::*;

/// A row in a flat package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build command specs for a flat package from its table.
fn rows(pkg: &'static str, table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary)| CommandSpec {
            name,
            arity,
            hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib package")),
            tcllib_package: Some(pkg),
            required_package: Some(pkg),
            ..CommandSpec::DEFAULT
        })
        .collect()
}

/// Options for `comm::comm send`.
const COMM_SEND_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-async",
        value: OptionValue::flag(),
        detail: "Return immediately; the result is discarded (or delivered via -command).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        // Asynchronous "futures" reply callback.  comm.tcl 1367-1375 builds a
        // fixed 7-pair option list (`-id -serial -chan -code -errorcode
        // -errorinfo -result`) and fires `uplevel #0 $callback $args`, which
        // re-splits it into exactly 14 words.  (Distinct from the channel-level
        // `configure -command` incoming-message hook, which appends only 1.)
        // Real callbacks use `proc cb {args} {array set r $args …}`, so the
        // arity check is silent unless a fixed-arity proc is wired up wrong.
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(14)),
        detail: "Deliver the reply asynchronously to this command prefix (invoked \
with the 7 -key/value reply pairs appended).",
        ..OptionSpec::DEFAULT
    },
];

/// The `comm::comm` ensemble's sub-commands.
const COMM_SUBS: &[SubCommand] = &[
    SubCommand {
        name: "send",
        arity: Arity::at_least(2),
        detail: "This invokes the given command in the interpreter named by id.",
        synopsis: "comm::comm send -async -command id cmd",
        options: COMM_SEND_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "self",
        arity: Arity::exact(0),
        detail: "Returns the id for this channel.",
        synopsis: "comm::comm self",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "interps",
        arity: Arity::exact(0),
        detail: "Returns a list of all the remote id's to which this channel is connected.",
        synopsis: "comm::comm interps",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "connect",
        arity: Arity::at_least(0),
        detail: "Whereas ::comm::comm send will automatically connect to the given id, this forces a connection to a remote id witho",
        synopsis: "comm::comm connect",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "new",
        arity: Arity::at_least(1),
        detail: "This creates a new channel and Tcl command with the given channel name.",
        synopsis: "comm::comm new chan",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "channels",
        arity: Arity::exact(0),
        detail: "This lists all the channels allocated in this Tcl interpreter.",
        synopsis: "comm::comm channels",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "config",
        arity: Arity::at_least(0),
        detail: "",
        synopsis: "comm::comm config",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "shutdown",
        arity: Arity::exact(1),
        detail: "This closes the connection to id, aborting all outstanding commands in progress.",
        synopsis: "comm::comm shutdown id",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "abort",
        arity: Arity::exact(0),
        detail: "This invokes shutdown on all open connections in this comm channel.",
        synopsis: "comm::comm abort",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "destroy",
        arity: Arity::exact(0),
        detail: "This aborts all connections and then destroys the this comm channel itself, including closing the listening socket.",
        synopsis: "comm::comm destroy",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "hook",
        arity: Arity::at_least(1),
        detail: "This uses a syntax similar to Tk's bind command.",
        synopsis: "comm::comm hook event",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "remoteid",
        arity: Arity::exact(0),
        detail: "Returns the id of the sender of the last remote command executed on this channel.",
        synopsis: "comm::comm remoteid",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "return_async",
        arity: Arity::exact(0),
        detail: "This command is used by a remotely invoked script to notify the comm channel which invoked it that the result to se",
        synopsis: "comm::comm return_async",
        ..SubCommand::DEFAULT
    },
];

/// The `profiler` package.
const PROFILER_CMDS: &[Row] = &[
    (
        "profiler::init",
        Arity::exact(0),
        &["profiler::init"],
        "Initiate profiling.",
    ),
    (
        "profiler::dump",
        Arity::exact(1),
        &["profiler::dump pattern"],
        "Dump profiling information for the all functions matching pattern.",
    ),
    (
        "profiler::print",
        Arity::at_least(0),
        &["profiler::print"],
        "Print profiling information for all functions matching pattern.",
    ),
    (
        "profiler::reset",
        Arity::at_least(0),
        &["profiler::reset"],
        "Reset profiling information for all functions matching pattern.",
    ),
    (
        "profiler::suspend",
        Arity::at_least(0),
        &["profiler::suspend"],
        "Suspend profiling for all functions matching pattern.",
    ),
    (
        "profiler::resume",
        Arity::at_least(0),
        &["profiler::resume"],
        "Resume profiling for all functions matching pattern.",
    ),
    (
        "profiler::sortFunctions",
        Arity::exact(1),
        &["profiler::sortFunctions key"],
        "Return a list of functions sorted by a particular profiling statistic.",
    ),
];

/// The `bench` package.
const BENCH_CMDS: &[Row] = &[
    (
        "bench::locate",
        Arity::exact(2),
        &["bench::locate pattern paths"],
        "This command locates Tcl interpreters and returns a list containing their paths.",
    ),
    (
        "bench::run",
        Arity::at_least(2),
        &["bench::run interp_list file..."],
        "This command executes the benchmarks declared in the set of files, once per Tcl interpreter specified via the.",
    ),
    (
        "bench::versions",
        Arity::exact(1),
        &["bench::versions interp_list"],
        "This command takes a list of Tcl interpreters, identified by their path, and returns a dictionary mapping from.",
    ),
    (
        "bench::del",
        Arity::exact(2),
        &["bench::del bench_result column"],
        "This command removes a column, i.e.",
    ),
    (
        "bench::edit",
        Arity::exact(3),
        &["bench::edit bench_result column newvalue"],
        "This command renames a column in the specified benchmark result and returns the modified result.",
    ),
    (
        "bench::merge",
        Arity::at_least(1),
        &["bench::merge bench_result..."],
        "This commands takes one or more benchmark results, merges them into one big result, and returns that as its result.",
    ),
    (
        "bench::norm",
        Arity::exact(2),
        &["bench::norm bench_result column"],
        "This command normalizes the timing results in the specified benchmark result and returns the modified result.",
    ),
    (
        "bench::out::raw",
        Arity::exact(1),
        &["bench::out::raw bench_result"],
        "This command formats the specified benchmark result for output to a file, socket, etc.",
    ),
];

/// All `comm` / `profiler` / `bench` command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = vec![CommandSpec {
        name: "comm::comm",
        arity: Arity::at_least(1),
        subcommands: COMM_SUBS,
        hover: Some(HoverSnippet::brief(
            "Inter-interpreter communication over sockets (send/self/interps/…).",
            &["comm::comm subcommand ?arg ...?"],
            "tcllib comm package",
        )),
        tcllib_package: Some("comm"),
        required_package: Some("comm"),
        ..CommandSpec::DEFAULT
    }];
    specs.extend(rows("profiler", PROFILER_CMDS));
    specs.extend(rows("bench", BENCH_CMDS));
    specs
}
