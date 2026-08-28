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

//! Additional tcllib packages — `rest`, `SASL`, `time` (SNTP), `javascript`,
//! `nmea`, `bibtex`, and `rcs`.
//!
//! Public command surface from the upstream tcllib 2.0 manual pages.  Command
//! names, arity bounds, synopses, and summaries are derived from those pages.
//! Requires Tcl 8.5+.

use crate::prelude::*;

/// A row in a package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build (non-pure) command specs for a package from its table.
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

/// The `rest` package.
const REST_CMDS: &[Row] = &[
    (
        "rest::simple",
        Arity::at_least(2),
        &["rest::simple url query"],
        "Perform a one-shot REST request and return the response body.",
    ),
    (
        "rest::get",
        Arity::at_least(2),
        &["rest::get url query"],
        "Perform a REST GET request to a URL.",
    ),
    (
        "rest::post",
        Arity::at_least(2),
        &["rest::post url query"],
        "Perform a REST POST request to a URL.",
    ),
    (
        "rest::patch",
        Arity::at_least(2),
        &["rest::patch url query"],
        "Perform a REST PATCH request to a URL.",
    ),
    (
        "rest::head",
        Arity::at_least(2),
        &["rest::head url query"],
        "Perform a REST HEAD request to a URL.",
    ),
    (
        "rest::put",
        Arity::at_least(2),
        &["rest::put url query"],
        "Perform a REST PUT request to a URL.",
    ),
    (
        "rest::delete",
        Arity::at_least(2),
        &["rest::delete url query"],
        "These commands are all equivalent except for the http method used.",
    ),
    (
        "rest::save",
        Arity::exact(2),
        &["rest::save name file"],
        "This command saves a copy of the dynamically created procedures for all the API calls specified in the array.",
    ),
    (
        "rest::describe",
        Arity::exact(1),
        &["rest::describe name"],
        "This command prints a description of all API calls specified in the array variable name to the channel stdout.",
    ),
    (
        "rest::parameters",
        Arity::at_least(1),
        &["rest::parameters url"],
        "This command parses an url query string into a dictionary and returns said dictionary as its result.",
    ),
    (
        "rest::parse_opts",
        Arity::exact(4),
        &["rest::parse_opts static required optional words"],
        "This command implements a custom parserfor command options.",
    ),
    (
        "rest::substitute",
        Arity::exact(2),
        &["rest::substitute string var"],
        "This command takes a string, substitutes values for any option identifiers found inside and returns the modified.",
    ),
    (
        "rest::create_interface",
        Arity::exact(1),
        &["rest::create_interface name"],
        "This command creates procedures for all the API calls specified in the array variable name.",
    ),
];

/// The `SASL` package.
const SASL_CMDS: &[Row] = &[
    (
        "SASL::new",
        Arity::at_least(1),
        &["SASL::new option value"],
        "Contruct a new SASL context.",
    ),
    (
        "SASL::configure",
        Arity::at_least(1),
        &["SASL::configure option value"],
        "Modify and inspect the SASL context option.",
    ),
    (
        "SASL::step",
        Arity::at_least(2),
        &["SASL::step context challenge"],
        "This is the core procedure for using the SASL framework.",
    ),
    (
        "SASL::response",
        Arity::exact(1),
        &["SASL::response context"],
        "Returns the next response string that should be sent to the server.",
    ),
    (
        "SASL::reset",
        Arity::exact(1),
        &["SASL::reset context"],
        "Re-initialize the SASL context.",
    ),
    (
        "SASL::cleanup",
        Arity::exact(1),
        &["SASL::cleanup context"],
        "Release all resources associated with the SASL context.",
    ),
    (
        "SASL::mechanisms",
        Arity::at_least(0),
        &["SASL::mechanisms"],
        "Returns a list of all the available SASL mechanisms.",
    ),
    (
        "SASL::register",
        Arity::at_least(3),
        &["SASL::register mechanism preference clientproc"],
        "New mechanisms can be added to the package by registering the mechanism name and the implementing procedures.",
    ),
];

