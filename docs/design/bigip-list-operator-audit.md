# BIG-IP List-Property Operator Audit

This page enumerates every list-valued property in the registry that
ships **without** a `list_operators` entry — and classifies what each
case means for `tmsh modify` emission.

The classification is what decides each property's `list_operators`
value in the generated BIG-IP spec data
(`rust/tcl-registry/src/bigip/data/*.rs`).

## Why this matters

`tmsh modify <kind> <path> { rules { /Common/r1 } }` is rejected by
the device.  The full-body modify form requires an explicit
operator on every list-valued field:

```
tmsh modify ltm virtual /Common/v { rules replace-all-with { /Common/r1 } }
```

`BigipPropertySpec::list_operators: &'static [&'static str]`
(`rust/tcl-registry/src/bigip/mod.rs`) tells the renderer which
operators a property accepts, and `is_list_valued()` is simply
"that slice is non-empty".  An empty slice means the renderer
falls through to a bare `<prop> { ... }` body — silently
producing scripts the device refuses.

## tmsh_list — needs `replace-all-with`

These are real list properties: a `tmsh modify` of the parent
object must use `replace-all-with` for the full-body write.  Every
entry below carries
`list_operators: &["add", "delete", "replace-all-with"]`; the
renderer (`rust/tcl-bigip/src/tmsh_emit.rs`) then emits
`<prop> replace-all-with { ... }` automatically.

- `gtm listener.rules`
- `gtm listener-doh-proxy.rules`
- `gtm listener-doh-server.rules`
- `gtm wideip a.rules`
- `gtm wideip aaaa.rules`
- `gtm wideip cname.rules`
- `gtm wideip https.rules`
- `gtm wideip mx.rules`
- `gtm wideip naptr.rules`
- `gtm wideip srv.rules`
- `gtm wideip svcb.rules`
- `ltm message-routing diameter route.peers`
- `ltm message-routing diameter transport-config.rules`
- `ltm message-routing generic route.peers`
- `ltm message-routing generic router.routes`
- `ltm message-routing generic transport-config.rules`
- `ltm message-routing mqtt transport-config.rules`
- `ltm message-routing sip route.peers`
- `ltm message-routing sip transport-config.rules`
- `ltm virtual.rules`
- `sys cluster.members`

## subblock_keyed — parent owns the operator

These properties live inside a sub-section of another property
(e.g. `apm profile connectivity.servers` sits under the
`client-policy` section).  The full-body emission renders the
whole parent property with one operator at the parent level; the
inner list doesn't need its own.  No override needed.

Examples:

- `apm profile connectivity.servers` (in section: `client-policy`)
- `ltm eviction-policy.bias-bytes` (in section: `strategies`)
- `pem forwarding-endpoint.hash-settings` (in section: `persistence`)
- `security dos profile.bot-signatures` (in section: `application`)
- `sys application service.column-names` (in section: `tables`)
- … 10 more

## uncertain — manual classification required

These need per-property inspection against the device.  Some are
real lists (e.g. `cm device.unicast-address`, `gtm link.cost-
segments`); others may be generator artifacts.  They are left with
an empty `list_operators` until classified — the worst case is a
silent emission gap on these specific properties, with the rest of
the catalogue covered.

Tracked as a follow-up.  Sample of the 66 uncertain entries:

- `apm aaa saml.auth-context-methods`
- `apm acl.entries`
- `cm device.unicast-address`
- `gtm link.cost-segments`
- `ltm dns cache resolver.root-hints`
- `ltm profile client-ssl.cert-extension-includes`
- `ltm virtual.fw-enforced-policy-rules`
- `net bwc policy.categories`
- `security http profile.evasion-techniques`
- `sys icall handler periodic.arguments`
- … 56 more

## Regenerating the audit

Walk the generated catalogue and print every list-typed property with
no operators:

```rust
use tcl_registry::bigip::{ValueKind, data::all_specs};

for spec in all_specs() {
    for prop in spec.properties {
        if prop.value_type == ValueKind::List && !prop.is_list_valued() {
            println!("{} .{}", spec.kind_spec.kind, prop.name);
        }
    }
}
```

Any new entry that lands in the `tmsh_list` bucket needs
`list_operators: &["add", "delete", "replace-all-with"]` on its
`BigipPropertySpec` in `rust/tcl-registry/src/bigip/data/*.rs` —
which is generated data, so the classification belongs in whatever
regenerates it, not in a hand edit that the next regeneration
discards.
