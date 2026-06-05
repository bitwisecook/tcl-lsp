//! `lseq` — generate a list of numeric values in a range (Tcl 9.0).
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "lseq ?start? ?op? end ?by step? ?count n?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "lseq",
        traits: Traits::PURE.union(Traits::CSE_CANDIDATE),
        dialects: Some(DialectSet::TCL90),
        arity: Arity::new(1, 5),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet::brief(
            "Generate a list of numeric values in a range.",
            &[
                "lseq n",
                "lseq start end ?step?",
                "lseq start to end",
                "lseq start 'count' count",
                "lseq start 'by' step 'count' count",
            ],
            "Tcl man page lseq.n (Tcl 9.0)",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