/// The `time` package.
const TIME_CMDS: &[Row] = &[
    (
        "time::gettime",
        Arity::at_least(1),
        &["time::gettime timeserver"],
        "Get the time from timeserver.",
    ),
    (
        "time::getsntp",
        Arity::at_least(1),
        &["time::getsntp timeserver"],
        "Get the time from an SNTP server.",
    ),
    (
        "time::configure",
        Arity::at_least(0),
        &["time::configure"],
        "Called with no arguments this command returns all the current configuration options and values.",
    ),
    (
        "time::cget",
        Arity::exact(1),
        &["time::cget name"],
        "Get the current value for the named configuration option.",
    ),
    (
        "time::unixtime",
        Arity::exact(1),
        &["time::unixtime token"],
        "Format the returned time for the unix epoch.",
    ),
    (
        "time::status",
        Arity::exact(1),
        &["time::status token"],
        "Returns the status flag.",
    ),
    (
        "time::error",
        Arity::exact(1),
        &["time::error token"],
        "Returns the error message provided for requests whose status is error.",
    ),
    (
        "time::reset",
        Arity::at_least(2),
        &["time::reset token"],
        "Reset or cancel the query optionally specfying the reason to record for the error command.",
    ),
    (
        "time::wait",
        Arity::exact(1),
        &["time::wait token"],
        "Wait for a query to complete and return the status upon completion.",
    ),
    (
        "time::cleanup",
        Arity::exact(1),
        &["time::cleanup token"],
        "Remove all state variables associated with the request.",
    ),
];

/// The `javascript` package.
const JAVASCRIPT_CMDS: &[Row] = &[
    (
        "javascript::makeSelectorWidget",
        Arity::at_least(1),
        &[
            "javascript::makeSelectorWidget {id leftLabel leftValueList rightLabel rightValueList rightNameList}",
        ],
        "Construct HTML code to create a dual-multi-selection megawidget.",
    ),
    (
        "javascript::makeSubmitButton",
        Arity::exact(1),
        &["javascript::makeSubmitButton {name value}"],
        "Create an HTML submit button that resets a hidden field for each registered multi-selection box.",
    ),
    (
        "javascript::makeProtectedSubmitButton",
        Arity::exact(1),
        &["javascript::makeProtectedSubmitButton {name value msg}"],
        "Create an HTML submit button that prompts the user with a continue/cancel shutdown warning before the form is.",
    ),
    (
        "javascript::makeMasterButton",
        Arity::exact(1),
        &["javascript::makeMasterButton {master value slavePattern boolean}"],
        "Create an HTML button that sets its slave checkboxs to the boolean value.",
    ),
    (
        "javascript::makeParentCheckbox",
        Arity::exact(1),
        &["javascript::makeParentCheckbox {parentName childName}"],
        "Create an HTML checkbox and tie its value to that of its child checkbox.",
    ),
    (
        "javascript::makeChildCheckbox",
        Arity::exact(1),
        &["javascript::makeChildCheckbox {parentName childName}"],
        "Create an HTML checkbox and tie its value to that of its parent checkbox.",
    ),
];

/// The `nmea` package.
const NMEA_CMDS: &[Row] = &[
    (
        "nmea::input",
        Arity::exact(1),
        &["nmea::input sentence"],
        "Processes and dispatches the supplied sentence.",
    ),
    (
        "nmea::open_port",
        Arity::at_least(1),
        &["nmea::open_port port speed"],
        "Open the specified COM port and read NMEA sentences when available.",
    ),
    (
        "nmea::close_port",
        Arity::exact(0),
        &["nmea::close_port"],
        "Close the com port connection if one is open.",
    ),
    (
        "nmea::configure_port",
        Arity::exact(1),
        &["nmea::configure_port settings"],
        "Changes the current port settings.",
    ),
    (
        "nmea::open_file",
        Arity::at_least(1),
        &["nmea::open_file file rate"],
        "Open file file and read NMEA sentences, one per line, at the rate specified by rate in milliseconds.",
    ),
    (
        "nmea::close_file",
        Arity::exact(0),
        &["nmea::close_file"],
        "Close the open file if one exists.",
    ),
    (
        "nmea::do_line",
        Arity::exact(0),
        &["nmea::do_line"],
        "If there is a currently open file, this command will read and process a single line from it.",
    ),
    (
        "nmea::rate",
        Arity::exact(0),
        &["nmea::rate"],
        "Sets the rate at which lines are processed from the open file, in milliseconds.",
    ),
    (
        "nmea::log",
        Arity::at_least(0),
        &["nmea::log file"],
        "Starts or stops input logging.",
    ),
    (
        "nmea::checksum",
        Arity::exact(1),
        &["nmea::checksum data"],
        "Returns the checksum of the supplied data.",
    ),
    (
        "nmea::write",
        Arity::exact(2),
        &["nmea::write sentence data"],
        "If there is a currently open port, this command will write the specified sentence and data to the port in proper.",
    ),
    (
        "nmea::event",
        Arity::at_least(1),
        &["nmea::event setence command"],
        "Registers a handler proc for a given NMEA sentence.",
    ),
];

