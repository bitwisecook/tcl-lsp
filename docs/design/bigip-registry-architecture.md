# BIG-IP object registry — architecture

This is the technical contract for the BIG-IP object registry — the single
source of truth `f5` uses for every TMSH object kind, every property, every
value type. The user-facing companion is
[`docs/kcs/features/kcs-feature-bigip-registry.md`](../kcs/features/kcs-feature-bigip-registry.md).

## Overview

The registry tells the rest of the system how to:

- Classify a property's value shape.
- Project that value through the query DSL.
- Render it back to TMSH source.
- Enumerate the references it carries for the graph / LSP layers.

The user-facing payoff is consistent behaviour across `f5 grep`, `f5 query`,
`f5 explain`, `f5 rename`, document links, definition jump, references,
rename, and semantic tokens: every place that asks "what does this property
reference?" hits the same dispatch.

The catalogue is a compile-time static in `rust/tcl-registry/src/bigip/`;
the typed values it describes live in `rust/tcl-bigip/src/value/`; the
consumers are `rust/tcl-bigip` (graph, links, tmsh emission) and
`rust/tcl-bigip-query` (DSL projection).

## Object kind catalogue

Every object kind is a curated `BigipObjectSpec` entry under
`rust/tcl-registry/src/bigip/data/`, split into one module per initial
letter (`a.rs`, `c.rs`, `g.rs`, …) purely to keep each file compilable at a
sane size. Hundreds of distinct kinds across:

- **`ltm.*`** — pool, virtual, monitor (40+ types), profile (60+ types),
  node, rule, persistence, snatpool, policy, data-group, global-settings,
  classification, …
- **`gtm.*`** — pool, server, wideip, region, monitor, listener,
  prober-pool, distributed-app, …
- **`security.*`** — firewall policy / rule-list / port-list / address-list
  / schedule, NAT policy / source-translation, ip-intelligence, dos
  profile, log profile, ssh profile, …
- **`net.*`** — vlan, self, route, route-domain, tunnel, ipsec,
  rate-shaping, fdb, packet-filter, …
- **`sys.*`** — file ssl-cert / ssl-key, management-route, log-config-*,
  crypto-*, ntp / dns / snmp / global-settings, …
- **`apm.*`**, **`auth.*`**, **`cm.*`**, **`pem.*`**, **`vcmp.*`**,
  **`cli.*`**, **`api_protection.*`** — full coverage.

`BigipRegistry` (built once via `default_registry()`) owns the lookup
surface:

| Method | Answers |
|---|---|
| `get(kind)` | The spec for a canonical kind name (`"ltm_virtual"`). |
| `get_by_header(module, object_type)` / `kind_for_header` | Which kind a parsed stanza header names. |
| `candidate_registry_kinds_for_display(display_kind)` | Translate a display-form kind back to the registry's underscored keys. |
| `candidate_kinds_for_key` / `candidate_kinds_for_section_item` | Which kinds a nested key or section item could belong to. |
| `kind_names()` / `specs()` | Whole-catalogue enumeration, for audits and dumps. |

## Property model

Each property is a `BigipPropertySpec` — a plain `Copy` struct of static
slices, with `..BigipPropertySpec::DEFAULT` filling the unset fields:

| Field | Meaning |
|---|---|
| `name` | Property identifier as it appears in tmsh. |
| `value_type` | `ValueKind` — the collapsed value tag. |
| `shape_kind` | A richer kind when `value_type` collapses the scalar (an `Enum` of `disabled`/`enabled` carrying `shape_kind: Boolean`). |
| `in_sections` | Parent sections this property may appear in; empty means unconstrained. |
| `required` / `repeated` / `allow_none` | Create-time requirement, multiplicity, and whether the literal `none` clears the value. |
| `enum_values`, `min_value`, `max_value`, `pattern` | The permitted value space. |
| `references` | The object kinds this property may name — the outbound graph edges. |
| `default`, `description`, `usage_flags` | Documented default, prose, and lifecycle flags (`deprecated`, `read_only`, `not_synced`, …). |
| `list_operators` | The tmsh list operators the property accepts (`add` / `delete` / `modify` / `replace-all-with`). |
| `block` | Nested sub-properties for object-shaped blocks. |

Two derived predicates carry contract weight:

