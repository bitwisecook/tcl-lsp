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

---

## Phase 2 — kinds the audit flagged as entirely unmodelled

The bundles above enrich kinds we already project. The kinds below
are **not yet projected at all** — they need a new dataclass +
parser + projection field map + dispatch, then a row in
`_KIND_FIELD_MAPS` / `_MODULE_KINDS`.

Each kind below has been verified to be a **persistent
configuration object** in the TMSH option-set (it advertises
`create` / `modify` / `delete`). Read-only / runtime / imperative
kinds are not in this list — they appear in the *Explicitly
skipped* section near the bottom.

Bundles are grouped by module and ordered by likely query value.
The number after the module name in each `## Bundle N — <module>`
header is the count of unticked kinds in the bundle. Some modules
have so many persistent kinds that they have been split across
multiple bundles.

### Bundle 7 — `ltm virtual-address` (1)

`ltm virtual-address` is a distinct kind from `ltm virtual`: it
represents the listener IP itself (route advertisement, ARP, ICMP
echo, connection-limit, traffic-group binding) and every `ltm
virtual.destination` references one.

- [x] `ltm virtual-address`

  Fields: address, mask, arp, icmp-echo, auto-delete,
  connection-limit, traffic-group (PathRef → `cm traffic-group`),
  inherited-traffic-group, route-advertisement, server-scope,
  spanning, unit, description, state (`enabled`/`disabled` bare
  flag), floating, traffic-group-restored.

### Bundle 8 — `auth.*` (14)

The `.auth` namespace is now projected. Six of the kinds are
singletons (`auth password`, `auth password-policy`, `auth source`,
`auth remote-role`, `auth remote-user`, `auth login-failures`) and
live under the empty-string key.

- [x] `auth partition` (central admin partition; referenced from
      virtually every full-path).  `default-route-domain` is a
      PathRef into `net route-domain` (id-keyed).
- [x] `auth user` (local users).  `partition` is a PathRef into
      `auth partition`; `partition-access` surfaces the keys of the
      sub-block.
- [x] `auth password`
- [x] `auth password-policy`
- [x] `auth source`
- [x] `auth remote-role`
- [x] `auth remote-user`
- [x] `auth login-failures`
- [x] `auth ldap`
- [x] `auth radius`.  `servers[]` is a PathRef list into
      `auth radius-server` for two-hop chains
      (`.auth.radius[].servers[].port`).
- [x] `auth radius-server`
- [x] `auth tacacs`.  `servers` is a plain string list (bare
      hostnames / IPs — not full-paths).
- [x] `auth cert-ldap`
- [x] `auth apm-auth`.  `profile` is a PathRef into
      `apm policy access-policy`.

### Bundle 9 — AFM `security firewall.*` core (13)

All 13 AFM firewall kinds are now projected.  The five enumerated
singletons (`global-rules`, `management-ip-rules`,
`global-fqdn-policy`, `on-demand-compilation`,
`on-demand-rule-deploy`, `uuid-default-autogenerate`,
`config-change-log`) live under the empty-string key via the
existing two-word singleton dispatch.

- [x] `security firewall policy`.  ``rules`` surfaces the top-level
      rule-binding keys; ``rule-lists`` is a PathRef list into
      `security firewall rule-list` so chains like
      `.security.firewall-policy[]."rule-lists"[].name` walk the
      bound rule-lists.
- [x] `security firewall address-list`.  ``addresses`` is the bare
      address / CIDR tokens; ``address-lists`` is a PathRef list
      into peer address-lists; ``fqdns`` is a plain string list.
- [x] `security firewall global-rules` (singleton).
      ``enforced-policy`` / ``staged-policy`` are PathRefs into
      `security firewall policy`.
- [x] `security firewall management-ip-rules` (singleton)
- [x] `security firewall schedule`
- [x] `security firewall user-list`
- [x] `security firewall user-domain`
- [x] `security firewall global-fqdn-policy` (singleton)
- [x] `security firewall port-misuse-policy`
- [x] `security firewall on-demand-compilation` (singleton)
- [x] `security firewall on-demand-rule-deploy` (singleton)
- [x] `security firewall uuid-default-autogenerate` (singleton)
- [x] `security firewall config-change-log` (singleton)

### Bundle 10 — `security` other (53)

