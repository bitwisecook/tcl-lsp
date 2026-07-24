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

//! `socket` — open a TCP client or server socket channel.
use crate::prelude::*;

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-myaddr",
        value: OptionValue::value("addr"),
        detail: "Local network interface, by domain name or IP address: the client-side interface to originate from (client sockets), or the interface to listen on (server sockets). Defaults to a system-chosen interface for a client socket, or the wildcard address (all interfaces) for a server socket.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-myport",
        value: OptionValue::value("port"),
        detail: "Client sockets only: local port to use for the connection. Defaults to a system-assigned port.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-async",
        value: OptionValue::flag(),
        detail: "Client sockets only: create the socket and return immediately instead of waiting for the connection to complete. A gets or flush issued before it completes then either blocks (blocking channel) or fails with fblocked returning 1 (nonblocking channel).",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-server",
        // Accept callback invoked as `command channel addr port` → 3 args.
        value: OptionValue::command_prefix_n("command", AppendedArity::Exactly(3)),
        detail: "Create a listening server socket instead of a client connection. command (a command prefix) is invoked as \"command channel clientAddr clientPort\" for each accepted connection.",
        dialects: None,
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-reuseaddr",
        value: OptionValue::value("boolean"),
        detail: "Server sockets only, Tcl 9.0+: whether the kernel may reuse the local address when no socket is actively listening on it (SO_REUSEADDR). True is the default on Windows; the default on other platforms is undocumented.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
    OptionSpec {
        name: "-reuseport",
        value: OptionValue::value("boolean"),
        detail: "Server sockets only, Tcl 9.0+: whether multiple sockets may bind the same local address and port (SO_REUSEPORT). The default value is undocumented.",
        dialects: Some(DialectSet::TCL90_PLUS),
        aliases: &[],
        min_version: None,
    },
];

const FORMS: &[FormSpec] = &[
    FormSpec {
        kind: FormKind::Default,
        synopsis: "socket ?options? host port",
        dialects: None,
    },
    FormSpec {
        kind: FormKind::Default,
        synopsis: "socket -server command ?options? port",
        dialects: None,
    },
];

