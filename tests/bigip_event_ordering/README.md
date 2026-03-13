# BigIP Event Ordering Test Infrastructure

Empirically discover and verify the firing order of iRules events on a
real BIG-IP device using High-Speed Logging (HSL).

## Overview

Each **scenario** defines a virtual server configuration (profiles, VIP,
pool member) and its expected event ordering from `namespace_data.py`.
The generators produce:

- An **iRule** that instruments every event with a counter and timestamp,
  sending structured logs via `HSL::send` to a UDP receiver
- A **tmsh** config to deploy the virtual server, pool, HSL pool, and iRule
- **Python server** scripts to act as pool members
- **Python client** scripts to drive connections
- A **log receiver** (`hsl_receiver.py`) that collects HSL output
- A **graph generator** for visualizing event flow

## Quick Start (Automated Harness)

The `harness.py` script orchestrates the full cycle:

```bash
cd tcl-lsp
python tests/bigip_event_ordering/harness.py \
    --bigip-host 10.1.1.1 --bigip-user admin --bigip-pass admin \
    --vip-ip 10.1.1.10 --server-ip 10.1.1.100 \
    --hsl-receiver-ip 10.1.1.200 \
    [--scenarios plain_tcp,tcp_http] \
    [--graph ascii]
```

This will:
1. Generate self-signed certificates
2. Generate iRules and tmsh configs
3. Deploy to the BigIP via SSH
4. Start the HSL log receiver
5. Start pool member servers
6. Run client scripts
7. Parse logs and display results with event flow graph
8. Optionally cleanup (`--cleanup`)

Requires `sshpass` for non-interactive SSH.

## Manual Step-by-Step

### 1. Generate artifacts

```bash
cd tcl-lsp
python tests/bigip_event_ordering/generate_all.py \
    --vip-ip 10.1.1.10 --server-ip 10.1.1.100 \
    --hsl-receiver-ip 10.1.1.200
```

This writes `.irul` and `.tmsh` files to `tests/bigip_event_ordering/output/`,
including `hsl_pool.tmsh` for the HSL logging pool.

### 2. Generate certificates

```bash
python tests/bigip_event_ordering/cert_gen.py --output-dir certs
```

### 3. Start the HSL log receiver

On the machine that will receive logs (the `--hsl-receiver-ip` host):

```bash
python tests/bigip_event_ordering/hsl_receiver.py \
    --port 5514 --output events.log
```

### 4. Start the pool member server

On the machine that will act as the pool member:

```bash
# Plain TCP scenario
python tests/bigip_event_ordering/server_tcp.py --port 9080

# HTTP scenarios
python tests/bigip_event_ordering/server_http.py --port 9081

# HTTPS scenarios
python tests/bigip_event_ordering/server_https.py --port 9443
```

### 5. Deploy to BigIP

```bash
# Deploy HSL pool first (shared by all scenarios)
scp output/hsl_pool.tmsh admin@bigip:/tmp/
ssh admin@bigip 'tmsh -c "source /tmp/hsl_pool.tmsh"'

# Deploy a scenario
scp output/tcp_clientssl_serverssl_http.tmsh admin@bigip:/tmp/
ssh admin@bigip 'tmsh -c "source /tmp/tcp_clientssl_serverssl_http.tmsh"'
```

### 6. Run the client

```bash
python tests/bigip_event_ordering/client_https.py \
    --host 10.1.1.10 --port 8443 --count 3
```

### 7. Parse logs and generate graph

```bash
python tests/bigip_event_ordering/parse_logs.py \
    --file events.log --compare --graph ascii
```

Graph formats: `ascii`, `mermaid`, `dot` (Graphviz).

## Architecture

### HSL Logging

Events are logged via `HSL::send` to a UDP pool (`evt_order_hsl_pool`)
pointing at the log receiver. This avoids syslog rate-limiting and
provides real-time log delivery.

The HSL handle is opened once in `RULE_INIT`:
```tcl
set static::hsl [HSL::open -proto UDP -pool evt_order_hsl_pool]
```

### Event Priority

Within a single event, multiple `when` handlers may exist across
different iRules on the same virtual server. Execution order:

1. **Lowest priority number fires first** (32-bit unsigned int, default 500)
2. **Equal priorities**: ASCII alphanumeric sort of the iRule name

The generated iRules use `priority 1` to ensure they fire before any
user iRules.

### BIG-IP 14.1+ Processing Order

```
IP Intelligence → AFM/L2-L4 DoS → L2-L4 policies → L2-L4 iRules →
L7 policies → ASM → L7 iRules → L7 DoS → Proactive Bot Defense
```

ASM event mode affects ordering:
- **Normal mode**: `ASM_REQUEST_DONE` fires after every request (pre-LB)
- **Compatibility mode**: `ASM_REQUEST_VIOLATION` fires only for violations (pre-LB)

## Scenarios

| Scenario | Profiles | Client | Server |
|----------|----------|--------|--------|
| `plain_tcp` | TCP | `client_tcp.py` | `server_tcp.py` |
| `tcp_http` | TCP, HTTP | `client_http.py` | `server_http.py` |
| `tcp_clientssl_http` | TCP, ClientSSL, HTTP | `client_https.py` | `server_http.py` |
| `tcp_clientssl_serverssl_http` | TCP, ClientSSL, ServerSSL, HTTP | `client_https.py` | `server_https.py` |
| `tcp_clientssl_serverssl_http_collect` | TCP, ClientSSL, ServerSSL, HTTP | `client_https_post.py` | `server_https.py` |
| `udp_dns` | UDP, DNS | dig / nslookup | named / dnsmasq |

## Log Format

Each event logs a structured line via HSL:

```
EVTORD scenario=<name> sid=<session_id> seq=<counter> t=<ms_since_accept> event=<EVENT_NAME>
```

- `scenario`: Which test scenario produced this log
- `sid`: 9-char MD5 hash identifying the connection (client IP + port + random)
- `seq`: Monotonic counter (0 = first event)
- `t`: Milliseconds elapsed since CLIENT_ACCEPTED (or DNS_REQUEST for UDP)
- `event`: The iRules event name

## Cleanup

```bash
# Remove scenario objects
ssh admin@bigip 'tmsh -c "delete ltm virtual evt_order_<scenario>_vs; \
    delete ltm rule evt_order_<scenario>; \
    delete ltm pool evt_order_<scenario>_pool"'

# Remove HSL pool
ssh admin@bigip 'tmsh -c "delete ltm pool evt_order_hsl_pool"'
```

Or use `harness.py --cleanup` to remove all objects after testing.
