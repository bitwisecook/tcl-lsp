//! `discard` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "discard",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Causes the current packet or connection to be dropped/discarded.",
            synopsis: &["discard"],
            snippet: "Causes the current packet or connection (depending on the context of\nthe event) to be dropped/discarded and the rule continues (no implied\nreturn). This command is identical to drop.\n\n**Warning**: After `discard`, the current iRule continues executing,\nand other iRules and later priorities in this event also run. This\ncan cause TCL errors. Always follow `discard` with `event disable\nall` and `return`.",
            source: "https://clouddocs.f5.com/api/irules/discard.html",
            examples: "when HTTP_REQUEST {\n  if { [IP::addr [IP::client_addr] equals 10.1.1.80] } {\n    discard\n    event disable all\n    return\n  }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "discard" },
        ],
        ..CommandSpec::DEFAULT
    }
}
