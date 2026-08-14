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

//! `zipfs` — mount and manipulate ZIP archives on the virtual
//! filesystem (Tcl 9.0+, TIP 430).
//!
//! Absent from Tcl 8.4, 8.5, and 8.6: `TclCmd/zipfs.html` 404s on the
//! 8.4 and 8.5 doc trees, and on 8.6 both the `.html` and `.htm`
//! extensions resolve to the site's generic "URL Not Found" page (HTTP
//! 200, but the same not-found body as the 8.4/8.5 404s) — fetched and
//! read directly for all three rather than inferred from the 9.0+
//! manpage's absence of an explicit "Changed in" note. Introduced in
//! Tcl 9.0; the tcl-lang.org `zipfs.html` manual pages for Tcl 9.0.4 and
//! 9.1b0 are byte-for-byte identical apart from doc-anchor line-number
//! IDs. That identity was further cross-checked directly against
//! `generic/tclZipfs.c` on GitHub's `core-9-0-branch` (patch level 9.0.5
//! at fetch time — one patch ahead of the 9.0.4 manpage snapshot, with
//! no code difference relevant here) and `main` (9.1b1, one beta ahead
//! of the 9.1b0 manpage snapshot): every `ZipFS*ObjCmd`'s `objc` bounds
//! check is unchanged between the two branches, so every
//! `SubCommand::arity` below is one fact true for the whole
//! `TCL90_PLUS` gate, not two facts that happen to agree.
//!
//! The public command is the bare `zipfs`; the ensemble also answers to
//! its fully-qualified `::tcl::zipfs` name, so two forms are registered
//! (the same pattern as `tcl::process` — see `tcl_process.rs`).
//!
//! Hidden wholesale in a safe interpreter on both 9.0 and 9.1 — unlike
//! `tcl::process`, whose ensemble word always resolves and only its
//! subcommands are individually blocked. Tcl 9.0's
//! `unsafeEnsembleCommands[]` (`generic/tclBasic.c`) carries a
//! `{"zipfs", NULL}` row: a `NULL` `commandName` hides the whole
//! top-level command (`Tcl_HideCommand(interp, "zipfs", "zipfs")`), the
//! same treatment `file`/`encoding` get, and the source's own comment
//! explicitly analogises it to `file` ("[zipfs] perhaps has some safe
//! commands. But like file make it inaccessible until they are analyzed
//! to be safe") — plus thirteen more rows individually blocking every
//! subcommand except `find` (mirrored by `tclZipfs.c`'s own local
//! `initMap` inside `TclZipfs_Init`). Tcl 9.1's refactored equivalent
//! (`EnsembleSetup`/`EnsembleImplMap`, `tclBasic.c`/`tclZipfs.c`) keeps
//! the identical shape: the `"zipfs"` row in `ensembleCommands[]` carries
//! `flags: 0` (no `CMD_IS_SAFE`, so `TclHideUnsafeCommands` still
//! `Tcl_HideCommand`s the whole word), and `tclZipfsImplMap[]` marks
//! every subcommand `unsafe: 1` except `find` (`unsafe: 0`). `find` is a
//! pure-Tcl proc in both releases (`TclZipfs_Init`'s embedded
//! `findproc` script — a plain recursive `glob`, byte-identical on the
//! 9.0 and 9.1 sources) rather than a C command, so it was never in the
//! per-subcommand block list to start with; in practice it is no more
//! reachable than its siblings while the whole command stays hidden by
//! default, and the distinction only matters to a caller that
//! explicitly `interp expose`s `zipfs` into a still-safe interpreter.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "zipfs subcommand ?arg ...?",
    ..FormSpec::DEFAULT
}];

const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    // Command-level fallback only — every subcommand below declares its
    // own more precise `side_effects`, which `dialect_side_effect_hints`
    // (`tcl-compiler/src/side_effects.rs`) prefers whenever the
    // subcommand is known. `FileIo` covers every subcommand's actual
    // target: the archive files, the files inside them, and the mount
    // table, never a Tcl variable or generic interpreter state.
    target: SideEffectTarget::FileIo,
    reads: true,
    writes: true,
    ..SideEffect::DEFAULT
}];

