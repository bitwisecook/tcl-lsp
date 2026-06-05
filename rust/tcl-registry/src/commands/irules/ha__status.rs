//! `HA::status` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "HA::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns true or false based on whether the unit the command is executed on is active or standby.",
            synopsis: &["HA::status (active | standby)"],
            snippet: "This iRule command returns true or false based on whether the unit the\ncommand is executed on is active or standby in the context of the\ncommand used. The primary use-case is for iRules that utilize sideband\nor HSL commands. This can be used to prevent the standby from opening\nextra connections.\nA Virtual IP (VIP) is bound to a Traffic Group, which handles failover\nfor the VIP. A unit can, at the same time, be \"active\" for one\ntraffic-group and \"standby\" for a different traffic-group.",
            source: "https://clouddocs.f5.com/api/irules/HA__status.html",
            examples: "when CLIENT_ACCEPTED {\n    log local0. \"active: [HA::status active]\"\n    log local0. \"standby: [HA::status standby]\"\n}",
            return_value: "HA::status active",
        }),
        excluded_events: &["RULE_INIT"],
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HA::status (active | standby)" },
        ],
        ..CommandSpec::DEFAULT
    }
}
