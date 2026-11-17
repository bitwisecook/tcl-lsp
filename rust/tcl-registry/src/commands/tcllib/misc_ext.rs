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

//! Additional tcllib package commands (non-math sweep).
//!
//! Remaining flat commands across many packages (html, ip, sha1/sha2/md5 incremental APIs, grammar, simulation, nettool, cron, zipfile, …), each attributed to its namespace package.

use super::{
    textutil__adjust, textutil__indent, textutil__longestcommonprefix, textutil__tabify,
    textutil__tabify2, textutil__trim, textutil__trimleft, textutil__trimright, textutil__undent,
    textutil__untabify, textutil__untabify2,
};
use crate::prelude::*;

mod digests;
mod documentation;
mod html;
mod networking;
mod parser_tools;
mod simulation;
mod terminal;
mod textutil;

use digests::{MD5_CMDS, SHA1_CMDS, SHA2_CMDS};
use documentation::{
    DOCTOOLS__CHANGELOG_CMDS, DOCTOOLS__CVS_CMDS, DOCTOOLS__HTML__CSSDEFAULTS_CMDS,
    DOCTOOLS__IDX_CMDS, DOCTOOLS__MSGCAT_CMDS, DOCTOOLS__NROFF__MAN_MACROS_CMDS,
    DOCTOOLS__TOC_CMDS, DOCTOOLS_CMDS, MKDOC_CMDS,
};
use html::{HTML_CMDS, html_rows};
use networking::{
    AUTOPROXY_CMDS, COMM_CMDS, DNS_CMDS, FTP_CMDS, IP_CMDS, MULTIPLEXER_CMDS,
    NAMESERV__COMMON_CMDS, NAMESERV__SERVER_CMDS, NAMESERV_CMDS, NETTOOL_CMDS, PICOIRC_CMDS,
    SMTPD_CMDS,
};
use parser_tools::{
    GRAMMAR__FA__OP_CMDS, GRAMMAR__ME__CPU__GASM_CMDS, GRAMMAR__ME__TCL_CMDS, GRAMMAR__ME_CMDS,
    GRAMMAR__PEG__INTERP_CMDS, PAGE__PLUGINMGR_CMDS, PAGE__UTIL__NORM_CMDS, PAGE__UTIL__PEG_CMDS,
    PAGE__UTIL__QUOTE_CMDS, PAGE__UTIL_CMDS, PT__PEG_CMDS, PT_CMDS, PT_EXPORT_API_CMDS,
    PT_IMPORT_API_CMDS,
};
use simulation::{
    SIMULATION__ANNEALING_CMDS, SIMULATION__MONTECARLO_CMDS, SIMULATION__RANDOM_CMDS,
};
use terminal::{
    TERM__ANSI__CODE__ATTR_CMDS, TERM__ANSI__CODE__CTRL_CMDS, TERM__ANSI__CODE__MACROS_CMDS,
    TERM__ANSI__CTRL__UNIX_CMDS, TERM__SEND_CMDS,
};
use textutil::{
    TEXTUTIL__ADJUST_CMDS, TEXTUTIL__PATCH_CMDS, TEXTUTIL__STRING_CMDS, TEXTUTIL__TABIFY_CMDS,
    TEXTUTIL__TRIM_CMDS, TEXTUTIL_CMDS,
};

/// A row in a package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build command specs for a package from its table.
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

