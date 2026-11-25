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

//! `fconfigure` — set and get options on a channel.
//!
//! Present in every Tcl version from 8.4 through 9.1 (unlike the `chan`
//! ensemble, which is 8.5+ — see `chan_.rs`). Tcl 9.0/9.1's own
//! `fconfigure.n` page has been reduced to a two-line stub — "The
//! `fconfigure` command has been superceded by the `chan configure`
//! command which supports the same syntax and options" — so the 9.0/9.1
//! facts captured below come from `chan.n`'s `configure` subcommand
//! section (confirmed identical option surface) rather than from
//! `fconfigure.n` itself for those two versions.
//!
//! The `ALL_TCL` surface on the command spec below is deliberate, not an
//! oversight: iRules bans `fconfigure`, and `ALL_TCL` does not carry an iRules
//! row, so this spec is simply absent under iRules — the same pattern
//! `chan_.rs` documents for `chan`.

use crate::channel_options::CONFIGURE_OPTIONS;
use crate::prelude::*;
use tcl_cmd_core::CmdError;
use tcl_cmd_core::channel::ConfigOption;
use tcl_cmd_core::prefix::OptionTable;
use tcl_dialect::DialectProfile;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "fconfigure channelId ?optionName? ?value ...?",
    ..FormSpec::DEFAULT
}];

fn runtime_option(name: &str) -> Option<ConfigOption> {
    match name {
        "-blocking" => Some(ConfigOption::Blocking),
        "-buffering" => Some(ConfigOption::Buffering),
        "-buffersize" => Some(ConfigOption::BufferSize),
        "-encoding" => Some(ConfigOption::Encoding),
        "-eofchar" => Some(ConfigOption::EofChar),
        "-profile" => Some(ConfigOption::Profile),
        "-translation" => Some(ConfigOption::Translation),
        _ => None,
    }
}

/// Resolve an `fconfigure` option from the command registry's dialect-filtered
/// descriptors, returning the typed operation consumed by both runtimes.
///
/// Channel-driver-specific options remain in the registry but are excluded
/// here until a concrete runtime channel advertises the matching driver.
///
/// # Errors
/// Tcl's canonical bad/ambiguous-option result for the active profile.
pub fn resolve_fconfigure_option(
    profile: &'static DialectProfile,
    word: &str,
) -> Result<ConfigOption, CmdError> {
    let query = Some(profile.surface_query());
    let available: Vec<&str> = CONFIGURE_OPTIONS
        .iter()
        .filter(|option| {
            runtime_option(option.name).is_some()
                && option.supports_dialect(query, Some(SpecSurface::ALL_TCL))
        })
        .map(|option| option.name)
        .collect();
    let index = OptionTable::abbreviating("option", &available).index_of_str(word)?;
    Ok(runtime_option(available[index]).expect("runtime option list was filtered by this mapping"))
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fconfigure",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("fconfigure"),
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::stepped(1, Arity::UNLIMITED, 2).with_also_exact(2),
        arg_roles: &[(0, ArgRole::Channel)],
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        options: CONFIGURE_OPTIONS,
        channel_configuration: Some(ChannelConfigurationSpec::EncodingTranslation),
        literal_argument_validator: Some(crate::channel_options::validate_fconfigure_configuration),
        option_prefix_words: 1,
        hover: Some(HoverSnippet {
            summary: "Set and get options on a channel.",
            synopsis: &[
                "fconfigure channelId",
                "fconfigure channelId optionName",
                "fconfigure channelId optionName value ?optionName value ...?",
            ],
            snippet: "With no optionName or value, returns a list of alternating option names and values for the channel. With optionName alone, returns that option's current value. With one or more optionName/value pairs, sets each option and returns the empty string. -blocking, -buffering, -buffersize, -encoding, -eofchar, and -translation apply to every channel; -profile is Tcl 9.0+; -nodelay/-keepalive only apply to socket channels (Tcl 9.0+) and -inputmode only to serial/console channels (Tcl 9.0+). Each channel type may also add its own options beyond this generic set — see socket(n) and open(n). Superseded by chan configure (Tcl 8.5+), which accepts the same syntax and options; fconfigure itself remains available as a compatibility form.",
            source: "Tcl fconfigure(n)",
            examples: "fconfigure stdout -buffering none\nfconfigure $sock -blocking 0\nset settings [fconfigure $chan]\nfconfigure $f -translation binary -eofchar {}",
            return_value: "A list of alternating option/value pairs, a single option's value, or the empty string, depending on how many arguments follow channelId.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_options_are_resolved_from_the_active_registry_surface() {
        let tcl9 = DialectProfile::find("tcl9.0").expect("Tcl 9 profile");
        let tcl8 = DialectProfile::find("tcl8.6").expect("Tcl 8 profile");
        assert_eq!(
            resolve_fconfigure_option(tcl9, "-prof"),
            Ok(ConfigOption::Profile)
        );
        assert_eq!(
            resolve_fconfigure_option(tcl8, "-prof")
                .expect_err("Tcl 8 has no profile option")
                .message()
                .unwrap(),
            "bad option \"-prof\": must be -blocking, -buffering, -buffersize, -encoding, -eofchar, or -translation"
        );
    }
}
