//! `regsub` — perform substitutions based on regular expression matching.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regsub ?switches? exp string subSpec ?varName?",
}];

/// Command spec for `regsub`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regsub",
        traits: Traits::BYTE_COMPILED
            | Traits::FRAME_HASH_BUILTIN,
        arity: Arity::new(3, 4),
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::Variable,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: &[
            OptionSpec {
                name: "-nocase",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-expanded",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-line",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-linestop",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-lineanchor",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-all",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-start",
                takes_value: true,
                value_hint: "index",
                detail: "",
                dialects: None,
            },
            // `regsub -command` is Tcl 9.0+ (TIP 463).
            OptionSpec {
                name: "-command",
                takes_value: false,
                value_hint: "",
                detail: "Treat subSpec as a command prefix to call per match.",
                dialects: Some(DialectSet::TCL90),
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
hover: Some(HoverSnippet {
    summary: "Perform substitutions based on regular expression matching.",
    synopsis: &["regsub ?switches? exp string subSpec ?varName?"],
    snippet: "Matches *exp* against *string* and replaces the matched portion with *subSpec*. With `-all`, replaces all occurrences.\n\n**Security**: Use `--` before the pattern when it comes from a variable to prevent option injection. The *subSpec* supports `\\0`..`\\9` backreferences and `&` for the full match.",
    source: "Tcl regsub(1)",
    examples: "",
    return_value: "The substituted string (Tcl 8.5+), or the count of replacements when *varName* is given.",
}),
        // GAP-D1: `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation. Mirrors `tcl/regsub_.py`.
        pattern_type: Some(PatternType::Regex),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
