# BIG-IP 21.1 address grammar validation

Measured on 23 November 2026 on the BIG-IP VE lab guest on `tank`, BIG-IP
21.1.0.1 build 0.0.26 with the FIPS module. LTM, APM and ASM were provisioned.
All rows used `tmsh -q load sys config merge file FIXTURE verify`, one fixture
per row. Exit 0 means the fixture verified. These rows measure loader and
compiler acceptance. [Lab and earlier fixtures](tmsh-21.1-name-handling.md)

## Destinations

The LTM virtual used `ip-protocol tcp` and `profiles { tcp { } }`; one
fixture also defined a virtual address named `alias` with
`address 192.0.2.54`. The HTTP monitor inherited `/Common/http`.

| Property | Accepted | Refused |
| --- | --- | --- |
| LTM virtual `destination` | `2001:db8::53.80`, `2001:db8::53%0.80`, `2001:db8::53.http`, `any6.any`, `any:any`, `192.0.2.53:http`, `192.0.2.53:0`, `/Common/alias:80`, `alias:80`, `[2001:db8::53]:80`, `any6:80` | `2001:db8::53%0:80`, `2001:db8::53:80`, `any`, `any6`: missing port or service name |
| LTM HTTP monitor `destination` | `*:*`, `*.*`, `192.0.2.53:http`, `2001:db8::53%0.80`, `2001:db8::53%0:80` | `2001:db8::53.*`: address type requires a port |

The monitor accepted the IPv6 colon service form that the LTM virtual and
[pool member tests](tmsh-21.1-name-handling.md#names-route-domains-and-services)
refused. TMSH listing and canonical spelling remain to be measured.

The follow-up fixtures verified a pool member `example.com:443` with an
`fqdn { name example.com }` block and a named member with an explicit
`address`. A named member with neither field failed because the node was
absent. A virtual with a missing named virtual address failed for the same
reason. The virtual destination `*.*` failed with a missing-service diagnostic.
A read-only lookup confirmed `/Common/http` exists as a shipped HTTP monitor.

## Route domains and address records

| Object | Fixture property | Exit | Result |
| --- | --- | --- | --- |
| `net self` on a fixture VLAN | `address 192.0.2.53%0/24`, `address 2001:db8::53%0/64` | 0 each | Both address families accept an explicit route domain before the prefix |
| Blackhole `net route` | `network default%0`, `network 192.0.2.0%0/24`, `network 2001:db8::%0/64` | 0 each | Default and explicit networks accepted |
| Blackhole `net route` | `network default%65432` | 1 | Domain 65432 does not exist for `0.0.0.0%65432` |
| Internal `type ip` data group | Record keys `192.0.2.53%0`, `192.0.2.0%0/24`, `2001:db8::%0/64`, `192.0.2.53%65432`, each with an empty value block | 0 each | The loader also accepted a record whose domain was absent from this fixture and the appliance |
| `net route-domain` and `ltm node` | `id 65534` and `address 192.0.2.53%65534` | 0 | Both verified together |
| `net route-domain` alone | `id 65535` | 1 | ID must be between 1 and 65534 |
| `net route-domain` and `ltm node` | `id 65536` and `address 192.0.2.53%65536` | 1 | ID out of range |

A separate fixture defined route domain 3999 as a partition's default and
verified a literal-address node with `%0` on both its name and address, on
either field alone, and on neither field. These checks establish loader
acceptance. Effective address identity still needs a listing or a TMM flow.

## Object names and FQDNs

| Fixture | Exit | Result |
| --- | --- | --- |
| Explicit nodes named `/Common/192.0.2.53%3999` and `/Common/2001:db8::53` with matching addresses and a definition for domain 3999 | 0 | Literal-address leaves accepted |
| Nodes named `/Common/10.0.0.1%0` or `/Common/2001:db8::1` with address `192.0.2.53` | 1 each | The name encodes an address that differs from the address field |
| Node leaves `has:colon`, `has%percent`, `has@at`, `has space`, `café`, `has~tilde` | 1 each | Invalid object name |
| Node paths `/Common/trail/` and `/Common/example//slash`, with an existing fixture folder `/Common/example` in the latter case | 0 each | Both verified. Without the folder, the missing-folder diagnostic displayed `/Common/example/slash` |
| Node path with surrounding spaces inside quotes | 1 | Loader treated the leading space as part of the name and reported a missing folder |
| Two partitions differing only in the case of their first letter, each with its own node | 0 | Coexist in the same verification |
| `sys folder /Common/example.app` with a child node | 0 | Textual folder and child verified |
| `sys folder /Common/3example` with a child node | 1 | Invalid folder name |
| Node `fqdn { name xn--caf-dma.example }` | 0 | ASCII-encoded internationalised name accepted |
| Node FQDNs `example.com.`, `single`, `café.example`, `*.example.com` | 1 each | Invalid FQDN |

## iRules compiler

Each body was the only command in `when CLIENT_ACCEPTED` in a standalone
`ltm rule`. Every case verified with exit 0 and no warning.

| Command | Operands |
| --- | --- |
| `IP::addr` | `192.0.2.53%0 equals 192.0.2.0/24` |
| `IP::addr` | `192.0.2.53 equals 192.0.2.0/255.255.255.0` |
| `IP::addr` | `2001:db8::53%0 equals 2001:db8::/64` |
| `node` | `192.0.2.53:80`, `192.0.2.53%0 80`, `2001:db8::53%0 80` |

An equivalent runtime verdict still needs an attached iRule and a TMM flow.
AFM and GTM were outside the provisioned module set. The only populated boot
slot contained 21.1.0.1, so 17.5, 17.1 and the legacy releases remain open.
