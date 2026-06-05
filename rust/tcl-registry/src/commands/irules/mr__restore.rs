//! `MR::restore` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::restore",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the stored variables to the current context tcl variable store.",
            synopsis: &["MR::restore (VAR)*"],
            snippet: "The MR::restore command retrieves one or more named Tcl variables previously stored with the message by the MR::store command. If no name is provided, it retrieves all stored variables from the current message context.",
            source: "https://clouddocs.f5.com/api/irules/MR__restore.html",
            examples: "when MR_EGRESS {\n    MR::restore client_addr\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::restore (VAR)*" },
        ],
        ..CommandSpec::DEFAULT
    }
}