/// The `bee` package.
const BEE_CMDS: &[Row] = &[
    (
        "bee::encodeString",
        Arity::exact(1),
        &["bee::encodeString string"],
        "Returns the bee-encoding of the string.",
    ),
    (
        "bee::encodeNumber",
        Arity::exact(1),
        &["bee::encodeNumber integer"],
        "Returns the bee-encoding of the integer number.",
    ),
    (
        "bee::encodeList",
        Arity::exact(1),
        &["bee::encodeList list"],
        "Takes a list of bee-encoded values and returns the bee-encoding of the list.",
    ),
    (
        "bee::encodeDict",
        Arity::exact(1),
        &["bee::encodeDict dict"],
        "Takes a dictionary list of string keys and bee-encoded values and returns the bee-encoding of the list.",
    ),
    (
        "bee::decodeCancel",
        Arity::exact(1),
        &["bee::decodeCancel token"],
        "This command cancels the decoder set up by ::bee::decodeChannel and represented by the handle token.",
    ),
    (
        "bee::decodePush",
        Arity::exact(2),
        &["bee::decodePush token string"],
        "This command appends the string to the internal decoder buffer.",
    ),
];

/// The `bench::in` package.
const BENCH__IN_CMDS: &[Row] = &[(
    "bench::in::read",
    Arity::exact(1),
    &["bench::in::read file"],
    "This command reads a benchmark result from the specified file and returns it as its result.",
)];

/// The `bench::out` package.
const BENCH__OUT_CMDS: &[Row] = &[
    (
        "bench::out::csv",
        Arity::exact(1),
        &["bench::out::csv bench_result"],
        "This command formats the specified benchmark result for output to a file, socket, etc.",
    ),
    (
        "bench::out::text",
        Arity::exact(1),
        &["bench::out::text bench_result"],
        "This command formats the specified benchmark result for output to a file, socket, etc.",
    ),
];

/// The `cron` package.
const CRON_CMDS: &[Row] = &[
    (
        "cron::at",
        Arity::new(2, 3),
        &["cron::at ?processname? timecode command"],
        "This command registers a command to be called at the time specified by timecode.",
    ),
    (
        "cron::cancel",
        Arity::exact(1),
        &["cron::cancel processname"],
        "This command unregisters the process processname and cancels any pending commands.",
    ),
    (
        "cron::every",
        Arity::exact(3),
        &["cron::every processname frequency command"],
        "This command registers a command to be called at the interval of frequency.",
    ),
    (
        "cron::in",
        Arity::new(2, 3),
        &["cron::in ?processname? timecode command"],
        "This command registers a command to be called after a delay of time specified by timecode.",
    ),
    (
        "cron::object_coroutine",
        Arity::new(2, 3),
        &["cron::object_coroutine object coroutine ?info?"],
        "This command registers a coroutine, associated with object to be called given the parameters of info.",
    ),
    (
        "cron::sleep",
        Arity::exact(1),
        &["cron::sleep milliseconds"],
        "When run within a coroutine, this command will register the coroutine for a callback at the appointed time, and i",
    ),
    (
        "cron::wake",
        Arity::new(0, 1),
        &["cron::wake ?who?"],
        "Wake up cron, and arrange for its event loop to be run during the next Idle cycle.",
    ),
    (
        "cron::clock_step",
        Arity::exact(1),
        &["cron::clock_step milliseconds"],
        "Return a clock time absolute to the epoch which falls on the next border between one second and the next for the ",
    ),
    (
        "cron::clock_delay",
        Arity::exact(1),
        &["cron::clock_delay milliseconds"],
        "Return a clock time absolute to the epoch which falls on the next border between one second and the next millisec",
    ),
    (
        "cron::clock_sleep",
        Arity::new(1, 2),
        &["cron::clock_sleep seconds ?offset?"],
        "Return a clock time absolute to the epoch which falls exactly seconds in the future.",
    ),
    (
        "cron::clock_set",
        Arity::exact(1),
        &["cron::clock_set newtime"],
        "Sets the internal clock for cron.",
    ),
];

/// The `fileutil` package.
const FILEUTIL_CMDS: &[Row] = &[
    (
        "fileutil::stripPath",
        Arity::exact(2),
        &["fileutil::stripPath prefix path"],
        "If, and only of the path is inside of the directory prefix (or the prefix directory itself) it is made relative t",
    ),
    (
        "fileutil::paths",
        Arity::exact(1),
        &["fileutil::paths poolName"],
        "Creates a new, empty pool of search paths with an associated global Tcl command whose name is poolName.",
    ),
];