/// Command spec for `socket`.
///
/// The core option surface (`-myaddr`/`-myport`/`-async` for client
/// sockets, `-myaddr`/`-server` for server sockets) is unchanged from Tcl
/// 8.4 through 9.1 — every fetched socket manpage (8.4, 8.5, 8.6, 9.0,
/// 9.1: the 8.6 tree only serves `.htm`, the rest `.html`) documents the
/// same two SYNOPSIS forms and the same -async blocking/nonblocking
/// (`fblocked`) semantics essentially word for word, back to 8.4's own
/// SYNOPSIS. Two real deltas exist:
///
/// - `-reuseaddr`/`-reuseport` (server sockets only; TIP 456) are new in
///   Tcl 9.0 — the 8.4, 8.5, and 8.6 SERVER SOCKETS sections each
///   document only `-myaddr` there (confirmed absent by explicit
///   substring search on all three pages), while both 9.0 and 9.1 add
///   `-reuseaddr`/`-reuseport` with byte-identical wording (independently
///   re-verified here via two full-section fetches per version). Default
///   values are narrower than they first appear: the shipped 9.0/9.1
///   manpage text for `-reuseaddr` is exactly "Tells the kernel whether
///   to reuse the local address if there is no socket actively listening
///   on it. This is the default on Windows." — it commits to a default
///   *only* for Windows and is silent on every other platform, so
///   `detail` above does not assert a blanket "defaults to true". (TIP
///   456's own rationale text says the reference implementation
///   "defaults to -reuseaddr set to yes as it was already doing before",
///   i.e. preserving Tcl's old pre-9.0 behaviour of unconditionally
///   setting `SO_REUSEADDR` — background on the *design intent*, not a
///   substitute for what the manpage itself documents.) `-reuseport` has
///   no default stated anywhere on the manpage, for either version.
/// - `host` may be an IPv6 literal (e.g. `2001:DB8::1`) from Tcl 8.6
///   onward; the 8.4/8.5 manpages describe TCP without ever mentioning
///   IPv6, and 8.6's own manpage states outright: "Support for IPv6 was
///   added in Tcl 8.6." This is a positional-argument value-space
///   change, not an option gate, since `host`/`port` carry no
///   [`ArgValue`] enumeration to begin with.
///
/// The channel-level `chan configure`/`fconfigure` options also
/// documented on this manpage (`-error`, `-sockname`, `-peername`,
/// `-connecting` [added 8.6], `-keepalive`/`-nodelay` [added 9.0, TIP
/// 344]) are queried *after* the channel already exists rather than
/// passed to `socket` itself, so they belong to `fconfigure`/`chan
/// configure`'s own option surface, not this `CommandSpec`'s `options`.
///
/// 9.1's socket manpage mirrors 9.0's exactly for every form, option,
/// and configuration entry checked here (confirmed via two independent
/// full-page fetches) — no 9.1-specific delta found for this command.
///
/// `socket` is present in every dialect that hosts a real Tcl core
/// (checked against `tcl-dialect/src/profile.rs`'s per-profile
/// `availability_mask`, which unions in a real Tcl-version bit for
/// every additive dialect — f5-iapps, f5-tmsh, expect, and the EDA
/// vendor shells all resolve it normally) except F5 iRules, which bans
/// it outright: its `dialects` group is `ALL_TCL` (no `IRULES` bit), so
/// it never intersects the bare `IRULES` availability mask — the
/// K36322151 ban falls straight out of that omission, with no separate
/// disable list, the same `ALL_TCL` convention `open` already uses for
/// its own, identical iRules ban.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "socket",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::BYTE_COMPILED
            | Traits::OPENS_CHANNEL
            | Traits::TAINT_SOURCE
            | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(2),
        return_type: Some(TclType::Channel),
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Open a TCP client connection or listening server socket as a channel.",
            synopsis: &[
                "socket ?options? host port",
                "socket ?-myaddr addr? ?-myport port? ?-async? host port",
                "socket -server command ?options? port",
                "socket -server command ?-myaddr addr? ?-reuseaddr boolean? ?-reuseport boolean? port",
            ],
            snippet: "Without -server, opens the client side of a TCP connection to host (a domain name, or an IPv4/IPv6 literal address — IPv6 support was added in Tcl 8.6) and port, returning a channel open for both reading and writing. -myaddr and -myport pick the local interface and port (system-assigned when omitted); -async returns the channel immediately instead of waiting for the connection to complete, so a gets or flush issued first either blocks (blocking channel) or fails with fblocked returning 1 (nonblocking channel) — poll progress with `chan configure $sock -connecting` (Tcl 8.6+) and check the outcome with `chan configure $sock -error` once it settles. With -server, creates a listening socket on port (0 lets the OS pick a free port, discoverable via `chan configure -sockname`) and invokes command — a command prefix — as \"command channel clientAddr clientPort\" for each accepted connection; the server channel itself supports no I/O, only closing it to stop accepting new connections, and needs the event loop (vwait or equivalent) running to accept at all. -reuseaddr and -reuseport (Tcl 9.0+) control SO_REUSEADDR/SO_REUSEPORT on the listening socket, letting a restarted or clustered server rebind the same address/port. -reuseaddr defaults to true on Windows; the default is undocumented for either option on other platforms.",
            source: "Tcl socket(n)",
            examples: "set sock [socket www.example.com 80]\nfconfigure $sock -translation crlf -buffering line\nputs $sock \"GET / HTTP/1.0\"\nputs $sock \"Host: www.example.com\"\nputs $sock \"\"\nflush $sock\nset response [read $sock]\nclose $sock\n\nproc accept {chan addr port} {\n    fconfigure $chan -buffering line\n    fileevent $chan readable [list echoLine $chan]\n}\nproc echoLine {chan} {\n    if {[eof $chan]} {\n        close $chan\n        return\n    }\n    puts $chan [gets $chan]\n}\nsocket -server accept 12345\nvwait forever",
            return_value: "A channel identifier: for a client connection (or a connection accepted by a server socket), a channel open for both reading and writing; for a listening server socket (-server), a channel that supports no I/O and exists only to be closed to stop accepting new connections.",
        }),
        // `host`/`port` are network-address args — SSRF sink
        // (T104).
        taint_network_sink_args: Some(&[0, 1]),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}
