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

//! `filename` — file name conventions supported by Tcl commands.

use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

/// `filename` is `filename(n)`: a conventions reference, not an invocable
/// command — there is no `filename ...` call to arity-check, so `traits`,
/// `forms`, `options`, and `arg_values` stay empty and `hover` carries the
/// entire fact set as prose. `xtask::command_backing::NOT_REQUIRED` lists
/// `"filename"` explicitly (there is no runtime command to back). Its surface
/// is `ALL_TCL`, which does not carry an iRules row, so it is simply absent
/// under iRules — excluded from iRules exactly as `cd`/`open` are, with no
/// disable list involved.
///
/// Fetched and compared word-for-word across all five manpages
/// (tcl8.4/8.5/8.6/9.0/9.1 `TclCmd/filename.html`, 8.6 via the `.htm`
/// swap). The INTRODUCTION, PATH TYPES, and Windows PATH SYNTAX sections
/// are byte-identical on every version. Real deltas found:
/// - Unix PATH SYNTAX: the "any number of trailing slashes are ignored"
///   sentence is absent from 8.4 and present from 8.5 on; the "except for
///   the first double slash `//` in absolute paths" clause on slash
///   collapsing is absent from 8.4/8.5 and present from 8.6 on.
/// - `zipfs` is added to SEE ALSO from 8.6 on (absent 8.4/8.5), but the
///   dedicated "Zipfs" PATH SYNTAX subsection with its own explanatory
///   paragraph only appears from 9.0 on (absent 8.4/8.5/8.6).
/// - TILDE SUBSTITUTION is a hard break at the 9.0 boundary: 8.4/8.5/8.6
///   describe unconditional implicit csh-style expansion of a leading `~`
///   in *every* file name argument; 9.0/9.1 replace that section entirely
///   — implicit expansion is gone except when initialising `auto_path`/the
///   Tcl module path from an environment variable, and `file home` /
///   `file tildeexpand` are introduced as the explicit replacement. (`cd`'s
///   own audited spec independently documents this same 8.4-8.6 vs. 9.0+
///   boundary for its `dirName` argument.)
/// - PORTABILITY ISSUES: the forbidden-character list gains `?` from 8.5
///   on (8.4 lists `<>:"/\|` without it); the "care with spaces / backslash
///   as separator" sentence is absent from 8.4 and present from 8.5 on; the
///   Windows 3.1 8.3-filename note is present in 8.4/8.5 and dropped from
///   8.6 on (long-dead platform, omitted from the hover text below).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "filename",
        surface: Some(SpecSurface::ALL_TCL),
        hover: Some(HoverSnippet {
            summary: "File name conventions supported by Tcl commands and C procedures.",
            synopsis: &[],
            snippet: "Not an invocable command — a conventions reference for how every Tcl command and C API that accepts a file name interprets it. Every name is absolute (fully qualified from a volume's root), relative (qualified from the current working directory), or volume-relative (qualified from the current volume's root, or from a named volume's own current directory); `file pathtype` reports which. Each platform accepts its own native form plus a Unix-like syntax; portable scripts should build and take paths apart with `file join` and `file split` rather than assume a particular form.\n\nOn Unix and (from Tcl 8.5) Apple macOS, components are slash-separated and `.`/`..` name the current and parent directory. Adjacent slashes collapse to one — except that a leading `//` in an absolute path is kept as-is (documented from 8.6 on) — and any number of trailing slashes are ignored (documented from 8.5 on), so `foo`, `foo/`, and `foo//` are equivalent and a trailing slash alone does not imply a directory.\n\nOn Windows, both `/` and `\\` work as separators. Drive-relative names take an optional drive letter (`c:foo` is relative to the current directory on drive C; `c:/foo` is absolute at drive C's root); UNC names follow `\\\\server\\share\\path\\file` and must contain at least the server and share (`\\\\server\\share`); `\\foo` and `\\\\foo` are both volume-relative to the current drive's root, since a bare `\\\\foo` isn't a well-formed UNC path. Repeated `file dirname` on a UNC path bottoms out at `//server/share`, never at just `//server`.\n\nOn builds with zipfs support, paths inside a mounted ZIP archive begin with the string `zipfs root` returns and stay case-sensitive even on a case-insensitive host filesystem; this became its own documented PATH SYNTAX section in Tcl 9.0, though the `zipfs` package itself has existed since 8.6.\n\nTilde substitution changed fundamentally at the Tcl 9.0 boundary. Through 8.4, 8.5, and 8.6, a leading `~` in any file name argument to any command was implicitly csh-style expanded: `~/...` substituted `$HOME`, and `~user/...` substituted that user's home directory (older Windows releases raised a \"user does not exist\" error for the `~user` form instead of resolving it). Tcl 9.0 and 9.1 remove that implicit expansion everywhere except when initialising `auto_path` and the Tcl module search path from an environment variable (see `tclvars`/`tm`); everywhere else a leading `~` is now taken literally, and code that wants expansion must call the new `file home` or `file tildeexpand` commands explicitly.\n\nNot every filesystem is case-sensitive, so scripts should avoid depending on name case. Avoid the characters `<>:?\"/\\|` in a file name (`?` joined this documented list in 8.5); the safest names use alphanumerics only. Take care with names that contain spaces (common on Windows) and remember backslash doubles as the Windows path separator — both cautions documented from 8.5 on. Windows paths are limited to roughly 260 characters; Windows silently drops trailing dots from a name at creation time (visible in `file normalize`'s result) and rejects a name made up only of dots.",
            source: "Tcl filename(n)",
            examples: "file pathtype ~/report.txt\nfile pathtype c:/foo\nfile join {*}[file split $path]\n\nset f [open ~/.bashrc]                        ;# Tcl 8.4-8.6: ~ expands automatically\nset f [open [file tildeexpand ~/.bashrc]]      ;# Tcl 9.0+: ~ is no longer expanded implicitly",
            return_value: "Not applicable — filename(n) is a documentation topic, not a callable command.",
        }),
        ..CommandSpec::DEFAULT
    }
}
