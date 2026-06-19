//! `glob` — return names of files that match patterns.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "glob ?switches? ?--? pattern ?pattern ...?",
}];

/// Command spec for `glob`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "glob",
        dialects: Some(DialectSet::NON_IRULES_OPERATORS),
        traits: Traits::BYTE_COMPILED,
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
        }],
        options: &[
            OptionSpec {
                name: "-directory",
                takes_value: true,
                value_hint: "dir",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-join",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-nocomplain",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-path",
                takes_value: true,
                value_hint: "pathPrefix",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-tails",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
            OptionSpec {
                name: "-types",
                takes_value: true,
                value_hint: "typeList",
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
            summary: "Return names of files that match patterns.",
            synopsis: &["glob ?switches? ?--? pattern ?pattern ...?"],
            snippet: "Performs file name globbing similar to `csh`. Returns a list of matching file names.\n\nUse `-nocomplain` to return an empty list instead of an error when no files match. Use `--` before patterns that may start with `-`.",
            source: "Tcl glob(1)",
            examples: "",
            return_value: "A list of file names matching the patterns.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
