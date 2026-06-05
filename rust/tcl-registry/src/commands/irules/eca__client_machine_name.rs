//! `ECA::client_machine_name` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::client_machine_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns NTLM authenticating user's machine name.",
            synopsis: &["ECA::client_machine_name"],
            snippet: "The ECA::client_machine_name command returns NTLM returns authenticating user's machine name",
            source: "https://clouddocs.f5.com/api/irules/ECA__client_machine_name.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
