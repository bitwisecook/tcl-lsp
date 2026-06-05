//! `POLICY::targets` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "POLICY::targets",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns or sets properties of the policy rule targets for the policies associated with the virtual server that the iRule is enabled on.",
            synopsis: &["POLICY::targets ('ltm-policy' |"],
            snippet: "Returns or sets properties of the policy rule targets for the policies\nassociated with the virtual server that the iRule is enabled on. A\npolicy rule target can be considered an action that the policy uses if\nthe rule conditions are met.\n\nAs of v11.4 the following policy targets are available:\n wam              - Application Acceleration Manager (AAM)\n asm              - Application Security Manager\n log              - Log\n http-cookie      - HTTP cookie\n http-header      - HTTP header\n http-host        - HTTP host header\n http-referer     - HTTP referer header",
            source: "https://clouddocs.f5.com/api/irules/policy__targets.html",
            examples: "# Log the policy targets for this virtual server\nwhen HTTP_REQUEST {\n\n        # Log the policy targets enabled on this virtual server\n        log local0. \"\\[POLICY::targets\\]: [POLICY::targets]\"\n\n        # Loop through each possible target type and log whether it is enabled or not (1 for enabled, 0 for not enabled)\n        foreach target {asm wam log http-cookie http-header http-host http-referer http-set-cookie http-uri log tcl tcp-nagle} {",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "POLICY::targets ('ltm-policy' |" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::BigipConfig,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
