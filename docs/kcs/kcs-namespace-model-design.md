# KCS: Namespace model design

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
   server and which events have fired.  The existing `EventRequires` /
   `EventProps` / `FlowChain` systems encode this knowledge but in scattered,
   implicit ways.

3. **F5 iRules "proc namespaces"** -- when you define `proc my_helper {}` in
   an iRule named `my_irule`, it ends up as `::my_irule::my_helper` and is
   globally callable from any other iRule via `call my_irule::my_helper` or
   `call /partition/my_irule::my_helper`.  The iRule name is the namespace.
   The LSP has no model for this today.

4. **EDA tool namespaces** -- EDA tools have their own namespace conventions
   but the LSP support is currently minimal and conservative.

## Property naming convention

All boolean and set properties must be expressed in **positive form**:
`has_flow`, `client_side`, `dialects` (not `no_flow`, `excluded_dialects`,
`never_inline_body`).  A positive property means "this thing is true /
present".  Consumers check `if prop.has_flow` rather than `if not
prop.no_flow`.

Where the existing model uses negative forms, the migration flips them:

| Old (negative)         | New (positive)           | Semantics                        |
|------------------------|--------------------------|----------------------------------|
| `EventRegistry._non_flow` | derived: `events where has_flow is False` | No stored negative set |
| `excluded_dialects`    | `dialects`               | Explicit set of supported dialects (out of scope for this plan but noted) |
| `never_inline_body`    | (out of scope)           | Noted for future cleanup         |

Within the new namespace model all properties follow positive form from
day one.

## Design: two-tier namespace model

Rather than one model trying to be everything, use two tiers:

### Tier 1: Tcl namespace scope tracker (all dialects)

A lightweight model that tracks the "current namespace" at any point in
a file.  This is dialect-agnostic and works for standard Tcl and iRules
alike.

```python
@dataclass(frozen=True, slots=True)
class NamespaceScope:
    """A namespace scope at a point in the source."""

    path: tuple[str, ...]               # ("", "foo", "bar") for ::foo::bar
    source: str                         # "namespace_eval", "proc_body", "irule", "oo_class"

    @property
    def qualified(self) -> str:
        """Fully qualified name: '::foo::bar'."""
        return "::" + "::".join(self.path) if self.path else "::"

    def resolve(self, name: str) -> str:
        """Resolve a potentially unqualified name in this scope."""
        if name.startswith("::"):
            return name
        return self.qualified + "::" + name


@dataclass(slots=True)
class NamespaceTracker:
    """Tracks namespace scope through a file's AST.

    Handles:
    - namespace eval ::foo { ... } -- pushes scope
    - proc definitions -- infers parent namespace
    - oo::class create -- pushes scope for methods
    - iRule name detection -- sets root scope for procs
    """

    _stack: list[NamespaceScope] = field(default_factory=list)
    _irule_name: str | None = None      # from pragma or filename

    @property
    def current(self) -> NamespaceScope:
        return self._stack[-1] if self._stack else NamespaceScope((), "global")

    def push(self, scope: NamespaceScope) -> None: ...
    def pop(self) -> None: ...

    def set_irule_name(self, name: str) -> None:
        """Set the iRule name (from pragma comment or filename).

        In iRules, procs defined at top level end up in ::irule_name::
        """
        self._irule_name = name

    @property
    def irule_namespace(self) -> str | None:
        """The iRule's implicit namespace, if known."""
        return f"::{self._irule_name}" if self._irule_name else None
```

#### iRule name detection

The iRule name determines the namespace for its procs.  We need it from
one of:

1. **Pragma comment** (most reliable):
   ```tcl
   # irule: my_irule_name
   # OR
   # irule: /Common/my_irule_name
   ```

2. **Filename heuristic**: strip `.tcl`/`.irule` suffix, take the basename.
   E.g. `/path/to/my_irule.tcl` -> `my_irule`.

3. **bigip.conf extraction**: when processing a `bigip.conf`, the iRule
   name comes from the `ltm rule` stanza.

Once known, procs defined in that iRule resolve to
`::irule_name::proc_name` and are callable from any other iRule via
`call irule_name::proc_name`.

#### Cross-iRule proc visibility

iRule procs are **globally visible** across all iRules loaded on the
system.  This means:

- `call my_proc` -- resolves to a proc defined in the **same** iRule
- `call other_irule::my_proc` -- resolves to a proc in `other_irule`
- `call /Common/other_irule::my_proc` -- fully qualified with partition

The namespace tracker needs a file-set-aware resolution mode for
multi-file iRules projects, but the basic model is: each iRule file
contributes procs to its `::irule_name::` namespace, and `call` can
reference any of them.

### Tier 2: F5 protocol namespace model (iRules only)

This is the profile/event/layer/side model specific to F5.

#### ProfileSpec -- profile type metadata

```python
@dataclass(frozen=True, slots=True)
class ProfileSpec:
    """Metadata for one F5 profile type."""

    name: str                           # "HTTP", "CLIENTSSL", "SERVERSSL", "TCP", ...
    layer: str                          # "transport", "tls", "application", "security"
    side: str                           # "client", "server", "both", "global"
    requires: frozenset[str] = frozenset()   # profiles this one requires
    conflicts: frozenset[str] = frozenset()  # profiles this one conflicts with
    capabilities: frozenset[str] = frozenset()  # capabilities this profile provides
