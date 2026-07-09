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

//! tcllib client / logging packages — `log`, `ftp`, `ldap`, `pop3`, `irc`,
//! and `uevent`.
//!
//! Public command surface from the upstream tcllib 2.0 manual pages
//! (`modules/{log,ftp,ldap,pop3,irc,uev}/*.man`).  Command names, arity
//! bounds, synopses, and summaries are derived from those pages.  These
//! commands drive network connections, logging, and event dispatch, so none
//! are marked pure.  Requires Tcl 8.5+.

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

/// The `log` package.
const LOG_CMDS: &[Row] = &[
    (
        "log::levels",
        Arity::exact(0),
        &["log::levels"],
        "Returns the names of all known levels, in alphabetical order.",
    ),
    (
        "log::lv2longform",
        Arity::exact(1),
        &["log::lv2longform level"],
        "Converts any unique abbreviation of a level name to the full level name.",
    ),
    (
        "log::lv2color",
        Arity::exact(1),
        &["log::lv2color level"],
        "Converts any level name including unique abbreviations to the corresponding color.",
    ),
    (
        "log::lv2priority",
        Arity::exact(1),
        &["log::lv2priority level"],
        "Converts any level name including unique abbreviations to the corresponding priority.",
    ),
    (
        "log::lv2cmd",
        Arity::exact(1),
        &["log::lv2cmd level"],
        "Converts any level name including unique abbreviations to the command prefix used to write messages with that.",
    ),
    (
        "log::lv2channel",
        Arity::exact(1),
        &["log::lv2channel level"],
        "Converts any level name including unique abbreviations to the channel used by ::log::Puts to write messages with.",
    ),
    (
        "log::lvCompare",
        Arity::exact(2),
        &["log::lvCompare level1 level2"],
        "Compares two levels (including unique abbreviations) with respect to their priority.",
    ),
    (
        "log::lvSuppress",
        Arity::new(1, 2),
        &["log::lvSuppress level ?suppress?"],
        "(Un)suppresses the output of messages having the specified level.",
    ),
    (
        "log::lvSuppressLE",
        Arity::new(1, 2),
        &["log::lvSuppressLE level ?suppress?"],
        "(Un)suppresses the output of messages having the specified level or one of lesser priority.",
    ),
    (
        "log::lvIsSuppressed",
        Arity::exact(1),
        &["log::lvIsSuppressed level"],
        "Asks the package whether the specified level is currently suppressed.",
    ),
    (
        "log::lvCmd",
        Arity::exact(2),
        &["log::lvCmd level cmd"],
        "Defines for the specified level with which command to write the messages having this level.",
    ),
    (
        "log::lvCmdForall",
        Arity::exact(1),
        &["log::lvCmdForall cmd"],
        "Defines for all known levels with which command to write the messages having this level.",
    ),
    (
        "log::lvChannel",
        Arity::exact(2),
        &["log::lvChannel level chan"],
        "Defines for the specified level into which channel ::log::Puts (the standard command) shall write the messages.",
    ),
    (
        "log::lvChannelForall",
        Arity::exact(1),
        &["log::lvChannelForall chan"],
        "Defines for all known levels with which which channel ::log::Puts (the standard command) shall write the messages.",
    ),
    (
        "log::lvColor",
        Arity::exact(2),
        &["log::lvColor level color"],
        "Defines for the specified level the color to return for it in a call to ::log::lv2color.",
    ),
    (
        "log::lvColorForall",
        Arity::exact(1),
        &["log::lvColorForall color"],
        "Defines for all known levels the color to return for it in a call to ::log::lv2color.",
    ),
    (
        "log::log",
        Arity::exact(2),
        &["log::log level text"],
        "Log a message according to the specifications for commands, channels and suppression.",
    ),
    (
        "log::logarray",
        Arity::at_least(2),
        &["log::logarray level arrayvar"],
        "Like ::log::log, but logs the contents of the specified array variable arrayvar, possibly restricted to entries.",
    ),
    (
        "log::loghex",
        Arity::exact(3),
        &["log::loghex level text data"],
        "Like ::log::log, but assumes that data contains binary data.",
    ),
    (
        "log::logsubst",
        Arity::exact(2),
        &["log::logsubst level msg"],
        "Like ::log::log, but msg may contain substitutions and variable references, which are evaluated in the caller.",
    ),
    (
        "log::logMsg",
        Arity::exact(1),
        &["log::logMsg text"],
        "Convenience wrapper around ::log::log.",
    ),
    (
        "log::logError",
        Arity::exact(1),
        &["log::logError text"],
        "Convenience wrapper around ::log::log.",
    ),
    (
        "log::Puts",
        Arity::exact(2),
        &["log::Puts level text"],
        "The standard log command, it writes messages and their levels to user-specified channels.",
    ),
];

