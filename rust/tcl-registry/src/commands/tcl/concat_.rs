//! `concat` — concatenate lists.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "concat ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "concat",
        const_fold: Some(crate::const_fold::fold_concat),
        traits: Traits::FRAMELESS_RUNTIME | Traits::PURE | Traits::PRODUCES_CANONICAL_LIST,
        arity: Arity::any(),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Join lists together",
            synopsis: &["concat ?arg arg ...?", "concat ?arg ...?"],
            snippet: "This command joins each of its arguments together with spaces after trimming leading and trailing white-space from each of them.",
            source: "Tcl man page concat.n",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
