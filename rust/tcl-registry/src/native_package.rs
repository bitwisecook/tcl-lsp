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

//! Package command operand extents and diagnostic metadata.

use tcl_core_types::NameBytes;
use tcl_dialect::TclVersion;
use tcl_syntax::naming::NamePolicyProtocol;

/// A package database key projected from already retained native name units.
///
/// The naming policy selects the package operation's `CString` extent. This
/// type retains its provider independently of the selected bytes: it neither
/// decodes display text nor authenticates a source operand, package table,
/// handler or successful operation. Source consumers retain those receipts
/// separately before supplying the original units and their matching policy.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativePackageNameKey {
    bytes: NameBytes,
    policy: NamePolicyProtocol,
}

impl NativePackageNameKey {
    /// Apply the selected package-name purpose to retained native units.
    #[must_use]
    pub fn from_native_units(original: &[u8], policy: NamePolicyProtocol) -> Self {
        Self {
            bytes: NameBytes::from(policy.recipe().package_key(original).selected()),
            policy,
        }
    }

    /// Exact selected database key, including non-Unicode native units.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }

    /// The independently selected naming provider used by this projection.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Represent this selected package key as a bare manifest source atom.
    /// The complete receiving source configuration and independently selected
    /// native string policy must round-trip the authored Document-channel word.
    /// This supplies no package availability, loader or runtime table evidence.
    #[must_use]
    pub fn manifest_atom(&self, config: tcl_lexer::LexerConfig) -> Option<String> {
        if config.escapes != self.policy.string_protocol().escape_syntax() {
            return None;
        }
        let atom = std::str::from_utf8(self.bytes()).ok()?;
        if atom.is_empty()
            || atom.chars().any(|character| {
                character.is_whitespace()
                    || character.is_control()
                    || matches!(
                        character,
                        ';' | '$' | '[' | ']' | '{' | '}' | '"' | '\\' | '#'
                    )
            })
        {
            return None;
        }
        let image = tcl_lexer::SourceImage::document(atom);
        let end = u32::try_from(image.len()).ok()?;
        let parsed =
            tcl_lexer::native_script_words_in(image, tcl_lexer::Span::new(0, end), config).ok()?;
        if parsed.fatal_tail.is_some()
            || parsed.commands.len() != 1
            || parsed.commands[0].words.len() != 1
        {
            return None;
        }
        let words = crate::native_compiler_words::NativeCompilerWords::capture(
            &parsed.commands[0].words,
            self.policy.string_protocol(),
        )
        .ok()?;
        let roundtrip = Self::from_native_units(words.literal(0)?, self.policy);
        (roundtrip == *self).then(|| atom.to_owned())
    }

    /// Match fixed authored ASCII metadata without decoding the native key.
    #[must_use]
    pub fn matches_ascii(&self, authored: &str) -> bool {
        authored.is_ascii() && self.bytes() == authored.as_bytes()
    }
}

/// Actual engine package command protocol, independent of source grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePackageProtocol {
    /// C Tcl's versioned package database and object command.
    C(TclVersion),
    /// Jim's direct package loader and subcommand dispatcher.
    Jim084,
}

/// Explicit authored package database capability, independent of the host table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredPackageProvider {
    /// The Tcl 8.4 command grammar and an isolated authored database.
    Tcl84Core,
}

impl AuthoredPackageProvider {
    /// Pure dispatcher grammar; this issues no native object or compiler receipt.
    #[must_use]
    pub const fn grammar(self) -> NativePackageProtocol {
        match self {
            Self::Tcl84Core => NativePackageProtocol::C(TclVersion::V8_4),
        }
    }

    /// Select an original member with the authored grammar's counted-word extent.
    /// No native index cache is constructed.
    ///
    /// # Errors
    /// Returns the authored ambiguity or unknown-member diagnostic.
    pub fn select_member(self, original: &[u8]) -> Result<&'static str, Vec<u8>> {
        let words = self
            .grammar()
            .c_members()
            .expect("authored C package grammar");
        select_authored_keyword(original, words).map(|index| words[index])
    }
}