/// The `fileutil::magic` package.
const FILEUTIL__MAGIC_CMDS: &[Row] = &[(
    "fileutil::magic::filetype",
    Arity::exact(1),
    &["fileutil::magic::filetype filename"],
    "This command is similar to the command fileutil::fileType.",
)];

/// The `fileutil::magic::cgen` package.
const FILEUTIL__MAGIC__CGEN_CMDS: &[Row] = &[
    (
        "fileutil::magic::cgen::2tree",
        Arity::exact(1),
        &["fileutil::magic::cgen::2tree script"],
        "This command converts the recognizer specified by the script into a tree and returns the object command of that t",
    ),
    (
        "fileutil::magic::cgen::treedump",
        Arity::exact(1),
        &["fileutil::magic::cgen::treedump tree"],
        "This command takes a tree as generated by ::fileutil::magic::cgen::2tree and returns a string encoding the tree f",
    ),
    (
        "fileutil::magic::cgen::treegen",
        Arity::exact(2),
        &["fileutil::magic::cgen::treegen tree node"],
        "This command takes a tree as generated by ::fileutil::magic::cgen::2tree and returns a Tcl script, the recognizer",
    ),
];

/// The `fileutil::magic::rt` package.
const FILEUTIL__MAGIC__RT_CMDS: &[Row] = &[
    (
        "fileutil::magic::rt::new",
        Arity::exact(3),
        &["fileutil::magic::rt::new chan named analyze"],
        "Create a new command which returns one description of the file each time it is called, and a code of break when t",
    ),
    (
        "fileutil::magic::rt::file_start",
        Arity::exact(1),
        &["fileutil::magic::rt::file_start name"],
        "This command marks the start of a magic file when debugging.",
    ),
    (
        "fileutil::magic::rt::emit",
        Arity::exact(1),
        &["fileutil::magic::rt::emit msg"],
        "This command adds the text msg to the result buffer.",
    ),
    (
        "fileutil::magic::rt::O",
        Arity::exact(1),
        &["fileutil::magic::rt::O where"],
        "Produce an offset from where, relative to the cursor one level up.",
    ),
    (
        "fileutil::magic::rt::Nv",
        Arity::exact(4),
        &["fileutil::magic::rt::Nv type offset [ arg compinvert] comp expected"],
        "A limited form of ::fileutile::magic::rt::N that only checks for equality and can't be told to invert the test.",
    ),
    (
        "fileutil::magic::rt::N",
        Arity::exact(7),
        &["fileutil::magic::rt::N type offset testinvert [ arg compinvert] mod mand comp expected"],
        "Fetch the numeric value with type from the absolute location offset, compare it with expected using comp as the c",
    ),
    (
        "fileutil::magic::rt::S",
        Arity::exact(6),
        &["fileutil::magic::rt::S type offset testinvert [ arg mod] mand comp val"],
        "Like ::fileutil::magic::rt::N except that it fetches and compares string types , not numeric data.",
    ),
    (
        "fileutil::magic::rt::L",
        Arity::exact(1),
        &["fileutil::magic::rt::L newlevel"],
        "Sets the current level in the calling context to newlevel.",
    ),
    (
        "fileutil::magic::rt::I",
        Arity::exact(5),
        &["fileutil::magic::rt::I offset it ioi ioo [ arg iir] io"],
        "Calculates an offset based on an initial offset and the provided modifiers.",
    ),
    (
        "fileutil::magic::rt::R",
        Arity::exact(1),
        &["fileutil::magic::rt::R offset"],
        "Given an initial offset, calculates an offset relative to the cursor at the next level up.",
    ),
    (
        "fileutil::magic::rt::U",
        Arity::exact(2),
        &["fileutil::magic::rt::U fileindex name"],
        "Add a level and use a named test script.",
    ),
];