```

The `capabilities` field models **what subset of protocol functionality** a
profile provides.  This is critical for profiles like SSL persistence that
provide a reduced subset of an SSL profile's functionality without requiring
a full TLS termination profile.  See
[SSL persistence profile](#ssl-persistence-profile) below.

The `conflicts` field encodes **mutual exclusivity**.  On a real BIG-IP,
certain profiles cannot coexist on the same connection -- or if they do,
one must be disabled before the other's events fire.

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

The model handles this through `StackModification`: disabling a profile
removes it from the active set, which removes its events from the
reachable event graph and its command namespace from the valid set.  The
`ConnectionModel` tracks the modification timeline so we can determine
what's valid at each event.

#### Conflicts in PROFILE_SPECS

The `conflicts` field on `ProfileSpec` encodes these exclusivity groups:

```python
"TCP":       ProfileSpec("TCP",       ..., conflicts=frozenset({"UDP", "FASTL4", "SCTP"})),
"UDP":       ProfileSpec("UDP",       ..., conflicts=frozenset({"TCP", "FASTL4", "SCTP"})),
"FASTL4":    ProfileSpec("FASTL4",    ..., conflicts=frozenset({"TCP", "UDP", "SCTP"})),
"HTTP":      ProfileSpec("HTTP",      ..., conflicts=frozenset({"FASTHTTP", "DNS", "SIP", "FIX", ...})),
"FASTHTTP":  ProfileSpec("FASTHTTP",  ..., conflicts=frozenset({"HTTP", "DNS", "SIP", "FIX", ...})),
"DNS":       ProfileSpec("DNS",       ..., conflicts=frozenset({"HTTP", "FASTHTTP", "SIP", "FIX", ...})),
```

When constructing a `VirtualServerModel`, the registry validates that
no two conflicting profiles are active simultaneously (unless one is
disabled via a `StackModification` before the other's events fire).

The `layer` field models the protocol stack position:

| Layer         | Examples                      |
|---------------|-------------------------------|
| transport     | TCP, UDP, FASTL4              |
| tls           | CLIENTSSL, SERVERSSL          |
| application   | HTTP, FASTHTTP, DNS, SIP, FIX |
| security      | ASM, BOTDEFENSE, ACCESS       |
| acceleration  | WEBACCELERATION, STREAM       |
| message       | GENERICMSG, MR, DIAMETER      |

The `side` field models the connection side:

| Side    | Meaning                                           |
|---------|---------------------------------------------------|
| client  | Client-to-BIG-IP leg (CLIENTSSL, client TCP)      |
| server  | BIG-IP-to-server leg (SERVERSSL, server TCP)      |
| both    | Spans both legs (HTTP, DNS -- proxied)             |
| global  | No connection context (RULE_INIT, PERSIST_DOWN)   |

#### ProtocolNamespaceSpec -- protocol command namespace

```python
@dataclass(frozen=True, slots=True)
class ProtocolNamespaceSpec:
    """An iRules protocol command namespace (e.g. HTTP::, SSL::, TCP::).

    Not a Tcl namespace -- these are command prefixes whose availability
    depends on attached profiles.
    """

    prefix: str                         # "HTTP", "SSL", "TCP", "LB", ...
    profiles: frozenset[str]            # profiles that provide this namespace
    layer: str                          # which protocol layer it operates on
    side: str                           # default connection side
    side_selectable: bool = False       # can append "clientside"/"serverside"
