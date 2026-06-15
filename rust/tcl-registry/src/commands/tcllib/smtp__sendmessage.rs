//! `smtp::sendmessage` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::NetworkIo,
    reads: false,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "smtp::sendmessage token ?options?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "smtp::sendmessage",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet {
            summary: "Send an e-mail message via SMTP.",
            synopsis: &[
                "smtp::sendmessage token ?-servers list? ?-ports list? ?-username user? ?-password pass? ?-usetls bool? ?-tlspolicy cmd? ?-originator addr? ?-recipients list? ?-header {key value} ...?",
            ],
            snippet: "",
            source: "tcllib smtp package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        side_effects: SIDE_EFFECTS,
        tcllib_package: Some("smtp"),
        required_package: Some("smtp"),
        ..CommandSpec::DEFAULT
    }
}
