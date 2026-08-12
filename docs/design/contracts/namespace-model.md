# KCS: Namespace model design

> **Status note (2026-07):** for standard-Tcl **command-name resolution**
> the implemented, tclsh-pinned contract is
> [command-resolution.md](command-resolution.md) — one canonical algorithm
> (`tcl_syntax::naming::resolve_command_with`) shared by the analyser,
> optimiser, bytecode VM, and WASM runtime, gated by a conformance vector
> suite. The design below predates that work; its point 1 claim that the
> LSP "does not track which namespace is active or resolve qualified
> names" is no longer accurate for command resolution. The dialect-model
> material (iRules protocol/proc namespaces, EDA) remains current.

## Problem

The term "namespace" means different things across the dialects this LSP
supports, and the current codebase has no unified model for any of them:

1. **Standard Tcl namespaces** -- `namespace eval ::foo { proc bar {} {...} }`
   creates `::foo::bar`.  Commands resolve through namespace paths, `namespace
   import`, `namespace export`, and `namespace unknown`.  Packages install
   into namespaces.  OO classes live in namespaces.  The LSP currently parses
   the `namespace` command but does not track which namespace is active or
   resolve qualified names.

2. **F5 iRules "protocol namespaces"** -- `HTTP::`, `SSL::`, `TCP::` etc.
   These are not Tcl `namespace eval` namespaces.  They are command prefixes
   whose availability depends on which profiles are attached to the virtual
   server and which events have fired.  `EventRequires` / `EventProps` /
   `FlowChain` (`rust/tcl-registry/src/events.rs`) encode this knowledge, and
   `ProtocolNamespaceSpec` (`rust/tcl-registry/src/profiles.rs`) maps each
   prefix to the profiles that provide it.

3. **F5 iRules "proc namespaces"** -- when you define `proc my_helper {}` in
   an iRule named `my_irule`, it ends up as `::my_irule::my_helper` and is
   globally callable from any other iRule via `call my_irule::my_helper` or
   `call /partition/my_irule::my_helper`.  The iRule name is the namespace.
   The LSP has no model for this today.

4. **EDA tool namespaces** -- EDA tools have their own namespace conventions
   and the support is deliberately minimal and conservative.  Note that EDA
   shells are no longer modelled as vendor *dialects*: the Synopsys /
   Cadence / Xilinx / Quartus / Mentor bits were retired from `DialectSet`,
   and an EDA shell is now a base Tcl version plus `required_package`-gated
   command libraries (see
   [eda-library-packages.md](../eda-library-packages.md)).

## Property naming convention

All boolean and set properties must be expressed in **positive form**:
`flow`, `client_side`, `dialects` (not `no_flow`, `excluded_dialects`,
`never_inline_body`).  A positive property means "this thing is true /
present".  Consumers check `if props.flow` rather than `if !props.no_flow`.

Where the existing model uses negative forms, the migration flips them:

| Old (negative)          | New (positive)                                   | Semantics                        |
|-------------------------|--------------------------------------------------|----------------------------------|
| a stored "non-flow" set  | `EventProps::flow: bool`, negated at the query site | No stored negative set        |
| `excluded_dialects`     | `CommandSpec::dialects: Option<DialectSet>`      | Explicit set of supported dialects |
| `never_inline_body`     | (out of scope)                                   | Noted for future cleanup         |

The Rust registry already follows this: `tcl_registry::events::EventProps`
stores `flow`, `client_side`, `server_side`, `hot`, and `common` as positive
booleans, and `tcl_registry::events::event_satisfies` derives the negative
cases at the point of use.  The one exception is
`CommandSpec::excluded_events`, which is a genuine per-command exclusion
list rather than a negated set.

## Design: two-tier namespace model

Rather than one model trying to be everything, use two tiers:

### Tier 1: Tcl namespace scope tracking (all dialects)

A model that tracks the "current namespace" at any point in a file.  This
is dialect-agnostic and works for standard Tcl and iRules alike.

**This tier is implemented, but not as a single standalone tracker type.**
The work is split between three places, none of which is called
`NamespaceTracker`:

| Concern | Where it lives |
|---|---|
| Name spelling: qualification, normalisation, splitting | [`tcl_syntax::naming`](../../../rust/tcl-syntax/src/naming.rs) — `normalise_qualified_name`, `qualify`, `qualifier_segments`, `key_holder_and_tail`, `is_qualified` |
| Call-site resolution against a candidate set | `tcl_syntax::naming::command_resolution_candidates` / `bareword_resolution_candidates` / `resolve_command_with` — the single canonical algorithm, contracted in [command-resolution.md](command-resolution.md) |
| Scope carried through lowering | `tcl_compiler::ir::Procedure::qualified_name` and `Procedure::namespace_scoped`; `Statement::Block` carries the fully-qualified `namespace` its body was lowered in |
| Declared `namespace path` per namespace | `tcl_compiler::analyser` — `handle_namespace_path_command` records literal declarations into `namespace_paths: HashMap<String, Vec<String>>`; `command_resolution_namespace` returns the namespace a call site resolves in |