```

The key insight: **profiles map to protocol namespaces**.  When a profile is
active, its command prefixes become available.  When it's removed (e.g.
`SSL::disable`), the commands are revoked for subsequent events.

| Namespace | Profiles                    | Layer       | Side   |
|-----------|-----------------------------|-------------|--------|
| HTTP      | {HTTP, FASTHTTP}            | application | both   |
| SSL       | {CLIENTSSL, SERVERSSL}      | tls         | both*  |
| TCP       | {TCP}                       | transport   | both*  |
| LB        | (any)                       | load_balance| global |
| DNS       | {DNS}                       | application | both   |

*`SSL` and `TCP` commands accept `clientside`/`serverside` qualifiers.

#### LayerStack -- protocol stack model

```python
@dataclass(frozen=True, slots=True)
class LayerStack:
    """Active protocol layers for a connection at a point in time."""

    transport: str | None = None        # "TCP" or "UDP"
    tls_client: str | None = None       # "CLIENTSSL" or None
    tls_server: str | None = None       # "SERVERSSL" or None
    application: frozenset[str] = frozenset()  # {"HTTP"}, {"DNS"}, ...
    security: frozenset[str] = frozenset()     # {"ASM", "ACCESS"}, ...
    acceleration: frozenset[str] = frozenset() # {"STREAM", "WEBACCELERATION"}, ...
```

#### StackModification -- dynamic profile changes

```python
@dataclass(frozen=True, slots=True)
class StackModification:
    """A command that changes the effective profile stack."""

    command: str                        # "SSL::disable", "HTTP::disable"
    side: str | None                    # "clientside", "serverside", or None (both)
    removes_profile: str | None = None  # profile type removed
    adds_profile: str | None = None     # profile type added (rare)
    requires_before_event: str | None = None  # must happen before this event
```

All properties use positive form.  `has_flow = True` means an active
traffic flow is present; `client_side = True` means the client-side
connection is available.  There are no negative-form stored sets like
`_non_flow` -- those are derived as needed by querying
`events where has_flow is False`.

#### ConnectionModel -- per-connection state

```python
@dataclass(slots=True)
class ConnectionModel:
    """Models a connection's profile state across its lifecycle."""

    initial_profiles: frozenset[str]    # from virtual server config
    current_stack: LayerStack           # active at current event
    modifications: list[StackModification] = field(default_factory=list)

    def available_namespaces(self) -> frozenset[str]:
        """Which protocol command namespaces are currently available."""
        ...

    def apply_modification(self, mod: StackModification) -> None:
        """Apply a dynamic profile change (SSL::disable, etc.)."""
        ...

    def profiles_at_event(self, event: str) -> frozenset[str]:
        """What profiles are effective when this event fires."""
        ...
```

#### Event graph model

```python
@dataclass(frozen=True, slots=True)
class EventNode:
    """A node in the event graph."""

    event: str
    side: str                           # "client", "server", "both", "global"
    profiles: frozenset[str]            # profiles active when this fires
    layer: str                          # which layer triggers this event
    multiplicity: str                   # "once_per_connection", "per_request"


@dataclass(frozen=True, slots=True)
class EventEdge:
    """An edge in the event graph (temporal ordering)."""

    source: str                         # preceding event
    target: str                         # following event
    conditional: bool = False           # only fires under conditions
    condition: str = ""                 # human-readable condition


@dataclass(slots=True)
class EventGraph:
    """The complete event flow graph for a profile combination."""

    nodes: dict[str, EventNode]
    edges: list[EventEdge]
    profiles: frozenset[str]            # virtual server profiles

    def reachable_after(
        self, event: str, modifications: list[StackModification] | None = None,
    ) -> frozenset[str]:
        """Events reachable after this one, considering modifications."""
        ...

    def valid_namespaces_at(
        self, event: str, modifications: list[StackModification] | None = None,
    ) -> frozenset[str]:
        """Protocol command namespaces valid at this event."""
        ...
