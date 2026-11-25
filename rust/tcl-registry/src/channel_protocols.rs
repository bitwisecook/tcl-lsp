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

//! Fixed host callback protocols for channels. These describe possible calls;
//! they establish neither installed handlers nor actual callback entry.

use crate::hover::{OptionArg, OptionValue};
use crate::{AppendedArity, AppendedAritySet};
use tcl_dialect::TclVersion;

/// Protocol selected by an authored callback operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelCallbackProtocol {
    /// Reflected channel created with `chan create`.
    Reflected,
    /// Reflected transformation installed with `chan push`.
    Transform,
    /// Accepted socket notification.
    SocketServer,
    /// Background channel-copy completion, with an optional error message.
    Copy,
}

/// Condition under which a handler must advertise a method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodRequirement {
    /// Every handler implements this method.
    Always,
    /// Required when the reflected channel is readable.
    Readable,
    /// Required when the reflected channel is writable.
    Writable,
    /// The handler may omit this method.
    Optional,
}

/// One method and its fixed words appended after the callback prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelMethod {
    /// Selector appended as the first callback word, or an outcome label for
    /// protocols without a method selector.
    pub name: &'static str,
    /// All appended words, including method and handle for dispatched methods.
    pub appended_words: &'static [&'static str],
    /// When this method must be advertised.
    pub requirement: MethodRequirement,
    /// Earliest Tcl release with this method.
    pub min_tcl: TclVersion,
    /// Other methods that must also be advertised when this one is present.
    pub requires: &'static [&'static str],
}

impl ChannelMethod {
    /// Number of words added after the prefix's already-bound operands.
    #[must_use]
    pub const fn appended_count(self) -> u8 {
        let mut count = 0u8;
        let mut index = 0;
        while index < self.appended_words.len() {
            count += 1;
            index += 1;
        }
        count
    }
}

const REFLECTED: &[ChannelMethod] = &[
    method(
        "initialize",
        &["initialize", "channel", "mode"],
        MethodRequirement::Always,
        &[],
    ),
    method(
        "finalize",
        &["finalize", "channel"],
        MethodRequirement::Always,
        &[],
    ),
    method(
        "watch",
        &["watch", "channel", "events"],
        MethodRequirement::Always,
        &[],
    ),
    method(
        "read",
        &["read", "channel", "count"],
        MethodRequirement::Readable,
        &[],
    ),
    method(
        "write",
        &["write", "channel", "data"],
        MethodRequirement::Writable,
        &[],
    ),
    method(
        "seek",
        &["seek", "channel", "offset", "base"],
        MethodRequirement::Optional,
        &[],
    ),
    method(
        "configure",
        &["configure", "channel", "option", "value"],
        MethodRequirement::Optional,
        &[],
    ),
    method(
        "cget",
        &["cget", "channel", "option"],
        MethodRequirement::Optional,
        &["cgetall"],
    ),
    method(
        "cgetall",
        &["cgetall", "channel"],
        MethodRequirement::Optional,
        &["cget"],
    ),
    method(
        "blocking",
        &["blocking", "channel", "mode"],
        MethodRequirement::Optional,
        &[],
    ),
    ChannelMethod {
        min_tcl: TclVersion::V9_0,
        ..method(
            "truncate",
            &["truncate", "channel", "length"],
            MethodRequirement::Optional,
            &[],
        )
    },
];
const TRANSFORM: &[ChannelMethod] = &[
    transform_method(
        "initialize",
        &["initialize", "handle", "mode"],
        MethodRequirement::Always,
        &[],
    ),
    transform_method(
        "finalize",
        &["finalize", "handle"],
        MethodRequirement::Always,
        &[],
    ),
    transform_method(
        "read",
        &["read", "handle", "data"],
        MethodRequirement::Optional,
        &[],
    ),
    transform_method(
        "write",
        &["write", "handle", "data"],
        MethodRequirement::Optional,
        &[],
    ),
    transform_method(
        "drain",
        &["drain", "handle"],
        MethodRequirement::Optional,
        &["read"],
    ),
    transform_method(
        "flush",
        &["flush", "handle"],
        MethodRequirement::Optional,
        &["write"],
    ),
    transform_method(
        "clear",
        &["clear", "handle"],
        MethodRequirement::Optional,
        &[],
    ),
    transform_method(
        "limit?",
        &["limit?", "handle"],
        MethodRequirement::Optional,
        &[],
    ),
];
const SOCKET_SERVER: &[ChannelMethod] = &[ChannelMethod {
    min_tcl: TclVersion::V8_4,
    ..method(
        "accepted",
        &["channel", "address", "port"],
        MethodRequirement::Always,
        &[],
    )
}];
const COPY: &[ChannelMethod] = &[
    ChannelMethod {
        min_tcl: TclVersion::V8_4,
        ..method("success", &["count"], MethodRequirement::Always, &[])
    },
    ChannelMethod {
        min_tcl: TclVersion::V8_4,
        ..method(
            "error",
            &["count", "message"],
            MethodRequirement::Always,
            &[],
        )
    },
];