The lowering pass qualifies procedure names as it walks
`namespace eval` bodies, so `namespace eval mylib { proc helper … }`
lands in `Module::procedures` under the key `::mylib::helper` (Example 26
of [example-script-walkthroughs.md](../example-script-walkthroughs.md)
traces this end to end).  Resolution of a *call* to that name is a
separate, call-time step, and it is the one governed by
[command-resolution.md](command-resolution.md).

> **Naming caution.** `tcl_registry::NamespaceScope` exists but is an
> unrelated type: it is the namespace facet of the world-effect
> descriptor (`Current` / `Named` / `Any`), describing *which* namespace a
> command's effect touches, not the lexical scope a name is written in.

#### iRule name detection

The iRule name determines the namespace for its procs.  Three sources are
possible:

1. **Pragma comment** — `# irule: my_irule_name` (or
   `# irule: /Common/my_irule_name`).  **Not implemented.**  The only
   leading-comment directive the registry parses today is
   `# profiles: …`, via
   `tcl_registry::profiles::parse_profile_directive`, which scans at most
   the first 20 lines and stops at the first non-comment line.  An
   `# irule:` pragma would follow the same scan convention.
2. **Filename heuristic** — strip the `.tcl` / `.irule` suffix and take
   the basename.  Not implemented.
3. **`bigip.conf` extraction** — when processing a BIG-IP configuration
   the rule name comes from the `ltm rule` stanza.  This one *is*
   available: `tcl_bigip` parses `ltm rule` stanzas into `BigipRule`
   objects, and `tcl_bigip::irule_context::build_irule_context` resolves
   the objects one named rule references.

Once known, procs defined in that iRule resolve to
`::irule_name::proc_name` and are callable from any other iRule via
`call irule_name::proc_name`.  The `call` command itself is registry data
(`rust/tcl-registry/src/commands/irules/call.rs`): it carries
`Traits::INVOKES_USER_PROC` and `arg_roles: &[(0, ArgRole::Name)]`, so the
consumer that resolves the callee never needs to know the command by name.

#### Cross-iRule proc visibility

iRule procs are **globally visible** across all iRules loaded on the
system.  This means:

- `call my_proc` -- resolves to a proc defined in the **same** iRule
- `call other_irule::my_proc` -- resolves to a proc in `other_irule`
- `call /Common/other_irule::my_proc` -- fully qualified with partition

Cross-iRule resolution needs a file-set-aware mode for multi-file iRules
projects, but the basic model is: each iRule file contributes procs to
its `::irule_name::` namespace, and `call` can reference any of them.

### Tier 2: F5 protocol namespace model (iRules only)

This is the profile/event/layer/side model specific to F5.

This tier is implemented in
[`rust/tcl-registry/src/profiles.rs`](../../../rust/tcl-registry/src/profiles.rs)
as `ProfileSpec`, `ProtocolNamespaceSpec`, `StackModification`, and the
`ProfileRegistry` facade over their static tables (57 profile types, 87
protocol command namespaces).

#### ProfileSpec -- profile type metadata

```rust
/// Metadata for an F5 profile type.
pub struct ProfileSpec {
    /// Profile type name (e.g. `"HTTP"`, `"CLIENTSSL"`, `"DNS"`).
    pub name: &'static str,
    /// Protocol stack layer.
    pub layer: &'static str,
    /// Connection side: `"client"`, `"server"`, `"both"`, `"global"`.
    pub side: &'static str,
    /// Required parent profiles.
    pub requires: &'static [&'static str],
    /// Conflicting profiles.
    pub conflicts: &'static [&'static str],
    /// Profile capabilities (e.g. `"sni"`, `"cipher"`, `"cert"`).
    pub capabilities: &'static [&'static str],
    /// Introduction / deprecation / retirement releases on the BIG-IP
    /// release axis; an absent introducing release inherits the axis
    /// baseline (BIG-IP 15.0).
    pub lifecycle: Lifecycle,
}
```

