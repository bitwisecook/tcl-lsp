//! `HTTP::cookie` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTTP::cookie",
        traits: Traits::PURE | Traits::CSE_CANDIDATE | Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Queries for or manipulates cookies in HTTP requests and responses.",
            &["HTTP::cookie <subcommand> ?arg ...?"],
            "F5 iRules",
        )),
        // GAP-D2: `HTTP::cookie insert|replace` with tainted data →
        // header injection (IRULE3002). Mirrors `irules/http__cookie.py`.
        // TODO(consumer): GAP-3a — once the iRules subcommands are
        // re-ported, attach `credential_arg=2` + `sensitive_headers` to
        // the `insert` / `replace` SubCommand specs.
        taint_output_sink: Some("IRULE3002"),
        taint_output_sink_subcommands: &["insert", "replace"],
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: Some("tcp"),
            profiles: &["FASTHTTP", "HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}
