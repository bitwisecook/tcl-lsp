//! `IPFIX::template` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IPFIX::template",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "IPFIX::template Provides the ability to create and delete IPFIX message templates that may be used to generate IPFIX messages based on processing in the iRule.",
            synopsis: &["IPFIX::template ( (create TEMPLATE_STRING) |"],
            snippet: "This command provides the ability to create and delete user defined IPFIX\nmessage templates that may be used to send IPFIX messages to a specified\ndestination.",
            source: "https://clouddocs.f5.com/api/irules/IPFIX__template.html",
            examples: "when RULE_INIT {\n    set static::http_track_dest \"\"\n    set static::http_track_tmplt \"\"\n}",
            return_value: "IPFIX::template create TEMPLATE_STRING returns an IPFIX template object that is used by the IPFIX::msg create command and IPFIX::template delete command.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IPFIX::template ( (create TEMPLATE_STRING) |",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
