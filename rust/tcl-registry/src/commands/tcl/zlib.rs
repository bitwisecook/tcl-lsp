//! `zlib` — data compression / decompression primitives (Tcl 8.6+).
use crate::prelude::*;

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "zlib",
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet::brief(
            "Compression / decompression using zlib.",
            &[
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
            "Tcl man page zlib.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
