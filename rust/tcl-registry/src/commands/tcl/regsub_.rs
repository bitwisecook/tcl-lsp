//! `regsub` — perform substitutions based on regular expression matching.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "regsub ?switches? exp string subSpec ?varName?",
}];

/// `regsub ?switches? exp string subSpec ?varName?` — after skipping leading
/// options (`-start` consumes a value; `--` terminates), the positional args
/// are `exp` (0), `string` (1), `subSpec` (2), and the optional `varName` (3).
/// When `varName` is present it names the variable the result is written to;
/// resolve it as `VarWrite` dynamically (the leading-option shift means a
/// static slot cannot place it).  Omitting `varName` (Tcl 8.7+/9 returns the
/// substituted string instead) simply yields no `VarWrite` index.
fn regsub_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
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
    // exp (i), string (i+1), subSpec (i+2), varName (i+3).
    let var_idx = i + 3;
    (var_idx < args.len())
        .then(|| u8::try_from(var_idx).ok().map(|v| (v, ArgRole::VarWrite)))
        .flatten()
        .into_iter()
        .collect()
}

/// Command spec for `regsub`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "regsub",
        traits: Traits::BYTE_COMPILED | Traits::FRAME_HASH_BUILTIN,
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
                dialects: Some(DialectSet::TCL90_PLUS),
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
        // `exp` is an ARE pattern — drives regex sub-tokens and
        // pattern validation.
        pattern_type: Some(PatternType::Regex),
        arg_role_resolver: Some(regsub_arg_roles),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
