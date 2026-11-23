# BIG-IP TLS event order and HTTP header forms

## Question

Which SSL events run in order on each side of a TLS-terminating BIG-IP virtual,
and what does the iRule runtime do with the apparent extra arguments in
`HTTP::header`? The event sequence constrains flow explanations and profile
scaffolds. The observed header writes distinguish a mutating form from a
getter for arity checking, side-effect analysis and taint.

## Assumptions

- Two fresh connections entered the same lab HTTP virtual with TCP, HTTP,
  Client SSL and Server SSL profiles. The client requested TLS 1.2. A
  disposable OpenSSL server on the same lab network accepted backend TLS.
- The first GET requested `/nhq-local` so the rule responded directly. The
  second requested `/nhq-proxy` so the backend responded. Both supplied
  `Host: nhq.invalid`, `nhq_no_such_subcommand: nhq-implicit`, a synthetic
  `Authorization: Bearer nhq-demo` and `Connection: close`.
- The temporary string data group `/Common/nhq261123_probe_group` contained
  the single record `probe`.
- The rule logged `CLIENTSSL_SERVERHELLO_SEND` if delivered. The run observed
  only the events below. It does not establish this event's conditions or
  behavior on untested handshakes, profiles, modules or releases.

## Script

```tcl-visible
when CLIENTSSL_CLIENTHELLO {
    log local0. "NHQ261123|CLIENTSSL_CLIENTHELLO"
}
when CLIENTSSL_SERVERHELLO_SEND {
    log local0. "NHQ261123|CLIENTSSL_SERVERHELLO_SEND"
}
when CLIENTSSL_HANDSHAKE {
    log local0. "NHQ261123|CLIENTSSL_HANDSHAKE|[SSL::cipher version]"
}
when HTTP_REQUEST {
    set good [catch {HTTP::header value Host} host]
    set before [HTTP::header value nhq_no_such_subcommand]
    set bad_subcommand [catch {HTTP::header nhq_no_such_subcommand Host} subcommand]
    set explicit [HTTP::header value nhq_no_such_subcommand]
    set extra_arg [catch {HTTP::header value Host Extra} extra]
    set host_after [HTTP::header value Host]
    set value_after [HTTP::header value value]
    set auth [HTTP::header value Authorization]
    set count [HTTP::header count Authorization]
    set deprecated [catch {matchclass probe equals /Common/nhq261123_probe_group} match]
    log local0. "NHQ261123|HTTP_REQUEST|value=$good,$host|before=$before|subcommand=$bad_subcommand,$subcommand|explicit=$explicit|extra=$extra_arg,$extra|host_after=$host_after|value_after=$value_after|authorization_matches=[expr {$auth eq {Bearer nhq-demo}}]|count=$count|matchclass=$deprecated,$match"
    if {[HTTP::path] eq "/nhq-local"} {
        HTTP::respond 200 content nhq-ok
    }
}
when SERVERSSL_CLIENTHELLO_SEND {
    log local0. "NHQ261123|SERVERSSL_CLIENTHELLO_SEND"
}
when SERVERSSL_SERVERHELLO {
    log local0. "NHQ261123|SERVERSSL_SERVERHELLO"
}
when SERVERSSL_HANDSHAKE {
    log local0. "NHQ261123|SERVERSSL_HANDSHAKE|[SSL::cipher version]"
}
```

## Results

### BIG-IP 21.1.0.1 build 0.0.26

- Invocation: TMM events on the lab virtual. A temporary TMSH merge attached
  the rule. The client sent the local and proxied requests in that order.
- Exit status: Not recorded.
- Standard output: Not recorded.
- Standard error: Not recorded.
- Recorded result:

```text-visible
NHQ261123|CLIENTSSL_CLIENTHELLO
NHQ261123|CLIENTSSL_HANDSHAKE|TLSv1.2
NHQ261123|HTTP_REQUEST|value=0,nhq.invalid|before=nhq-implicit|subcommand=0,|explicit=Host|extra=0,|host_after=Extra|value_after=|authorization_matches=1|count=1|matchclass=0,1
NHQ261123|CLIENTSSL_CLIENTHELLO
NHQ261123|CLIENTSSL_HANDSHAKE|TLSv1.2
NHQ261123|HTTP_REQUEST|value=0,nhq.invalid|before=nhq-implicit|subcommand=0,|explicit=Host|extra=0,|host_after=Extra|value_after=|authorization_matches=1|count=1|matchclass=0,1
NHQ261123|SERVERSSL_CLIENTHELLO_SEND
NHQ261123|SERVERSSL_SERVERHELLO
NHQ261123|SERVERSSL_HANDSHAKE|TLSv1.3
```

These are the matching `/var/log/ltm` message payloads in delivery order,
without syslog envelopes. Both requests returned HTTP 200. The local response
contained `nhq-ok`. The lab virtual, rule, pool and data group were confirmed
absent after cleanup.

The unrecognised header position wrote the value `Host` to that header. The
apparent extra argument in `HTTP::header value Host Extra` wrote `Extra` to
the Host header. Both writes returned an empty result with catch code 0.
The Authorization getter matched the supplied bytes and the count returned
`1`. The documented deprecated `matchclass` returned the matched index `1`.
[HTTP::header reference](https://clouddocs.f5.com/api/irules/HTTP__header.html) ·
[matchclass reference](https://clouddocs.f5.com/api/irules/matchclass.html)