This bundle was split into two sub-bundles for review-size reasons.
Bundle 10a covers the 14 highest-query-value kinds (NAT policies,
log / DoS / SSH / HTTP / bot-defense profiles, IP-intel feed-list +
global-policy, zones, packet-filter); bundle 10b covers the
remaining 39 (most of `dos.*`, `debug.*`, `datasync.*`, anti-fraud,
blacklist-publisher, protocol-inspection sub-kinds, etc.).

- [x] `security analytics settings`
- [x] `security anti-fraud profile`
- [x] `security anti-fraud signatures-update`
- [x] `security blacklist-publisher category`
- [x] `security blacklist-publisher profile`
- [x] `security bot-defense profile`
- [x] `security bot-defense signature`
- [x] `security bot-defense signature-category`
- [x] `security cloud-services connector`
- [x] `security datasync background-tasks`
- [x] `security datasync global-profile`
- [x] `security datasync local-profile`
- [x] `security debug drop-redirect-stats`
- [x] `security debug matcher`
- [x] `security debug register`
- [x] `security device device-context`
- [x] `security dos autodos-file-object`
- [x] `security dos behavioral-signature`
- [x] `security dos bot-signature`
- [x] `security dos bot-signature-category`
- [x] `security dos device-config`
- [x] `security dos dns-nxdomain-stat`
- [x] `security dos dos-signature`
- [x] `security dos dynamic-signatures`
- [x] `security dos ip-uncommon-protolist`
- [x] `security dos l4bdos-file-object`
- [x] `security dos network-whitelist`
- [x] `security dos profile`
- [x] `security dos stress-stats`
- [x] `security dos udp-portlist`
- [x] `security dos virtual`
- [x] `security flowspec-route-injector profile`
- [x] `security http profile`
- [x] `security ip-intelligence blacklist-category`
- [x] `security ip-intelligence feed-list`
- [x] `security ip-intelligence global-policy` (singleton)
- [x] `security log profile`
- [x] `security nat destination-translation`
- [x] `security nat policy` (sister to `security firewall policy`;
      `rule-lists` is a PathRef list into
      `security firewall rule-list`)
- [x] `security nat source-translation`
- [x] `security packet-filter default-rules` (singleton)
- [x] `security packet-filter policy`
- [x] `security protected zone`
- [x] `security protocol-inspection common-config`
- [x] `security protocol-inspection learning-stats`
- [x] `security protocol-inspection profile`
- [x] `security protocol-inspection signature`
- [x] `security scrubber profile`
- [x] `security ssh ciphers`
- [x] `security ssh profile`
- [x] `security zone` (single-word kind; `vlans` is a PathRef list
      into `net vlan`, `tunnels` into `net tunnels tunnel`)

Bundle 10b shares ``BigipSecurityMinimalObject`` and
``_SECURITY_MINIMAL_FIELDS`` for the 37 minimal kinds — every kind
has its own ``BigipConfig`` attribute and its own ``_KIND_FIELD_MAPS``
/ ``_MODULE_KINDS`` row so the dispatch routes correctly, but the
dataclass + projection field map are reused.  Surfaces ``name``,
``full-path``, ``kind``, and ``description`` only; richer fields
land in dedicated dataclasses if and when there's a query-shape
that needs them.

### Bundle 11 — `gtm monitor.*` (31)

Analogue of `ltm monitor` — all per-protocol monitor variants
collapse into a single `gtm monitor` container tagged by
`monitor_type`, mirroring the LTM pattern.  GTM monitors used to
land in `config.monitors` alongside LTM monitors (same path could
collide); the new `gtm_monitors` container fixes that.

- [x] `gtm monitor bigip`
- [x] `gtm monitor bigip-link`
- [x] `gtm monitor external`
- [x] `gtm monitor firepass`
- [x] `gtm monitor ftp`
- [x] `gtm monitor gateway-icmp`
- [x] `gtm monitor gtp`
- [x] `gtm monitor http`
- [x] `gtm monitor https`
- [x] `gtm monitor imap`
- [x] `gtm monitor ldap`
- [x] `gtm monitor mssql`
- [x] `gtm monitor mysql`
- [x] `gtm monitor nntp`
- [x] `gtm monitor oracle`
- [x] `gtm monitor pop3`
- [x] `gtm monitor postgresql`
- [x] `gtm monitor radius`
- [x] `gtm monitor radius-accounting`
- [x] `gtm monitor real-server`
- [x] `gtm monitor scripted`
- [x] `gtm monitor sip`
- [x] `gtm monitor smtp`
- [x] `gtm monitor snmp`
- [x] `gtm monitor snmp-link`
- [x] `gtm monitor soap`
- [x] `gtm monitor tcp`
- [x] `gtm monitor tcp-half-open`
- [x] `gtm monitor udp`
- [x] `gtm monitor wap`
- [x] `gtm monitor wmi`

