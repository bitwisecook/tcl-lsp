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

//! Tcl channel configuration and output encoding.
//!
//! A channel's encoding, conversion profile, and newline translation are one
//! stateful contract shared by every runtime adapter.  This module owns the
//! release-aware defaults and pure text-to-byte conversion; runtimes retain
//! the mutable configuration beside their handles and supply only the raw byte
//! sink.

use tcl_dialect::TclVersion;
use tcl_platform::SystemEncoding;

use crate::CmdError;

/// Tcl's structured POSIX identity for an output conversion failure.
pub const EILSEQ_ERROR_CODE: &str =
    "POSIX EILSEQ {invalid or incomplete multibyte or wide character}";

const EILSEQ_REASON: &str = "invalid or incomplete multibyte or wide character";

/// A channel option understood by `fconfigure`, independent of a runtime's
/// concrete channel table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOption {
    Blocking,
    Buffering,
    BufferSize,
    Encoding,
    EofChar,
    Profile,
    Translation,
}

/// Encodings implemented by the native runtimes' channel boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelEncoding {
    Utf8,
    Iso88591,
    Ascii,
    Unicode,
    /// Tcl 8's `binary` pseudo-encoding (low eight bits of each character).
    Tcl8Binary,
}

impl ChannelEncoding {
    /// Canonical Tcl spelling returned by `fconfigure -encoding`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Utf8 => "utf-8",
            Self::Iso88591 => "iso8859-1",
            Self::Ascii => "ascii",
            Self::Unicode => "unicode",
            Self::Tcl8Binary => "binary",
        }
    }

    fn from_system(encoding: SystemEncoding) -> Self {
        match encoding {
            SystemEncoding::Utf8 => Self::Utf8,
            SystemEncoding::Iso88591 => Self::Iso88591,
            SystemEncoding::Ascii => Self::Ascii,
            SystemEncoding::Unicode => Self::Unicode,
        }
    }
}

/// Resolve an exact supported name for `encoding system`.
///
/// # Errors
/// An unknown encoding, with Tcl's structured lookup identity.
pub fn resolve_system_encoding(value: &str) -> Result<SystemEncoding, CmdError> {
    match value {
        "utf-8" => Ok(SystemEncoding::Utf8),
        "" | "iso8859-1" => Ok(SystemEncoding::Iso88591),
        "ascii" => Ok(SystemEncoding::Ascii),
        "unicode" => Ok(SystemEncoding::Unicode),
        _ => Err(CmdError::with_error_code(
            format!("unknown encoding \"{value}\""),
            tcl_syntax::list::join_list(["TCL", "LOOKUP", "ENCODING", value]),
        )),
    }
}

/// Tcl 9's output conversion profile.  Tcl 8 channels use `tcl8`
/// conversion semantics implicitly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodingProfile {
    Replace,
    Strict,
    Tcl8,
}

impl EncodingProfile {
    /// Canonical Tcl spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replace => "replace",
            Self::Strict => "strict",
            Self::Tcl8 => "tcl8",
        }
    }
}

/// Newline translation retained by a channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTranslation {
    Auto,
    Cr,
    Lf,
    CrLf,
}

/// Which direction(s) one channel exposes. Tcl retains input and output
/// newline modes independently and reports a two-element value for a
/// bidirectional channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDirection {
    Input,
    Output,
    Bidirectional,
}

/// Normalised file-channel facts derived from Tcl's simple or list-form
/// `open` access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenAccess {
    direction: ChannelDirection,
    flags: u8,
}

impl OpenAccess {
    const APPEND: u8 = 1 << 0;
    const TRUNCATE: u8 = 1 << 1;
    const CREATE: u8 = 1 << 2;
    const EXCLUSIVE: u8 = 1 << 3;
    const BINARY: u8 = 1 << 4;

    const fn new(direction: ChannelDirection, flags: u8) -> Self {
        Self { direction, flags }
    }

    /// Whether the mode permits reads.
    #[must_use]
    pub const fn is_readable(self) -> bool {
        matches!(
            self.direction,
            ChannelDirection::Input | ChannelDirection::Bidirectional
        )
    }

    /// Whether the mode permits writes.
    #[must_use]
    pub const fn is_writable(self) -> bool {
        matches!(
            self.direction,
            ChannelDirection::Output | ChannelDirection::Bidirectional
        )
    }

