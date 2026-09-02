---
name: fetch-tcl-source
description: >
  Download and extract Tcl source trees (8.4, 8.5, 8.6, 9.0, 9.1) to tmp/ for
  test suites and bytecode reference. Idempotent — skips versions already present.
allowed-tools: Bash, Read
---

# Fetch Tcl Source

Downloads Tcl release source tarballs from GitHub's codeload CDN
(`https://codeload.github.com/tcltk/tcl/tar.gz/refs/tags/<tag>`) and
extracts them to `tmp/` so test suites, bytecode snippets, and reference
data are available without bundling Tcl source in the repository. Tarballs
are preferred over `git clone` because they are CDN-cached, smaller on
disk (no `.git` metadata), and easier on the upstream Tcl project.

## Usage

```bash
bash .claude/skills/fetch-tcl-source/fetch_tcl_source.sh <version|all|status>
```

## Commands

| Command | What it does |
|---|---|
| `84` or `8.4` | Fetch Tcl 8.4.20 source |
| `85` or `8.5` | Fetch Tcl 8.5.19 source |
| `86` or `8.6` | Fetch Tcl 8.6.16 source |
| `90` or `9.0` | Fetch Tcl 9.0.4 source |
| `91` or `9.1` | Fetch Tcl 9.1b0 source |
| `all` | Fetch all five versions |
| `status` | Show which versions are present in `tmp/` |

## What it provides

After fetching, the following directories are available:

| Directory | Key contents |
|---|---|
| `tmp/tcl8.4.20/tests/` | Tcl 8.4 test suite (for reference capture) |
| `tmp/tcl8.5.19/tests/` | Tcl 8.5 test suite |
| `tmp/tcl8.6.16/tests/` | Tcl 8.6 test suite |
| `tmp/tcl9.0.4/tests/` | Tcl 9.0 test suite (primary reference) |
| `tmp/tcl9.1b0/tests/` | Tcl 9.1 test suite (beta — newest features) |

Each source tree also contains `generic/`, `library/`, `doc/`, and build files.

Tcl 9.1 is a beta (`9.1b0`). Patchlevels and source tags for every release
live in `rust/tcl-dialect/data/reference-toolchains.tsv`; the fetcher and
host dependency installer both consume that manifest.

## When to use

- At the start of a cloud session that needs Tcl test files
- Before running `scripts/capture/test_results.sh`
- Before running `scripts/capture/bytecode.sh`
- When investigating Tcl test suite behaviour across versions

## Notes

- `tmp/` is gitignored — downloads are local only
- Idempotent: re-running skips existing directories
- Downloads from `codeload.github.com` with retry logic (4 attempts,
  exponential backoff)
- Update `rust/tcl-dialect/data/reference-toolchains.tsv` when a new
  pinned patch release comes out; `tcl-dialect` validates and generates the
  ordered `TclVersion` release facts from that manifest

$ARGUMENTS
