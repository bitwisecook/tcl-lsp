//! `lrepeat` — build a list by repeating elements.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lrepeat count ?element ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lrepeat",
        const_fold: Some(crate::const_fold::fold_lrepeat),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE,
        dialects: None,
        arity: Arity::at_least(1),
        return_type: Some(TclType::List),
        inferred_storage_type: Some(StorageType::List),
        hover: Some(HoverSnippet {
            summary: "Build a list by repeating elements",
            synopsis: &["lrepeat count ?element ...?"],
            snippet: "The lrepeat command creates a list of size count * number of elements by repeating count times the sequence of elements element ....",
            source: "Tcl man page lrepeat.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_types: &[(
            0,
            ArgTypeHint {
                expected: Some(TclType::Int),
                shimmers: true,
            },
        )],
        ..CommandSpec::DEFAULT
    }
}
