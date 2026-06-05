//! `urlcatquery` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "urlcatquery",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Query the URL for URL categorization.",
            synopsis: &["urlcatquery URL_STRING"],
            snippet: "This command is similar in functionality to whereis command of geoip.\nThis will be available from HTTP_REQUEST irule event. It takes the URL\nas the input. The input could be a URL string or an IPV4 address. IPV6\naddresses are not currently supported. iRule returns the URL categories\nreturned by the urlcat library.",
            source: "https://clouddocs.f5.com/api/irules/urlcatquery.html",
            examples: "when HTTP_REQUEST {\n    set input_url [HTTP::host][HTTP::uri]\n    set urlcat [urlcatquery  $input_url]\n    log local0. \"INPUT-URL: $input_url\"\n    log local0. \"Category - $urlcat\"\n    CLASSIFY::urlcat add $urlcat\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "urlcatquery URL_STRING" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DataGroup,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        deprecated_replacement: Some("CATEGORY::lookup"),
        ..CommandSpec::DEFAULT
    }
}
