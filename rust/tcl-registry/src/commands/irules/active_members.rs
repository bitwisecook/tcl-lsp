//! `active_members` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "active_members",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the number or list of active members in the specified pool.",
            synopsis: &["active_members ('-list')? POOL_OBJ"],
            snippet: "Returns the number or list of active members in the specified pool.",
            source: "https://clouddocs.f5.com/api/irules/active_members.html",
            examples: "when HTTP_REQUEST {\n    if { [active_members http_pool] >= 2 } {\n        pool http_pool\n    }\n}",
            return_value: "active_members <pool_name> Returns the number of active members in the specified pool.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &["LB_FAILED", "LB_SELECTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "active_members ('-list')? POOL_OBJ" },
        ],
        options: &[
            OptionSpec { name: "-list", takes_value: false, value_hint: "", detail: "Return as list instead of count.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
