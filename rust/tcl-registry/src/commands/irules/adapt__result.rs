//! `ADAPT::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ADAPT::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sets or returns the adaptation result code.",
            synopsis: &["ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?"],
            snippet: "The ADAPT::result command sets or returns the adaptation result\ncode of the ADAPT filter on the current or specified side of the\nvirtual server connection for which the iRule is being executed.\n\nPossible result codes are:\n    * unknown - The internal virtual server has not returned a\n      result yet. It is not possible to change the result code\n      to this value.\n    * bypass - The internal virtual server does not need to modify\n      the request or response.",
            source: "https://clouddocs.f5.com/api/irules/ADAPT__result.html",
            examples: "when ADAPT_REQUEST_RESULT {\n     if {[ADAPT::result] == \"respond\"} {\n        # Force ADAPT to ignore any direct response from IVS\n        # (contrived example, probably not useful as-is).\n        ADAPT::result bypass\n     }\n}",
            return_value: "Returns the current or modified result code.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["REQUESTADAPT", "RESPONSEADAPT"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ADAPT::result (ADAPT_CTX)? (ADAPT_SIDE)? ('bypass' | 'close' | 'abort')?" },
        ],
        ..CommandSpec::DEFAULT
    }
}
