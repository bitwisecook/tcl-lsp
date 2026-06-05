//! `ASM::threat_campaign` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::threat_campaign",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the list of threat campaigns.",
            synopsis: &["ASM::threat_campaign ( names | staged_names )"],
            snippet: "Returns the list of threat campaigns.",
            source: "https://clouddocs.f5.com/api/irules/ASM__threat_campaign.html",
            examples: "when ASM_REQUEST_DONE {\n    log local0. \"names=[ASM::threat_campaign names] staged_names=[ASM::threat_campaign staged_names]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::threat_campaign ( names | staged_names )" },
        ],
        ..CommandSpec::DEFAULT
    }
}
