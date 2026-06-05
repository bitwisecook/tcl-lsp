//! `MR::connection_instance` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::connection_instance",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the connection instance and the number of connections.",
            synopsis: &["MR::connection_instance"],
            snippet: "returns the connection instance number of the current connection and the number of\nconnections as configured in the peer object used to create the connection.\nThe return will be formated as \"<instance> of <num_connections>\".\nFor incoming connections, it will return \"0 of 1\".",
            source: "https://clouddocs.f5.com/api/irules/MR__connection_instance.html",
            examples: "when SERVER_CONNECTED {\n    log local0. \"[MR::connection_instance] [MR::connection_mode]\"\n}",
            return_value: "returns the connection instance number and the number of connections formatted as \"<instance> of <num_connections>\".",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::connection_instance" },
        ],
        ..CommandSpec::DEFAULT
    }
}
