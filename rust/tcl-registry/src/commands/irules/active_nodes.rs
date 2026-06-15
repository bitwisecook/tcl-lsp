//! `active_nodes` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "active_nodes",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the alias for active members of the specified pool (for BIG-IP version 4.X compatibility).",
            synopsis: &["active_nodes ('-list')? POOL_OBJ"],
            snippet: "Returns the alias for active members of the specified pool (for BIG-IP version 4.X compatibility).",
            source: "https://clouddocs.f5.com/api/irules/active_nodes.html",
            examples: "when HTTP_REQUEST {\n    log local0. \"There are [active_nodes http_pool] active nodes in the pool.\"\n}",
            return_value: "active_nodes <pool name> Returns the number of active members of the specified pool (for BIG-IP version 4.X compatibility).",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "active_nodes ('-list')? POOL_OBJ",
        }],
        options: &[OptionSpec {
            name: "-list",
            takes_value: false,
            value_hint: "",
            detail: "Return as list instead of count.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::PoolSelection,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        deprecated_replacement: Some("active_members"),
        ..CommandSpec::DEFAULT
    }
}
