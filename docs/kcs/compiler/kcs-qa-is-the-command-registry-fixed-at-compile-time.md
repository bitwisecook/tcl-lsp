# KCS: Is the command registry the analyser reads fixed at compile time?

> **Audience:** Contributor
> **Type:** Q&A

## Applies to

all-editors, tcl-lsp-cli, mcp

## Question

Can a command-existence fact — W002, W123, hover, arity, anything the
registry answers — change while the process is running, and what does that
mean for a compiler-fact consumer?

## Answer

It can. A workspace's SpecTcl `.tclspec` packs (discovered at workspace,
user, and bundled tiers, nearest wins) layer extra commands onto a dialect
profile's registry after the process has started, so "which commands exist"
has an answer that moves at runtime.

**Identity.** The plain, un-overlaid registry for a profile is one closed
set, built once, alive for the process. A profile can also carry a *layered*
registry on top of it, identified by the pair *(profile, overlay key)*. The
overlay key is a content hash of the loaded pack set, not a per-document or
per-session number: the same pack set always resolves to the same entry, and
one edited word in a `.tclspec` file is a different key. Zero means "no
packs".

**Lookup only.** The analyser carries the key as an opaque number arriving
through its configuration. It may look the pair up; it may never build it.
Building a layered registry needs the pack's contents, which only the loader
that parsed the `.tclspec` files holds, and the analyser must not depend on
that loader. A miss therefore falls back to the plain profile registry rather
than caching a pack-less entry under the pack's key — an entry that would be
permanently wrong for the rest of the process. That fallback is the honest
pre-install state, not an error condition to special-case.

**Scope.** Packs install once at **workspace** scope, keyed by the pack set's
content hash. A pack is re-parsed and re-installed only when the pack file
itself changes, never when a document that merely calls one of its commands
is edited. This is a different mechanism from the per-document overlay path
stubs use, which does re-evaluate per edit. A name collision with a shipped
command loses to the shipped command unless the pack declares itself an
override, and both outcomes are reported rather than silent.

**What a consumer must do.** W002 and command-existence diagnostics can
legitimately flip in either direction once a pack loads or reloads, without a
restart. Commands from EDA vendor libraries are pack-provided, not compiled
in, so they exist for a profile only once that profile's pack is installed.
Anything that memoises "is this command known" for longer than a single
analysis pass must key that memo on the same (profile, overlay key) pair.

A pack spec that declares a hook body — a const-folder, say — dispatches
through a sandboxed host that exists **per thread**, built on that thread's
first use. A thread that has not built its host yet abstains on that hook, so
a hook-bearing spec's answer is not uniformly available the instant the pack
installs.

**Ownership.** The pack-carrying accessors hand back a reference-counted
handle, not a `'static` reference; a superseded generation is retired, and
its memory freed, once the cache has dropped it and the last holder finishes.
Keep the handle for the whole of your own read — an analysis holds one for
the whole walk — and never stash a bare reference that outlives it. The
plain per-profile registries stay process-lifetime data.

## Related

- [SpecTcl pack design](../../design/spec-packs.md) — the registry-layering
  rule, discovery tiers, and collision policy this note summarises.
- [Command registry](../../design/compiler/command-registry.md) — the full
  registry contract and pack integration.
- [How to write a SpecTcl pack](../kcs-howto-write-a-tclspec-pack.md)
- [Which commands are available in a dialect?](../kcs-qa-which-commands-are-available-in-a-dialect.md)
- [KCS index](../README.md)
- [Glossary](../../GLOSSARY.md)
