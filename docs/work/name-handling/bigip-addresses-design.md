# BIG-IP object names and addresses: one rule

**Question** (plan Q8): one rule for BIG-IP object names (`/Partition/folder/name`, iApp folders, relative names, REST `~Common~name`, `(no partition)`) and for addresses (IPv4 and IPv6, route domains `%N`, service suffixes, masks and prefixes, wildcards, FQDNs) in every context that carries them: tmsh configuration, iControl REST, iRules strings and URL/Host text. Which crate owns it, and which consumers move to it?

**Method.** 713 input strings ([`inputs.tsv`](handover/oo-dispatch-probe/bigip-addresses/inputs.tsv), made by `gen_inputs.py`): 183 hand-written incantations covering every form above, plus 530 generated combinations of eight hosts, three route-domain suffixes, six port suffixes (including the wrong separator for the family) and four path prefixes. Temporary `#[ignore]` probes (copies in [`probes/`](handover/oo-dispatch-probe/bigip-addresses/probes/); all removed from the tree after the run) passed every string through 44 functions:

- `tcl-bigip`: `value::{IPAddress, parse_address, FQDN, Destination, Network, Port, RouteDomain, Partition, Folder, ObjectPath, IPRange, PortSet}`, the `object_names` helpers, `pcap_enrich::{split_destination, resolve_port}`, and through in-crate tests `convert::split_destination` and `redact::collect_addresses`;
- `tcl-bigip-query`: the `typed_values` copy and the `host`, `port`, `route_domain`, `net`, `ip`, `with_route_domain` and `with_port` builtins, plus `architecture::bare_ip`;
- verbatim copies of private helpers with no test module to host a probe: `bigip-report-gen` `graph::{split_rd, mask_to_prefix, is_any, normalise_cidr}`, `enrich::to_ip` and `model::split_dest`; `f5-cli` `explain_flow::parse_destination` with its regex; the report front end's `parseIpQuery`, `toNet` and `IP_TOKEN` (run under Node);
- `f5-cli` `remote::object_io::encode_path` (in-crate) and `f5_xc::names::xc_object_name`.

Results are in `matrix-*.tsv`. `oracle-comparison.tsv` sets the oracle rows below against every splitter, using the analogue inputs. `summary-endpoints.tsv` (from `summarize.py`) puts the eleven destination and member splitters side by side, one row per input. A configuration fixture, [`addresses.conf`](handover/oo-dispatch-probe/bigip-addresses/addresses.conf), has one object per form. It went through `parse_bigip_conf`, `run_lint`, `validate_bigip_source`, the object graph, `compute_stats`, the pcap name index, the Wireshark profile and `scf_to_as3` (`config-tcl-bigip.txt`), and through the report generator's `collect_model` (`config-report-gen.json`). Thirty-eight iRules snippets went through `Analyser::analyse(…, "f5-irules")` (`irules-snippets.tsv`, `analyser-irules.tsv`). The code is at `b3f2f32c`.

