# BIG-IP Object Registry — Architecture

This is the technical contract for the BIG-IP object registry — the
single source of truth `f5` uses for every TMSH object kind, every
property, every value type.  The user-facing companion is
[`docs/kcs/features/kcs-feature-bigip-registry.md`](../kcs/features/kcs-feature-bigip-registry.md).

## Overview

The registry tells the rest of the system how to:

- Parse a property's raw text into a structured value.
- Project that value through the query DSL.
- Render it back to TMSH source.
- Enumerate the references it carries for the graph / LSP layers.

The user-facing payoff is consistent behaviour across `f5 grep`,
`f5 query`, `f5 explain`, `f5 rename`, document links, definition
jump, references, rename, and semantic tokens: every place that
asks "what does this property reference?" hits the same dispatch.

## Object kind catalogue

Every object kind the registry understands is a curated entry in
`dialects/f5/bigip/registry/specs/`.  Hundreds of distinct kinds across:

- **`ltm.*`** — pool, virtual, monitor (40+ types), profile (60+
  types), node, rule, persistence, snatpool, policy, data-group,
  global-settings, classification, …
- **`gtm.*`** — pool, server, wideip, region, monitor, listener,
  prober-pool, distributed-app, …
- **`security.*`** — firewall policy / rule-list / port-list /
  address-list / schedule, NAT policy / source-translation,
  ip-intelligence, dos profile, log profile, ssh profile, …
- **`net.*`** — vlan, self, route, route-domain, tunnel, ipsec,
  rate-shaping, fdb, packet-filter, …
- **`sys.*`** — file ssl-cert / ssl-key, management-route,
  log-config-*, crypto-*, ntp / dns / snmp / global-settings, …
- **`apm.*`**, **`auth.*`**, **`cm.*`**, **`pem.*`**, **`vcmp.*`**,
  **`cli.*`**, **`api_protection.*`** — full coverage.

Look up the registry key for a header via
`HEADER_KIND_MAP[(module, object_type)]` or — to translate a
display-form kind back to the registry's underscored keys —
`candidate_registry_kinds_for_display(display_kind)`.

## Value-spec architecture

Every property's value is described by a :class:`ValueSpec`.  The
protocol is intentionally narrow:

```
class ValueSpec:
    def parse(raw: str, ctx)    -> ParsedValue
    def project(value, ctx)     -> object         # DSL projection
    def render(value, ctx)      -> str            # TMSH text
    def references(value, ctx)  -> Iterable[Reference]
```

Concrete specs in `dialects/f5/bigip/registry/value_specs.py`:

### Scalar foundations

- **`StringSpec`** — opaque text (descriptions, free-form strings).
- **`IntSpec`** — bounded integers with min/max validation.
- **`BoolSpec`** — enabled/disabled, yes/no, true/false, on/off, 1/0.
- **`EnumSpec`** — one-of value, with diagnostic on unknown values.
- **`ObjectRefSpec`** — full-path reference to another kind.
- **`ListSpec`** — list-shaped value owning parse / project /
  render / references for every item; emits :class:`BigipList`
  with :class:`ListItem` per element.

### Domain-specific

- **`DestinationSpec`** — `ltm virtual` / pool-member destination
  (`[partition/]address[%route-domain][.|:port]`).
- **`NetworkSpec`** — CIDR with `default` keyword + dotted-quad
  netmask round-trip.
- **`AddressSpec`** — IPv4 / IPv6 / FQDN.
- **`PortSpec`** — single port + wildcards.

### Compound (per-property)

- **`MonitorExpressionSpec`** — pool / node / GTM monitor
  expression (`default` / `none` / single / `M1 and M2` / `min N of
  { … }`); exposes `.mode`, `.monitors[]`, `.minimum`, with
  per-monitor source spans.
- **`ProfileAttachmentSpec`** — `ltm virtual.profiles` keyed-block
  item (`/Common/clientssl { context clientside }`); exposes
  `.path`, `.context`, `.name`.
- **`PersistenceAttachmentSpec`** — `ltm virtual.persist` item;
  exposes `.path`, `.default`, `.name`.
- **`SnatModeSpec`** — `source-address-translation` sum type
  (automap / snat-pool / none); resolves snatpool refs.
- **`DataGroupRecordSpec`** — `ltm data-group internal.records`
  entry; exposes `.key`, `.data`.
