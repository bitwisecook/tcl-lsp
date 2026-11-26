# Dialects, libraries and environments: the target model

Status: designed 26 November 2026, owner direction. Plan items 266–272 carry
it, and the crate layout is in [registry split](registry-split-design.md).

This document supersedes the classification in
[the registry model](dialect-and-package-registry-redesign.md) §0 and §2 where
they disagree: iRules and iApps are libraries, not dialect families. That
document stays the reference for evidence, detection and the analysis world.

## The model

There are three kinds of thing, and nothing is both.

1. **A dialect is a fundamental language.** It is a *family* at a *release*,
   under a *build*.
   - It owns everything that changes how source is read and evaluated: the
     lexer and expression grammars, numerals, escapes, the character model, the
     release semantics of core commands, and the core command surface.
   - The families are:

     | Family | Releases | Builds |
     |---|---|---|
     | `tcl` | 8.4, 8.5, 8.6, 9.0, 9.1 | canonical |
     | `jim` | 0.76–0.84 | `full`, `minimal`, plus capability flags for its optional extensions (`binary`, `ensemble`, `json`, `tclprefix`, `zlib`) |
     | `f5tcl` | the BIG-IP releases, its real version axis | `tmm` (the traffic interpreter that runs iRules) and `scriptd` (the configuration interpreter that runs iApps and tmsh scripts) |

   - A grammar difference that belongs to one F5 interpreter, such as
     `irules_brace_separator`, is a property of the `tmm` build, not of a
     library.
2. **A library is a provider of commands on top of dialects.** Tk, the
   libraries Tcl bundles (http, msgcat, tcltest, …), each tcllib module, Expect,
   Itcl, Thread, iRules, iApps, tmsh and the EDA vendor libraries are all
   libraries. A library has:
   - **Its own version axis, with trains.** For example, Itcl 3.x and 4.x;
     `struct::graph` 1.x and 2.x.
   - **Compatibility rows.** Each row says that versions R of this library work
     on family F at releases S, optionally under a build or capability
     predicate.
     - Rows are a disjunction; each row is a conjunction.
     - Examples: "Itcl 3.4 on tcl 8.4–8.5", "Itcl 4.x on tcl 8.6–9.1",
       "Thread 2.8 on tcl 8.6", "Thread 3.0 on tcl 9.0–9.1",
       "iRules (tracks host) on f5tcl, any release, build tmm".
   - **Requirements on other libraries,** with ranges on each side. For
     example, "tcllib's `snit` 2.x requires Tcl 8.5 and TclOO" or "Tk 8.6
     requires tcl 8.6".
   - **Commands.** Each command, subcommand and option carries a window on the
     library's own axis (introduced, deprecated, retired). It may add a
     dialect constraint where a library command depends on the host release.
     The two conjoin.
   - **Tracking.** A library may *track its host*: its version is the
     dialect's release. iRules does this; the iRules surface of BIG-IP 17.1 is
     iRules 17.1.
3. **An environment is a selectable combination.** It is a dialect selector
   (family, a release or a release range, a build) plus libraries. Each library
   has a version range and a placement:
   - **ambient:** loaded without `package require`;
   - **hosted:** available once required;
   - **bundled:** shipped with the dialect release, at the version that release
     bundles.

   It also carries the policy (open or closed world), the editor identity,
   detection rows and aliases. `tcl8.6`, `wish` (tcl + Tk ambient),
   `f5-irules` (f5tcl/tmm + iRules ambient), `f5-iapps` (f5tcl/scriptd + iApps
   and tmsh), `expect` and the EDA shells are environments.

Resolution for a document runs in three steps:

1. **Environment.** The environment, after detection and overrides, gives the
   dialect point or target range.
2. **Library versions.** Each library's candidate version set is the
   intersection of three things:
   - its placement range;
   - its compatibility rows against the dialect;
   - the `package require` constraints found in the code, with lower and upper
     bounds and multi-train selection.
3. **Visible commands.** A command is visible when its window intersects its
   library's candidate set and its dialect constraint holds.

Answers over a target range are sets, as today (`tclLsp.targets`).

The lexer and grammar-level consumers take the dialect only. Visibility,
version gating, closed-world policy and editor features take the resolved
environment.

### Bundled libraries

A library a dialect release ships with gets an ambient or bundled placement in
that release's environments, at the bundled version. The bundled versions come
from each pinned source tree's `pkgs/package.list.txt` and the library
directory. That table is generated with the release facts (items 79 and 202),
never written by hand.

TclOO is the worked example:
- It is a library on tcl 8.5 (the `TclOO` package).
- It is part of the core surface from 8.6 on.
- So it is a library whose compatibility row ends at 8.5, plus core commands
  from 8.6.

## What changes from today

| Today | Target |
|---|---|
| `Family { Tcl, F5Tcl, F5Irules, Jim }`; F5 has one release, `tmos`/`tmm`; BIG-IP versions sit on a keyed side axis defaulting to 16.1.0 | `{ tcl, jim, f5tcl }`; f5tcl's releases are BIG-IP releases; `tmm` and `scriptd` are builds |
| iRules is a core family for its surface and a keyed package for versions; `irules_policy` sits in generic visibility | iRules is a library on f5tcl/tmm that tracks its host; iRules policy hangs off the library's presence |
| `DialectProfile` `CATALOG`: 19 rows mixing dialects and environments; Jim has no row; aliases kept twice | `tcl-dialect` holds dialects only; environments are data with one alias table |
| a spec's applicability: `SpecSurface` rows (one provider each, OR'd) + `required_package` + `tcllib_package` + `lifecycle` on whichever axis `lifecycle_axis` picks | one `Availability`: a window on the library's axis ∧ an optional dialect constraint; library-level compatibility rows |
| package windows in SpecTcl `available` parsed and dropped (D17-P); `package require` read as a lower bound only | windows carried; requirements read as full ranges with trains |
| Jim builds and capabilities not selectable | selectable through the environment; libraries and commands may require a capability |
| the surface list kept in four disagreeing copies | one catalogue crate lists the shipped dialects and libraries |

