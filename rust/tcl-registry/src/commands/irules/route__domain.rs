//! `ROUTE::domain` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ROUTE::domain",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the current routing domain of the current connection.",
            synopsis: &["ROUTE::domain"],
            snippet: "Returns the current routing domain of the current connection. Several\ncommands allow an addition rt_domain option: node, snat, LB::status",
            source: "https://clouddocs.f5.com/api/irules/ROUTE__domain.html",
            examples: "when CLIENT_ACCEPTED {\n    set gateway 10.3.1.11\n    set bandwidth [ROUTE::bandwidth [IP::remote_addr] $gateway%[ROUTE::domain]]\n    if { $bandwidth > 0 } {\n        log local0. \"Destination found in cache, bandwidth = $bandwidth\"\n    }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ROUTE::domain" },
        ],
        ..CommandSpec::DEFAULT
    }
}