### Bundle 12 — `gtm` listeners / topology / settings (10)

- [x] `gtm listener` (DNS listener — GTM equivalent of `ltm virtual`;
      ``pool`` is a PathRef into `gtm pool`)
- [x] `gtm listener-doh-proxy`
- [x] `gtm listener-doh-server`
- [x] `gtm link` (``datacenter`` PathRef → `gtm datacenter`,
      ``prober-pool`` PathRef → `gtm prober-pool`)
- [x] `gtm topology` (multi-token condition stored as the identifier;
      the parser pre-extracts ``block.header[len("gtm topology "):]``
      before the standard header dispatch)
- [x] `gtm distributed-app` (``wide-ips[]`` PathRef list → `gtm wideip`)
- [x] `gtm global-settings general` (singleton)
- [x] `gtm global-settings load-balancing` (singleton)
- [x] `gtm global-settings metrics` (singleton)
- [x] `gtm global-settings metrics-exclusions` (singleton)

### Bundle 13 — `ltm.*` cross-cutting infra (10)

- [x] `ltm virtual-address` (already done as bundle 7)
- [x] `ltm cipher group`.  ``allow`` / ``require`` / ``exclude`` are
      PathRef lists into ``ltm cipher rule`` so chains like
      ``.ltm.cipher-group[].allow[].cipher`` walk through.
- [x] `ltm cipher rule`
- [x] `ltm nat`
- [x] `ltm snat`.  ``snatpool`` is a PathRef into ``ltm snatpool``;
      ``origins`` is a string list of originating subnets.
- [x] `ltm snat-translation`
- [x] `ltm policy-strategy`.  ``operands`` surfaces the indexed
      sub-block keys; per-operand bodies are left to the source view.
- [x] `ltm traffic-class`
- [x] `ltm traffic-matching-criteria`.  ``destination-address-list``
      / ``source-address-list`` are PathRefs into
      ``security firewall address-list``;
      ``destination-port-list`` / ``source-port-list`` into
      ``security firewall port-list``; ``route-domain`` into
      ``net route-domain``.
- [x] `ltm ifile`
- [x] `ltm eviction-policy`

### Bundle 14 — `ltm dns.*` (DNS Express, 17)

Required the header parser to gain a fourth-word and a three-word-
singleton arity branch.  ``_THREE_WORD_TYPES`` now carries dns
kinds alongside the apm ``policy agent <type>`` family;
``_FOUR_WORD_TYPES`` is new and currently exclusive to the
``dns cache records *`` family.  Five sub-kinds of ``cache
records`` merge into one container keyed by full-path, with
``record-kind`` disambiguating them.

- [x] `ltm dns nameserver` (``tsig-key`` PathRef → `ltm dns tsig-key`)
- [x] `ltm dns tsig-key`
- [x] `ltm dns zone` (``dns-express-server`` PathRef → `ltm dns
      nameserver`; ``dns-express-allow-notify[]`` PathRef list)
- [x] `ltm dns dnssec key`
- [x] `ltm dns dnssec zone` (``keys[]`` PathRef list → `ltm dns
      dnssec key`)
- [x] `ltm dns cache resolver`
- [x] `ltm dns cache transparent`
- [x] `ltm dns cache validating-resolver`
- [x] `ltm dns cache global-settings` (singleton — 3-word kind,
      4-token header)
- [x] `ltm dns cache records all`
- [x] `ltm dns cache records key`
- [x] `ltm dns cache records msg`
- [x] `ltm dns cache records nameserver`
- [x] `ltm dns cache records rrset`
- [x] `ltm dns hpke key`
- [x] `ltm dns hpke profile` (``keys[]`` PathRef list → `ltm dns
      hpke key`)
- [x] `ltm dns analytics global-settings` (singleton)

### Bundle 15 — `ltm message-routing.*` (20)

Four protocol families (Diameter, SIP, MQTT, Generic).  All 20
kinds share ``BigipLtmMessageRoutingObject`` +
``_LTM_MESSAGE_ROUTING_FIELDS`` (minimal shape — name / full-path
/ kind / description); a ``_LTM_MESSAGE_ROUTING_DISPATCH`` table
routes each kind to its own ``BigipConfig`` attribute.  14 of the
20 are three-word kinds (5-token headers); the six ``... profile
router/session`` rows are four-word kinds (6-token headers,
parsed via the bundle-14 ``_FOUR_WORD_TYPES`` extension).

