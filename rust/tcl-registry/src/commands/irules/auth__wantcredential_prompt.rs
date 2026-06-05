//! `AUTH::wantcredential_prompt` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_prompt",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a string for an authorization session authidXs credential prompt.",
            synopsis: &["AUTH::wantcredential_prompt AUTH_ID"],
            snippet: "Returns the authorization session authid’s credential prompt string\nthat the system last requested (when the system generated an\nAUTH_WANTCREDENTIAL event). An example of a promopt string is\nUsername:. The AUTH::wantcredential_prompt command is especially\nhelpful in providing authentication services to interactive protocols\n(for example, telnet and ftp), where the actual text prompts and\nresponses may be directly communicated with the remote user.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_prompt.html",
            examples: "when AUTH_WANTCREDENTIAL {\n  HTTP::respond 401 \"WWW-Authenticate\" \"Basic realm=\\\"\\\"\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
