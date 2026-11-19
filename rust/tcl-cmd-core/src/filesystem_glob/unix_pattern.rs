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

//! The section of the Unix filesystem pattern reached by C Tcl's `DoGlob`.
use std::ops::Range;

use tcl_core_types::c_string_extent;

/// Original byte offsets in the current pattern. A brace group is expanded
/// before the filesystem processes even the preceding literal in this section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CGlobHead {
    Component {
        text: Range<usize>,
        /// Includes the quoting backslash for a quoted slash.
        separator: Option<Range<usize>>,
        /// The native matcher is required for `*`, `[`, `]`, `?` or backslash.
        matcher: bool,
    },
    Alternatives {
        braces: Range<usize>,
        /// One range per alternative, including each empty alternative.
        choices: Vec<Range<usize>>,
    },
}

/// The unmatched delimiter in the section actually reached.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GlobBraceError {
    pub offset: usize,
    pub opening: bool,
}

/// Scan through the first Unix directory separator or the first outer brace
/// group. Quoted separators are boundaries too. Inside a brace alternative,
/// a slash belongs to the alternative and nested braces mask commas.
pub fn c_glob_head(pattern: &[u8]) -> Result<CGlobHead, GlobBraceError> {
    let pattern = c_string_extent(pattern);
    let mut i = 0;
    let mut matcher = false;
    while i < pattern.len() {
        match pattern[i] {
            b'/' => {
                return Ok(CGlobHead::Component {
                    text: 0..i,
                    separator: Some(i..i + 1),
                    matcher,
                });
            }
            b'\\' if pattern.get(i + 1) == Some(&b'/') => {
                return Ok(CGlobHead::Component {
                    text: 0..i,
                    separator: Some(i..i + 2),
                    matcher,
                });
            }
            b'\\' => {
                matcher = true;
                i += 1;
            }
            b'*' | b'?' | b'[' | b']' => matcher = true,
            b'}' => {
                return Err(GlobBraceError {
                    offset: i,
                    opening: false,
                });
            }
            b'{' => return brace_alternatives(pattern, i),
            _ => {}
        }
        i += 1;
    }
    Ok(CGlobHead::Component {
        text: 0..pattern.len(),
        separator: None,
        matcher,
    })
}

