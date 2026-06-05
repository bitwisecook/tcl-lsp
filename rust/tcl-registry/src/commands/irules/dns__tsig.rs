//! `DNS::tsig` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::tsig",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Manipulates the current DNS message and its TSIG resource record.",
            synopsis: &["DNS::tsig 'remove'", "DNS::tsig 'exists'"],
            snippet: "This command manipulates the current DNS message and its TSIG resource\nrecord.\n\nNote: This command requires the DNS Profile, which is only enabled as\npart of GTM or the DNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__tsig.html",
            examples: "when DNS_REQUEST {\n  if { [DNS::tsig exists] } {\n    DNS::tsig remove\n  }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}