/// The `bibtex` package.  `bibtex::parse` is modelled separately
/// ([`bibtex_parse_spec`]) because its callback options carry command
/// prefixes.
const BIBTEX_CMDS: &[Row] = &[
    (
        "bibtex::wait",
        Arity::exact(1),
        &["bibtex::wait token"],
        "This command waits for the parser represented by the token to complete and then returns.",
    ),
    (
        "bibtex::destroy",
        Arity::exact(1),
        &["bibtex::destroy token"],
        "This command cleans up all internal state associated with the parser represented by the handle token, effectively.",
    ),
    (
        "bibtex::addStrings",
        Arity::exact(2),
        &["bibtex::addStrings token stringdict"],
        "This command adds the macro definitions stored in the dictionary stringdict to the parser represented by the.",
    ),
];

/// The `rcs` package.
const RCS_CMDS: &[Row] = &[
    (
        "rcs::text2dict",
        Arity::exact(1),
        &["rcs::text2dict text"],
        "Converts the argument text into a dictionary containing and representing the same text in an indexed form and.",
    ),
    (
        "rcs::dict2text",
        Arity::exact(1),
        &["rcs::dict2text dict"],
        "This command provides the complementary operation to ::rcs::text2dict.",
    ),
    (
        "rcs::file2dict",
        Arity::exact(1),
        &["rcs::file2dict filename"],
        "This command is identical to ::rcs::text2dict, except that it reads the text to convert from the file with path.",
    ),
    (
        "rcs::dict2file",
        Arity::exact(2),
        &["rcs::dict2file filename dict"],
        "This command is identical to ::rcs::2dict2text, except that it stores the resulting text in the file with path.",
    ),
    (
        "rcs::decodeRcsPatch",
        Arity::exact(1),
        &["rcs::decodeRcsPatch text"],
        "Converts the text argument into a patch command list (PCL) as specified in the section {RCS PATCH COMMAND LIST}.",
    ),
    (
        "rcs::encodeRcsPatch",
        Arity::exact(1),
        &["rcs::encodeRcsPatch pcmds"],
        "This command provides the complementary operation to ::rcs::decodeRcsPatch.",
    ),
    (
        "rcs::applyRcsPatch",
        Arity::exact(2),
        &["rcs::applyRcsPatch text pcmds"],
        "This operation applies a patch in the form of a PCL to a text given in the form of a dictionary and returns the.",
    ),
];

/// Options for `bibtex::parse`.  Every callback is dispatched through the one
/// `::bibtex::Callback token type args` proc (bibtex.tcl 265-272), which fires
/// `eval $cmd [linsert $args 0 $token]` — so each prefix is invoked with the
/// parser `token` first, then that callback's own payload words (verified vs
/// the default-callback signatures `AddRecord {token type key recdata}` /
/// `addStrings {token strings}` and re-run in tclsh).
const BIBTEX_PARSE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-command",
        // `Callback $token {} $result` (bibtex.tcl 294) — batch mode: token +
        // the whole accumulated records list.
        value: OptionValue::deferred_command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Batch callback invoked once at EOF with the token and the full record list.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-recordcommand",
        // `Callback $token record [Tidy $type] [string trim $key] [ParseBlock
        // $rest]` (395-396) — token, type, key, recdata.
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(4)),
        detail: "Per-record callback invoked with token, entry type, key, and record data.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-preamblecommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Per-preamble callback invoked with token and the preamble text.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-stringcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Per-@string callback invoked with token and the string macro(s).",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-commentcommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Per-comment callback invoked with token and the comment text.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-progresscommand",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(2)),
        detail: "Progress callback invoked with token and a percentage.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-channel",
        value: OptionValue::channel("chan"),
        detail: "Parse incrementally from this channel instead of a text string.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-casesensitivestrings",
        value: OptionValue::boolean(),
        detail: "Treat @string macro names case-sensitively.",
        ..OptionSpec::DEFAULT
    },
];

/// Resolve the phase of `bibtex::parse` callbacks from its input mode.
///
/// `-command` is the completion callback and is only valid with `-channel`,
/// hence always deferred. The SAX-style callbacks run inline for an in-memory
/// text argument, but `-channel` makes the parser install a `fileevent` and
/// return a token before any of them run.
fn bibtex_parse_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    let mut channel_mode = false;
    let mut callbacks = Vec::new();
    let mut index = 0usize;
    while index + 1 < args.len() {
        let option = args[index];
        if option == "-channel" {
            channel_mode = true;
        } else if matches!(
            option,
            "-command"
                | "-recordcommand"
                | "-preamblecommand"
                | "-stringcommand"
                | "-commentcommand"
                | "-progresscommand"
        ) {
            callbacks.push((index + 1, option == "-command"));
        } else if !matches!(option, "-casesensitivestrings") {
            // Options precede the optional in-memory text. Stop at that text
            // (or an unknown switch) so a literal callback value/text named
            // `-channel` cannot accidentally select background semantics.
            break;
        }
        index += 2;
    }

    callbacks
        .into_iter()
        .filter_map(|(position, completion_callback)| {
            let timing = if completion_callback || channel_mode {
                ScriptTiming::Deferred
            } else {
                ScriptTiming::SameInvocation
            };
            u8::try_from(position)
                .ok()
                .map(|position| (position, timing))
        })
        .collect()
}

