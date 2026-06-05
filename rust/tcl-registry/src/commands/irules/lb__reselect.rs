//! `LB::reselect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::reselect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Advance to the next available node in a pool.",
            synopsis: &["LB::reselect (clone pool POOL_OBJ (member IP_ADDR)?)?", "LB::reselect pool POOL_OBJ (member ((IP_ADDR PORT) |", "LB::reselect snat (automap |", "LB::reselect snatpool SNAT_POOL_OBJ (member IP_ADDR)?"],
            snippet: "This command is used to advance to the next available node in a pool, either using the load balancing settings of that pool, or by specifying a member explicitly. Note that the reselect may not happen immediately; it may wait until the current iRule event is completely finished executing.\n\nThere is no reselect retry limit built into the command: You MUST implement a limiting mechanism in your iRule using logic similar to that in the examples below.",
            source: "https://clouddocs.f5.com/api/irules/LB__reselect.html",
            examples: "when CLIENT_ACCEPTED {\n    set def_pool [LB::server pool]\n    set lb_fails 0\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["LB_FAILED", "LB_QUEUED", "LB_SELECTED", "PERSIST_DOWN"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "LB::reselect (clone pool POOL_OBJ (member IP_ADDR)?)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::PoolSelection,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Server,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