## SpecTcl 3

SpecTcl is redesigned to state the model directly. Old packs keep loading
through shims.

- **Top-level forms.**
  - `dialect FAMILY {…}` is trusted tiers only: releases, builds, capability
    flags and axis values.
  - `library NAME {…}` holds `versions` and trains, `compatible {FAMILY RANGE
    ?-build B? ?-requires CAPS?}` rows, `requires {LIBRARY RANGE}`, and
    `tracks-host`.
  - `command`, `subcommand` and `option` carry `available LIBRARY-RANGE
    ?-dialect {FAMILY RANGE …}?`, plus the existing argument, role, effect and
    hook vocabulary.
  - `environment NAME {dialect …; library NAME RANGE -placement ambient|hosted|bundled; policy …; alias …; detect …}`.
- **Hooks.** A shipped library's hooks are native Rust functions in its crate,
  named by `-native ID`, as today. Tcl-bodied hooks remain for third-party
  packs.
- **Shims.**
  - 1.x (`dialects {…}`) and 2.x (`available`, `environment`, `include from`)
    load by translation into the 3 model, the role `upgrade.rs`'s
    `TRANSLATIONS` and `MEMBERSHIP` tables play today.
  - The promise changes from byte-equal `CommandSpec` to *equivalent answers*.
    The loader's equivalence tests compare visibility, arity and roles for
    every command across the release matrix, for every bundled pack and the
    test corpus.
  - `tcl spec upgrade` rewrites any old pack to 3.
- **Not shimmed.** sslictcl is internal to this project, so its surface and
  format change with its consumers, with no compatibility layer. The same goes
  for the equivalence packs, which are generated measuring instruments.
- **Loader placement.** The loader moves off `tcl-compiler` and `tcl-lsp-core`:
  - it lexes with `tcl-source` (compiler split, item 251);
  - it reads files through a source-store trait the entry point supplies.

  It can then sit just above the registry core, below every analysis crate.

## Libraries as data (decided by a spike)

The surfaces are 185k lines of Rust command data plus 174k lines of BIG-IP
schema data. Every per-command fix recompiles Rust. Separately, the
compiled-surface plan measures 1.3 ms to build native Tcl 8.6 against 697 ms
to load the same surface from SpecTcl, and `SpecArena`/`CompiledSurface` are
not built.

The target, if the spike holds:
- **Each shipped library is authored once, in SpecTcl 3.** Its crate holds
  only the native hooks.
- **Packs are compiled to images.** A tool compiles the library's pack into a
  compact binary image, which the crate embeds with `include_bytes!`. The image
  is checked in and regenerated with a `--check` gate (item 119). Loading
  decodes the image without parsing.
- **A spec edit is a data edit.** Regenerating the image recompiles only that
  one small crate.
- **One catalogue.** The 98k lines of equivalence packs and the native Rust
  catalogue are both deleted.

The spike (item 271) does this end to end for Expect (2.7k lines). It measures:
- load time and memory against the native surface;
- the build time of a one-command edit;
- equivalence of answers.

It decides between three outcomes: libraries as data; native Rust per library
crate (the crate split alone); or data for some libraries and Rust for
hook-heavy ones (the Tcl core surface).

## Consumers

- **iRules checks.** The `is_irules()` checks (18 in `tcl-compiler`, 14 in
  `tcl-lsp-core`, 7 in `tcl-lsp-server`, 3 in `tcl-cli`) and the 114 + 53
  `"f5-irules"` strings become queries on the resolved environment:
  - "is the iRules library present";
  - "is the build `tmm`".
- **`irules_checks`** belongs to the iRules library side. That decides the
  open question in compiler split step 257.
- **The lexer** receives the dialect core profile only.
- **The VM and the runtime** receive the dialect plus the libraries they can
  execute.

## Plan items

| Item | Step |
|---|---|
| 266 | The dialect model: three families, F5 releases and builds, selectable Jim builds and capabilities, and `tcl-dialect` cleared of environments |
| 267 | The library model: version trains, compatibility rows, requirements, bundling, one `Availability` per command, D17-P fixed, full `package require` ranges |
| 268 | Environments as data over 266 and 267, with one alias table and one catalogue |
| 269 | iRules, iApps and tmsh as libraries on f5tcl; consumers move to environment queries |
| 270 | SpecTcl 3 with 1.x/2.x shims, the loader on `tcl-source`, and `tcl spec upgrade` to 3 |
| 271 | The libraries-as-data spike on Expect, which decides 272 |
| 272 | Shipped libraries as SpecTcl-sourced images (conditional on 271) |

## Order

1. **Crate split first.** The registry crate split (262–265) needs none of
   this model, and it pays for itself in build time.
2. **Then the model.** 266, 267 and 268 change types the surface crates
   already hold, so they follow the split.
3. **iRules and iApps.** 269 follows 266 and 267.
4. **SpecTcl.** 270 needs 267's vocabulary and the compiler split's
   `tcl-source` (item 251).
5. **The spike.** 271 follows 270, and 272 follows 271.
