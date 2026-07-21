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

//! `zlib` — data compression / decompression primitives (Tcl 8.6+).
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "zlib subcommand ?args ...?",
    dialects: None,
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "zlib",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Compression / decompression using zlib.",
            synopsis: &[
                "zlib compress data ?level?",
                "zlib decompress data ?bufferSize?",
                "zlib deflate data ?level?",
                "zlib inflate data ?bufferSize?",
                "zlib gzip data ?-level level? ?-header header?",
                "zlib gunzip data ?-buffersize n? ?-headerVar varname?",
                "zlib crc32 data ?initValue?",
                "zlib adler32 data ?initValue?",
                "zlib stream mode ?level?",
                "zlib push mode channel ?options?",
            ],
            snippet: "Compress / decompress data, compute CRC32 / Adler-32 checksums, or attach a compression filter to a channel.  Not yet implemented in the WASM runtime — traps with ``unsupported command: zlib``.",
            source: "Tcl man page zlib.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