/// Pure authored keyword selection, without a native table or index header.
///
/// # Errors
/// Returns the shared prefix owner's ambiguity or unknown-word diagnostic.
pub fn select_authored_keyword(original: &[u8], words: &[&str]) -> Result<usize, Vec<u8>> {
    tcl_cmd_core::prefix::OptionTable::abbreviating("option", words)
        .index_of(tcl_core_types::c_string_extent(original))
}

/// Outcome of Jim's package subcommand table scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageMemberSelection {
    /// Canonical member from the actual package command table.
    Found(&'static str),
    /// No matching member.
    Unknown,
    /// More than one partial match.
    Ambiguous,
}

/// Jim package dispatcher outcome before a package database operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackageDispatch {
    /// Selected canonical database operation.
    Invoke(&'static str),
    /// Successful dispatcher result, including native help and command lists.
    Result(Vec<u8>),
    /// Native dispatcher lookup failure.
    Error(Vec<u8>),
}

/// Reached package failure purpose, independently of diagnostic text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePackageFailure {
    /// Require found no package.
    RequireMissing,
    /// Present found no package or a version mismatch.
    PresentMissing,
    /// Ifneeded completed without providing its selected version.
    LoaderMissing,
    /// Ifneeded provided a different version.
    LoaderWrongVersion,
    /// Loader or unknown returned an unsupported completion.
    LoaderCompletion,
    /// Require recursively reached an active loader.
    Circularity,
}

