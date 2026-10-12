# W210 current boundary (temporary source review)

Read-only diagnosis; no Cargo, oracle, compiler/test, Git or proof changes.
Reviewed four compiler owners; `native_each.rs` was neither read nor edited.
Published C-source inventory/#2287 success is separate from these negatives.
`experiments/name-handling/results/lifecycle-argument-offset/result.json`
records Tcltest 6 pass/3 fail: first C8.4 direct/alias/oracle missing-read
controls return W210 0 instead of 1/1/2. Those counts do not locate a guard.
The real matching-init+tcltest C probe remains the missing-read/cleanup oracle;
compiler source inspection does not substitute for its reached observations.

## One concrete boundary

The earliest **observed** refusal is `Builder::selection` returning `None`
for the outer `test`, after entry/config/words matched in the older
`/tmp/w210-debug-selection.log`. Current negative receipts contain no inner
guard trace, so no particular metadata refusal is established for frozen code.
`command_binding/declaration_flow.rs:854` then makes the dispatch unknown and
returns before `command_flow` builds setup/body/cleanup. This concrete selection
boundary blocks the declared phase graph that would contain `$absent`.

The first carrier to instrument is `retained.source_binding`, constructed at
`declaration_flow.rs:794–815`, before the metadata call at 817. It starts as a
**clone of tokens**. Matching inventory points (dispatch, offset, origin and
declaration variable frame) replace that binding only when at least one exists.
An empty filtered iterator does not itself clear an inherited `lookup_state`.
Consequently “no matching inventory dispatch” does not prove “no retained point”.
No current receipt records either value; this is a source-proven distinction,
not evidence that a stale or invalid point actually caused these failures.

## Exact refusal and purpose boundaries

`registry_invocation/declaration_flow.rs:168` converts the metadata `Result`
to `Option` with `.ok()?`; all metadata errors become indistinguishable `None`.
`OriginalDeclarationWordSource::metadata_context`
(`command_binding/declaration_layout.rs:1976`) first checks purpose, full vector,
origin, registry semantic key, dialect, original lexical command, invocation site
and lexer config. A present `lookup_state` calls
`original_invocation_metadata_at_point` at 2022, then requires reached/declaration
metadata-owner equality. That refusal is terminal; declaration rows cannot mend it.
Without a point it instead requires unanimous original rows owning the source,
with equal words/config/metadata owner. SuppliedSource and explicit Standalone
are admitted; Supplied availability-only and Unavailable owners refuse.

`command_binding/materialized_footprint.rs:458` selects the actual point owner.
Its SuppliedSource path requires original config/site/origin/registry geometry
and complete input equality (or the exact nested-source input); partial owners
refuse. Its separate absent-point source-advice route at 490 requires positively
Logical declaration metadata. Native input cannot take that fallback (313).
These source/conditional schemas do not grant Native dispatch, entered frames,
Normal handlers, physical reads, stores or edit permission. Registry completion
descriptors consumed by the graph are also conditional, not observed completion.

## Narrow next experiment

Under the build owner's lease, instrument only the first outer `test` in the
direct negative C8.4 control. Record incoming binding/point presence, filtered
dispatch count and retained binding/point presence; preserve the metadata Result
before `.ok()?`, its first failed named guard, owner tag and owner equality.
Also distinguish layout-advice/dialect failure before metadata from later
registry-resolution/arity/completion failure and scoped-lifecycle refusal.
Stop at the earliest failed guard. If selection succeeds, then record phase
entry ownership, authored `$absent`, warning-path reachability and warning_reads.
An ordinary `proc p {} {puts $absent}` control may compare the same carrier join;
it is a separate input, never authority for the Tcltest source or execution.
Do not relax a retained invalid-owner guard or manufacture an absent Native point.