/// The `interp` package.
const INTERP_CMDS: &[Row] = &[
    (
        "interp::snitLink",
        Arity::exact(2),
        &["interp::snitLink path methodlist"],
        "This command assumes that it was called from within a method of a snit object, and that the command mymethod is a",
    ),
    (
        "interp::snitDictLink",
        Arity::exact(2),
        &["interp::snitDictLink path methoddict"],
        "This command behaves like ::interp::snitLink, except that it takes a dictionary mapping from commands to methods ",
    ),
];

/// The `logger::utils` package.
const LOGGER__UTILS_CMDS: &[Row] = &[(
    "logger::utils::createFormatCmd",
    Arity::exact(1),
    &["logger::utils::createFormatCmd formatString"],
    "This command translates formatString into an expandable command string.",
)];

/// The `map::slippy` package.
const MAP__SLIPPY_CMDS: &[Row] = &[
    (
        "map::slippy::cache",
        Arity::exact(3),
        &["map::slippy::cache cacheName cachedir provider"],
        "Creates the cache cacheName and configures it with both the path to the directory contaiing the locally cached ti",
    ),
    (
        "map::slippy::fetcher",
        Arity::exact(3),
        &["map::slippy::fetcher fetcherName levels url"],
        "Creates the fetcher fetcherName and configures it with the number of zoom levels supported by the tile server, an",
    ),
];

/// The `processman` package.
const PROCESSMAN_CMDS: &[Row] = &[
    (
        "processman::find_exe",
        Arity::exact(1),
        &["processman::find_exe name"],
        "Locate an executable by the name of name in the system path.",
    ),
    (
        "processman::kill",
        Arity::exact(1),
        &["processman::kill id"],
        "Kill a child process id.",
    ),
    (
        "processman::kill_all",
        Arity::exact(0),
        &["processman::kill_all"],
        "Kill all processes spawned by this program.",
    ),
    (
        "processman::killexe",
        Arity::exact(1),
        &["processman::killexe name"],
        "Kill a process identified by the executable.",
    ),
    // `processman::onexit` is modelled separately ([`processman_onexit_spec`]):
    // its `cmd` is a deferred *script*, not a command prefix.
    (
        "processman::priority",
        Arity::exact(2),
        &["processman::priority id level"],
        "Mark process id with the priorty level.",
    ),
    (
        "processman::process_list",
        Arity::exact(0),
        &["processman::process_list"],
        "Return a list of processes that have been triggered by this program, as well as a boolean flag to indicate if the",
    ),
    (
        "processman::spawn",
        Arity::at_least(3),
        &["processman::spawn id cmd args"],
        "Start a child process, identified by id.",
    ),
];

/// The `tcl::chan` package.
const TCL__CHAN_CMDS: &[Row] = &[
    (
        "tcl::chan::core",
        Arity::exact(1),
        &["tcl::chan::core objectName"],
        "This command creates a new channel core object with an associated global Tcl command whose name is objectName.",
    ),
    (
        "tcl::chan::events",
        Arity::exact(1),
        &["tcl::chan::events objectName"],
        "This command creates a new channel event core object with an associated global Tcl command whose name is objectNa",
    ),
];

/// The `tcl::transform` package.
const TCL__TRANSFORM_CMDS: &[Row] = &[(
    "tcl::transform::core",
    Arity::exact(1),
    &["tcl::transform::core objectName"],
    "This command creates a new transform core object with an associated global Tcl command whose name is objectName.",
)];

/// The `uevent` package.
const UEVENT_CMDS: &[Row] = &[(
    "uevent::onidle",
    Arity::exact(2),
    &["uevent::onidle objectName commandprefix"],
    "The command creates a new onidle object with an associated global Tcl command whose name is objectName.",
)];