```

### The NamespaceRegistry -- unifying facade

```python
@dataclass(slots=True)
class NamespaceRegistry:
    """Registry-backed namespace model.

    Combines:
    - Tcl namespace scope tracking (tier 1, all dialects)
    - F5 protocol namespace model (tier 2, iRules only)
    """

    _profiles: dict[str, ProfileSpec]
    _protocol_namespaces: dict[str, ProtocolNamespaceSpec]
    _modifications: dict[str, StackModification]

    @classmethod
    def build_default(cls) -> NamespaceRegistry: ...

    # ── Tier 1: Tcl namespace scope ──────────────────────────────────

    def tracker_for_file(
        self, source: str, *, filename: str | None = None,
        dialect: str = "tcl8.6",
    ) -> NamespaceTracker:
        """Create a namespace tracker for a file.

        For iRules dialect, attempts to detect the iRule name from:
        1. ``# irule: name`` pragma comment in source
        2. Filename heuristic (basename minus extension)
        """
        ...

    # ── Tier 2: F5 protocol namespaces (iRules only) ─────────────────

    # Profile queries
    def profile_for_namespace(self, prefix: str) -> frozenset[str]: ...
    def namespaces_for_profile(self, profile: str) -> frozenset[str]: ...
    def required_profiles(self, profile: str) -> frozenset[str]: ...

    # Event-to-profile mapping (wraps EventProps)
    def profiles_implied_by_event(self, event: str) -> frozenset[str]: ...
    def events_for_profile(self, profile: str) -> frozenset[str]: ...

    # Command-to-namespace mapping (wraps CommandSpec.event_requires)
    def namespace_for_command(self, command: str) -> str | None: ...
    def profiles_for_command(self, command: str) -> frozenset[str]: ...

    # Dynamic modification lookups
    def modification_for_command(self, command: str) -> StackModification | None: ...

    # Stack computation
    def initial_stack(self, profiles: frozenset[str]) -> LayerStack: ...
    def stack_after_modification(
        self, stack: LayerStack, mod: StackModification,
    ) -> LayerStack: ...

    # Virtual server model
    def virtual_server_model(
        self, profiles: frozenset[str],
    ) -> VirtualServerModel: ...
```

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

The `NamespaceTracker` with `irule_name = "utility_irule"` resolves:
- `proc log_request` -> `::utility_irule::log_request`
- `call log_request` -> first looks in current iRule's namespace, then global

Without a known iRule name (no pragma, unclear filename), the tracker
still works but can't resolve cross-iRule references -- procs are
treated as local.  This is a graceful degradation.

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

The `capabilities` field on `ProfileSpec` captures this:

```python
"CLIENTSSL": ProfileSpec("CLIENTSSL", layer="tls", side="client",
                         requires=frozenset({"TCP"}),
                         capabilities=frozenset({
                             "sni", "extensions", "sessionid",
                             "cipher", "cert", "tls_data",
                             "tls_control",  # enable/disable/renegotiate
                         })),
"SSL_PERSISTENCE": ProfileSpec("SSL_PERSISTENCE", layer="tls", side="client",
                               requires=frozenset({"TCP"}),
                               capabilities=frozenset({
                                   "sni", "extensions", "sessionid",
                               })),
```

Commands declare the capability they need rather than (or in addition to)
a profile name:

```python
# In EventRequires or the new CommandNamespaceRequires:
"SSL::sni":        requires_capability="sni"         # works with SSL_PERSISTENCE or CLIENTSSL
"SSL::extensions": requires_capability="extensions"   # works with SSL_PERSISTENCE or CLIENTSSL
"SSL::cipher":     requires_capability="cipher"       # needs full CLIENTSSL/SERVERSSL
"SSL::collect":    requires_capability="tls_data"     # needs full CLIENTSSL/SERVERSSL
```

The validation logic becomes:

```python
def command_valid_with_profiles(
    command_capability: str,
    active_profiles: frozenset[str],
    profile_specs: dict[str, ProfileSpec],
) -> bool:
    """Check if any active profile provides the required capability."""
    return any(
        command_capability in profile_specs[p].capabilities
        for p in active_profiles
        if p in profile_specs
    )
```

This means `SSL::sni` in `CLIENTSSL_CLIENTHELLO` is valid whether the VS
has a full CLIENTSSL profile *or* just an SSL persistence profile, because
both provide the `"sni"` capability.

#### SSL persistence events

With only an SSL persistence profile (no CLIENTSSL), only a subset of SSL
events fire:

| Event | Fires with SSL persistence? |
|-------|:--:|
| `CLIENTSSL_CLIENTHELLO` | yes |
| `CLIENTSSL_HANDSHAKE` | **no** |
| `CLIENTSSL_DATA` | **no** |

The `EventNode.profiles` field in the event graph needs to reflect this:
`CLIENTSSL_CLIENTHELLO` should list both `CLIENTSSL` and
`SSL_PERSISTENCE` as profiles that trigger it.

#### Impact on PROTOCOL_NAMESPACE_SPECS

The SSL protocol namespace becomes available with SSL_PERSISTENCE too:

```python
"SSL": ProtocolNamespaceSpec("SSL",
    profiles=frozenset({"CLIENTSSL", "SERVERSSL", "SSL_PERSISTENCE"}),
    layer="tls", side="both", side_selectable=True),