- [x] `ltm message-routing diameter peer`
- [x] `ltm message-routing diameter route`
- [x] `ltm message-routing diameter profile router`
- [x] `ltm message-routing diameter profile session`
- [x] `ltm message-routing diameter transport-config`
- [x] `ltm message-routing sip peer`
- [x] `ltm message-routing sip route`
- [x] `ltm message-routing sip profile router`
- [x] `ltm message-routing sip profile session`
- [x] `ltm message-routing sip transport-config`
- [x] `ltm message-routing mqtt peer`
- [x] `ltm message-routing mqtt route`
- [x] `ltm message-routing mqtt profile router`
- [x] `ltm message-routing mqtt profile session`
- [x] `ltm message-routing mqtt transport-config`
- [x] `ltm message-routing generic peer`
- [x] `ltm message-routing generic protocol`
- [x] `ltm message-routing generic route`
- [x] `ltm message-routing generic router`
- [x] `ltm message-routing generic transport-config`

### Bundle 16 — `ltm` auth profiles (11)

All 11 kinds share ``BigipLtmAuthObject`` + ``_LTM_AUTH_FIELDS``
(name / full-path / kind / description / defaults-from).  These
are the LTM-side auth profile objects, distinct from the
administrative ``auth.*`` namespace projected in bundle 8.

- [x] `ltm auth profile`
- [x] `ltm auth ldap`
- [x] `ltm auth radius`
- [x] `ltm auth radius-server`
- [x] `ltm auth tacacs`
- [x] `ltm auth crldp-server`
- [x] `ltm auth ocsp-responder`
- [x] `ltm auth kerberos-delegation`
- [x] `ltm auth ssl-cc-ldap`
- [x] `ltm auth ssl-crldp`
- [x] `ltm auth ssl-ocsp`

### Bundle 17 — `ltm` CGNAT / LSN (3)

- [x] `ltm lsn-pool`
- [x] `ltm lsn-log-profile`
- [x] `ltm alg-log-profile`

### Bundle 18 — `ltm` global-settings + misc singletons (6)

- [x] `ltm default-node-monitor`
- [x] `ltm global-settings connection`
- [x] `ltm global-settings general`
- [x] `ltm global-settings rule`
- [x] `ltm global-settings traffic-control`
- [x] `ltm rule-profiler`

### Bundle 19 — `ltm classification.*` (URL DB, 11)

- [x] `ltm classification application`
- [x] `ltm classification auto-update settings`
- [x] `ltm classification category`
- [x] `ltm classification ce`
- [x] `ltm classification signature-update-schedule`
- [x] `ltm classification url-cat-policy`
- [x] `ltm classification url-category`
- [x] `ltm classification urldb-feed-list`
- [x] `ltm classification urldb-file`
- [x] `ltm clientssl ocsp-stapling-responses`
- [x] `ltm clientssl-proxy cached-certs`

### Bundle 20 — `ltm tacdb.*` traffic-accel DB (3)

- [x] `ltm tacdb customdb`
- [x] `ltm tacdb customdb-file`
- [x] `ltm tacdb licenseddb`

### Bundle 21 — `net.*` routing (10)

- [x] `net routing access-list`
- [x] `net routing bfd`
- [x] `net routing bgp`
- [x] `net routing community-list`
- [x] `net routing extcommunity-list`
- [x] `net routing prefix-list`
- [x] `net routing profile bgp`
- [x] `net routing route-map`
- [x] `net routing debug`
- [x] `net router-advertisement`

### Bundle 22 — `net.*` tunnels family (14)

- [x] `net tunnels endpoint`
- [x] `net tunnels etherip`
- [x] `net tunnels fec`
- [x] `net tunnels geneve`
- [x] `net tunnels gre`
- [x] `net tunnels ipip`
- [x] `net tunnels ipsec`
- [x] `net tunnels lw4o6`
- [x] `net tunnels map`
- [x] `net tunnels ppp`
- [x] `net tunnels tcp-forward`
- [x] `net tunnels v6rd`
- [x] `net tunnels vxlan`
- [x] `net tunnels wccp`

### Bundle 23 — `net.*` IPsec (5)

