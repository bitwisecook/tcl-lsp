# Lane: R12 — one relation mechanism

> Numbered **R12**: `R11` was already taken by the F5 evidence-generated-rows
> ruling (centralisation §R11, ledger V9).

**Branch:** `claude/tcl-dialect-registry-design-lrzbsn`
**Ruling:** owner, 2026-08-28 — "is it something we should roll into one mechanism?" → *do it*.

## Why

The registry checked "X requires / conflicts with Y" in five places and four
vocabularies:

| Site | Shape before |
|---|---|
| `spec::OptionRelation` | typed, four kinds, native evaluator, lifecycle + dialect gating (E-R14) |
| `ProfileSpec::requires` | bare `&[&str]`, hand-walked as a transitive closure |
| `ProfileSpec::conflicts` | bare `&[&str]`, hand-walked pairwise for `BIGIP6039` — **every shipped row empty** |
| `EventProps::implied_profiles` / `::transport` | bare `&[&str]`, hand-walked as inference |
| `EventRequires` | six bespoke fields, hand-checked |

Only the first had lifecycle gating or evidence-carrying messages. This is the
"duplicate ways to do things" the owner has ruled against repeatedly, and it
predates the #1631 programme rather than being introduced by it.

## The design content

`requires` does **not** mean the same thing in the two domains, and that is the
one thing a naive merge would get wrong:

- `-command` requires `-channel` **asserts** — a call with the first and not the
  second is a defect.
- `HTTP` requires `TCP` **infers** — BIG-IP attaches the parent itself, so a
  config naming only `HTTP` is not missing anything; the edge exists to *add*
  `TCP` before anything is judged.

So `RelationMode { Assert, Infer }` is per edge, not derived from the kind.
`Relation::evaluate` short-circuits `Infer` edges; `closure_over` follows them.

## Shape after

- `tcl-registry::relation` — `Relation<T>`, `RelationKind`, `RelationMode`,
  `RelationTermKind`, `RelationFactSource`, `TermHolds`, `RelationVerdict<T>`,
  `RelationViolation<T>`, `closure_over`. Domain-neutral; no invocation
  assumptions.
- `spec::OptionTerm` (was `RelationTerm`) + `spec::OptionFacts` (was
  `RelationFacts`) + `pub type OptionRelation = Relation<OptionTerm>`.
- `profiles::ProfileTerm` + `profiles::ProfileFacts` +
  `pub type ProfileRelation = Relation<ProfileTerm>`.
- `ProfileSpec::relations` replaces `requires` + `conflicts`; the registry
  snapshot projects the two directions back out so its JSON is unchanged.

## Checkpoints

- [x] C1 — generic relation core in `tcl-registry::relation`, option domain ported, crate green
- [x] C2 — `ProfileSpec` on `relations`, `expand_profile_stack` via `closure_over`, snapshot projection
- [x] C3 — `tcl-bigip` validator conflict walk deleted, `BIGIP6039` on the shared evaluator
- [x] C4 — event graph profile checks routed through the shared evaluator
- [x] C5 — **not applicable, struck.** No SpecTcl surface declares a profile:
      `ProfileSpec` is a shipped Rust table with no `profile` block in the
      loader and no studio schema. There is nothing to spell.
- [x] C6 — I6 monotone security merge (`tcl-registry::security_floor`)
- [x] C7 — gates: `no_registry_table_declares_a_bare_relation_slice` and
      `every_security_bearing_field_is_in_the_floor`

## Open

`ProfileSpec` relations carry no `Forbids` edges yet — every shipped
`conflicts` row was empty, so `BIGIP6039` still cannot fire. Unifying does not
change that; populating it from F5 appliance evidence (V-series) is the
separate follow-up the owner leaned towards.


## What C4 actually did

`stack_satisfies` now reads its facts through `ProfileFacts`, the same source
the conflict checker uses, and lost its per-candidate closure: the active
expansion is a fixed point over the inference edges, so `expand(candidate) ⊆
expanded` reduces to `candidate ∈ expanded`. One closure per call instead of
one plus one per candidate, on a path hover and completion take
(`CommandRegistry::valid_irules_commands_for_event`).
`stack_satisfies_agrees_with_the_subset_formulation` holds the equivalence
over ~10,000 ordered pairs from the shipped table.

`EventRequires` deliberately keeps its compact record — 461 literals across as
many files, on that same hot path, where rebuilding a relation list per check
would allocate for no semantic gain. Its profile half is on the shared fact
source; the record is an authoring form, not a second vocabulary. The R12 gate
records that exemption by name rather than leaving it implicit.

## I6 — the monotone security merge (C6)

Not in the original plan for this lane. It landed here because one relation
vocabulary means packs can author relations, and a merge that lets an override
*delete* a shipped relation is the same defect as one that lets it clear
`TAINT_SINK` — so the floor had to be general rather than trait-specific.

The hole was real and measured, not inferred. Before this change:

```
speclib probe 2.0 {
  command exec -override {
    arity 1..
  }
}
```

in a `Tier::Workspace` pack loaded with `load_error = None` and produced an
`exec` with `TAINT_SINK = false` and `TAINT_SOURCE = false`, while the shipped
`exec` has `TAINT_SINK = true`. A repository could silence taint diagnostics
about its own code with four committed lines. `tests/i6_security_floor.rs` is
that probe, now asserting the opposite.

The floor is **not keyed on the tier**, deliberately. §6.4 keys its untrusted
class on the editor's Workspace Trust state, which nothing on the discovery
path is told (ledger O9), so a tier-keyed rule would protect nothing today;
and a security fact a *trusted* pack may quietly drop is not much of a
security fact. This resolves the "implement I6 regardless of tier" half of O9.
The other half — plumbing the editor's trust state so `WorkspaceUntrusted`
becomes reachable at all — is untouched and still open.
