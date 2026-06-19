//! `SSL::sni` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "name",
        arity: Arity::exact(0),
        detail: "Get SNI name.",
        synopsis: "SSL::sni name",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "required",
        arity: Arity::exact(0),
        detail: "Get SNI required setting.",
        synopsis: "SSL::sni required",
        pure: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::sni",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns Server Name Indication information.",
            synopsis: &["SSL::sni (name | required)"],
            snippet: "Returns a Server Name Indication name, and require SNI support.",
            source: "https://clouddocs.f5.com/api/irules/SSL__sni.html",
            examples: "when HTTP_REQUEST {\n    log local0.info \"SNI name: [SSL::sni name]\"\n    log local0.info \"SNI required: [SSL::sni required]\"\n}",
            return_value: "SSL::sni name Returns the current Server Name Indication as specified in the SSL profile. SSL::sni required Returns the require SNI support as specified in the SSL profile.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SSL::sni <name | required>",
        }],
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::SslState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        taint_source: Some(TaintColour::TAINTED.union(TaintColour::FQDN)),
        ..CommandSpec::DEFAULT
    }
}
