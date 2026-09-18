# KCS: feature — XC Translation

> **Audience:** User
> **Type:** Functionality

## Summary

Translate F5 BIG-IP iRules to F5 Distributed Cloud (XC) Terraform HCL, ves.io JSON, or pasteable Console documents, with a coverage report.

## Applies to

VS Code, JetBrains, Copilot Chat, MCP, Claude skill, transform, lowering

## Availability

| Context | How |
|---------|-----|
| VS Code command | `Tcl: Translate iRule to F5 XC` |
| VS Code command | `Tcl: Translate iRule to F5 XC (Console JSON)` |
| VS Code chat | `@irule /xc` |
| JetBrains action | `Translate iRule to F5 XC` |
| MCP | `xc_translate` tool |
| Claude Code | `/irule-xc` |
| LSP command | `tcl-lsp.xcTranslate` |

## Question

What does XC translation do, and how do I use it?

## How to use

Every entry point runs the same static translator and returns the same
result: a Terraform HCL document, a ves.io JSON API document, a coverage
percentage, and a per-command list of what was translated, what was only
partially translated, what has no XC equivalent, and what maps to a
separate XC feature.

### VS Code

Open an iRule file and run `Tcl: Translate iRule to F5 XC`. Two scratch
tabs open beside the editor — the Terraform HCL and the JSON API
configuration — and a notification reports the coverage with the
translatable and untranslatable counts. Nothing is written to disk; save
either tab yourself to keep it.

`Tcl: Translate iRule to F5 XC (Console JSON)` renders the same objects
for the Distributed Cloud Console instead. It opens one tab per object,
each holding a document you paste straight into that object's JSON
editor in the Console.

### VS Code chat

`@irule /xc` translates the iRule in the editor, or one you attach or
paste. It opens the same two scratch tabs, lists the untranslatable and
advisory constructs in the chat, and when coverage is below 100 % asks
the model to suggest XC alternatives for the gaps.

### JetBrains

Run the `Translate iRule to F5 XC` action on an open iRule. The result
opens in a scratch file.

### MCP and Claude Code

The `xc_translate` tool takes `source` and an optional `output_format`
(`terraform`, `json`, or `both` — the default), and returns both
documents with the coverage breakdown. `/irule-xc` calls that tool and
writes `$FILE.tf` and `$FILE.xc.json`.

## Options

- `output_format` — `terraform`, `json`, `console`, or `both`. Defaults
  to `both`, which is Terraform plus JSON API; an unrecognised value is
  treated as `both` too. `console` returns `console_objects` instead: one
  entry per object, each with its `object_type`, the derived XC `name`,
  the `source_path` it came from, the `namespace`, and the `document` to
  paste.

## Operational context

The translator walks the lowered IR of each event handler and maps
commands to XC routes, service policy rules, origin pool references,
header actions, and WAF exclusion rules. That model is rendered two
ways: Terraform HCL for the F5 XC Terraform provider, and ves.io
JSON-API objects. The Console rendering is a third view of those same
objects, split one document per object because the Console configures
one object at a time. Each is the `{metadata, spec}` create request its
schema defines, with the metadata block the Console shows — `name`,
`namespace`, `labels`, `annotations`, `description`, `disable` — and
nothing the editor would reject. The coverage report is never mixed in. The Terraform carries `TODO` comments where XC needs a value
the iRule cannot supply, such as origin server addresses and
load-balancer domains. A command the translator has an entry for is
always reported — mapped to its XC construct, or listed as having no XC
equivalent — and the same analysis drives the XC100-301 diagnostics
shown inline on iRule files.

## Failure modes

- **Nothing happens on an empty file.** The command needs a non-empty
  iRule; a blank buffer returns no result.
- **Coverage below 100 %.** Procedural logic, L4 events, and `table` or
  `session` state have no static XC equivalent. The items list names
  each one and its XC-side alternative, such as App Stack, Rate
  Limiting, or Bot Defence.
- **A command the translator has no entry for is dropped without an
  item.** It counts neither for nor against coverage, so a run can
  report 100 % with such a command in the iRule. Read the generated
  configuration against the source rather than trusting the percentage
  alone.
- **The generated Terraform does not apply as-is.** Every `TODO` in the
  output marks a value you must supply before `terraform apply`.
- **XC rejects the emitted configuration.** The translator renders what
  the iRule says; it does not validate against a live tenant, so an
  object that clashes with existing XC configuration fails on apply.

## Example

### Before (iRule)

```tcl
when HTTP_REQUEST {
    if { [HTTP::uri] starts_with "/api" } {
        pool api_pool
    }
}
```

### After (Terraform HCL, abridged)

```hcl
resource "volterra_origin_pool" "api_pool" {
  # Translated from BIG-IP api_pool
  name      = "api-pool-2275b8b6"
  namespace = "default"

  # TODO: Configure origin servers
  origin_servers {
    public_name {
      dns_name = "example.com"  # TODO: Set actual server address
    }
  }

  port                  = 80
  endpoint_selection     = "LOCAL_PREFERRED"
  loadbalancer_algorithm = "LB_OVERRIDE"
}

resource "volterra_http_loadbalancer" "translated-lb" {
  name      = "translated-lb"
  namespace = "default"

  simple_route {
    path {
      prefix = "/api"
    }
    origin_pools {
      pool {
        name      = volterra_origin_pool.api_pool.name
        namespace = volterra_origin_pool.api_pool.namespace
      }
    }
  }
}
```

The JSON API document carries the same route and origin pool under
`http_loadbalancer` and `origin_pools`, and the run reports
`Coverage: 100.0% — 2 translatable, 0 partial, 0 untranslatable, 0 advisory`.

### After (Console document, the origin pool)

```json
{
  "metadata": {
    "annotations": {},
    "description": "Translated from BIG-IP api_pool",
    "disable": false,
    "labels": {},
    "name": "api-pool-2275b8b6",
    "namespace": "default"
  },
  "spec": {
    "endpoint_selection": "LOCAL_PREFERRED",
    "loadbalancer_algorithm": "LB_OVERRIDE",
    "origin_servers": [{ "public_name": { "dns_name": "example.com" } }],
    "port": 80
  }
}
```

Paste that into the origin pool's JSON editor in the Console, then do
the same with the load balancer document from its own tab.

The name is derived, not copied. XC names must follow DNS-1035, and a
BIG-IP path is not one, so `pool /Common/web-pool` becomes a name that
keeps the partition — the same pool name in another partition stays a
separate object — and the path itself is recorded in the description.
Every rendering derives the name identically, so the Terraform and the
Console documents configure the same object.

A pattern with no direct equivalent — a `HTTP::header insert` that
mutates response headers, say — is counted as untranslatable and listed
with the command that produced it.

## Related

- [XC translation output contract](../../design/f5/xc-translation-output-contract.md)
- [KCS feature index](README.md)
- [Glossary](../../GLOSSARY.md)
