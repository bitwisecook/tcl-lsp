# Original Jim namespace subcommand table

The question is how the original namespace parser separates exact lookup,
abbreviation, cache installation, public arity and helper lookup. These are
original C API observations, using `Jim_RegisterCoreCommands`, the original
`Jim_InitStaticExtensions` and `Jim_EvalNamespace`. No Tcltest harness is used.

Both captures identify upstream and local Jim revision
`5bac7c99ad65864c87da513e22e2f01703fa4e03` at execution time, reporting
`0.84-9-g5bac7c9`. The compiler is Debian cc 14.2.0-19. Each manifest retains
247 unchanged provider source, configuration, library and executable hashes,
exact compile/run commands, driver/source hashes and raw channels. Compilation
and execution exit zero without stderr or timeout. This does not establish a
later latest-upstream revision, another build or another provider.

`capture01` contains 1,992 rows for 71 cases in cold/reuse phases. `capture02`
adds a reached head-getter NUL control: 2,023 rows for 72 cases in 144 phases.
Every predecessor row is retained byte-exactly, recorded in `retention.json`.
The first 60 cases exercise all 15 original table members with zero through
three original integer operands. Their results distinguish parser arity failures
from subsequently reached helper/body errors; a helper error does not establish
that public arity rejected an invocation.

Measured results include:

- Accepted abbreviated `upv` and exact `upvar` with a NUL suffix both use the
  canonical public synopsis `namespace upvar ns ?arg ...?` on missing operands.
- Abbreviated `upv` with a NUL suffix is unknown. Empty and `c` selectors are
  ambiguous. Accepted `co` with an operand reaches literal `namespace co`
  helper lookup and fails there, independently of source metadata restrictions.
- The original member becomes `subcmd-lookup` before public arity rejection;
  that primary is retained on reuse. Public arity results have the original
  `string` primary before their result getter.
- A renamed command supplies its original invoked head for arity. A controlled
  selector getter restores a genuine custom original head updater; its reached
  arity getter observes the already published `wrong # args: should be "`
  prefix. The COLD NUL-head control then appends only the head's CString extent.
  Its REUSE invocation fails command lookup before arity because the retained
  head now contains the NUL suffix. The separate ordinary NUL-head control
  likewise never reaches the parser. Those failures are not arity observations.

The probe samples headers before result getters. It does not identify private
native table addresses, prove every cache branch, or observe destructor/final-free
chronology. The original interpreter and library are the provider; Runtime and
VM acceptance require separate exact controls.

`transport.json` records byte-exact copies of both original captures. Original
receipt and invocation paths remain unchanged; each `probe.elf` is the captured
executable originally named `probe`. Each capture retains its exact C source and
capture driver. The top-level driver can create a fresh output directory with:

```sh
python3 capture.py --sdk-dir /absolute/original/jimtcl --out /fresh/capture
```

The driver checks upstream/local identity and preserves setup failures separately
from observed completions. A new run has its own receipts and does not replace
either retained capture.