/// `bibtex::parse`'s **proven** option relations (P5, completed under E-R14).
///
/// `bibtex.tcl:196-211` computes `sax` as "any of the five `-TYPEcommand`
/// options is present" and `bg` as "`-command` is present", then raises
/// *"The options `-command` and `-TYPEcommand` exclude each other"* when
/// both hold.  That is five pairwise exclusions, one per SAX callback.
///
/// The three rows after them are what the retired `OptionConstraint` could
/// not say, and what E-R14's typed relation does (census gap G2's directional
/// half, and the option-to-positional half beside it).  All three are the same
/// site, `bibtex.tcl:213-241`:
///
/// - `-channel` present ⇒ *"Option -channel and text exclude each other"*, an
///   exclusion between an option and a **positional argument**;
/// - no `-channel` ⇒ *"Option -command and text exclude each other"*, i.e.
///   `-command` **requires** `-channel` — the directional relation P5 raised;
/// - no `-channel` and no text ⇒ *"Neither -channel nor text specified"*, an
///   unconditional **requires-one-of** over an option and an argument.
const BIBTEX_PARSE_RELATIONS: &[OptionRelation] = &[
    OptionRelation::conflict(&[
        OptionTerm::Option("-command"),
        OptionTerm::Option("-recordcommand"),
    ]),
    OptionRelation::conflict(&[
        OptionTerm::Option("-command"),
        OptionTerm::Option("-preamblecommand"),
    ]),
    OptionRelation::conflict(&[
        OptionTerm::Option("-command"),
        OptionTerm::Option("-stringcommand"),
    ]),
    OptionRelation::conflict(&[
        OptionTerm::Option("-command"),
        OptionTerm::Option("-commentcommand"),
    ]),
    OptionRelation::conflict(&[
        OptionTerm::Option("-command"),
        OptionTerm::Option("-progresscommand"),
    ]),
    OptionRelation {
        kind: RelationKind::Requires,
        subject: Some(OptionTerm::Option("-command")),
        terms: &[OptionTerm::Option("-channel")],
        message: Some(
            "Option -command and text exclude each other: bibtex::parse -command \
             is the channel-completion callback and needs -channel",
        ),
        ..OptionRelation::DEFAULT
    },
    OptionRelation {
        kind: RelationKind::Forbids,
        subject: Some(OptionTerm::Option("-channel")),
        terms: &[OptionTerm::Argument(0)],
        message: Some("Option -channel and text exclude each other"),
        ..OptionRelation::DEFAULT
    },
    OptionRelation {
        kind: RelationKind::RequiresOneOf,
        subject: None,
        terms: &[OptionTerm::Option("-channel"), OptionTerm::Argument(0)],
        message: Some("Neither -channel nor text specified"),
        ..OptionRelation::DEFAULT
    },
];

/// `bibtex::parse` — structured so its callback options carry command prefixes.
fn bibtex_parse_spec() -> CommandSpec {
    CommandSpec {
        name: "bibtex::parse",
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This is the general form of the command for parsing a bibliography.",
            &["bibtex::parse ?options? ?text?"],
            "tcllib bibtex package",
        )),
        options: BIBTEX_PARSE_OPTIONS,
        option_relations: BIBTEX_PARSE_RELATIONS,
        script_timing_resolver: Some(bibtex_parse_script_timing),
        tcllib_package: Some("bibtex"),
        required_package: Some("bibtex"),
        ..CommandSpec::DEFAULT
    }
}

/// All additional-package command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(rows("rest", REST_CMDS));
    specs.extend(rows("SASL", SASL_CMDS));
    specs.extend(rows("time", TIME_CMDS));
    specs.extend(rows("javascript", JAVASCRIPT_CMDS));
    specs.extend(rows("nmea", NMEA_CMDS));
    specs.extend(rows("bibtex", BIBTEX_CMDS));
    specs.push(bibtex_parse_spec());
    specs.extend(rows("rcs", RCS_CMDS));
    specs
}
