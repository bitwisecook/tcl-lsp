# Proposed crate-scope namespace revision owner entry

2026-10-12 read-only diagnosis; only this temporary proposal is written.
No source, durable contract, canonical proof, Git or Cargo changes/checks.

Selected seam: Runtime namespace lookup-policy revision. The frozen receipt
`experiments/name-handling/results/gate-validation/20261012T161110Z/owner-resolution-frozen.log:290`
rejects `Namespaces::guard_lookup_environment_revision` as a public owner entry.
The gate requires `Visibility::Public` (`rust/xtask/src/owner_resolution.rs:1670–1672`);
its current manifest validation has no separately parsed internal-entry contract
(`owner_resolution.rs:245–272`). Do not broaden this default or promote the API.

Current owner: `runtime/rust/src/namespace.rs:2561–2563`, a `pub(crate)` method
on public `Namespaces` (`:457`) in public `crate::namespace` (`lib.rs:149`).
Only actual consumer call: `runtime/rust/src/interp.rs:3507`, inside private
`Interp::synchronise_command_environment_guards` (`:3506`). Its receiver is
`Interp::namespaces() -> Ref<Namespaces>` (`:6360`), not a same-named foreign API.
Checked increment exhaustion yields `None` (`namespace.rs:2571–2575`); the
consumer poisons CommandEnvironment, Namespace and UnknownHandling (`interp.rs:3510–3520`).

Exact post-thaw contract change: remove only this symbol from the public-entry
cell of `Runtime command registration and identity` (`shared-utility-contracts-rust.md:80`).
Add a separately parsed `owner-resolution-internal-manifest` table, ending at
`end-owner-resolution-internal-manifest` (both HTML-comment markers), with this row:

| Owner | Source | Crate | Module | Entry | Scope | Actual consumer |
| --- | --- | --- | --- | --- | --- | --- |
| Runtime lookup-policy revision | `runtime/rust/src/namespace.rs` | `tcl-runtime` | `crate::namespace` | `Namespaces::guard_lookup_environment_revision` | crate | `runtime/rust/src/interp.rs`, `crate::interp`, `Interp::synchronise_command_environment_guards`, line 3507 |

Require declaration, source-to-module route, enclosing type, owning Cargo crate,
consumer function and typed call resolution. Unknown/missing scope fails; existing
five-column rows remain public. Do not silently discard the moved entry.
Minimal method doc: `/// Lookup-policy revision; None means increment exhaustion.`

Post-thaw gate cases: this internal row resolves; the unchanged public claim
fails; an xtask consumer fails crate scope; a mismatched receiver/module or private
method inaccessible from the declared consumer fails. Preserve the existing API.
Other Runtime registration-row entries and non-xtask gate routes remain separate
obligations. No gate pass or reduction of the frozen 165 obligations is claimed.
