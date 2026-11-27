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

//! Source-aware projections of the channel value grammars.

use super::{CHANNEL_OPTIONS, CONFIGURE_OPTIONS, ChannelKind, ChannelValueGrammar};
use crate::InvocationArguments;
use crate::literal_validation::{
    LiteralArgumentIssue, LiteralArgumentIssueReason, LiteralArgumentValidation,
    LiteralValidationDecline,
};
use tcl_dialect::TclVersion;

/// A value grammar's answer, independent of channel existence or handler entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelValueValidation {
    /// The value is in the declared grammar.
    Valid,
    /// The value violates a release-independent or selected-release grammar.
    Invalid(&'static str),
    /// Release, channel direction or external catalogue information is missing.
    Unknown,
}

impl ChannelValueGrammar {
    /// Check a Tcl value under an explicitly selected release. Custom encoding
    /// names remain unknown because an encoding search path may supply them.
    #[must_use]
    pub fn validate(self, value: &str, version: Option<TclVersion>) -> ChannelValueValidation {
        use ChannelValueValidation::{Invalid, Unknown, Valid};
        match self {
            Self::DriverValue => Unknown,
            Self::Boolean => {
                if tcl_syntax::boolean::parse_boolean_strict(value).is_some() {
                    Valid
                } else {
                    Invalid("expected a strict Tcl boolean")
                }
            }
            Self::Integer => {
                let Some(version) = version else {
                    return Unknown;
                };
                let number =
                    tcl_syntax::number::Numbers::Target(version.number_syntax()).parse_wide(value);
                if number.is_some() {
                    Valid
                } else {
                    Invalid("expected a Tcl wide integer")
                }
            }
            Self::Buffering => {
                let values = super::BUFFERING;
                if !value.is_empty()
                    && values.iter().filter(|word| word.starts_with(value)).count() == 1
                {
                    Valid
                } else {
                    Invalid("expected a unique prefix of a declared option value")
                }
            }
            Self::Profile | Self::InputMode | Self::Handshake => {
                let values = match self {
                    Self::Profile => super::PROFILES,
                    Self::InputMode => super::INPUT_MODES,
                    _ => super::HANDSHAKES,
                };
                if values.contains(&value) {
                    Valid
                } else {
                    Invalid("expected a declared option value")
                }
            }
            Self::Translation => validate_translation(value, version),
            Self::EofChar => validate_eofchar(value, version),
            Self::Encoding => {
                let Some(version) = version else {
                    return Unknown;
                };
                if version.removed_encoding_names().contains(&value) {
                    return Invalid(
                        "not an encoding from Tcl 9.0; use `-translation binary` or `-encoding iso8859-1`",
                    );
                }
                if version.release_facts().encodings.contains(&value)
                    || (value == "binary" && version < TclVersion::V9_0)
                {
                    Valid
                } else {
                    Unknown
                }
            }
        }
    }
}

fn read_list(
    value: &str,
    version: Option<TclVersion>,
) -> Result<Option<Vec<String>>, tcl_syntax::list::ListError> {
    version.map_or_else(
        || Ok(crate::literal_folds::split_list(value)),
        |version| {
            tcl_native_model::InvocationDialect::for_version(version)
                .read_list(value)
                .map(|rows| Some(rows.into_iter().map(std::borrow::Cow::into_owned).collect()))
        },
    )
}

fn validate_translation(value: &str, version: Option<TclVersion>) -> ChannelValueValidation {
    use ChannelValueValidation::{Invalid, Unknown, Valid};
    let elements = match read_list(value, version) {
        Ok(Some(elements)) => elements,
        Ok(None) => return Unknown,
        Err(_) => return Invalid("expected a well-formed translation list"),
    };
    if !(1..=2).contains(&elements.len()) {
        return Invalid("expected one translation mode or an input/output pair");
    }
    if elements.iter().all(|element| {
        ["", "auto", "binary", "cr", "lf", "crlf", "platform"].contains(&element.as_str())
    }) {
        Valid
    } else {
        // Reflected channels may have no I/O direction and ignore both members.
        Unknown
    }
}

fn validate_eofchar(value: &str, version: Option<TclVersion>) -> ChannelValueValidation {
    use ChannelValueValidation::{Invalid, Unknown, Valid};
    let Some(version) = version else {
        return Unknown;
    };
    if version >= TclVersion::V9_0 {
        let bytes = value.as_bytes();
        if bytes.is_empty()
            || (bytes[0].is_ascii() && bytes[0] != 0 && (bytes.len() == 1 || &bytes[1..] == b" {}"))
        {
            Valid
        } else {
            Invalid("expected an empty value or a non-NUL ASCII EOF character")
        }
    } else {
        let elements = match read_list(value, Some(version)) {
            Ok(Some(elements)) => elements,
            Ok(None) => return Unknown,
            Err(_) => return Invalid("expected a well-formed EOF-character list"),
        };
        if elements.len() > 2 {
            return Invalid("expected zero, one or two EOF characters");
        }
        // Tcl 8 uses the first byte of each list member, even if it has more
        // characters. NUL is encoded differently by Tcl and is not ASCII here.
        if elements.iter().all(|element| {
            element
                .as_bytes()
                .first()
                .is_none_or(|byte| byte.is_ascii() && *byte != 0)
        }) {
            Valid
        } else {
            Invalid("expected non-NUL ASCII EOF characters")
        }
    }
}

fn issue(index: usize, subject: &'static str, reason: &'static str) -> LiteralArgumentValidation {
    LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
        argument_index: index,
        subject,
        reason: LiteralArgumentIssueReason::InvalidValue(reason),
        allowed_values: &[],
        replacement_value: None,
    })
}

