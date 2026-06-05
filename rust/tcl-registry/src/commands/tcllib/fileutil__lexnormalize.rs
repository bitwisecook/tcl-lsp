//! `fileutil::lexnormalize` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::lexnormalize path",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::lexnormalize",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(1),
        hover: Some(HoverSnippet {
            summary: "Lexically normalise a path.",
            synopsis: &["fileutil::lexnormalize path"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
