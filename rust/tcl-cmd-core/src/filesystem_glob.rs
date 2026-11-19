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

//! Filesystem `glob` filters applied after the original switch scan.

mod unix_pattern;

pub use unix_pattern::{CGlobHead, GlobBraceError, c_glob_head};

use crate::CmdError;
use tcl_core_types::c_string_extent;
use tcl_dialect::TclVersion;
use tcl_platform::{FileAccess, Filesystem};
use tcl_syntax::value::ValueOps;

const GLOB_OPTIONS: crate::prefix::OptionTable<'static> = crate::prefix::OptionTable::abbreviating(
    "option",
    &[
        "-directory",
        "-join",
        "-nocomplain",
        "-path",
        "-tails",
        "-types",
        "--",
    ],
);

/// A switch already resolved by the filesystem command's own option grammar.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum GlobOption {
    Directory,
    Join,
    NoComplain,
    Path,
    Tails,
    Types,
    End,
}

impl GlobOption {
    /// Index supplied by the C interpreter's native option table lookup.
    #[must_use]
    fn c(index: usize) -> Self {
        [
            Self::Directory,
            Self::Join,
            Self::NoComplain,
            Self::Path,
            Self::Tails,
            Self::Types,
            Self::End,
        ][index]
    }

    /// The installed Jim glob library uses its own switch -glob sequence.
    fn jim(word: &[u8]) -> Result<Self, CmdError> {
        if word.starts_with(b"-d") {
            return Ok(Self::Directory);
        }
        if word.starts_with(b"-n") {
            return Ok(Self::NoComplain);
        }
        if word.starts_with(b"-ta") {
            return Ok(Self::Tails);
        }
        if word == b"--" {
            return Ok(Self::End);
        }
        let mut error = b"bad option \"".to_vec();
        error.extend_from_slice(word);
        error.extend_from_slice(b"\": must be -directory, -nocomplain, -tails, or --");
        Err(CmdError::new_bytes(error))
    }
}

/// Whether the command's search root is a directory or the original
/// path prefix whose last component is literal.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlobBase {
    Directory(usize),
    Path(usize),
}

impl GlobBase {
    #[must_use]
    pub fn index(self) -> usize {
        match self {
            Self::Directory(index) | Self::Path(index) => index,
        }
    }
}

/// Selected switches and the first unconsumed pattern in the original argv.
#[derive(Debug)]
pub struct GlobOptions {
    pub base: Option<GlobBase>,
    pub join: bool,
    pub tails: bool,
    pub no_complain: bool,
    pub types: GlobTypes,
    pub patterns_start: usize,
}

