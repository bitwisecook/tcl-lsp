//! `SOCKS::version` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SOCKS::version",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command gets the version of the SOCKS protocol.",
            synopsis: &["SOCKS::version"],
            snippet: "This command gets the version of the SOCKS protocol, returning one of \"4\", \"4A\" or \"5\".\n\nDetails (Syntax):\nSOCKS::version\n    Gets the version of the protocol.",
            source: "https://clouddocs.f5.com/api/irules/SOCKS__version.html",
            examples: "when SOCKS_REQUEST {\n    log local0. \"SOCKS is using version [SOCKS::version]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SOCKS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SOCKS::version" },
        ],
        ..CommandSpec::DEFAULT
    }
}