```

But individual SSL:: commands are gated by capability, not just namespace
availability.  The namespace is reachable (the prefix resolves), but
specific subcommands may produce diagnostics if the required capability
is missing.

### Profile requirements inference

Given an iRule using events `{HTTP_REQUEST, CLIENTSSL_HANDSHAKE,
SERVER_CONNECTED}` and commands `{HTTP::header, SSL::cert, pool}`:

```python
registry = NamespaceRegistry.build_default()
model = registry.virtual_server_model(frozenset())
needed = model.required_profiles_for_events(events)
# -> {"HTTP", "CLIENTSSL", "TCP"}

needed |= model.required_profiles_for_commands(commands)
# -> {"HTTP", "CLIENTSSL", "TCP"}  (SSL::cert needs CLIENTSSL/SERVERSSL)

needed = registry.resolve_profile_dependencies(needed)
# -> {"HTTP", "CLIENTSSL", "TCP"}  (HTTP requires TCP, already present)
```

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

The `NamespaceTracker` models this by tracking the scope stack as the
AST is walked.  It doesn't do full Tcl namespace resolution (that would
require runtime semantics), but it covers the common patterns:
`namespace eval`, `namespace import`, `proc` definitions, and qualified
command references.

## Integration with existing code

### Wrapping, not replacing

The `NamespaceRegistry` wraps the existing registries:

- `EventProps` / `EVENT_PROPS` -> indexed by profile metadata
- `EventRequires` on `CommandSpec` -> queried via `profiles_for_command()`
- `FlowChain` / `MASTER_ORDER` -> used by `ConnectionModel.profiles_at_event()`
- `BigipConfig.profiles` -> feeds `VirtualServerModel` construction

### New diagnostic opportunities

- **IRULE1010**: "command `SSL::cert` used after `SSL::disable`"
- **IRULE1011**: "virtual server needs CLIENTSSL profile for this iRule"
- **IRULE1012**: "HTTP::header in CLIENT_ACCEPTED -- HTTP events fire later"
- **IRULE1013**: "`SSL::disable serverside` must be before SERVER_CONNECTED"
- **IRULE1014**: "profiles HTTP and DNS are mutually exclusive -- disable
  one before its events fire"
- **IRULE1015**: "HTTP::header used after HTTP::disable in same connection
  flow"
- **IRULE1016**: "`SSL::cipher` requires a client-ssl or server-ssl profile
  (ssl persistence alone is not sufficient)"
- **TCL2010**: "proc `foo` defined in namespace `::bar` shadows import"

### Existing pragma format

The existing `# profiles: HTTP, CLIENTSSL` pragma (in `event_tree.py:
parse_profile_directive`) is extended with:

```
# irule: my_irule_name
# irule: /Common/my_irule_name
```

Same scan-first-20-lines convention.

## File layout

```
core/commands/registry/
    namespace_registry.py       # NamespaceRegistry facade
    namespace_models.py         # NamespaceScope, NamespaceTracker,
                                # ProfileSpec, ProtocolNamespaceSpec,
                                # LayerStack, ConnectionModel,
                                # StackModification, EventNode/Edge/Graph,
                                # VirtualServerModel
    namespace_data.py           # PROFILE_SPECS, PROTOCOL_NAMESPACE_SPECS,
                                # MODIFICATION_SPECS tables

docs/kcs/
    kcs-namespace-model-design.md   # this document
