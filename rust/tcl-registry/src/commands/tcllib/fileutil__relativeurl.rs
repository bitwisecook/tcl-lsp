//! `fileutil::relativeUrl` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "fileutil::relativeUrl base dst",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fileutil::relativeUrl",
        traits: Traits::PURE,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Compute a relative URL path.",
            synopsis: &["fileutil::relativeUrl base dst"],
            snippet: "",
            source: "tcllib fileutil package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
