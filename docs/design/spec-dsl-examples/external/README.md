# External-library command-shape census

Evidence for [spec-packs.md](../../spec-packs.md) (issue #1363) gathered from
four real, independently-authored Tcl code bases — none of them written with
tcl-lsp or the spec-pack DSL in mind. The goal: find out what a *real*
private-library author would need to say, and where the provisional DSL
sketch (and the ported core-command examples already sitting in the parent
directory) falls short.

Every claim below is grounded in a `file:line` from an actual clone; nothing
here is invented from the library's documentation alone. Syntax that is
*invented* for the drafted specs is marked `DSL GAP` inline in the
`.tclspec.tcl` files and cross-referenced from [§4](#4-full-gap-catalogue)
below by tag (`G1`, `G2`, …).

## Contents

1. [Method](#1-method)
2. [Census table](#2-census-table)
3. [Per-library findings](#3-per-library-findings)
4. [Full gap catalogue](#4-full-gap-catalogue)
5. [Ranked DSL requirements](#5-ranked-dsl-requirements-by-cross-library-frequency)
6. [Relationship to `tricky-surfaces.md`](#6-relationship-to-tricky-surfacesmd)

## 1. Method

Cloned shallow (`--depth 1`) into
`/tmp/claude-0/-home-user-tcl-lsp/241b398e-7587-50ca-bb66-f781a31ce1e2/scratchpad/corpus/`:

| Library | Repo | Commit (shallow HEAD) | Date | Pkg version |
|---|---|---|---|---|
| ticklecharts | `nico-robert/ticklecharts` | `b49f014c` | 2025-07-04 | 3.2.8 |
| apave (package) / pave (repo) | `aplsimple/pave` | `875de1f1` | 2026-07-08 | 4.9.1 |
| SpiceGenTcl | `georgtree/SpiceGenTcl` | `e8aa45ce` | 2026-08-11 | 0.71 |
| tcllib | `tcltk/tcllib` | `6093f8d6` | 2026-08-07 | (per-module below) |

Two corrections to the task brief worth recording:

- The apave *package* (`::apave`, `package provide apave 4.9.1`,
  `pave/apave.tcl:10`) ships from the repo **`aplsimple/pave`**, not
  `aplsimple/apave` — that path 404s through the session's GitHub proxy
  (confirmed with `mcp__github__search_repositories user:aplsimple`: no
  `apave` repo exists; `pave`, topics `gui layout-engine paver tcl-tk`, does).
  The draft file is still named `apave.tclspec.tcl` because `apave` is the
  package/namespace identity a `speclib` declaration would key off, matching
  how `oo-class.tclspec.tcl` names itself after the Tcl command, not the
  shipping crate file.
- `rust/tcl-lsp/tmp/` (mentioned in the task as a possible source of
  pre-cloned trees) does not exist in this checkout — it is `.gitignore`d
  (`.gitignore:79`) and empty. Everything here was cloned fresh. The repo's
  own `docs/design/issue-923-differential-audit/` shows the *same four
  corpora families* (georgtree's SpiceGenTcl/argparse/tclopt, nico-robert's
  ticklecharts/pix/tomato, tcllib, Tk) were mined once before for a
  name-resolution audit — that scratchpad is gone, but its
  `01-mine-tricky-tcl-patterns.js` confirms these are the right libraries to
  keep returning to.

tcllib modules were picked by diffing against
`rust/tcl-registry/src/commands/tcllib/` (206 files spanning `base32`
through `yaml`, checked exhaustively, no `struct__tree*`, `struct__graph*`,
or `fileutil__traverse*` present — only the *generic-language* `snit__type`/
`snit__typemethod` specs exist, which model the `snit::type` **definer**
grammar, not any specific snit-authored library). Picked:
`struct::tree` (2.1.3), `struct::graph` (2.4.4), `fileutil::traverse` (0.7)
— a bare-command handle factory, a richer one with third-level dispatch, and
a snit-built one with callback-shaped options, respectively. All three also
happen to be real dependencies or close cousins of the other three corpora
(`SpiceGenTcl.tcl:19` requires `struct::tree`/`struct::list`; tcllib's own
`snit` is what `fileutil::traverse` is built from and what
`rust/tcl-registry` already partially models).

I did not clone `struct::list`/`struct::set`/`struct::queue`/`struct::stack`
(already shipped) or attempt full coverage of any corpus — census counts
below are exhaustive greps over the whole clone; drafted specs cover 3-5
representative commands/methods per library, as asked.

## 2. Census table

Counts are exhaustive `grep`/`wc` over the full clone (commands given in §1).
"Ensemble-like dispatch" = subcommand-shaped behaviour that is **not** a
native `namespace ensemble`. "Handle factory" = a command whose call returns
or binds a new dispatchable command/object.

| Library | Public surface | Object system | Ensemble-like dispatch | Options | Callbacks | Version gates | Handle factories | Taint-relevant |
|---|---|---|---|---|---|---|---|---|
| ticklecharts | 12 `oo::class`, ~90 exported procs, 3,961 `setdef` option declarations | plain TclOO (`oo::class create` / `oo::define`) | `chart::Add` literal-first-word switch (chart.tcl:1026-1066) | 3,961 `setdef` calls across option-builder procs; 124 shared closed-enum validators (eformat.tcl) | none found (no command-prefix option) | 3,961× `-minversion` against **Apache ECharts'** own release train (5, 5.2.0, 5.3.3, …), not Tcl | `chart`/`chart3D`/`dataset`/`jsfunc`/`Gridlayout`/`timeline`/`eColor` via `new` | `Render` writes files (chart.tcl:271); `jsfunc` embeds literal/`subst`'d JS into generated HTML (jsfunc.tcl:6) |
| apave (pave) | 5 `oo::class` (APave/APaveBase/APaveDialog/ObjectProperty/ObjectTheming), ~250 methods total | plain TclOO, single-inheritance chain + `mixin` | widget-type dispatch on first-3-chars of a name (apavebase.tcl:981-1201, `switch -glob`) | grid/pack options + widget attrs as nested `-flag value` sub-lists inside table rows | `-com`/`-link` callback-ish attrs (apavebase.tcl:1044-1048) | none (pure Tcl/Tk version, no per-option gate found) | `paveWindow`/`Window` pave a whole widget tree from one table argument (apavebase.tcl:3476,3631) | `Render`-adjacent: none found directly, but dynamic method/type registration (`makeWidgetMethod` apavebase.tcl:2385, `defaultATTRS` apavebase.tcl:1201) |
| SpiceGenTcl | 241 `oo::class create` (45 `oo::configurable`), ~90 exported class names | TclOO **and** `oo::configurable` (TIP 558 properties) | `actOnParam`/`actOnPin`: `-add/-get/-set/-delete/-all` flags select an action (generalClasses.tcl:795-908) | 164 `argparse` call sites; 75 `-forbid`, 189 `-require` relationships | none as Tcl callbacks; `exec`/`open \|cmd` process spawns instead | `package require Tcl 9.0-` (SpiceGenTcl.tcl:17) — whole-package dialect gate, not per-option | 241 classes via `new`/`create`; `R superclass Resistor` alias-subclass (specElementsClassesNgspice.tcl:138) | `exec {*}[list $Command …]` (specSimulatorClassesNgspice.tcl:77); `open "\|$command 2>@1"` (specSimulatorClassesNgspice.tcl:128); path built from untrusted string (specSimulatorClassesNgspice.tcl:71) |
| tcllib `struct::tree` | 1 factory + ~40 `_verb` procs | hand-rolled: `interp alias` + naming-convention dispatch, **not** `namespace ensemble` | `TreeProc` dispatches `$name cmd args` → `_$cmd`, unknown-option list built by **runtime introspection** of `info commands ::struct::tree::_*` (tree_tcl.tcl:200-218) | none (procedural, not `-flag`) | `walk`'s `script` argument is a `foreach`-shaped **Body**, not a CommandPrefix (tree_tcl.tcl:1698, WalkCall tree_tcl.tcl:2090-2097, `upvar 2`/`uplevel 2`) | none | `tree ?name?` → `interp alias {} $name {} ::struct::tree::TreeProc $name` (tree_tcl.tcl:49-161) | custom completion code 5 (`prune`, tree_tcl.tcl:181-183) only meaningful inside `walk`'s body |
| tcllib `struct::graph` | 1 factory + ~93 procs | same hand-rolled shape as `struct::tree` (graph_tcl.tcl:52,184) | **third-level** dispatch: `$g arc <op>`/`$g node <op>` (graph_tcl.tcl:287 `_arc`, 1575 `_node`, dozens of `__arc_*`/`__node_*`) | none | none found | none | `graph ?name?` (graph_tcl.tcl:52) | none found beyond struct::tree's shape |
| tcllib `fileutil::traverse` | 1 `snit::type`, 3 options, 3 methods | `snit::type` (traverse.tcl:23) | none — `next`/`foreach`/`files` are a small closed method set | `-filter`/`-prefilter`/`-errorcmd`, each `-readonly 1` (traverse.tcl:95-97) | all 3 options are documented command-prefix callbacks with **different** appended arities (1 vs 2 args) (traverse.tcl:66-77) | none | `::fileutil::traverse create %AUTO% basedir ...` (snit factory) | `-errorcmd` receives filesystem error text; `next fvar` writes a path into a caller variable (traverse.tcl:148) |

## 3. Per-library findings

### 3.1 ticklecharts — chart DSL, huge option surfaces

- **12** `oo::class create` sites total (`chart`, `chart3D`, `dataset`,
  `eColor`, `ehuddle`, `eList`, `eDict`, `eString`, `eStruct`, `jsfunc`,
  `Gridlayout`, `timeline`) — confirmed by exhaustive grep, not a sample.
  `chart.tcl:6` is the flagship; `chart.tcl:36-1179` carries 47 `method`
  declarations (grep `"    method [A-Za-z]"` count), almost all
  `AddXSeries {args}` one-liners forwarding into a private builder proc.
- **The option surface is enormous and version-gated against a
  *third-party library's* release train, not Tcl's.** `series.tcl:6-16`
  (`barSeries`) alone declares 50+ options via a private `setdef` macro
  (`utils.tcl:415-448`): `setdef options -colorBy -minversion "5.2.0"
  -validvalue formatColorBy -type str -trace no -default "series"`.
  Across the whole tree: **3,961** `setdef options` calls
  (`grep -rc "setdef options" *.tcl`), essentially all carrying a
  `-minversion` against Apache ECharts' own version numbers (`5`,
  `"5.2.0"`, `"5.3.3"`). `SpiceGenTcl.tcl`/`fields.md` already have the
  *right field* for this (`OptionSpec::lifecycle`,
  `rust/tcl-registry/src/hover.rs:402-408`, gated "against the version
  resolved from `package require` — orthogonal to `dialects`") — the DSL
  sketch just never shows syntax for it. See G3.
- **Closed value-enums are centralised and referenced symbolically.**
  `eformat.tcl:6` (`ticklecharts::formatEcharts`) is a single dispatcher
  keyed on a `formattype` symbol; **124** distinct `formatXxx` cases exist
  (`grep -c "^        format[A-Za-z0-9]* {$" eformat.tcl`), e.g.
  `formatTextAlign` at `eformat.tcl:46-48`:
  `set validvalue {auto left right center}; if {$value ni $validvalue}
  {error ...}` — an exhaustive closed set, referenced from `setdef` calls
  by symbol (`-validvalue formatTextAlign`). This is *exactly* the shape
  `values NAME { value V -detail {...} }` + `-values-from NAME` already
  solves for positional `arg`s in `string.tclspec.tcl:17-42` — it is not
  yet shown applied to `option`s. See G5.
- **No native ensemble anywhere.** `chart::Add` (`chart.tcl:1006-1069`)
  takes a literal first word (`"barSeries"`, `"lineSeries"`, …) and
  `switch -exact --`s on it (`chart.tcl:1026-1050`); on no match it
  *introspects its own class definition* (`ticklecharts::classDef [self
  class] [self method]`, `chart.tcl:1051`) to build the error message's
  legal-value list, rather than declaring the set once. This is
  `object_class` territory — see G1.
- **Foreign, non-Tcl code as a first-class value.** `jsfunc.tcl:6`
  (`oo::class create ticklecharts::jsfunc`) wraps a literal or
  `subst`-substituted **JavaScript** source blob that is later spliced
  verbatim into the generated HTML (`chart.tcl:348-351`,
  `ticklecharts::htmlMap`). `Body` in `fields.md`
  ("Tcl script body, recursed into by the analyser") is the wrong role —
  this is never analysed as Tcl, and there is no taint colour parallel to
  `HTML_ESCAPED` for "this is safe JS to inline". See G6.
- **A file-write sink sits on an object-class method.** `chart.tcl:250-289`
  (`method Render`) does `open $outputFile w+` / `puts $fp [my toHTML
  ...]` (`chart.tcl:271-272`) where `-outfile` is caller-supplied. `Render`
  is a `SubCommand` inside `object_class.instance_methods` — see G7 for why
  today's taint fields can't reach it even in principle.
- **One method body shared textually across three unrelated classes.**
  `esnap.tcl:6-13`: `foreach class {ticklecharts::chart
  ticklecharts::Gridlayout ticklecharts::timeline} { oo::define $class {
  method SnapShot {args} {...} } }` installs the *identical* `SnapShot`
  method on three classes that share no common ancestor for it. Soft gap,
  see G10.

### 3.2 apave (`aplsimple/pave`) — widget layout DSL

- **5** `oo::class create` sites (`apave.tcl:810` `APave`, superclass
  `APaveDialog` at `apave.tcl:812`; `apavedialog.tcl:46` `APaveDialog`,
  superclass `APaveBase`; `apavebase.tcl:430` `APaveBase`, `mixin
  ::apave::ObjectTheming` at `apavebase.tcl:432`; `obbit.tcl:1159`
  `ObjectProperty`; `obbit.tcl:1212` `ObjectTheming`). Single-inheritance
  spine plus deliberate mixins — `ObjectProperty`'s constructor
  (`obbit.tcl:1163-1167`) defensively checks `[llength [self next]]`
  before chaining, i.e. it is *designed* to be usable either standalone or
  mixed in, never assuming its MRO position.
- **The layout DSL is a table of 7-field tuples, not a flat clause list.**
  `apavebase.tcl:3476` (`method Window {w inplists}`) and its public
  wrapper `apavebase.tcl:3631` (`method paveWindow {args}`, itself taking
  repeated `win lwidgets` pairs) process a list whose every element is
  `{name neighbor posofnei rowspan colspan options attrs}`
  (`apavebase.tcl:3480-3487`). `name`'s first 3 characters select a widget
  type (`apavebase.tcl:981` `method widgetType`, `switch -glob -- $nam3
  {bts {...} but {...} chb {...} ...}` at `apavebase.tcl:986-1201` — dozens
  of cases). Fields 5 and 6 (`options`, `attrs`) are themselves nested
  `-flag value` lists, evaluated via `uplevel 2 subst -nocommand
  -nobackslashes` (`apavebase.tcl:3546`) — variable substitution only, two
  frames up. This is one nesting level past what the already-solved
  `clause_grammar` (`if.tclspec.tcl:33-37`) or `case_list`
  (`switch.tclspec.tcl:69-81`) constructs reach. See G8/G9.
- **The widget-type vocabulary is closed *and* runtime-extensible at the
  same time.** `apavebase.tcl:1201` (`method defaultATTRS`) lets a caller
  register a brand-new 3-letter type code at runtime (documented example
  at `apavebase.tcl:1212`: `my defaultATTRS tbt {} {-style Toolbutton
  -compound top} ttk::button`). `closed_value_args`'s "exhaustive legal
  set" promise and this genuine extensibility can't both be true at once
  for the same field. See G9.
- **Dynamic, per-*object* (not per-class) method installation with a
  computed name.** `apavebase.tcl:2385` (`method makeWidgetMethod`) runs
  `oo::objdefine [self] "method $method {} {return $wnamefull} ; export
  $method"` (`apavebase.tcl:2407-2408`) to mint one accessor per named
  widget on *this instance only*. No `CommandSpec`/`SubCommand` field
  models "this call installs a method on an object" — only interpreter
  command-table effects exist (`proc`/`rename`/`interp alias`). See G11.
- **Getter/setter-by-arity, cleanly.** `obbit.tcl:1176-1190`
  (`method setProperty {name args}`) is 0-args-getter / 1-arg-setter by
  its own docstring and a `switch -exact [llength $args] {0 {...} 1
  {...}}` (`obbit.tcl:1183-1189`) — a clean, non-Tk-widget confirmation
  that `forms` (`Getter`/`Setter`/`Default`) already covers this shape;
  no gap, just fresh grounding (see `fields.md`'s own `$w cget -opt`
  versus `$w configure -opt value` example).
- **Deprecation as a same-class method pair.** `apavebase.tcl:3673-3678`
  (`method window {args}`) is `uplevel 1 [list [self] paveWindow {*}$args]`
  with the docstring "Obsolete version of paveWindow (remains for
  compatibility)" — a drop-in `deprecated_replacement` at the
  `object_class` method level. Again a confirmation, not a gap.

### 3.3 SpiceGenTcl (`georgtree/SpiceGenTcl`) — the issue's own library, TclOO-heavy

- **241** `oo::class create` sites (`grep -rc`, exhaustive), of which
  **45** are `oo::configurable create` (Tcl 9's TIP 558 property
  metaclass — `oo::configurable create SPICEElement` at
  `generalClasses.tcl:76`). `package require Tcl 9.0-`
  (`SpiceGenTcl.tcl:17`) — this whole library requires Tcl 9.0+ and
  nothing older, a real, current, whole-package `dialects` example.
- **`argparse` is the option/positional grammar for nearly everything.**
  **164** `argparse` call sites (constructors and ordinary methods alike);
  `Resistor`'s constructor (`specElementsClassesNgspice.tcl:34-135`) is
  representative: `argparse -inline -pfirst -help {...} { {-r=
  -help {...}} {-beh -forbid {model} -require {r} -help {...}} {-model=
  -forbid {beh} -help {...}} ... {name -help {...}} {np -help {...}} {nm
  -help {...}} }` (`specElementsClassesNgspice.tcl:90-107`). From the
  *outside*, every one of the 241 constructors is just `{args}` — arity
  0..∞ — the real shape (3 mandatory positionals + a dozen mutually
  constrained flags) exists only as data interpreted by a third-party
  command at runtime. See G4.
- **`-forbid` (mutual exclusion) and `-require` (conditional requirement)
  are both extremely common, and only one has any struct backing.**
  Exhaustive count: **75** `-forbid`, **189** `-require`
  (`grep -roc -- "-forbid"` / `"-require"` over `src/`).
  `OptionConstraint` (`rust/tcl-registry/src/spec.rs:520-525`) is `{
  options: &[&str], dialects }` — a flat "may not co-occur" set, which
  covers `-forbid` (once the DSL grows syntax for it — no sibling
  `.tclspec.tcl` in the parent directory shows `option_constraints`
  syntax yet either) but has **no field at all** for "-require" ‑ there
  is no directionality, no "implies", nothing. `-l=` and `-w=` both
  `-require {model}` at `specElementsClassesNgspice.tcl:102-103`. See G2.
- **Argument-shape ensembles, again.** `Device::actOnParam`
  (`generalClasses.tcl:795-908`) takes `-add|-get|-set|-delete|-all` flags
  that `argparse -key action -value X` maps onto a single `action`
  variable, then `switch -- $action { add {...} get {...} ... }`
  (`generalClasses.tcl:837-908`) — same *shape* as ticklecharts' `Add` and
  apave's `widgetType`, third distinct implementation strategy for the
  same underlying need. `actOnPin` (`generalClasses.tcl:727-793`) is the
  same pattern again.
- **One method body copy-pasted across 6 unrelated classes at
  class-definition time.** `method actOnParam {*}[info class definition
  ::SpiceGenTcl::Device actOnParam]` appears **6** times, verbatim
  (`generalClasses.tcl:1031, 1250, 1300, 1440, 1863, 1958`) — classes that
  don't inherit `actOnParam` normally (they override `genSPICEString`
  only) instead splice Device's method body in by literal `info class
  definition` copy. Soft gap, see G10.
- **A definer-grammar member shape the DSL's `definition_body` field
  can't describe.** `oo::configurable`'s `property` clause
  (`generalClasses.tcl:342-344` `property value`; `:376-388` `property
  name -set {...}`; the `Batch` class's `property log -get {...}`,
  `specSimulatorClassesNgspice.tcl:24-30`) has *zero* param-list and up
  to **two** optional named body attachments (`-get`, `-set`, each with an
  implicit `$value` in `-set`). `definition_body`'s documented shape
  (`fields.md`: "which words of each are the name, the parameter list,
  and the body") is a fixed `{name, params, body}` triple and doesn't fit.
  `tricky-surfaces.md:28` already names this abstractly ("flag-keyed —
  `oo::configurable`'s `property`, which is also 9.0-gated *per member*")
  — this is its first concrete real-world grounding. See G12.
- **Real command-injection-shaped sinks live inside TclOO instance
  methods, not free commands.** `Batch::runAndRead`
  (`specSimulatorClassesNgspice.tcl:59-86`) does `exec {*}[list $Command
  -b -r $rawFileName -o $logFileName $cirFileName]` at line 77;
  `BatchLiveLog::runAndRead` (`specSimulatorClassesNgspice.tcl:108-146`,
  overriding the same method) does `open "|$command 2>@1"` at line 128.
  Both also build a **file path directly from untrusted input**:
  `[file join $runLocation ${firstLine}.cir]` where `firstLine` is the
  first line of the caller-supplied `circuitStr`
  (`specSimulatorClassesNgspice.tcl:69-71, 74-76, 119-121, 124-126`).
  `Batch`/`BatchLiveLog` are `object_class` methods (`SubCommand`
  entries) — see G7 for why `taint_code_sink_args` etc. can't be declared
  on them at all today.
- **Prior art: this exact library already hand-annotates for a rival
  linter.** **55** `##nagelfar ...` comments across `src/`
  (`grep -rc "##nagelfar"`): **19** `subcmd+` (teaching
  [Nagelfar](https://nagelfar.sourceforge.net/) about pseudo-subcommands
  on `configure`, e.g. `##nagelfar subcmd+ _obj,Batch configure` at
  `specSimulatorClassesNgspice.tcl:20`), 16 `ignore`, 11 `variable`, 6
  `implicitvarcmd`, 3 `nocover`. The library's own author is *already*
  hand-writing exactly the kind of shape metadata spec-packs would let
  them declare once, in one place, for every tool at once.

### 3.4 tcllib (`struct::tree`, `struct::graph`, `fileutil::traverse`)

- **None of the three uses `namespace ensemble create`.** All three are
  hand-rolled dispatch, three *different* ways:
  - `struct::tree`/`struct::graph`: a bare factory proc
    (`tree_tcl.tcl:49-161`, `graph_tcl.tcl:52`) that installs
    `interp alias {} $name {} ::struct::tree::TreeProc $name`
    (`tree_tcl.tcl:131`) — a textbook `command_table_effect: CreatesAliases`
    + `creates_instance_at 0` pairing, more literal than any TclOO
    example. The dispatcher (`TreeProc`, `tree_tcl.tcl:200-228`;
    `GraphProc`, `graph_tcl.tcl:184`) maps `$name cmd args` onto
    `::struct::tree::_$cmd` by **naming convention**, and on an unknown
    `cmd` it lists the legal set by **introspecting `info commands
    ::struct::tree::_*` at runtime** (`tree_tcl.tcl:208-217`) rather than
    reading a fixed table.
  - `struct::graph` additionally nests a **third level**: `$g arc <op>
    ...` / `$g node <op> ...` (`graph_tcl.tcl:287` `_arc`, `:1575`
    `_node`), each dispatching dozens of its own `__arc_*`/`__node_*` ops
    (`get`, `getall`, `keys`, `set`, `append`, `attr`, `move`,
    `setweight`, …, `graph_tcl.tcl:425-1050`). A clean, non-`info
    object`, real-world grounding for `sub_subcommands` — and since
    `arc`/`node` are `SubCommand` entries under an `object_class`, this
    also confirms `sub_subcommands` reaches through `object_class` (not
    just plain ensembles), which no existing ported example shows.
  - `fileutil::traverse`: `snit::type ::fileutil::traverse`
    (`traverse.tcl:23`) with exactly 3 methods (`files`, `foreach`,
    `next`).
- **`struct::tree walk`'s "script" is a loop body, not a callback.**
  `tree_tcl.tcl:1698` (`proc _walk {name node args}`) with usage `"$name
  walk node ?-type {bfs|dfs}? ?-order {pre|post|in|both}? ?--? loopvar
  script"`; `WalkCall` (`tree_tcl.tcl:2090-2097`) does `upvar 2 $avar a;
  set a $action`, `upvar 2 $nvar n; set n $node`, then `uplevel 2 $cmd` —
  exactly `foreach`'s shape (`LoopVarList` + `Body(Plain)`), *not*
  `CommandPrefix`. Correctly modelled with existing fields; cited as a
  clean confirmation that a hand-rolled "visit each X" method can be
  either shape and a spec author needs to read the body to tell which.
- **A library-defined, non-standard completion code, scoped to one
  command's body.** `prune` (`tree_tcl.tcl:181-183`): `proc
  ::struct::tree::prune_tcl {} { return -code 5 }`. Consumed *only* inside
  `WalkCall`'s `switch -exact -- $code { ... 5 {... return -code continue
  ...} ... }` (`tree_tcl.tcl:2109-2134`). Tcl's `-code` accepts any
  integer; `fields.md`'s `completion` vocabulary only names 0-4
  (`ok`/`error`/`return`/`break`/`continue`). See G13.
- **`fileutil::traverse`'s three options are the cleanest
  `command_prefixes`-with-different-arities example found in any corpus.**
  Documented in the source itself (`traverse.tcl:66-77`): `-prefilter`
  "invoked with a single argument, the path"; `-filter` "invoked with a
  single argument, the path"; `-errorcmd` "invoked with a two arguments,
  the path ... and the error message" — i.e. appended arity 1, 1, 2
  respectively on three options of the same command. All three are also
  `-readonly 1` (`traverse.tcl:95-97`) — settable only at construction,
  which has **no backing field anywhere** on `OptionSpec`
  (`rust/tcl-registry/src/hover.rs:380-415`, read in full — `name`,
  `value`, `detail`, `dialects`, `aliases`, `lifecycle`, `min_abbrev`;
  nothing about post-construction mutability). See G14.
- **`next fvar` writes into a caller variable while returning a
  boolean.** `traverse.tcl:148` (`method next {fvar}`) — a clean
  `var_write_typing: Fixed` example (`Tcl`'s own `gets chan varName`
  shape), grounded outside the stdlib for the first time in this
  directory.

## 4. Full gap catalogue

Each gap is tagged `[SYNTAX]` (the DSL sketch just never shows the spelling
— the underlying `CommandSpec`/`SubCommand`/`OptionSpec` field already
exists and was read directly in `rust/tcl-registry/src/spec.rs` /
`hover.rs`), `[STRUCT]` (no field exists on the Rust struct at all — a
registry change, not just a DSL-syntax change, would be needed), or `[SOFT]`
(expressible today via duplication/authoring effort; a nice-to-have, not a
blocker). Every drafted `.tclspec.tcl` marks its own invented syntax inline
with the matching tag.

- **G1 `[SYNTAX]` — no `object_class`/TclOO syntax at all.** `spec-packs.md`'s
  sketch shows zero TclOO. `ObjectClassSpec` (`spec.rs:249-268`:
  `class_name`, `instance_methods: &[SubCommand]`, `superclasses: &[&str]`,
  `allow_unknown_methods`) and `CommandSpec::object_class`
  (`spec.rs:1122-1125`, whose own doc comment names `ticklecharts::chart`
  as *the* example of "ordinary TclOO `new`/`create` dispatch") are fully
  designed and load-bearing for every one of the 4 corpora. Invented in
  every drafted file: an `object_class { superclasses {...}
  allow_unknown_methods no  method NAME { ...same grammar as `subcommand`...
  } }` block, reusing `subcommand`'s existing body grammar under a `method`
  keyword (methods **are** `SubCommand`s structurally, so no new per-field
  syntax is invented — only the wrapping keyword).
- **G2 `[STRUCT]` — option "requires" relationship.** `-require` outnumbers
  `-forbid` in SpiceGenTcl (189 vs 75). `OptionConstraint`
  (`spec.rs:520-525`) is a flat "may not co-occur" set with no
  directionality — it cannot express "requires" even as a hook target; a
  `literal_argument_validator` (code, command-scoped) *could* express it,
  but there is no declarative mirror of `option_constraints` for
  requirement the way there is for exclusion. Invented:
  `option_constraints { forbid {A B} ... requires {A B} {C} }` with the
  `requires` clause marked as unsupported by any struct field today.
- **G3 `[SYNTAX]` — per-option lifecycle against a non-Tcl version axis.**
  3,961 grounded occurrences in ticklecharts alone. `OptionSpec::lifecycle:
  Lifecycle` (`hover.rs:402-408`) already exists and is explicitly
  documented as "orthogonal to `dialects`". Invented: `option NAME -since
  VERSION` (maps to `Lifecycle.introduced`), noting `-deprecated`/`-retired`
  as the natural (undemonstrated in the draft, since no ticklecharts option
  sampled needed them) extensions.
- **G4 `[SOFT]`, tooling not vocabulary — `argparse`-declared option
  grammar is invisible to static declaration.** 164 call sites across
  SpiceGenTcl; every constructor's *declared* arity is `{args}` from the
  outside. No missing field — `options`/`arity`/`arg_roles` can say
  everything an `argparse` block says once hand-transcribed — the gap is
  that there is no mechanism to point the DSL at the embedded literal as a
  source of truth, so 164 grammars must be hand-duplicated (and kept in
  sync by hand on every upstream change). `fields.md`'s `OptProc` analyser
  hook ("argparse-style proc") suggests *plain-proc* argparse recognition
  exists already; whether it extends to TclOO constructor bodies is an
  open question, not confirmed either way here.
- **G5 `[SYNTAX]` — closed value-enums shared across many `option`s.**
  124 validator cases, ~180+ references, in ticklecharts. The exact
  mechanism already exists for positional `arg`s (`values NAME { value V
  -detail {...} }` + `arg N -values-from NAME`, `string.tclspec.tcl:17-42`)
  — invented here is only its *application* to `option NAME -takes value
  -values-from NAME -closed`, not a new mechanism.
- **G6 `[STRUCT]` — foreign (non-Tcl) code as a value.** `ticklecharts::jsfunc`
  wraps JavaScript, not Tcl. No `ArgRole`/`BodyKind` fits (`Body` is
  documented as "recursed into by the analyser" — this must not be); no
  taint colour parallels `HTML_ESCAPED` for "safe to inline as script".
  Not drafted as a fix — flagged as out of scope for a per-command DSL
  addition and noted verbatim in the spec file's header comment.
- **G7 `[STRUCT]` — most taint-sink fields are `CommandSpec`-only, but
  TclOO instance methods are `SubCommand`s.** Per `fields.md`'s own
  per-field scope markers: `taint_code_sink_args`, `taint_network_sink_args`,
  `taint_log_sink`, `taint_source`, `taint_sink_safe_colour`,
  `credential_options` are all *command only*; only `taint_output_sink`,
  `taint_transform`, `taint_double_encode_colour`, `sensitive_headers` are
  *command and subcommand*. `ObjectClassSpec.instance_methods`
  (`spec.rs:255-256`) is `&[SubCommand]`. `Batch::runAndRead` /
  `BatchLiveLog::runAndRead` (both real `exec`/`open |cmd` process-spawn
  sinks) and `chart::Render` (real file-write sink) are exactly the shape
  this can't reach. Drafted anyway in `SpiceGenTcl.tclspec.tcl` with a loud
  inline gap marker, to show what an author *wants* to write.
- **G8 `[STRUCT]`, speculative — structured multi-field-tuple body
  argument.** apave's `Window`/`paveWindow` row shape
  (`{name neighbor posofnei rowspan colspan options attrs}`) is one nesting
  level past `clause_grammar` (flat `Expr`/`Body` word sequence,
  `if.tclspec.tcl:33-37`) or `case_list` (pattern/body pairs only,
  `switch.tclspec.tcl:69-81`): each *row* has 7 typed positional fields, two
  of which are themselves nested `-flag value` option lists. No existing
  construct reaches two nesting levels. Most speculative invention in this
  census — drafted as `arg N -role body -repeats { row { field IDX NAME
  ... } }`, marked heavily.
- **G9 `[STRUCT]` — a "closed" value set that is also runtime-extensible.**
  apave's widget-type codes (`switch -glob` over ~40 cases,
  `apavebase.tcl:986-1201`) are exhaustively enumerable *and* a caller can
  register new ones at runtime (`defaultATTRS`, `apavebase.tcl:1201-1243`).
  `closed_value_args`'s "exhaustive" contract and `allow_unknown_subcommands`
  exist at the whole-command level; nothing lets one *value position*
  inside a nested field be "closed, but with an escape valve."
- **G10 `[SOFT]` — one method body shared across unrelated classes.**
  ticklecharts' `SnapShot` (`esnap.tcl:6-13`, shared via a `foreach` loop
  over 3 class names) and SpiceGenTcl's 6× literal `actOnParam
  {*}[info class definition ::SpiceGenTcl::Device actOnParam]` copy-splice
  (`generalClasses.tcl:1031,1250,1300,1440,1863,1958`). Fully expressible
  today by declaring the method N times across N `object_class` blocks —
  flagged as a duplication/maintenance cost, not a blocker. A "nice to
  have" would be a named, shared method-set reusable across several
  `object_class` declarations the way `values`/`descriptor` blocks are
  already shared across `command`s.
- **G11 `[STRUCT]` — dynamic single-*object* method installation with a
  computed name.** apave's `makeWidgetMethod`
  (`apavebase.tcl:2385-2409`, `oo::objdefine [self] "method $method
  {}..."`). Distinct from `command_table_effect` (interpreter command
  table: `proc`/`rename`/`interp alias`) and from `definition_body`
  (class-wide, not per-instance). No field models "this call adds a
  method to one specific existing object."
- **G12 `[STRUCT]`, already named abstractly in `tricky-surfaces.md:28` —
  `oo::configurable`'s `property` clause doesn't fit `definition_body`'s
  `{name, params, body}` triple.** `property value` (bare),
  `property name -set {...}`, `property log -get {...}` — zero param-list,
  up to two optional named bodies, an implicit `$value` in `-set`.
  Grounded 45×/63 in SpiceGenTcl (`oo::configurable create` /
  `property` counts). Not drafted directly (definer grammars are named,
  shared, maintainer-authored descriptors per `fields.md`, not
  DSL-authorable per-command data) — noted in the spec file's header
  instead.
- **G13 `[STRUCT]` — a library-defined completion code, scoped to one
  command's body.** `struct::tree::prune`'s `return -code 5`
  (`tree_tcl.tcl:181-183`), meaningful only inside `walk`
  (`tree_tcl.tcl:2109-2134`). `fields.md`'s `completion` vocabulary names
  only 0-4. No construct pairs a custom code to the one command whose body
  legitimately accepts it, the way `HAS_LOOP_BODY` pairs with
  `BREAKS_LOOP`/`CONTINUES_LOOP` for the standard codes.
- **G14 `[STRUCT]` — option immutability after construction.**
  `fileutil::traverse`'s `-readonly 1` (`traverse.tcl:95-97`, all 3
  options). `OptionSpec` (`hover.rs:380-415`) has no field for it at all.

## 5. Ranked DSL requirements (by cross-library frequency)

1. **`object_class` / TclOO method modelling (G1).** Present in **4/4**
   corpora, load-bearing for the majority of every public surface examined
   (chart's 47 methods, apave's ~250, 241 SpiceGenTcl classes, tcllib's
   hand-rolled dispatchers). Completely absent from the DSL sketch. The
   single highest-value addition this census found.
2. **Argument-shape ensembles are the norm, native `namespace ensemble` is
   not.** Not a field gap (each individual case reduces to `object_class`
   methods, `subcommands`, or `sub_subcommands`) but the **dominant
   real-world pattern**: `chart::Add` (literal-word switch),
   `actOnParam`/`actOnPin` (`argparse -key/-value` flags), apave's
   `widgetType` (first-3-chars + `switch -glob`), `struct::tree`/`graph`'s
   naming-convention `_$cmd` dispatch with **runtime-introspected** error
   messages. **Zero** uses of `namespace ensemble create` for a primary
   public surface across all 4 corpora. `object_class.instance_methods` +
   `subcommands`/`sub_subcommands` already cover every one of these once
   G1 lands — this is the evidence that they must.
3. **Option relationship vocabulary (G2, G5).** 264 grounded
   `-forbid`/`-require` relationships in SpiceGenTcl alone, plus
   ticklecharts' shared closed-enum validators (124 cases, ~180+
   references) and apave's dynamic type registry. Half of this (mutual
   exclusion, closed enums) is `[SYNTAX]`-only; the `-require` half is
   `[STRUCT]` — no field exists.
4. **Per-option lifecycle against a third-party package's own version
   axis (G3).** 3,961 raw occurrences in ticklecharts. `[SYNTAX]`-only —
   `OptionSpec::lifecycle` already does the job.
5. **Method-level taint-sink modelling (G7).** Only reachable where an
   `object_class` method does something dangerous, but that is *exactly*
   where real libraries put dangerous things: SpiceGenTcl's
   `exec`/`open |cmd` process spawns and untrusted-path file writes are
   both inside TclOO methods, not free commands. `[STRUCT]` — the
   command-only taint fields cannot be attached to a `SubCommand` at all
   today.
6. **`argparse`-as-embedded-grammar / no derivation path (G4).** 164 raw
   occurrences, but a tooling gap, not a vocabulary one — every fact
   `argparse` states, the DSL can already say once transcribed by hand.
7. **Structured multi-field-tuple bodies with nested option sub-lists
   (G8, G9).** apave-specific among the 4 corpora sampled, but the
   general shape (a declarative "table describing a tree of things to
   build") is a common pattern for any layout/config DSL. The single
   biggest reach beyond what `clause_grammar`/`case_list` already solve.
8. **Custom completion codes scoped to one command's body (G13).** Narrow
   (1 corpus, 1 case found) but clean and previously undocumented anywhere
   in `tricky-surfaces.md` or the shipped registry's own vocabulary.
9. **Dynamic method installation — per-object (G11) and shared-across-
   classes (G10, soft).** Real, grounded, but lower priority: workable via
   duplication (G10) or simply out of scope for what a *static* spec
   describes about an object whose shape depends on which methods a
   *previous* call happened to install (G11).
10. **Option-level immutability-after-construction (G14).** Narrow (1
    corpus) but a clean, total struct gap (`OptionSpec` has nothing for
    it), cheap to add if wanted.

## 6. Relationship to `tricky-surfaces.md`

The [rubric](../tricky-surfaces.md) predates this census and was built by
porting *shipped, core-Tcl* commands. Cross-checked against it:

- **Confirms with fresh, non-core grounding (no new gap):** "per-anything
  lifecycle gates" (rubric, Dialects/versions/events) ← G3's 3,961
  occurrences; `object_class` "instance methods with superclass resolution"
  (rubric, TclOO) ← `R superclass Resistor`
  (`specElementsClassesNgspice.tcl:138-140`) needs *zero* new fields, just
  `superclasses {Resistor}` with an empty method list; "flag-keyed —
  `oo::configurable`'s `property`" (rubric, TclOO,
  `tricky-surfaces.md:28`) ← G12's first real-world grounding;
  "credential options and args" / "sensitive headers" (rubric, Taint) ←
  already exercised by `irules-http-header.tclspec.tcl`, unaffected by
  this census; deprecation-as-data (rubric, Documentation) ← apave's
  `window`/`paveWindow` pair.
- **Sharpens an existing rubric item into something more precise:** the
  rubric's Taint section lists "per-slot code/network sinks" as something
  the DSL must cover, without noting *where* — G7 is the sharper claim
  that the command-only scoping of those fields collides with exactly
  where real libraries put the dangerous calls (TclOO methods).
- **Genuinely new, not on the rubric at all:** G2's "requires" option
  relationship (rubric only anticipates "mutual exclusion sets"); G13's
  custom completion codes (rubric only anticipates the 5 standard ones);
  G8/G9's nested structured-table body shape (rubric's "clause grammars"
  section is `if`/`switch`/`expect` shaped — flat word sequences — never a
  table of typed tuples); G11's per-object dynamic method installation
  (rubric's TclOO section covers class-wide definer spellings and method-
  alias binders, not `oo::objdefine [self]` with a *computed* method
  name); G14's option read-only-after-construction; and the cross-library
  **absence of native ensembles** as the dominant real-world shape (§5,
  item 2) — the rubric, written from the core-command vocabulary's
  perspective, has no item that says "most third-party subcommand-like
  dispatch will not be a `namespace ensemble` at all."