    /// Whether writes append at the end of the file.
    #[must_use]
    pub const fn appends(self) -> bool {
        self.flags & Self::APPEND != 0
    }

    /// Whether opening truncates an existing file.
    #[must_use]
    pub const fn truncates(self) -> bool {
        self.flags & Self::TRUNCATE != 0
    }

    /// Whether opening creates a missing file.
    #[must_use]
    pub const fn creates(self) -> bool {
        self.flags & Self::CREATE != 0
    }

    /// Whether creation fails if the file already exists.
    #[must_use]
    pub const fn is_exclusive(self) -> bool {
        self.flags & Self::EXCLUSIVE != 0
    }

    /// Whether the channel starts with binary translation.
    #[must_use]
    pub const fn is_binary(self) -> bool {
        self.flags & Self::BINARY != 0
    }
}

impl OutputTranslation {
    /// Canonical Tcl spelling returned by `fconfigure -translation`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Cr => "cr",
            Self::Lf => "lf",
            Self::CrLf => "crlf",
        }
    }

    fn line_ending(self) -> &'static str {
        match self {
            Self::Cr => "\r",
            Self::CrLf => "\r\n",
            Self::Auto | Self::Lf => "\n",
        }
    }
}

/// The mutable output-affecting configuration of one Tcl channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelConfig {
    pub encoding: ChannelEncoding,
    pub profile: EncodingProfile,
    pub direction: ChannelDirection,
    pub input_translation: OutputTranslation,
    /// The output direction consumed by [`encode_output`].
    pub translation: OutputTranslation,
}

impl ChannelConfig {
    /// Tcl defaults for a standard channel.  Standard error is replace-mode;
    /// Tcl 9's other standard channels are strict.  Input retains `auto`
    /// translation while output channels are normalised to the platform EOL.
    #[must_use]
    pub fn standard(version: TclVersion, system: SystemEncoding, name: &str) -> Self {
        let profile = if version < TclVersion::V9_0 {
            EncodingProfile::Tcl8
        } else if name == "stderr" {
            EncodingProfile::Replace
        } else {
            EncodingProfile::Strict
        };
        Self {
            encoding: ChannelEncoding::from_system(system),
            profile,
            direction: if name == "stdin" {
                ChannelDirection::Input
            } else {
                ChannelDirection::Output
            },
            input_translation: OutputTranslation::Auto,
            translation: if name == "stdin" {
                OutputTranslation::Auto
            } else {
                platform_translation()
            },
        }
    }

    /// Tcl defaults for a newly opened file channel.
    #[must_use]
    pub fn file(version: TclVersion, system: SystemEncoding, direction: ChannelDirection) -> Self {
        Self {
            encoding: ChannelEncoding::from_system(system),
            profile: if version >= TclVersion::V9_0 {
                EncodingProfile::Strict
            } else {
                EncodingProfile::Tcl8
            },
            direction,
            input_translation: OutputTranslation::Auto,
            translation: platform_translation(),
        }
    }

    /// Tcl defaults for a channel opened with one normalised access mode.
    #[must_use]
    pub fn for_open_access(
        version: TclVersion,
        system: SystemEncoding,
        access: OpenAccess,
    ) -> Self {
        let mut config = Self::file(version, system, access.direction);
        if access.is_binary() {
            config
                .set_translation(version, "binary")
                .expect("the literal binary translation is valid for every Tcl release");
        }
        config
    }

    /// Tcl's query form for `-translation`.
    #[must_use]
    pub fn translation_value(self) -> String {
        match self.direction {
            ChannelDirection::Input => self.input_translation.as_str().to_owned(),
            ChannelDirection::Output => self.translation.as_str().to_owned(),
            ChannelDirection::Bidirectional => tcl_syntax::list::join_list([
                self.input_translation.as_str(),
                self.translation.as_str(),
            ]),
        }
    }

