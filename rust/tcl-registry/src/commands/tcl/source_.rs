//! `source` — evaluate a file or resource as a Tcl script.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "source ?-encoding name? fileName",
}];

/// Command spec for `source`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "source",
        traits: Traits::NOT_PROC_FACTORY
            | Traits::BYTE_COMPILED
            | Traits::LANGUAGE_KEYWORD
            | Traits::SOURCES_FILE,
        arity: Arity::new(1, 2),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        options: &[OptionSpec {
            name: "-encoding",
            takes_value: true,
            value_hint: "encoding",
            detail: "",
            dialects: None,
        }],
        hover: Some(HoverSnippet::brief(
            "Evaluate a file or resource as a Tcl script.",
            &["source ?-encoding name? fileName"],
            "Tcl source(1)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
