# Differential-audit corpus (issue #923)

A reusable research corpus: 130 tricky Tcl patterns mined from eight
real-world Tcl projects, each differentially audited against a real C Tcl
interpreter, plus the scripts that produced them. The corpus is kept
because re-mining and re-auditing it is expensive; the audit's own findings
were triaged into GitHub issues, all of which are now closed.

Nothing here is a work tracker. Treat it as input data for the next
differential-testing pass, not as a to-do list.

## The three-way differential method

The method the corpus was built with, and the one to follow when adding to
it:

1. Mine a real, tricky pattern from an actual corpus file — never an
   invented one.
2. Reduce it to a minimal, faithful `.tcl` repro.
3. Run the repro under a real C Tcl interpreter — `tclsh9.0` by default,
   `tclsh8.6` when the behaviour is version-sensitive — to establish
   ground truth. Never assume Tcl semantics; verify them.
4. Run the identical file through the built `tcl-lsp-server` over real LSP
   JSON-RPC (the `lsp-client` skill drives this).
5. Diff oracle against LSP behaviour and classify the result: **CONFIRMED**
   (provably diverges from tclsh), **REFUTED** (the LSP is correct, or the
   pattern does not reproduce), **PLAUSIBLE**, or **INCONCLUSIVE**.

A fix that comes out of a CONFIRMED finding is registry-driven — driven by
`tcl_registry::CommandSpec` / `SubCommand` data, hooks, or traits, or by
analysis state already recorded by an earlier pass. String branching on a
command name (`if cmd_name == "foo"`) in the analyser or compiler is not an
acceptable shape for one.

## Oracle environment

None of this survives a fresh container.

```sh
# Tcl 9.0.4 oracle, built from source into the gitignored tmp/ tree.
mkdir -p tmp && cd tmp
git clone --branch core-9-0-4 https://github.com/tcltk/tcl tcl9.0.4
cd tcl9.0.4/unix && ./configure && make -j

# Run it straight out of the build dir — no `make install` needed.
cat > /usr/local/bin/tclsh9.0 <<'EOF'
#!/bin/bash
export LD_LIBRARY_PATH="$PWD/tmp/tcl9.0.4/unix:${LD_LIBRARY_PATH:-}"
exec "$PWD/tmp/tcl9.0.4/unix/tclsh" "$@"
EOF
chmod +x /usr/local/bin/tclsh9.0

# Tcl 8.6 oracle — the distribution package already provides tclsh8.6.
apt-get install -y tcl8.6

# The server and CLI under test.
cargo build -p tcl-lsp-server -p tcl-cli
```

The `fetch-tcl-source` skill automates the source-tree half of this for
every supported version.

## Corpora

The eight projects the patterns were mined from, needed only to mine new
findings:

| Project | Why it is in the set |
|---|---|
| `tcltk/tcllib` (2.0) | The broadest body of idiomatic library Tcl there is. |
| `tcltk/tk` | Megawidget metaclasses, `namespace upvar`, `::tcl::OptProc`. |
| `georgtree/SpiceGenTcl` | Heavy TclOO, `mixin`, cross-file class hierarchies. |
| `georgtree/argparse` | Dynamic variable creation, `upvar`, `subst`-driven locals. |
| `georgtree/tclopt` | TclOO with `mixin`-only method resolution. |
| `nico-robert/ticklecharts` | Dynamic method installers, ensemble `-map`. |
| `nico-robert/pix` | `namespace eval $ns` remapping, `load`-only packages. |
| `nico-robert/tomato` | Copy constructors, `::tcl::mathfunc` injection. |

## `data/`

| File | Contents |
|---|---|
| `01-mined-findings-per-corpus.json` | Raw mining output, one entry per corpus. |
| `02-mined-findings-flattened-130.json` | The same 130 candidates as one flat list; the index numbering every other file uses. |
| `03-tcllib-audit-input-25.json` | The 25 tcllib candidates (flat indices 105–129). |
| `04-tcllib-audit-results-COMPLETE-25of25.json` | All 25 audited: 22 CONFIRMED, 3 REFUTED. |
| `05-main-audit-input-105.json` | The 105 candidates from the other seven corpora (flat indices 0–104). |
| `06-main-audit-results-COMPLETE-105of105.json` | All 105 audited: 85 CONFIRMED, 20 REFUTED. |
| `06-main-audit-results-PARTIAL-49of105.json` | Superseded by the COMPLETE file above. |
| `07-remaining-tcllib-findings-14.json` | Per-finding detail for 14 tcllib findings: summary, failure scenario, oracle output, LSP output, root-cause hint. |
| `08-research-plans-PARTIAL-8of14.json` | Code-verified fix plans for 8 of those 14. |

Every finding entry carries a `root_cause_hint` with file and line
pointers. Those pointers were accurate when the audit ran and the codebase
has moved since — re-verify against current code before trusting one.
The repro `.tcl` files themselves were scratchpad-only and are gone; the
hints are detailed enough to rebuild one.

## `scripts/`

Node scripts, run with the corpora checked out beside them:

| Script | Role |
|---|---|
| `01-mine-tricky-tcl-patterns.js` | Walks each corpus and extracts candidate patterns. |
| `02-differential-audit-tcllib-COMPLETE.js` | Drives the oracle-vs-LSP diff over the tcllib wave. |
| `03-differential-audit-main105-IN_PROGRESS.js` | The same driver for the main wave. |
| `04-remaining14-research-IN_PROGRESS.js` | Re-checks a root-cause hint against current code. |

The `IN_PROGRESS` / `COMPLETE` suffixes are historical filenames, not a
statement about the scripts themselves — all four run.

## Related

- [`docs/design/README.md`](../README.md) — the design-doc index.
- The `fuzz-findings` skill — the native differential fuzzer, which covers
  the runtime-semantics half of the same question the LSP half is covered
  here.