fn brace_alternatives(pattern: &[u8], open: usize) -> Result<CGlobHead, GlobBraceError> {
    let mut i = open + 1;
    let mut start = i;
    let mut nested = 0;
    let mut choices = Vec::new();
    while i < pattern.len() {
        match pattern[i] {
            b'\\' => i += 1,
            b'{' => nested += 1,
            b'}' if nested == 0 => {
                choices.push(start..i);
                return Ok(CGlobHead::Alternatives {
                    braces: open..i + 1,
                    choices,
                });
            }
            b'}' => nested -= 1,
            b',' if nested == 0 => {
                choices.push(start..i);
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }
    Err(GlobBraceError {
        offset: open,
        opening: true,
    })
}

use super::{GlobOptions, argument_error};
use crate::CmdError;
use tcl_dialect::TclVersion;
use tcl_platform::Filesystem;

/// Prefix from the option's original value: C `-path` makes its last
/// component literal even if it contains glob syntax.
pub(super) fn c_matches(
    fs: &dyn Filesystem,
    options: &GlobOptions,
    directory: Option<&[u8]>,
    pattern: &[u8],
    version: TclVersion,
) -> Result<Vec<Vec<u8>>, CmdError> {
    let mut pending = vec![starting_point(options, directory, pattern)];
    let mut hits = Vec::new();
    while let Some((path, shown, mut rest)) = pending.pop() {
        // DoGlob consumes all leading (including quoted) Unix separators
        // before looking for braces and metacharacters in this component.
        rest.drain(..leading_separators(&rest));
        let head = match c_glob_head(&rest) {
            Ok(head) => head,
            Err(_) if version == TclVersion::V8_4 && options.no_complain => {
                continue;
            }
            Err(error) => {
                let message: &[u8] = if error.opening {
                    b"unmatched open-brace in file name"
                } else {
                    b"unmatched close-brace in file name"
                };
                return Err(argument_error(
                    version,
                    message.to_vec(),
                    b"TCL OPERATION GLOB BALANCE",
                ));
            }
        };
        let (text, separator, matcher) = match head {
            CGlobHead::Alternatives { braces, choices } => {
                for choice in choices.into_iter().rev() {
                    let expanded =
                        [&rest[..braces.start], &rest[choice], &rest[braces.end..]].concat();
                    pending.push((path.clone(), shown.clone(), expanded));
                }
                continue;
            }
            CGlobHead::Component {
                text,
                separator,
                matcher,
            } => (text, separator, matcher),
        };
        let component = &rest[text];
        let remaining = separator.as_ref().map(|separator| &rest[separator.end..]);
        let probe = |path: &[u8]| -> Result<bool, CmdError> {
            let path = host_path(path)?;
            Ok(fs.symlink_metadata(path).is_ok()
                && options
                    .types
                    .matches_entry(fs, path, version != TclVersion::V8_4))
        };
        if !matcher {
            let next = join(&path, component);
            let result = join(&shown, component);
            if let Some(remaining) = remaining {
                pending.push(next_component(next, result, remaining, version));
            } else if probe(&next)? {
                let result = if result.is_empty() {
                    b".".to_vec()
                } else {
                    result
                };
                hits.push(result);
            }
            continue;
        }
        let need_hidden = component.starts_with(b".")
            || component.starts_with(b"\\.")
            || (remaining.is_none() && options.types.hidden);
        let Ok(mut names) = fs.read_dir(host_path(&path)?) else {
            continue;
        };
        if need_hidden {
            names.push(".".to_owned());
            names.push("..".to_owned());
        }
        let mut subdirs = Vec::new();
        for name in names {
            let name = name.as_bytes();
            if name.starts_with(b".") != need_hidden
                || !tcl_syntax::native_glob::match_c_string_glob(version, component, name, false)
            {
                continue;
            }
            let next = join(&path, name);
            let result = join(&shown, name);
            if let Some(remaining) = remaining {
                if fs.metadata(host_path(&next)?).is_ok_and(|meta| meta.is_dir) {
                    subdirs.push(next_component(next, result, remaining, version));
                }
            } else if options.types.matches(fs, host_path(&next)?) {
                hits.push(result);
            }
        }
        pending.extend(subdirs.into_iter().rev());
    }
    Ok(hits)
}

fn starting_point(
    options: &GlobOptions,
    directory: Option<&[u8]>,
    pattern: &[u8],
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let pattern = c_string_extent(pattern);
    let mut pattern = pattern.to_vec();
    let mut base = directory.map_or(&[][..], c_string_extent);
    if matches!(options.base, Some(super::GlobBase::Path(_))) {
        let split = base.iter().rposition(|b| *b == b'/').map_or(0, |i| i + 1);
        let mut prefixed = Vec::new();
        for &byte in &base[split..] {
            if b"\\{}[]*?".contains(&byte) {
                prefixed.push(b'\\');
            }
            prefixed.push(byte);
        }
        prefixed.append(&mut pattern);
        pattern = prefixed;
        base = &base[..split];
    }
    let absolute = directory.is_none() && pattern.first() == Some(&b'/');
    let base = if absolute { b"/" } else { base };
    let shown = if options.tails {
        Vec::new()
    } else {
        base.to_vec()
    };
    (base.to_vec(), shown, pattern)
}

fn next_component(
    path: Vec<u8>,
    mut shown: Vec<u8>,
    rest: &[u8],
    version: TclVersion,
) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let leading = leading_separators(rest);
    shown.push(b'/');
    if version == TclVersion::V8_4 {
        shown.extend(rest[..leading].iter().copied().filter(|byte| *byte == b'/'));
    }
    (path, shown, rest[leading..].to_vec())
}

fn leading_separators(rest: &[u8]) -> usize {
    let mut begin = 0;
    while begin < rest.len() {
        if rest[begin] == b'/' {
            begin += 1;
        } else if rest[begin..].starts_with(b"\\/") {
            begin += 2;
        } else {
            break;
        }
    }
    begin
}

fn host_path(path: &[u8]) -> Result<&str, CmdError> {
    if path.is_empty() {
        return Ok(".");
    }
    std::str::from_utf8(path).map_err(|_| {
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("host filesystem pathname bytes")
            .into()
    })
}

fn join(path: &[u8], name: &[u8]) -> Vec<u8> {
    if name.is_empty() {
        return path.to_vec();
    }
    let mut result = path.to_vec();
    if !result.is_empty() && result.last() != Some(&b'/') {
        result.push(b'/');
    }
    result.extend_from_slice(name);
    result
}