    /// Apply an exact Tcl encoding name.
    ///
    /// # Errors
    /// An unknown encoding, including Tcl 9's removed empty/`binary` spellings.
    pub fn set_encoding(&mut self, version: TclVersion, value: &str) -> Result<(), CmdError> {
        self.encoding = match value {
            "utf-8" => ChannelEncoding::Utf8,
            "iso8859-1" => ChannelEncoding::Iso88591,
            "ascii" => ChannelEncoding::Ascii,
            "unicode" => ChannelEncoding::Unicode,
            "" | "binary" if version < TclVersion::V9_0 => ChannelEncoding::Tcl8Binary,
            "" | "binary" => {
                return Err(CmdError::new(format!(
                    "unknown encoding \"{value}\": No longer supported.\n\tplease use either \"-translation binary\" or \"-encoding iso8859-1\""
                )));
            }
            _ => {
                return Err(CmdError::with_error_code(
                    format!("unknown encoding \"{value}\""),
                    tcl_syntax::list::join_list(["TCL", "LOOKUP", "ENCODING", value]),
                ));
            }
        };
        Ok(())
    }

    /// Apply an exact Tcl 9 conversion profile name.
    ///
    /// # Errors
    /// A non-profile spelling.
    pub fn set_profile(&mut self, value: &str) -> Result<(), CmdError> {
        self.profile = match value {
            "replace" => EncodingProfile::Replace,
            "strict" => EncodingProfile::Strict,
            "tcl8" => EncodingProfile::Tcl8,
            _ => {
                return Err(CmdError::with_error_code(
                    format!("bad profile name \"{value}\": must be replace, strict, or tcl8"),
                    tcl_syntax::list::join_list(["TCL", "ENCODING", "PROFILE", value]),
                ));
            }
        };
        Ok(())
    }

    /// Apply Tcl's one- or two-element translation value to the channel
    /// directions it exposes. `binary` also selects the release's
    /// byte-preserving encoding.
    ///
    /// # Errors
    /// A malformed Tcl list or unknown translation spelling.
    pub fn set_translation(&mut self, version: TclVersion, value: &str) -> Result<(), CmdError> {
        let elements = tcl_syntax::list::split_list(value)
            .map_err(|error| CmdError::new(error.message().to_owned()))?;
        let (input, output) = match elements.as_slice() {
            [one] => (Some(one.as_ref()), Some(one.as_ref())),
            [input, output] => (
                (!input.is_empty()).then(|| input.as_ref()),
                (!output.is_empty()).then(|| output.as_ref()),
            ),
            _ => {
                return Err(CmdError::new(
                    "bad value for -translation: must be a one or two element list",
                ));
            }
        };
        if let Some(input) = input
            && self.direction != ChannelDirection::Output
        {
            self.input_translation = parse_translation(input, true)?;
            if input == "binary" {
                self.encoding = if version >= TclVersion::V9_0 {
                    ChannelEncoding::Iso88591
                } else {
                    ChannelEncoding::Tcl8Binary
                };
            }
        }
        if let Some(output) = output
            && self.direction != ChannelDirection::Input
        {
            self.translation = parse_translation(output, false)?;
            if output == "binary" {
                self.encoding = if version >= TclVersion::V9_0 {
                    ChannelEncoding::Iso88591
                } else {
                    ChannelEncoding::Tcl8Binary
                };
            }
        }
        Ok(())
    }
}