/// The `ftp` package.
const FTP_CMDS: &[Row] = &[
    (
        "ftp::Open",
        Arity::at_least(3),
        &["ftp::Open server user passwd"],
        "This command is used to start a FTP session by establishing a control connection to the FTP server.",
    ),
    (
        "ftp::Close",
        Arity::exact(1),
        &["ftp::Close handle"],
        "This command terminates the specified ftp session.",
    ),
    (
        "ftp::Cd",
        Arity::exact(2),
        &["ftp::Cd handle directory"],
        "This command changes the current working directory on the ftp server to a specified target directory.",
    ),
    (
        "ftp::Pwd",
        Arity::exact(1),
        &["ftp::Pwd handle"],
        "This command returns the complete path of the current working directory on the ftp server, or an empty string in.",
    ),
    (
        "ftp::Type",
        Arity::at_least(1),
        &["ftp::Type handle"],
        "This command sets the ftp file transfer type to either ascii, binary, or tenex.",
    ),
    (
        "ftp::List",
        Arity::at_least(1),
        &["ftp::List handle"],
        "This command returns a human-readable list of files.",
    ),
    (
        "ftp::NList",
        Arity::at_least(1),
        &["ftp::NList handle"],
        "This command has the same behavior as the ::ftp::List command, except that it only retrieves an abbreviated.",
    ),
    (
        "ftp::FileSize",
        Arity::exact(2),
        &["ftp::FileSize handle file"],
        "This command returns the size of the specified file on the ftp server.",
    ),
    (
        "ftp::ModTime",
        Arity::exact(2),
        &["ftp::ModTime handle file"],
        "This command retrieves the time of the last modification of the file on the ftp server as a system dependent.",
    ),
    (
        "ftp::Delete",
        Arity::exact(2),
        &["ftp::Delete handle file"],
        "This command deletes the specified file on the ftp server.",
    ),
    (
        "ftp::Rename",
        Arity::exact(3),
        &["ftp::Rename handle from to"],
        "This command renames the file from in the current directory of the ftp server to the specified new file name to.",
    ),
    (
        "ftp::Put",
        Arity::at_least(4),
        &["ftp::Put handle (local | -data data | -channel chan)"],
        "This command transfers a local file local to a remote file remote on the ftp server.",
    ),
    (
        "ftp::Append",
        Arity::at_least(4),
        &["ftp::Append handle (local | -data data | -channel chan)"],
        "This command behaves like ::ftp::Puts, but appends the transfered information to the remote file.",
    ),
    (
        "ftp::Get",
        Arity::at_least(2),
        &["ftp::Get handle remote ("],
        "This command retrieves a remote file remote on the ftp server and stores its contents into the local file local.",
    ),
    (
        "ftp::Reget",
        Arity::at_least(2),
        &["ftp::Reget handle remote"],
        "This command behaves like ::ftp::Get, except that if local file local exists and is smaller than remote file.",
    ),
    (
        "ftp::Newer",
        Arity::at_least(2),
        &["ftp::Newer handle remote"],
        "This command behaves like ::ftp::Get, except that it retrieves the remote file only if the modification time of.",
    ),
    (
        "ftp::MkDir",
        Arity::exact(2),
        &["ftp::MkDir handle directory"],
        "This command creates the specified directory on the ftp server.",
    ),
    (
        "ftp::RmDir",
        Arity::exact(2),
        &["ftp::RmDir handle directory"],
        "This command removes the specified directory on the ftp server.",
    ),
    (
        "ftp::Quote",
        Arity::at_least(4),
        &["ftp::Quote handle arg1 arg2 ..."],
        "This command is used to send an arbitrary ftp command to the server.",
    ),
    (
        "ftp::DisplayMsg",
        Arity::at_least(2),
        &["ftp::DisplayMsg handle msg"],
        "This command is used by the package itself to show the different messages from the ftp sessions.",
    ),
];

