//! `priority` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "priority",
        traits: Traits::IRULES_TOP_LEVEL_ONLY,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets the order of execution for iRule events.",
            synopsis: &["priority EVENT_PRIORITY"],
            snippet: "The priority command is used as an attribute associated with any iRule\nevent. When the iRules are loaded into the internal iRules engine for a\ngiven virtual server, they are stored in a table with the event name\nand a priority (with a default of 500).\nLower numbered priority events are evaluated before higher numbered\npriority events: When an event is triggered an event, the irules engine\npasses control to each of the code blocks for that given event in the\norder of lowest to highest priority.",
            source: "https://clouddocs.f5.com/api/irules/priority.html",
            examples: "when CLIENT_ACCEPTED {\n       log \"Client [IP::remote_addr] connected\"\n    }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "priority EVENT_PRIORITY" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