**Oracle.** BIG-IP VE 21.1.0.1 build 0.0.26, `tmsh load sys config merge … verify` ([TMSH name and address checks](../../../evidence/bigip/tmsh-21.1-name-handling.md#names-route-domains-and-services), decided in [Q8](evidence/bigip-language-and-release.md#q8-partition-paths-addresses-and-transport-encoding)):

| Form (member key unless stated) | 21.1.0.1 |
| --- | --- |
| `/P/f/v4:80`, `/P/f/v6:443` (named nodes, IPv4- and IPv6-backed) | accepted: colon service after a node name |
| `/P/f/v6.443` | rejected: missing port or service |
| `2001:db8::54%0.443` (IPv6 literal) | accepted: dot service, route domain `%0` |
| `2001:db8::54%0:443` | rejected: TMSH asks for `.<port>` |
| node `address 2001:db8::53%65432` | rejected because route domain 65432 does not exist, so a route-domain ID is checked against configuration, not by range |
| `ltm pool ~Common~name` | rejected: the tilde form is a REST resource identifier, not a tmsh name |

One real saved configuration is in the repository: `rust/bigip-report-gen/python/tests/data/device-01.bigip.conf`, `#TMSH-VERSION: 15.1.8.2`. It shows numeric services (`/Common/app1_Node1:80`, `/Common/0.0.0.0:0`), `address any`, `mask any`, monitor `destination *:*` and `source 0.0.0.0/0`. It has no IPv6, route domain or service name. Forms that are neither in the 21.1 run nor in that sample are listed under *Unmeasured* at the end. They are not claimed.

**Verdict.** No single owner reads names or addresses today. `tcl-bigip::value` has typed `IPAddress`, `Destination`, `Network`, `Port` and `RouteDomain`, but they are reached from four places in the model. The rest of the estate has at least **11 independent destination and member splitters**, **13 route-domain strippers** and **two full copies of the value layer** (`tcl-bigip::value` and `tcl-bigip-query::builtins::net::typed_values`). Each guesses the grammar from the string, and they disagree on most inputs that carry a route domain or an IPv6 address. On 572 of the 713 inputs, at least two splitters return different addresses.

Measured against the 21.1 oracle:

- Every pool member parsed with an address gets port 0. A test pins this.
- Every IPv6 pool member loses its edge to its node.
- Route domains are dropped from self-IPs, route networks, the pcap index, the flow explainer and the data-group check.
- `with_route_domain` writes an IPv6 destination with a colon port, which reads back as a different address.
- None of our readers rejects either form the appliance rejected.

The registry already declares an address shape on 244 properties (188 `IpAddress`, 56 `Endpoint`), but nothing reads it. The rule this page proposes is: **the property's value grammar, from the registry, selects the reader; the string never does.** It follows the decision recorded for Q8: `tcl-bigip::object_names` owns paths, `tcl-bigip::value` owns typed addresses and services, and REST tilde encoding stays at the iControl boundary.

## Measured

Wrong answers first. "Oracle" is the 21.1 run; an input differing only in digits from an oracle form is called its *analogue* (for example `2001:db8::1%0.80` for `2001:db8::54%0.443`).

### Against the appliance

| Input (analogue of) | Oracle | Our readers |
| --- | --- | --- |
| `/Common/node1:80` (`/P/f/v4:80`) | accept: node `node1`, service 80 | `value::Destination` and the query copy **reject** it (they read the leaf as an FQDN without a dot); the query `split_destination`, AS3 and report splitters return `node1`, 80; pcap returns empty; `reference_without_numeric_service` gives `/Common/node1` (right) |
| `/Common/node1.80` (`/P/f/v6.443`) | reject | nothing rejects it. Both `Destination`s accept it as FQDN `node1.80` with no port, and `reference_without_numeric_service` treats it as a node named `node1.80` |
| `2001:db8::1%0.80` (`2001:db8::54%0.443`) | accept: v6, RD 0, service 80 | both `Destination`s are right. Elsewhere: pcap empty; AS3 `2001:db8::1%0.80` with no port; report `split_dest` port `1.80` on the model path (below); `explain_flow` and the front end reject; query `split_destination` drops `%0` ([note](#rd0)); `reference_without_numeric_service` leaves `.80` on, so no node edge |
| `2001:db8::1%0:80` (`2001:db8::54%0:443`) | **reject**: TMSH asks for `.<port>` | nothing rejects it. Four readings: both `Destination`s give address `2001:db8::1:80` with route domain 0 (they cut `%0` out of the middle and join the rest), with no port; AS3, `explain_flow`, `bare_ip` and `to_ip` give `2001:db8::1` with port 80 (colon port); the front end gives `2001:db8::1:80` with RD 0; report `split_dest` keeps the whole string |
| node address `…%65432` | reject: RD does not exist | `IPAddress` and the query copy accept any `u32` or `u64` RD (`%65535`). Nothing checks against `net route-domain` objects. `IPAddress::parse("10.0.0.1%99999999999")` returns `10.0.0.1` with **no** route domain (an overflowing ID is silently discarded); `Destination` reports it as out of range |

### Configuration fixture (`config-tcl-bigip.txt`, `config-report-gen.json`)

| Case | Result |
| --- | --- |
| every `ltm pool` member with an `address` (all IPv4 and IPv6 members) | **`port = 0`**. `parse_pool_members` reads the port from the name only when `address` is empty (`parser/bespoke.rs:604`), and `parser/bespoke/tests.rs:77` pins "address present -> port stays 0". f5-query's projection (`projection/ltm.rs:196`) and the iRule context JSON (`irule_context.rs:655`) publish it. AS3 conversion re-splits the name and so gets 80 for IPv4, 80 for `2001:db8::1.80`, but **0** for `2001:db8::1%2.443` |
| IPv6 pool members `/Common/2001:db8::1.80`, `…%2.443` | **no graph edge pool→node**; validator BIGIP6013 "could not be resolved" (false); report model `port` `1.80` and `1%2.443` (`model.rs split_dest` splits on the last `:` because the path's first segment has no colon); report node `/Common/2001:db8::1%2` listed as an orphan (false) |
| member `/Common/10.0.0.1:http` | BIGIP6013 (false), no edge to node `/Common/10.0.0.1`. Report port `http`; AS3 resolves it to 80 |
| `net self address 10.0.0.2%2/24`, `net route network 172.16.0.0%2/12`, `network default%2` | **route domain dropped**: `Network` has no RD field and its `Display` prints the stripped text (`10.0.0.2/24`, `default`). `gw 10.0.0.254%2` keeps it (`IPAddress`). The pcap index labels `10.0.0.2` (RD 0) as the RD-2 self-IP |
| `ltm virtual destination /Common/10.0.0.10%2:443` | RD kept, but on `Destination.route_domain` with `address.route_domain = None`: two places for one fact (`IPAddress` alone keeps it on the address) |
| `destination /Common/any6.any` | parsed as **FQDN `any6.any`**; report listener family IPv4, prefix 32, `anyAddr` false; AS3 `virtualAddresses: ["any6.any"]` |
| `destination /Common/10.0.0.11:http` | `Destination` rejects `http` → `destination = None`; report listener `anyAddr: true`, port any (a specific virtual becomes a catch-all); AS3 `virtualAddresses: []`; BIGIP6013 |
| `destination /Common/2001:db8::10%2.80` | AS3 `virtualAddresses: ["2001:db8::10%2.80"]`, no port (`convert::split_destination` only takes `.port` when the host parses without `%`) |
| `source 0.0.0.0%2/0` | report `sourcePrefix: 32` (`normalise_cidr` cuts at `%`, losing `/0`) |
| `virtual-address address any` / `any6` | `address = None` (both are documented wildcard spellings; `any` is in the 15.1 sample) |
| virtual→virtual-address edge | built for IPv4 only; `vs_v6 → /Common/2001:db8::10` and `vs_any6 → /Common/any6` missing; BIGIP6013 on every IPv6 destination |
| `explain_flow` virtual matching | a route-domain-2 virtual matches route-domain-0 flows (regex discards `%\d+`); **no tmsh IPv6 destination ever matches** (the regex accepts only `[v6]:port` or colon-port, and `Destination` renders `.port`) |
| data group `type ip` record `10.0.0.0%2/8` | BIGIP6011 "Invalid IP address" (validator splits at `/` and parses with `std`) |
| pcap name index, Wireshark profile | every `%N` object (virtual, node, member) absent (`split_destination` returns empty) |
| stats `partitionCounts` | `(no partition)` comes from `object_path_partition` returning `None`; it is correct for the cases measured |

### Readers against each other (`summary-endpoints.tsv`, matrices)

- **IPv6 port separator.** Twelve readers handle it in five different ways:
  - Both `Destination`s take a dot port when the host has two or more colons, and a colon port inside brackets.
  - pcap tries a colon first, then a dot (`2001:db8::1:80` gives `2001:db8::1`).
  - AS3 tries a dot, then a colon.
  - `explain_flow` and `bare_ip` read a colon port for IPv6.
  - The front end reads no IPv6 port at all and rejects `2001:db8::1.80`.
  - `report.split_dest` works on the leaf after the last `/` and counts colons. `model.rs` instead takes the member name's last `:` (`1.80`).
  - Only the two `Destination`s accept `[v6]:port`, and only `explain_flow` requires it.
- **Writers.**
  - f5-query `with_route_domain` rebuilds every destination with `:` (`rebuild_destination`): `2001:db8::1.80` becomes `2001:db8::1%5:80`, and `[2001:db8::1]:80` becomes `2001:db8::1%5:80`. Both read back as address `2001:db8::1:80`.
  - On a non-destination it appends: `10.0.0.1:http%5`, `10.0.0.0%2/8%5`.
  - `with_port` keeps the separator.
  - `Destination::Display` drops `%0` and moves a spliced RD to the end (`2001:db8::1%2:80` displays `2001:db8::1:80%2`).
- **Route domain.**
  - `IPAddress` splits at the last `%`, `Destination` at the first. `%a` and `%-1` leave the text to the IP parser.
  - `Network` strips the RD and loses it.
  - The query copy uses `u64`, `tcl-bigip` uses `u32`, and the front end uses `parseInt`.
  - `%0` is kept as `Some(0)` by `IPAddress` but rendered away by `Destination` and the query `split_destination` (<a id="rd0"></a>`%0` is meaningful in a partition whose default route domain is not 0, so dropping it changes the address. That this holds on the appliance is unmeasured).
- **Mapped IPv4.**
  - `tcl-bigip` `Destination` rejects `::ffff:10.0.0.1` with no port, because it splits at the last `.`. The query copy has a fallback and accepts it.
  - `redact::collect_addresses` finds `::ffff:10` in `::ffff:10.0.0.1`, so the mapped address is redacted wrongly.
  - The front end `v6ToBig` returns null for embedded IPv4, and `inNet` treats null as "matches everything".
- **FQDN.**
  - `tcl-bigip` accepts Unicode labels (`bücher.example`). The query copy is ASCII-only.
  - Both reject a trailing dot and a single label (`localhost`).
  - Both accept `any6.any` and `node1.80`.
- **Wildcards.**
  - `0.0.0.0:any`, `::.0` and `::.any` parse.
  - `any:any` (the registry's default for `ltm virtual destination`), `*:*` and `*.*` (monitor destinations; `*:*` is in the 15.1 sample), `any6` and `any6.any` are rejected or misread by every typed reader.
  - The report treats `any`, `0.0.0.0`, `::`, `0.0.0.0/0` and `::/0` as any address, but not `any6`.
- **Service names.**
  - Three name→port tables: `tcl-bigip/src/model/port_names/`, `bigip-report-gen/rust/src/services.rs`, and `pcap_enrich::resolve_port` reusing the first.
  - `value::Port` and both `Destination`s accept only numbers, `any`, `*` and `0`.
  - pcap and AS3 resolve `http`; the report keeps it as text.
- **Names.**
  - Both `ObjectPath`s reject names BIG-IP creates for itself: nodes `/Common/10.0.0.1%2` and `/Common/2001:db8::1`, the snat-translation `/Common/10.0.0.50%2`. They also reject `:`, `%`, `@`, space and non-ASCII in a leaf.
  - They silently normalise `/Common//name`, `/Common/name/` and `/Common/name ` to `/Common/name`, and accept `/common/name` as a different partition.
  - `Destination`'s folder split stops at the first segment that starts with a digit or contains `:`, so a folder named `2019` is read as a host.
  - `encode_path` maps `/` to `~` but leaves `%`, space and `:` raw in the URL (`~Common~10.0.0.1%2`, `~Common~my name`), so a route-domain node cannot be fetched.
  - `xc_object_name` folds names into DNS-1035 labels with a hash suffix, so distinct names stay distinct.
- **Pool member to node.**
  - `reference_without_numeric_service` strips the service only when the reference has exactly one colon and a numeric service. That handles `/P/n:80` and `/Common/10.0.0.1%2:80`, but not IPv6 (`.80`) or `:http`.
  - `irule_context` cuts at the last `:` (`/Common/2001:db8::1.80` gives `/Common/2001:db8:`).
  - `tcl-irule-test/tcl/scf_loader.tcl` splits the destination at the last `:` (by reading).

### iRules (`analyser-irules.tsv`)

See the table filled from the run below the census. Read from code: `IP::addr … equals …` is folded only for IPv4 without a route domain or dotted mask (`tcl-cmd-core/src/irules.rs:681`, everything else "unmodelled", which is right). The analyser's invalid-IP check (`analyser/diagnostics/dataflow/value_domains.rs:313–383`) reports any `hex(:hex){2,7}` run that `Ipv6Addr` rejects.

## Census

| Owner | What | Notes |
| --- | --- | --- |
| `tcl-bigip::value` (`address.rs`, `destination.rs`, `network.rs`, `port.rs`, `port_set.rs`, `route_domain.rs`, `ip_range.rs`, `partition.rs`, `folder.rs`, `ip_class.rs`) | the typed layer | reached by the model at 8 call sites (`parser/bespoke.rs:616–1480`); `Network` has no RD; `Port` has no service names |
| `tcl-bigip-query::builtins::net::typed_values` + `net.rs` | **a second full copy** (TypedIp, FQDN, TypedPort, Partition, Folder, ObjectPath, Destination, `legacy_dest_re`, `rebuild_destination`, network) | diverges on mapped IPv4, Unicode FQDN, RD width; f5-query re-parses `tcl-bigip`'s rendered destination with it |
| `tcl-bigip::object_names` | `object_path_partition`, 8 `ObjectNamePolicy` tiers, `reference_without_numeric_service`, `owner_partition_reference_paths` | the Q8 owner for paths; stats' `(no partition)` and query projection already use it |
| destination/member splitters outside `value` | `pcap_enrich::split_destination` (also `wireshark_profile`, GTM `addr_only`), `convert::split_destination` (AS3), `parser/bespoke.rs:604` member port, `irule_context.rs:597`, `conf_tokens.rs:561` (highlighting, IPv4 only), report `model.rs split_dest` and `model/shaping.rs`, report `graph.rs split_rd`/`parse_listener`, `f5-cli explain_flow` regex, query `architecture::bare_ip`, front end `parseIpQuery`, `tcl-irule-test` `scf_loader.tcl` | 11 independent grammars |
| route-domain strippers | `address.rs:61` (last `%`), `destination.rs:311` (first `%`), `network.rs:317`, query `typed_values.rs:49`/`:307`, `pcap_enrich.rs:487`, `wireshark_profile.rs:289`, report `graph.rs:59`/`:101`, `enrich.rs:47`, `architecture.rs:150`, front end `v6ToBig`/`ipToBig`/`parseIpQuery` | 13 |
| IP literal scanners/classifiers | `grep.rs` (ipnet), `redact.rs` (own regexes), `pcap_remap.rs`, `flow/tshark.rs`, `validator.rs:791` (BIGIP6011), `tcl-lsp-core refactor/datagroup.rs` and its copy `tcl-mcp/src/datagroup.rs`, `tcl-lsp-core hover.rs:860`, `tcl-compiler value_domains.rs`, `tcl-cmd-core irules.rs:681`, front end `IP_TOKEN` | none takes `%N`; three IP classifiers (`ip_class.rs`, hover, query `classification.rs`) |
| service names | `tcl-bigip/src/model/port_names/` (four files), report `services.rs`, query none | two tables |
| path helpers outside `object_names` | `/Common/` fallbacks in `explain_flow.rs:234`, `explain.rs:154`, report `model.rs:584`, `apm.rs:176`, `forensics.rs:704`, query `projection.rs:157`, `gantt.rs:169`, `validator.rs:950`, `lint.rs:310`, `convert::short_name`; `leaf()` ×3 in the report; front end `split("/").pop()` ×12 and `"/Common/" + target` (`topology.ts:1300`, iRule relative pool); driver `partition_prefix` ×2 with `NO_PARTITION_PREFIX` | listed in the Q8 decision for migration |
| REST | `f5-cli remote/object_io.rs encode_path` + `percent_encode` | the only tilde encoder; no decoder |
| registry | `BigipPropertySpec::shape_kind` `IpAddress` ×188, `Endpoint` ×56; `ltm virtual destination` is `Reference → ltm_virtual_address` default `any:any`; iRules `ip__client_addr.rs` documents `x.x.x.x%rd` returns | shape declared, **no consumer** (only `tests/it_5/registry_sweep/f5_sweeps.rs`) |

## Design

**Rule.** A value is read by the grammar the registry declares for its property, never by inspecting the string. Each property's `shape_kind` becomes a value grammar:

| Grammar | Fields | Example properties |
| --- | --- | --- |
| `Address` | host IP, RD | `ltm node address`, `virtual-address address`, route `gw`, snat-translation `address` |
| `AddressOrWildcard` | + `any`, `any6` | `virtual-address address` |
| `Network` | IP, RD, prefix or dotted mask, `default`, `default-inet6` | `net self address`, route `network`, `source`, data group `type ip` keys, AFM address lists |
| `Mask` | dotted, `any`, `any6`, `none` | `mask` |
| `Destination` | address or wildcard, RD, service | `ltm virtual destination` |
| `MemberKey` | node reference **or** address literal, plus service | `ltm pool members`, `snatpool members` (no service) |
| `MonitorDestination` | `*`, address, service | monitor `destination` |
| `GtmMemberKey` | server path `:` virtual-server name | `gtm pool members` |
| `Fqdn` | name, trailing dot kept as spelled | node `fqdn name` |
| `Service` | number, `any`, `*`, name | port fields |

A `MemberKey` reader knows that `/P/f/v6:443` is node `v6` with service 443 and that `2001:db8::54%0.443` is a literal with service 443. An `Address` reader takes `fe80::1:80` as one address. Neither guesses. A string with no declared grammar, such as a search box, a pcap token or an iRules literal of unknown role, goes to an explicit `Endpoint::read_any`. It returns every reading that the active release's separator rules allow (two for `fe80::1:80`: the address alone, or `fe80::1` with service 80 where colon service is in force), and callers must choose.

**Types** (in `tcl-bigip::value`, the owner the Q8 decision names):

- `BigIpName { partition: Option<Partition>, folders: Vec<Segment>, leaf: Segment, app: Option<usize> /* index of the `.app` folder */, spelling: Span }`. It is relative when `partition` is `None`. Unpartitioned kinds (the driver's `NO_PARTITION_PREFIX`) carry a `Scope::Unpartitioned` instead of a fake `/Common`. `object_names` keeps the lookup policies and is changed to take `BigIpName` instead of `&str`. It renders `(no partition)` from `partition: None`, as now.
- `Endpoint { host: Host, rd: Option<RouteDomainId>, service: Option<Service>, prefix: Option<Prefix>, spelling: Span }`, where:
  - `Host` is `V4 | V6 | Any | Any6 | Star | Fqdn(Fqdn) | Node(BigIpName)`;
  - `RouteDomainId(u16)` keeps an explicit `%0` as distinct from absent;
  - `Service` is `Number(u16) | Any | Named(&'static str, u16)`, with one table moved out of `model/port_names`;
  - `Prefix` is `Bits(u8) | Mask(dotted)`.

  `IPAddress`, `Destination`, `Network` and `Port` become constructors or views over it. `Network` gains its RD.
- **Readers and writers per context**, each taking `Release`:
  - **tmsh.** Saved configuration and CLI. The IPv4 service separator is `:`. For IPv6 it is `.` after a literal (21.1) and `:` after a node name (21.1). The legacy IPv6 colon service is user-reported for early releases (which release, and which fields, unmeasured).
  - **REST.** `~` in the URL path, `fullPath` and `partition` in the JSON. The encoder percent-encodes `%`, space and `:` in the components. A decoder is added for `selfLink`.
  - **iRules strings.** `IP::client_addr` returns `addr%rd` (registry text). `node`, `pool … member` and `snat` take the address and port as separate words. `LB::server pool` returns a path.
  - **URL and Host.** `[v6]:port`, a zone is not an RD, and `%25` in URLs.

  Writers round-trip the spelling unless a field was edited. `with_route_domain` and `with_port` go through the tmsh writer.
- **Validation layer**, separate from reading:
  - RD existence against the configuration's `net route-domain` objects, as the appliance does, not by range;
  - service range;
  - host bits;
  - separator legality per release, which is where the inverse forms the oracle rejected become diagnostics.

**Consumers.**

- The model stores `Endpoint` and `BigIpName` (pool member port from the key, fixing `port = 0`).
- Lint, the validator (BIGIP6011/6013), graph edges (member→node, virtual→virtual-address for every family and RD), stats, the pcap index, the Wireshark profile, AS3 and `explain_flow` match on `(host, rd, service)`.
- f5-query drops `typed_values` and projects `Endpoint` fields (`host`, `route_domain` and `port` become accessors).
- `bigip-report-gen` consumes the projected fields and stops re-splitting.
- The front end receives `family/address/rd/port` from the model and keeps only a query parser that calls `read_any` (via WASM, or a generated table).
- `f5-xc` takes `BigIpName` for names and `Endpoint` for origin pools.
- The iRules analyser reads iRules address arguments with the registry's argument roles and the iRules context reader. The invalid-IP check uses the same parser.

## Plan

1. **Pool member port.** Owner: `tcl-bigip` parser. Read the service from the member key with the `MemberKey` grammar, whether or not `address` is present; replace the pinning test with oracle forms. *Done when* every member in `addresses.conf` and in `device-01.bigip.conf` has its port, and AS3 no longer re-splits. (Quick fix; with plan 107.)
2. **One `Endpoint` and `BigIpName` in `tcl-bigip::value`.** Owner: `tcl-bigip`. RD on `Network`, `%0` kept distinct, wildcards `any`/`any6`/`*` typed, service names from one table, mapped IPv4 without a port. *Done when* the 713-input matrix has one row of answers per grammar and every oracle form gives the oracle's verdict (accepts parse, rejects are diagnostics). (Plan 111, 118.)
3. **Registry grammar drives readers.** Owner: `tcl-registry` plus `tcl-bigip`. Turn the 244 `shape_kind` declarations into the grammar table above (adding `MemberKey`, `MonitorDestination`, `GtmMemberKey` and `Mask`, which are missing today), and make the model's property parse dispatch on it. *Done when* no model field is parsed by a string-shape heuristic and a test fails if a property has an address-shaped value with no grammar. (Plan 118, Q7.)
4. **Delete the copies.** Owners: `tcl-bigip-query`, `bigip-report-gen`, `f5-cli`, front end. Remove `typed_values`, the report `split_dest`/`split_rd`/`to_ip`/`normalise_cidr`, the pcap and AS3 splitters, the `explain_flow` regex, `irule_context`'s `rsplit_once`, `bare_ip` and the front end's IPv4/IPv6 helpers. Fix `with_route_domain` through the tmsh writer. *Done when* `grep` for `split_once('%')`, `rsplit_once(':')` on addresses, and `parse::<IpAddr>` on configuration values returns only `tcl-bigip::value`. (Plan 111.)
5. **Graph and validator over typed values.** Owner: `tcl-bigip`. Member→node and virtual→virtual-address edges for IPv6, RD and service-name forms; the BIGIP6011 data-group check through `Network` with RD; auto-created virtual addresses not reported as unresolved. *Done when* `addresses.conf` gives no false BIGIP6013 or BIGIP6011 and has every expected edge. (Plan 111.)
6. **Paths.** Owner: `tcl-bigip::object_names`. `BigIpName` reader that keeps `%`, `:` and IPv6 leaves BIG-IP itself creates, refuses `//`, a trailing `/` and surrounding space instead of normalising them, and models `.app` folders. The REST encoder and decoder live at the iControl boundary and percent-encode. Migrate the path helpers listed in the census and in the Q8 decision. *Done when* the names group of the matrix has one answer per input and `encode_path("/Common/10.0.0.1%2")` is `~Common~10.0.0.1%252`. (Plan 111, Q4.)
7. **iRules.** Owner: `tcl-compiler` and the registry. Address arguments of `node`, `pool … member`, `snat`, `IP::addr` and `class match` read through the iRules reader with route domains. *Done when* the analyser rows below match the oracle or are recorded as unmodelled. (Q6.)
8. **Oracle coverage.** Owner: whoever runs the Q1 matrix. Measure the *Unmeasured* list on 21.1, 17.5, 17.1 and the legacy releases, and record each verdict in `evidence/bigip/`. (Q1, Q3.)

Overlaps: item 1 is a 107-style quick fix. Items 2 to 6 are the address and path half of 111 and the value-grammar half of 118. Positions are not affected (110). Q4 is answered for this slice by the Q8 decision. Q7 shares item 3's grammar table. Q10: items 1 and 5 are wrong answers suitable for the current release; the rest wait for the data crate.

## Unmeasured on a real BIG-IP

Measured: 21.1.0.1 for the six oracle rows above; 15.1.8.2 saved-config spellings for IPv4, `any`, `*:*` and numeric services. Not measured on any release:

- Which releases, and which fields, used colon service after an IPv6 literal. This is user-reported for early releases, version unknown. In 21.1 the member key rejects it. Virtual `destination`, monitor `destination`, GTM `virtual-server destination` and snat forms are unmeasured.
- Virtual `destination` forms: IPv6 `.port`, `any6.any`, `any:any`, service names (`:http`) in saved configuration versus listing, `%0` meaning in a partition with a non-zero default route domain, a virtual-address *name* in place of an address.
- Monitor `destination *.*` for IPv6 and alias forms. Self-IP `%N/len`, route `default%N`. Data group `type ip` records with `%N`, and AFM address lists with `%N` or ranges.
- Names: which characters a leaf may contain (`:`, `%`, `@`, space, non-ASCII), case-sensitivity of partitions, `.app` folder rules, a folder whose name starts with a digit, a trailing `/`.
- FQDN: trailing dot, single label, IDN, wildcard in node and pool member versus GTM wide IP.
- Service-name table per release.
- Route-domain ID upper bound when the domain exists.
- iRules: `IP::client_addr`/`LB::server addr` spelling with RD and IPv6, relative `pool p` resolution (rule's partition or virtual's), `IP::addr` with `%N`, dotted masks and IPv6, `node 10.0.0.1:80` single-word form.
- 17.1, 17.5, 16.1 and 15.1 for every row above, including the 21.1 rows.
