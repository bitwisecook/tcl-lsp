//! `HSL::open` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HSL::open",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        return_type: Some(TclType::Channel),
hover: Some(HoverSnippet {
            summary: "Opens a handle for High Speed Logging communication.",
            synopsis: &["HSL::open ('-publisher' | '-pub') PUBLISHER", "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ"],
            snippet: "Open a handle for High Speed Logging communication. After creating the\nconnection, send data on the connection using HSL::send.",
            source: "https://clouddocs.f5.com/api/irules/HSL__open.html",
            examples: "#2\nwhen CLIENT_ACCEPTED {\n    set hsl [HSL::open -publisher /Common/lpAll]\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
