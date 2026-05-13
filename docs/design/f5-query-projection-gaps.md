# f5 query — projection enrichment backlog

## What this file is

A working checklist of properties that the authoritative TMSH option
set documents on a configured object but the typed projection in
`core/bigip/query/projection.py` does not surface today. Each line
names one property; the corresponding field needs to be parsed in
`core/bigip/parser.py` (after the dataclass in `core/bigip/model.py`
gains the field) and exposed via the per-kind field map in
`core/bigip/query/projection.py`.

## How to work through it

1. Pick a bundle (a top-level `##` header below).
2. For every unticked field in the bundle:
   - Add the field (with an empty / default value) to the relevant
     dataclass in `core/bigip/model.py`.
   - Populate it in the relevant `_parse_*` function in
     `core/bigip/parser.py`. Use `_description` / `_unquote` /
     `_state_flag` where appropriate. Flag-style options (`internal`,
     `ip-forward`, …) live in `props` with an empty string value;
     surface them as `bool` via `"<flag>" in props`.
   - Add an entry to the per-kind `_*_FIELDS` map in
     `core/bigip/query/projection.py`. Promote to `ref_kind=` when
     the field references another projected kind.
3. Add at least one test in `tests/test_f5_query.py` covering the new
   field (and any new `PathRef` chain it enables).
4. Tick the line in this file.
5. Commit the bundle as one change.

Run `make ci-fast` before committing each bundle, and the broader
`tests/test_f5_query.py` plus `tests/` sweep before opening a PR.

## Notes on the checklist

- Some "options" in the underlying TMSH grammar are enum values
  (`automap`, `lsn`, `snat` under `source-address-translation type`),
  bare keywords (`add` / `delete` / `modify` / `replace-all-with`),
  or sub-block keys. These have been filtered out — only items below
  are real, addressable properties.
- Where the new field is a reference to another projected kind, the
  line carries a `→` annotation. Promote those to `PathRef` with the
  appropriate `ref_kind=` rather than a plain string.
- `(needs sub-block walker)` marks properties that live inside a
  numbered or anonymous sub-block; the parser already has helpers
  (`_collect_named_property_from_subblocks`,
  `_collect_named_property_from_anon_subblocks`,
  `_parse_multitoken_keyed_entries`) — pick or extend them as needed.

---

## Bundle 1 — cert / key metadata

### `cm cert`

- [x] `issuer`
- [x] `subject`
- [x] `subject-alternative-name`
- [x] `expiration-date`
- [x] `expiration-string`
- [x] `fingerprint`
- [x] `serial-number`
- [x] `version`
- [x] `key-type`
- [x] `certificate-key-size`
- [x] `is-bundle`
- [x] `email`
- [x] `source-path`
- [x] `system-path`
- [x] `size`
- [x] `mode`
- [x] `create-time`
- [x] `created-by`
- [x] `last-update-time`
- [x] `updated-by`

### `cm key`

- [x] `key-size`
- [x] `key-type`
- [x] `security-type`
- [x] `source-path`
- [x] `system-path`
- [x] `size`
- [x] `mode`
- [x] `create-time`
- [x] `created-by`
- [x] `last-update-time`
- [x] `updated-by`

### `sys file ssl-cert` (extend)

- [x] `expiration-date`
- [x] `key-type`
- [x] `is-bundle`
- [x] `certificate-key-size`
- [x] `issuer-cert` → `sys file ssl-cert`
- [x] `serial-number`
- [x] `version`
- [x] `subject-alternative-name`
- [x] `bundle-certificates`
- [x] `cert-validation-options`
- [x] `cert-validators`
- [x] `checksum`
- [x] `mode`
- [x] `size`
- [x] `create-time`
- [x] `created-by`
- [x] `last-update-time`
- [x] `updated-by`

### `sys file ssl-key` (extend)

- [x] `checksum`
- [x] `mode`
- [x] `size`
- [x] `create-time`
- [x] `created-by`
- [x] `last-update-time`
- [x] `updated-by`

---

## Bundle 2 — `ltm virtual` flags + refs

- [x] `address-status`
- [x] `auto-discovery`
- [x] `cmp-enabled`
- [x] `eviction-protected`
- [x] `dhcp-relay` (bool flag)
- [x] `internal` (bool flag)
- [x] `ip-forward` (bool flag)
- [x] `l2-forward` (bool flag)
- [x] `reject` (bool flag)
- [x] `nat64`
- [x] `gtm-score`
- [x] `mirror`
- [x] `service-down-immediate-action`
- [x] `source-port`
- [x] `serverssl-use-sni`
- [x] `rate-limit-dst-mask`
- [x] `rate-limit-src-mask`
- [x] `per-flow-request-access-policy` → `apm policy access-policy`
- [x] `transparent-nexthop` → `net vlan`
- [x] `rate-class`

---

## Bundle 3 — `ltm pool` scalars

- [x] `connection-limit`
- [x] `rate-limit`
- [x] `ratio`
- [x] `down-interval`
- [x] `interval`
- [x] `min-up-members-action`
- [x] `min-up-members-checking`
- [x] `ip-tos-to-client`
- [x] `ip-tos-to-server`
- [x] `link-qos-to-client`
- [x] `link-qos-to-server`
- [x] `gateway-failsafe-device`
- [x] `ignore-persisted-weight`
- [x] `inherit-profile`
- [x] `queue-on-connection-limit`
- [x] `address-family`
- [x] `autopopulate`
- [x] `profiles` (list) → `ltm profile`

---

