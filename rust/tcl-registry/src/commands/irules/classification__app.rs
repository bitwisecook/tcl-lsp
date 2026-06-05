//! `CLASSIFICATION::app` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CLASSIFICATION::app",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Deprecated: Provides classification for the most explicit application name.",
            synopsis: &["CLASSIFICATION::app"],
            snippet: "This command provides classification for the most explicit application\nname. (Example: cnn, amazon)\n\n* Note: APM / AFM / PEM license is required for functionality to work.\n\nCLASSIFICATION::app",
            source: "https://clouddocs.f5.com/api/irules/CLASSIFICATION__app.html",
            examples: "when CLASSIFICATION_DETECTED {\n  if { [CLASSIFICATION::app] equals \"application1\"}  {\n    drop\n  }\n}",
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
        ..CommandSpec::DEFAULT
    }
}
