# Root-context command inventory observations

Original C8.4–9.1 and current Jim inputs/outputs and before/after executable
and library seals are retained. C8.6/9.0/9.1 and Jim distinguish :N::x from
:::N::x in the fixed leading-colon namespace control. C8.4/8.5 instead expose
their older procedure-publication behavior; those captures stay separate.
These observations do not prove Runtime inventory or Engine extension loading.

Runtime transports actual namespace components, simple names and generations;
a proposed root operand must select the exact actual slot and generation.
Opaque/ambiguous rows remain explicit and the String-based extension roster
refuses them. Reports are never reparsed to recover identities.

The Main shared Runtime API compiles in the retained check. Runtime/Engine
consumer controls and real-WASM export/load checks have not executed.