## Bundle 4 — `ltm persistence` behaviour flags

- [x] `match-across-pools`
- [x] `match-across-services`
- [x] `match-across-virtuals`
- [x] `mirror`
- [x] `override-connection-limit`
- [x] `cookie-name`
- [x] `cookie-encryption`
- [x] `cookie-encryption-passphrase`
- [x] `httponly`
- [x] `secure`
- [x] `expiration`
- [x] `method`
- [x] `hash-length`
- [x] `hash-offset`
- [x] `always-send`

---

## Bundle 5 — `net.*` enrichments

### `net vlan`

- [x] `failsafe-action`
- [x] `failsafe-timeout`
- [x] `fwd-mode`
- [x] `hardware-syncookie`
- [x] `learning`
- [x] `tag-mode`
- [x] `virtual-wire`
- [x] `source-checking`
- [x] `syn-flood-rate-limit`
- [x] `syncache-threshold`
- [x] `service-policy`

### `net self`

- [x] `service-policy`
- [x] `fw-enforced-policy`
- [x] `fw-staged-policy`
- [x] `inherited-traffic-group`
- [x] `address-source`

### `net route-domain`

- [x] `bwc-policy`
- [x] `connection-limit`
- [x] `flow-eviction-policy`
- [x] `routing-protocol` (list)
- [x] `security-nat-policy`
- [x] `service-policy`

### `net interface`

- [x] `mtu`
- [x] `flow-control`
- [x] `mac-address`
- [x] `media-active`
- [x] `media-max`
- [x] `media-sfp`
- [x] `port-fwd-mode`
- [x] `qinq-ethertype`
- [x] `stp`
- [x] `stp-edge-port`
- [x] `stp-link-type`
- [x] `stp-auto-edge-port`
- [x] `stp-reset`
- [x] `sflow` (surfaced as `sflow-poll-interval` and `sflow-poll-interval-global`)
- [x] `vendor`
- [x] `vendor-oui`
- [x] `vendor-partnum`
- [x] `vendor-revision`
- [x] `virtual-wire`
- [x] `transmitter-technology`
- [x] `lacp-port-priority`

### `net tunnels tunnel`

- [x] `mtu`
- [x] `mode`
- [x] `idle-timeout`
- [x] `auto-lasthop`
- [x] `secondary-address`
- [x] `traffic-group` → `cm traffic-group`
- [x] `transparent`
- [x] `key`
- [x] `use-pmtu`
- [x] `tos`

### `net dns-resolver`

- [x] `nameservers` (surfaces entry keys)
- [x] `answer-default-zones`
- [x] `prefetch`
- [x] `nameserver-min-rtt`
- [x] `nameserver-ttl`
- [x] `outbound-msg-retry`

### `net stp`

- [x] `priority`
- [x] `external-path-cost`
- [x] `internal-path-cost`
- [x] `vlans` (list) → `net vlan`

---

## Bundle 6 — `apm` enrichments

### `apm oauth db-instance`

- [x] `db-name`
- [x] `purge-frequency`
- [x] `purge-time`

### `apm policy agent`

The current projection covers `agent_type` and `customization_group`
only. Each agent sub-type carries its own grammar; the items below
are the meaningful, addressable properties that occur across the
common AAA / ending / Kerberos sub-types.

- [x] `auth` (bool flag — e.g. on AAA agents)
- [x] `auth-max-logon-attempt` / `max-logon-attempt`
- [x] `fetch-nested-groups`
- [x] `fetch-primary-groups`
- [x] `password-source`
- [x] `query`
- [x] `query-attrname`
- [x] `query-filter`
- [x] `server` → `apm aaa <type>` (no projected kind yet — kept as a plain string)
- [x] `show-extended-error`
- [x] `upn`
- [x] `username-source`
- [x] `attribute-consuming-service`
- [x] `attr-consuming-service-session-var`
- [x] `hints`

Additionally, the parser now recognises all ~50 documented
`policy agent <type>` sub-kinds (AAA, accounting, endpoint, logon,
ending, OAuth, SAML, …) — previously only `ending-allow`,
`ending-deny`, and `kerberos` were dispatched.

---

## Out-of-scope follow-ups

These need a model rewrite or a new top-level kind, so they are not
in the bundles above:

- **`ltm profile`** — per-type fields (HTTP `idle-timeout`,
  client-ssl `ciphers` / `cert` / `chain` / `key`, …) belong on
  per-type kinds; the current single-container projection collapses
  every profile sub-type into one container. Consider splitting into
  `ltm profile http`, `ltm profile client-ssl`, etc.
- **`ltm monitor`** — same shape: adaptive / args / send-recv-
  per-protocol fields belong on per-type kinds.
- **`sys snmp`** — the SNMP stanza is essentially several sub-blocks
  (`communities`, `users`, `traps`, `process-monitors`,
  `disk-monitors`, plus `sys-contact` / `sys-location`); needs a
  structural pass.
- **`sys ntp` restrict** — per-network ACL sub-block.
- **`cm ha-group`** — new top-level kind; would unblock
  `cm traffic-group.ha-group` as a `PathRef`.
- **`ltm rate-class`** — new top-level kind; would unblock
  `ltm virtual.rate-class` as a `PathRef`.
- **`gtm pool` members** — the per-member sub-objects (with their
  own `monitor`, `member-order`, `service-port`) are not modelled.
- **`cm cert` / `sys file ssl-cert` audit fields** — `create-time`
  etc. are listed above; verify whether the parser receives them on
  a real save (TMSH may suppress them on `list` output).