- [x] `net ipsec ike-daemon`
- [x] `net ipsec ike-peer`
- [x] `net ipsec ipsec-policy`
- [x] `net ipsec manual-security-association`
- [x] `net ipsec traffic-selector`

### Bundle 24 — `net.*` BWC + rate-shaping + cos (12)

- [x] `net bwc policy`
- [x] `net bwc priority-group`
- [x] `net bwc traffic-group`
- [x] `net cos global-settings`
- [x] `net cos map-8021p`
- [x] `net cos map-dscp`
- [x] `net cos traffic-priority`
- [x] `net rate-shaping class`
- [x] `net rate-shaping color-policer`
- [x] `net rate-shaping drop-policy`
- [x] `net rate-shaping queue`
- [x] `net rate-shaping shaping-policy`

### Bundle 25 — `net.*` packet-level + L2 + misc (19)

- [x] `net address-list`
- [x] `net arp`
- [x] `net dag-globals`
- [x] `net fdb tunnel`
- [x] `net fdb vlan`
- [x] `net interface-cos`
- [x] `net ipv6-subscriber-prefix-length`
- [x] `net lacp-globals`
- [x] `net lldp-globals`
- [x] `net multicast-globals`
- [x] `net ndp`
- [x] `net packet-filter`
- [x] `net packet-filter-trusted`
- [x] `net port-mirror`
- [x] `net rst-cause`
- [x] `net self-allow`
- [x] `net service-policy`
- [x] `net stp-globals`
- [x] `net timer-policy`
- [x] `net trunk`
- [x] `net vlan-group`
- [x] `net wccp`

### Bundle 26 — `net.*` service-chain (3)

- [x] `net sfc chain`
- [x] `net sfc sf`

### Bundle 27 — `apm aaa.*` providers (24)

- [x] `apm aaa active-directory`
- [x] `apm aaa active-directory-trusted-domains`
- [x] `apm aaa crldp`
- [x] `apm aaa endpoint-management-system`
- [x] `apm aaa f5-mfa-configuration`
- [x] `apm aaa f5-service-connector`
- [x] `apm aaa http`
- [x] `apm aaa http-connector-request`
- [x] `apm aaa http-connector-transport`
- [x] `apm aaa kerberos`
- [x] `apm aaa kerberos-keytab-file`
- [x] `apm aaa ldap`
- [x] `apm aaa oam`
- [x] `apm aaa oauth-provider`
- [x] `apm aaa oauth-request`
- [x] `apm aaa oauth-server`
- [x] `apm aaa ocsp`
- [x] `apm aaa okta-connector`
- [x] `apm aaa radius`
- [x] `apm aaa saml`
- [x] `apm aaa saml-idp-automation`
- [x] `apm aaa saml-idp-connector`
- [x] `apm aaa securid`
- [x] `apm aaa tacacsplus`

### Bundle 28 — `apm profile.*` + `apm sso.*` (16)

- [x] `apm profile access`
- [x] `apm profile connectivity`
- [x] `apm profile exchange`
- [x] `apm profile oauth`
- [x] `apm profile vdi`
- [x] `apm sso basic`
- [x] `apm sso form-based`
- [x] `apm sso form-basedv2`
- [x] `apm sso kerberos`
- [x] `apm sso ntlmv1`
- [x] `apm sso ntlmv2`
- [x] `apm sso oauth-bearer`
- [x] `apm sso saml`
- [x] `apm sso saml-resource`
- [x] `apm sso saml-sp-automation`
- [x] `apm sso saml-sp-connector`

### Bundle 29 — `apm resource.*` + remote-desktop (16)

- [x] `apm resource address-space`
- [x] `apm resource app-tunnel`
- [x] `apm resource client-rate-class`
- [x] `apm resource client-traffic-classifier`
- [x] `apm resource ipv6-leasepool`
- [x] `apm resource leasepool`
- [x] `apm resource network-access`
- [x] `apm resource portal-access`
- [x] `apm resource remote-desktop citrix`
- [x] `apm resource remote-desktop citrix-client-bundle`
- [x] `apm resource remote-desktop citrix-client-package-file`
- [x] `apm resource remote-desktop quest`
- [x] `apm resource remote-desktop rdp`
- [x] `apm resource remote-desktop vmware-view`
- [x] `apm resource sandbox`
- [x] `apm resource webtop`
- [x] `apm resource webtop-link`

### Bundle 30 — `apm oauth.*` (7)