/// The `ldap` package.
const LDAP_CMDS: &[Row] = &[
    (
        "ldap::connect",
        Arity::at_least(1),
        &["ldap::connect host"],
        "Opens a LDAPv3 connection to the specified host, at the given port, and returns a token for the connection.",
    ),
    (
        "ldap::tlsoptions",
        Arity::at_least(0),
        &["ldap::tlsoptions reset"],
        "This command resets TLS options to default values.",
    ),
    (
        "ldap::secure_connect",
        Arity::at_least(1),
        &["ldap::secure_connect host"],
        "Like ::ldap::connect, except that the created connection is secured by SSL.",
    ),
    (
        "ldap::disconnect",
        Arity::exact(1),
        &["ldap::disconnect handle"],
        "Closes the ldap connection refered to by the token handle.",
    ),
    (
        "ldap::starttls",
        Arity::at_least(1),
        &["ldap::starttls handle"],
        "Start TLS negotiation on the connection denoted by handle, with TLS parameters set with ::ldap::tlsoptions.",
    ),
    (
        "ldap::bind",
        Arity::at_least(1),
        &["ldap::bind handle"],
        "This command authenticates the ldap connection refered to by the token in handle, with a user name and associated.",
    ),
    (
        "ldap::bindSASL",
        Arity::at_least(1),
        &["ldap::bindSASL handle"],
        "This command uses SASL authentication mechanisms to do a multistage bind.",
    ),
    (
        "ldap::unbind",
        Arity::exact(1),
        &["ldap::unbind handle"],
        "This command asks the ldap server to release the last bind done for the connection refered to by the token in.",
    ),
    (
        "ldap::search",
        Arity::exact(5),
        &["ldap::search handle baseObject filterString attributes options"],
        "This command performs a LDAP search below the baseObject tree using a complex LDAP search expression filterString.",
    ),
    (
        "ldap::searchInit",
        Arity::exact(5),
        &["ldap::searchInit handle baseObject filterString attributes options"],
        "This command initiates a LDAP search below the baseObject tree using a complex LDAP search expression filterString.",
    ),
    (
        "ldap::searchNext",
        Arity::exact(1),
        &["ldap::searchNext handle"],
        "This command returns the next entry from a LDAP search initiated by ::ldap::searchInit.",
    ),
    (
        "ldap::searchEnd",
        Arity::exact(1),
        &["ldap::searchEnd handle"],
        "This command terminates a LDAP search initiated by ::ldap::searchInit.",
    ),
    (
        "ldap::modify",
        Arity::at_least(3),
        &["ldap::modify handle dn attrValToReplace"],
        "This command modifies the object dn on the ldap server we are connected to via handle.",
    ),
    (
        "ldap::modifyMulti",
        Arity::at_least(3),
        &["ldap::modifyMulti handle dn attrValToReplace"],
        "This command modifies the object dn on the ldap server we are connected to via handle.",
    ),
    (
        "ldap::add",
        Arity::exact(3),
        &["ldap::add handle dn attrValueTuples"],
        "This command creates a new object using the specified dn.",
    ),
    (
        "ldap::addMulti",
        Arity::exact(3),
        &["ldap::addMulti handle dn attrValueTuples"],
        "This command is the preferred one to create a new object using the specified dn.",
    ),
    (
        "ldap::delete",
        Arity::exact(2),
        &["ldap::delete handle dn"],
        "This command removes the object specified by dn, and all its attributes from the server.",
    ),
    (
        "ldap::modifyDN",
        Arity::at_least(3),
        &["ldap::modifyDN handle dn newrdn"],
        "This command moves or copies the object specified by dn to a new location in the tree of object.",
    ),
    (
        "ldap::info",
        Arity::at_least(0),
        &["ldap::info ip handle"],
        "This command returns the IP address of the remote LDAP server the handle is connected to.",
    ),
];

/// The `pop3` package.
const POP3_CMDS: &[Row] = &[
    (
        "pop3::open",
        Arity::at_least(1),
        &["pop3::open {host username password}"],
        "Open a socket connection to the server specified by host, transmit the username and password as login information.",
    ),
    (
        "pop3::config",
        Arity::exact(1),
        &["pop3::config chan"],
        "Returns the configuration of the pop3 connection identified by the channel handle chan as a serialized array.",
    ),
    (
        "pop3::status",
        Arity::exact(1),
        &["pop3::status chan"],
        "Query the server for the status of the mail spool.",
    ),
    (
        "pop3::last",
        Arity::exact(1),
        &["pop3::last chan"],
        "Query the server for the last email message read from the spool.",
    ),
    (
        "pop3::retrieve",
        Arity::at_least(1),
        &["pop3::retrieve {chan startIndex}"],
        "Retrieve a range of messages from the server.",
    ),
    (
        "pop3::delete",
        Arity::at_least(1),
        &["pop3::delete {chan startIndex}"],
        "Delete a range of messages from the server.",
    ),
    (
        "pop3::list",
        Arity::at_least(1),
        &["pop3::list chan"],
        "Returns the scan listing of the mailbox.",
    ),
    (
        "pop3::top",
        Arity::exact(3),
        &["pop3::top chan msg n"],
        "Optional POP3 command, not all servers may support this.",
    ),
    (
        "pop3::uidl",
        Arity::at_least(1),
        &["pop3::uidl chan"],
        "Optional POP3 command, not all servers may support this.",
    ),
    (
        "pop3::capa",
        Arity::exact(1),
        &["pop3::capa chan"],
        "Optional POP3 command, not all servers may support this.",
    ),
    (
        "pop3::close",
        Arity::exact(1),
        &["pop3::close chan"],
        "Gracefully close the connect after sending a POP3 QUIT command down the socket.",
    ),
];