impl NativePackageProtocol {
    /// Native usage suffix following the actual original member word.
    #[must_use]
    pub fn member_usage(self, member: &str) -> Option<&'static str> {
        Some(match member {
            "provide" => {
                if self == Self::Jim084 {
                    "name ?version?"
                } else {
                    "package ?version?"
                }
            }
            "require" | "present" => match self {
                Self::Jim084 => "name ?version?",
                Self::C(TclVersion::V8_4) => "?-exact? package ?version?",
                Self::C(TclVersion::V8_5) => "?-exact? package ?requirement...?",
                Self::C(_) => "?-exact? package ?requirement ...?",
            },
            "vsatisfies" => match self {
                Self::C(TclVersion::V8_4) => "version1 version2",
                Self::C(TclVersion::V8_5) => "version requirement requirement...",
                Self::C(_) => "version ?requirement ...?",
                Self::Jim084 => return None,
            },
            "ifneeded" => "package version ?script?",
            "unknown" => "?command?",
            "prefer" => "?latest|stable?",
            "files" | "versions" => "package",
            "vcompare" => "version1 version2",
            "names" => "",
            "forget" => {
                if self == Self::Jim084 {
                    "package ..."
                } else {
                    "?package ...?"
                }
            }
            _ => return None,
        })
    }

    /// Actual C object-command option table, in native diagnostic order.
    #[must_use]
    pub const fn c_members(self) -> Option<&'static [&'static str]> {
        match self {
            Self::C(TclVersion::V8_4) => Some(&[
                "forget",
                "ifneeded",
                "names",
                "present",
                "provide",
                "require",
                "unknown",
                "vcompare",
                "versions",
                "vsatisfies",
            ]),
            Self::C(TclVersion::V8_5 | TclVersion::V8_6) => Some(&[
                "forget",
                "ifneeded",
                "names",
                "prefer",
                "present",
                "provide",
                "require",
                "unknown",
                "vcompare",
                "versions",
                "vsatisfies",
            ]),
            Self::C(TclVersion::V9_0 | TclVersion::V9_1) => Some(&[
                "files",
                "forget",
                "ifneeded",
                "names",
                "prefer",
                "present",
                "provide",
                "require",
                "unknown",
                "vcompare",
                "versions",
                "vsatisfies",
            ]),
            Self::Jim084 => None,
        }
    }

    /// Actual package-preference table declaration, shared by both ports.
    #[must_use]
    pub const fn preference_members(self) -> Option<&'static [&'static str]> {
        match self {
            Self::C(TclVersion::V8_4) | Self::Jim084 => None,
            Self::C(_) => Some(&["latest", "stable"]),
        }
    }

    /// Tcl 8.6 and later retain the first version result object in the package
    /// record. Earlier engines create a fresh string for each query.
    #[must_use]
    pub const fn retains_version_object(self) -> bool {
        matches!(
            self,
            Self::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
        )
    }

    /// Native package-loader source inventory exists in C Tcl 9.
    #[must_use]
    pub const fn tracks_files(self) -> bool {
        matches!(self, Self::C(TclVersion::V9_0 | TclVersion::V9_1))
    }

    /// Version-conflict metadata changed after the legacy package command.
    #[must_use]
    pub const fn conflict_error_code(self) -> &'static [u8] {
        match self {
            Self::C(TclVersion::V8_4 | TclVersion::V8_5) | Self::Jim084 => b"NONE",
            Self::C(_) => b"TCL PACKAGE VERSIONCONFLICT",
        }
    }
    /// Actual native failure metadata, including Present's separate lookup code.
    #[must_use]
    pub fn failure_error_code(self, purpose: NativePackageFailure, name: &[u8]) -> Vec<u8> {
        use NativePackageFailure as Failure;
        if matches!(purpose, Failure::PresentMissing) {
            if matches!(
                self,
                Self::C(TclVersion::V8_5 | TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
            ) {
                let mut code = b"TCL LOOKUP PACKAGE ".to_vec();
                tcl_syntax::list::append_list_element(&mut code, self.c_word(name), true);
                return code;
            }
        } else if matches!(
            self,
            Self::C(TclVersion::V8_6 | TclVersion::V9_0 | TclVersion::V9_1)
        ) {
            return match purpose {
                Failure::RequireMissing => b"TCL PACKAGE UNFOUND".to_vec(),
                Failure::LoaderMissing => b"TCL PACKAGE UNPROVIDED".to_vec(),
                Failure::LoaderWrongVersion => b"TCL PACKAGE WRONGPROVIDE".to_vec(),
                Failure::LoaderCompletion => b"TCL PACKAGE BADRESULT".to_vec(),
                Failure::Circularity => b"TCL PACKAGE CIRCULARITY".to_vec(),
                Failure::PresentMissing => unreachable!(),
            };
        }
        b"NONE".to_vec()
    }

    /// Package tables use native `STRING_KEYS`, including plain-char promotion.
    #[must_use]
    pub const fn hash_recipe(
        self,
        abi: tcl_core_types::NativeHashAbi,
    ) -> Option<tcl_core_types::NativeHashRecipe> {
        use tcl_core_types::NativeHashRecipe;
        match self {
            Self::C(version) => Some(NativeHashRecipe::Tcl {
                promotion: abi.plain_char,
                width: match version {
                    TclVersion::V8_4 | TclVersion::V8_5 | TclVersion::V8_6 => abi.unsigned_int,
                    TclVersion::V9_0 | TclVersion::V9_1 => abi.size_t,
                },
            }),
            Self::Jim084 => match abi.jim_seed {
                Some(seed) => Some(NativeHashRecipe::Jim { seed }),
                None => None,
            },
        }
    }

    /// `Tcl_Init`'s native source-inventory scope, independent of package require.
    #[must_use]
    pub const fn initialization_package(self) -> Option<&'static [u8]> {
        if self.tracks_files() {
            Some(b"tcl")
        } else {
            None
        }
    }

    /// Native registration arity header, before subcommand dispatch.
    #[must_use]
    pub const fn command_usage(self) -> &'static str {
        match self {
            Self::C(_) => "package option ?arg ...?",
            Self::Jim084 => "package subcommand ?arg ...?",
        }
    }

    /// Resolve Jim's exact control words and optional help target. Extra words
    /// after a control are ignored by the native dispatcher.
    #[must_use]
    pub fn jim_dispatch(
        self,
        original: &[u8],
        help_target: Option<&[u8]>,
    ) -> Option<PackageDispatch> {
        if self != Self::Jim084 {
            return None;
        }
        let help = original == b"-help";
        let top_help = || {
            PackageDispatch::Result(
            b"Usage: \"package command ... \", where command is one of: forget, names, provide, require".to_vec(),
        )
        };
        let member = if help {
            let Some(target) = help_target else {
                return Some(top_help());
            };
            target
        } else {
            original
        };
        if member == b"-commands" {
            return Some(PackageDispatch::Result(
                b"forget names provide require".to_vec(),
            ));
        }
        let selected = self.jim_member(member)?;
        Some(match selected {
            PackageMemberSelection::Found(member) if help => {
                let arguments = match member {
                    "forget" => " package ...",
                    "provide" | "require" => " name ?version?",
                    _ => "",
                };
                PackageDispatch::Result(format!("Usage: package {member}{arguments}").into_bytes())
            }
            PackageMemberSelection::Found(member) => PackageDispatch::Invoke(member),
            _ if help => top_help(),
            outcome => PackageDispatch::Error(
                self.jim_member_error(member, outcome == PackageMemberSelection::Ambiguous)?,
            ),
        })
    }

    /// Jim's counted exact match followed by bounded C-string prefix comparison.
    /// The hidden `list` alias participates in lookup, but not diagnostics.
    #[must_use]
    pub fn jim_member(self, original: &[u8]) -> Option<PackageMemberSelection> {
        if self != Self::Jim084 {
            return None;
        }
        let mut partial = None;
        for member in ["forget", "provide", "require", "list", "names"] {
            let bytes = member.as_bytes();
            if original == bytes {
                return Some(PackageMemberSelection::Found(member));
            }
            let equal = (0..original.len()).all(|index| {
                let left = original[index];
                let right = bytes.get(index).copied().unwrap_or(0);
                left == right
            });
            // strncmp stops at a shared NUL, regardless of its remaining bound.
            let shared_nul = original.get(bytes.len()) == Some(&0) && original.starts_with(bytes);
            if equal || shared_nul {
                if partial.is_some() {
                    return Some(PackageMemberSelection::Ambiguous);
                }
                partial = Some(member);
            }
        }
        Some(partial.map_or(
            PackageMemberSelection::Unknown,
            PackageMemberSelection::Found,
        ))
    }

    /// Native Jim package subcommand diagnostic, including the hidden alias policy.
    #[must_use]
    pub fn jim_member_error(self, original: &[u8], ambiguous: bool) -> Option<Vec<u8>> {
        if self != Self::Jim084 {
            return None;
        }
        Some(
            [
                &b"package, "[..],
                if ambiguous { b"ambiguous" } else { b"unknown" },
                b" command \"",
                self.c_word(original),
                b"\": should be forget, names, provide, require",
            ]
            .concat(),
        )
    }
    /// Extent consumed by the package database and C version converter.
    #[must_use]
    pub fn c_word(self, bytes: &[u8]) -> &[u8] {
        tcl_core_types::c_string_extent(bytes)
    }

    /// Metadata for a C package option lookup failure.
    /// Jim's subcommand dispatcher uses its separate diagnostic recipe.
    #[must_use]
    pub fn index_error_code(self, what: &[u8], word: &[u8]) -> Option<Vec<u8>> {
        let Self::C(version) = self else { return None };
        if version == TclVersion::V8_4 {
            return Some(b"NONE".to_vec());
        }
        let mut code = b"TCL LOOKUP INDEX ".to_vec();
        tcl_syntax::list::append_list_element(&mut code, what, true);
        code.push(b' ');
        tcl_syntax::list::append_list_element(&mut code, self.c_word(word), true);
        Some(code)
    }

    /// Metadata for a version or range validation failure.
    #[must_use]
    pub const fn version_error_code(self, range: bool) -> Option<&'static [u8]> {
        match self {
            Self::C(TclVersion::V8_4 | TclVersion::V8_5) => Some(b"NONE"),
            Self::C(_) if range => Some(b"TCL VALUE VERSIONRANGE"),
            Self::C(_) => Some(b"TCL VALUE VERSION"),
            Self::Jim084 => None,
        }
    }
}