- [x] `apm oauth jwk-config`
- [x] `apm oauth jwt-config`
- [x] `apm oauth jwt-provider-list`
- [x] `apm oauth oauth-claim`
- [x] `apm oauth oauth-client-app`
- [x] `apm oauth oauth-resource-server`
- [x] `apm oauth oauth-scope`

### Bundle 31 — `apm saml.*` + NTLM + ACL + others (15)

- [x] `apm saml artifact-resolution-service`
- [x] `apm saml attribute-consuming-service`
- [x] `apm saml auth-context-class-list`
- [x] `apm ntlm machine-account`
- [x] `apm ntlm ntlm-auth`
- [x] `apm acl`
- [x] `apm log-setting`
- [x] `apm url-filter`
- [x] `apm swg-scheme`
- [x] `apm client image`
- [x] `apm configuration captcha`
- [x] `apm epsec epsec-package`
- [x] `apm apm-avr-config`
- [x] `apm report custom-report-field`
- [x] `apm policy customization-group`
- [x] `apm policy customization-languages`
- [x] `apm policy image-file`
- [x] `apm policy windows-group-policy-file`

### Bundle 32 — `pem.*` globals + protocol (9)

- [x] `pem global-settings analytics`
- [x] `pem global-settings gx`
- [x] `pem global-settings hsl-flow`
- [x] `pem global-settings hsl-report`
- [x] `pem global-settings insert-content`
- [x] `pem global-settings policy`
- [x] `pem global-settings quota-mgmt`
- [x] `pem global-settings session-mgmt-attributes`
- [x] `pem global-settings subscriber-activity-log`
- [x] `pem protocol diameter-avp`
- [x] `pem protocol radius-avp`
- [x] `pem protocol profile gx`
- [x] `pem protocol profile radius`
- [x] `pem reporting format-script`
- [x] `pem subscriber`
- [x] `pem subscriber-attribute`

### Bundle 33 — `sys` core configuration kinds (17)

- [ ] `sys ha-group` (referenced from `cm traffic-group.ha-group`)
- [ ] `sys application service`
- [ ] `sys application template`
- [ ] `sys application apl-script`
- [ ] `sys application custom-stat`
- [ ] `sys autoscale-group`
- [ ] `sys db`
- [ ] `sys httpd`
- [ ] `sys sshd`
- [ ] `sys syslog`
- [ ] `sys outbound-smtp`
- [ ] `sys smtp-server`
- [ ] `sys feature-module`
- [ ] `sys console`
- [ ] `sys log-rotate`
- [ ] `sys ucs`
- [ ] `sys url-db download-schedule`
- [ ] `sys url-db url-category`

### Bundle 34 — `sys file.*` referenceable file objects (7)

- [ ] `sys file data-group`
- [ ] `sys file external-monitor`
- [ ] `sys file ifile`
- [ ] `sys file rewrite-rule`
- [ ] `sys file apache-ssl-cert`
- [ ] `sys file ssl-crl`
- [ ] `sys file lwtunneltbl`
- [ ] `sys file browser-capabilities-db`
- [ ] `sys file device-capabilities-db`

### Bundle 35 — `sys log-config.*` HSL pipeline (12)

- [ ] `sys log-config destination alertd`
- [ ] `sys log-config destination arcsight`
- [ ] `sys log-config destination ipfix`
- [ ] `sys log-config destination local-database`
- [ ] `sys log-config destination local-syslog`
- [ ] `sys log-config destination management-port`
- [ ] `sys log-config destination remote-high-speed-log`
- [ ] `sys log-config destination remote-syslog`
- [ ] `sys log-config destination splunk`
- [ ] `sys log-config filter`
- [ ] `sys log-config publisher`

### Bundle 36 — `sys daemon-log-settings.*` (7)

- [ ] `sys daemon-log-settings clusterd`
- [ ] `sys daemon-log-settings csyncd`
- [ ] `sys daemon-log-settings icr-eventd`
- [ ] `sys daemon-log-settings icrd`
- [ ] `sys daemon-log-settings lind`
- [ ] `sys daemon-log-settings mcpd`
- [ ] `sys daemon-log-settings tmm`

### Bundle 37 — `sys crypto.*` (12)

