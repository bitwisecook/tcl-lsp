//! `SSL::allow_nonssl` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::allow_nonssl",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set Allow Non-SSL connections.",
            synopsis: &["SSL::allow_nonssl (ZERO_ONE)?"],
            snippet: "SSL::allow_nonssl\n  Returns the currently set value for Allow Non-SSL connections\nSSL::allow_nonssl ( 0 | 1 )\n  0 disables Non-SSL Connections, 1 enables it.\n  Allow Non-ssl connections, sets SSL to passthrough mode.",
            source: "https://clouddocs.f5.com/api/irules/SSL__allow_nonssl.html",
            examples: "when CLIENT_ACCEPTED {\n    SSL::allow_nonssl 1\n}",
            return_value: "SSL::allow_nonssl Returns the currently set Allow Non-SSL connections value. SSL::allow_nonssl [0|1] There is no return value.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::allow_nonssl (ZERO_ONE)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
