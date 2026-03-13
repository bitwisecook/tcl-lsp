---
name: irule-migrate
description: >
  Convert nginx, Apache, or HAProxy configuration to an F5 BIG-IP iRule.
  Detects the source format and applies appropriate construct mappings.
allowed-tools: Bash, Read, Write
---

# iRule Migrate

Convert load balancer configuration from nginx, Apache, or HAProxy to an iRule.

## Steps

1. Read the domain knowledge from `ai/prompts/irules_system.md` (includes migration patterns)
2. Read the source configuration file
3. Detect the format and apply these mappings:

   **nginx:**
   - `location` -> `switch -glob [HTTP::path]` or `class match`
   - `proxy_pass` -> `pool <pool_name>`
   - `rewrite` -> `HTTP::uri` / `HTTP::redirect`
   - `add_header` -> `HTTP::header insert`
   - `return 301/302` -> `HTTP::redirect`
   - `if ($host)` -> `string match` or `class match` on `[HTTP::host]`

   **Apache:**
   - `RewriteRule` -> `HTTP::uri` / `HTTP::redirect`
   - `RewriteCond` -> if/switch conditions
   - `ProxyPass` -> `pool <pool_name>`
   - `Header set` -> `HTTP::header insert/replace`
   - `<VirtualHost>` -> `switch` on `[HTTP::host]`
   - `<Location>` -> `switch` on `[HTTP::path]`

   **HAProxy:**
   - `acl` -> `if` conditions or `class match`
   - `use_backend` -> `pool <pool_name>`
   - `http-request redirect` -> `HTTP::redirect`
   - `http-request set-header` -> `HTTP::header replace`
   - `http-request add-header` -> `HTTP::header insert`
   - `frontend bind` -> handled by virtual server (note in comments)

4. Generate the iRule following security best practices
5. Write to a file and validate:
   ```bash
   uv run --no-dev python ai/claude/tcl_ai.py diagnostics $FILE
   ```
6. Fix any issues and re-validate (up to 5 iterations)
7. Add comments explaining the mapping from the original config
8. Note anything that cannot be directly translated (e.g., backend health checks, rate limiting)

$ARGUMENTS