- [ ] `sys crypto cert`
- [ ] `sys crypto key`
- [ ] `sys crypto crl`
- [ ] `sys crypto csr`
- [ ] `sys crypto master-key`
- [ ] `sys crypto cert-order-manager`
- [ ] `sys crypto ca-bundle-manager`
- [ ] `sys crypto cert-validator crl`
- [ ] `sys crypto cert-validator ocsp`
- [ ] `sys crypto cert-validation-response ocsp`
- [ ] `sys crypto client`
- [ ] `sys crypto server`
- [ ] `sys crypto acceleration-strategy`
- [ ] `sys crypto fips key`
- [ ] `sys crypto fips external-hsm`

### Bundle 38 — `sys ipfix.*` + `sys icall.*` (8)

- [ ] `sys ipfix destination`
- [ ] `sys ipfix element`
- [ ] `sys ipfix irules`
- [ ] `sys icall handler periodic`
- [ ] `sys icall handler perpetual`
- [ ] `sys icall handler triggered`
- [ ] `sys icall script`
- [ ] `sys icall istats-trigger`

### Bundle 39 — `sys management*` + state-mirroring + sflow (10)

- [ ] `sys management-dhcp`
- [ ] `sys management-ip`
- [ ] `sys management-ovsdb`
- [ ] `sys management-proxy-config`
- [ ] `sys state-mirroring`
- [ ] `sys datastor`
- [ ] `sys sflow receiver`
- [ ] `sys sflow global-settings http`
- [ ] `sys sflow global-settings interface`
- [ ] `sys sflow global-settings system`
- [ ] `sys sflow global-settings vlan`

### Bundle 40 — `sys software*` (4)

- [ ] `sys software hotfix`
- [ ] `sys software image`
- [ ] `sys software signature`
- [ ] `sys software volume`

### Bundle 41 — `sys` runtime-adjacent config (8)

Includes the kinds whose man page says `create/modify` but that
many sites treat as set-once or platform-driven.

- [ ] `sys alert lcd`
- [ ] `sys aom`
- [ ] `sys appiq config`
- [ ] `sys cluster`
- [ ] `sys config`
- [ ] `sys default-config`
- [ ] `sys failover`
- [ ] `sys internal-proxy`
- [ ] `sys traffic`
- [ ] `sys tmm-traffic`
- [ ] `sys turboflex profile-config`
- [ ] `sys fpga firmware-config`

### Bundle 42 — `vcmp.*` (4)

- [ ] `vcmp guest`
- [ ] `vcmp traffic-profile`
- [ ] `vcmp virtual-disk`
- [ ] `vcmp virtual-disk-template`

### Bundle 43 — `cm.*` follow-ons (2)

- [ ] `cm ha-group` (would unblock `cm traffic-group.ha-group` as
      a PathRef)
- [ ] `cm config-sync` (singleton)

### Bundle 44 — `cli.*` (8)

Mostly per-user / per-session prefs. Low query value but small:

- [ ] `cli admin-partitions`
- [ ] `cli alias private`
- [ ] `cli alias shared`
- [ ] `cli global-settings`
- [ ] `cli preference`
- [ ] `cli script`
- [ ] `cli transaction`
- [ ] `cli version`

### Bundle 45 — `api-protection.*` (3)

- [ ] `api-protection profile apiprotection`
- [ ] `api-protection response`
- [ ] `api-protection server`

---

## Explicitly skipped (not modelled)

The following are deliberately left out of the bundles above. They
either have no persistent body, are runtime / read-only views,
have a body so opaque or stats-shaped that a typed projection is
not useful, or live in a deprecated module.

### Runtime / display-only kinds (no persistent body)

These advertise only `show` / `list` (no `create` / `modify`) and
hold computed or fast-changing state. They would only be useful
if the query DSL grows a "read runtime telemetry" mode, which is
out of scope.

- All `analytics.*` *report* and *scheduled-report* kinds — these
  are reporting templates over runtime stats.
- `apm access-info`, `apm epsec software-status`, `apm license`,
  `apm oauth purged-entries`, `apm oauth token-details`,
  `apm profile remote-desktop`, `apm session`, `apm swg-content-type`
- `cm failover-status`, `cm sha1-fingerprint`, `cm sync-status`
- `gtm iquery`, `gtm ldns`, `gtm monitor none`, `gtm path`,
  `gtm persist`, `gtm traffic`
- `ltm classification auto-update status`,
  `ltm classification signature-definition`,
  `ltm classification signature-version`, `ltm nat-stats`,
  `ltm tacdb query`, `ltm urlcat-cloud-cache`,
  `ltm urlcat-query`
