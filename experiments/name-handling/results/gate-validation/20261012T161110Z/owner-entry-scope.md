# How should owner entry visibility be checked?

Keep the existing manifest's public-entry default strict. A declaration is
public only when its item and enclosing type/module route permit a consumer
outside the owning crate to reach it. A method on a private type does not become
an external API merely because the method is `pub`.

Internal implementation entries need explicit, narrower contracts. Each such
entry should identify its source, owning Rust module, symbol, required scope,
and actual consumer modules. Validate the declaration and consumer calls, then
check the real visibility boundary of every enclosing item:

| Entry scope | Required check |
| --- | --- |
| Public | Exported declaration, enclosing type/module visibility, and an exported route to every consumer outside the crate. |
| Crate | `pub(crate)` or broader declaration and enclosing items; all declared consumers belong to the owning crate. |
| Restricted module | Resolve `pub(super)` or `pub(in ...)` to its actual module boundary; every declared consumer lies inside that boundary. |
| Private implementation | The item and enclosing type are reachable from the declared consumers under Rust's private module rules. Never infer reachability from a matching symbol elsewhere. |

For example, `runtime/rust/src/interp.rs` declares
`Interp::register_stock_builtin` as `pub(crate)`, and
`runtime/rust/src/cmd_alias.rs` calls it from the same crate. It belongs to an
explicit crate-scope implementation contract, while the externally callable
`Interp::with_native_core` remains a public owner entry. A request to call
`register_stock_builtin` from `xtask` must fail that scope check.

Likewise, a `pub` field on a private `Frame` needs the owning module and actual
consumer module recorded. It must fail a public-entry claim even if a sibling
implementation has valid local access. Such entries must not be promoted to
public merely to satisfy the manifest.

An internal entry can be recorded separately from the five-column public-owner
manifest, or under an explicitly parsed internal-implementation part of its
semantic axis. Either format must preserve strict public defaults and require
the source/module/consumer data above. The present run changes neither format
nor the scanner's public visibility rule.

The two uncovered owner bullets also need exact spellings: brace shorthand
`Commands::{dispatch,dispatch_id}` does not identify two individually validated
entry points. The four `check-name-resolution-proofs` references require the
actual Makefile-to-check implementation route to be declared and checked; a
non-xtask check cannot silently pass as an xtask gate.
