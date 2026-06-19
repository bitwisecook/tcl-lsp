//! `COMPRESS::buffer_size` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "COMPRESS::buffer_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets the compression buffer size.",
            synopsis: &["COMPRESS::buffer_size (request | response)? NONNEGATIVE_INTEGER"],
            snippet: "COMPRESS::buffer_size <value>\n    Sets the compression buffer size according to the value you specify in bytes.",
            source: "https://clouddocs.f5.com/api/irules/COMPRESS__buffer_size.html",
            examples: "when HTTP_RESPONSE {\n  if { [HTTP::header Content-Type] contains \"text/html;charset=UTF-8\"} {\n    COMPRESS::buffer_size 10240\n    COMPRESS::enable\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "COMPRESS::buffer_size (request | response)? NONNEGATIVE_INTEGER",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