/// Resolve Tcl's simple or list-form `open` access mode once for every runtime.
///
/// # Errors
/// A malformed Tcl list, illegal simple spelling, unknown list flag, missing
/// direction, or conflicting direction flags.
pub fn resolve_open_access_mode(version: TclVersion, access: &str) -> Result<OpenAccess, CmdError> {
    if let Some(simple) = resolve_simple_open_access(access) {
        return Ok(simple);
    }
    if access
        .as_bytes()
        .first()
        .is_some_and(u8::is_ascii_lowercase)
        || access.contains('+')
    {
        return Err(open_mode_error(
            version,
            format!("illegal access mode \"{access}\""),
        ));
    }

    let flags = tcl_syntax::list::split_list(access).map_err(|error| {
        if version >= TclVersion::V9_0 {
            CmdError::with_error_code(error.full_message(access), "TCL OPENMODE INVALID")
        } else {
            CmdError::list(error, access)
        }
    })?;
    let mut direction = None;
    let mut access_flags = 0;
    for flag in flags {
        let flag = flag.as_ref();
        match flag {
            "RDONLY" | "WRONLY" | "RDWR" if version >= TclVersion::V9_0 && direction.is_some() => {
                return Err(open_mode_error(
                    version,
                    format!(
                        "invalid access mode \"{flag}\": modes RDONLY, RDWR, and WRONLY cannot be combined"
                    ),
                ));
            }
            "RDONLY" => direction = Some(ChannelDirection::Input),
            "WRONLY" => direction = Some(ChannelDirection::Output),
            "RDWR" => direction = Some(ChannelDirection::Bidirectional),
            "APPEND" => access_flags |= OpenAccess::APPEND,
            "BINARY" => access_flags |= OpenAccess::BINARY,
            "CREAT" => access_flags |= OpenAccess::CREATE,
            "EXCL" => access_flags |= OpenAccess::EXCLUSIVE,
            "TRUNC" => access_flags |= OpenAccess::TRUNCATE,
            "NOCTTY" | "NONBLOCK" => {}
            _ => {
                let choices = if version >= TclVersion::V9_0 {
                    "APPEND, BINARY, CREAT, EXCL, NOCTTY, NONBLOCK, RDONLY, RDWR, TRUNC, or WRONLY"
                } else {
                    "RDONLY, WRONLY, RDWR, APPEND, BINARY, CREAT, EXCL, NOCTTY, NONBLOCK, or TRUNC"
                };
                return Err(open_mode_error(
                    version,
                    format!("invalid access mode \"{flag}\": must be {choices}"),
                ));
            }
        }
    }
    direction.map_or_else(
        || {
            Err(CmdError::new(
                "access mode must include either RDONLY, RDWR, or WRONLY",
            ))
        },
        |direction| Ok(OpenAccess::new(direction, access_flags)),
    )
}

fn open_mode_error(version: TclVersion, message: String) -> CmdError {
    if version >= TclVersion::V9_0 {
        CmdError::with_error_code(message, "TCL OPENMODE INVALID")
    } else {
        CmdError::new(message)
    }
}

fn resolve_simple_open_access(access: &str) -> Option<OpenAccess> {
    match access {
        "r" => Some(OpenAccess::new(ChannelDirection::Input, 0)),
        "rb" => Some(OpenAccess::new(ChannelDirection::Input, OpenAccess::BINARY)),
        "r+" => Some(OpenAccess::new(ChannelDirection::Bidirectional, 0)),
        "rb+" | "r+b" => Some(OpenAccess::new(
            ChannelDirection::Bidirectional,
            OpenAccess::BINARY,
        )),
        "w" => Some(OpenAccess::new(
            ChannelDirection::Output,
            OpenAccess::TRUNCATE | OpenAccess::CREATE,
        )),
        "wb" => Some(OpenAccess::new(
            ChannelDirection::Output,
            OpenAccess::TRUNCATE | OpenAccess::CREATE | OpenAccess::BINARY,
        )),
        "w+" => Some(OpenAccess::new(
            ChannelDirection::Bidirectional,
            OpenAccess::TRUNCATE | OpenAccess::CREATE,
        )),
        "wb+" | "w+b" => Some(OpenAccess::new(
            ChannelDirection::Bidirectional,
            OpenAccess::TRUNCATE | OpenAccess::CREATE | OpenAccess::BINARY,
        )),
        "a" => Some(OpenAccess::new(
            ChannelDirection::Output,
            OpenAccess::APPEND | OpenAccess::CREATE,
        )),
        "ab" => Some(OpenAccess::new(
            ChannelDirection::Output,
            OpenAccess::APPEND | OpenAccess::CREATE | OpenAccess::BINARY,
        )),
        "a+" => Some(OpenAccess::new(
            ChannelDirection::Bidirectional,
            OpenAccess::APPEND | OpenAccess::CREATE,
        )),
        "ab+" | "a+b" => Some(OpenAccess::new(
            ChannelDirection::Bidirectional,
            OpenAccess::APPEND | OpenAccess::CREATE | OpenAccess::BINARY,
        )),
        _ => None,
    }
}

/// Mutable configuration of Tcl's three standard channel handles.
///
/// The runtime owns the sharing topology; this type owns their names and
/// release-aware initial state so every adapter observes the same defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StandardChannelConfigs {
    stdin: ChannelConfig,
    stdout: ChannelConfig,
    stderr: ChannelConfig,
}