impl crate::InvocationDialect {
    /// Authenticate the actual package command protocol.
    #[must_use]
    pub fn native_package_protocol(self) -> Option<NativePackageProtocol> {
        let string = self.native_string_protocol()?;
        match string.tcl_version() {
            Some(version) => Some(NativePackageProtocol::C(version)),
            None if string.is_jim084() => Some(NativePackageProtocol::Jim084),
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        NativePackageNameKey, NativePackageProtocol, PackageDispatch, PackageMemberSelection,
    };

    #[test]
    fn package_name_keys_keep_native_units_and_package_extent_separate() {
        // Implementation contract: naming.package.selected-native-unit-key
        // docs/design/analysis/name-resolution-proofs/selected-native-unit-package-key.md

        use tcl_dialect::TclVersion;
        use tcl_syntax::naming::NamePolicyProtocol;

        let policies = [
            NamePolicyProtocol::authored_tcl(TclVersion::V8_4),
            NamePolicyProtocol::authored_tcl(TclVersion::V8_5),
            NamePolicyProtocol::authored_tcl(TclVersion::V8_6),
            NamePolicyProtocol::authored_tcl(TclVersion::V9_0),
            NamePolicyProtocol::authored_tcl(TclVersion::V9_1),
            NamePolicyProtocol::authored_jim084(),
        ];
        for policy in policies {
            let prefix = NativePackageNameKey::from_native_units(b"Pkg", policy);
            assert_eq!(
                NativePackageNameKey::from_native_units(b"Pkg\0Tail", policy),
                prefix
            );
            assert!(prefix.matches_ascii("Pkg"));
            assert!(!prefix.matches_ascii("pkg"));

            for units in [
                b"Pkg\xc0\x80Tail".as_slice(),
                b"Pkg\xed\xa0\x80",
                b"Pkg\xff",
            ] {
                let key = NativePackageNameKey::from_native_units(units, policy);
                assert_eq!(key.bytes(), units);
                assert_eq!(key.policy(), policy);
                assert_ne!(key, prefix);
            }
        }
        // The same selected bytes do not erase an independently selected
        // engine or authored/native provider from diagnostic/cache identity.
        assert_ne!(
            NativePackageNameKey::from_native_units(b"Pkg", policies[0]),
            NativePackageNameKey::from_native_units(b"Pkg", policies[5]),
        );
    }

    #[test]
    fn jim_dispatch_keeps_counted_controls_and_native_help_selection() {
        let protocol = NativePackageProtocol::Jim084;
        let commands = PackageDispatch::Result(b"forget names provide require".to_vec());
        assert_eq!(
            protocol.jim_dispatch(b"-commands", None),
            Some(commands.clone())
        );
        assert_eq!(
            protocol.jim_dispatch(b"-help", Some(b"-commands")),
            Some(commands)
        );
        assert_eq!(
            protocol.jim_dispatch(b"-help", Some(b"pr")),
            Some(PackageDispatch::Result(
                b"Usage: package provide name ?version?".to_vec()
            ))
        );
        assert_eq!(
            protocol.jim_dispatch(b"-help", Some(b"list")),
            Some(PackageDispatch::Result(b"Usage: package list".to_vec()))
        );
        let top_help = protocol.jim_dispatch(b"-help", None);
        assert_eq!(protocol.jim_dispatch(b"-help", Some(b"")), top_help);
        assert_eq!(protocol.jim_dispatch(b"-help", Some(b"unknown")), top_help);
        assert!(matches!(
            protocol.jim_dispatch(b"-commands\0suffix", None),
            Some(PackageDispatch::Error(_))
        ));
        assert!(matches!(
            protocol.jim_dispatch(b"-hel", None),
            Some(PackageDispatch::Error(_))
        ));
        assert_eq!(
            protocol.jim_member(b"names\0suffix"),
            Some(PackageMemberSelection::Found("names"))
        );
        assert_eq!(
            protocol.jim_member(b"na\0suffix"),
            Some(PackageMemberSelection::Unknown)
        );
        assert_eq!(
            protocol.jim_member(b""),
            Some(PackageMemberSelection::Ambiguous)
        );
        assert_eq!(
            NativePackageProtocol::C(tcl_dialect::TclVersion::V9_1).jim_dispatch(b"-help", None),
            None
        );
    }
}

#[cfg(test)]
mod authored_tests {
    use super::*;

