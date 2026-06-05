//! `ONECONNECT::reuse` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::reuse",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Controls server-side connection reuse.",
            synopsis: &["ONECONNECT::reuse (BOOL_VALUE)?"],
            snippet: "This command controls whether server-side connections are picked from\nthe pool of idle connections, and whether idle server-side connections\nare returned to the pool or closed when a client connection detaches or\ncloses. It will also display the current status of connection reuse, if\ncalled without any options.\nFor information on how to control the detaching behavior, see\nONECONNECT::detach.\nThe semantics of this command depend on the context in which it is\nbeing executed. Refer to Considering Context Part 1 and\nConsidering Context Part 2 for more information on contexts.",
            source: "https://clouddocs.f5.com/api/irules/ONECONNECT-reuse.html",
            examples: "when HTTP_REQUEST {\nif {[HTTP::method] equals GET } {\n      ONECONNECT::reuse enable\n   } else {\n      ONECONNECT::reuse disable\n   }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ONECONNECT::reuse (BOOL_VALUE)?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