- `is_list_valued()` is defined as `!list_operators.is_empty()`. A property
  is list-valued to the emitter *because* it declares operators, not
  because its `value_type` is `List` — so a `List` property that declares
  no operators is deliberately excluded from full-body list emission.
- `is_block()` is `!block.is_empty()`.

`ValueKind` is the canonical vocabulary: `String`, `Integer`, `Float`,
`Boolean`, `Enum`, `Reference`, `List`, `Block`, `IpAddress`, `Endpoint`,
`Object`, `Unknown`. `as_str()` gives each a stable wire tag
(`"ip-address"`, …) that the audit dumper and JSON output use.

## Reference edges

`ReferenceEdge` records `(from_kind, property) -> to_kinds`.
`reference_edges_from(kind)` iterates a kind's outbound edges and
`reference_targets(from_kind, property)` resolves one property's candidate
targets. This is the declarative half of the graph: it says which
properties *can* point somewhere, without saying how to parse the value.

## Typed values

The value types the registry's vocabulary describes are concrete Rust types
in `rust/tcl-bigip/src/value/`. Each round-trips to its canonical F5
spelling through `Display` and parses through `parse` / `try_parse`, so a
reconstructed object compares equal to the parsed one.

### Scalar foundations

- `Address` / `IPAddress` / `FQDN` (`address.rs`) — IPv4, IPv6, FQDN.
- `Network` / `Cidr` (`network.rs`) — CIDR with the `default` keyword and
  dotted-quad netmask round-trip.
- `Port` (`port.rs`), `PortSet` (`port_set.rs`) — single port and wildcards.
- `Partition` (`partition.rs`), `Folder` / `ObjectPath` (`folder.rs`) —
  path identity.
- `RouteDomain` (`route_domain.rs`), `IPRange` (`ip_range.rs`).
- `BigipList` / `ListItem` / `ListItemValue` / `ListSyntax` /
  `SourceSpan` (`bigip_list.rs`) — the typed list, one `ListItem` per
  element, each carrying its own source span.

### Compound (per-property)

- `Destination` (`destination.rs`) — `ltm virtual` / pool-member
  destination (`[partition/]address[%route-domain][.|:port]`).
- `MonitorExpression` / `MonitorMode` (`monitor_expression.rs`) —
  pool / node / GTM monitor expression (`default` / `none` / a single
  monitor / `M1 and M2` / `min N of { … }`), with per-monitor source spans.
- `ProfileAttachment` / `PersistenceAttachment` (`attachments.rs`) —
  `ltm virtual.profiles` and `.persist` keyed-block items.
- `SnatMode` (`snat_mode.rs`) — the `source-address-translation` sum type
  (automap / snat-pool / none).
- `DataGroupRecord` (`data_group_record.rs`) — an
  `ltm data-group internal.records` entry.
- `GtmRegionMember` (`gtm_region_member.rs`) — a `gtm region.region-members`
  row.
- `CertKeyChain` (`cert_key_chain.rs`) — an
  `ltm profile {client,server}-ssl.cert-key-chain` entry.
- `FirewallRule` / `FirewallEndpoint` / `NatRule` (`firewall_rule.rs`) —
  a `security firewall rule-list.rules[]` body.
- The `ltm policy` rule / condition / action types (`policy.rs`).

## Registry-first reference dispatch

`tcl_bigip::graph::pilot_references(module, object_type, property, raw)` is
the registry-first edge path. It returns `Some(refs)` for a property whose
reference extraction is modelled, and `None` for one that is not — in which
case `build_forward_edges` falls through to the unconditional legacy
passes. Adding a property to this dispatch is the unit of migration.

Each arm is a slim extractor over the raw property text rather than a full
typed-value materialisation, because the graph only consumes each
reference's `(target_kind, target_path)` pair. The modelled shapes:

| Shape | Properties |
|---|---|
| List of object refs | `ltm virtual.rules` / `.policies` / `.vlans`, `security firewall policy.rule-lists`, `security firewall address-list.address-lists` |
| Keyed-block whose key *is* the referenced path | `ltm virtual.profiles`, `ltm virtual.persist` |
| Monitor expression | `ltm pool` / `ltm node` / `gtm pool` / `gtm server` `.monitor` |
| SNAT mode | `ltm virtual.source-address-translation` |
| Keyed-block with refs in each body | `ltm profile {client,server}-ssl.cert-key-chain`, `security firewall rule-list.rules` |

