//! `STREAM::max_matchsize` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::max_matchsize",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets a maximum number of bytes that the system can buffer during partial matches.",
            synopsis: &["STREAM::max_matchsize SIZE"],
            snippet: "Sets the maximum size, in bytes, that the system can buffer during\npartial matches. The default value is 4096.\nThe STREAM profile will buffer data for partial matches; if more than\nmax_matchsize would be buffered, the connection will be torn down. This\nway a regex like foobarbaz+ won't keep matching until the box runs\nout of memory. The default is 4K, and STREAM::max_matchsize can be\nuse to set it to something else.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__max_matchsize.html",
            examples: "when HTTP_RESPONSE {\n    STREAM::max_matchsize 2048\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "STREAM::max_matchsize SIZE",
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
