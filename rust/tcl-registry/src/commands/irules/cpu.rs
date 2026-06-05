//! `cpu` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cpu",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the average TMM cpu load for the given interval.",
            synopsis: &["cpu usage ("],
            snippet: "The cpu usage command returns the average TMM cpu load for the given\ninterval. All averages are exponential weighted moving averages over\nthe interval.",
            source: "https://clouddocs.f5.com/api/irules/cpu.html",
            examples: "when HTTP_REQUEST {\n  if{ [cpu usage 5sec] <= 1} {\n    pool1\n  } else {\n    HTTP::redirect \"http://anotherpool.com\"\n  }\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "cpu usage (" },
        ],
        ..CommandSpec::DEFAULT
    }
}
