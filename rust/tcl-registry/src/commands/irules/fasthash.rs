//! `fasthash` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "fasthash",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a hash for the specified string.",
            synopsis: &["fasthash DATA"],
            snippet: "fasthash is guaranteed to return a high quality hash of the input as quickly as practical. The hash value returned is between 0 and 2^63-1 inclusive (a positive integer).\n\nfasthash was added because there are many use cases (ie CARP) which need a hash of some value (ie URI) and which were using crc32 (which is a bad and slow hash function).\n\nNote: fasthash does not guarantee to provide the same hash value across different BIGIP versions and over BIGIP reboots. Do not use fasthash for long term and persistent storage.",
            source: "https://clouddocs.f5.com/api/irules/fasthash.html",
            examples: "when CLIENT_ACCEPTED {\n    set str \"hello world\"\n    log local0. \"hash of $str is [fasthash $str]\"\n}",
            return_value: "Returns the numeric hash for the specified string",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "fasthash DATA" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}