impl StandardChannelConfigs {
    /// Construct the standard handles for one Tcl release and system encoding.
    #[must_use]
    pub fn new(version: TclVersion, system: SystemEncoding) -> Self {
        Self {
            stdin: ChannelConfig::standard(version, system, "stdin"),
            stdout: ChannelConfig::standard(version, system, "stdout"),
            stderr: ChannelConfig::standard(version, system, "stderr"),
        }
    }

    /// Read a standard handle's configuration by its Tcl channel name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<ChannelConfig> {
        match name {
            "stdin" => Some(self.stdin),
            "stdout" => Some(self.stdout),
            "stderr" => Some(self.stderr),
            _ => None,
        }
    }

    /// Replace a standard handle's configuration by its Tcl channel name.
    pub fn set(&mut self, name: &str, config: ChannelConfig) -> bool {
        let target = match name {
            "stdin" => &mut self.stdin,
            "stdout" => &mut self.stdout,
            "stderr" => &mut self.stderr,
            _ => return false,
        };
        *target = config;
        true
    }
}

/// Whether `name` is one of Tcl's predefined standard channel handles.
#[must_use]
pub fn is_standard_channel(name: &str) -> bool {
    matches!(name, "stdin" | "stdout" | "stderr")
}

/// Query one resolved `fconfigure` option.
#[must_use]
pub fn config_value(option: ConfigOption, name: &str, config: ChannelConfig) -> String {
    match option {
        ConfigOption::Blocking => "1".to_owned(),
        ConfigOption::Buffering => channel_buffering(name).to_owned(),
        ConfigOption::BufferSize => "4096".to_owned(),
        ConfigOption::Encoding => config.encoding.as_str().to_owned(),
        ConfigOption::EofChar => String::new(),
        ConfigOption::Profile => config.profile.as_str().to_owned(),
        ConfigOption::Translation => config.translation_value(),
    }
}

/// Query the complete `fconfigure` option/value list for one channel.
#[must_use]
pub fn config_list(version: TclVersion, name: &str, config: ChannelConfig) -> String {
    let mut result = format!(
        "-blocking 1 -buffering {} -buffersize 4096 -encoding {} -eofchar {{}}",
        channel_buffering(name),
        config.encoding.as_str()
    );
    if version >= TclVersion::V9_0 {
        result.push_str(" -profile ");
        result.push_str(config.profile.as_str());
    }
    result.push_str(" -translation ");
    result.push_str(&tcl_syntax::list::list_element(&config.translation_value()));
    result
}

/// Apply one resolved `fconfigure` option to the retained channel state.
///
/// Options outside the byte-conversion subset are accepted for compatibility;
/// their operational backing remains the adapter's responsibility.
///
/// # Errors
/// An invalid encoding, conversion profile, or translation value.
pub fn set_config_value(
    version: TclVersion,
    config: &mut ChannelConfig,
    option: ConfigOption,
    value: &str,
) -> Result<(), CmdError> {
    match option {
        ConfigOption::Encoding => config.set_encoding(version, value),
        ConfigOption::Profile => config.set_profile(value),
        ConfigOption::Translation => config.set_translation(version, value),
        ConfigOption::Blocking
        | ConfigOption::Buffering
        | ConfigOption::BufferSize
        | ConfigOption::EofChar => Ok(()),
    }
}

fn channel_buffering(name: &str) -> &'static str {
    match name {
        "stdin" | "stdout" => "line",
        "stderr" => "none",
        _ => "full",
    }
}

fn parse_translation(value: &str, input: bool) -> Result<OutputTranslation, CmdError> {
    match value {
        "auto" if input => Ok(OutputTranslation::Auto),
        "auto" | "platform" => Ok(platform_translation()),
        "binary" | "lf" => Ok(OutputTranslation::Lf),
        "cr" => Ok(OutputTranslation::Cr),
        "crlf" => Ok(OutputTranslation::CrLf),
        _ => Err(bad_translation()),
    }
}

fn bad_translation() -> CmdError {
    CmdError::new(
        "bad value for -translation: must be one of auto, binary, cr, lf, crlf, or platform",
    )
}

const fn platform_translation() -> OutputTranslation {
    if cfg!(windows) {
        OutputTranslation::CrLf
    } else {
        OutputTranslation::Lf
    }
}