    #[test]
    fn authored_package_selection_uses_tcl84_prefixes_without_native_index_authority() {
        let provider = AuthoredPackageProvider::Tcl84Core;
        assert_eq!(provider.select_member(b"req"), Ok("require"));
        assert_eq!(provider.select_member(b"prov"), Ok("provide"));
        assert_eq!(provider.select_member(b"req\0suffix"), Ok("require"));
        assert!(
            provider
                .select_member(b"pr")
                .unwrap_err()
                .starts_with(b"ambiguous option")
        );
        assert!(
            provider
                .select_member(b"files")
                .unwrap_err()
                .starts_with(b"bad option")
        );
        assert!(provider.select_member(b"prefer").is_err());
        let namespace = [
            "children",
            "code",
            "current",
            "delete",
            "eval",
            "exists",
            "export",
            "forget",
            "import",
            "inscope",
            "origin",
            "parent",
            "qualifiers",
            "tail",
            "which",
        ];
        assert!(
            select_authored_keyword(b"e", &namespace)
                .unwrap_err()
                .starts_with(b"ambiguous option")
        );
        assert_eq!(select_authored_keyword(b"cur", &namespace), Ok(2));
        assert!(select_authored_keyword(b"path", &namespace).is_err());
    }
}

#[cfg(test)]
mod manifest_key_tests {
    use super::*;

