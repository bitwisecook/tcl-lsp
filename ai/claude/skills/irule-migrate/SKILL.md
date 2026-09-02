---
name: irule-migrate
description: "Convert nginx, Apache, or HAProxy configuration to an F5 BIG-IP iRule. Detects the source format and applies appropriate construct mappings. Use when migrating load balancer config to iRules, converting nginx rules to F5, translating Apache RewriteRule to iRules, or converting HAProxy ACLs to iRule logic."
allowed-tools: mcp__tcl-lsp__analyze, Read, Write
---

# iRule Migrate

## Steps

1. Read `../_prompts/irules_system.md`, then the source configuration.
2. Detect the format and map:
   - **nginx:** `location` → `switch -glob [HTTP::path]` or `class match`;
     `proxy_pass` → `pool`; `rewrite` / `return 301|302` → `HTTP::uri` /
     `HTTP::redirect`; `add_header` → `HTTP::header insert`; `if ($host)` →
     `string match` / `class match` on `[HTTP::host]`
   - **Apache:** `RewriteRule` → `HTTP::uri` / `HTTP::redirect`;
     `RewriteCond` → if/switch; `ProxyPass` → `pool`; `Header set` →
     `HTTP::header insert/replace`; `<VirtualHost>` → switch on
     `[HTTP::host]`; `<Location>` → switch on `[HTTP::path]`
   - **HAProxy:** `acl` → `if` / `class match`; `use_backend` → `pool`;
     `http-request redirect` → `HTTP::redirect`; `set-header` /
     `add-header` → `HTTP::header replace` / `insert`; `frontend bind` →
     the virtual server (note in comments)
3. Generate the iRule with security best practices and a comment on each
   mapping; note what has no direct translation (health checks, rate
   limiting).
4. Write the file, call `mcp__tcl-lsp__analyze` with the contents as
   `source`, fix and re-validate up to 5 iterations, then report what was
   migrated and what remains.

$ARGUMENTS
