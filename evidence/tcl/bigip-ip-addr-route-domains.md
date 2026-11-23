# BIG-IP IP::addr address operands in TMM

## Question

How does `IP::addr equals` compare an explicit route domain, an omitted
route domain, a dotted mask, and IPv6?

## Assumptions

The probe ran in `HTTP_REQUEST` on a temporary TCP/HTTP virtual in
`/Common` on the lab BIG-IP VE. The default route domain for that
partition was 0. The request supplied `Host: nhaddr.invalid` and went to
the temporary virtual directly. It exercised local address comparisons.

## Script

```tcl-visible
when HTTP_REQUEST {
    set lines {}
    foreach {label left right} {
        equal_rd0 192.0.2.53%0 192.0.2.53%0
        rd0_vs_absent 192.0.2.53%0 192.0.2.53
        routed_vs_absent 192.0.2.53%2 192.0.2.53
        domain_mismatch 192.0.2.53%2 192.0.2.53%3
        routed_prefix 192.0.2.53%2 192.0.2.0%2/24
        dotted_mask 192.0.2.53 192.0.2.0/255.255.255.0
        ipv6_prefix 2001:db8::53%0 2001:db8::/64
        dual_prefix 192.0.2.53/24 192.0.2.0/16
    } {
        set status [catch {IP::addr $left equals $right} result]
        lappend lines "$label status=$status result=$result"
    }
    HTTP::respond 200 content [join $lines "\n"] Content-Type text/plain
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26

- Invocation: TMM `HTTP_REQUEST` on the temporary TCP/HTTP virtual.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
equal_rd0 status=0 result=1
rd0_vs_absent status=0 result=1
routed_vs_absent status=0 result=0
domain_mismatch status=0 result=0
routed_prefix status=0 result=1
dotted_mask status=0 result=1
ipv6_prefix status=0 result=1
dual_prefix status=0 result=1
```

An explicit zero matched an omitted route domain in this context. A
nonzero domain failed to match the same address in the omitted domain or in
a different nonzero domain. Dotted masks and the IPv6 prefix both worked.
The final case matches under either prefix, so it leaves prefix precedence
unmeasured. The temporary rule and virtual were removed after the request.
