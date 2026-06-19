//! `pem_dtos` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "pem_dtos",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Queries DTOS (Device Type and OS) database.",
            synopsis: &["pem_dtos 'tac' 'lookup' PEM_DTOS_MCRO"],
            snippet: "Queries DTOS (Device Type and OS) database",
            source: "https://clouddocs.f5.com/api/irules/pem_dtos.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "pem_dtos 'tac' 'lookup' PEM_DTOS_MCRO",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}
