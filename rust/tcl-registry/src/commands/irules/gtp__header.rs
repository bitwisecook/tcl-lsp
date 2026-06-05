//! `GTP::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Allows for the parsing of GTP header information.",
            synopsis: &["GTP::header ('version' | 'type') ('-message' MESSAGE)?", "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?", "GTP::header ('teid' | 'npdu' | 'sequence') 'set' ('-message' MESSAGE)? VALUE", "GTP::header ('teid' | 'npdu' | 'sequence') 'remove' ('-message' MESSAGE)?"],
            snippet: "Allows for the parsing of GTP header information. UINT -- Unsigned\ninteger value of n bits. For n > 8, appropriate network to host byte\norder conversion happens transparently.",
            source: "https://clouddocs.f5.com/api/irules/GTP__header.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    log local0. \"GTP version [GTP::header version]\"\n    log local0. \"GTP type [GTP::header type]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