/// The `irc` package.
const IRC_CMDS: &[Row] = &[
    (
        "irc::config",
        Arity::at_least(0),
        &["irc::config key value"],
        "Sets configuration key to value.",
    ),
    (
        "irc::connection",
        Arity::exact(0),
        &["irc::connection"],
        "The command creates a new object to deal with an IRC connection.",
    ),
    (
        "irc::connections",
        Arity::exact(0),
        &["irc::connections"],
        "Returns a list of all the current connections that were created with connection.",
    ),
];

/// The `uevent` package.
const UEVENT_CMDS: &[Row] = &[
    (
        "uevent::bind",
        Arity::exact(3),
        &["uevent::bind tag event command"],
        "Using this command registers the command prefix to be triggered when the event occurs for the tag.",
    ),
    (
        "uevent::unbind",
        Arity::exact(1),
        &["uevent::unbind token"],
        "This command releases the event binding represented by the token.",
    ),
    (
        "uevent::generate",
        Arity::at_least(2),
        &["uevent::generate tag event"],
        "This command generates an event for the tag, triggering all commands bound to that combination.",
    ),
    (
        "uevent::list",
        Arity::at_least(0),
        &["uevent::list"],
        "In this form the command returns a list containing the names of all tags which have events with commands bound to.",
    ),
    (
        "uevent::watch::tag::add",
        Arity::exact(2),
        &["uevent::watch::tag::add pattern command"],
        "This command sets up a sort of reverse events.",
    ),
    (
        "uevent::watch::tag::remove",
        Arity::exact(1),
        &["uevent::watch::tag::remove token"],
        "This command removes a watcher for (un)bind events on tags.",
    ),
    (
        "uevent::watch::event::add",
        Arity::exact(3),
        &["uevent::watch::event::add tag_pattern event_pattern command"],
        "This command sets up a sort of reverse events.",
    ),
    (
        "uevent::watch::event::remove",
        Arity::exact(1),
        &["uevent::watch::event::remove token"],
        "This command removes a watcher for (un)bind events on tag/event combinations.",
    ),
];

/// Command-prefix callback positions for commands in the flat [`Row`] tables
/// above (which cannot carry a `command_prefixes` field), applied by name in
/// [`specs`].  Index is relative to the args after the command name.
///
/// Ground truth: the `log` man page pins the message-writer command to
/// `cmd level text` (Exactly(2)); `uevent::bind`'s handler is triggered as
/// `command tag event ?details?` — details is optional, so AtLeast(2).
const PREFIX_OVERRIDES: &[(&str, &[(u8, AppendedArity)])] = &[
    ("log::lvCmd", &[(1, AppendedArity::Exactly(2))]),
    ("log::lvCmdForall", &[(0, AppendedArity::Exactly(2))]),
    ("uevent::bind", &[(2, AppendedArity::AtLeast(2))]),
];

/// All client / logging package command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = Vec::new();
    specs.extend(rows("log", LOG_CMDS));
    specs.extend(rows("ftp", FTP_CMDS));
    specs.extend(rows("ldap", LDAP_CMDS));
    specs.extend(rows("pop3", POP3_CMDS));
    specs.extend(rows("irc", IRC_CMDS));
    specs.extend(rows("uevent", UEVENT_CMDS));
    // Patch in the callback positions the flat `Row` tables can't express.
    for spec in &mut specs {
        if let Some(&(_, prefixes)) = PREFIX_OVERRIDES.iter().find(|&&(n, _)| n == spec.name) {
            spec.command_prefixes = prefixes;
        }
    }
    specs
}
