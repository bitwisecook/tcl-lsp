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

//! Grammars and parser generation: the `grammar::*`, `page::*` and `pt` packages.

use super::{Arity, Row};

/// The `grammar::fa::op` package.
pub(super) const GRAMMAR__FA__OP_CMDS: &[Row] = &[
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
pub(super) const GRAMMAR__ME_CMDS: &[Row] = &[(
    "grammar::me::cpu",
    Arity::exact(2),
    &["grammar::me::cpu meName matchcode"],
    "The command creates a new ME machine object with an associated global Tcl command whose name is meName.",
)];

/// The `grammar::me::cpu::gasm` package.
pub(super) const GRAMMAR__ME__CPU__GASM_CMDS: &[Row] = &[
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
pub(super) const GRAMMAR__ME__TCL_CMDS: &[Row] = &[
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
pub(super) const GRAMMAR__PEG__INTERP_CMDS: &[Row] = &[
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

/// The `page::pluginmgr` package.
pub(super) const PAGE__PLUGINMGR_CMDS: &[Row] = &[
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
pub(super) const PAGE__UTIL_CMDS: &[Row] = &[(
    "page::util::flow",
    Arity::exact(4),
    &["page::util::flow start flowvar nodevar script"],
    "This command contains the core logic to drive the walking of an arbitrary data structure which can partitioned in",
)];

/// The `page::util::norm` package.
pub(super) const PAGE__UTIL__NORM_CMDS: &[Row] = &[
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
pub(super) const PAGE__UTIL__PEG_CMDS: &[Row] = &[
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
pub(super) const PAGE__UTIL__QUOTE_CMDS: &[Row] = &[(
    "page::util::quote::unquote",
    Arity::exact(1),
    &["page::util::quote::unquote char"],
    "A character, as stored in an abstract syntax tree by a PEG processor (See the packages grammar::peg::interpreter,",
)];

/// The `pt` package.
pub(super) const PT_CMDS: &[Row] = &[(
    "pt::rde",
    Arity::exact(1),
    &["pt::rde objectName"],
    "The command creates a new runtime object for a recursive descent parser with backtracking and returns the fully q",
)];

/// The `pt::peg` package.
pub(super) const PT__PEG_CMDS: &[Row] = &[
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
pub(super) const PT_EXPORT_API_CMDS: &[Row] = &[(
    "export",
    Arity::exact(2),
    &["export serial configuration"],
    "This command has to accept the canonical serialization of a parsing expression grammar and the configuration for ",
)];

/// The `pt_import_api` package.
pub(super) const PT_IMPORT_API_CMDS: &[Row] = &[(
    "import",
    Arity::exact(1),
    &["import text"],
    "This command has to accept the a text containing a parsing expression grammar in some format.",
)];