- **`GtmRegionMemberSpec`** — `gtm region.region-members` row.
- **`CertKeyChainSpec`** — `ltm profile {client,server}-ssl.cert-
  key-chain` entry; resolves cert / key / chain / passphrase
  references.
- **`LtmPolicyConditionSpec`** — `ltm policy.rules[].conditions[]`.
- **`LtmPolicyActionSpec`** — `ltm policy.rules[].actions[]`.
- **`LtmPolicyRuleSpec`** — full `ltm policy` rule with nested
  conditions and actions; resolves pool refs from `forward select
  pool` actions.
- **`FirewallRuleSpec`** — `security firewall rule-list.rules[]`
  body (action / ip-protocol / source / destination endpoints);
  resolves nested port-list / address-list / rule-list refs.
- **`NatRuleSpec`** — alias for `FirewallRuleSpec`.

## Pilot migration table

The pilot table in `dialects/f5/bigip/registry/pilot.py` opts properties
into the new dispatch.  Adding an entry is the unit of migration —
projection, edit, parser, and reference layers all consult the
table before falling back to the legacy `BigipPropertySpec` path.

Migrated properties as of this release:

- `ltm virtual` — `destination` (`DestinationSpec`), `source-
  address-translation` (`SnatModeSpec`), `profiles` /
  `persist` / `rules` / `policies` / `vlans` (each `ListSpec`).
- `ltm pool` / `ltm node` / `gtm pool` / `gtm server` — `monitor`
  (`MonitorExpressionSpec`).
- `ltm data-group internal` — `records` (`ListSpec[DataGroupRecord]`).
- `ltm policy` — `rules` (`ListSpec[LtmPolicyRule]`).
- `ltm profile client-ssl` / `ltm profile server-ssl` —
  `cert-key-chain` (`ListSpec[CertKeyChain]`).
- `gtm region` — `region-members` (`ListSpec[GtmRegionMember]`).
- `security firewall rule-list` — `rules`
  (`ListSpec[FirewallRule]`).
- `security firewall policy` — `rule-lists` (`ListSpec` of refs).
- `security firewall address-list` — `address-lists` (nested-list
  refs).

## Source-range fidelity

Every :class:`Reference` the dispatch emits carries a
:class:`SourceRange` pointing at the exact byte span where the
reference token lives in the source.  The LSP layer consumes
these for:

- **Document links** — `lsp/features/_bigip_links.py` emits a
  `DocumentLink` per reference, with the range scoped to the
  reference token (not the surrounding property line).
- **Go to definition** — `lsp/features/definition.py` walks every
  block's properties through the registry and picks the
  reference whose span covers the cursor offset.
- **Semantic tokens** — `lsp/features/_semantic_tokens/_bigip.py`
  emits `object` tokens at every registry reference with a
  populated range.
- **Reference / rename** — uses TMSH-path-bounded regex (already
  byte-precise); registry-driven filtering is a future
  enhancement.

## Query DSL surface

When a property is migrated, its DSL projection exposes the
structured fields directly:

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

.ltm.policy[].rules[].actions[].pool    # PathRef into the pool the action forwards to
```

The legacy PathRef-style accessors continue to work in parallel
(`.profiles[].full-path`, `.monitor.full-path`) via aliases on
the structured types so existing queries keep their shape.

## Extending the registry

Adding a new typed value:

1. Define the typed value in `dialects/f5/bigip/types/` (`@dataclass(frozen=True, slots=True)`).
2. Add a `*Spec` class in `dialects/f5/bigip/registry/value_specs.py`
   implementing `parse` / `project` / `render` / `references`.
3. Register the spec in the pilot table
   (`dialects/f5/bigip/registry/pilot.py`) keyed by
   `(module, object_type, property_name)`.
4. Add audit tests in
   `tests/test_bigip_registry_value_specs.py`.
5. If the model needs a typed companion field (e.g.
   `profile_attachments` alongside legacy `profiles`), update
   the parser to populate it.

## Operational context

The registry was rolled out in six phases (Phases 1–6 of the
"BIG-IP property registry rearchitecture"):

1. ValueSpec scaffolding.
2. Projection dispatch through ValueSpec.
3. Edit rendering through ValueSpec.
4. Parser dispatch through ValueSpec.
5. Reference dispatch through ValueSpec.
6. Compound specs for keyed-block lists and rule bodies.

All six phases are complete; the pilot table grows incrementally
as more properties opt into the typed path.
