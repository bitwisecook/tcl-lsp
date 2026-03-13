---
name: irule-scaffold
description: >
  Generate an iRule skeleton from selected events. Creates a template
  with proper log gating, local variable extraction, and placeholder
  sections for each event handler.
allowed-tools: Bash, Read, Write
---

# iRule Scaffold

Generate an iRule skeleton from event names.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md`
2. Parse the event names from the user's request. Common events:
   - RULE_INIT — Runs once when the iRule loads. Initialise static:: variables here.
   - CLIENT_ACCEPTED — Fires when a client TCP connection is accepted.
   - HTTP_REQUEST — Fires for each HTTP request from the client.
   - HTTP_RESPONSE — Fires for each HTTP response from the server.
   - HTTP_REQUEST_DATA — Fires when HTTP request payload data is collected.
   - HTTP_RESPONSE_DATA — Fires when HTTP response payload data is collected.
   - SERVER_CONNECTED — Fires when a server-side connection is established.
   - CLIENT_CLOSED — Fires when the client connection closes.
   - SERVER_CLOSED — Fires when the server connection closes.
   - CLIENTSSL_HANDSHAKE — Fires when a client SSL handshake completes.
   - SERVERSSL_HANDSHAKE — Fires when a server SSL handshake completes.
   - DNS_REQUEST — Fires for DNS query requests.
   - DNS_RESPONSE — Fires for DNS query responses.
   - LB_SELECTED — Fires after a load balancing decision is made.
   - LB_FAILED — Fires when no pool member is available.
3. If no events specified, show the reference list above and ask which to include
4. Generate the skeleton following these conventions:
   - If any HTTP event is present, include a CLIENT_ACCEPTED block with `set debug 0` for log gating
   - In HTTP events, gate log calls with `if {$debug}` — never use static:: for this
   - In HTTP events, extract commonly-used values (HTTP::uri, HTTP::host, HTTP::method) into local variables at the top
   - Include commented placeholder sections (e.g. `# --- Request routing ---`) in each event
   - K&R brace style: `when HTTP_REQUEST {` on the same line
   - 4-space indentation
   - Add brief comments explaining when each event fires
5. Write the skeleton to a file
6. Validate with the analyser:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
7. Fix any issues and re-validate (up to 3 iterations)

$ARGUMENTS