impl GlobOptions {
    /// Convert each original list operand during the option scan. The C
    /// release determines when a missing pattern takes precedence over bad
    /// type members. Jim's Tcl library supplies its own switch grammar.
    pub fn parse<O: ValueOps>(
        ops: &mut O,
        args: &[O::Value],
        version: TclVersion,
        jim: bool,
    ) -> Result<Self, CmdError> {
        let mut result = Self {
            base: None,
            join: false,
            tails: false,
            no_complain: !jim && version >= TclVersion::V9_0,
            types: GlobTypes::default(),
            patterns_start: 0,
        };

        let mut types = Vec::new();
        let mut i = 0;
        while let Some(original) = args.get(i) {
            let word = ops.native_string_bytes(original)?;
            if !word.starts_with(b"-") {
                break;
            }
            let option = if jim {
                GlobOption::jim(&word)?
            } else {
                GlobOption::c(GLOB_OPTIONS.index_of_original(ops, original)?)
            };
            i += 1;
            match option {
                GlobOption::Directory | GlobOption::Path | GlobOption::Types => {
                    let Some(value) = args.get(i) else {
                        let switch = match option {
                            _ if jim => word.as_ref(),
                            GlobOption::Directory => b"-directory",
                            GlobOption::Path => b"-path",
                            _ => b"-types",
                        };
                        let message = [b"missing argument to \"" as &[u8], switch, b"\""].concat();
                        return Err(if jim {
                            CmdError::new_bytes(message)
                        } else {
                            argument_error(version, message, b"TCL ARGUMENT MISSING")
                        });
                    };
                    if option == GlobOption::Types {
                        types = ops.list_elements(value)?;
                    } else {
                        let path = option == GlobOption::Path;
                        if !jim && result.base.is_some() {
                            let legacy = version < TclVersion::V8_6;
                            let message: &[u8] = match (
                                path,
                                matches!(result.base, Some(GlobBase::Path(_))),
                                legacy,
                            ) {
                                (false, false, false) => br#""-directory" may only be used once"#,
                                (false, _, _) => br#""-directory" cannot be used with "-path""#,
                                (true, true, false) => br#""-path" may only be used once"#,
                                (true, _, true) => br#""-path" cannot be used with "-directory""#,
                                (true, _, false) => br#""-path" cannot be used with "-dictionary""#,
                            };
                            return Err(argument_error(
                                version,
                                message.to_vec(),
                                b"TCL OPERATION GLOB BADOPTIONCOMBINATION",
                            ));
                        }
                        result.base = Some(if path {
                            GlobBase::Path(i)
                        } else {
                            GlobBase::Directory(i)
                        });
                    }
                    i += 1;
                }
                GlobOption::Join => result.join = true,
                GlobOption::NoComplain => result.no_complain = true,
                GlobOption::Tails => result.tails = true,
                GlobOption::End => break,
            }
        }
        if i == args.len() && (jim || version < TclVersion::V8_6) {
            return Err(if jim {
                CmdError::new(r#"wrong # args: should be "glob ?options? pattern ?pattern ...?""#)
            } else {
                CmdError::wrong_args("glob ?switches? name ?name ...?")
            });
        }
        if !jim && result.tails && result.base.is_none() {
            return Err(argument_error(
                version,
                br#""-tails" must be used with either "-directory" or "-path""#.to_vec(),
                b"TCL OPERATION GLOB BADOPTIONCOMBINATION",
            ));
        }
        result.types = GlobTypes::from_members(ops, &types, version)?;
        result.patterns_start = i;
        Ok(result)
    }

    /// Traverse Unix filesystem patterns one at a time, in the caller's
    /// order, retaining repeated explicit patterns and brace alternatives.
    pub fn c_unix_matches(
        &self,
        fs: &dyn Filesystem,
        directory: Option<&[u8]>,
        patterns: &[Vec<u8>],
        version: TclVersion,
    ) -> Result<Vec<Vec<u8>>, CmdError> {
        let mut matches = Vec::new();
        for pattern in patterns {
            matches.extend(unix_pattern::c_matches(
                fs, self, directory, pattern, version,
            )?);
        }
        if matches.is_empty()
            && let Some(error) = self.no_match(patterns, version, false)
        {
            return Err(error);
        }
        Ok(matches)
    }

    /// Applied only after traversal produced no matches. Joined patterns
    /// arrive as the one path the command actually searched.
    #[must_use]
    pub fn no_match(
        &self,
        patterns: &[Vec<u8>],
        version: TclVersion,
        jim: bool,
    ) -> Option<CmdError> {
        if self.no_complain {
            return None;
        }
        let mut msg = b"no files matched glob pattern".to_vec();
        if !self.join && patterns.len() != 1 {
            msg.push(b's');
        }
        msg.extend_from_slice(b" \"");
        for (index, pattern) in patterns.iter().enumerate() {
            if index > 0 {
                msg.push(b' ');
            }
            msg.extend_from_slice(if jim {
                pattern
            } else {
                c_string_extent(pattern)
            });
        }
        msg.push(b'"');
        Some(if jim {
            CmdError::new_bytes(msg)
        } else {
            argument_error(version, msg, b"TCL OPERATION GLOB NOMATCH")
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct GlobTypes {
    kinds: Vec<u8>,
    permissions: Vec<FileAccess>,
    pub hidden: bool,
    readonly: bool,
    mac_type: Option<Vec<u8>>,
    mac_creator: Option<Vec<u8>>,
}

fn argument_error(version: TclVersion, message: Vec<u8>, code: &'static [u8]) -> CmdError {
    if version >= TclVersion::V8_6 {
        CmdError::with_error_code_bytes(message, code)
    } else {
        CmdError::new_bytes(message)
    }
}

impl GlobTypes {
    /// Each `-types` value must be a list during option scanning. Only the
    /// last such list has its members interpreted. C reads that list backwards.
    pub fn from_members<O: ValueOps>(
        ops: &mut O,
        members: &[O::Value],
        version: TclVersion,
    ) -> Result<Self, CmdError> {
        let mut types = Self::default();
        for member in members.iter().rev() {
            let original = ops.native_string_bytes(member)?;
            let item = original.as_ref();
            if c_string_extent(item) == b"readonly" {
                types.readonly = true;
            } else if c_string_extent(item) == b"hidden" {
                types.hidden = true;
            } else if let [kind @ (b'b' | b'c' | b'd' | b'p' | b'f' | b'l' | b's')] = item {
                types.kinds.push(*kind);
            } else if let [permission @ (b'r' | b'w' | b'x')] = item {
                types.permissions.push(match permission {
                    b'r' => FileAccess::Read,
                    b'w' => FileAccess::Write,
                    _ => FileAccess::Execute,
                });
            } else {
                let mac_value = if item.len() == 4 {
                    Some((&mut types.mac_type, item.to_vec()))
                } else {
                    match ops.list_elements(member) {
                        Ok(parts) if parts.len() == 3 => {
                            let platform = ops.native_string_bytes(&parts[0])?;
                            let field = ops.native_string_bytes(&parts[1])?;
                            let slot = match (c_string_extent(&platform), c_string_extent(&field)) {
                                (b"macintosh", b"type") => Some(&mut types.mac_type),
                                (b"macintosh", b"creator") => Some(&mut types.mac_creator),
                                _ => None,
                            };
                            if let Some(slot) = slot {
                                Some((slot, ops.native_string_bytes(&parts[2])?.to_vec()))
                            } else {
                                None
                            }
                        }
                        Ok(_) => None,
                        Err(error) => {
                            let error = CmdError::from(error);
                            if error.native_execution_refusal().is_some() {
                                return Err(error);
                            }
                            None
                        }
                    }
                };
                let Some((slot, value)) = mac_value else {
                    let mut error = br#"bad argument to "-types": "#.to_vec();
                    error.extend_from_slice(c_string_extent(item));
                    return Err(argument_error(version, error, b"TCL ARGUMENT BAD"));
                };
                if slot.replace(value).is_some() {
                    return Err(argument_error(
                        version,
                        br#"only one MacOS type or creator argument to "-types" allowed"#.to_vec(),
                        b"TCL ARGUMENT BAD",
                    ));
                }
            }
        }
        Ok(types)
    }

    /// Unix filesystem filters use a union of kinds and an intersection of
    /// permission tests. The link test independently admits a dangling link.
    #[must_use]
    pub fn matches(&self, fs: &dyn Filesystem, path: &str) -> bool {
        self.matches_entry(fs, path, true)
    }

    fn matches_entry(&self, fs: &dyn Filesystem, path: &str, check_hidden: bool) -> bool {
        if self.kinds.is_empty() && self.permissions.is_empty() && !self.hidden && !self.readonly {
            return true;
        }
        let name = path.rsplit('/').next().unwrap_or(path);
        if self.hidden && check_hidden && !name.starts_with('.') {
            return false;
        }
        let meta = fs.metadata(path).ok();
        if self.hidden && meta.is_none() {
            return false;
        }
        if self.readonly && meta.is_none_or(|m| m.mode & 0o222 != 0) {
            return false;
        }
        if !self
            .permissions
            .iter()
            .all(|&permission| fs.allows_access(path, permission))
        {
            return false;
        }
        self.kinds.is_empty()
            || self.kinds.iter().any(|&kind| {
                if kind == b'l' {
                    return fs.symlink_metadata(path).is_ok_and(|m| m.is_symlink);
                }
                meta.is_some_and(|m| match kind {
                    b'd' => m.is_dir,
                    b'f' => m.is_file,
                    b'p' => m.mode & 0o170_000 == 0o010_000,
                    b'c' => m.mode & 0o170_000 == 0o020_000,
                    b'b' => m.mode & 0o170_000 == 0o060_000,
                    b's' => m.mode & 0o170_000 == 0o140_000,
                    _ => false,
                })
            })
    }
}
