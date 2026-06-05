//! `snit::widgetadaptor` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "snit::widgetadaptor name definition",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "snit::widgetadaptor",
        traits: Traits::CREATES_BARRIER | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::exact(2),
        hover: Some(HoverSnippet {
            summary: "Define a snit widget adaptor that wraps an existing widget.",
            synopsis: &["snit::widgetadaptor name definition"],
            snippet: "",
            source: "tcllib snit package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        arg_roles: &[(1, ArgRole::Body)],
        ..CommandSpec::DEFAULT
    }
}
