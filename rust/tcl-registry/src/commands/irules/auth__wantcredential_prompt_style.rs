//! `AUTH::wantcredential_prompt_style` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "AUTH::wantcredential_prompt_style",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns an authorization session authidXs credential prompt style.",
            synopsis: &["AUTH::wantcredential_prompt_style AUTH_ID"],
            snippet: "Returns the authorization session authid’s credential prompt style that\nthe system last requested (when the system generated an\nAUTH_WANTCREDENTIAL event). The value of the <authid> argument is\neither echo_on, echo_off, or unknown. This command is especially\nhelpful in providing authentication services to interactive protocols\n(or example, telnet and ftp), where the actual text prompts and\nresponses may be directly communicated with the remote user.",
            source: "https://clouddocs.f5.com/api/irules/AUTH__wantcredential_prompt_style.html",
            examples: "when AUTH_WANTCREDENTIAL {\n  HTTP::respond 401 \"WWW-Authenticate\" \"Basic realm=\\\"\\\"\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AUTH::wantcredential_prompt_style AUTH_ID" },
        ],
        ..CommandSpec::DEFAULT
    }
}
