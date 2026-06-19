//! `zlib` — data compression / decompression primitives (Tcl 8.6+).
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "zlib subcommand ?args ...?",
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
