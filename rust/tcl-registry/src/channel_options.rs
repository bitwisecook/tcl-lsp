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

//! Channel-driver options and their value grammars, shared by `chan configure`
//! and `fconfigure`. A driver catalogue is not proof of a live channel's type.

use crate::hover::{ArgValue, OptionArg, OptionSpec, OptionValue};
mod validation;
use crate::lifecycle::Lifecycle;
use tcl_dialect::{
    TclVersion,
    model::{Family, SpecSurface, SurfaceQuery},
};
pub use validation::{
    ChannelValueValidation, validate_chan_configuration, validate_create_modes,
    validate_fconfigure_configuration, validate_postevent_modes,
};

/// A channel driver family. Generic rows apply to every family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelKind {
    /// Generic channel configuration shared by all drivers.
    Generic,
    /// Ordinary file channel.
    File,
    /// TCP client or server socket.
    Socket,
    /// Serial/terminal channel.
    Serial,
    /// Platform console channel.
    Console,
    /// User-defined reflected channel; custom options remain open.
    Reflected,
}

/// Whether a driver option may be queried or set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelOptionAccess {
    /// Query and assignment are supported.
    ReadWrite,
    /// Query only.
    Query,
    /// Assignment only.
    Set,
}

/// Grammar of an authored option value; platform-specific formats stay open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelValueGrammar {
    /// Strict Tcl boolean: boolean words or exactly zero or one.
    Boolean,
    /// Tcl wide integer, possibly clamped by the channel setter.
    Integer,
    /// Unique prefix of full, line or none.
    Buffering,
    /// Exact encoding profile name.
    Profile,
    /// Exact console/terminal input mode.
    InputMode,
    /// Platform-specific serial handshaking mode.
    Handshake,
    /// One translation word or an input/output pair of words.
    Translation,
    /// Release-selected EOF character or input/output character pair.
    EofChar,
    /// Name in the reference release's encoding catalogue.
    Encoding,
    /// Driver-specific data whose grammar is not yet described.
    DriverValue,
}

/// One authored channel option, including its driver and value contract.
#[derive(Debug, Clone)]
pub struct ChannelOption {
    /// Shared completion, availability and option-scanning descriptor.
    pub option: OptionSpec,
    /// Driver families supplying this option; Generic applies to every driver.
    pub kinds: &'static [ChannelKind],
    /// Query/set capabilities, separate from the setter's value grammar.
    pub access: ChannelOptionAccess,
    /// Grammar for a literal assigned value.
    pub grammar: ChannelValueGrammar,
}

const BUFFERING: &[&str] = &["full", "line", "none"];
const PROFILES: &[&str] = &["strict", "tcl8", "replace"];
const INPUT_MODES: &[&str] = &["normal", "password", "raw", "reset"];
const HANDSHAKES: &[&str] = &["none", "rtscts", "xonxoff", "dtrdsr"];
const BUFFERING_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "full",
        detail: "Fully buffered.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "line",
        detail: "Line buffered.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "none",
        detail: "Unbuffered.",
        ..ArgValue::DEFAULT
    },
];
const PROFILE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "strict",
        detail: "Report encoding errors.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "tcl8",
        detail: "Tcl 8 compatible conversion.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "replace",
        detail: "Replace invalid input.",
        ..ArgValue::DEFAULT
    },
];
const INPUTMODE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "normal",
        detail: "Normal terminal input.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "password",
        detail: "Input without echo.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "raw",
        detail: "Unprocessed input.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "reset",
        detail: "Restore original input mode (set only).",
        ..ArgValue::DEFAULT
    },
];
const TRANSLATION_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "platform",
        detail: "Platform newline handling.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "auto",
        detail: "Platform/default newline handling.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "lf",
        detail: "Linefeed newline handling.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "cr",
        detail: "Carriage-return newline handling.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "crlf",
        detail: "Carriage-return/linefeed handling.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "binary",
        detail: "No translation; also sets binary encoding and EOF mode.",
        ..ArgValue::DEFAULT
    },
];

