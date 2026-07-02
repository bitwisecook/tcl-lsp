//! `unicode` — Unicode normalization (Tcl 9.1).
//!
//! `unicode to<form> ?-profile PROFILE? STRING` converts a string to one of
//! the four Unicode normalization forms (NFC / NFD / NFKC / NFKD).  The
//! optional `-profile` selects the encoding profile used for invalid input.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "unicode subcommand ?-profile PROFILE? STRING",
}];

static PROFILE_OPTIONS: [OptionSpec; 1] = [OptionSpec {
    name: "-profile",
    takes_value: true,
    value_hint: "PROFILE",
    detail: "Encoding profile used for invalid input.",
    dialects: None,
}];

/// One `unicode to<form>` normalization subcommand: `?-profile PROFILE? STRING`
/// (1 positional + optional value-bearing flag ⇒ 1–3 words).
const fn normalise_sub(
    name: &'static str,
    synopsis: &'static str,
    detail: &'static str,
) -> SubCommand {
    SubCommand {
        name,
        arity: Arity::new(1, 3),
        detail,
        synopsis,
        options: &PROFILE_OPTIONS,
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    }
}

static SUBCOMMANDS: [SubCommand; 4] = [
    normalise_sub(
        "tonfc",
        "unicode tonfc ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form C (NFC).",
    ),
    normalise_sub(
        "tonfd",
        "unicode tonfd ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form D (NFD).",
    ),
    normalise_sub(
        "tonfkc",
        "unicode tonfkc ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form KC (NFKC).",
    ),
    normalise_sub(
        "tonfkd",
        "unicode tonfkd ?-profile PROFILE? STRING",
        "Return STRING converted to Unicode Normalization Form KD (NFKD).",
    ),
];

/// Command spec for `unicode` (Tcl 9.1).
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "unicode",
        dialects: Some(DialectSet::TCL91),
        arity: Arity::at_least(1),
        subcommands: &SUBCOMMANDS,
        return_type: Some(TclType::String),
        forms: FORMS,
        hover: Some(HoverSnippet::brief(
            "Unicode normalization (Tcl 9.1).",
            &["unicode subcommand ?-profile PROFILE? STRING"],
            "Tcl man page unicode.n",
        )),
        ..CommandSpec::DEFAULT
    }
}
