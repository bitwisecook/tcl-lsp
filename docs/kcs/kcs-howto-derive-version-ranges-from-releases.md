# KCS: How do I derive version ranges from release history?

> **Audience:** User
> **Type:** How-To

## Applies to

tcl-lsp CLI, MCP, claude-skill

## Question

My package has several releases. How do I get a SpecTcl pack whose
`introduced_version` and `retired_version` fields describe when each
command actually arrived and left, instead of guessing from what the
newest release looks like?

## Before you start

A pack built from one snapshot ([importing a
package](features/kcs-feature-spec-studio.md#importing-a-package), or the
spec-author skill's ordinary run) can only describe what the sources look
like *now*. Stamping every command with the newest release's own version
would claim they all arrived there, which is almost never true. `tcl spec
import` reads **several** releases instead and only writes a lifecycle
field when two releases disagree about whether something exists.

## Answer

1. **Get the releases as labelled snapshots.** Three ways, in order of
   convenience:
   - **From GitHub, over the network** — `tcl spec import --github
     OWNER/REPO` enumerates the repository's tags and fetches each
     release's tarball. Narrow the set with `--tag-pattern GLOB` (`*`
     matches any run of characters, `?` matches one, the whole tag must
     match — `v*` keeps `v1.2` but not `1.2`) and `--limit N` to keep only
     the newest `N` matching releases. A tag maps to a version by dropping
     a leading `v`/`V` and a non-numeric project prefix (`tcllib-1.20` →
     `1.20`). Run once with `--list-tags` to print what would be fetched
     without fetching it. Set `GITHUB_TOKEN` in your environment if the
     unauthenticated rate limit (60 requests/hour/IP) is a problem; it is
     sent as a bearer token, and the standard proxy environment variables
     are honoured.
   - **From a local checkout, no network** — export one directory per
     release yourself (`git archive TAG | tar -x -C snapshots/VERSION`, or
     unpack an existing release archive), then pass each one with
     `--snapshot VERSION=PATH` (repeatable). `PATH` may be a directory, a
     `.zip`, or a `.tar.gz` — an archive needs no unpacking first.
   - **Already-local artefacts, from an AI agent** — the MCP `spec_import`
     tool takes `snapshots: [{version, path}]` naming local directories or
     archives, plus `dialect`, `package`, and `complete_history`. It has no
     fetcher by design, so point it at what already landed on disk (from
     `tcl spec import --github`, from `git`, or from anywhere else).
2. **Say whether the history is complete.** `--complete-history` (the MCP
   tool's `complete_history: true`) declares the snapshots are *every*
   release of the package — the only claim that makes presence in the
   earliest snapshot count as an introduction. It is off by default
   (spelled out explicitly with `--partial-history`, for a script that
   wants to say so): a wrongly-claimed `introduced_version` cannot be told
   apart from a correctly-derived one afterwards, so only pass it once
   you have checked the full tag list.
3. **Run the import.** With local snapshots:

   ```sh
   tcl spec import \
     --snapshot 1.0=snapshots/1.0 \
     --snapshot 1.2=snapshots/1.2 \
     --snapshot 2.0=snapshots/2.0 \
     --dialect tcl8.6 \
     --out mylib.tclspec
   ```

   With GitHub:

   ```sh
   tcl spec import --github tcltk/tcllib --tag-pattern 'tcllib-*' --limit 8 \
     --complete-history --out tcllib.tclspec
   ```

   The pack is written to `--out`, or printed to stdout when `--out` is
   omitted; add `--json` to get the same result as structured data
   (per-command ranges, warnings, and the pack source together) instead of
   the bare pack text.
4. **Read the evidence header before the body.** The rendered pack opens
   with `#` comment lines naming the releases analysed, whether the
   history was declared complete, every contradiction the derivation
   found (a command that vanishes and reappears leaves its lifecycle
   unbounded rather than guessed), any `version-gate:` note recording a
   per-argument-value fact the pack format cannot hold as a field yet, and
   any field the renderer could not carry. Read it before you read the
   commands — it is the audit trail for every range the pack claims.
5. **Validate the result** the same way as any other pack: run it through
   `mcp__tcl-lsp__spectcl_check` (see [how to write a SpecTcl
   pack](kcs-howto-write-a-tclspec-pack.md)).

The spec-author Claude Code skill drives this whole flow for you,
including the offline fallback in step 1 when you have a checkout but no
network — it runs the `git archive` loop itself and then calls `tcl spec
import --snapshot …`.

## How to tell it worked

The command's summary line on stderr names how many commands were seen,
across how many releases, and how many got an `introduced_version` or
`retired_version`. A command you know changed shape between two releases
should show up with a range that matches what you expect; one you are not
sure about is exactly what the evidence header's notes are for.

## Related

- [How to write a SpecTcl pack](kcs-howto-write-a-tclspec-pack.md)
- [How to create a command spec without knowing Rust](kcs-howto-create-command-specs-without-rust.md)
- [The Command Spec Studio](features/kcs-feature-spec-studio.md)
- [SpecTcl pack design](../design/spec-packs.md)
- [KCS index](README.md)
- [Glossary](../GLOSSARY.md)
