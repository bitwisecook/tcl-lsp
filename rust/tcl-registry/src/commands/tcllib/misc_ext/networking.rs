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

//! Network clients, servers and addressing: autoproxy, comm, dns, ftp, ip, multiplexer, nameserv, nettool, picoirc and smtpd.

use super::{Arity, Row};

/// The `autoproxy` package.
pub(super) const AUTOPROXY_CMDS: &[Row] = &[
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

/// The `comm` package.
pub(super) const COMM_CMDS: &[Row] = &[(
    "comm::comm_send",
    Arity::exact(0),
    &["comm::comm_send"],
    "Invoking this procedure will substitute the Tk send and winfo interps commands with these equivalents that use ::",
)];

/// The `dns` package.
pub(super) const DNS_CMDS: &[Row] = &[(
    "dns::nameservers",
    Arity::exact(0),
    &["dns::nameservers"],
    "Attempts to return a list of the nameservers currently configured for the users system.",
)];

/// The `ftp` package.
pub(super) const FTP_CMDS: &[Row] = &[(
    "ftp::geturl",
    Arity::exact(1),
    &["ftp::geturl url"],
    "This command can be used by the generic command ::uri::geturl (See package uri) to retrieve the contents of ftp u",
)];

/// The `ip` package.
pub(super) const IP_CMDS: &[Row] = &[
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

/// The `multiplexer` package.
pub(super) const MULTIPLEXER_CMDS: &[Row] = &[(
    "multiplexer::create",
    Arity::exact(0),
    &["multiplexer::create"],
    "The create command creates a new multiplexer 'instance'.",
)];

/// The `nameserv` package.
pub(super) const NAMESERV_CMDS: &[Row] = &[
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
pub(super) const NAMESERV__COMMON_CMDS: &[Row] = &[(
    "nameserv::common::port",
    Arity::exact(0),
    &["nameserv::common::port"],
    "The result returned by the command is the id of the default TCP/IP port a nameservice server will listen on, and ",
)];

/// The `nameserv::server` package.
pub(super) const NAMESERV__SERVER_CMDS: &[Row] = &[
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
pub(super) const NETTOOL_CMDS: &[Row] = &[
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

/// The `picoirc` package.
pub(super) const PICOIRC_CMDS: &[Row] = &[(
    "picoirc::post",
    Arity::exact(3),
    &["picoirc::post context channel message"],
    "This should be called to process user input and send it to the server.",
)];

/// The `smtpd` package.
pub(super) const SMTPD_CMDS: &[Row] = &[(
    "smtpd::stop",
    Arity::exact(0),
    &["smtpd::stop"],
    "Halt the server and release the listening socket.",
)];
