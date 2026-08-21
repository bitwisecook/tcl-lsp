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

/// The `autoproxy` package.
const AUTOPROXY_CMDS: &[Row] = &[
    (
        "autoproxy::init",
        Arity::exact(0),
        &["autoproxy::init"],
        "Initialize the autoproxy package from system resources.",
    ),
    (
        "autoproxy::cget",
        Arity::exact(1),
        &["autoproxy::cget -option"],
        "Retrieve individual package configuration options.",
    ),
    (
        "autoproxy::tls_connect",
        Arity::at_least(1),
        &["autoproxy::tls_connect args"],
        "Connect to a secure socket through a proxy.",
    ),
    (
        "autoproxy::tunnel_connect",
        Arity::at_least(1),
        &["autoproxy::tunnel_connect args"],
        "Connect to a target host throught a proxy.",
    ),
    (
        "autoproxy::tls_socket",
        Arity::at_least(1),
        &["autoproxy::tls_socket args"],
        "This function is to be used to register a proxy-aware secure socket handler for the https protocol.",
    ),
];

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

/// The `comm` package.
const COMM_CMDS: &[Row] = &[(
    "comm::comm_send",
    Arity::exact(0),
    &["comm::comm_send"],
    "Invoking this procedure will substitute the Tk send and winfo interps commands with these equivalents that use ::",
)];

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

/// The `dns` package.
const DNS_CMDS: &[Row] = &[(
    "dns::nameservers",
    Arity::exact(0),
    &["dns::nameservers"],
    "Attempts to return a list of the nameservers currently configured for the users system.",
)];

/// The `doctools` package.
const DOCTOOLS_CMDS: &[Row] = &[
    (
        "doctools::idx",
        Arity::exact(1),
        &["doctools::idx objectName"],
        "This command creates a new container object with an associated Tcl command whose name is objectName.",
    ),
    (
        "doctools::toc",
        Arity::exact(1),
        &["doctools::toc objectName"],
        "This command creates a new container object with an associated Tcl command whose name is objectName.",
    ),
];

/// The `doctools::changelog` package.
const DOCTOOLS__CHANGELOG_CMDS: &[Row] = &[
    (
        "doctools::changelog::scan",
        Arity::exact(1),
        &["doctools::changelog::scan text"],
        "The command takes the text and parses it under the assumption that it contains a ChangeLog as generated by emacs.",
    ),
    (
        "doctools::changelog::flatten",
        Arity::exact(1),
        &["doctools::changelog::flatten entries"],
        "This command converts a list of entries as generated by change::scan above into a simpler list of plain text bloc",
    ),
    (
        "doctools::changelog::toDoctools",
        Arity::exact(4),
        &["doctools::changelog::toDoctools title module version entries"],
        "This command converts the pre-parsed ChangeLog entries as generated by the command ::doctools::changelog::scan in",
    ),
];

/// The `doctools::cvs` package.
const DOCTOOLS__CVS_CMDS: &[Row] = &[
    (
        "doctools::cvs::scanLog",
        Arity::exact(4),
        &["doctools::cvs::scanLog text evar cvar fvar"],
        "The command takes the text and parses it under the assumption that it contains a CVS log as generated by cvs log.",
    ),
    (
        "doctools::cvs::toChangeLog",
        Arity::exact(3),
        &["doctools::cvs::toChangeLog evar cvar fvar"],
        "The three arguments for this command are the same as the last three arguments of the command ::doctools::cvs::sca",
    ),
];

/// The `doctools::html::cssdefaults` package.
const DOCTOOLS__HTML__CSSDEFAULTS_CMDS: &[Row] = &[(
    "doctools::html::cssdefaults::contents",
    Arity::exact(0),
    &["doctools::html::cssdefaults::contents"],
    "This command returns the text of the default CSS style to use for HTML markup generated by the various HTML expor",
)];

/// The `doctools::idx` package.
const DOCTOOLS__IDX_CMDS: &[Row] = &[
    (
        "doctools::idx::help",
        Arity::exact(0),
        &["doctools::idx::help"],
        "This is a convenience command for applications wishing to provide their user with a short description of the avai",
    ),
    (
        "doctools::idx::search",
        Arity::exact(1),
        &["doctools::idx::search path"],
        "Whenever an object created by this the package has to map the name of a format to the file containing the code fo",
    ),
    (
        "doctools::idx::export",
        Arity::exact(1),
        &["doctools::idx::export objectName"],
        "This command creates a new export manager object with an associated Tcl command whose name is objectName.",
    ),
    (
        "doctools::idx::import",
        Arity::exact(1),
        &["doctools::idx::import objectName"],
        "This command creates a new import manager object with an associated Tcl command whose name is objectName.",
    ),
];

/// The `doctools::msgcat` package.
const DOCTOOLS__MSGCAT_CMDS: &[Row] = &[(
    "doctools::msgcat::init",
    Arity::exact(1),
    &["doctools::msgcat::init prefix"],
    "The command locates and loads the message catalogs for all the languages returned by msgcat::mcpreferences, provi",
)];

/// The `doctools::nroff::man_macros` package.
const DOCTOOLS__NROFF__MAN_MACROS_CMDS: &[Row] = &[(
    "doctools::nroff::man_macros::contents",
    Arity::exact(0),
    &["doctools::nroff::man_macros::contents"],
    "This command returns the text of the default CSS style to use for NROFF generated by the various NROFF export plu",
)];