The `capabilities` field models **what subset of protocol functionality** a
profile provides.  This is critical for profiles like SSL persistence that
provide a reduced subset of an SSL profile's functionality without requiring
a full TLS termination profile.  See
[SSL persistence profile](#ssl-persistence-profile) below.

The `lifecycle` field version-gates a profile type: `ProfileRegistry::
profile_available_at(name, version)` answers whether the type exists at a
given BIG-IP release (e.g. `AIMCP` is introduced in 21.1.0), and
`profile_lifecycle` returns the resolved range.

The `conflicts` field is intended to encode **mutual exclusivity**.  On a
real BIG-IP, certain profiles cannot coexist on the same connection -- or
if they do, one must be disabled before the other's events fire.
**No shipped `ProfileSpec` populates `conflicts` today** — every entry in
the generated table is `conflicts: &[]`, and no consumer reads the field.
The exclusivity groups below are therefore the design intent, not
enforced data.

#### Mutual exclusivity groups

Within a layer, at most one profile from each exclusivity group can be
active at a time on a given connection:

| Layer       | Exclusivity group  | Members                   | Notes |
|-------------|--------------------|---------------------------|-------|
| transport   | transport          | TCP, UDP, FASTL4, SCTP    | A VS listens on exactly one transport |
| application | app_protocol       | HTTP, FASTHTTP, DNS, SIP, FIX, DIAMETER, MQTT, RTSP, GENERICMSG | Only one L7 protocol parser active per connection |
| tls         | tls_client         | CLIENTSSL                 | At most one client-side TLS profile |
| tls         | tls_server         | SERVERSSL                 | At most one server-side TLS profile |

This means:
- You cannot have both TCP and UDP on the same connection.
- You cannot have both HTTP and DNS processing the same traffic.
- HTTP and FASTHTTP are mutually exclusive (FASTHTTP is a
  high-performance HTTP variant).

#### Dynamic protocol switching via collect + inspect

Some advanced configurations use `TCP::collect` in `CLIENT_ACCEPTED` or
`CLIENT_DATA` to inspect the first payload bytes and then enable/disable
profiles accordingly.  For example, a VS that handles both HTTP and
HTTPS on the same port:

```tcl
# profiles: TCP, CLIENTSSL, HTTP
when CLIENT_ACCEPTED {
    TCP::collect
}

when CLIENT_DATA {
    # Look at first byte to determine if it's TLS
    binary scan [TCP::payload] c first_byte
    if { $first_byte != 22 } {
        # Not a TLS ClientHello -- disable SSL, keep HTTP
        SSL::disable
        TCP::release
    } else {
        # TLS handshake -- let SSL handle it
        TCP::release
    }
}
```

Similarly, a VS that might handle DNS or HTTP based on the port or
payload would disable one protocol early:

```tcl
# profiles: TCP, HTTP, DNS
when CLIENT_ACCEPTED {
    # Determine protocol from destination port or early payload
    if { [TCP::local_port] == 53 } {
        HTTP::disable
        # DNS events will fire, HTTP events will not
    } else {
        # HTTP events will fire, DNS events will not
    }
}
```

The design handles this through `StackModification`: disabling a profile
removes it from the active set, which removes its events from the
reachable event set and its command namespace from the valid set, with
some per-connection state tracking the modification timeline so we can
determine what is valid at each event.  `StackModification` exists as
registry data (four entries, see below); the replay that would consume it
does not.

#### Conflicts in the profile table

Populating `conflicts` would look like this (proposed — see the note
above, the shipped table leaves every entry empty):

```rust
ProfileSpec { name: "TCP",    conflicts: &["UDP", "FASTL4", "SCTP"], .. },
ProfileSpec { name: "UDP",    conflicts: &["TCP", "FASTL4", "SCTP"], .. },
ProfileSpec { name: "FASTL4", conflicts: &["TCP", "UDP", "SCTP"],    .. },
ProfileSpec { name: "HTTP",   conflicts: &["FASTHTTP", "DNS", "SIP", "FIX"], .. },
```

A consumer would then validate that no two conflicting profiles are
active simultaneously (unless one is disabled via a `StackModification`
before the other's events fire).

The `layer` field models the protocol stack position.  The values in the
shipped table, and the wire-proximity order `ProfileRegistry::layer_rank`
assigns them (lowest is nearest the wire), are:

| Rank | Layer         | Examples                      |
|:----:|---------------|-------------------------------|
| 0    | transport     | TCP, UDP, FASTL4, SCTP        |
| 1    | tls_shared    | PERSIST, SSL_PERSISTENCE      |
| 2    | tls           | CLIENTSSL, SERVERSSL          |
| 3    | application   | HTTP, FASTHTTP, DNS, SIP, FIX, MQTT, DIAMETER, MR |
| 4    | load_balance  | (namespaces only — no profile type) |
| 5    | security      | ASM, BOTDEFENSE, ACCESS, PEM  |
| 6    | acceleration  | WEBACCELERATION, STREAM, XML  |
| 7    | utility       | (namespaces only — no profile type) |

`ProfileRegistry::is_infrastructure_profile` derives "the stack implies
this, the operator does not pick it" from the `transport` and
`tls_shared` layers; the `# Profiles:` header code action filters those
out.

The `side` field models the connection side:

| Side    | Meaning                                           |
|---------|---------------------------------------------------|
| client  | Client-to-BIG-IP leg (CLIENTSSL, client TCP)      |
| server  | BIG-IP-to-server leg (SERVERSSL, server TCP)      |
| both    | Spans both legs (HTTP, DNS -- proxied)             |
| global  | No connection context (RULE_INIT, PERSIST_DOWN)   |

#### ProtocolNamespaceSpec -- protocol command namespace

```rust
/// iRules protocol command namespace availability.
///
/// Not a Tcl namespace — these are command prefixes whose availability
/// depends on attached profiles.
pub struct ProtocolNamespaceSpec {
    /// Namespace prefix (e.g. `"HTTP"`, `"SSL"`, `"TCP"`).
    pub prefix: &'static str,
    /// Profiles that provide this namespace.
    pub profiles: &'static [&'static str],
    /// Protocol layer.
    pub layer: &'static str,
    /// Default connection side.
    pub side: &'static str,
    /// Whether `clientside`/`serverside` qualifiers are supported.
    pub side_selectable: bool,
}
```

The key insight: **profiles map to protocol namespaces**.  When a profile is
active, its command prefixes become available.  When it's removed (e.g.
`SSL::disable`), the commands are revoked for subsequent events.

A sample of the shipped table (`ProfileRegistry::get_namespace`):

| Namespace | Profiles                                        | Layer        | Side   | Side-selectable |
|-----------|-------------------------------------------------|--------------|--------|:---------------:|
| HTTP      | `FASTHTTP`, `HTTP`, `HTTP_PROXY_CONNECT`        | application  | both   | no              |
| SSL       | `CLIENTSSL`, `PERSIST`, `SERVERSSL`, `SSL_PERSISTENCE` | tls   | both   | yes             |
| TCP       | `TCP`                                           | transport    | both   | yes             |
| UDP       | `UDP`                                           | transport    | both   | yes             |
| IP        | (none — always present)                         | transport    | both   | yes             |
| LB        | (none — always present)                         | load_balance | global | no              |
| DNS       | `DNS`                                           | application  | both   | no              |

An empty `profiles` slice means the namespace has no profile gate: `LB::`
and `IP::` resolve regardless of the attached stack.

#### Modelling the layer stack

There is **no `LayerStack` type in Rust**.  The two facts a stack model
would carry are held directly on the specs and derived on demand:

- The layer a profile sits at is `ProfileSpec::layer`; ordering the stack
  bottom-up is `ProfileRegistry::layer_rank`.
- Transitive stack membership is
  `ProfileRegistry::expand_profile_stack(&["HTTP"])`, which adds every
  transitive `requires` parent (so `HTTP` pulls in `TCP`), uppercasing as
  it goes.  `ProfileRegistry::stack_satisfies(required, active)` then
  tests a requirement set against an active stack with OR semantics.

Shared/non-terminating TLS helpers such as `PERSIST` and
`SSL_PERSISTENCE` carry `layer: "tls_shared"` so they rank below — and
coexist with — the client-side or server-side `tls` profiles rather than
displacing them.

`tcl_registry::profiles::compute_file_profiles(source, events, profiles)`
is the whole-file entry point: it unions the `# profiles:` directive with
the profiles implied by every `when EVENT` handler present, expands the
transitive stack, and returns a sorted, uppercased profile list.

#### StackModification -- dynamic profile changes

```rust
/// A command that changes the active profile stack at runtime.
pub struct StackModification {
    /// Command name (e.g. `"SSL::disable"`).
    pub command: &'static str,
    /// Connection side affected.
    pub side: Option<&'static str>,
    /// Profile removed by this command.
    pub removes_profile: Option<&'static str>,
    /// Profile added by this command.
    pub adds_profile: Option<&'static str>,
}
```

The shipped table has four entries (`SSL::disable`, `SSL::enable`,
`HTTP::disable`, `HTTP::enable`), reachable via
`ProfileRegistry::modifications()`.  **No consumer reads them yet** — the
timeline-replay analysis the rest of this section describes is design, not
implemented behaviour.  The design also wanted a
`requires_before_event: Option<&'static str>` field for the
"`SSL::disable serverside` must precede `SERVER_CONNECTED`" case; that
field does not exist.

All properties use positive form.  `flow = true` means an active traffic
flow is present; `client_side = true` means the client-side connection is
available.  There are no negative-form stored sets — those are derived at
the query site (`tcl_registry::events::event_satisfies`).

#### Per-connection state

There is **no `ConnectionModel` type**.  What exists instead is the static
event model in
[`rust/tcl-registry/src/events.rs`](../../../rust/tcl-registry/src/events.rs):

| Design concept | Implemented as |
|---|---|
| "what profiles are effective at this event" | `EventProps::implied_profiles` (plus `transport`, `client_side`, `server_side`, `flow`), read via `EventRegistry::get_props` |
| "which namespaces are available here" | derived: intersect `ProtocolNamespaceSpec::profiles` with the event's implied stack |
| "apply a dynamic profile change" | not implemented (see `StackModification` above) |

#### Event graph model

There is **no `EventNode` / `EventEdge` / `EventGraph`**.  The ordering
and reachability facts they would carry are held as two flat static
tables on `EventRegistry`:

```rust
/// A step in an event flow chain.
pub struct FlowStep {
    /// Event name.
    pub event: &'static str,
    /// Logical phase (`init`, `l4_client`, `tls_client`, `http_request`, …).
    pub phase: &'static str,
    /// Whether this event only fires conditionally.
    pub conditional: bool,
    /// Human-readable condition note.
    pub condition_note: &'static str,
}

/// Complete event flow for a profile combination.
pub struct FlowChain {
    /// Unique identifier (e.g. `"plain_tcp"`, `"tcp_clientssl_http"`).
    pub chain_id: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Profile types on the virtual server.
    pub profiles: &'static [&'static str],
    /// Ordered steps.
    pub steps: Vec<FlowStep>,
    /// Any caveats or implementation notes.
    pub notes: &'static str,
}

/// An entry in the master event firing order.
pub struct OrderEntry {
    /// Event name.
    pub event: &'static str,
    /// Profile gates (must be active for the event to fire). Empty = always.
    pub profile_gates: &'static [&'static str],
}
```

`EventRegistry::flow_chains()` returns the per-stack chains
(`plain_tcp`, `tcp_http`, `tcp_clientssl_http`,
`tcp_clientssl_serverssl_http`, `tcp_clientssl_serverssl_http_collect`,
`udp_dns`, `tcp_dns`); `EventRegistry::master_order()` returns the
profile-gated global ordering, and `order_events` / `order_events_for_file`
sort an event set into firing order using it.

### The registry facade

There is **no `NamespaceRegistry`**.  Tier 1 and tier 2 are reached
through separate, already-built registries rather than one unifying type:

| Query | Rust entry point |
|---|---|
| Namespace-qualified name handling | `tcl_syntax::naming::*` (tier 1) |
| Look up a profile type | `ProfileRegistry::get_profile(name)` |
| Look up a protocol namespace | `ProfileRegistry::get_namespace(prefix)` |
| Transitive profile requirements | `ProfileRegistry::expand_profile_stack(&[…])` |
| Does an active stack satisfy a requirement? | `ProfileRegistry::stack_satisfies(required, active)` |
| Profiles implied by an event | `EventRegistry::get_props(event)` → `EventProps::implied_profiles` |
| Profiles a command needs | `CommandSpec::event_requires` → `EventRequires::profiles` |
| Is this command legal in this event? | `CommandRegistry::is_irules_command_legal_in_event(cmd, event, events, profiles)` — the O(1) legality-matrix test that drives IRULE1001 |
| Every command legal in an event | `CommandRegistry::valid_irules_commands_for_event(event, events, profiles, bigip_version)` |
| Stack-modification commands | `ProfileRegistry::modifications()` |
| Profiles for a whole file | `tcl_registry::profiles::compute_file_profiles(source, events, profiles)` |

The registries are built once and cached:
`tcl_registry::cache::default_registry()` /
`registry_for_dialect(dialect)` for commands, `ProfileRegistry::build()`
and `EventRegistry::build()` for the F5 tables.  There is no
`VirtualServerModel` type.

## Modelling complex scenarios

### iRule proc namespaces and cross-iRule calls

```tcl
# irule: /Common/utility_irule
proc log_request { msg } {
    log local0. $msg
}
# -> defines ::utility_irule::log_request
# -> callable from any iRule via: call utility_irule::log_request "hello"
```

```tcl
# irule: /Common/main_irule
when HTTP_REQUEST {
    call utility_irule::log_request [HTTP::uri]
    # also valid: call /Common/utility_irule::log_request [HTTP::uri]
}
```

With the iRule name known to be `utility_irule`, the intended resolution
is:
- `proc log_request` -> `::utility_irule::log_request`
- `call log_request` -> first looks in the current iRule's namespace, then
  global — which is exactly the base order
  `tcl_syntax::naming::command_resolution_candidates` already produces
  once the current namespace is `::utility_irule`

Without a known iRule name (no pragma, unclear filename), resolution
degrades gracefully: procs are treated as local, so a cross-iRule
reference simply stays unresolved rather than binding to the wrong
target.

### HTTP + HTTPS on the same port with SSL::disable

```tcl
# irule: /Common/ssl_offload
# profiles: TCP, CLIENTSSL, HTTP
when CLIENTSSL_CLIENTHELLO {
    if { [SSL::extensions exists -type 0] == 0 } {
        SSL::disable
        # After this: CLIENTSSL removed from stack
        # HTTP namespace still available (plain HTTP)
    }
}

when HTTP_REQUEST {
    # Fires for both SSL and non-SSL connections
    # HTTP:: commands work in both cases
    HTTP::header replace Host "example.com"
}
```

The model tracks this via `StackModification`:

1. Initial stack: `{TCP, CLIENTSSL, HTTP}`
2. In `CLIENTSSL_CLIENTHELLO`: `SSL::disable` applied
3. Effective stack for subsequent events: `{TCP, HTTP}` (CLIENTSSL removed)
4. `HTTP::` namespace remains valid because HTTP profile is still active
5. `SSL::` commands after the modification produce a diagnostic

### SSL persistence profile

On a real BIG-IP, an **SSL persistence profile** (`ltm persistence ssl`)
can be attached to a virtual server *without* a full client-ssl or
server-ssl profile.  The system parses the TLS ClientHello just enough
to extract the session ID for persistence keying.  This partial TLS
parsing makes a subset of read-only SSL:: commands available:

| Command | Works with SSL persistence? | Notes |
|---------|:--:|-------|
| `SSL::sni` | yes | Extracted from ClientHello SNI extension |
| `SSL::extensions exists -type N` | yes | ClientHello extension inspection |
| `SSL::sessionid` | yes | The persistence key itself |
| `SSL::cipher` | **no** | Requires completed handshake |
| `SSL::cert` | **no** | Requires full TLS termination |
| `SSL::collect` / `SSL::release` | **no** | Requires active TLS data path |
| `SSL::disable` / `SSL::enable` | **no** | Requires a TLS profile to toggle |
| `SSL::renegotiate` | **no** | Requires active TLS session |

This is a common pattern for **routing based on SNI** without TLS
termination (TLS pass-through with SNI inspection):

```tcl
# profiles: TCP, (ssl persistence only — no client-ssl)
when CLIENTSSL_CLIENTHELLO {
    # SSL persistence profile lets us read SNI without terminating TLS
    switch -- [SSL::sni name] {
        "app1.example.com" { pool app1_pool }
        "app2.example.com" { pool app2_pool }
        default            { pool default_pool }
    }
}
```

#### Modelling SSL persistence as a reduced-capability profile

The `capabilities` field on `ProfileSpec` captures this, and the shipped
table already carries it:

```rust
ProfileSpec {
    name: "CLIENTSSL",
    layer: "tls",
    side: "client",
    requires: &["TCP"],
    conflicts: &[],
    capabilities: &[
        "cert", "cipher", "extensions", "sessionid", "sni",
        "tls_control", "tls_data",
    ],
    ..ProfileSpec::DEFAULT
},
ProfileSpec {
    name: "SSL_PERSISTENCE",
    // Side-independent (shared) TLS layer. Stack infrastructure,
    // not an operator-selected profile.
    layer: "tls_shared",
    side: "client",
    requires: &["TCP"],
    conflicts: &[],
    capabilities: &["extensions", "sessionid", "sni"],
    ..ProfileSpec::DEFAULT
},
```

`SERVERSSL` carries the same seven capabilities as `CLIENTSSL`.  These
three are the only profile types in the table with a non-empty
`capabilities` list.

The intent is that a command declares the capability it needs rather than
(or in addition to) a profile name.  `EventRequires` has the field for it:

```rust
pub struct EventRequires {
    /// Requires client side.
    pub client_side: bool,
    /// Requires server side.
    pub server_side: bool,
    /// Required transport (`"tcp"` or `"udp"`).
    pub transport: Option<&'static str>,
    /// Required profile types.
    pub profiles: &'static [&'static str],
    /// Events where the command is unconditionally valid.
    pub also_in: &'static [&'static str],
    /// Only valid in `RULE_INIT`.
    pub init_only: bool,
    /// Requires active traffic flow.
    pub flow: bool,
    /// Required profile capability (e.g. `"sni"`).
    pub capability: Option<&'static str>,
}
```

**The `capability` field is declared but not yet used.**  Every shipped
command spec sets `capability: None`, and
`tcl_registry::events::event_satisfies` — the function that decides
IRULE1001 validity — does not consult it.  Wiring it up means (a) setting
`capability: Some("sni")` on `SSL::sni`, `Some("cipher")` on
`SSL::cipher`, `Some("tls_data")` on `SSL::collect`, and so on, and
(b) adding a clause to `event_satisfies` that asks whether any profile in
`EventProps::implied_profiles` lists that capability.

Once wired, `SSL::sni` in `CLIENTSSL_CLIENTHELLO` would be valid whether
the virtual server has a full CLIENTSSL profile *or* just an SSL
persistence profile, because both provide the `"sni"` capability.

#### SSL persistence events

With only an SSL persistence profile (no CLIENTSSL), only a subset of SSL
events fire:

| Event | Fires with SSL persistence? |
|-------|:--:|
| `CLIENTSSL_CLIENTHELLO` | yes |
| `CLIENTSSL_HANDSHAKE` | **no** |
| `CLIENTSSL_DATA` | **no** |

`EventProps::implied_profiles` already reflects this — the shipped entry
for `CLIENTSSL_CLIENTHELLO` is:

```rust
(
    "CLIENTSSL_CLIENTHELLO",
    EventProps {
        client_side: true,
        transport: &["tcp"],
        implied_profiles: &["CLIENTSSL", "PERSIST", "SSL_PERSISTENCE"],
        ..EventProps::DEFAULT
    },
),
```

`CLIENTSSL_HANDSHAKE` and `CLIENTSSL_DATA` do not list
`SSL_PERSISTENCE`.  The master-order gate for `CLIENTSSL_CLIENTHELLO` is
correspondingly wider than its siblings'
(`profile_gates: &["CLIENTSSL", "PERSIST"]`).

#### Impact on the protocol-namespace table

The SSL protocol namespace is available with SSL persistence too — this
is shipped data:

```rust
ProtocolNamespaceSpec {
    prefix: "SSL",
    profiles: &["CLIENTSSL", "PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
    layer: "tls",
    side: "both",
    side_selectable: true,
},
```

But individual `SSL::` commands should be gated by capability, not just
namespace availability.  The namespace is reachable (the prefix
resolves), but specific subcommands ought to produce diagnostics if the
required capability is missing — which is the unused
`EventRequires::capability` field described above.

### Profile requirements inference

Given an iRule using events `{HTTP_REQUEST, CLIENTSSL_HANDSHAKE,
SERVER_CONNECTED}`, the profile set the file needs is computed by
`compute_file_profiles`, which unions the `# profiles:` directive with
every event's `implied_profiles` and then expands the transitive stack:

```rust
use tcl_registry::events::EventRegistry;
use tcl_registry::profiles::{ProfileRegistry, compute_file_profiles};

let events = EventRegistry::build();
let profiles = ProfileRegistry::build();

let needed = compute_file_profiles(
    "# profiles: HTTP\nwhen CLIENTSSL_HANDSHAKE { }\n",
    &events,
    &profiles,
);
// -> ["CLIENTSSL", "HTTP", "TCP"]  (sorted; TCP is the transitive
//    parent of both CLIENTSSL and HTTP)
```

Expansion alone, without the event scan, is
`ProfileRegistry::expand_profile_stack(&["HTTP", "CLIENTSSL"])`.  Going
the other way — testing whether a command's declared requirement is met
by an active stack — is
`ProfileRegistry::stack_satisfies(spec.event_requires.profiles, active)`,
which is what `event_satisfies` calls.

There is no per-command "required profiles" roll-up helper: a consumer
reads `CommandSpec::event_requires` directly.

### Connection sides and proxy model

```
   Client           BIG-IP Proxy              Server
     |                  |                       |
     |--- client-side --|--- server-side -------|
     |   (CLIENTSSL)    |   (SERVERSSL)         |
     |   (client TCP)   |   (server TCP)        |
     |                  |                       |
     |    HTTP (both sides -- proxied)          |
```

### Standard Tcl namespace resolution

```tcl
namespace eval ::mylib {
    namespace export greet
    proc greet { name } { return "hello $name" }
    proc _helper {} { ... }  ;# not exported
}

namespace eval ::app {
    namespace import ::mylib::greet
    greet "world"             ;# resolves via import
    ::mylib::_helper          ;# resolves via FQN
}
```

Command-name resolution across these forms is the shared algorithm in
`tcl_syntax::naming::resolve_command_with`, pinned to real tclsh by the
conformance vectors — see
[command-resolution.md](command-resolution.md) for the rule, its
consumers, and the anti-drift gates.  The analyser records each
namespace's declared `namespace path` (`handle_namespace_path_command`)
and settles every call site against the whole-file proc table after the
walk, which is what gives call-time semantics for a proc defined later in
the file.  What it deliberately does not do is model runtime-only
behaviour: a namespace created by a computed name, or an import performed
inside an `eval`.

## Integration with existing code

### Where the pieces already live

Rather than a wrapping facade, the tier-2 model is spread across the
registry crate:

- `EventProps` (`rust/tcl-registry/src/events.rs`) — per-event side,
  transport, implied profiles, flow, and lifecycle.
- `EventRequires` on `CommandSpec` — per-command stack requirements,
  consumed by `event_satisfies` and thence
  `CommandRegistry::is_irules_command_legal_in_event` (IRULE1001).
- `FlowChain` / `OrderEntry` — the per-stack event chains and the
  profile-gated master firing order, consumed by
  `EventRegistry::order_events`.
- `tcl_bigip`'s parsed configuration — `BigipConfig`, `BigipRule`,
  `BigipProfile` — supplies a virtual server's real profile list to
  `tcl_bigip::irule_context::build_irule_context`.

### New diagnostic opportunities

None of the codes below exist.  The shipped iRules families are
IRULE1001–1008, IRULE1201/1202, IRULE2xxx, IRULE3xxx, IRULE4xxx, and
IRULE5xxx (`docs/generated/diagnostic_codes.md` is the generated
inventory), so any of these would need a new code allocated and the
editor catalogues regenerated (`make gen-editor-settings`):

- command `SSL::cert` used after `SSL::disable`
- virtual server needs a CLIENTSSL profile for this iRule
- `HTTP::header` in `CLIENT_ACCEPTED` — HTTP events fire later
- `SSL::disable serverside` must be before `SERVER_CONNECTED`
- profiles HTTP and DNS are mutually exclusive — disable one before its
  events fire
- `HTTP::header` used after `HTTP::disable` in the same connection flow
- `SSL::cipher` requires a client-ssl or server-ssl profile (ssl
  persistence alone is not sufficient)
- proc `foo` defined in namespace `::bar` shadows an import

The first six all depend on the `StackModification` replay that is not
implemented; the seventh depends on wiring
`EventRequires::capability`.

### Existing pragma format

The `# profiles: HTTP, CLIENTSSL` pragma is parsed by
`tcl_registry::profiles::parse_profile_directive`
([`rust/tcl-registry/src/profiles.rs`](../../../rust/tcl-registry/src/profiles.rs)).
It accepts the singular `# profile:` spelling too, matches
case-insensitively, splits on commas or whitespace, uppercases the
names, scans at most the first 20 lines, and stops at the first
non-comment, non-blank line.  Extending it with

```
# irule: my_irule_name
# irule: /Common/my_irule_name
```

would follow the same scan convention (`profile_directive_payload` is
the per-line matcher to generalise).

## File layout

```
rust/tcl-registry/src/
    profiles.rs         # ProfileSpec, ProtocolNamespaceSpec,
                        # StackModification, ProfileRegistry,
                        # layer_rank, parse_profile_directive,
                        # scan_file_events, compute_file_profiles,
                        # plus the generated static tables
    events.rs           # EventProps, EventRequires, FlowStep, FlowChain,
                        # OrderEntry, EventRegistry, event_satisfies,
                        # plus the generated event tables
    registry.rs         # CommandRegistry — is_irules_command_legal_in_event,
                        # valid_irules_commands_for_event
    spec.rs             # CommandSpec (event_requires, excluded_events, …)

rust/tcl-syntax/src/
    naming.rs           # tier 1: qualification, normalisation, and the
                        # canonical command-resolution algorithm

docs/design/contracts/
    namespace-model.md  # this document
```

## Data tables

All three tables live at the bottom of
[`rust/tcl-registry/src/profiles.rs`](../../../rust/tcl-registry/src/profiles.rs)
under an `AUTO-GENERATED — do not edit manually` marker, split into
chunked builder functions (`profile_specs_0()` … `profile_specs_6()`,
`protocol_namespace_specs_0()` … `protocol_namespace_specs_9()`) so no
single function is oversized.  `ProfileRegistry::build()` indexes them by
`name` / `prefix`.

### Profile specs

57 entries.  A representative slice, verbatim:

```rust
ProfileSpec {
    name: "TCP",
    layer: "transport",
    side: "both",
    requires: &[],
    conflicts: &[],
    capabilities: &[],
    ..ProfileSpec::DEFAULT
},
ProfileSpec {
    name: "CLIENTSSL",
    layer: "tls",
    side: "client",
    requires: &["TCP"],
    conflicts: &[],
    capabilities: &[
        "cert", "cipher", "extensions", "sessionid", "sni",
        "tls_control", "tls_data",
    ],
    ..ProfileSpec::DEFAULT
},
ProfileSpec {
    name: "HTTP",
    layer: "application",
    side: "both",
    requires: &["TCP"],
    conflicts: &[],
    capabilities: &[],
    ..ProfileSpec::DEFAULT
},
ProfileSpec {
    name: "ASM",
    layer: "security",
    side: "both",
    requires: &["HTTP"],
    conflicts: &[],
    capabilities: &[],
    ..ProfileSpec::DEFAULT
},
ProfileSpec {
    name: "AIMCP",
    layer: "application",
    side: "both",
    requires: &["HTTP"],
    conflicts: &[],
    capabilities: &[],
    lifecycle: Lifecycle::introduced_in("21.1.0"),
},
```

`ProfileSpec::DEFAULT` supplies `lifecycle: Lifecycle::UNSPECIFIED`, which
resolves to the BIG-IP 15.0 axis baseline; only a profile with real
version knowledge (such as `AIMCP`) names its own `lifecycle`.

### Protocol namespace specs

87 entries.  A representative slice, verbatim:

```rust
ProtocolNamespaceSpec {
    prefix: "HTTP",
    profiles: &["FASTHTTP", "HTTP", "HTTP_PROXY_CONNECT"],
    layer: "application",
    side: "both",
    side_selectable: false,
},
ProtocolNamespaceSpec {
    prefix: "SSL",
    profiles: &["CLIENTSSL", "PERSIST", "SERVERSSL", "SSL_PERSISTENCE"],
    layer: "tls",
    side: "both",
    side_selectable: true,
},
ProtocolNamespaceSpec {
    prefix: "TCP",
    profiles: &["TCP"],
    layer: "transport",
    side: "both",
    side_selectable: true,
},
ProtocolNamespaceSpec {
    prefix: "LB",
    profiles: &[],
    layer: "load_balance",
    side: "global",
    side_selectable: false,
},
```

`side_selectable: true` is rare — only `SSL`, `TCP`, `UDP`, and `IP`
accept `clientside` / `serverside` qualifiers.

### Stack-modification specs

The complete table, verbatim:

```rust
fn modification_specs() -> Vec<StackModification> {
    vec![
        StackModification {
            command: "SSL::disable",
            side: None,
            removes_profile: None,
            adds_profile: None,
        },
        StackModification {
            command: "SSL::enable",
            side: None,
            removes_profile: None,
            adds_profile: None,
        },
        StackModification {
            command: "HTTP::disable",
            side: None,
            removes_profile: Some("HTTP"),
            adds_profile: None,
        },
        StackModification {
            command: "HTTP::enable",
            side: None,
            removes_profile: None,
            adds_profile: Some("HTTP"),
        },
    ]
}
```

The `SSL::` pair leaves `removes_profile` / `adds_profile` unset because
the affected profile (`CLIENTSSL` or `SERVERSSL`) depends on the
`clientside` / `serverside` argument, which a consumer would have to
resolve from the call words.

## Discoverability

- [KCS index](../../../docs/design/README.md)
- [Command registry and event model](../../../docs/design/contracts/command-registry-event-model.md)
- [LSP feature providers](../../../docs/design/contracts/lsp-feature-providers.md)