- `net f5optics`, `net interface-ddm`, `net ipsec ike-sa`,
  `net ipsec ipsec-sa`, `net ipsec-stat`, `net lldp-neighbors`,
  `net mroute`, `net sfc hop`, `net tunnels fec-stat`,
  `net vlan-allowed`
- `security` runtime read-outs: `bot-defense anomaly` family,
  `dos auto-thresholds *`, `firewall fqdn-entity`,
  `firewall fqdn-info`, `firewall ipi-category-info`,
  `flowspec-route-injector flowspec-advertised-route-info`,
  `http file-type`, `http mandatory-header`,
  `ip-intelligence info`, `log *-storage-field`,
  `log remote-format`, `malicious-sources *`,
  `presentation tmui *`, `protocol-inspection auto-update *`,
  `protocol-inspection compliance*`,
  `protocol-inspection learning-suggestions`,
  `protocol-inspection profile-status`,
  `protocol-inspection service`,
  `protocol-inspection staging`,
  `protocol-inspection system`,
  `protocol-inspection updates`,
  `protocol-inspection virtual-servers`,
  `scrubber dwbl-scrubber-stat`,
  `protected-servers netflow-tmc-stat`,
  `scrubber dwbl-scrubber-category-stats`,
  `blacklist-publisher all-blacklist-publisher`,
  `blacklist-publisher blacklist-publisher-stats`,
  `blacklist-publisher by-addr`,
  `blacklist-publisher by-category`,
  `firewall container-stat`, `firewall context-stat`,
  `firewall current-state`, `firewall matching-rule`,
  `firewall rule-stat`, `packet-filter rule-stat`,
  `datasync device-stats`, `dos spva-stats`
- `sys air-filter-reset`, `sys availability`, `sys clock`,
  `sys config-diff`, `sys cpu`, `sys crypto encrypted-attributes`,
  `sys diags ihealth-request`, `sys diags ihealth-result`,
  `sys dynad status`, `sys fix-connection`, `sys fpga info`,
  `sys fpga turboflex-profile`, `sys ha-status`, `sys hardware`,
  `sys host-info`, `sys hypervisor-info`, `sys iapp-restricted-key`,
  `sys iapprestricted key`, `sys icall publisher`,
  `sys ip-address`, `sys iprep-status`, `sys license`, `sys log`,
  `sys mac-address`, `sys mcp-state`, `sys memory`,
  `sys performance *`, `sys proc-info`, `sys raid disk`,
  `sys ready`, `sys sflow data-source *`,
  `sys software block-device-hotfix`,
  `sys software block-device-image`, `sys software status`,
  `sys software update-status`, `sys tmm-info`,
  `sys turboflex features`, `sys turboflex profile all`,
  `sys turboflex profile feature`, `sys turboflex warning`,
  `sys url-db download-result`, `sys version`
- `vcmp global`, `vcmp health *`
- `wam roi-statistics`, `wom remote-route`
- `asm device-sync`, `asm http-method`, `asm predefined-policy`,
  `asm response-code`, `asm webapp-language`
- `cli history`

### Imperative / command-only kinds

These have only `run` (no persistent body). They model tmsh
commands rather than persistent objects, so a projection is
meaningless.

- `cm add-to-trust`, `cm remove-from-trust`,
  `cm watch-devicegroup-device`, `cm watch-sys-device`,
  `cm watch-trafficgroup-device`, `cm sniff-updates`
- `gtm add`
- `ltm classification update-signatures`,
  `ltm classification updates`
- `security anti-fraud engine-update`,
  `security cloud-services cmd`, `security scrubber unredirect`
- `wom diagnose-conn`, `wom verify-config`
- The runtime / one-shot helpers that the man pages list as
  `run` + `show`: `pem sessiondb`, `pem subscribers`,
  `net packet-tester security`

### Deprecated modules

Entirely deprecated in current releases — no enrichment planned:

- `wam.*` (Web Acceleration Manager)
- `wom.*` (WAN Optimisation Manager)

### Opaque / not query-friendly

- `asm.*` (Application Security Manager) — configuration is held
  in opaque nested XML / JSON policy bodies that don't fit the
  scalar-property model.

### To assess separately

- `api-protection.*` — only three kinds, but the value depends on
  whether this module sees real use in the configs we care about.
- `analytics.*` *scheduled-report* — these are real persistent
  config objects (`create` / `modify`) but they wrap stats
  reporting. Skip unless someone needs to audit who has
  configured which scheduled report.
