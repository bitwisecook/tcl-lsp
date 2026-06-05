//! `DNS::rpz_policy` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::rpz_policy",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the RPZ policy associated with the DNS cache.",
            synopsis: &["DNS::rpz_policy"],
            snippet: "Returns the RPZ (Response Policy Zones) policy associated with the DNS cache.\n\nThe possible return values are:\n    * \"\" (empty string) if RPZ is not configured.\n    * \"NXDOMAIN\" if RPZ is configured to return an NXDOMAIN response on a match.\n    * \"WG <walled garden name>\" if RPZ is configured to return a Walled Garden redirect on a match.",
            source: "https://clouddocs.f5.com/api/irules/DNS__rpz_policy.html",
            examples: "when DNS_RESPONSE {\n     if { [DNS::origin] eq \"RPZ\"} {\n        log local0. \"[DNS::question name] resulted in an RPZ [DNS::rpz_policy]\"\n     }\n}",
            return_value: "* \"\" (empty string) if RPZ is not configured. * \"NXDOMAIN\" if RPZ is configured to return an NXDOMAIN response on a match. * \"WG <walled garden name>\" if RPZ is configured to return a Walled Garden redirect on a match.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DNS::rpz_policy" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::DnsState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
