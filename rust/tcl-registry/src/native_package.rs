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

use tcl_dialect::TclVersion;

/// Actual engine package command protocol, independent of source grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativePackageProtocol {
    /// C Tcl's versioned package database and object command.
    C(TclVersion),
    /// Jim's direct package loader and subcommand dispatcher.
    Jim084,
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
    use super::{NativePackageProtocol, PackageDispatch, PackageMemberSelection};

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
