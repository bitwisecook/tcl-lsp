//! `CATEGORY::filetype` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CATEGORY::filetype",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get mime type and mime subtype of payload.",
            synopsis: &["CATEGORY::filetype HTTP_PAYLOAD"],
            snippet: "Checks for the mime type and mime subtype of an HTTP request payload and returns the values to specified variables; use one or both to specify them name of the variable that you want the value to be given to.",
            source: "https://clouddocs.f5.com/api/irules/CATEGORY__filetype.html",
            examples: "when HTTP_RESPONSE {\n    # Collect 256 bytes of payload and trigger HTTP_RESPONSE_DATA\n    HTTP::collect 256\n}",
            return_value: "Sets the specified variables with the mime type or mime subtype",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CATEGORY::filetype HTTP_PAYLOAD ?options?" },
        ],
        options: &[
            OptionSpec { name: "-mimetype", takes_value: true, value_hint: "TYPE", detail: "Variable name to store MIME type.", dialects: None },
            OptionSpec { name: "-mimesubtype", takes_value: true, value_hint: "SUBTYPE", detail: "Variable name to store MIME subtype.", dialects: None },
        ],
        ..CommandSpec::DEFAULT
    }
}