```

## Data tables

### PROFILE_SPECS

```python
PROFILE_SPECS: dict[str, ProfileSpec] = {
    "TCP":         ProfileSpec("TCP",       layer="transport",    side="both"),
    "UDP":         ProfileSpec("UDP",       layer="transport",    side="both"),
    "FASTL4":      ProfileSpec("FASTL4",    layer="transport",    side="both"),
    "CLIENTSSL":   ProfileSpec("CLIENTSSL", layer="tls",          side="client",
                               requires=frozenset({"TCP"}),
                               capabilities=frozenset({
                                   "sni", "extensions", "sessionid",
                                   "cipher", "cert", "tls_data", "tls_control",
                               })),
    "SERVERSSL":   ProfileSpec("SERVERSSL", layer="tls",          side="server",
                               requires=frozenset({"TCP"}),
                               capabilities=frozenset({
                                   "sni", "extensions", "sessionid",
                                   "cipher", "cert", "tls_data", "tls_control",
                               })),
    "SSL_PERSISTENCE": ProfileSpec("SSL_PERSISTENCE", layer="tls", side="client",
                               requires=frozenset({"TCP"}),
                               capabilities=frozenset({
                                   "sni", "extensions", "sessionid",
                               })),
    "HTTP":        ProfileSpec("HTTP",      layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "FASTHTTP":    ProfileSpec("FASTHTTP",  layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "DNS":         ProfileSpec("DNS",       layer="application",  side="both"),
    "SIP":         ProfileSpec("SIP",       layer="application",  side="both"),
    "FIX":         ProfileSpec("FIX",       layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "DIAMETER":    ProfileSpec("DIAMETER",  layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "MQTT":        ProfileSpec("MQTT",      layer="application",  side="both",
                               requires=frozenset({"TCP"})),
    "ASM":         ProfileSpec("ASM",       layer="security",     side="both",
                               requires=frozenset({"HTTP"})),
    "ACCESS":      ProfileSpec("ACCESS",    layer="security",     side="client",
                               requires=frozenset({"TCP"})),
    "BOTDEFENSE":  ProfileSpec("BOTDEFENSE",layer="security",     side="client",
                               requires=frozenset({"HTTP"})),
    "STREAM":      ProfileSpec("STREAM",    layer="acceleration", side="both",
                               requires=frozenset({"TCP"})),
    "WEBACCELERATION": ProfileSpec("WEBACCELERATION", layer="acceleration", side="both",
                               requires=frozenset({"HTTP"})),
    # ... remaining profiles
}
```

### PROTOCOL_NAMESPACE_SPECS

```python
PROTOCOL_NAMESPACE_SPECS: dict[str, ProtocolNamespaceSpec] = {
    "HTTP":   ProtocolNamespaceSpec("HTTP",  profiles=frozenset({"HTTP", "FASTHTTP"}),
                                    layer="application", side="both"),
    "SSL":    ProtocolNamespaceSpec("SSL",   profiles=frozenset({"CLIENTSSL", "SERVERSSL", "SSL_PERSISTENCE"}),
                                    layer="tls", side="both", side_selectable=True),
    "TCP":    ProtocolNamespaceSpec("TCP",   profiles=frozenset({"TCP"}),
                                    layer="transport", side="both", side_selectable=True),
    "UDP":    ProtocolNamespaceSpec("UDP",   profiles=frozenset({"UDP"}),
                                    layer="transport", side="both"),
    "IP":     ProtocolNamespaceSpec("IP",    profiles=frozenset({"TCP", "UDP"}),
                                    layer="transport", side="both", side_selectable=True),
    "LB":     ProtocolNamespaceSpec("LB",    profiles=frozenset(),
                                    layer="load_balance", side="global"),
    "DNS":    ProtocolNamespaceSpec("DNS",   profiles=frozenset({"DNS"}),
                                    layer="application", side="both"),
    "SIP":    ProtocolNamespaceSpec("SIP",   profiles=frozenset({"SIP"}),
                                    layer="application", side="both"),
    "ACCESS": ProtocolNamespaceSpec("ACCESS",profiles=frozenset({"ACCESS"}),
                                    layer="security", side="client"),
    "ASM":    ProtocolNamespaceSpec("ASM",   profiles=frozenset({"ASM"}),
                                    layer="security", side="both"),
    # ... remaining namespaces
}
```

### MODIFICATION_SPECS

```python
MODIFICATION_SPECS: dict[str, StackModification] = {
    "SSL::disable": StackModification(
        command="SSL::disable",
        side=None,          # resolved from arg: clientside/serverside
        removes_profile=None,  # resolved: CLIENTSSL or SERVERSSL per side
    ),
    "SSL::enable": StackModification(
        command="SSL::enable",
        side=None,
        adds_profile=None,  # resolved per side arg
    ),
    "HTTP::disable": StackModification(
        command="HTTP::disable",
        side=None,
        removes_profile="HTTP",
    ),
    "HTTP::enable": StackModification(
        command="HTTP::enable",
        side=None,
        adds_profile="HTTP",
    ),
}
```

## Discoverability

- [KCS index](README.md)
- [Command registry and event model](kcs-command-registry-event-model.md)
- [LSP feature providers](kcs-lsp-feature-providers.md)
