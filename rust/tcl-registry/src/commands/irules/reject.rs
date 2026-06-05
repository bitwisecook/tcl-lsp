//! `reject` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "reject",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Causes the connection to be rejected.",
            synopsis: &["reject"],
            snippet: "Causes the connection to be rejected, returning a reset as appropriate\nfor the protocol. Subsequent code in the current event in the current\niRule or other iRules on the VS are still executed prior to the reset\nbeing sent.\n\n**Warning**: After `reject`, the current iRule continues executing,\nand other iRules on the VS also run. This can cause TCL errors\n(e.g. `ASM::disable` on a rejected connection). Always follow\n`reject` with `event disable all` and `return`.\n\nIf the VS is using FastHTTP, reject commands will not work, at least\nunder 11.3.0.",
            source: "https://clouddocs.f5.com/api/irules/reject.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [TCP::local_port] != 443 } {\n    reject\n    event disable all\n    return\n  }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "reject" },
        ],
        ..CommandSpec::DEFAULT
    }
}
