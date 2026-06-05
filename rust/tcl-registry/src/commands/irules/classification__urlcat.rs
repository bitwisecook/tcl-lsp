//! `CLASSIFICATION::urlcat` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::urlcat",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: provides classification url category name.",
            synopsis: &["CLASSIFICATION::urlcat"],
            snippet: "This command provides classification url category name.\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFICATION::urlcat",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__urlcat.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["CLASSIFICATION"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CLASSIFICATION::urlcat" },
        ],
        ..CommandSpec::DEFAULT
    }
}
