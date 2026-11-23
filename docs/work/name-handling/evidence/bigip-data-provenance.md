# BIG-IP data provenance and ownership

Reviewed against `work` at `c0980e638a`. The live comparison is in the
[21.1 TMSH checks](../../../../evidence/bigip/tmsh-21.1-name-handling.md). The history below records what can be
recovered and which original inputs are still missing.

## Q2. Table and generator provenance

| Current data | Recovered origin | Recovery decision |
| --- | --- | --- |
| `tcl-registry/src/base_objects/generated_{objects,names}.rs` | Added in `671227f905` (#796). `BASE_OBJECTS_VERSION` identifies the one base configuration snapshot as 17.1. The companion module explains which object identities and fields are required for SCF fallback. The original extractor and its exact file manifest/build were not located. | Rebuild the extraction from a versioned manifest of shipped base files. Emit identities separately from selected field defaults. Use the current 17.1 output as a historical comparison fixture until the original input can be recovered. |
| `tcl-registry/src/bigip/references.rs` | Added in #796 as a reference-edge projection with canonical kinds. The old `scripts/registry-audit/gen_bigip_rust.py` produces `bigip/data/` from the reconciled Python `OBJECT_SPECS`. It does not emit `references.rs`. The exact original edge emitter and appliance release are unverified. | Derive edges from the canonical property and section descriptors. Encode genuine multi-target or nested exceptions in that schema, then check that every target resolves and every emitted edge has a property source. |
| `tcl-registry/src/profile_defaults/generated.rs` | Historical `58633c7049:scripts/registry-audit/gen_profile_defaults.py` parses root `ltm profile` stanzas from `profile_base.conf`. #796 describes the later 17.1 snapshot and the field overrides in `profile_defaults/mod.rs`. The recovered script is an earlier version and its output was not reproduced byte for byte. | Port the extraction to Rust using the SCF parser. Preserve source lexemes, field types and release provenance. Keep documented overrides distinct until captures confirm them. |
| `tcl-registry/src/events/event_props_table.rs` | Extracted from `events.rs` in `971901ba71`. #796 describes the profile implications in its predecessor as hand-curated and records 84 disagreements across 158 shared events with a schema-derived profile projection, plus 18 events absent from that projection. The historical `gen_event_descriptions.py` emits event prose in another file. The emitter and single source release for this property table were not located. | Keep authored event properties with per-fact provenance and consistency checks. Generate projections only from those properties. |
| `tcl-registry/src/profiles/profile_table.rs` | Extracted from `profiles.rs` in `971901ba71`. The generated annotation predates the split and is visible in the Rust port. No verified emitter, input manifest or single appliance release was located. This table models relationships and lifecycles. | Author the relationship facts with references, validate their graph and test combinations against TMSH. Keep the separate field-default extraction mechanical. |
| `tcl-registry/src/event_facts/generated.rs` | Added in #796. The commit describes schema-derived event rows and removal of the conflicting `profiles` field. Only protocol categories remain here. The generator is mentioned in the commit, but its source and exact schema release were not located. | Move categories into the versioned event catalogue, preserve evidence for each row and generate compatibility views where needed. |
| `tcl-bigip/src/model/gen/` | `553b71b6dd:scripts/codegen/gen_bigip_model_rust.py` introspects Python dataclasses under `dialects/f5/bigip/model/`. Its implementation writes the per-module structs, `parsers.rs`, `object.rs`, `canon.rs`, `dispatch.rs` and `mod.rs`. That script was removed in the Python retirement (#736). The dataclasses were a model input with no one appliance build established by the generator. | Use the recovered generator to understand historical field mappings. Generate the current model from the Rust-side schema and typed model annotations, preserving bespoke semantics and compatibility tests. |

History references: [#788 profile import](https://github.com/bitwisecook/tcl-lsp/pull/788),
[#796 defaults, references and event categories](https://github.com/bitwisecook/tcl-lsp/pull/796),
[#736 Python retirement](https://github.com/bitwisecook/tcl-lsp/pull/736),
[recovered model generator](https://github.com/bitwisecook/tcl-lsp/blob/553b71b6dd/scripts/codegen/gen_bigip_model_rust.py),
[recovered profile generator](https://github.com/bitwisecook/tcl-lsp/blob/58633c7049/scripts/registry-audit/gen_profile_defaults.py)
and [table split](https://github.com/bitwisecook/tcl-lsp/commit/971901ba71).

The separate local `bigip-extract` checkout contains a 21.0.0.1 build 0.0.13
schema capture, extracted TMSH manpages and a script that generates historical
Python object specs. This is a recoverable secondary input for checking
catalogue coverage. Its presence does not establish that any of the tables
above came from that build. The current native
[`bigip-data-schema` gate](../../../../rust/xtask/src/bigip_data_schema.rs)
checks the curated object specs. The
[editor grammar generator](../../../../rust/xtask/src/gen_bigip_grammars.rs)
projects vocabulary and has its own check.

## Q3. Release snapshots

17.1 is the only embedded base-object snapshot. The 21.1.0.1 appliance
supplied a second measured release point for the client and server SSL
`options` fields. The shipped stanzas and the effective default profiles both
list `dont-insert-empty-fragments`, `no-ssl`, `no-tlsv1.1` and
`no-tlsv1`. The current registry's open-ended 17.1 override contains
`no-tlsv1.3` and `no-dtlsv1.2` instead of those three protocol flags.
This refutes carrying the 17.1 row forward indefinitely. It does not locate
the first build on which each flag changed.
[Shipped and effective results](../../../../evidence/bigip/tmsh-21.1-name-handling.md#shipped-and-effective-defaults) ·
[Current field overrides](../../../../rust/tcl-registry/src/profile_defaults/mod.rs)

Capture complete normalized snapshots per exact release/build and platform or
module set where that affects the base. Use one typed schema, one build
manifest per capture and deterministic add/change/remove deltas or computed
version intervals for the shipped representation. Preserve absence as
distinct from unknown. Start with 17.1.x, 17.5.x and 21.1.x for live coverage.
Keep 15.1.x and 16.1.x as historical import and regression lanes. Label
unmeasured builds and any nearest-snapshot fallback. Archive-provided
configuration values remain authoritative for that archive.
[F5 support policy](https://my.f5.com/manage/s/article/K5903)

## Q4. Ownership

Extract the configuration vocabulary and versioned data into the dedicated
data crate proposed in item 118. It owns TMSH kind/property and section
descriptors, reference-edge definitions, shipped base identities and profile
default snapshots. Give it a dependency direction that lets both
`tcl-registry` and `tcl-bigip` consume the same facts. Generate the generic
parts of `tcl-bigip`'s typed model from the schema plus explicit model
annotations for bespoke types. Keep iRule commands, profiles as event
requirements, protocol phases and event order in `tcl-registry` so the
language tools can consume them independently of configuration parsing.
[Current catalogue contract](../../../design/f5/bigip-registry-architecture.md)

| Component | Responsibility |
| --- | --- |
| `tcl-bigip` | Parse and render SCF. Own typed value interpretation, object identities and scoped reference resolution. Build the graph of actual objects and relationships from the parsed device and the canonical reference schema. |
| `tcl-bigip-query` | Project the typed model and shared graph into query objects and builtins. Own query semantics, including filtering and traversal. |
| `bigip-report-gen` | Project devices and their shared graphs into reports. Own cross-device presentation, report summaries and layout. Its browser event display should consume a generated projection of registry facts. |
| `f5-xc` | Own XC equivalents, advisories and deliberate conversion losses. Read TMOS event and configuration semantics from their owners. |

Current graph entry points are
[`tcl-bigip`](../../../../rust/tcl-bigip/src/graph.rs),
[report generation](../../../../rust/bigip-report-gen/rust/src/graph.rs) and
[query builtins](../../../../rust/tcl-bigip-query/src/builtins/graph.rs).
A live configuration with two valid monitor edges is a useful cross-consumer
fixture because a first-word projection loses one of them.