const fn described(
    name: &'static str,
    kinds: &'static [ChannelKind],
    access: ChannelOptionAccess,
    grammar: ChannelValueGrammar,
    surface: Option<&'static [SpecSurface]>,
    detail: &'static str,
) -> ChannelOption {
    let value = match grammar {
        ChannelValueGrammar::Boolean => OptionValue::boolean(),
        _ => OptionValue::Takes(OptionArg {
            hint: "value",
            values: match grammar {
                ChannelValueGrammar::Buffering => BUFFERING_VALUES,
                ChannelValueGrammar::Profile => PROFILE_VALUES,
                ChannelValueGrammar::InputMode => INPUTMODE_VALUES,
                ChannelValueGrammar::Translation => TRANSLATION_VALUES,
                _ => &[],
            },
            // The source-aware grammar owns checking, including list forms and
            // prefixes; flat completion vocabulary is deliberately open.
            ..OptionArg::DEFAULT
        }),
    };
    ChannelOption {
        option: OptionSpec {
            name,
            value,
            detail,
            surface,
            // Tcl's generic translation parser accepts -t before consulting
            // driver options such as -timeout and -ttystatus.
            aliases: match grammar {
                ChannelValueGrammar::Translation => &["-t"],
                _ => &[],
            },
            lifecycle: Lifecycle::UNSPECIFIED,
            min_abbrev: None,
            effect: None,
        },
        kinds,
        access,
        grammar,
    }
}

use ChannelKind::{Console, File, Generic, Serial, Socket};
use ChannelOptionAccess::{Query, ReadWrite, Set};
use ChannelValueGrammar::{
    Boolean, Buffering, DriverValue, Handshake, InputMode, Integer, Profile,
};

/// Sole authored list of generic and channel-driver configuration options.
pub static CHANNEL_OPTIONS: [ChannelOption; 27] = [
    described(
        "-blocking",
        &[Generic],
        ReadWrite,
        Boolean,
        None,
        "Whether I/O blocks.",
    ),
    described(
        "-buffering",
        &[Generic],
        ReadWrite,
        Buffering,
        None,
        "full, line or none; unique prefixes are accepted.",
    ),
    described(
        "-buffersize",
        &[Generic],
        ReadWrite,
        Integer,
        None,
        "Integer buffer size, clamped to 1 through 1048576 bytes.",
    ),
    described(
        "-encoding",
        &[Generic],
        ReadWrite,
        ChannelValueGrammar::Encoding,
        None,
        "Channel character encoding.",
    ),
    described(
        "-eofchar",
        &[Generic],
        ReadWrite,
        ChannelValueGrammar::EofChar,
        None,
        "EOF character; an input/output pair is supported through Tcl 8.6.",
    ),
    described(
        "-profile",
        &[Generic],
        ReadWrite,
        Profile,
        Some(SpecSurface::TCL90_PLUS),
        "Encoding conversion profile (Tcl 9.0+).",
    ),
    described(
        "-translation",
        &[Generic],
        ReadWrite,
        ChannelValueGrammar::Translation,
        None,
        "Newline handling, as one word or an input/output pair.",
    ),
    described(
        "-stat",
        &[File],
        Query,
        DriverValue,
        Some(SpecSurface::TCL90_PLUS),
        "File metadata dictionary (Tcl 9.0+).",
    ),
    described(
        "-error",
        &[Socket],
        Query,
        DriverValue,
        None,
        "Pending socket error.",
    ),
    described(
        "-sockname",
        &[Socket],
        Query,
        DriverValue,
        None,
        "Local socket address and port.",
    ),
    described(
        "-peername",
        &[Socket],
        Query,
        DriverValue,
        None,
        "Remote socket address and port.",
    ),
    described(
        "-connecting",
        &[Socket],
        Query,
        Boolean,
        Some(SpecSurface::TCL86_PLUS),
        "Whether asynchronous socket connection is pending.",
    ),
    described(
        "-keepalive",
        &[Socket],
        ReadWrite,
        Boolean,
        Some(SpecSurface::TCL90_PLUS),
        "Socket keepalive (Tcl 9.0+).",
    ),
    described(
        "-nodelay",
        &[Socket],
        ReadWrite,
        Boolean,
        Some(SpecSurface::TCL90_PLUS),
        "Disable Nagle's algorithm (Tcl 9.0+).",
    ),
    described(
        "-mode",
        &[Serial],
        ReadWrite,
        DriverValue,
        None,
        "Serial baud, parity, data and stop bits.",
    ),
    described(
        "-handshake",
        &[Serial],
        Set,
        Handshake,
        None,
        "Serial handshaking; supported modes depend on the platform.",
    ),
    described(
        "-queue",
        &[Serial],
        Query,
        DriverValue,
        None,
        "Serial input and output queue sizes.",
    ),
    described(
        "-timeout",
        &[Serial],
        ReadWrite,
        Integer,
        None,
        "Serial read timeout in milliseconds.",
    ),
    described(
        "-ttycontrol",
        &[Serial],
        Set,
        DriverValue,
        None,
        "Serial signal/boolean pairs.",
    ),
    described(
        "-ttystatus",
        &[Serial],
        Query,
        DriverValue,
        None,
        "Serial signal status.",
    ),
    described(
        "-xchar",
        &[Serial],
        ReadWrite,
        DriverValue,
        None,
        "XON and XOFF characters.",
    ),
    described(
        "-closemode",
        &[Serial],
        ReadWrite,
        DriverValue,
        Some(SpecSurface::TCL90_PLUS),
        "Windows serial close policy (Tcl 9.0+).",
    ),
    described(
        "-pollinterval",
        &[Serial],
        ReadWrite,
        Integer,
        None,
        "Windows serial polling interval.",
    ),
    described(
        "-sysbuffer",
        &[Serial],
        ReadWrite,
        DriverValue,
        None,
        "Windows serial input/output buffer sizes.",
    ),
    described(
        "-lasterror",
        &[Serial],
        Query,
        DriverValue,
        None,
        "Windows serial error details.",
    ),
    described(
        "-inputmode",
        &[Serial, Console],
        ReadWrite,
        InputMode,
        Some(SpecSurface::TCL90_PLUS),
        "Console or terminal input mode (Tcl 9.0+).",
    ),
    described(
        "-winsize",
        &[Serial, Console],
        Query,
        DriverValue,
        Some(SpecSurface::TCL90_PLUS),
        "Console or terminal window dimensions.",
    ),
];