const fn method(
    name: &'static str,
    appended_words: &'static [&'static str],
    requirement: MethodRequirement,
    requires: &'static [&'static str],
) -> ChannelMethod {
    ChannelMethod {
        name,
        appended_words,
        requirement,
        min_tcl: TclVersion::V8_5,
        requires,
    }
}

const fn transform_method(
    name: &'static str,
    appended_words: &'static [&'static str],
    requirement: MethodRequirement,
    requires: &'static [&'static str],
) -> ChannelMethod {
    ChannelMethod {
        min_tcl: TclVersion::V8_6,
        ..method(name, appended_words, requirement, requires)
    }
}

const fn required_counts(methods: &[ChannelMethod]) -> [u8; 2] {
    let mut min = u8::MAX;
    let mut max = 0;
    let mut index = 0;
    while index < methods.len() {
        if matches!(methods[index].requirement, MethodRequirement::Always) {
            let count = methods[index].appended_count();
            if count < min {
                min = count;
            }
            if count > max {
                max = count;
            }
        }
        index += 1;
    }
    [min, max]
}
const REFLECTED_COUNTS: [u8; 2] = required_counts(REFLECTED);
const TRANSFORM_COUNTS: [u8; 2] = required_counts(TRANSFORM);
const COPY_COUNTS: [u8; 2] = required_counts(COPY);

impl ChannelCallbackProtocol {
    /// Stable protocol label for documentation and tools.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Reflected => "refchan",
            Self::Transform => "transchan",
            Self::SocketServer => "socket-server",
            Self::Copy => "channel-copy",
        }
    }

    /// Authored method signatures, including their availability and requirements.
    #[must_use]
    pub const fn methods(self) -> &'static [ChannelMethod] {
        match self {
            Self::Reflected => REFLECTED,
            Self::Transform => TRANSFORM,
            Self::SocketServer => SOCKET_SERVER,
            Self::Copy => COPY,
        }
    }

    /// Method signatures available in the selected reference release.
    pub fn methods_for(self, version: TclVersion) -> impl Iterator<Item = &'static ChannelMethod> {
        self.methods().iter().filter(move |method| {
            method.min_tcl <= version && (self != Self::Transform || version >= TclVersion::V8_6)
        })
    }

    /// Whether a method advertisement must preserve at least one I/O direction.
    /// Reflected channels instead require each requested mode's method and may
    /// be created with an empty mode list.
    #[must_use]
    pub const fn needs_accessible_direction(self) -> bool {
        matches!(self, Self::Transform)
    }

    /// Counts every handler must accept. Optional methods are checked only
    /// after their advertised presence is independently known.
    #[must_use]
    pub const fn appended_arity(self) -> AppendedArity {
        match self {
            Self::Reflected => {
                AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&REFLECTED_COUNTS))
            }
            Self::Transform => {
                AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&TRANSFORM_COUNTS))
            }
            Self::SocketServer => AppendedArity::Exactly(SOCKET_SERVER[0].appended_count()),
            Self::Copy => AppendedArity::OneOf(AppendedAritySet::from_sorted_unique(&COPY_COUNTS)),
        }
    }

    /// Deferred callback option whose arity is derived from this protocol.
    #[must_use]
    pub const fn option(self, hint: &'static str) -> OptionValue {
        OptionValue::Takes(OptionArg {
            role: crate::ArgRole::CommandPrefix,
            script_timing: crate::ScriptTiming::Deferred,
            hint,
            callback_protocol: Some(self),
            appended_arity: self.appended_arity(),
            ..OptionArg::DEFAULT
        })
    }
}

#[cfg(test)]
mod tests;