    #[test]
    fn original_package_manifest_atom_roundtrips_the_selected_key_and_channel() {
        // Implementation contract: naming.package.original-manifest-source-advice
        // docs/design/analysis/name-resolution-proofs/original-package-manifest-source-advice.md
        for version in TclVersion::ALL {
            let policy = NamePolicyProtocol::authored_tcl(version);
            let config = tcl_lexer::LexerConfig::from_grammar(
                crate::InvocationDialect::for_version(version).lexer_grammar,
            );
            let plain = NativePackageNameKey::from_native_units(b"json::write", policy);
            assert_eq!(plain.manifest_atom(config).as_deref(), Some("json::write"));
            assert_eq!(
                NativePackageNameKey::from_native_units(b"json\0tail", policy)
                    .manifest_atom(config)
                    .as_deref(),
                Some("json")
            );
            for bytes in [
                b"p\xc0\x80tail".as_slice(),
                b"p\xed\xa0\x80",
                b"p\xff",
                b"$package",
                b"two words",
                b"bad;command",
                b"line\nname",
            ] {
                assert!(
                    NativePackageNameKey::from_native_units(bytes, policy)
                        .manifest_atom(config)
                        .is_none()
                );
            }
            let mut different = config;
            different.escapes =
                tcl_syntax::native_string::NativeStringProtocol::Jim084.escape_syntax();
            assert!(plain.manifest_atom(different).is_none());
        }
    }
}
