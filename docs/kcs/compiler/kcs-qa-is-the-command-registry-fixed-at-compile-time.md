# KCS: Is the command registry the analyser reads fixed at compile time?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

Now that SpecTcl `.tclspec` packs can add commands at load time, can a
command-existence fact — W002, W123, hover, arity, anything the registry
answers — change while the process is running, and what does a compiler-fact
consumer need to do differently because of it?

## Answer

Before SpecTcl packs, a dialect's command registry was compiled-in data: one
registry per dialect profile, built once, alive for the process. A pass could
treat "unknown now" as "unknown for the life of this run" without checking
again. That assumption no longer holds. A workspace's `.tclspec` packs
(discovered at workspace, user, and bundled tiers, nearest wins) can layer
extra commands onto a profile's registry after the process has already
started, so "which commands exist" is now a question with an answer that can
change at runtime, not just at startup.

The plain, un-overlaid registry per profile is still exactly what it always
was: one closed set, built once, alive for the process. What changed is that
a profile can now also have a **layered** registry on top of it, identified
by the pair *(profile, overlay key)* — the overlay key is a content hash of
the loaded pack set, not a per-document or per-session number. The same pack
set always resolves to the same layered entry; a different pack set, even one
edited word in a `.tclspec` file, is a different key and a different entry.
Zero means "no packs".

The analyser only ever carries this key as an opaque number, arriving via its
configuration. It may **look the (profile, key) pair up — never build it**.
This is deliberate: building a layered registry needs the pack's actual
contents, and only the loader that parsed the `.tclspec` files has those; the
analyser is not allowed to depend on that loader. If a lookup-only miss were
allowed to build and cache a pack-less registry under the pack's key anyway,
that entry would be permanently wrong for the rest of the process — "packs
are installed under this key" and "this key answers with no packs" cannot
both be true once cached. So a miss just falls back to the plain, un-overlaid
profile registry: the same commands the profile answered with a moment ago,
before this particular pack set existed. That fallback is never an error
condition for a consumer to special-case — it is the honest pre-install
state.

Scope is the other half of what makes this cheap rather than a hot-path
problem. Packs install once at **workspace scope**, keyed by the pack set's
content hash, not per document: a pack is re-parsed and re-installed only
when the pack file itself changes, never when a document that merely calls
one of its commands is edited. This is a different mechanism from the
per-document overlay path stubs use, which does re-evaluate per edit. So
editing ordinary source that happens to call a vendor command touches
nothing in the registry; only editing the pack does. A name collision with a
shipped command loses to the shipped command unless the pack declares itself
an override, and both outcomes are reported rather than happening silently.

For a compiler-fact consumer, this means: W002 (command disabled in dialect)
and command-existence diagnostics can legitimately flip once a pack loads or
reloads, without a restart, in either direction. Commands from EDA vendor
libraries are pack-provided, not compiled in — they exist for a profile only
once that profile's pack has actually been installed. Anything that memoises
"is this command known" for longer than a single analysis pass needs to key
that memo on the same (profile, overlay key) pair, or it will go on answering
with a fact that used to be true.

One more thing worth knowing: a pack spec that declares a hook body (for
example a const-folder) dispatches through a sandboxed host that exists
**per thread**, built on that thread's first use. A thread that has not
built its host yet still abstains on that hook, exactly as it would with no
packs loaded at all — a hook-bearing spec's answer is not uniformly available
the instant the pack installs, the way a compiled-in spec's answer always
was.

Ownership follows from the same fact. A layered generation is not
process-lifetime data: the pack-carrying accessors hand back a
reference-counted **handle**, not a `'static` reference, and a superseded
generation is retired — its memory actually freed — once the cache has
dropped it and the last holder finishes. So a consumer keeps the handle it
was given for the duration of its own read (an analysis holds one for the
whole walk), and never stashes a bare reference somewhere that outlives it.
The plain, un-overlaid per-profile registries are still process-lifetime,
exactly as before. None of this changes the semantics above — identity,
lookup-only, and fallback hold regardless.

## Related

- [SpecTcl pack design](../../design/spec-packs.md) — the registry-layering
  rule, discovery tiers, and collision policy this note summarises.
- [Command registry](../../design/compiler/command-registry.md) — the full
  registry contract and pack integration.
- [How to write a SpecTcl pack](../kcs-howto-write-a-tclspec-pack.md)
- [Which commands are available in a dialect?](../kcs-qa-which-commands-are-available-in-a-dialect.md)
- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
