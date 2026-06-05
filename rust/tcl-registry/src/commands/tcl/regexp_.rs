//! `regexp` — match a regular expression against a string.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regexp ?switches? exp string ?matchVar? ?subMatchVar ...?",
}];

/// Command spec for `regexp`.
#[allow(clippy::too_many_lines)] // data-heavy: full hover + 11 options
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp",
        traits: Traits::BYTE_COMPILED | Traits::WARN_WITHOUT_TERMINATOR,
        arity: Arity::at_least(1),
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
                name: "-inline",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-indices",
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
            OptionSpec {
                name: "-about",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
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
            summary: "Match a regular expression against a string.",
            synopsis: &["regexp ?switches? exp string ?matchVar? ?subMatchVar ...?"],
            snippet: "Returns 1 if *exp* matches part of *string*, 0 otherwise. Matching substrings are stored in *matchVar* and *subMatchVar*.\n\n**Security**: Use `--` before the pattern when it comes from a variable to prevent option injection. Avoid nested quantifiers like `(a+)+` which can cause catastrophic backtracking (ReDoS) on crafted input.",
            source: "Tcl regexp(1)",
            examples: "",
            return_value: "1 if the pattern matches, 0 otherwise.",
        }),
        // GAP-D1: `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation. Mirrors `tcl/regexp_.py`.
        pattern_type: Some(PatternType::Regex),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
