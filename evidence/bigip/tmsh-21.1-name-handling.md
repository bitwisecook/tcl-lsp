# BIG-IP 21.1 TMSH validation for name handling

Measured on 23 November 2026 on the BIG-IP VE lab guest on `tank`.
`tmsh show sys version` reported 21.1.0.1 build 0.0.26 with the FIPS module.
`tmsh list sys provision one-line` showed LTM, APM and ASM at nominal.
`tmsh list sys software volume one-line` showed this build in the only populated
boot slot. The tests below used `tmsh -q load sys config merge file "$fixture"
verify` with one case per file. Exit 0 means accepted by this loader. Warnings
and errors below are the pertinent output excerpts. The verification cases had
no configuration writes. Post-check lookups reported the test rule, pool,
virtual and partition absent.

## Questions this answers

These TMSH checks answer the BIG-IP and iRules questions in [the name-handling plan](../../docs/work/name-handling/plan.md#big-ip-and-irules-questions), with the reasons and full question text in [language and release](../../docs/work/name-handling/evidence/bigip-language-and-release.md) and [data provenance](../../docs/work/name-handling/evidence/bigip-data-provenance.md):

- **Shipped and effective defaults** (Q3): whether the registry's open-ended 17.1 SSL profile `options` override still holds on a later release.
- **Rule compiler** (Q5, Q6): which commands and forms the iRule compiler accepts, warns on or disables in which events, as evidence for per-event availability and for the iRules definition audit (the analysis container had no BIG-IP to play tclsh's role).
- **SSL rule attachment** (Q5): which profiles make the client- and server-side SSL events attachable, because `tcl-mcp`'s `irule_gen` adds a CLIENTSSL profile for SERVERSSL events and the registry lists PERSIST as an alternative.
- **Monitor rules** (Q7): the accepted LTM pool monitor grammar, because `orphan-monitor`, conversion and iRule context take only the first word of a monitor rule and the typed `MonitorExpression::try_parse` was untested against an appliance.
- **Names, route domains and services** (Q8): which path, route-domain and service-separator forms TMSH accepts, because path and address rules are implemented several times with different outputs, and IPv6 uses `.port` in tmsh but `[addr]:port` in URLs.

## Shipped and effective defaults

`rpm -qf /defaults/config_base.conf.virtual /defaults/profile_base.conf`
identified both files as `f5base-21.1.0.1-0.0.26.noarch`. The latter has
`ltm profile tcp tcp` and these default SSL profile fields:

```text
ltm profile client-ssl clientssl {
    options { dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl }
ltm profile server-ssl serverssl {
    options { dont-insert-empty-fragments no-tlsv1.1 no-tlsv1 no-ssl }
```

The output of `tmsh -q list ltm profile client-ssl clientssl options` and
`tmsh -q list ltm profile server-ssl serverssl options` showed the same four
tokens with `no-ssl` moved before `no-tlsv1.1`. This comparison concerns two
default profile fields. It did not enumerate the 21.1 base object inventory.

## Rule compiler

Each row replaces `EVENT` and `BODY` in this input.

```text
ltm rule /Common/nhq261123_rule {
when EVENT {
    BODY
}
}
```

| Event | Body | Exit | Message |
| --- | --- | --- | --- |
| `HTTP_REQUEST` | `set x [HTTP::header value Host]` | 0 | No warning |
| `RULE_INIT` | `set x [HTTP::header value Host]` | 0 | `command is not valid in RULE_INIT (RULE_INIT)` |
| `HTTP_REQUEST` | `set x [HTTP::header nhq_no_such_subcommand Host]` | 0 | No warning |
| `HTTP_REQUEST` | `set x [HTTP::header value Host Extra]` | 0 | No warning |
| `RULE_INIT` | `exec` | 1 | `command is disabled: "exec"` |
| `RULE_INIT` | `open` | 1 | `command is disabled: "open"` |
| `RULE_INIT` | `set x [clock seconds]` | 0 | No warning |
| `HTTP_REQUEST` | `set x [matchclass probe equals /Common/nhq_missing]` | 0 | No warning |
| `SERVERSSL_CLIENTHELLO` | `set x 1` | 1 | `unknown event (SERVERSSL_CLIENTHELLO)` |
| `SERVERSSL_CLIENTHELLO_SEND` | `set x 1` | 0 | No warning |

`tmsh -q help ltm rule` listed `help rule [ command | event | namespace ]
[name]` among its interfaces. Several directories were checked for
`fakeCMP` and no executable with that name was located. In this repository
`fakeCMP` is implemented by the iRule test harness and MCP.

## Offline checker cross-check

The live 21.1.0.1 build 0.0.26 appliance supplied the 64-bit
`libtclCheck_tcl.so.1.0` from its matching mcpd package. The transferred
bytes matched the file on the appliance. Decoding the exported script
payloads with XOR `0xF5` yielded 17 Tcl files.

The repository's [offline harness](../../scripts/dev/bigip-probes/open-questions/irule-oracle.tcl)
ran under stock Tcl 8.6.17 with appliance hooks stubbed. It produced
182,628 event/command checks across 171 events and 1,068 checker entries.
The `HTTP::header` result was valid in `HTTP_REQUEST` and invalid with a
warning in `RULE_INIT`, agreeing with the two live compiler checks above.
The matrix models bare-call constraints. It does not measure arity,
event delivery or invocation effects in TMM. The harness failed to
initialise under Tcl 9.0.3 with a namespace lookup error.

The vendor library, decoded source and generated tables are kept privately.
This receipt retains the build identity, method and observed comparison.
[Prior checker investigation](../../docs/design/f5/bigip-irule-parser-measurements.md#14-how-the-rule-loader-decides)

## SSL rule attachment

The rule body was `when EVENT { set x 1 }`, in an otherwise empty rule.
Each virtual had `destination 192.0.2.254:41903`, `ip-protocol tcp`,
`profiles { tcp { } ... }` and the one rule attachment. For a persistence
variant, it had only the TCP transport profile plus
`persist { /Common/source_addr { default yes } }` or
`persist { /Common/ssl { default yes } }`.

| Event | Extra profile or persistence | Exit | Message |
| --- | --- | --- | --- |
| `CLIENTSSL_CLIENTHELLO` | None | 1 | `requires an associated CLIENTSSL or PERSIST profile` |
| `CLIENTSSL_CLIENTHELLO` | `clientssl { context clientside }` | 0 | No warning |
| `SERVERSSL_HANDSHAKE` | `clientssl { context clientside }` | 1 | `requires an associated SERVERSSL or PERSIST profile` |
| `SERVERSSL_HANDSHAKE` | `serverssl { context serverside }` | 0 | No warning |
| `CLIENTSSL_CLIENTHELLO` | `/Common/source_addr`, then `/Common/ssl` | 0 each | No warning |
| `SERVERSSL_HANDSHAKE` | `/Common/source_addr`, then `/Common/ssl` | 0 each | No warning |

A second verify-only run used the same template with `/Common/source_addr`
for the persistence-only virtual. Each event also verified with its
corresponding SSL profile on the correct side.

| Event | Persistence only | Required profile when refused |
| --- | --- | --- |
| `CLIENTSSL_CLIENTHELLO` | Accepted | |
| `CLIENTSSL_DATA` | Rejected | CLIENTSSL |
| `CLIENTSSL_HANDSHAKE` | Rejected | CLIENTSSL |
| `CLIENTSSL_PASSTHROUGH` | Rejected | CLIENTSSL |
| `CLIENTSSL_SERVERHELLO_SEND` | Rejected | CLIENTSSL |
| `SERVERSSL_CLIENTHELLO_SEND` | Rejected | SERVERSSL |
| `SERVERSSL_DATA` | Rejected | SERVERSSL |
| `SERVERSSL_SERVERCERT` | Rejected | SERVERSSL |
| `SERVERSSL_SERVERHELLO` | Accepted | |
| `SERVERSSL_HANDSHAKE` | Accepted | |

The persistence-only cases measured attachment eligibility. The
[live TLS flow](../tcl/bigip-name-handling-tls-http.md) exercised actual event
delivery with the two SSL profiles attached.

## Monitor rules

Each expression was placed in `ltm pool /Common/nhq261123_pool { ... }`
or `ltm pool /Common/nhq261123_values { ... }` as a `monitor` field. The
appliance already had the factory `/Common/http` and `/Common/https`
monitors. Every accepted expression below was tested on its own.

| Expression | Exit | Result |
| --- | --- | --- |
| `/Common/http` | 0 | Single reference |
| `/Common/http and /Common/https` | 0 | Both references accepted |
| `"/Common/http" and "/Common/https"` | 0 | Quoted path tokens accepted |
| `"/Common/http and /Common/https"` | 0 | Whole quoted expression accepted |
| `min 1 of { /Common/http /Common/https }` | 0 | Minimum list accepted |
| `min 2 of { /Common/http /Common/https }` | 0 | Minimum equal to list length accepted |
| `min 1 of { /Common/http and /Common/https }` | 1 | `The monitor template and was not found.` |
| `/Common/http and` | 1 | `incomplete command` |
| `/Common/http or /Common/https` | 1 | `"or" unknown property` |
| `min 0 of { /Common/http /Common/https }` | 1 | `"0" invalid value, must be between 1 and 65535` |
| `min -1 of { /Common/http and /Common/https }` | 1 | `"-1" invalid value, must be between 1 and 65535` |
| `min 3 of { /Common/http /Common/https }` | 1 | `"min" must be less than or equal to the number of monitors` |
| `min 65536 of { /Common/http /Common/https }` | 1 | Same list-length error |
| `default` | 0 | Accepted |
| `none` | 0 | Accepted |

A pool with `monitor /Common/http` and a named member having
`monitor default` verified successfully. A separate attempt to create
`ltm monitor http "/Common/nhq261123 with space" { defaults-from /Common/http }`
and reference it from a pool failed because the object name was invalid.
Quoting accepted path tokens and expressing a legal object name with spaces
are different questions. The probes did not measure canonical rendering of
the quoted expression or GTM monitor grammars.

## Names, route domains and services

The following fixture was verified with the member block varied as shown.

```text
auth partition nhq261123 { }
sys folder /nhq261123/lab { }
ltm node /nhq261123/lab/v4 { address 192.0.2.53%0 }
ltm node /nhq261123/lab/v6 { address 2001:db8::53%0 }
ltm pool /nhq261123/lab/pool {
    monitor /Common/http and /Common/https
    members { MEMBERS }
}
```

| Members | Exit | Result |
| --- | --- | --- |
| `/nhq261123/lab/v4:80 { } /nhq261123/lab/v6:443 { }` | 0 | Qualified node names and colon service accepted for both address families |
| `/nhq261123/lab/v6.443 { }` | 1 | Missing port or service for the node name |
| `2001:db8::54%0.443 { address 2001:db8::54%0 }` | 0 | IPv6 literal, route domain and dot service accepted |
| `2001:db8::54%0:443 { address 2001:db8::54%0 }` | 1 | Missing port or service. TMSH requested `.<port>` |

Separate controls:

- `ltm node /Common/nhq261123_missing_rd { address 2001:db8::53%65432 }`
  exited 1 with `The specified route domain (65432) does not exist`.
- `ltm pool ~Common~nhq261123_pool { monitor /Common/http }` exited 1 with
  `The requested object name (/Common/~Common~nhq261123_pool) is invalid`.
  The iControl REST spelling is documented as a resource identifier.
  [F5 iControl REST resource identifiers](https://clouddocs.f5.com/training/community/firewall/html/archive/archive1/lab6/step5.html)
- A TCP virtual with `persist { /Common/source_addr { default yes } }` and
  `fallback-persistence /Common/dest_addr` exited 0.
- `when HTTP_REQUEST priority N { set x 1 }` verified with 0 and 1000.
  With -1 or 1001 it exited 1 with `unexpected extra argument`.
  [F5 priority reference](https://clouddocs.f5.com/api/irules/priority.html)
