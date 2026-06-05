//! `TMM::cmp_count` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TMM::cmp_count",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Provides the active number of TMM instances running.",
            synopsis: &["TMM::cmp_count"],
            snippet: "This command provides the active number of TMM instances running.\nTo determine the blade the iRule is currently executing on, see the\nTMM::cmp_group page. To determine the CPU ID an iRule is currently\nexecuting on within a blade, see the TMM::cmp_unit page.",
            source: "https://clouddocs.f5.com/api/irules/TMM__cmp_count.html",
            examples: "when CLIENT_ACCEPTED {\n  if { [TMM::cmp_count] >= 2 } {\n    set cmpstatus 1\n  } else { set cmpstatus 0 }\n}",
            return_value: "Returns the active number of TMM instances running.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TMM::cmp_count" },
        ],
        ..CommandSpec::DEFAULT
    }
}
