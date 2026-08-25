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

//! tcllib `doctools` package commands and the `tcl::chan::*` /
//! `tcl::randomseed` reflected-channel (virtual channel) creators.
//!
//! Each `tcl::chan::*` command is its own tcllib package whose name is the
//! command name.  Command names, arity bounds, synopses, and summaries are
//! derived from the upstream tcllib 2.0 manual pages.  Requires Tcl 8.5+.

use crate::prelude::*;

/// A row in a flat package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build command specs whose package is a single shared name.
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

/// Build channel-creator specs where each command *is* its own package.
fn self_pkg_rows(table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary)| CommandSpec {
            name,
            arity,
            hover: Some(HoverSnippet::brief(
                summary,
                synopsis,
                "tcllib virtchannel package",
            )),
            tcllib_package: Some(name),
            required_package: Some(name),
            ..CommandSpec::DEFAULT
        })
        .collect()
}

/// The `doctools` package-level commands.
const DOCTOOLS_CMDS: &[Row] = &[
    (
        "doctools::new",
        Arity::at_least(1),
        &["doctools::new objectName"],
        "This command creates a new doctools object with an associated Tcl command whose name is objectName.",
    ),
    (
        "doctools::help",
        Arity::exact(0),
        &["doctools::help"],
        "This is a convenience command for applications wishing to provide their user with a short description of the.",
    ),
    (
        "doctools::search",
        Arity::exact(1),
        &["doctools::search path"],
        "Whenever an object created by this the package has to map the name of a format to the file containing the code.",
    ),
];

/// The reflected-channel creators (each is its own `tcl::chan::*` package).
const VIRTCHAN_CMDS: &[Row] = &[
    (
        "tcl::chan::cat",
        Arity::at_least(1),
        &["tcl::chan::cat chan..."],
        "This command creates the concatenation channel using all the provided channels, and returns its handle.",
    ),
    (
        "tcl::chan::facade",
        Arity::exact(1),
        &["tcl::chan::facade chan"],
        "This command creates the facade channel around the provided channel chan, and returns its handle.",
    ),
    (
        "tcl::chan::fifo",
        Arity::exact(0),
        &["tcl::chan::fifo"],
        "This command creates a new fifo channel and returns its handle.",
    ),
    (
        "tcl::chan::fifo2",
        Arity::exact(0),
        &["tcl::chan::fifo2"],
        "This command creates a new connected pair of fifo channels and returns their handles, as a list containing two.",
    ),
    (
        "tcl::chan::memchan",
        Arity::exact(0),
        &["tcl::chan::memchan"],
        "This command creates a new memchan channel and returns its handle.",
    ),
    (
        "tcl::chan::null",
        Arity::exact(0),
        &["tcl::chan::null"],
        "This command creates a new null channel and returns its handle.",
    ),
    (
        "tcl::chan::nullzero",
        Arity::exact(0),
        &["tcl::chan::nullzero"],
        "This command creates a new nullzero channel and returns its handle.",
    ),
    (
        "tcl::chan::random",
        Arity::exact(1),
        &["tcl::chan::random seed"],
        "This command creates a new random channel and returns its handle.",
    ),
    (
        "tcl::chan::std",
        Arity::exact(0),
        &["tcl::chan::std"],
        "This command creates the std channel and returns its handle.",
    ),
    (
        "tcl::chan::string",
        Arity::exact(1),
        &["tcl::chan::string content"],
        "This command creates a new string channel and returns its handle.",
    ),
    (
        "tcl::chan::textwindow",
        Arity::exact(1),
        &["tcl::chan::textwindow widget"],
        "This command creates a new textwindow channel and returns its handle.",
    ),
    (
        "tcl::chan::variable",
        Arity::exact(1),
        &["tcl::chan::variable varname"],
        "This command creates a new variable channel and returns its handle.",
    ),
    (
        "tcl::chan::zero",
        Arity::exact(0),
        &["tcl::chan::zero"],
        "This command creates a new zero channel and returns its handle.",
    ),
    (
        "tcl::combine",
        Arity::exact(2),
        &["tcl::combine seed1 seed2"],
        "This command takes to seed lists and combines them into a single list by XORing them elementwise, modulo 256.",
    ),
    (
        "tcl::randomseed",
        Arity::exact(0),
        &["tcl::randomseed"],
        "This command creates returns a list of seed integers suitable as seed argument for random channels.",
    ),
    (
        "tcl::transform::adler32",
        Arity::at_least(2),
        &["tcl::transform::adler32 chan -option value..."],
        "This command creates an adler32 checksumming transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::base64",
        Arity::exact(1),
        &["tcl::transform::base64 chan"],
        "This command creates a base64 transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::counter",
        Arity::at_least(2),
        &["tcl::transform::counter chan -option value..."],
        "This command creates a counter transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::crc32",
        Arity::at_least(2),
        &["tcl::transform::crc32 chan -option value..."],
        "This command creates a crc32 checksumming transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::hex",
        Arity::exact(1),
        &["tcl::transform::hex chan"],
        "This command creates a hex transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::identity",
        Arity::exact(1),
        &["tcl::transform::identity chan"],
        "This command creates an identity transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::limitsize",
        Arity::exact(2),
        &["tcl::transform::limitsize chan max"],
        "This command creates a size limiting transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::observe",
        Arity::exact(3),
        &["tcl::transform::observe chan logw logr"],
        "This command creates an observer transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::otp",
        Arity::exact(3),
        &["tcl::transform::otp chan keychanw keychanr"],
        "This command creates a one-time pad based encryption transformation on top of the channel chan and returns its.",
    ),
    (
        "tcl::transform::rot",
        Arity::exact(2),
        &["tcl::transform::rot chan key"],
        "This command creates a rot encryption transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::spacer",
        Arity::at_least(2),
        &["tcl::transform::spacer chan n"],
        "This command creates a spacer transformation on top of the channel chan and returns its handle.",
    ),
    (
        "tcl::transform::zlib",
        Arity::at_least(1),
        &["tcl::transform::zlib chan"],
        "This command creates a zlib compressor transformation on top of the channel chan and returns its handle.",
    ),
];

