//! `drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "drop",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Causes the current packet or connection to be dropped/discarded.",
            synopsis: &["drop"],
            snippet: "Causes the current packet or connection (depending on the context of\nthe event) to be dropped/discarded and the rule continues (no implied\nreturn). This command is identical to discard.\n\n**Warning**: After `drop`, the current iRule continues executing, and\nother iRules and later priorities in this event also run. This can\ncause TCL errors (e.g. `ASM::disable` on a dropped connection).\nAlways follow `drop` with `event disable all` and `return`.",
            source: "https://clouddocs.f5.com/api/irules/drop.html",
            examples: "when HTTP_REQUEST {\n  if { [IP::addr [IP::client_addr] equals 10.1.1.80] } {\n    drop\n    event disable all\n    return\n  }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "drop" },
        ],
        ..CommandSpec::DEFAULT
    }
}