/// Encoded bytes plus an optional conversion failure that occurred after the
/// returned prefix.  Tcl writes the representable prefix before reporting
/// `EILSEQ`; the runtime sink therefore must consume `bytes` even when `error`
/// is present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedOutput {
    pub bytes: Vec<u8>,
    pub error: Option<CmdError>,
}

/// Encode Tcl string text according to one channel's retained configuration.
/// The optional `puts` newline participates in the same newline translation as
/// embedded line feeds and is omitted if conversion fails first.
#[must_use]
pub fn encode_output(text: &str, newline: bool, config: ChannelConfig) -> EncodedOutput {
    let ending = config.translation.line_ending();
    let mut translated = String::with_capacity(text.len() + usize::from(newline));
    for character in text.chars() {
        if character == '\n' {
            translated.push_str(ending);
        } else {
            translated.push(character);
        }
    }
    if newline {
        translated.push_str(ending);
    }

    if config.encoding == ChannelEncoding::Utf8 {
        return EncodedOutput {
            bytes: translated.into_bytes(),
            error: None,
        };
    }

    let mut bytes = Vec::with_capacity(translated.len());
    for character in translated.chars() {
        let point = u32::from(character);
        match config.encoding {
            ChannelEncoding::Iso88591 if point <= 0xff => {
                bytes.push(u8::try_from(point).expect("checked Latin-1 scalar"));
            }
            ChannelEncoding::Ascii if point <= 0x7f => {
                bytes.push(u8::try_from(point).expect("checked ASCII scalar"));
            }
            ChannelEncoding::Tcl8Binary => bytes.push(point.to_le_bytes()[0]),
            ChannelEncoding::Unicode => {
                let mut units = [0_u16; 2];
                for unit in character.encode_utf16(&mut units) {
                    bytes.extend_from_slice(&unit.to_ne_bytes());
                }
            }
            ChannelEncoding::Iso88591 | ChannelEncoding::Ascii
                if config.profile == EncodingProfile::Strict =>
            {
                return EncodedOutput {
                    bytes,
                    error: Some(CmdError::with_error_code(EILSEQ_REASON, EILSEQ_ERROR_CODE)),
                };
            }
            ChannelEncoding::Iso88591 | ChannelEncoding::Ascii => bytes.push(b'?'),
            ChannelEncoding::Utf8 => unreachable!("UTF-8 returned above"),
        }
    }
    EncodedOutput { bytes, error: None }
}

/// Encode a runtime object's Tcl string bytes at the channel boundary.
///
/// Runtime-created Tcl strings are UTF-8 and take the Unicode conversion path.
/// A C embedder can nevertheless supply an opaque invalid-UTF-8 string; the
/// default UTF-8 channel preserves those bytes exactly, while a legacy target
/// interprets each byte as its corresponding U+00XX character.
#[must_use]
pub fn encode_output_bytes(bytes: &[u8], newline: bool, config: ChannelConfig) -> EncodedOutput {
    if let Ok(text) = core::str::from_utf8(bytes) {
        return encode_output(text, newline, config);
    }
    if config.encoding != ChannelEncoding::Utf8 {
        let text: String = bytes.iter().copied().map(char::from).collect();
        return encode_output(&text, newline, config);
    }

    let ending = config.translation.line_ending().as_bytes();
    let mut output = Vec::with_capacity(bytes.len() + usize::from(newline));
    for &byte in bytes {
        if byte == b'\n' {
            output.extend_from_slice(ending);
        } else {
            output.push(byte);
        }
    }
    if newline {
        output.extend_from_slice(ending);
    }
    EncodedOutput {
        bytes: output,
        error: None,
    }
}

