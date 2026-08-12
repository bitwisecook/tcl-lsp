# <What the matrix maps, in a noun phrase>

<Lead paragraph, with no heading above it. Say which pipeline stage and
which fact families the matrix covers, and what goes wrong without it —
for example, that overlapping consumers duplicate diagnostics or break a
downstream assumption when no explicit ownership map exists. Two or
three sentences.
[`../compiler/pass-fact-ownership-matrix.md`](../compiler/pass-fact-ownership-matrix.md)
is the worked example.>

## Contracts

<The ownership rules the matrix enforces. Three is the usual shape: who
owns a fact, what a consumer may and may not do with it, and what a
producer change obliges. Delete this section if the table alone carries
the contract.>

1. **<One primary owner per fact family.>** <What that means here.>
2. **<Consumers do not redefine producer semantics.>** <What that means
   here.>
3. **<Ownership changes require cross-pass validation.>** <What that
   means here.>

## <Owner> → <fact> → <consumer>

<State the path root once above the table — "All paths are relative to
`rust/<crate>/src/` unless stated otherwise" — so the cells stay short.
The entry-point column names real functions or types; check each against
the source.>

| Owner | Primary facts produced | Typical consumers | Entry points |
|---|---|---|---|
| `<module>/` | <facts> | <consumers> | `<fn_name>` |
| `<module>.rs` | <facts> | <consumers> | `<fn_name>` |

## Failure modes

<What goes wrong when ownership drifts — two passes emitting overlapping
findings, a consumer assuming an invariant a refactor dropped, an
aggregator treating a derived fact as canonical.>

- <Failure mode 1>
- <Failure mode 2>

## Test anchors

<The tests that catch an ownership regression. Delete this section
rather than guess.>

- `rust/<crate>/tests/<file>.rs` — <what it guards>

## Related docs

<`## Related docs` is the closing-pointer heading used under
`docs/design/compiler/`. Use `## Discoverability` instead if the page is
filed under `docs/design/contracts/`.>

- [compiler design index](../compiler/README.md)
- <neighbouring doc, with a clause saying what it answers that this one
  does not>
