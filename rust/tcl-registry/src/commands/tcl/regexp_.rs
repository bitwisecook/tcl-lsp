//! `regexp` — match a regular expression against a string.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regexp ?switches? exp string ?matchVar? ?subMatchVar ...?",
}];

/// `regexp ?switches? exp string ?matchVar ...?` — after skipping leading
/// options (`-start` consumes a value; `--` terminates), arg 0 is the
/// pattern, arg 1 the string, and args 2+ are capture variables.  Resolve
/// `VarWrite` for every trailing capture var dynamically (the leading-option
/// shift means a static slot list cannot place them).
fn regexp_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let mut i = 0;
    while i < args.len() {
        let a = args[i];
        if a == "--" {
            i += 1;
            break;
        }
        if a.starts_with('-') {
            i += 1;
            if a == "-start" && i < args.len() {
                i += 1;
            }
            continue;
        }
        break;
    }
    let capture_start = i + 2; // skip pattern + string
    (capture_start..args.len())
        .filter_map(|j| u8::try_from(j).ok().map(|j| (j, ArgRole::VarWrite)))
        .collect()
}

/// Command spec for `regexp`.
#[allow(clippy::too_many_lines)] // data-heavy: full hover + 11 options
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regexp",
        traits: Traits::BYTE_COMPILED
            | Traits::WARN_WITHOUT_TERMINATOR
            | Traits::FRAME_HASH_BUILTIN,
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
        // `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation.
        pattern_type: Some(PatternType::Regex),
        forms: FORMS,
        arg_role_resolver: Some(regexp_arg_roles),
        ..CommandSpec::DEFAULT
    }
}
