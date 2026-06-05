//! `FLOWTABLE::count` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FLOWTABLE::count",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns flow counts.",
            synopsis: &["FLOWTABLE::count (virtual (VIRTUAL_SERVER_OBJ)?)?", "FLOWTABLE::count route_domain (ROUTE_DOMAIN_NAME)?"],
            snippet: "This iRules command returns flow counts\nNote: When virtual server or route domain name is omitted the commands\nuse virtual or route domain of the current connection. Specifying the\nname incurs significant performance hit.",
            source: "https://clouddocs.f5.com/api/irules/FLOWTABLE__count.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "FLOWTABLE::count (virtual (VIRTUAL_SERVER_OBJ)?)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FlowState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
