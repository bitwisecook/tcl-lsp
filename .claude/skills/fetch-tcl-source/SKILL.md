---
name: fetch-tcl-source
description: >
  Download and extract Tcl and Tk source trees (8.4, 8.5, 8.6, 9.0, 9.1) to
  tmp/ for test suites and bytecode reference. Idempotent — skips versions
  already present.
allowed-tools: Bash, Read
---

# Fetch Tcl Source

Fetches release tarballs from GitHub's codeload CDN (cached, no `.git`
metadata, kinder to upstream than `tcl.tk`) and extracts them under `tmp/`
(gitignored). Four attempts with exponential backoff, git-clone fallback.

```bash
bash .claude/skills/fetch-tcl-source/fetch_tcl_source.sh <cmd>
```

| Command | Fetches |
|---|---|
| `84` `85` `86` `90` `91` (or `8.4` …) | one Tcl tree: 8.4.20, 8.5.19, 8.6.16, 9.0.4, 9.1b0 |
| `tk84` … `tk91` | the matching Tk tree |
| `all` / `tkall` | every Tcl / every Tk tree |
| `status` | what is present |

Each tree is complete (`tests/`, `generic/`, `library/`, `doc/`, build files);
`tmp/tcl9.0.4/tests/` is the primary reference, `tmp/tcl9.1b0/` the beta.
Patchlevels and tags come from `rust/tcl-dialect/data/reference-toolchains.tsv`,
which the host installer and `tcl-dialect`'s release facts share — update that
manifest for a new patch release. Remote agent sessions run this from the
SessionStart hook; use it by hand before `scripts/capture/test_results.sh`,
`scripts/capture/bytecode.sh`, or a cross-version test-suite investigation.

$ARGUMENTS