const fn project_options() -> [OptionSpec; CHANNEL_OPTIONS.len()] {
    let mut options = [const {
        OptionSpec {
            name: "",
            value: OptionValue::Flag,
            detail: "",
            surface: None,
            aliases: &[],
            lifecycle: Lifecycle::UNSPECIFIED,
            min_abbrev: None,
            effect: None,
        }
    }; CHANNEL_OPTIONS.len()];
    let mut index = 0;
    while index < CHANNEL_OPTIONS.len() {
        let option = &CHANNEL_OPTIONS[index].option;
        options[index] = OptionSpec {
            name: option.name,
            value: option.value,
            detail: option.detail,
            surface: option.surface,
            aliases: option.aliases,
            lifecycle: option.lifecycle,
            min_abbrev: option.min_abbrev,
            effect: option.effect,
        };
        index += 1;
    }
    options
}

static CONFIGURE_OPTION_ROWS: [OptionSpec; CHANNEL_OPTIONS.len()] = project_options();

/// Compatibility projection of the single authored driver table. Both command
/// spellings and the ensemble's completion vocabulary share this exact slice.
pub static CONFIGURE_OPTIONS: &[OptionSpec] = &CONFIGURE_OPTION_ROWS;

/// Driver-specific rows plus generic rows, respecting the selected release.
pub fn options_for(
    kind: ChannelKind,
    version: TclVersion,
) -> impl Iterator<Item = &'static ChannelOption> {
    let query = SurfaceQuery::core(Family::Tcl, version.version_string());
    CHANNEL_OPTIONS.iter().filter(move |row| {
        (row.kinds.contains(&ChannelKind::Generic) || row.kinds.contains(&kind))
            && row
                .option
                .supports_dialect(Some(query), Some(SpecSurface::ALL_TCL))
    })
}

#[cfg(test)]
mod tests;