/// Add the channel identity to a conversion failure after its representable
/// prefix has been written, preserving the structured error code.
#[must_use]
pub fn channel_output_error(name: &str, error: CmdError) -> CmdError {
    let (message, code) = error.into_parts();
    let message = format!("error writing \"{name}\": {message}");
    match code {
        Some(code) => CmdError::with_error_code(message, code),
        None => CmdError::new(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_translation_is_release_aware() {
        let mut nine = ChannelConfig::file(
            TclVersion::V9_0,
            SystemEncoding::Utf8,
            ChannelDirection::Output,
        );
        nine.set_translation(TclVersion::V9_0, "binary").unwrap();
        assert_eq!(nine.encoding.as_str(), "iso8859-1");
        assert_eq!(nine.translation.as_str(), "lf");

        let mut eight = ChannelConfig::file(
            TclVersion::V8_6,
            SystemEncoding::Utf8,
            ChannelDirection::Output,
        );
        eight.set_translation(TclVersion::V8_6, "binary").unwrap();
        assert_eq!(eight.encoding.as_str(), "binary");
        assert_eq!(eight.profile.as_str(), "tcl8");
    }

    #[test]
    fn strict_conversion_returns_the_written_prefix_and_eilseq() {
        let config = ChannelConfig {
            encoding: ChannelEncoding::Iso88591,
            profile: EncodingProfile::Strict,
            direction: ChannelDirection::Output,
            input_translation: OutputTranslation::Auto,
            translation: OutputTranslation::Lf,
        };
        let output = encode_output("A\u{178}B", true, config);
        assert_eq!(output.bytes, b"A");
        let error = output.error.expect("strict conversion failure");
        assert_eq!(error.message(), EILSEQ_REASON);
        assert_eq!(error.error_code(), Some(EILSEQ_ERROR_CODE));
    }

    #[test]
    fn binary_and_explicit_utf8_are_distinct() {
        let mut config = ChannelConfig::file(
            TclVersion::V9_0,
            SystemEncoding::Utf8,
            ChannelDirection::Output,
        );
        config.set_translation(TclVersion::V9_0, "binary").unwrap();
        assert_eq!(encode_output("ÿA", false, config).bytes, [0xff, b'A']);
        config.set_encoding(TclVersion::V9_0, "utf-8").unwrap();
        assert_eq!(encode_output("ÿA", false, config).bytes, [0xc3, 0xbf, b'A']);
    }

    #[test]
    fn system_and_channel_encoding_errors_retain_tcl_identity() {
        assert_eq!(resolve_system_encoding(""), Ok(SystemEncoding::Iso88591));
        let system = resolve_system_encoding("bogus").expect_err("unknown system encoding");
        assert_eq!(system.message(), "unknown encoding \"bogus\"");
        assert_eq!(system.error_code(), Some("TCL LOOKUP ENCODING bogus"));

        let mut config = ChannelConfig::file(
            TclVersion::V9_0,
            SystemEncoding::Utf8,
            ChannelDirection::Output,
        );
        let profile = config.set_profile("bogus").expect_err("unknown profile");
        assert_eq!(profile.error_code(), Some("TCL ENCODING PROFILE bogus"));
        let binary = config
            .set_encoding(TclVersion::V9_0, "binary")
            .expect_err("Tcl 9 removed binary encoding");
        assert!(binary.message().contains("-translation binary"));
        assert_eq!(binary.error_code(), None);
        let empty = config
            .set_encoding(TclVersion::V9_0, "")
            .expect_err("Tcl 9 removed the empty binary encoding");
        assert!(empty.message().starts_with("unknown encoding \"\""));
        assert!(empty.message().contains("-translation binary"));
        assert_eq!(empty.error_code(), None);

        let mut eight = config;
        eight
            .set_encoding(TclVersion::V8_6, "")
            .expect("Tcl 8 empty encoding selects binary");
        assert_eq!(eight.encoding, ChannelEncoding::Tcl8Binary);
    }

    #[test]
    fn open_access_mode_uses_simple_and_tcl_list_grammars() {
        for access in ["wb", "rb+", "r+b", "RDWR BINARY", "WRONLY {BINARY}"] {
            let resolved = resolve_open_access_mode(TclVersion::V9_0, access)
                .expect("valid binary access mode");
            assert!(resolved.is_binary(), "{access:?}");
        }
        let list = resolve_open_access_mode(TclVersion::V9_0, "RDWR CREAT TRUNC")
            .expect("valid access list");
        assert!(list.is_readable());
        assert!(list.is_writable());
        assert!(list.creates());
        assert!(list.truncates());
        assert!(!list.appends());
        assert!(!list.is_exclusive());
        assert!(!list.is_binary());

        let illegal =
            resolve_open_access_mode(TclVersion::V9_0, "brw").expect_err("illegal simple mode");
        assert_eq!(illegal.error_code(), Some("TCL OPENMODE INVALID"));
        let unknown = resolve_open_access_mode(TclVersion::V9_0, "WRONLY BAD")
            .expect_err("unknown list flag");
        assert_eq!(unknown.error_code(), Some("TCL OPENMODE INVALID"));
        assert!(
            resolve_open_access_mode(TclVersion::V9_0, "RDONLY WRONLY")
                .expect_err("conflicting list flags")
                .message()
                .contains("cannot be combined")
        );
        assert_eq!(
            resolve_open_access_mode(TclVersion::V9_0, "BINARY")
                .expect_err("missing direction")
                .message(),
            "access mode must include either RDONLY, RDWR, or WRONLY"
        );

        let eight = resolve_open_access_mode(TclVersion::V8_6, "RDONLY WRONLY")
            .expect("Tcl 8 uses the last direction flag");
        assert!(!eight.is_readable());
        assert!(eight.is_writable());
        let eight_unknown = resolve_open_access_mode(TclVersion::V8_6, "WRONLY BAD")
            .expect_err("unknown Tcl 8 list flag");
        assert_eq!(eight_unknown.error_code(), None);

        let malformed_nine = resolve_open_access_mode(TclVersion::V9_0, "{RDONLY")
            .expect_err("malformed Tcl 9 access list");
        assert_eq!(malformed_nine.error_code(), Some("TCL OPENMODE INVALID"));
        let malformed_eight = resolve_open_access_mode(TclVersion::V8_6, "{RDONLY")
            .expect_err("malformed Tcl 8 access list");
        assert_eq!(malformed_eight.error_code(), Some("TCL VALUE LIST BRACE"));
    }

    #[test]
    fn opaque_invalid_utf8_remains_byte_exact_on_a_utf8_channel() {
        let config = ChannelConfig::standard(TclVersion::V9_0, SystemEncoding::Utf8, "stdout");
        assert_eq!(
            encode_output_bytes(&[0xff, b'\n', 0x80], true, config).bytes,
            [0xff, b'\n', 0x80, b'\n']
        );
    }

    #[test]
    fn replace_profile_and_crlf_translation_apply_to_all_newlines() {
        let config = ChannelConfig {
            encoding: ChannelEncoding::Ascii,
            profile: EncodingProfile::Replace,
            direction: ChannelDirection::Output,
            input_translation: OutputTranslation::Auto,
            translation: OutputTranslation::CrLf,
        };
        assert_eq!(
            encode_output("A\u{178}\nB", true, config).bytes,
            b"A?\r\nB\r\n"
        );
    }

    #[test]
    fn unicode_uses_native_endian_utf16() {
        let config = ChannelConfig {
            encoding: ChannelEncoding::Unicode,
            profile: EncodingProfile::Strict,
            direction: ChannelDirection::Output,
            input_translation: OutputTranslation::Auto,
            translation: OutputTranslation::Lf,
        };
        let mut expected = Vec::new();
        expected.extend_from_slice(&u16::from(b'A').to_ne_bytes());
        expected.extend_from_slice(&0x0178_u16.to_ne_bytes());
        assert_eq!(encode_output("A\u{178}", false, config).bytes, expected);
    }

    #[test]
    fn bidirectional_translation_retains_both_directions() {
        let mut config = ChannelConfig::file(
            TclVersion::V9_0,
            SystemEncoding::Utf8,
            ChannelDirection::Bidirectional,
        );
        assert_eq!(config.translation_value(), "auto lf");
        config.set_translation(TclVersion::V9_0, "cr {}").unwrap();
        assert_eq!(config.translation_value(), "cr lf");
        config
            .set_translation(TclVersion::V9_0, "binary {}")
            .unwrap();
        assert_eq!(config.translation_value(), "lf lf");
        assert_eq!(config.encoding, ChannelEncoding::Iso88591);
    }

    #[test]
    fn bidirectional_translation_commits_each_direction_in_order() {
        let mut config = ChannelConfig::file(
            TclVersion::V9_0,
            SystemEncoding::Utf8,
            ChannelDirection::Bidirectional,
        );
        assert!(
            config
                .set_translation(TclVersion::V9_0, "cr invalid")
                .is_err()
        );
        assert_eq!(config.translation_value(), "cr lf");
    }
}