/// The `doctools::toc` package.
const DOCTOOLS__TOC_CMDS: &[Row] = &[
    (
        "doctools::toc::help",
        Arity::exact(0),
        &["doctools::toc::help"],
        "This is a convenience command for applications wishing to provide their user with a short description of the avai",
    ),
    (
        "doctools::toc::search",
        Arity::exact(1),
        &["doctools::toc::search path"],
        "Whenever an object created by this the package has to map the name of a format to the file containing the code fo",
    ),
    (
        "doctools::toc::export",
        Arity::exact(1),
        &["doctools::toc::export objectName"],
        "This command creates a new export manager object with an associated Tcl command whose name is objectName.",
    ),
    (
        "doctools::toc::import",
        Arity::exact(1),
        &["doctools::toc::import objectName"],
        "This command creates a new import manager object with an associated Tcl command whose name is objectName.",
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

/// The `ftp` package.
const FTP_CMDS: &[Row] = &[(
    "ftp::geturl",
    Arity::exact(1),
    &["ftp::geturl url"],
    "This command can be used by the generic command ::uri::geturl (See package uri) to retrieve the contents of ftp u",
)];

/// The `grammar::fa::op` package.
const GRAMMAR__FA__OP_CMDS: &[Row] = &[
    (
        "grammar::fa::op::constructor",
        Arity::exact(1),
        &["grammar::fa::op::constructor cmd"],
        "This command has to be called by the user of the package before any other operations is performed, to establish a",
    ),
    (
        "grammar::fa::op::reverse",
        Arity::exact(1),
        &["grammar::fa::op::reverse fa"],
        "Reverses the fa.",
    ),
    (
        "grammar::fa::op::remove_eps",
        Arity::exact(1),
        &["grammar::fa::op::remove_eps fa"],
        "Removes all epsilon-transitions from the fa in such a manner the the language of fa is unchanged.",
    ),
    (
        "grammar::fa::op::complement",
        Arity::exact(1),
        &["grammar::fa::op::complement fa"],
        "Complements fa.",
    ),
    (
        "grammar::fa::op::kleene",
        Arity::exact(1),
        &["grammar::fa::op::kleene fa"],
        "Applies Kleene's closure to fa.",
    ),
    (
        "grammar::fa::op::optional",
        Arity::exact(1),
        &["grammar::fa::op::optional fa"],
        "Makes the fa optional.",
    ),
    (
        "grammar::fa::op::toRegexp",
        Arity::exact(1),
        &["grammar::fa::op::toRegexp fa"],
        "This command generates and returns a regular expression which accepts the same language as the finite automaton f",
    ),
    (
        "grammar::fa::op::toRegexp2",
        Arity::exact(1),
        &["grammar::fa::op::toRegexp2 fa"],
        "This command has the same functionality as ::grammar::fa::op::toRegexp, but uses a different algorithm to simplif",
    ),
    (
        "grammar::fa::op::toTclRegexp",
        Arity::exact(2),
        &["grammar::fa::op::toTclRegexp regexp symdict"],
        "This command generates and returns a regular expression in Tcl syntax for the regular expression regexp, if that ",
    ),
    (
        "grammar::fa::op::simplifyRegexp",
        Arity::exact(1),
        &["grammar::fa::op::simplifyRegexp regexp"],
        "This command simplifies a regular expression by applying the following algorithm first to the main expression and",
    ),
];

/// The `grammar::me` package.
const GRAMMAR__ME_CMDS: &[Row] = &[(
    "grammar::me::cpu",
    Arity::exact(2),
    &["grammar::me::cpu meName matchcode"],
    "The command creates a new ME machine object with an associated global Tcl command whose name is meName.",
)];

/// The `grammar::me::cpu::gasm` package.
const GRAMMAR__ME__CPU__GASM_CMDS: &[Row] = &[
    (
        "grammar::me::cpu::gasm::done",
        Arity::exact(1),
        &["grammar::me::cpu::gasm::done --> t"],
        "This command finalizes the creation of an instruction sequence and then clears the state of the assembler.",
    ),
    (
        "grammar::me::cpu::gasm::state",
        Arity::exact(0),
        &["grammar::me::cpu::gasm::state"],
        "This command returns the current state of the assembler.",
    ),
    (
        "grammar::me::cpu::gasm::lift",
        Arity::exact(3),
        &["grammar::me::cpu::gasm::lift t dst = src"],
        "This command operates on the tree t.",
    ),
    (
        "grammar::me::cpu::gasm::Inline",
        Arity::exact(3),
        &["grammar::me::cpu::gasm::Inline t node label"],
        "This command links an instruction sequence created by an earlier begin/done pair into the current instruction seq",
    ),
    (
        "grammar::me::cpu::gasm::Bra",
        Arity::exact(0),
        &["grammar::me::cpu::gasm::Bra"],
        "This is a convenience command to create a .BRA pseudo-instruction.",
    ),
    (
        "grammar::me::cpu::gasm::Nop",
        Arity::exact(1),
        &["grammar::me::cpu::gasm::Nop text"],
        "This is a convenience command to create a .NOP pseudo-instruction.",
    ),
    (
        "grammar::me::cpu::gasm::Note",
        Arity::exact(1),
        &["grammar::me::cpu::gasm::Note text"],
        "This is a convenience command to create a .C pseudo-instruction, i.e.",
    ),
    (
        "grammar::me::cpu::gasm::Jmp",
        Arity::exact(1),
        &["grammar::me::cpu::gasm::Jmp label"],
        "This command creates an arc from the anchor to the instruction labeled with label, and tags with the the current ",
    ),
    (
        "grammar::me::cpu::gasm::Exit",
        Arity::exact(0),
        &["grammar::me::cpu::gasm::Exit"],
        "This command creates an arc from the anchor to one of the exit instructions, based on the operation mode (see ::g",
    ),
    (
        "grammar::me::cpu::gasm::Who",
        Arity::exact(1),
        &["grammar::me::cpu::gasm::Who label"],
        "This command returns a reference to the instruction labeled with label.",
    ),
];

/// The `grammar::me::tcl` package.
const GRAMMAR__ME__TCL_CMDS: &[Row] = &[
    (
        "grammar::me::tcl::ict_advance",
        Arity::exact(1),
        &["grammar::me::tcl::ict_advance message"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::ict_match_token",
        Arity::exact(2),
        &["grammar::me::tcl::ict_match_token tok message"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::ict_match_tokrange",
        Arity::exact(3),
        &["grammar::me::tcl::ict_match_tokrange tokbegin tokend message"],
        "If, and only if a token map was specified during initialization then the arguments are the numeric representation",
    ),
    (
        "grammar::me::tcl::ict_match_tokclass",
        Arity::exact(2),
        &["grammar::me::tcl::ict_match_tokclass code message"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::inc_restore",
        Arity::exact(1),
        &["grammar::me::tcl::inc_restore nt"],
        "Instead of taking a branchlabel the command returns a boolean value.",
    ),
    (
        "grammar::me::tcl::inc_save",
        Arity::exact(2),
        &["grammar::me::tcl::inc_save nt startlocation"],
        "The command takes the start location as additional argument, as it is managed on the Tcl stack, and not in the ma",
    ),
    (
        "grammar::me::tcl::iok_ok",
        Arity::exact(0),
        &["grammar::me::tcl::iok_ok"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::iok_fail",
        Arity::exact(0),
        &["grammar::me::tcl::iok_fail"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::iok_negate",
        Arity::exact(0),
        &["grammar::me::tcl::iok_negate"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::icl_get",
        Arity::exact(0),
        &["grammar::me::tcl::icl_get"],
        "This command returns the current location CL in the input.",
    ),
    (
        "grammar::me::tcl::icl_rewind",
        Arity::exact(1),
        &["grammar::me::tcl::icl_rewind oldlocation"],
        "The command takes the location as argument as it comes from the Tcl stack, not the machine state.",
    ),
    (
        "grammar::me::tcl::ier_get",
        Arity::exact(0),
        &["grammar::me::tcl::ier_get"],
        "This command returns the current error state ER.",
    ),
    (
        "grammar::me::tcl::ier_clear",
        Arity::exact(0),
        &["grammar::me::tcl::ier_clear"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::ier_nonterminal",
        Arity::exact(2),
        &["grammar::me::tcl::ier_nonterminal message location"],
        "The command takes the location as argument as it comes from the Tcl stack, not the machine state.",
    ),
    (
        "grammar::me::tcl::ier_merge",
        Arity::exact(1),
        &["grammar::me::tcl::ier_merge olderror"],
        "The command takes the second error state to merge as argument as it comes from the Tcl stack, not the machine sta",
    ),
    (
        "grammar::me::tcl::isv_clear",
        Arity::exact(0),
        &["grammar::me::tcl::isv_clear"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::isv_terminal",
        Arity::exact(0),
        &["grammar::me::tcl::isv_terminal"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::isv_nonterminal_leaf",
        Arity::exact(2),
        &["grammar::me::tcl::isv_nonterminal_leaf nt startlocation"],
        "The command takes the start location as argument as it comes from the Tcl stack, not the machine state.",
    ),
    (
        "grammar::me::tcl::isv_nonterminal_range",
        Arity::exact(2),
        &["grammar::me::tcl::isv_nonterminal_range nt startlocation"],
        "The command takes the start location as argument as it comes from the Tcl stack, not the machine state.",
    ),
    (
        "grammar::me::tcl::ias_push",
        Arity::exact(0),
        &["grammar::me::tcl::ias_push"],
        "No changes.",
    ),
    (
        "grammar::me::tcl::ias_mark",
        Arity::exact(0),
        &["grammar::me::tcl::ias_mark"],
        "This command returns a marker for the current state of the AST stack AS.",
    ),
    (
        "grammar::me::tcl::ias_pop2mark",
        Arity::exact(1),
        &["grammar::me::tcl::ias_pop2mark marker"],
        "The command takes the marker as argument as it comes from the Tcl stack, not the machine state.",
    ),
];

/// The `grammar::peg::interp` package.
const GRAMMAR__PEG__INTERP_CMDS: &[Row] = &[
    (
        "grammar::peg::interp::setup",
        Arity::exact(1),
        &["grammar::peg::interp::setup peg"],
        "This command (re)initializes the interpreter.",
    ),
    (
        "grammar::peg::interp::parse",
        Arity::exact(3),
        &["grammar::peg::interp::parse nextcmd errorvar astvar"],
        "This command interprets the loaded grammar and tries to match it against the stream of characters represented by ",
    ),
];

/// A row in the `html` package command table: name, arity, synopsis,
/// summary, return type, purity (side-effect-free), and the html-package
/// version that introduced the command (`None` = present in every modelled
/// release).
///
/// The `html` table is modelled richer than the shared 4-tuple [`Row`]
/// because the package mixes pure string transformers (`quoteFormValue`,
/// `nl2br`, `doctype`, …) with state-accumulating generators (`head`,
/// `bodyTag`, `openTag`, …) and carries a handful of version-gated commands
/// (`css`/`js`/`doctype`, new in html 1.4), and issue #811 needs every
/// command's real arity and synopsis rather than the previous partial set.
type HtmlRow = (
    &'static str,
    Arity,
    &'static [&'static str],
    &'static str,
    TclType,
    bool,
    Option<&'static str>,
);

/// Build command specs for the `html` package from its table.
fn html_rows(table: &'static [HtmlRow]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(
            |&(name, arity, synopsis, summary, return_type, pure, introduced)| CommandSpec {
                name,
                arity,
                traits: if pure { Traits::PURE } else { Traits::empty() },
                return_type: Some(return_type),
                hover: Some(HoverSnippet::brief(
                    summary,
                    synopsis,
                    "tcllib html package",
                )),
                tcllib_package: Some("html"),
                required_package: Some("html"),
                lifecycle: Lifecycle {
                    introduced,
                    ..Lifecycle::UNSPECIFIED
                },
                ..CommandSpec::DEFAULT
            },
        )
        .collect()
}

/// The `html` package (tcllib `html` 1.6). Every command mirrors the real
/// `proc ::html::…` signature in `modules/html/html.tcl`: arities and
/// synopses come straight from the definitions, and the lifecycle records the
/// html-package release that introduced the later additions
/// (`css`/`js`/`css-clear`/`js-clear`/`doctype`, all html 1.4).
const HTML_CMDS: &[HtmlRow] = &[
    (
        "html::author",
        Arity::exact(1),
        &["html::author author"],
        "Set the author name for the document, included as a comment in the head section. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::bodyTag",
        Arity::any(),
        &["html::bodyTag ?key value ...?"],
        "Generate a <body> tag, using any accumulated body tag parameters plus the given key/value pairs. Takes zero or more arguments.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::cell",
        Arity::new(2, 3),
        &["html::cell param value ?tag?"],
        "Generate a table cell (a td or th element, defaulting to td) wrapping value, with the given tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::checkbox",
        Arity::exact(2),
        &["html::checkbox name value"],
        "Generate a checkbox form element with the specified name and value.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::checkSet",
        Arity::exact(3),
        &["html::checkSet key sep list"],
        "Generate a set of checkbox form elements and associated labels from a list of value/label pairs, separated by sep.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::checkValue",
        Arity::new(1, 2),
        &["html::checkValue name ?value?"],
        "Generate the name=name value=value for a checkbox form element, marking it checked when the current form value matches (value defaults to 1).",
        TclType::String,
        false,
        None,
    ),
    (
        "html::closeTag",
        Arity::exact(0),
        &["html::closeTag"],
        "Pop a tag off the stack created by ::html::openTag and generate the corresponding close tag (e.g., </body>).",
        TclType::String,
        false,
        None,
    ),
    (
        "html::css",
        Arity::exact(1),
        &["html::css href"],
        "Register a link to an external stylesheet (href) for inclusion in the head section. Side effect only.",
        TclType::String,
        false,
        Some("1.4"),
    ),
    (
        "html::css-clear",
        Arity::exact(0),
        &["html::css-clear"],
        "Clear the accumulated set of stylesheet references registered with ::html::css.",
        TclType::String,
        false,
        Some("1.4"),
    ),
    (
        "html::default",
        Arity::new(1, 2),
        &["html::default key ?param?"],
        "Return the parameter string for a form element, merging in any default value previously registered for key.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::description",
        Arity::exact(1),
        &["html::description description"],
        "Set the description meta tag for the document head. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::doctype",
        Arity::exact(1),
        &["html::doctype id"],
        "Build the standard DOCTYPE declaration string for the well-known document-type id (e.g. HTML401, XHTML1-STRICT).",
        TclType::String,
        true,
        Some("1.4"),
    ),
    (
        "html::end",
        Arity::exact(0),
        &["html::end"],
        "Pop all open tags from the stack and generate the corresponding close HTML tags, (e.g., </body></html>).",
        TclType::String,
        false,
        None,
    ),
    (
        "html::eval",
        Arity::any(),
        &["html::eval arg ?arg ...?"],
        "Evaluate the arguments as a Tcl script in the caller's context, the way the built-in eval does; used inside html templates.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::extractParam",
        Arity::new(2, 3),
        &["html::extractParam param key ?varName?"],
        "Extract the value of key from an HTML parameter string; stores it in varName (returning 1/0) when given, otherwise returns the value.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::font",
        Arity::any(),
        &["html::font ?key value ...?"],
        "Generate a standard <font> tag from the given key/value parameter pairs. Takes zero or more arguments.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::for",
        Arity::exact(4),
        &["html::for start test next body"],
        "Like the built-in Tcl for control structure, but the body is substituted (not evaluated as a script) and the result appended to the page.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::foreach",
        Arity::at_least(2),
        &["html::foreach varlist list ?varlist list ...? body"],
        "Like the built-in Tcl foreach control structure, but the body is substituted (not evaluated as a script) and the result appended to the page.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::formValue",
        Arity::new(1, 2),
        &["html::formValue name ?defvalue?"],
        "Return the current value for the named form element (from the query data), or defvalue if it is not set.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::getFormInfo",
        Arity::any(),
        &["html::getFormInfo ?pattern ...?"],
        "Generate hidden fields capturing the current form values whose names match the given patterns (all values when no pattern is given).",
        TclType::String,
        false,
        None,
    ),
    (
        "html::getTitle",
        Arity::exact(0),
        &["html::getTitle"],
        "Return the title string, without the surrounding title tag, set with a previous call to ::html::title.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::h",
        Arity::new(2, 3),
        &["html::h level string ?param?"],
        "Generate a heading element of the given level (1-6) wrapping string, with optional tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::h1",
        Arity::new(1, 2),
        &["html::h1 string ?param?"],
        "Generate an <h1> heading element wrapping string, with optional tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::h2",
        Arity::new(1, 2),
        &["html::h2 string ?param?"],
        "Generate an <h2> heading element wrapping string, with optional tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::h3",
        Arity::new(1, 2),
        &["html::h3 string ?param?"],
        "Generate an <h3> heading element wrapping string, with optional tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::h4",
        Arity::new(1, 2),
        &["html::h4 string ?param?"],
        "Generate an <h4> heading element wrapping string, with optional tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::h5",
        Arity::new(1, 2),
        &["html::h5 string ?param?"],
        "Generate an <h5> heading element wrapping string, with optional tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::h6",
        Arity::new(1, 2),
        &["html::h6 string ?param?"],
        "Generate an <h6> heading element wrapping string, with optional tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::hdrRow",
        Arity::any(),
        &["html::hdrRow ?value ...?"],
        "Generate a table header row, including tr and th tags, from the given values. Takes zero or more arguments.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::head",
        Arity::exact(1),
        &["html::head title"],
        "Generate the head section that includes the page title and any registered meta/link tags.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::headTag",
        Arity::exact(1),
        &["html::headTag string"],
        "Save a tag for inclusion in the head section generated by ::html::head.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::if",
        Arity::at_least(2),
        &["html::if test body ?elseif test body ...? ?else body?"],
        "Like the built-in Tcl if control structure, but each body is substituted (not evaluated as a script) and the result appended to the page.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::init",
        Arity::new(0, 1),
        &["html::init ?nvlist?"],
        "(Re)initialise the html package's page state, optionally seeding the default parameter array from a name/value list.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::js",
        Arity::exact(1),
        &["html::js href"],
        "Register a link to an external JavaScript file (href) for inclusion in the head section. Side effect only.",
        TclType::String,
        false,
        Some("1.4"),
    ),
    (
        "html::js-clear",
        Arity::exact(0),
        &["html::js-clear"],
        "Clear the accumulated set of JavaScript references registered with ::html::js.",
        TclType::String,
        false,
        Some("1.4"),
    ),
    (
        "html::keywords",
        Arity::any(),
        &["html::keywords ?keyword ...?"],
        "Set the keywords meta tag for the document head. Side effect only. Takes zero or more arguments.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::mailto",
        Arity::new(1, 2),
        &["html::mailto email ?subject?"],
        "Generate a mailto: hyperlink for the given email address, with an optional pre-filled subject.",
        TclType::String,
        true,
        None,
    ),
    (
        "html::meta",
        Arity::any(),
        &["html::meta ?key value ...?"],
        "Compatibility alias for html::meta_name: register a name/content meta tag for the head section. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::meta_charset",
        Arity::exact(1),
        &["html::meta_charset charset"],
        "Register a charset meta tag for the head section. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::meta_equiv",
        Arity::any(),
        &["html::meta_equiv ?key value ...?"],
        "Register an http-equiv meta tag for the head section. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::meta_name",
        Arity::any(),
        &["html::meta_name ?key value ...?"],
        "Register a name/content meta tag for the head section. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::minorList",
        Arity::new(1, 2),
        &["html::minorList list ?ordered?"],
        "Generate an unordered (or, when ordered is false, ordered) list of hyperlinks from a list of label/URL pairs.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::minorMenu",
        Arity::new(1, 2),
        &["html::minorMenu list ?sep?"],
        "Generate a horizontal menu of hyperlinks from a list of label/URL pairs, joined by sep (default \" | \").",
        TclType::String,
        false,
        None,
    ),
    (
        "html::nl2br",
        Arity::exact(1),
        &["html::nl2br string"],
        "Replace all line-endings in the string with a <br> tag and return the modified text.",
        TclType::String,
        true,
        None,
    ),
    (
        "html::openTag",
        Arity::new(1, 2),
        &["html::openTag tag ?param?"],
        "Push tag onto the open-tag stack and generate its opening tag with the given parameters; balanced by ::html::closeTag.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::paramRow",
        Arity::new(1, 3),
        &["html::paramRow list ?rparam? ?cparam?"],
        "Generate a table row from a list of cell values, with optional row (rparam) and cell (cparam) tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::passwordInput",
        Arity::new(0, 1),
        &["html::passwordInput ?name?"],
        "Generate an input form tag with type password (name defaults to \"password\").",
        TclType::String,
        false,
        None,
    ),
    (
        "html::passwordInputRow",
        Arity::new(1, 2),
        &["html::passwordInputRow label ?name?"],
        "Generate a password input form tag formatted into a table row with an associated label (name defaults to \"password\").",
        TclType::String,
        false,
        None,
    ),
    (
        "html::quoteFormValue",
        Arity::exact(1),
        &["html::quoteFormValue value"],
        "Quote special characters in value by replacing them with HTML entities for quotes, ampersand, and angle brackets.",
        TclType::String,
        true,
        None,
    ),
    (
        "html::radioSet",
        Arity::new(3, 4),
        &["html::radioSet key sep list ?defaultSelection?"],
        "Generate a set of input tags of type radio and an associated text label from a list of value/label pairs.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::radioValue",
        Arity::new(2, 3),
        &["html::radioValue name value ?defaultSelection?"],
        "Generate the name=name value=value for a radio form element, marking it checked when it matches the default selection.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::refresh",
        Arity::new(1, 2),
        &["html::refresh content ?url?"],
        "Register a refresh meta tag that reloads after content seconds, optionally redirecting to url. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::row",
        Arity::any(),
        &["html::row ?value ...?"],
        "Generate a table row, including tr and td tags, from the given values. Takes zero or more arguments.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::select",
        Arity::new(3, 4),
        &["html::select name param choices ?current?"],
        "Generate a <select> form element from a list of value/label choices, marking current as selected.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::selectPlain",
        Arity::new(3, 4),
        &["html::selectPlain name param choices ?current?"],
        "Generate a <select> form element from a plain list of choices (used as both value and label), marking current as selected.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::set",
        Arity::exact(2),
        &["html::set var val"],
        "Like the built-in Tcl set command: assign val to the variable var in the caller's context; used inside html templates.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::submit",
        Arity::new(1, 3),
        &["html::submit label ?name? ?title?"],
        "Generate a submit button with the given label (name defaults to \"submit\", with an optional title attribute).",
        TclType::String,
        false,
        None,
    ),
    (
        "html::tableFromArray",
        Arity::new(1, 3),
        &["html::tableFromArray arrname ?param? ?pat?"],
        "Generate a two-column table of the key/value pairs of the named array whose keys match pat, with optional table parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::tableFromList",
        Arity::new(1, 2),
        &["html::tableFromList querylist ?param?"],
        "Generate a two-column table from a name/value list, with optional table parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::tagParam",
        Arity::new(1, 2),
        &["html::tagParam tag ?param?"],
        "Return the parameter string for tag, merging in any default parameters previously registered for that tag.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::textarea",
        Arity::new(1, 3),
        &["html::textarea name ?param? ?current?"],
        "Generate a <textarea> form element with the given name, tag parameters, and current contents.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::textInput",
        Arity::at_least(1),
        &["html::textInput name ?value? ?key value ...?"],
        "Generate an input form tag with type text, with an optional initial value and extra tag parameters.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::textInputRow",
        Arity::at_least(2),
        &["html::textInputRow label name ?value? ?key value ...?"],
        "Generate a text input form tag formatted into a table row with an associated label.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::title",
        Arity::exact(1),
        &["html::title title"],
        "Set the document title, generated inside the head section by ::html::head. Side effect only.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::urlParent",
        Arity::exact(1),
        &["html::urlParent url"],
        "Return the parent URL of url (the URL with its last path component removed).",
        TclType::String,
        true,
        None,
    ),
    (
        "html::varEmpty",
        Arity::exact(1),
        &["html::varEmpty name"],
        "Return 1 if the named variable either does not exist or has the empty string for its value, and 0 otherwise.",
        TclType::Boolean,
        false,
        None,
    ),
    (
        "html::while",
        Arity::exact(2),
        &["html::while test body"],
        "Like the built-in Tcl while control structure, but the body is substituted (not evaluated as a script) and the result appended to the page.",
        TclType::String,
        false,
        None,
    ),
    (
        "html::wrapTag",
        Arity::at_least(1),
        &["html::wrapTag tag ?text? ?key value ...?"],
        "Generate a balanced element: an opening tag (with the given parameters) around text and the matching close tag.",
        TclType::String,
        false,
        None,
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

/// The `ip` package.
const IP_CMDS: &[Row] = &[
    (
        "ip::distance",
        Arity::exact(2),
        &["ip::distance ipaddr1 ipaddr2"],
        "This command computes the (integer) distance from IPv4 address ipaddr1 to IPv4 address ipaddr2, i.e.",
    ),
    (
        "ip::prefixToNative",
        Arity::exact(1),
        &["ip::prefixToNative prefix"],
        "This command converts the string prefix from dotted form (<ipaddr>/<mask> format) to native (hex) form.",
    ),
    (
        "ip::toInteger",
        Arity::exact(1),
        &["ip::toInteger ipaddr"],
        "This command converts a dotted form ip into an integer number.",
    ),
    (
        "ip::toHex",
        Arity::exact(1),
        &["ip::toHex ipaddr"],
        "This command converts dotted form ip into a hexadecimal number.",
    ),
    (
        "ip::maskToInt",
        Arity::exact(1),
        &["ip::maskToInt ipmask"],
        "This command convert an ipmask in either dotted (255.255.255.0) form or mask length form (24) into an integer num",
    ),
    (
        "ip::ipToLayer2Multicast",
        Arity::exact(1),
        &["ip::ipToLayer2Multicast ipaddr"],
        "This command an converts ipv4 address in dotted form into a layer 2 multicast address, also in dotted form.",
    ),
    (
        "ip::reduceToAggregates",
        Arity::exact(1),
        &["ip::reduceToAggregates prefixlist"],
        "This command finds nets that overlap and filters out the more specific nets.",
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

/// The `md5` package.
const MD5_CMDS: &[Row] = &[
    (
        "md5::MD5Init",
        Arity::exact(0),
        &["md5::MD5Init"],
        "Begins a new MD5 hash.",
    ),
    (
        "md5::MD5Update",
        Arity::exact(2),
        &["md5::MD5Update token data"],
        "Add data to the hash identified by token.",
    ),
    (
        "md5::MD5Final",
        Arity::exact(1),
        &["md5::MD5Final token"],
        "Returns the hash value and releases any resources held by this token.",
    ),
    (
        "md5::HMACInit",
        Arity::exact(1),
        &["md5::HMACInit key"],
        "This is equivalent to the ::md5::MD5Init command except that it requires the key that will be included in the HMA",
    ),
    (
        "md5::HMACUpdate",
        Arity::exact(2),
        &["md5::HMACUpdate token data"],
        "tcllib command.",
    ),
    (
        "md5::HMACFinal",
        Arity::exact(1),
        &["md5::HMACFinal token"],
        "These commands are identical to the MD5 equivalent commands.",
    ),
];

/// The `mkdoc` package.
const MKDOC_CMDS: &[Row] = &[(
    "mkdoc::run",
    Arity::exact(1),
    &["mkdoc::run infile"],
    "The command reads the specified infile, extracts the embedded documentation, and then executes the contents of th",
)];

/// The `multiplexer` package.
const MULTIPLEXER_CMDS: &[Row] = &[(
    "multiplexer::create",
    Arity::exact(0),
    &["multiplexer::create"],
    "The create command creates a new multiplexer 'instance'.",
)];

/// The `nameserv` package.
const NAMESERV_CMDS: &[Row] = &[
    (
        "nameserv::bind",
        Arity::exact(2),
        &["nameserv::bind name data"],
        "The caller of this command registers the given name as its name in the configured name service, and additionally ",
    ),
    (
        "nameserv::release",
        Arity::exact(0),
        &["nameserv::release"],
        "Invoking this command releases all names (and their data) registered by all previous calls to ::nameserv::bind of",
    ),
    (
        "nameserv::protocol",
        Arity::exact(0),
        &["nameserv::protocol"],
        "This command returns the highest version of the name service protocol supported by the package.",
    ),
    (
        "nameserv::server_protocol",
        Arity::exact(0),
        &["nameserv::server_protocol"],
        "This command returns the highest version of the name service protocol supported by the name service the client is",
    ),
    (
        "nameserv::server_features",
        Arity::exact(0),
        &["nameserv::server_features"],
        "This command returns a list containing the names of the features of the name service protocol which are supported",
    ),
    (
        "nameserv::cget",
        Arity::at_least(0),
        &["nameserv::cget -option"],
        "This command returns the currently configured value for the specified -option.",
    ),
    (
        "nameserv::configure",
        Arity::exact(0),
        &["nameserv::configure"],
        "In this form the command returns a dictionary of all supported options, and their current values.",
    ),
];

/// The `nameserv::common` package.
const NAMESERV__COMMON_CMDS: &[Row] = &[(
    "nameserv::common::port",
    Arity::exact(0),
    &["nameserv::common::port"],
    "The result returned by the command is the id of the default TCP/IP port a nameservice server will listen on, and ",
)];

/// The `nameserv::server` package.
const NAMESERV__SERVER_CMDS: &[Row] = &[
    (
        "nameserv::server::start",
        Arity::exact(0),
        &["nameserv::server::start"],
        "This command starts the server and causes it to listen on the configured port.",
    ),
    (
        "nameserv::server::stop",
        Arity::exact(0),
        &["nameserv::server::stop"],
        "Invoking this command stops the server and releases all information it had.",
    ),
    (
        "nameserv::server::cget",
        Arity::at_least(0),
        &["nameserv::server::cget -option"],
        "This command returns the currently configured value for the specified -option.",
    ),
    (
        "nameserv::server::configure",
        Arity::exact(0),
        &["nameserv::server::configure"],
        "In this form the command returns a dictionary of all supported options, and their current values.",
    ),
];

/// The `nettool` package.
const NETTOOL_CMDS: &[Row] = &[
    (
        "cat",
        Arity::exact(1),
        &["cat filename"],
        "Dump the contents of a file as a result.",
    ),
    (
        "nettool::allocate_port",
        Arity::exact(1),
        &["nettool::allocate_port startingport"],
        "Attempt to allocate startingport, or, if busy, advance the port number sequentially until a free port is found, a",
    ),
    (
        "nettool::arp_table",
        Arity::exact(0),
        &["nettool::arp_table"],
        "Dump the contents of this computer's Address Resolution Protocol (ARP) table.",
    ),
    (
        "nettool::broadcast_list",
        Arity::exact(0),
        &["nettool::broadcast_list"],
        "Returns a list of broadcast addresses (suitable for UDP multicast) that this computer is associated with.",
    ),
    (
        "nettool::cpuinfo",
        Arity::at_least(1),
        &["nettool::cpuinfo args"],
        "If no arguments are given, return a key/value list describing the CPU of the present machine.",
    ),
    (
        "nettool::find_port",
        Arity::exact(1),
        &["nettool::find_port startingport"],
        "Return startingport if it is available, or the next free port after startingport.",
    ),
    (
        "nettool::network_list",
        Arity::exact(0),
        &["nettool::network_list"],
        "Return a list of networks associated with this computer.",
    ),
    (
        "nettool::status",
        Arity::exact(0),
        &["nettool::status"],
        "Return a key/value list describing the status of the computer.",
    ),
    (
        "nettool::user_data_root",
        Arity::exact(1),
        &["nettool::user_data_root appname"],
        "Return a fully qualified path to a folder where appname should store it's data.",
    ),
];

/// The `page::pluginmgr` package.
const PAGE__PLUGINMGR_CMDS: &[Row] = &[
    (
        "page::pluginmgr::reportvia",
        Arity::exact(1),
        &["page::pluginmgr::reportvia cmd"],
        "This command defines the callback command used by ::page::pluginmgr::report (see below) to report input errors an",
    ),
    (
        "page::pluginmgr::log",
        Arity::exact(1),
        &["page::pluginmgr::log cmd"],
        "This command defines a log callback command to be used by loaded plugins for the reporting of internal errors, wa",
    ),
    (
        "page::pluginmgr::configuration",
        Arity::exact(1),
        &["page::pluginmgr::configuration name"],
        "This command loads the named configuration plugin, retrieves the options encoded in it, and then immediately unlo",
    ),
    (
        "page::pluginmgr::reader",
        Arity::exact(1),
        &["page::pluginmgr::reader name"],
        "This command loads the named reader plugin and initializes it.",
    ),
    (
        "page::pluginmgr::rconfigure",
        Arity::exact(1),
        &["page::pluginmgr::rconfigure dict"],
        "This commands configures the loaded reader plugin.",
    ),
    (
        "page::pluginmgr::rtimeable",
        Arity::exact(0),
        &["page::pluginmgr::rtimeable"],
        "This commands checks if the loaded reader plugin is able to collect timing statistics.",
    ),
    (
        "page::pluginmgr::rtime",
        Arity::exact(0),
        &["page::pluginmgr::rtime"],
        "This command activates the collection of timing statistics in the loaded reader plugin.",
    ),
    (
        "page::pluginmgr::rgettime",
        Arity::exact(0),
        &["page::pluginmgr::rgettime"],
        "This command retrieves the collected timing statistics of the loaded reader plugin after it was executed.",
    ),
    (
        "page::pluginmgr::rhelp",
        Arity::exact(0),
        &["page::pluginmgr::rhelp"],
        "This command retrieves the help string of the loaded reader plugin.",
    ),
    (
        "page::pluginmgr::rlabel",
        Arity::exact(0),
        &["page::pluginmgr::rlabel"],
        "This command retrieves the human-readable name of the loaded reader plugin.",
    ),
    (
        "page::pluginmgr::writer",
        Arity::exact(1),
        &["page::pluginmgr::writer name"],
        "This command loads the named writer plugin and initializes it.",
    ),
    (
        "page::pluginmgr::wconfigure",
        Arity::exact(1),
        &["page::pluginmgr::wconfigure dict"],
        "This commands configures the loaded writer plugin.",
    ),
    (
        "page::pluginmgr::wtimeable",
        Arity::exact(0),
        &["page::pluginmgr::wtimeable"],
        "This commands checks if the loaded writer plugin is able to measure execution times.",
    ),
    (
        "page::pluginmgr::wtime",
        Arity::exact(0),
        &["page::pluginmgr::wtime"],
        "This command activates the collection of timing statistics in the loaded writer plugin.",
    ),
    (
        "page::pluginmgr::wgettime",
        Arity::exact(0),
        &["page::pluginmgr::wgettime"],
        "This command retrieves the collected timing statistics of the loaded writer plugin after it was executed.",
    ),
    (
        "page::pluginmgr::whelp",
        Arity::exact(0),
        &["page::pluginmgr::whelp"],
        "This command retrieves the help string of the loaded writer plugin.",
    ),
    (
        "page::pluginmgr::wlabel",
        Arity::exact(0),
        &["page::pluginmgr::wlabel"],
        "This command retrieves the human-readable name of the loaded writer plugin.",
    ),
    (
        "page::pluginmgr::write",
        Arity::exact(2),
        &["page::pluginmgr::write chan data"],
        "The loaded writer plugin is invoked to generate the output.",
    ),
    (
        "page::pluginmgr::transform",
        Arity::exact(1),
        &["page::pluginmgr::transform name"],
        "This command loads the named transformation plugin and initializes it.",
    ),
    (
        "page::pluginmgr::tconfigure",
        Arity::exact(2),
        &["page::pluginmgr::tconfigure id dict"],
        "This commands configures the identified transformation plugin.",
    ),
    (
        "page::pluginmgr::ttimeable",
        Arity::exact(1),
        &["page::pluginmgr::ttimeable id"],
        "This commands checks if the identified transformation plugin is able to collect timing statistics.",
    ),
    (
        "page::pluginmgr::ttime",
        Arity::exact(1),
        &["page::pluginmgr::ttime id"],
        "This command activates the collection of timing statistics in the identified transformation plugin.",
    ),
    (
        "page::pluginmgr::tgettime",
        Arity::exact(1),
        &["page::pluginmgr::tgettime id"],
        "This command retrieves the collected timing statistics of the identified transformation plugin after it was execu",
    ),
    (
        "page::pluginmgr::thelp",
        Arity::exact(1),
        &["page::pluginmgr::thelp id"],
        "This command retrieves the help string of the identified transformation plugin.",
    ),
    (
        "page::pluginmgr::tlabel",
        Arity::exact(1),
        &["page::pluginmgr::tlabel id"],
        "This command retrieves the human-readable name of the identified transformation plugin.",
    ),
    (
        "page::pluginmgr::transform_do",
        Arity::exact(2),
        &["page::pluginmgr::transform_do id data"],
        "The identified transformation plugin is invoked to process the specified data.",
    ),
];

/// The `page::util` package.
const PAGE__UTIL_CMDS: &[Row] = &[(
    "page::util::flow",
    Arity::exact(4),
    &["page::util::flow start flowvar nodevar script"],
    "This command contains the core logic to drive the walking of an arbitrary data structure which can partitioned in",
)];

/// The `page::util::norm` package.
const PAGE__UTIL__NORM_CMDS: &[Row] = &[
    (
        "page::util::norm::lemon",
        Arity::exact(1),
        &["page::util::norm::lemon tree"],
        "This command assumes the tree object contains for a lemon grammar.",
    ),
    (
        "page::util::norm::peg",
        Arity::exact(1),
        &["page::util::norm::peg tree"],
        "This command assumes the tree object contains for a parsing expression grammar.",
    ),
];

/// The `page::util::peg` package.
const PAGE__UTIL__PEG_CMDS: &[Row] = &[
    (
        "page::util::peg::symbolNodeOf",
        Arity::exact(2),
        &["page::util::peg::symbolNodeOf tree node"],
        "Given an arbitrary expression node in the AST tree it determines the node (itself or an ancestor) containing the ",
    ),
    (
        "page::util::peg::symbolOf",
        Arity::exact(2),
        &["page::util::peg::symbolOf tree node"],
        "As ::page::util::peg::symbolNodeOf, but returns the symbol name instead of the node.",
    ),
    (
        "page::util::peg::updateUndefinedDueRemoval",
        Arity::exact(1),
        &["page::util::peg::updateUndefinedDueRemoval tree"],
        "The removal of nodes in the AST tree can cause symbols to lose one or more users.",
    ),
    (
        "page::util::peg::flatten",
        Arity::exact(2),
        &["page::util::peg::flatten treequery tree"],
        "This commands flattens nested sequence and choice operators in the AST tree, re-using the treeql object treequery",
    ),
    (
        "page::util::peg::getWarnings",
        Arity::exact(1),
        &["page::util::peg::getWarnings tree"],
        "This command looks at the attributes of the AST tree for problems with the grammar and issues warnings.",
    ),
    (
        "page::util::peg::printWarnings",
        Arity::exact(1),
        &["page::util::peg::printWarnings msg"],
        "The argument of the command is a dictionary mapping nonterminal names to their associated warnings, as generated ",
    ),
    (
        "page::util::peg::peOf",
        Arity::exact(2),
        &["page::util::peg::peOf tree eroot"],
        "This command converts the parsing expression starting at the node eroot in the AST tree into a nested list.",
    ),
    (
        "page::util::peg::printTclExpr",
        Arity::exact(1),
        &["page::util::peg::printTclExpr pe"],
        "This command converts the parsing expression contained in the nested list pe into a Tcl string which can be place",
    ),
];

/// The `page::util::quote` package.
const PAGE__UTIL__QUOTE_CMDS: &[Row] = &[(
    "page::util::quote::unquote",
    Arity::exact(1),
    &["page::util::quote::unquote char"],
    "A character, as stored in an abstract syntax tree by a PEG processor (See the packages grammar::peg::interpreter,",
)];

/// The `picoirc` package.
const PICOIRC_CMDS: &[Row] = &[(
    "picoirc::post",
    Arity::exact(3),
    &["picoirc::post context channel message"],
    "This should be called to process user input and send it to the server.",
)];

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

/// The `pt` package.
const PT_CMDS: &[Row] = &[(
    "pt::rde",
    Arity::exact(1),
    &["pt::rde objectName"],
    "The command creates a new runtime object for a recursive descent parser with backtracking and returns the fully q",
)];

/// The `pt::peg` package.
const PT__PEG_CMDS: &[Row] = &[
    (
        "pt::peg::export",
        Arity::exact(1),
        &["pt::peg::export objectName"],
        "This command creates a new export manager object with an associated Tcl command whose name is objectName.",
    ),
    (
        "pt::peg::import",
        Arity::exact(1),
        &["pt::peg::import objectName"],
        "This command creates a new import manager object with an associated Tcl command whose name is objectName.",
    ),
    (
        "pt::peg::interp",
        Arity::exact(2),
        &["pt::peg::interp objectName grammar"],
        "The command creates a new parser object and returns the fully qualified name of the object command as its result.",
    ),
];

/// The `pt_export_api` package.
const PT_EXPORT_API_CMDS: &[Row] = &[(
    "export",
    Arity::exact(2),
    &["export serial configuration"],
    "This command has to accept the canonical serialization of a parsing expression grammar and the configuration for ",
)];

/// The `pt_import_api` package.
const PT_IMPORT_API_CMDS: &[Row] = &[(
    "import",
    Arity::exact(1),
    &["import text"],
    "This command has to accept the a text containing a parsing expression grammar in some format.",
)];

// The `report` package's commands all carry dedicated specs (arg roles, hover
// snippets, the `report::report` object-method class, and the
// `report::defstyle` style-script scoped environment) that the flat `Row` table
// cannot express — see `report__report.rs`, `report__defstyle.rs`,
// `report__rmstyle.rs`, `report__stylearguments.rs`, `report__stylebody.rs`,
// and `report__styles.rs`.

/// The `sha1` package.
const SHA1_CMDS: &[Row] = &[
    (
        "sha1::hmac",
        Arity::exact(2),
        &["sha1::hmac key string"],
        "tcllib command.",
    ),
    (
        "sha1::SHA1Init",
        Arity::exact(0),
        &["sha1::SHA1Init"],
        "Begins a new SHA1 hash.",
    ),
    (
        "sha1::SHA1Update",
        Arity::exact(2),
        &["sha1::SHA1Update token data"],
        "Add data to the hash identified by token.",
    ),
    (
        "sha1::SHA1Final",
        Arity::exact(1),
        &["sha1::SHA1Final token"],
        "Returns the hash value and releases any resources held by this token.",
    ),
    (
        "sha1::HMACInit",
        Arity::exact(1),
        &["sha1::HMACInit key"],
        "This is equivalent to the ::sha1::SHA1Init command except that it requires the key that will be included in the H",
    ),
    (
        "sha1::HMACUpdate",
        Arity::exact(2),
        &["sha1::HMACUpdate token data"],
        "tcllib command.",
    ),
    (
        "sha1::HMACFinal",
        Arity::exact(1),
        &["sha1::HMACFinal token"],
        "These commands are identical to the SHA1 equivalent commands.",
    ),
];

/// The `sha2` package.
const SHA2_CMDS: &[Row] = &[
    (
        "sha2::hmac",
        Arity::exact(2),
        &["sha2::hmac key string"],
        "tcllib command.",
    ),
    (
        "sha2::SHA256Init",
        Arity::exact(0),
        &["sha2::SHA256Init"],
        "tcllib command.",
    ),
    (
        "sha2::SHA224Init",
        Arity::exact(0),
        &["sha2::SHA224Init"],
        "Begins a new SHA256/SHA224 hash.",
    ),
    (
        "sha2::SHA256Update",
        Arity::exact(2),
        &["sha2::SHA256Update token data"],
        "Add data to the hash identified by token.",
    ),
    (
        "sha2::SHA256Final",
        Arity::exact(1),
        &["sha2::SHA256Final token"],
        "tcllib command.",
    ),
    (
        "sha2::SHA224Final",
        Arity::exact(1),
        &["sha2::SHA224Final token"],
        "Returns the hash value and releases any resources held by this token.",
    ),
    (
        "sha2::HMACInit",
        Arity::exact(1),
        &["sha2::HMACInit key"],
        "This is equivalent to the ::sha2::SHA256Init command except that it requires the key that will be included in the",
    ),
    (
        "sha2::HMACUpdate",
        Arity::exact(2),
        &["sha2::HMACUpdate token data"],
        "tcllib command.",
    ),
    (
        "sha2::HMACFinal",
        Arity::exact(1),
        &["sha2::HMACFinal token"],
        "These commands are identical to the SHA256 equivalent commands.",
    ),
];

/// The `simulation::annealing` package.
const SIMULATION__ANNEALING_CMDS: &[Row] = &[
    (
        "simulation::annealing::getOption",
        Arity::exact(1),
        &["simulation::annealing::getOption keyword"],
        "Get the value of an option given as part of the findMinimum command.",
    ),
    (
        "simulation::annealing::hasOption",
        Arity::exact(1),
        &["simulation::annealing::hasOption keyword"],
        "Returns 1 if the option is available, 0 if not.",
    ),
    (
        "simulation::annealing::setOption",
        Arity::exact(2),
        &["simulation::annealing::setOption keyword value"],
        "Set the value of the given option.",
    ),
    (
        "simulation::annealing::findMinimum",
        Arity::at_least(1),
        &["simulation::annealing::findMinimum args"],
        "Find the minimum of a function using simulated annealing.",
    ),
    (
        "simulation::annealing::findCombinatorialMinimum",
        Arity::at_least(1),
        &["simulation::annealing::findCombinatorialMinimum args"],
        "Find the minimum of a function of discrete variables using simulated annealing.",
    ),
];

/// The `simulation::montecarlo` package.
const SIMULATION__MONTECARLO_CMDS: &[Row] = &[
    (
        "simulation::montecarlo::getOption",
        Arity::exact(1),
        &["simulation::montecarlo::getOption keyword"],
        "Get the value of an option given as part of the singleExperiment command.",
    ),
    (
        "simulation::montecarlo::hasOption",
        Arity::exact(1),
        &["simulation::montecarlo::hasOption keyword"],
        "Returns 1 if the option is available, 0 if not.",
    ),
    (
        "simulation::montecarlo::setOption",
        Arity::exact(2),
        &["simulation::montecarlo::setOption keyword value"],
        "Set the value of the given option.",
    ),
    (
        "simulation::montecarlo::setTrialResult",
        Arity::exact(1),
        &["simulation::montecarlo::setTrialResult values"],
        "Store the results of the trial for later analysis List of values to be stored.",
    ),
    (
        "simulation::montecarlo::setExpResult",
        Arity::exact(1),
        &["simulation::montecarlo::setExpResult values"],
        "Set the results of the entire experiment (typically used in the final phase).",
    ),
    (
        "simulation::montecarlo::getTrialResults",
        Arity::exact(0),
        &["simulation::montecarlo::getTrialResults"],
        "Get the results of all individual trials for analysis (typically used in the final phase or after completion of t",
    ),
    (
        "simulation::montecarlo::getExpResult",
        Arity::exact(0),
        &["simulation::montecarlo::getExpResult"],
        "Get the results of the entire experiment (typically used in the final phase or even after completion of the singl",
    ),
    (
        "simulation::montecarlo::transposeData",
        Arity::exact(1),
        &["simulation::montecarlo::transposeData values"],
        "Interchange columns and rows of a list of lists and return the result.",
    ),
    (
        "simulation::montecarlo::integral2D",
        Arity::at_least(1),
        &["simulation::montecarlo::integral2D ..."],
        "Integrate a function over a two-dimensional region using a Monte Carlo approach.",
    ),
    (
        "simulation::montecarlo::singleExperiment",
        Arity::at_least(1),
        &["simulation::montecarlo::singleExperiment args"],
        "Iterate code over a number of trials and store the results.",
    ),
];

/// The `simulation::random` package.
const SIMULATION__RANDOM_CMDS: &[Row] = &[
    (
        "simulation::random::prng_Bernoulli",
        Arity::exact(1),
        &["simulation::random::prng_Bernoulli p"],
        "Create a command (PRNG) that generates numbers with a Bernoulli distribution: the value is either 1 or 0, with a ",
    ),
    (
        "simulation::random::prng_Discrete",
        Arity::exact(1),
        &["simulation::random::prng_Discrete n"],
        "Create a command (PRNG) that generates numbers 0 to n-1 with equal probability.",
    ),
    (
        "simulation::random::prng_Poisson",
        Arity::exact(1),
        &["simulation::random::prng_Poisson lambda"],
        "Create a command (PRNG) that generates numbers according to the Poisson distribution.",
    ),
    (
        "simulation::random::prng_Uniform",
        Arity::exact(2),
        &["simulation::random::prng_Uniform min max"],
        "Create a command (PRNG) that generates uniformly distributed numbers between min and max.",
    ),
    (
        "simulation::random::prng_Triangular",
        Arity::exact(2),
        &["simulation::random::prng_Triangular min max"],
        "Create a command (PRNG) that generates triangularly distributed numbers between min and max.",
    ),
    (
        "simulation::random::prng_SymmTriangular",
        Arity::exact(2),
        &["simulation::random::prng_SymmTriangular min max"],
        "Create a command (PRNG) that generates numbers distributed according to a symmetric triangle around the mean of m",
    ),
    (
        "simulation::random::prng_Exponential",
        Arity::exact(2),
        &["simulation::random::prng_Exponential min mean"],
        "Create a command (PRNG) that generates exponentially distributed numbers with a given minimum value and a given m",
    ),
    (
        "simulation::random::prng_Normal",
        Arity::exact(2),
        &["simulation::random::prng_Normal mean stdev"],
        "Create a command (PRNG) that generates normally distributed numbers with a given mean value and a given standard ",
    ),
    (
        "simulation::random::prng_Pareto",
        Arity::exact(2),
        &["simulation::random::prng_Pareto min steep"],
        "Create a command (PRNG) that generates numbers distributed according to Pareto with a given minimum value and a g",
    ),
    (
        "simulation::random::prng_Gumbel",
        Arity::exact(2),
        &["simulation::random::prng_Gumbel min f"],
        "Create a command (PRNG) that generates numbers distributed according to Gumbel with a given minimum value and a g",
    ),
    (
        "simulation::random::prng_chiSquared",
        Arity::exact(1),
        &["simulation::random::prng_chiSquared df"],
        "Create a command (PRNG) that generates numbers distributed according to the chi-squared distribution with df degr",
    ),
    (
        "simulation::random::prng_Disk",
        Arity::exact(1),
        &["simulation::random::prng_Disk rad"],
        "Create a command (PRNG) that generates (x,y)-coordinates for points uniformly spread over a disk of given radius.",
    ),
    (
        "simulation::random::prng_Sphere",
        Arity::exact(1),
        &["simulation::random::prng_Sphere rad"],
        "Create a command (PRNG) that generates (x,y,z)-coordinates for points uniformly spread over the surface of a sphe",
    ),
    (
        "simulation::random::prng_Ball",
        Arity::exact(1),
        &["simulation::random::prng_Ball rad"],
        "Create a command (PRNG) that generates (x,y,z)-coordinates for points uniformly spread within a ball of given rad",
    ),
    (
        "simulation::random::prng_Rectangle",
        Arity::exact(2),
        &["simulation::random::prng_Rectangle length width"],
        "Create a command (PRNG) that generates (x,y)-coordinates for points uniformly spread over a rectangle.",
    ),
    (
        "simulation::random::prng_Block",
        Arity::exact(3),
        &["simulation::random::prng_Block length width depth"],
        "Create a command (PRNG) that generates (x,y,z)-coordinates for points uniformly spread over a block Length of the",
    ),
];

/// The `smtpd` package.
const SMTPD_CMDS: &[Row] = &[(
    "smtpd::stop",
    Arity::exact(0),
    &["smtpd::stop"],
    "Halt the server and release the listening socket.",
)];

/// The `struct::tree` package.
const STRUCT__TREE_CMDS: &[Row] = &[(
    "struct::tree::prune",
    Arity::exact(0),
    &["struct::tree::prune"],
    "This command is provided outside of the tree methods, as it is not a tree method per se.",
)];

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

/// The `term::ansi::code::attr` package.
const TERM__ANSI__CODE__ATTR_CMDS: &[Row] = &[(
    "term::ansi::code::attr::names",
    Arity::exact(0),
    &["term::ansi::code::attr::names"],
    "This command is for introspection.",
)];

/// The `term::ansi::code::ctrl` package.
const TERM__ANSI__CODE__CTRL_CMDS: &[Row] = &[
    (
        "term::ansi::code::ctrl::names",
        Arity::exact(0),
        &["term::ansi::code::ctrl::names"],
        "This command is for introspection.",
    ),
    (
        "term::ansi::code::ctrl::skd",
        Arity::exact(2),
        &["term::ansi::code::ctrl::skd code str"],
        "Set Key Definition.",
    ),
    (
        "term::ansi::code::ctrl::title",
        Arity::exact(1),
        &["term::ansi::code::ctrl::title str"],
        "Set the terminal title.",
    ),
    (
        "term::ansi::code::ctrl::gron",
        Arity::exact(0),
        &["term::ansi::code::ctrl::gron"],
        "Switch to character/box graphics.",
    ),
    (
        "term::ansi::code::ctrl::groff",
        Arity::exact(0),
        &["term::ansi::code::ctrl::groff"],
        "Switch to regular characters.",
    ),
    (
        "term::ansi::code::ctrl::groptim",
        Arity::exact(1),
        &["term::ansi::code::ctrl::groptim str"],
        "Optimize character graphics.",
    ),
    (
        "term::ansi::code::ctrl::clear",
        Arity::exact(0),
        &["term::ansi::code::ctrl::clear"],
        "Clear screen.",
    ),
    (
        "term::ansi::code::ctrl::init",
        Arity::exact(0),
        &["term::ansi::code::ctrl::init"],
        "Initialize default and alternate fonts to ASCII and box graphics.",
    ),
    (
        "term::ansi::code::ctrl::showat",
        Arity::exact(3),
        &["term::ansi::code::ctrl::showat row col text"],
        "Format the block of text for display at the specified location.",
    ),
];

/// The `term::ansi::code::macros` package.
const TERM__ANSI__CODE__MACROS_CMDS: &[Row] = &[
    (
        "term::ansi::code::macros::names",
        Arity::exact(0),
        &["term::ansi::code::macros::names"],
        "This command is for introspection.",
    ),
    (
        "term::ansi::code::macros::menu",
        Arity::exact(1),
        &["term::ansi::code::macros::menu menu"],
        "The description of a menu is converted into a formatted rectangular block of text, with the menu command characte",
    ),
    (
        "term::ansi::code::macros::frame",
        Arity::exact(1),
        &["term::ansi::code::macros::frame string"],
        "The paragraph of text contained in the string is padded with spaces at the right margin, after normalizing intern",
    ),
];

/// The `term::ansi::ctrl::unix` package.
const TERM__ANSI__CTRL__UNIX_CMDS: &[Row] = &[
    (
        "term::ansi::ctrl::unix::raw",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::raw"],
        "This command switches the standard input of the current process to raw input mode.",
    ),
    (
        "term::ansi::ctrl::unix::cooked",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::cooked"],
        "This command switches the standard input of the current process to cooked input mode.",
    ),
    (
        "term::ansi::ctrl::unix::columns",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::columns"],
        "This command queries the terminal connected to the standard input for the number of columns available for display",
    ),
    (
        "term::ansi::ctrl::unix::rows",
        Arity::exact(0),
        &["term::ansi::ctrl::unix::rows"],
        "This command queries the terminal connected to the standard input for the number of rows (aka lines) available fo",
    ),
];

/// The `term::send` package.
const TERM__SEND_CMDS: &[Row] = &[
    (
        "term::send::wrch",
        Arity::exact(2),
        &["term::send::wrch chan str"],
        "Send the text str to the channel specified by the handle chan.",
    ),
    (
        "term::send::wr",
        Arity::exact(1),
        &["term::send::wr str"],
        "This convenience command is like ::term::send::wrch, except that the destination channel is fixed to stdout.",
    ),
];

/// The `textutil` package.
const TEXTUTIL_CMDS: &[Row] = &[(
    "textutil::expander",
    Arity::exact(1),
    &["textutil::expander expanderName"],
    "The command creates a new expander object with an associated Tcl command whose name is expanderName.",
)];

/// The `textutil::adjust` package.
///
/// `adjust` and `indent` are the package's two primary, most-used commands
/// (tcllib-2.0 `modules/textutil/adjust.tcl`); they are also re-exported as
/// the flattened `::textutil::adjust` / `::textutil::indent` aliases by the
/// `textutil` umbrella package (`textutil.tcl`'s `namespace import -force
/// adjust::adjust adjust::indent adjust::undent`) — see
/// `textutil__adjust.rs` / `textutil__indent.rs` for those, which are gated
/// on `required_package: "textutil"` (the umbrella), not this package.
/// Requiring `textutil::adjust` alone grants only these three-segment names
/// (issue #923 idx 3/4, confirmed against tclsh 9.0.4 + real tcllib-2.0: a
/// bare `::textutil::adjust` command exists only after `package require
/// textutil`, never after `package require textutil::adjust` alone).
const TEXTUTIL__ADJUST_CMDS: &[Row] = &[
    (
        "textutil::adjust::adjust",
        Arity::at_least(1),
        &[
            "textutil::adjust::adjust string ?-length num? ?-justify left|right|center|plain? ?-hyphenate bool? ?-full bool? ?-strictlength bool?",
        ],
        "Adjust/reformat a paragraph of text to a given line length, with optional justification and hyphenation.",
    ),
    (
        "textutil::adjust::indent",
        Arity::new(2, 3),
        &["textutil::adjust::indent text prefix ?skip?"],
        "Indent each line of text by prefixing it with prefix, skipping the first skip lines.",
    ),
    (
        "textutil::adjust::readPatterns",
        Arity::exact(1),
        &["textutil::adjust::readPatterns filename"],
        "Loads the internal storage for hyphenation patterns with the contents of the file filename.",
    ),
    (
        "textutil::adjust::listPredefined",
        Arity::exact(0),
        &["textutil::adjust::listPredefined"],
        "This command returns a list containing the names of the hyphenation files coming with this package.",
    ),
    (
        "textutil::adjust::getPredefined",
        Arity::exact(1),
        &["textutil::adjust::getPredefined filename"],
        "Use this command to query the package for the full path name of the hyphenation file filename coming with the pac",
    ),
    (
        "textutil::adjust::undent",
        Arity::exact(1),
        &["textutil::adjust::undent string"],
        "The command computes the common prefix for all lines in string consisting solely out of whitespace, removes this ",
    ),
];

/// The `textutil::patch` package.
const TEXTUTIL__PATCH_CMDS: &[Row] = &[(
    "textutil::patch::apply",
    Arity::exact(4),
    &["textutil::patch::apply basedirectory striplevel patch reportcmd"],
    "Applies the patch (text of the path, not file) to the files in the basedirectory using the specified striplevel.",
)];

/// The `textutil::string` package.
const TEXTUTIL__STRING_CMDS: &[Row] = &[
    (
        "textutil::string::chop",
        Arity::exact(1),
        &["textutil::string::chop string"],
        "A convenience command.",
    ),
    (
        "textutil::string::tail",
        Arity::exact(1),
        &["textutil::string::tail string"],
        "A convenience command.",
    ),
    (
        "textutil::string::cap",
        Arity::exact(1),
        &["textutil::string::cap string"],
        "Capitalizes the first character of string and returns the modified string.",
    ),
    (
        "textutil::string::capEachWord",
        Arity::exact(1),
        &["textutil::string::capEachWord string"],
        "Capitalizes the first character of word of the string and returns the modified string.",
    ),
    (
        "textutil::string::uncap",
        Arity::exact(1),
        &["textutil::string::uncap string"],
        "The complementary operation to ::textutil::string::cap.",
    ),
    (
        "textutil::string::longestCommonPrefixList",
        Arity::exact(1),
        &["textutil::string::longestCommonPrefixList list"],
        "tcllib command.",
    ),
    (
        "textutil::string::longestCommonPrefix",
        Arity::at_least(0),
        &["textutil::string::longestCommonPrefix ?string...?"],
        "Returns the longest common prefix of the given strings.",
    ),
];

/// The `textutil::trim` package.
///
/// `trim`/`trimleft`/`trimright` are also re-exported as the flattened
/// `::textutil::trim` / `::textutil::trimleft` / `::textutil::trimright`
/// aliases by the `textutil` umbrella package — see `textutil__trim.rs` /
/// `textutil__trimleft.rs` / `textutil__trimright.rs` for those (gated on
/// `required_package: "textutil"`, not this package). Requiring
/// `textutil::trim` alone grants only these three-segment names (same
/// meta-package-flattening shape as `textutil::adjust`, issue #923 idx 3/4;
/// confirmed against tclsh 9.0.4 + real tcllib-2.0).
const TEXTUTIL__TRIM_CMDS: &[Row] = &[
    (
        "textutil::trim::trim",
        Arity::new(1, 2),
        &["textutil::trim::trim string ?trim?"],
        "Removes any leading and trailing occurrences of the characters in trim (a regular expression, default whitespace) from string.",
    ),
    (
        "textutil::trim::trimleft",
        Arity::new(1, 2),
        &["textutil::trim::trimleft string ?trim?"],
        "Removes any leading occurrences of the characters in trim (a regular expression, default whitespace) from string.",
    ),
    (
        "textutil::trim::trimright",
        Arity::new(1, 2),
        &["textutil::trim::trimright string ?trim?"],
        "Removes any trailing occurrences of the characters in trim (a regular expression, default whitespace) from string.",
    ),
    (
        "textutil::trim::trimPrefix",
        Arity::exact(2),
        &["textutil::trim::trimPrefix string prefix"],
        "Removes the prefix from the beginning of string and returns the result.",
    ),
    (
        "textutil::trim::trimEmptyHeading",
        Arity::exact(1),
        &["textutil::trim::trimEmptyHeading string"],
        "Looks for empty lines (including lines consisting of only whitespace) at the beginning of the string and removes ",
    ),
];

/// The `textutil::tabify` package.
///
/// `tabify`/`untabify`/`tabify2`/`untabify2` are also re-exported as the
/// flattened `::textutil::tabify` / `::textutil::untabify` /
/// `::textutil::tabify2` / `::textutil::untabify2` aliases by the `textutil`
/// umbrella package — see `textutil__tabify.rs` / `textutil__untabify.rs` /
/// `textutil__tabify2.rs` / `textutil__untabify2.rs` for those (gated on
/// `required_package: "textutil"`, not this package). Requiring
/// `textutil::tabify` alone grants only these three-segment names (same
/// meta-package-flattening shape as `textutil::adjust`, issue #923 idx 3/4;
/// confirmed against tclsh 9.0.4 + real tcllib-2.0).
const TEXTUTIL__TABIFY_CMDS: &[Row] = &[
    (
        "textutil::tabify::tabify",
        Arity::new(1, 2),
        &["textutil::tabify::tabify string ?num?"],
        "Replaces all appropriate occurrences of num spaces with a tab character in string.",
    ),
    (
        "textutil::tabify::untabify",
        Arity::new(1, 2),
        &["textutil::tabify::untabify string ?num?"],
        "Replaces all tabs in string with the appropriate number of spaces, assuming a tab size of num.",
    ),
    (
        "textutil::tabify::tabify2",
        Arity::new(1, 2),
        &["textutil::tabify::tabify2 string ?num?"],
        "Like tabify, but works line by line and only replaces spaces at true tab-stop-aligned positions.",
    ),
    (
        "textutil::tabify::untabify2",
        Arity::new(1, 2),
        &["textutil::tabify::untabify2 string ?num?"],
        "Like untabify, but works line by line.",
    ),
];

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
    ("sha2", SHA2_CMDS),
    ("simulation::annealing", SIMULATION__ANNEALING_CMDS),
    ("simulation::montecarlo", SIMULATION__MONTECARLO_CMDS),
    ("simulation::random", SIMULATION__RANDOM_CMDS),
    ("smtpd", SMTPD_CMDS),
    ("struct::tree", STRUCT__TREE_CMDS),
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
        // `::reached` (issue #1672 audit).
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
/// trait descriptors (`Traits::PURE`, …) — [`sync_textutil_submodule_traits`]
/// copies them onto the `Row`-built submodule spec below rather than a
/// second, independent `Traits::PURE` declaration, so the two spellings of
/// the same real tcllib command can never drift apart (issue #923 idx 3/4
/// review follow-up: the submodule spec was built via the generic `Row` →
/// `CommandSpec::DEFAULT` path and so silently lacked the `Traits::PURE` its
/// umbrella sibling already carried, blocking purity-based GVN/DCE on the
/// canonical three-segment spelling only).
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
        // 4-tuple [`Row`], so it is appended separately (issue #811).
        .chain(html_rows(HTML_CMDS))
        .chain(std::iter::once(processman_onexit_spec()))
        .collect();
    sync_textutil_submodule_traits(&mut specs);
    specs
}
