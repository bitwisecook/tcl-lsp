# Lane: R11 — one relation mechanism

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
- [ ] C4 — event graph profile checks routed through the shared evaluator
- [ ] C5 — SpecTcl spelling + round trip for profile relations
- [ ] C6 — I6 monotone security merge, covering traits *and* relations
- [ ] C7 — gate against a new hand-rolled relation walker; docs

## Open

`ProfileSpec` relations carry no `Forbids` edges yet — every shipped
`conflicts` row was empty, so `BIGIP6039` still cannot fire. Unifying does not
change that; populating it from F5 appliance evidence (V-series) is the
separate follow-up the owner leaned towards.
