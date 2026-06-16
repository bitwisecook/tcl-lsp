//! `mime::finalize` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "mime::finalize token ?-subordinates all?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "mime::finalize",
        dialects: None,
        arity: Arity::new(1, 3),
        hover: Some(HoverSnippet {
            summary: "Destroy a MIME part and free resources.",
            synopsis: &["mime::finalize token ?-subordinates all?"],
            snippet: "",
            source: "tcllib mime package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("mime"),
        required_package: Some("mime"),
        ..CommandSpec::DEFAULT
    }
}
