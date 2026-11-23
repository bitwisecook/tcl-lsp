# BIG-IP HTTP header dispatch and matchclass arity

## Question

Does the HTTP header command abbreviate or case-fold its selector, and which
invocations read or write a header? What argument counts does the deprecated
`matchclass` command accept? The answers determine registry form selection,
side effects and arity diagnostics.

## Assumptions

A single request went to a temporary TCP/HTTP virtual on the 21.1 lab
appliance. The request supplied `Host: nhq.invalid` and `X-NHQ: before`.
The string data group contained the one record `probe`. Both malformed
`matchclass` calls use a computed head so they reach TMM. Literal two- and
four-operand counterparts were already rejected by TMSH verify.

## Script

```tcl-visible
when HTTP_REQUEST {
    set short [HTTP::header va Host]
    set va_after [HTTP::header value va]
    set upper [HTTP::header VALUE Host]
    set host_before [HTTP::header value Host]
    set implicit [HTTP::header X-NHQ]
    set iw [catch {HTTP::header X-NHQ wrote} implicit_write]
    set implicit_after [HTTP::header X-NHQ]
    set ew [catch {HTTP::header value Host wrote} explicit_write]
    set host_after [HTTP::header value Host]
    set legacy_command matchclass
    set n2 [catch [list $legacy_command probe equals] two]
    set n3 [catch {matchclass probe equals /Common/nhreg261123_http_group} three]
    set n4 [catch [list $legacy_command probe equals /Common/nhreg261123_http_group extra] four]
    log local0. "NHREG261123|short=$short|va_after=$va_after|upper=$upper|host_before=$host_before|implicit=$implicit|implicit_write=$iw,$implicit_write|implicit_after=$implicit_after|explicit_write=$ew,$explicit_write|host_after=$host_after|matchclass=$n2,$n3,$three,$n4|two=$two|four=$four"
    HTTP::respond 200 content ok
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26

- Invocation: TMM `HTTP_REQUEST` on the temporary lab virtual.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHREG261123|short=|va_after=Host|upper=|host_before=nhq.invalid|implicit=before|implicit_write=0,|implicit_after=wrote|explicit_write=0,|host_after=wrote|matchclass=1,0,1,1|two=wrong # args: should be "matchclass <string> <contains | ends_with | equals | starts_with> <class>"|four=wrong # args: should be "matchclass <string> <contains | ends_with | equals | starts_with> <class>"
```

The log payload shows that `va` acted as a header name, while uppercase
`VALUE` did not select the getter. Both the implicit name/value form and
the exact `value` setter returned an empty string and changed the header.
The valid three-operand `matchclass` returned index 1. Two and four
operands both raised a catchable wrong-argument-count error. The HTTP
response was 200 and the temporary virtual, rule and data group were removed.
