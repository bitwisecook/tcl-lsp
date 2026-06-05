//! `FLOW::priority` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOW::priority",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Set/Get flow's internal packet priority.",
            synopsis: &["FLOW::priority FLOW_PRIORITY", "FLOW::priority (clientside | serverside) (FLOW_PRIORITY)?", "FLOW::priority (ANY_CHARS) (FLOW_PRIORITY)?"],
            snippet: "This command is used to get/set the flow's internal packet priority.\nValid priority is any integer value from 0 to 7.\nSyntax:\nFLOW::priority [TCL handle|clientside|serverside] [priority]\n\nFollowing are the variations of this command:\n\nFLOW::priority\n\n Returns the internal packet priority of current flow.\n\nFlow::priority <priority>\n\n Sets the priority of the current flow's internal packet priority.\n Exception is thrown if priority is outside the allowed range [0-7].\n\nFLOW::priority clientside\n\n Returns the priority of the clientside flow's internal packet priority.",
            source: "https://clouddocs.f5.com/api/irules/FLOW__priority.html",
            examples: "when SERVER_CONNECTED {\n  FLOW::priority serverside 4\n\n  # Alternate way to use the command using the TCL flow handle.\n  # Set priority on both client side and server side flow.\n  FLOW::priority $clientflow 2\n  FLOW::priority [FLOW::this] 2\n}",
            return_value: "Get operation returns an integer between 0-7. Set operation returns nothing. Exception is thrown if the operation cannot be completed.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FLOW"],
            also_in: &["CLIENT_ACCEPTED", "SERVER_CONNECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