Properties that are typed but reference-free — `destination`, data-group
records, GTM region members — deliberately return `None` and take the
legacy path, since they contribute no edges.

## Source-range fidelity

Every reference the dispatch emits carries the exact byte span where the
reference token lives in the source. The LSP layer consumes these for:

- **Document links** (`rust/tcl-bigip/src/links.rs`) — one `DocumentLink`
  per reference, scoped to the reference token rather than the surrounding
  property line. iRule bodies always emit a link (with a `None` target and
  a "no definition found" tooltip when it does not resolve); registry
  properties emit one only when the reference resolves.
- **Go to definition** — walks a block's properties through the registry
  and picks the reference whose span covers the cursor offset.
- **Semantic tokens** — emits an `object` token at every registry reference
  with a populated range.
- **References / rename** (`rust/tcl-bigip/src/refs.rs`) — resolves the
  `/Partition/Name` path-shaped token under the cursor and returns every
  identifier-bounded occurrence of it. Bare (non-`/`) tokens are not
  renameable BIG-IP paths, so a cursor on one yields nothing. Both are
  single-document providers; cross-file resolution awaits a workspace
  config index.

## Query DSL surface

`rust/tcl-bigip-query/src/projection.rs` turns a parsed `BigipConfig` into
a lazily-navigated tree of `Container`s: a synthetic `<root>` holds one
child per module, each module container one child per kind, each kind
container the objects projected as `ObjectRef`s. Objects only materialise
when navigated into, and the refs memoise on `Root.object_cache` keyed by
`(kind, full_path)`.

Each object's top-level scalar properties get a `field_slot` — the byte
range of the value half — so the edit-plan engine can rewrite a single
property in place, and a `stanza_slot` from the object's range so `--scf`
and auto output match the canonical layout.

A projected property exposes its structured fields directly:

```
.ltm.pool[].monitor.mode                # "default" | "single" | "all" | "min-of"
.ltm.pool[].monitor.monitors[]          # one path per referenced monitor
.ltm.pool[].monitor.minimum             # threshold for min-of

.ltm.virtual[].profiles[].context       # "clientside" | "serverside" | "all" | ""
.ltm.virtual[].profiles[] |             #   filter to client-side profiles
  select(.context == "clientside") |
  .name

.ltm.virtual[].persist[].default        # boolean flag
.ltm.virtual[].persist[] |              #   filter to the default persistence profile
  select(.default) | .name

.ltm.virtual[].destination.host         # typed Destination
.ltm.virtual[].destination.port         #   structured children

.ltm.policy[].rules[].actions[].pool    # a reference to the pool the action forwards to
```

PathRef-style accessors (`.profiles[].full-path`, `.monitor.full-path`)
continue to work in parallel via aliases on the structured types, so
existing queries keep their shape.

## tmsh emission

`rust/tcl-bigip/src/tmsh_emit.rs` renders objects back to `tmsh create` /
`tmsh modify` commands. The full-body modify form requires an explicit
operator on every list-valued field —

```
tmsh modify ltm virtual /Common/v { rules replace-all-with { /Common/r1 } }
```

— because `tmsh modify ltm virtual /Common/v { rules { /Common/r1 } }` is
rejected by the device. `list_field` emits `replace-all-with`
unconditionally, which is correct for every property the emitter currently
touches (pool members, data-group records, snatpool members, virtual
profiles / rules / persist): the registry declares that operator on all of
them.

## Extending the registry

Adding a new typed value:

1. Define the type in `rust/tcl-bigip/src/value/` with `Display` +
   `parse` / `try_parse`, and re-export it from `value/mod.rs`.
2. Give the property its `BigipPropertySpec` fields in the relevant
   `rust/tcl-registry/src/bigip/data/*.rs` file — at minimum `value_type`,
   plus `references` and `list_operators` where they apply.
3. Add a `ReferenceEdge` if the property names another kind.
4. Add an arm to `pilot_references` in `rust/tcl-bigip/src/graph.rs` so the
   graph, links, and definition layers see the edge.
5. Extend the projection in `rust/tcl-bigip-query/src/projection.rs` if the
   DSL should expose the structured fields.

The catalogue-wide invariants are guarded by
`rust/tcl-registry/tests/registry_sweep.rs`, which walks every property of
every kind and asserts the derived predicates agree with the underlying
slices, that an `Enum` kind enumerates values or declares a `shape_kind`,
and that no reference target or enum member is empty.