fn validate_configuration(
    arguments: InvocationArguments<'_>,
    first_option: usize,
) -> LiteralArgumentValidation {
    let Some(count) = arguments.exact_argv_len() else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::IncompleteInvocation);
    };
    if count == first_option || count == first_option + 1 {
        return LiteralArgumentValidation::Valid;
    }
    if count < first_option || !(count - first_option).is_multiple_of(2) {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::IncompleteInvocation);
    }
    let version = arguments.dialect().and_then(|dialect| dialect.tcl_version);
    let mut unknown = false;
    for index in (first_option..count).step_by(2) {
        let Some(name) = arguments.literal_at(index) else {
            return LiteralArgumentValidation::Abstain(
                LiteralValidationDecline::NonLiteralArgument,
            );
        };
        let Some(option) = crate::spec::resolve_option_prefix(CONFIGURE_OPTIONS, name) else {
            return LiteralArgumentValidation::Abstain(
                LiteralValidationDecline::InvalidDiscriminator,
            );
        };
        let row = CHANNEL_OPTIONS
            .iter()
            .find(|row| row.option.name == option.name)
            .expect("selected channel option belongs to its authored table");
        // An unavailable option is owned by the availability diagnostic.
        if let Some(version) = version {
            let query = tcl_dialect::model::SurfaceQuery::core(
                tcl_dialect::model::Family::Tcl,
                version.version_string(),
            );
            if !row
                .option
                .supports_dialect(Some(query), Some(tcl_dialect::model::SpecSurface::ALL_TCL))
            {
                return LiteralArgumentValidation::Abstain(
                    LiteralValidationDecline::InvalidDiscriminator,
                );
            }
        }
        if !row.kinds.contains(&ChannelKind::Generic) {
            // A reflected driver may supply a custom setter with this spelling.
            unknown = true;
            continue;
        }
        let Some(value) = arguments.literal_at(index + 1) else {
            unknown = true;
            continue;
        };
        match row.grammar.validate(value, version) {
            ChannelValueValidation::Valid => {}
            ChannelValueValidation::Unknown => unknown = true,
            ChannelValueValidation::Invalid(reason) => {
                return issue(index + 1, "channel option value", reason);
            }
        }
    }
    if unknown {
        LiteralArgumentValidation::Abstain(LiteralValidationDecline::NonLiteralArgument)
    } else {
        LiteralArgumentValidation::Valid
    }
}

/// Validate the standalone channel configuration spelling.
#[must_use]
pub fn validate_fconfigure_configuration(
    arguments: InvocationArguments<'_>,
) -> LiteralArgumentValidation {
    validate_configuration(arguments, 1)
}

/// Validate the ensemble spelling, retaining the selecting word's offset.
#[must_use]
pub fn validate_chan_configuration(
    arguments: InvocationArguments<'_>,
) -> LiteralArgumentValidation {
    validate_configuration(arguments, 2)
}

fn validate_modes(
    arguments: InvocationArguments<'_>,
    index: usize,
    empty_since: Option<TclVersion>,
) -> LiteralArgumentValidation {
    if arguments.exact_argv_len() != Some(3) {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::IncompleteInvocation);
    }
    let Some(value) = arguments.literal_at(index) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::NonLiteralArgument);
    };
    let version = arguments.dialect().and_then(|dialect| dialect.tcl_version);
    let modes = match read_list(value, version) {
        Ok(Some(modes)) => modes,
        Ok(None) => {
            return LiteralArgumentValidation::Abstain(LiteralValidationDecline::UnselectedGrammar);
        }
        Err(_) => {
            return issue(
                index,
                "channel mode list",
                "expected a well-formed Tcl list",
            );
        }
    };
    if modes.is_empty() {
        if version == Some(TclVersion::V8_4) {
            return LiteralArgumentValidation::Abstain(
                LiteralValidationDecline::InvalidDiscriminator,
            );
        }
        if let Some(since) = empty_since {
            let Some(version) = version else {
                return LiteralArgumentValidation::Abstain(
                    LiteralValidationDecline::UnselectedGrammar,
                );
            };
            if version >= since {
                return LiteralArgumentValidation::Valid;
            }
        }
        return issue(
            index,
            "channel mode list",
            "expected at least one channel mode",
        );
    }
    let invalid: Vec<_> = modes
        .into_iter()
        .filter(|mode| {
            mode.is_empty()
                || ["read", "write"]
                    .iter()
                    .filter(|word| word.starts_with(mode))
                    .count()
                    != 1
        })
        .collect();
    if invalid.is_empty() {
        LiteralArgumentValidation::Valid
    } else {
        LiteralArgumentValidation::Invalid(LiteralArgumentIssue {
            argument_index: index,
            subject: "channel mode list",
            reason: LiteralArgumentIssueReason::InvalidMembers(invalid),
            allowed_values: &["read", "write"],
            replacement_value: None,
        })
    }
}

/// `chan create` permits unique read/write prefixes and, since Tcl 8.6, empty lists.
#[must_use]
pub fn validate_create_modes(arguments: InvocationArguments<'_>) -> LiteralArgumentValidation {
    validate_modes(arguments, 1, Some(TclVersion::V8_6))
}

/// `chan postevent` requires a nonempty reflected-channel event-mask list.
#[must_use]
pub fn validate_postevent_modes(arguments: InvocationArguments<'_>) -> LiteralArgumentValidation {
    validate_modes(arguments, 2, None)
}
