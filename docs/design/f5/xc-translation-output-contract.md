# XC translation output contract

How `f5-xc` renders one translation, and the naming rule every rendering
shares. The translator itself — how iRule commands map to XC constructs — is
not this file's subject; this is what a caller receives and what it may rely
on.

## Three renderings, one translation

`translate_irule` produces an `XCTranslationResult`. Three functions render
it, and every caller reaches them through `report::translation_payload`:

| Function | Shape | For |
|---|---|---|
| `terraform::render_terraform` | One HCL document | `terraform apply` against the F5 XC provider |
| `json_api::render_json` | One value bundling every object plus the coverage summary | An automated caller reading the whole translation |
| `json_api::render_console_objects` | One document per object | Pasting into the Distributed Cloud Console |

`OutputFormat` selects between them from a client's `output_format`
argument. `Both` means Terraform plus the JSON bundle, and keeps that
meaning: the Console rendering is opt-in under `Console`, so a caller that
asked for everything before the rendering existed receives what it always
did.

## Console documents

The Console configures one object at a time and its JSON editor takes the
object alone, so a bundle is unusable there. Each `ConsoleObject` carries
its `object_type` (which names the Console screen), the derived `name`, the
`source_path` it came from, the `namespace`, and a `document`.

The document is the object's create request as its schema defines it:

```
{ "metadata": { … }, "spec": { … } }
```

Exactly those two keys. The metadata is `ves.io.schema.ObjectCreateMetaType`
— `name`, `namespace`, `labels`, `annotations`, `description`, `disable` —
and nothing else; a key the editor does not expect is a paste that fails.
The coverage report never travels inside a document. It rides alongside in
the payload, where a client reads it for the notification it shows.

The spec bodies are the ones `render_json` emits. There is one spec per
object type, not one per rendering.

## Object names

`ObjectCreateMetaType.name` is required and must follow DNS-1035: a leading
lowercase letter, then lowercase letters, digits and hyphens, 63 characters
at most. A BIG-IP object path satisfies none of that — `/Common/web-pool`
leads with a separator and carries two — so no rendering may pass a path
through as a name. `names::xc_object_name` is the one derivation, and every
rendering uses it, including for the references between objects.

The rule, in order:

1. A name already in DNS-1035 form is returned unchanged, so an unqualified
   `web-pool` stays itself.
2. Anything else folds to lowercase, with every character outside the
   alphabet becoming a hyphen. Runs of hyphens collapse and the ends are
   trimmed. The partition therefore survives into the name rather than being
   stripped, so the same pool name in two partitions stays two objects.
3. A result that does not begin with a letter gains an `xc-` prefix, which a
   path of digits or of nothing else legal would otherwise fail on.
4. A hash of the original path is appended whenever the name had to change,
   because the fold alone is lossy: `web_pool` and `web-pool` fold alike and
   would otherwise become one object. This mirrors how `hcl_ident`
   disambiguates Terraform resource labels.
5. The name is truncated to fit the 63-character limit with its suffix.

Nothing about the original is lost. An object that stands for a named BIG-IP
object carries `Translated from BIG-IP <path>` in its description; an object
the translation invents — the load balancer and its service policy answer to
no BIG-IP object — says so instead, rather than naming a source that does
not exist.

Terraform resource *labels* are a separate alphabet with a separate
function: `terraform::hcl_ident` produces a valid HCL identifier, which
permits `_` where DNS-1035 does not. A label is not an object name and the
two must not be confused.

## What a change here must preserve

- A Console document has exactly `metadata` and `spec`, and its metadata has
  exactly the six fields above.
- Every rendering derives object names through `xc_object_name`, and a
  reference to an object uses the same derived name as that object's own
  metadata.
- `Both` does not start returning Console documents.
- The derivation is deterministic: the same path yields the same name across
  runs and across renderings.
