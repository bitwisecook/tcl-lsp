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

//! Model enums shared across BIG-IP kinds.

/// Whether a data-group is stored inline or in an external file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataGroupType {
    /// Inline (`internal`) data-group.
    #[default]
    Internal,
    /// External-file (`external`) data-group.
    External,
}

impl DataGroupType {
    /// The canonical `DataGroupType` member name (`"INTERNAL"` / `"EXTERNAL"`).
    #[must_use]
    pub const fn py_name(self) -> &'static str {
        match self {
            Self::Internal => "INTERNAL",
            Self::External => "EXTERNAL",
        }
    }
}

/// Broad classification of BIG-IP profile types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfileType {
    /// `aimcp`.
    Aimcp,
    /// `http`.
    Http,
    /// `tcp`.
    Tcp,
    /// `udp`.
    Udp,
    /// `client-ssl`.
    ClientSsl,
    /// `server-ssl`.
    ServerSsl,
    /// `ftp`.
    Ftp,
    /// `dns`.
    Dns,
    /// `sip`.
    Sip,
    /// `diameter`.
    Diameter,
    /// `fix`.
    Fix,
    /// `radius`.
    Radius,
    /// `mqtt`.
    Mqtt,
    /// `websocket`.
    Websocket,
    /// `stream`.
    Stream,
    /// `sse`.
    Sse,
    /// `html`.
    Html,
    /// `json`.
    Json,
    /// `rewrite`.
    Rewrite,
    /// `fasthttp`.
    Fasthttp,
    /// `fastl4`.
    Fastl4,
    /// `one-connect`.
    OneConnect,
    /// `persistence`.
    Persistence,
    /// Unclassified / other.
    #[default]
    Other,
}

impl ProfileType {
    /// The canonical `ProfileType` member name (`"HTTP"`, `"CLIENT_SSL"`, …).
    #[must_use]
    pub const fn py_name(self) -> &'static str {
        match self {
            Self::Aimcp => "AIMCP",
            Self::Http => "HTTP",
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
            Self::ClientSsl => "CLIENT_SSL",
            Self::ServerSsl => "SERVER_SSL",
            Self::Ftp => "FTP",
            Self::Dns => "DNS",
            Self::Sip => "SIP",
            Self::Diameter => "DIAMETER",
            Self::Fix => "FIX",
            Self::Radius => "RADIUS",
            Self::Mqtt => "MQTT",
            Self::Websocket => "WEBSOCKET",
            Self::Stream => "STREAM",
            Self::Sse => "SSE",
            Self::Html => "HTML",
            Self::Json => "JSON",
            Self::Rewrite => "REWRITE",
            Self::Fasthttp => "FASTHTTP",
            Self::Fastl4 => "FASTL4",
            Self::OneConnect => "ONE_CONNECT",
            Self::Persistence => "PERSISTENCE",
            Self::Other => "OTHER",
        }
    }
}
