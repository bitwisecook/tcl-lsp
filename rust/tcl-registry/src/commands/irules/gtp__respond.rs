//! `GTP::respond` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::respond",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends the GTP message back to the remote node of this connection.",
            synopsis: &["GTP::respond MESSAGE"],
            snippet: "Sends this GTP message back to the remote node of this connection.\nIf this is clientside flow, send it back to client that initiated the connection.\nIf this is serverside flow, send it to the remote node that is connected to.",
            source: "https://clouddocs.f5.com/api/irules/GTP__respond.html",
            examples: "when GTP_SIGNALLING_EGRESS {\n    set t2 [GTP::new 2 10]\n    GTP::respond $t2\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GTP::respond MESSAGE" },
        ],
        ..CommandSpec::DEFAULT
    }
}