/// `list`'s two mutually-exclusive matching-mode switches. Both are
/// unchanged between the Tcl 9.0 and 9.1 manpages and `ZipFSListObjCmd`'s
/// own `options[]` table, so neither carries its own narrower `dialects`
/// — they inherit the command's `TCL90_PLUS` gate.
static LIST_OPTIONS: [OptionSpec; 2] = [
    OptionSpec {
        name: "-glob",
        value: OptionValue::flag(),
        detail: "Match pattern as a glob pattern, per the rules of `string match` — the default when neither -glob nor -regexp is given.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
    OptionSpec {
        name: "-regexp",
        value: OptionValue::flag(),
        detail: "Match pattern as a regular expression instead of a glob pattern.",
        dialects: None,
        aliases: &[],
        lifecycle: Lifecycle::UNSPECIFIED,
        min_abbrev: None,
    },
];

fn make_spec(name: &'static str) -> CommandSpec {
    CommandSpec {
        name,
        dialects: Some(DialectSet::TCL90_PLUS),
        // `BYTE_COMPILED` follows this codebase's convention (see
        // `traits.rs`'s doc comment): "recognised core builtin", not
        // literal bytecode compilation. Unlike `tcl::process`'s four
        // subcommands — each of which carries a generic arg-count
        // `CompileProc` in `tclProcessImplMap` — none of zipfs's
        // fourteen do (every `compileProc` slot in `tclZipfsImplMap`/
        // `initMap` is `NULL`), but the trait still applies under the
        // documented convention.
        //
        // `SAFE_INTERP_HIDDEN`: see the module doc comment above for the
        // full `unsafeEnsembleCommands`/`EnsembleImplMap` evidence — the
        // whole `zipfs`/`::tcl::zipfs` command is `Tcl_HideCommand`-ed in
        // a safe interpreter on both Tcl 9.0 and 9.1, the same treatment
        // `file`/`encoding`/`cd`/`load`/`open` get.
        //
        // `TAINT_SINK`: the `zipfile`/`outfile`/`indir`/`inlist`/
        // `mountpoint` arguments to `mount`, `mountdata`, `mkzip`,
        // `mkimg`, `lmkzip`, `lmkimg`, and `unmount` unconditionally
        // become real filesystem paths that are read from or written
        // to — attacker-influenced input reaching any of them enables
        // arbitrary file read (a crafted `mount`/`mkzip` source) or file
        // overwrite (`mkzip`/`mkimg` silently overwrite an existing
        // `outfile`, confirmed in `ZipFSMkImgObjCmd`'s own doc comment),
        // the same path-traversal hazard `open`/`cd`/`load` carry.
        traits: Traits::BYTE_COMPILED | Traits::SAFE_INTERP_HIDDEN | Traits::TAINT_SINK,
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        hover: Some(HoverSnippet {
            summary: "Mount ZIP archives as a Tcl virtual filesystem, and build ZIP archives or self-contained executables.",
            synopsis: &[
                "zipfs canonical ?mountpoint? filename",
                "zipfs exists filename",
                "zipfs find directoryName",
                "zipfs info file",
                "zipfs list ?(-glob|-regexp)? ?pattern?",
                "zipfs lmkimg outfile inlist ?password? ?infile?",
                "zipfs lmkzip outfile inlist ?password?",
                "zipfs mkimg outfile indir ?strip? ?password? ?infile?",
                "zipfs mkkey password",
                "zipfs mkzip outfile indir ?strip? ?password?",
                "zipfs mount ?zipfile? ?mountpoint? ?password?",
                "zipfs mountdata data mountpoint",
                "zipfs root",
                "zipfs unmount mountpoint",
            ],
            snippet: "Mounts the contents of a ZIP archive file as a Tcl virtual filesystem, so files inside it can be read (and, in memory only, written) through the ordinary file/open/source commands, and provides commands to build such archives — including self-contained executables — from a Tcl script. Supported ZIP features are deliberately minimal: only the STORE and DEFLATE storage methods, with an optional simple obfuscating encryption strong enough to deter casual inspection but not a determined attacker; multi-part archives, zip64, and other compression methods such as bzip2 are unsupported. New files or directories cannot be created inside a mount, and writes to existing files stay in memory only — nothing is ever persisted back to the archive on disk. Paths inside a mount are case-sensitive on every platform, even a case-insensitive host filesystem.\n\ncanonical maps filename to where it would sit inside a zipfs mount, without requiring it to actually exist there; mountpoint selects which mount (default: the zipfs root). exists tests literal membership, and also reports true for a directory implied by a mounted file's path. find recursively lists paths under directoryName — an ordinary directory works too, it need not be inside a mount — in sorted order, and silently returns an empty list rather than erroring if directoryName can't be read; it also backs mkzip/mkimg's own directory walk. info returns {archive uncompressedSize compressedSize offset} for one file, or errors if the path isn't found in any mounted volume, including a path that is only an ancestor of a mount point rather than the mount point itself; querying the mount point itself gives the start of the ZIP data as offset, handy for truncating the ZIP off an executable. list returns every path across all mounts, or only those matching pattern — glob syntax by default or with -glob, a regular expression with -regexp — in arbitrary order, treating path separators as ordinary characters.\n\nmount has three forms: with no arguments it returns a dict mapping every mount point to its archive's path; with one argument (mountpoint) it returns the archive path mounted there; with zipfile and mountpoint (and an optional password) it mounts zipfile there and returns the normalized mount point — an empty mountpoint defaults to the zipfs root, and it is an error for mountpoint to name a drive or UNC volume. mountdata is the same three-way command but reads the archive from an in-memory data string instead of a file. Because the current working directory is an OS-level, per-process concept, cd-ing into a mount only works within the current process, and even then not entirely consistently. root returns the platform's zipfs mount-point root (commonly //zipfs:/, though callers should not assume the exact string is the same on every platform). unmount reverses mount, and errors if any file inside the archive is still open.\n\nmkzip builds outfile from the regular files under indir, stripping the strip prefix from each stored name when given — typically indir itself or its parent, since whatever remains becomes the archive's root name — and encrypting with password when given; mkimg does the same but prepends the result to infile (default: the running tclsh/wish executable) to build a self-contained image, silently overwriting outfile and discarding any ZIP chunk already attached to infile — if the resulting archive's root has a main.tcl and the base image is a tclsh/wish, the image will source it after mounting and exit once it returns. lmkzip/lmkimg are the same pair but take inlist, a flat list of alternating source-file/archive-name pairs, instead of a directory. mkkey returns an obfuscated form of password in the format mkimg embeds — not real encryption, and the empty string maps to itself.\n\nThe whole zipfs command is hidden by default inside a safe interpreter on both Tcl 9.0 and 9.1 (calling it raises \"invalid command name\"), unlike tcl::process's ensemble word, which always resolves; mkzip/mkimg/lmkzip/lmkimg additionally refuse with \"operation not permitted in a safe interpreter\" even if the command is explicitly re-exposed. Both the zipfs and ::tcl::zipfs spellings resolve to this command. The multi-optional-argument subcommand syntax is explicitly documented as provisional, and may move to a uniform ?-option value? style in a future release.",
            source: "Tcl zipfs(n)",
            examples: "set base [file join [zipfs root] myApp]\nzipfs mount myApp.zip $base\nsource [file join $base main.tcl]\nzipfs unmount $base\n\nset srcDir [file normalize myApp]\nzipfs mkzip myApp.zip $srcDir $srcDir\nzipfs mkimg myApp.bin $srcDir $srcDir [zipfs mkkey hunter2]",
            return_value: "Depends on the subcommand: exists returns a boolean; find and list return a list of paths; info returns a 4-element list {archive uncompressedSize compressedSize offset}; mount with no arguments returns a dict mapping each mount point to its archive path, and otherwise a path string; canonical, mountdata, mkzip, mkimg, lmkzip, lmkimg, mkkey, root, and unmount all return a string (the empty string for unmount, an obfuscated password for mkkey, a path for the rest).",
        }),
        ..CommandSpec::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 14] = [
    SubCommand {
        name: "canonical",
        arity: Arity::new(1, 2),
        detail: "Compute where filename would be mapped inside a zipfs mount, without checking whether it actually exists there. mountpoint selects which mount to resolve against; the zipfs root is used when omitted.",
        synopsis: "zipfs canonical ?mountpoint? filename",
        return_type: Some(TclType::String),
        // Pure string computation — `ZipFSCanonicalObjCmd`'s own doc
        // comment says "Side effects: None", and unlike exists/info/list
        // it never takes the zipfs read lock or consults the live mount
        // table at all.
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::new(1, 1),
        detail: "Test whether filename exists in a mounted archive; also true for a directory that is only implied by a mounted file's path, not just a literal archive entry.",
        synopsis: "zipfs exists filename",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "find",
        arity: Arity::new(1, 1),
        detail: "Recursively list, in sorted order, every path under directoryName (which need not be inside a mounted archive — any real directory works). Also used internally by mkzip/mkimg to walk their input directory. Silently returns an empty list rather than erroring if directoryName can't be read.",
        synopsis: "zipfs find directoryName",
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "info",
        arity: Arity::new(1, 1),
        detail: "Return {archive uncompressedSize compressedSize offset} for a file in a mounted archive. Querying the mount point itself gives the start of the ZIP data as offset. Errors (\"not found in any zipfs volume\") for a path that doesn't exist, including a path that is only an ancestor of a mount point rather than the mount point itself.",
        synopsis: "zipfs info file",
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "list",
        arity: Arity::new(0, 2),
        detail: "List every path across all mounts, or only those matching pattern: a glob pattern (string match rules) by default or with -glob, or a regular expression with -regexp. Returned in arbitrary order; path separators are matched as ordinary characters, not specially.",
        synopsis: "zipfs list ?(-glob|-regexp)? ?pattern?",
        return_type: Some(TclType::List),
        options: &LIST_OPTIONS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lmkimg",
        arity: Arity::new(2, 4),
        detail: "Like mkimg, but inlist is a flat Tcl list of alternating source-file/archive-name pairs instead of a directory: odd elements are files to copy in, even elements are their names inside the archive.",
        synopsis: "zipfs lmkimg outfile inlist ?password? ?infile?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lmkzip",
        arity: Arity::new(2, 3),
        detail: "Like mkzip, but inlist is a flat Tcl list of alternating source-file/archive-name pairs instead of a directory: odd elements are files to copy in, even elements are their names inside the archive.",
        synopsis: "zipfs lmkzip outfile inlist ?password?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mkimg",
        arity: Arity::new(2, 5),
        detail: "Like mkzip, but prepends the result to infile (default: the running tclsh/wish executable) to build a self-contained image, silently overwriting outfile and discarding any ZIP already attached to infile. If the archive's root has a main.tcl and the base image is a tclsh/wish, the built image sources it after mounting and exits once it returns.",
        synopsis: "zipfs mkimg outfile indir ?strip? ?password? ?infile?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mkkey",
        arity: Arity::new(1, 1),
        detail: "Return an obfuscated form of password in the format mkimg embeds between its image and ZIP chunks — not real encryption, only enough to deter casual inspection. An empty password returns the empty string.",
        synopsis: "zipfs mkkey password",
        return_type: Some(TclType::String),
        // Pure in-memory byte transform — no filesystem access.
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mkzip",
        arity: Arity::new(2, 4),
        detail: "Create outfile from the regular files under indir, stripping the strip prefix from each stored name if given (typically indir itself or its parent, since whatever remains becomes the archive's root name) and encrypting with password if given.",
        synopsis: "zipfs mkzip outfile indir ?strip? ?password?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mount",
        arity: Arity::new(0, 3),
        // `return_type` stays `String` rather than `Dict`: only the
        // zero-argument form returns a dict (mount point -> archive
        // path); the one-argument (query) and two/three-argument (mount)
        // forms both return a plain path string, the more common calls
        // by far. `String` is the conservative choice that stays
        // correct for every form; `Dict` would be actively wrong for two
        // of the three. See the detail text below for the full
        // per-form breakdown.
        detail: "Three forms: no arguments returns a dict mapping every mount point to its archive's path; mountpoint alone returns the archive path mounted there; zipfile and mountpoint (with optional password) mounts zipfile there and returns the normalized mount point — an empty mountpoint defaults to the zipfs root. Errors if mountpoint names a drive or UNC volume.",
        synopsis: "zipfs mount ?zipfile? ?mountpoint? ?password?",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            // Every form reads (the query forms read the mount table;
            // the mounting form also reads zipfile from disk to
            // validate it); only the mounting form writes.
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mountdata",
        arity: Arity::new(2, 2),
        detail: "Mount the in-memory ZIP archive content data as a virtual filesystem at mountpoint — like mount's third form, but reading from a byte string instead of a file.",
        synopsis: "zipfs mountdata data mountpoint",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "root",
        arity: Arity::new(0, 0),
        detail: "Return the platform's zipfs mount-point root (commonly //zipfs:/); the exact string is not guaranteed to be the same across platforms.",
        synopsis: "zipfs root",
        return_type: Some(TclType::String),
        // Returns a fixed platform constant — no filesystem access.
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "unmount",
        arity: Arity::new(1, 1),
        detail: "Dismount a previously mounted ZIP archive. Errors if any file inside it is still open.",
        synopsis: "zipfs unmount mountpoint",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for the public global form `zipfs`.
pub fn spec() -> CommandSpec {
    make_spec("zipfs")
}

/// Command spec for the fully-qualified ensemble form `::tcl::zipfs`.
pub fn spec_qualified() -> CommandSpec {
    make_spec("::tcl::zipfs")
}
