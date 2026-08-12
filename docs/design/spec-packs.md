# Spec packs — a loadable command database for private libraries

> **Status:** proposal, for discussion on
> [issue #1363](https://github.com/bitwisecook/tcl-lsp/issues/1363).
> Nothing here is implemented.

## The problem

Users with private Tcl libraries cannot contribute their command specs to
the shipped registry, and stubs deliberately carry only a fraction of what
a `CommandSpec` can say. They need a way to build a full command database
for their own code and load it into the server — without a Rust toolchain,
and without rebuilding it for every tcl-lsp release.

## Ship data, not code

The stable-ABI question answers itself once the artefact is data rather
than a dynamic library:

- A **cdylib** would pin us to a `repr(C)` mirror of the whole
  `CommandSpec` family — a huge surface that changes in most releases, so
  it recreates the recompile-every-release problem it was meant to solve.
  It also loads foreign code into the server (a sandboxing and trust
  problem), and cannot work at all in the WASM-hosted contexts (the spec
  studio, the explorer).
- A **data pack** has no ABI, only a file format — and format stability is
  a solved problem: version header, ignore-unknown, default-missing.

Most of the machinery already exists. The spec studio's draft model is a
complete data-only serialisation of a `CommandSpec`, keyed by Rust field
name, kept complete by the `coverage.rs` build gates; the compiler hooks
are already **typed IDs**, not function pointers, so they serialise as
names; and the fields that genuinely are function pointers
(`arg_role_resolver`, `const_fold`, gates) are exactly the ones the studio
already marks unrenderable — a pack simply cannot carry them, which is
also the security property: **a pack can never make the server execute
anything.**

## The format

One file, `<name>.tclspec` — JSON for now (packs are small; a binary
encoding is a later optimisation, not a format change):

```json
{
  "tclspec": 1,
  "generator": "spec-studio 2.x",
  "package": "mylib",
  "commands": [ { "name": "mylib::frobnicate", "arity": {…}, … } ]
}
```

Each entry is a studio draft: the same keys, the same catalogues
(traits, roles, dialects, hook IDs) spelled by name.

## Compatibility policy

- **Loading is tolerant by construction.** Unknown keys are ignored
  (a newer pack on an older server), missing keys default (an older pack
  on a newer server), and an unrecognised trait, role, or hook *name* is
  dropped with a logged notice while the rest of the spec loads. A pack
  therefore keeps working across ordinary releases with no rebuild.
- **`tclspec` major version** changes only on a semantic break — a field
  whose meaning changed, not one that was added. The server refuses a
  major it does not know, with a message naming the studio as the
  migration path (load the pack, re-render, download).
- The existing schema-coverage gates double as the format's change log: a
  new `CommandSpec` field cannot ship without a schema key, and the
  schema key **is** the pack key.

## Loading

- Discovery: a `tclLsp.specPacks` setting, plus auto-discovery of
  `*.tclspec` beside a `tclpkg.tcl` manifest and under a project's
  `.tcl-lsp/` directory.
- The pack layers into the registry the way the per-document stub overlay
  does, but at workspace scope, loaded once per profile and keyed by
  content hash. Drafts deserialise into owned specs (leaked once per
  unique content, so the `&'static` registry contract is unchanged).
- A pack never overrides a shipped spec silently: a name collision is
  reported, and the shipped spec wins unless the pack says `"override"`.

## Authoring

Both existing authoring paths gain a pack target:

- the **spec studio** adds "Download pack" beside the `.rs` renderer, so
  the whole browse–edit–import workflow already documented for
  contributions produces a private pack instead;
- the **`spec-author` skill** and a `tcl spec pack` CLI verb build one
  from a library's sources non-interactively.

## What a pack cannot say, and the escape hatch

Function-pointer behaviour stays out, by design. In practice the gap is
narrower than it looks: `repeated_args` covers strided layouts, options
carry roles at value positions, and clause lists / definer grammars are
shared *named* constants — so the pack can reference them by name the
same way it names hook IDs. Commands that still need a bespoke resolver
are contribution candidates, which is the boundary we want: private data
stays private, behaviour that needs code goes through review.
