# <ownership matrix topic>

## Symptom

<Where ownership confusion creates regressions or duplication.>

## Operational context

<Which pipeline stage/fact families are covered.>

## Decision rules / contracts

1. <Primary owner rule>
2. <Consumer behaviour rule>
3. <Change-management rule>

## <Entity> -> <fact> -> <consumer> matrix

| Owner | Facts | Consumers | Anchors |
|---|---|---|---|
| `...` | `...` | `...` | `...` |

## File-path anchors

- `rust/<crate>/src/...` — the crate that owns the contract
- `rust/<crate>/src/...` — the consuming crate

## Failure modes

- <Failure mode 1>
- <Failure mode 2>

## Test anchors

- `rust/<crate>/src/...` — unit tests guarding the contract
- `rust/<crate>/tests/...` — integration tests

## Discoverability

- [compiler design index](../compiler/README.md)
- related note: `../compiler/<note>.md`