/// The `zipfile::decode` package.
const ZIPFILE__DECODE_CMDS: &[Row] = &[
    (
        "zipfile::decode::archive",
        Arity::exact(0),
        &["zipfile::decode::archive"],
        "This command decodes the last opened (and not yet closed) zip archive file.",
    ),
    (
        "zipfile::decode::close",
        Arity::exact(0),
        &["zipfile::decode::close"],
        "This command releases all state associated with the last call of ::zipfile::decode::open.",
    ),
    (
        "zipfile::decode::comment",
        Arity::exact(1),
        &["zipfile::decode::comment adict"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::content",
        Arity::exact(1),
        &["zipfile::decode::content archive"],
        "This is a convenience command which decodes the specified zip archive file and returns the list of paths found in",
    ),
    (
        "zipfile::decode::copyfile",
        Arity::exact(3),
        &["zipfile::decode::copyfile adict path dst"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::files",
        Arity::exact(1),
        &["zipfile::decode::files adict"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::getfile",
        Arity::exact(2),
        &["zipfile::decode::getfile zdict path"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::hasfile",
        Arity::exact(2),
        &["zipfile::decode::hasfile adict path"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::filesize",
        Arity::exact(2),
        &["zipfile::decode::filesize zdict path"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::filecomment",
        Arity::exact(2),
        &["zipfile::decode::filecomment zdict path"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::iszip",
        Arity::exact(1),
        &["zipfile::decode::iszip archive"],
        "This command takes the path of a presumed zip archive file and returns a boolean flag as the result of the comman",
    ),
    (
        "zipfile::decode::open",
        Arity::exact(1),
        &["zipfile::decode::open archive"],
        "This command takes the path of a zip archive file and prepares it for decoding.",
    ),
    (
        "zipfile::decode::unzip",
        Arity::exact(2),
        &["zipfile::decode::unzip adict dstdir"],
        "This command takes a dictionary describing the currently open zip archive file, as returned by ::zipfile::decode:",
    ),
    (
        "zipfile::decode::unzipfile",
        Arity::exact(2),
        &["zipfile::decode::unzipfile archive dstdir"],
        "This is a convenience command which unpacks the specified zip archive file in the given destination directory dst",
    ),
];

/// (package, command-table) pairs.
const GROUPS: &[(&str, &[Row])] = &[
    ("autoproxy", AUTOPROXY_CMDS),
    ("bee", BEE_CMDS),
    ("bench::in", BENCH__IN_CMDS),
    ("bench::out", BENCH__OUT_CMDS),
    ("comm", COMM_CMDS),
    ("cron", CRON_CMDS),
    ("dns", DNS_CMDS),
    ("doctools", DOCTOOLS_CMDS),
    ("doctools::changelog", DOCTOOLS__CHANGELOG_CMDS),
    ("doctools::cvs", DOCTOOLS__CVS_CMDS),
    (
        "doctools::html::cssdefaults",
        DOCTOOLS__HTML__CSSDEFAULTS_CMDS,
    ),
    ("doctools::idx", DOCTOOLS__IDX_CMDS),
    ("doctools::msgcat", DOCTOOLS__MSGCAT_CMDS),
    (
        "doctools::nroff::man_macros",
        DOCTOOLS__NROFF__MAN_MACROS_CMDS,
    ),
    ("doctools::toc", DOCTOOLS__TOC_CMDS),
    ("fileutil", FILEUTIL_CMDS),
    ("fileutil::magic", FILEUTIL__MAGIC_CMDS),
    ("fileutil::magic::cgen", FILEUTIL__MAGIC__CGEN_CMDS),
    ("fileutil::magic::rt", FILEUTIL__MAGIC__RT_CMDS),
    ("ftp", FTP_CMDS),
    ("grammar::fa::op", GRAMMAR__FA__OP_CMDS),
    ("grammar::me", GRAMMAR__ME_CMDS),
    ("grammar::me::cpu::gasm", GRAMMAR__ME__CPU__GASM_CMDS),
    ("grammar::me::tcl", GRAMMAR__ME__TCL_CMDS),
    ("grammar::peg::interp", GRAMMAR__PEG__INTERP_CMDS),
    ("interp", INTERP_CMDS),
    ("ip", IP_CMDS),
    ("logger::utils", LOGGER__UTILS_CMDS),
    ("map::slippy", MAP__SLIPPY_CMDS),
    ("md5", MD5_CMDS),
    ("mkdoc", MKDOC_CMDS),
    ("multiplexer", MULTIPLEXER_CMDS),
    ("nameserv", NAMESERV_CMDS),
    ("nameserv::common", NAMESERV__COMMON_CMDS),
    ("nameserv::server", NAMESERV__SERVER_CMDS),
    ("nettool", NETTOOL_CMDS),
    ("page::pluginmgr", PAGE__PLUGINMGR_CMDS),
    ("page::util", PAGE__UTIL_CMDS),
    ("page::util::norm", PAGE__UTIL__NORM_CMDS),
    ("page::util::peg", PAGE__UTIL__PEG_CMDS),
    ("page::util::quote", PAGE__UTIL__QUOTE_CMDS),
    ("picoirc", PICOIRC_CMDS),
    ("processman", PROCESSMAN_CMDS),
    ("pt", PT_CMDS),
    ("pt::peg", PT__PEG_CMDS),
    ("pt_export_api", PT_EXPORT_API_CMDS),
    ("pt_import_api", PT_IMPORT_API_CMDS),
    ("sha1", SHA1_CMDS),
    // The `::sha2` namespace; the package is `sha256`.
    ("sha256", SHA2_CMDS),
    ("simulation::annealing", SIMULATION__ANNEALING_CMDS),
    ("simulation::montecarlo", SIMULATION__MONTECARLO_CMDS),
    ("simulation::random", SIMULATION__RANDOM_CMDS),
    ("smtpd", SMTPD_CMDS),
    ("tcl::chan", TCL__CHAN_CMDS),
    ("tcl::transform", TCL__TRANSFORM_CMDS),
    ("term::ansi::code::attr", TERM__ANSI__CODE__ATTR_CMDS),
    ("term::ansi::code::ctrl", TERM__ANSI__CODE__CTRL_CMDS),
    ("term::ansi::code::macros", TERM__ANSI__CODE__MACROS_CMDS),
    ("term::ansi::ctrl::unix", TERM__ANSI__CTRL__UNIX_CMDS),
    ("term::send", TERM__SEND_CMDS),
    ("textutil", TEXTUTIL_CMDS),
    ("textutil::adjust", TEXTUTIL__ADJUST_CMDS),
    ("textutil::patch", TEXTUTIL__PATCH_CMDS),
    ("textutil::string", TEXTUTIL__STRING_CMDS),
    ("textutil::tabify", TEXTUTIL__TABIFY_CMDS),
    ("textutil::trim", TEXTUTIL__TRIM_CMDS),
    ("uevent", UEVENT_CMDS),
    ("zipfile::decode", ZIPFILE__DECODE_CMDS),
];

/// `processman::onexit id cmd` — arranges to run `cmd` when process `id`
/// terminates.  Source (processman.tcl 178/256) fires it as a bare
/// `eval $cmd` with **nothing** appended: `cmd` is a deferred Tcl *script*, not
/// a command prefix (so it is deliberately NOT a `command_prefixes` entry).
/// Modelled with `ArgRole::Body` + `BodyKind::Structural` — the same shape as
/// `tcltest::loadscript` — so a literal `{…}` script records its inner calls
/// (references / W123 / call-graph) while SSA does not bind its vars to the
/// caller's frame (the eval happens later, in `::processman::events`).
fn processman_onexit_spec() -> CommandSpec {
    CommandSpec {
        name: "processman::onexit",
        arity: Arity::exact(2),
        hover: Some(HoverSnippet::brief(
            "Arrange to execute the script cmd when this program detects that process id has terminated.",
            &["processman::onexit id cmd"],
            "tcllib processman package",
        )),
        tcllib_package: Some("processman"),
        required_package: Some("processman"),
        arg_roles: &[(1, ArgRole::Body)],
        body_kind: BodyKind::Structural,
        // `DEFERS_BODY` — the fact the doc comment above already states, now
        // said in data: the eval happens later, in `::processman::events`.
        // tclsh 8.6.16 / 9.0.4, byte-identical: `proc p {} {
        // processman::onexit x {error stop}; set ::reached 1 }` sets
        // `::reached`.
        traits: Traits::DEFERS_BODY,
        ..CommandSpec::DEFAULT
    }
}

/// `(submodule-qualified name, that command's canonical spec)` pairs: every
/// `Row`-declared submodule command above that is *also* re-exported,
/// unmodified, as a flattened `textutil::NAME` umbrella alias by one of the
/// individual `textutil__*.rs` files elsewhere in this crate (`adjust` /
/// `indent` / `undent` from the `textutil::adjust` package; `trim` /
/// `trimleft` / `trimright` from `textutil::trim`; `tabify` / `untabify` /
/// `tabify2` / `untabify2` from `textutil::tabify`; `longestCommonPrefix`
/// from `textutil::string`).
///
/// The umbrella alias file is the sole source of truth for that command's
/// trait descriptors and its value-transfer declaration —
/// [`sync_textutil_submodule_traits`] copies them onto the `Row`-built
/// submodule spec below rather than a second, independent declaration, so the
/// two spellings of the same real tcllib command can never drift apart:
/// without it, the submodule spec built via the generic `Row` →
/// `CommandSpec::DEFAULT` path would silently lack the traits its
/// umbrella sibling carries, blocking GVN/DCE on the canonical
/// three-segment spelling only.
fn textutil_submodule_trait_sources() -> Vec<(&'static str, CommandSpec)> {
    vec![
        ("textutil::adjust::adjust", textutil__adjust::spec()),
        ("textutil::adjust::indent", textutil__indent::spec()),
        ("textutil::adjust::undent", textutil__undent::spec()),
        ("textutil::trim::trim", textutil__trim::spec()),
        ("textutil::trim::trimleft", textutil__trimleft::spec()),
        ("textutil::trim::trimright", textutil__trimright::spec()),
        ("textutil::tabify::tabify", textutil__tabify::spec()),
        ("textutil::tabify::untabify", textutil__untabify::spec()),
        ("textutil::tabify::tabify2", textutil__tabify2::spec()),
        ("textutil::tabify::untabify2", textutil__untabify2::spec()),
        (
            "textutil::string::longestCommonPrefix",
            textutil__longestcommonprefix::spec(),
        ),
    ]
}

/// Apply [`textutil_submodule_trait_sources`]'s canonical traits onto their
/// matching `specs` entries in place. A missing name is a silent no-op — the
/// pairing is exhaustively checked by the `registry_commands.rs` contract
/// test instead of panicking here.
fn sync_textutil_submodule_traits(specs: &mut [CommandSpec]) {
    for (name, canonical) in textutil_submodule_trait_sources() {
        if let Some(spec) = specs.iter_mut().find(|s| s.name == name) {
            spec.traits = canonical.traits;
            spec.semantics = canonical.semantics;
        }
    }
}

/// All command specs in this group.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs: Vec<CommandSpec> = GROUPS
        .iter()
        .flat_map(|&(pkg, table)| rows(pkg, table))
        // The `html` package uses the richer [`HtmlRow`] table (per-command
        // return type, purity, and introducing release) rather than the shared
        // 4-tuple [`Row`], so it is appended separately.
        .chain(html_rows(HTML_CMDS))
        .chain(std::iter::once(processman_onexit_spec()))
        .collect();
    sync_textutil_submodule_traits(&mut specs);
    specs
}
