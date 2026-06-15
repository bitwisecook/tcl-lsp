//! `ACCESS::log` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ACCESS::log",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Logs a message using APM logging framework.",
            synopsis: &["ACCESS::log (COMPONENT_LOGLEVEL)? MSG"],
            snippet: "ACCESS::log [component.][loglevel] <message>\n\nLogs the specified message using the optionally specified APM component name\nand log level as specified in the log setting for the access profile that is\nassigned to the virtual server.\nThe message is sent to the destination specified in the log setting.\nIf component is specified, it must be one of the supported values (see below)\nand must end with a dot character. If not specified, the accesscontrol component\nis assumed.\nIf log level is specified, it must be one of the supported values (see below).\nIf not specified, notice level is assumed.",
            source: "https://clouddocs.f5.com/api/irules/ACCESS__log.html",
            examples: "when HTTP_REQUEST {\n    ACCESS::log debug \"an Access Control debug log\"\n    ACCESS::log sso.error \"an SSO error log\"\n    ACCESS::log eca. \"an ECA notice log\"\n    ACCESS::log \"an Access Control notice log\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ACCESS::log (COMPONENT_LOGLEVEL)? MSG",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::LogIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