/// Options for `tcl::chan::halfpipe`.  All three fire through the one
/// `method Call {o args} { uplevel #0 [list {*}$options($o) {*}$args] }`
/// (halfpipe.tcl 189-192), the clean `[list {*}prefix {*}args]` idiom, so the
/// appended-word count equals the number of args passed at each call site.
/// There is no `-read-command`.
const HALFPIPE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-write-command",
        // `my Call -write-command $c $bytes` (115) → channel handle + bytes.
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Invoked on a write with the channel handle and the written bytes.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-empty-command",
        // `my Call -empty-command $channel` (184) → channel handle only.
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(1)),
        detail: "Invoked when the buffer drains, with the channel handle.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-close-command",
        // `my Call -close-command $c` (51) → channel handle only.
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(1)),
        detail: "Invoked when the channel is closed, with the channel handle.",
        ..OptionSpec::DEFAULT
    },
];

/// `tcl::chan::halfpipe` — structured so its callback options carry command
/// prefixes.
fn halfpipe_spec() -> CommandSpec {
    CommandSpec {
        name: "tcl::chan::halfpipe",
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command creates a halfpipe channel and configures it with the callbacks to run when the channel is closed.",
            &["tcl::chan::halfpipe ?-write-command cmd? ?-empty-command cmd? ?-close-command cmd?"],
            "tcllib virtchannel package",
        )),
        options: HALFPIPE_OPTIONS,
        tcllib_package: Some("tcl::chan::halfpipe"),
        required_package: Some("tcl::chan::halfpipe"),
        ..CommandSpec::DEFAULT
    }
}

/// All doctools + virtual-channel command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = rows("doctools", DOCTOOLS_CMDS);
    specs.extend(self_pkg_rows(VIRTCHAN_CMDS));
    specs.push(halfpipe_spec());
    specs
}
