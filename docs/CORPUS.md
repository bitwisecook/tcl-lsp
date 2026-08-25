# External corpus sources

This is the source catalogue for the external corpus work tracked by
[#1181](https://github.com/bitwisecook/tcl-lsp/issues/1181).  It describes
where to obtain material and why it is useful; it does **not** vendor or cache
third-party source.  Downloaded checkouts, generated indexes, and experimental
reductions must stay outside the repository (for example in a temporary
directory or the ignored `scripts/perf/corpus/` directory).

`scripts/perf/MANIFEST.toml` is the sole machine-readable benchmark contract:
it contains the canonical clone URLs and exact commit IDs.  This document must
not duplicate those IDs, as a stale duplicate is worse than no duplicate.

## Status and fetch hygiene

- **Benchmark-pinned** means a repository is one of the 35 public entries in
  the revisioned performance corpus.  Its exact SHA, fetch command, scope and
  file-selection rules are in [the manifest](../scripts/perf/MANIFEST.toml).
  The one optional private entry has the same contract, but is fetched only
  with credentials and `--include-private`; its source list is deliberately
  not reproduced here.
- **Research candidate** means it is valuable for a review, focused fixture or
  a future benchmark revision, but is *not* an implied benchmark addition.
  Adding, removing, or moving a benchmark pin changes the workload: update the
  manifest revision and regenerate comparable measurements as described in
  [the performance README](../scripts/perf/README.md).
- Fetch to a disposable or ignored location, record the URL, revision and
  retrieval date with any finding, and inspect the upstream licence before
  copying even a small fixture.  Prefer a hand-written minimal regression test
  in-tree; retain an attribution/link when a test is reduced from a source.
- "Created" and "last push" below are GitHub metadata observed on **2026-08-24**
  (a last push is not a release or a guarantee of maintenance).  "Dialect
  guess" is an analyst's intended-language guess, not an assertion by the
  upstream project or a tcl-lsp auto-detection result.  `NOASSERTION` means
  GitHub did not supply an SPDX identifier: consult the canonical licence.

## Machine-pinned benchmark sources

These are deliberately grouped as the manifest groups, not ranked by quality.
The short notes say what a targeted search should look for in each source.

| Group / repository | Created; last push observed | Licence metadata | Dialect guess | Concise coverage / style note |
| --- | --- | --- | --- | --- |
| georgtree / [SpiceGenTcl](https://github.com/georgtree/SpiceGenTcl) | 2024; 2026-08-11 | NOASSERTION | Tcl 8.6 + TclOO | SPICE tooling; package/source forest, class-heavy application code. |
| georgtree / [tclopt](https://github.com/georgtree/tclopt) | 2025; 2026-06-18 | MIT | Tcl 8.6+ | Option parsing and declarative CLI-like APIs. |
| georgtree / [ruff](https://github.com/georgtree/ruff) | 2025; 2025-08-13 | BSD-2-Clause | Tcl 8.6+ | Documentation generator; TclOO and command-prefix library style. |
| georgtree / [argparse](https://github.com/georgtree/argparse) | 2025; 2026-06-25 | MIT | Tcl 8.6+ | Compact argument grammar, `switch`/list-driven parsing. |
| georgtree / [tcl_tools](https://github.com/georgtree/tcl_tools) | 2024; 2025-07-22 | MIT | Tcl 8.6+ | Utility package collection; `source` and maths helpers. |
| georgtree / [tclinterp](https://github.com/georgtree/tclinterp) | 2024; 2026-06-15 | LGPL-2.1 | Tcl 8.6+ | Interpreter/metaprogramming code; nested evaluation and command names. |
| georgtree / [tclmeasure](https://github.com/georgtree/tclmeasure) | 2025; 2026-06-14 | LGPL-2.1 | Tcl 8.6+ | Measurement utility; small modern procedure corpus. |
| georgtree / [extexpr](https://github.com/georgtree/extexpr) | 2025; 2026-06-04 | LGPL-2.1 | Tcl 8.6+ | Extended-expression package; expression/list boundary cases. |
| nico-robert / [ticklecharts](https://github.com/nico-robert/ticklecharts) | 2022; 2025-07-04 | MIT | Tcl/Tk 8.6 | Chart widgets; Tk callback and option-heavy style. |
| nico-robert / [tomato](https://github.com/nico-robert/tomato) | 2021; 2026-01-20 | NOASSERTION | Tcl 8.6+ | TclOO framework; inheritance and object command idioms. |
| nico-robert / [pix](https://github.com/nico-robert/pix) | 2024; 2026-08-05 | NOASSERTION | Tcl/Tk 8.6+ | Modern Tk/image application code. |
| nico-robert / [zesty](https://github.com/nico-robert/zesty) | 2025; 2026-01-08 | MIT | Tcl/Tk 8.6+ | Small modern Tk package. |
| nico-robert / [haru](https://github.com/nico-robert/haru) | 2022; 2025-07-08 | NOASSERTION | Tcl/Tk 8.6+ | GUI/library code and object-facing APIs. |
| nico-robert / [implottk](https://github.com/nico-robert/implottk) | 2022; 2023-01-19 | MIT | Tcl/Tk 8.6 | Older plotting/Tk code; useful contrast with active projects. |
| tcltk / [tcllib](https://github.com/tcltk/tcllib) | 2012; 2026-08-15 | NOASSERTION | Tcl 8.4–9 | Canonical broad pure-Tcl library corpus; package conventions and compatibility style. |
| tcltk / [tklib](https://github.com/tcltk/tklib) | 2012; 2026-07-10 | NOASSERTION | Tcl/Tk 8.4–8.6 | Tk megawidgets, bindings and legacy-compatible packages. |
| tcltk / [tk](https://github.com/tcltk/tk) | 2011; 2026-08-24 | NOASSERTION | Tk / Tcl core | Canonical Tk scripts, widget bindings and percent substitutions. |
| aplsimple / [pave](https://github.com/aplsimple/pave) | 2019; 2026-07-08 | MIT | Tcl/Tk 8.6 | Editor-like TclOO/Tk application style. |
| aplsimple / [alited](https://github.com/aplsimple/alited) | 2021; 2026-08-20 | MIT | Tcl/Tk 8.6 | Large editor application; UI callbacks and project-wide procedures. |
| other / [XilinxTclStore](https://github.com/Xilinx/XilinxTclStore) | 2013; 2026-08-21 | NOASSERTION | Vivado Tcl dialect | Vendor EDA command DSL, configuration scripts and versioned APIs. |
| other / [OSVVM-Scripts](https://github.com/OSVVM/OSVVM-Scripts) | 2019; 2026-08-20 | NOASSERTION | EDA Tcl dialect | Simulator/build orchestration and generated/configuration-style Tcl. |
| irules-kaiwilke / [RADIUS Server](https://github.com/KaiWilke/F5-iRule-RADIUS-Server-Stack) | 2022; 2026-03-03 | GPL-3.0 | F5 iRules | Long, deeply nested iRule procedures; binary formats and events. |
| irules-kaiwilke / [RADIUS Client](https://github.com/KaiWilke/F5-iRule-RADIUS-Client-Stack) | 2022; 2026-03-03 | GPL-3.0 | F5 iRules | Client counterpart; event-driven network handling. |
| irules-kaiwilke / [Natural Speech Expression](https://github.com/KaiWilke/F5-Natural-Speech-Expression) | 2023; 2023-02-02 | GPL-3.0 | F5 iRules | Expression/compiler-like iRule code; older activity. |
| irules-kaiwilke / [F5CrowdSRC](https://github.com/KaiWilke/F5CrowdSRC) | 2022; 2022-11-25 | GPL-3.0 | F5 iRules | Community iRule examples; frozen early snapshot. |
| irules-kaiwilke / [PrismJS grammar](https://github.com/KaiWilke/F5-PrismJS-iRule-Language-Definition) | 2023; 2026-03-03 | GPL-3.0 | iRules metadata | Not executable Tcl; independent keyword/grammar reference. |
| irules-f5devcentral / [agility labs](https://github.com/f5devcentral/f5-agility-labs-irules) | 2017; 2026-03-04 | MIT | F5 iRules | Vendor teaching examples and event idioms. |
| irules-f5devcentral / [irules-toolbox](https://github.com/f5devcentral/irules-toolbox) | 2020; 2025-03-20 | MIT | F5 iRules | Reusable recipes across HTTP/TCP/LB namespaces. |
| irules-community / [TesTcl](https://github.com/landro/TesTcl) | 2012; 2023-11-01 | BSD-3-Clause | F5 iRules + Tcl | iRule unit-test library; mocks plus executable rules. |
| irules-community / [f5](https://github.com/e-XpertSolutions/f5) | 2016; 2017-10-26 | MIT | F5 iRules | Historical community rules. |
| irules-community / [iRules](https://github.com/megamattzilla/iRules) | 2019; 2026-06-11 | NOASSERTION | F5 iRules | Active community collection, varied file naming. |
| irules-community / [f5-irules-json](https://github.com/JuergenMang/f5-irules-json) | 2024; 2026-05-13 | GPL-3.0 | F5 iRules | Recent JSON/iRules-focused patterns. |
| irules-community / [simple-sideband](https://github.com/pwhitef5/simple-sideband) | 2023; 2025-07-10 | Apache-2.0 | F5 iRules | Sideband networking callbacks and state. |
| irules-community / [HTTP Debug iRule](https://github.com/0xHiteshPatel/HTTP_Debug_iRule) | 2015; 2015-07-02 | GPL-2.0 | F5 iRules | Single historical debugging rule. |
| irules-community / [f5-irules](https://github.com/erkac/f5-irules) | 2021; 2023-09-21 | NOASSERTION | F5 iRules | Community snippets; limited recent activity. |
| private / `tcl-lsp-testsrc` (optional; see manifest) | 2026; 2026-07-04 | private; licence/provenance are access-controlled | F5 iRules aggregate | Publicly sourced iRule aggregate used only with credentials; do not enumerate, copy, or treat it as a public source catalogue. |

## Review and research candidates (not benchmark-pinned)

These candidates were identified during the #1701 callback review.  They are
not an endorsement to add every repository to a timing workload; first search
for distinct syntax and avoid vendored copies.  Dates again are GitHub metadata
observed on 2026-08-24 unless explicitly marked as a non-GitHub canonical
source.

| Source | Created; last activity observed | Licence/status | Intended-dialect guess | Why keep it in the research set / canonical-source note |
| --- | --- | --- | --- | --- |
| [jianiau/ezdit](https://github.com/jianiau/ezdit) | 2016; 2016-10-20 | NOASSERTION; historical candidate | Tcl/Tk 8.6 | Editor code: older Tk binding/callback style.  Do not mistake inactivity for incompatibility. |
| [tcltk/bwidget](https://github.com/tcltk/bwidget) | 2013; 2026-03-20 | NOASSERTION (BSD-style upstream); active | Tcl/Tk 8.4+ | Canonical megawidgets; option schemas and binding-heavy Tk code. |
| [tcltk/itcl](https://github.com/tcltk/itcl) | 2012; 2026-08-24 | NOASSERTION (Tcl-style upstream); active | \[incr Tcl\], Tcl/Tk | Third OO family: visibility wrappers, methods and object callbacks. |
| [gustafn/nsf](https://github.com/gustafn/nsf) (includes NX) | 2014; 2026-07-06 | NOASSERTION; active | NSF/NX, Tcl 8.6+ | Canonical maintained NSF/NX source; rich meta-object, filters and method dispatch. |
| [XOTcl canonical project](https://xotcl.org/) / [OpenACS xotcl-core](https://github.com/openacs/xotcl-core) | 2000; canonical site changed 2025-06-21; OpenACS mirror 2011; 2026-05-24 | upstream terms required; canonical distribution plus real-use mirror | XOTcl, Tcl 8.x | XOTcl's own release history begins in 2000; use the canonical distribution for language tests and the OpenACS mirror only for application usage.  Do not duplicate vendored XOTcl copies. |
| [StefanSchippers/xschem](https://github.com/StefanSchippers/xschem) | 2020; 2026-08-23 | NOASSERTION; active | Tcl/Tk + EDA DSL | Large current EDA application; UI and schematic-command style distinct from Vivado. |
| [tcltk/thread](https://github.com/tcltk/thread) | 2013; 2026-08-24 | NOASSERTION; active | Tcl Thread extension | Thread/event callback registration and cross-interpreter idioms. |
| [TclTLS canonical Fossil](https://core.tcl-lang.org/tcltls/) | 1997 code lineage; 2026-07-01 release | upstream `license.terms`; active | TclTLS extension, Tcl 8.6/9 | `tls::socket` server callbacks, fileevents and option routing.  Prefer the Fossil project or the maintainer's current release mirror over stale forks. |
| [tcl-mirror/tclhttpd](https://github.com/tcl-mirror/tclhttpd) | 2016; 2020-01-06 | NOASSERTION; mirror, inactive | TclHttpd / Tcl 8.x | Web-server callback and URL-dispatch corpus; mirror only, retain canonical project provenance. |
| [TclRAL canonical Fossil](http://chiselapp.com/user/mangoa01/repository/tclral) / [GitHub mirror](https://github.com/tcl-mirror/tclral) | 2006; 2017-08-01 release; mirror 2019; 2021-03-03 | upstream terms required; canonical Fossil plus mirror | TclRAL / Raloo relational Tcl | Relation/value DSL and package style.  Raloo is an associated programming style, not a second copy to pin: use the canonical TclRAL source once. |
| [tcl-mirror/tclws](https://github.com/tcl-mirror/tclws) | 2015; 2021-12-19 | NOASSERTION; mirror/candidate | Tcl web services | SOAP/web-service callbacks and generated-looking Tcl; use canonical provenance. |
| [tcler/wub](https://github.com/tcler/wub) | 2015; 2019-03-08 | NOASSERTION; optional/inactive | Wub web server | Optional historical web/server callback corpus; do not include by default without a distinct pattern census. |
| [Dash-OS/tcl-modules](https://github.com/Dash-OS/tcl-modules) | 2017; 2018-06-06 | MIT; historical | Tcl 8.x modules | Small modular package/application patterns. |
| [Dash-OS/tcl-cluster](https://github.com/Dash-OS/tcl-cluster) | 2017; 2017-10-23 | MIT; historical | Tcl cluster/distributed code | Remote/event-oriented command dispatch; include only if it contributes non-duplicated shapes. |
| [flightaware/tclrmq](https://github.com/flightaware/tclrmq) | 2017; 2021-08-19 | BSD-3-Clause; candidate | Tcl/RabbitMQ extension | Message-consumer callbacks and extension API style. |
| [tcltk/tdbc](https://github.com/tcltk/tdbc) | 2012; 2014-03-20 | NOASSERTION; historical GitHub mirror | Tcl database connectivity | Handle commands and callback-adjacent database APIs; first establish maintained canonical location. |
| [jdc8/tclzmq](https://github.com/jdc8/tclzmq) | 2012; 2021-12-20 | NOASSERTION; candidate | Tcl/ZeroMQ extension | Socket callback and event-loop API patterns. |
| [AthenaModel/athena](https://github.com/AthenaModel/athena) | 2015; 2016-02-29 | NOASSERTION; historical | Tcl/Tk application | Larger application/object UI coding style; assess before harvesting. |
| [tcltk/tclapps](https://github.com/tcltk/tclapps) | 2012; 2026-08-24 | NOASSERTION; active core mirror | Mixed Tcl/Tk 8.4–8.6, classic Tk and early ttk | Distinct historical application collection (`tkchat`, `tkvnc`, games, timers and IRC): direct widgets, canvas/text, menus, bindings, timers, sockets and legacy platform idioms. |
| [phase1geo/tke](https://github.com/phase1geo/tke) | 2020; 2024-02-09 | GPL-2.0; maintained but infrequently pushed | Tcl/Tk 8.6, Tcllib/Tklib, TkDND | Large editor application: text tags, notebook/panes, virtual events, bindtags, custom styles, image/font lifetimes, platform branches and TclOO-based bundled plugins. |
| [j6t/git-gui](https://github.com/j6t/git-gui) | 2024; 2026-07-16 | NOASSERTION in GitHub metadata; verify canonical Git licensing | Contemporary Tcl/Tk / Wish, classic plus ttk | Production Git GUI: custom `class` language, dialogs, `tkwait`, `fileevent`, deferred work, validation callbacks, virtual events and platform-specific window management. |
| [flightaware/piaware-gui](https://github.com/flightaware/piaware-gui) | 2020; 2026-08-13 | BSD-2-Clause; active | Tcl/Tk 8.6-era embedded Linux, Itcl | Compact live-device dashboard: periodic update loops, socket/fileevent callbacks, dynamic keypad construction and reconfiguration, named images/fonts, ttk layouts and modal lifecycle. |
| [tcltk-depot/tkpath](https://github.com/tcltk-depot/tkpath) | 2017; 2026-05-11 | BSD-style README; active maintenance fork | Tcl/Tk 8 with Tcl/Tk 9 fixes | SVG-like path/canvas extension and demos: canvas-item bindings, image resources, direct literal `pack -in`, and platform drawing backends. |
| [johnbuckman/androwish-tcl91](https://github.com/johnbuckman/androwish-tcl91) | 2026; 2026-08-07 | NOASSERTION; experimental forward port | AndroWish / Tcl-Tk 9.1 on Android | Mobile-specific smoke source for SDL window-manager emulation, DPI/scaling, bundled fonts, `wm`, event-loop timing and console/intent launch boundaries.  Use the canonical AndroWish distribution for broad application corpus, not this port alone. |
| [zdia/gorilla](https://github.com/zdia/gorilla) | 2010; 2026-03-07 | GPL-2.0-or-later in `sources/LICENSE.txt`; active 2026 | Tcl/Tk 8.x, Itcl 3-era application | Password manager with real Itcl use, ttk Treeview, images, status timers and progress UI.  Search application/modules; exclude its vendored `sources/itcl3.4` when counting language-runtime patterns. |
| [SAOImageDS9/SAOImageDS9](https://github.com/SAOImageDS9/SAOImageDS9) | 2019; 2026-08-24 | GPL-3.0; active | Tcl/Tk scientific application, classic plus ttk | Very large astronomical UI.  Scope to `ds9/library`; exclude vendored `tk9` and `tklib`.  Distinctive frames/dialogs, menu callbacks, timer-driven cube playback, grid constraints and dynamic scientific display controls. |
| [totalspectrum/flexprop](https://github.com/totalspectrum/flexprop) | 2018; 2026-08-10 | MIT; active | Tcl/Tk 8.6-era desktop tool | Propeller development GUI: terminal `fileevent`, timed updates, bindtag insertion/removal, virtual events, text/editor tags, dynamic fonts and tooltip callback composition.  Scope to application `src`; exclude vendored `tcl_library`. |
| [hvxl/otmonitor](https://github.com/hvxl/otmonitor) | 2018; 2026-08-15 | LGPL-3.0; active | Tcl/Tk 8.5-era Starkit/Thread application | Embedded OpenTherm monitor.  Scope to `otmonitor.vfs` application files and exclude bundled library copies under `otmonitor.vfs/lib/tcl8`; distinctive serial/socket `fileevent`, timer cancellation, coroutine-oriented upgrade flow, named resources, ttk styles and dialogs. |
| [BRL-CAD](https://github.com/BRL-CAD/brlcad) | 1979 lineage; 2026-08-24 | LGPL-2.1-or-later umbrella with identified BSD/public-domain and third-party components; active | BRL-CAD host Tcl/Tk, Itcl/Itk/Iwidgets | Scope to `src/tclscripts` application modules and `src/archer`; exclude `hv3`, bundled tcllib, `tkcon.tcl`, images and generated data.  Itcl classes, `itk_component`, callback options, dynamically named components and multi-pane CAD UI supply strong host-command and megawidget patterns. |
| [Magic](https://github.com/RTimothyEdwards/magic) | 1985-or-earlier lineage; 2026-08-07 | permissive UC/Stanford-style root licence; active | Magic host API, Tcl/Tk 8.2+ with current Tcl/Tk 8.6/9 compatibility work | Scope to `tcltk`, excluding the vendored `tkcon.tcl`.  Dynamic `${framename}` widget paths, `event generate`, socket `fileevent`, layer images and timer-driven EDA UI complement xschem without duplicating its command dialect. |
| [XCircuit](https://github.com/RTimothyEdwards/XCircuit) | 1993 lineage; 2026-07-26 | GPL-2.0; active | XCircuit host API, classic Tcl/Tk 8.4-era style | Scope to `lib/tcl`, excluding `tkcon.tcl` and copied wrapper support shared with Magic.  Dynamic widget trees, parameter/schematic dialogs, `event generate` and ngspice integration expose a distinct schematic-editor command surface. |
| [FlightAware Scotty/Tkined](https://github.com/flightaware/scotty) | 1993 lineage; 2020-10-07 | Tcl/Tk-style permissive `license.terms`; historical | Tnm/Tkined network-management dialect, Tcl/Tk 8.5–8.6 | Include `tkined/{apps,library}` and `tnm/{library,agents,examples}`; exclude C/build artefacts.  Custom `ined` RPC, `job create -command`, SNMP handlers, sockets, `fileevent` and `vwait` make this a primary non-OO callback-DSL source. |
| [XCrySDen](https://github.com/ssajeewa-perera/XCrySDen) ([canonical release site](http://www.xcrysden.org/Download.html)) | 1990s lineage; 1.6.2 released 2019-10-28, still downstream-packaged in 2026 | GPL-2.0-or-later | Tcl/Tk 8.6, Togl 2 and BWidget scientific UI | Include `Tcl` and `examples/Scripting`; exclude `external`, bundled libraries, binaries and scientific data.  Togl widgets, variable traces, external-process `fileevent`, image/font lifetimes and pack/place `-in` exercise scientific visualisation idioms. |
| [TkGate](https://github.com/bnoordhuis/tkgate) | 1987 lineage; 2022-02-15 | GPL-2.0-or-later; historical | Classic Tcl/Tk 8.0–8.4 plus TkGate host commands | Scope to `scripts/*.tcl`.  Old global/string callback construction, `$w.child` paths, circuit-canvas host commands, `bindtags` and timers provide valuable compatibility cases absent from modern ttk applications. |
| [andrey-nakin/tcl-measure](https://github.com/andrey-nakin/tcl-measure) | 2011; 2020-10-23 | MIT; historical | Tcl 8.4/8.5, Tk/ttk and laboratory-device DSLs | Include `measure`, `hardware` and focused tests; exclude vendored `thirdparty`.  Inline widget constructors inside geometry calls, `$prefix.child`, validation `%P`, timer cancellation, socket/HTTP `fileevent`, SCPI and Modbus cover compact industrial-control idioms. |
| [decentespresso/de1app](https://github.com/decentespresso/de1app) | 2020s; 2026-08-18 | GPL-3.0; active | AndroWish/Tcl-Tk 8.6 on Android | Scope to project Tcl/UI sources; exclude submodules.  Mobile UI DSLs, callback dictionaries, themed controls, timers and device/network event handling exercise generated-looking widget paths and callback composition. |
| [AndroWish canonical Fossil](https://www.androwish.org/) | 1990s Tcl lineage; 2026-08-23 | Mixed upstream licences; review per subtree | AndroWish/Tcl-Tk 8.6–9.1 on Android/SDL | Use project/core examples and application scripts; exclude generated/build artefacts and third-party bundles.  Mobile window-manager emulation, DPI/scaling, intents, fonts, timers and platform-specific command wrappers complement desktop Tk. |
| [stepan-mitkin/drakon_editor](https://github.com/stepan-mitkin/drakon_editor) | 2010s; 2026-04-21 | Public domain application; exclude bundled PDF/fonts | Tcl/Tk 8.6 | Scope editor/application Tcl.  Stored callback dictionaries, generator DSLs, canvas diagrams, selection bindings and dynamic dialogs are valuable non-widget-factory UI patterns. |
| [mittelmark/oowidgets](https://github.com/mittelmark/oowidgets) | 2010s; 2026-07-28 | BSD-3-Clause; active | TclOO/Tk 8.6+ | Include package, samples and focused tutorial; exclude generated HTML.  `oowidgets::widget` and `new` form a megawidget DSL with callback-prefix helpers, inheritance, mixins and widget-wrapper command forwarding. |
| [MadScienceZone/gma-mapper](https://github.com/MadScienceZone/gma-mapper) | 2020s; 2026-08-05 | BSD-3-Clause; active | Tcl/Tk 8.6 | Scope application Tcl and canvas code; exclude packaged dependencies.  Canvas editing, network-fed state, dynamic paths, bind/event callbacks and incremental redraw expose an interactive mapping workload. |
| [TkCon canonical Fossil](https://core.tcl-lang.org/tkcon/) | 1990s lineage; 2026-07-04 | Permissive Tcl-style licence; canonical source | Tcl/Tk 8.5–9.0 | Scope TkCon application/library Tcl; exclude bundled support copies.  Console command dispatch, completion, text tags, history, `send`, timers and interpreter-facing callbacks cover a mature utility application. |
| [johnbuckman/iwish](https://github.com/johnbuckman/iwish) | 2010s; 2026-08-07 | BSD-style project scripts; active | Tcl/Tk 8.6 on iOS | Scope iWish project scripts; exclude platform/build vendoring.  Mobile embedding, orientation/window events, touch bindings, resource paths and application callbacks test non-desktop Tk assumptions. |
| [ambaker1/wob](https://github.com/ambaker1/wob) | 2010s; 2025-02-08 | MIT; historical | Tcl/Tk 8.6 | Include application Tcl and tests; exclude bundled examples not used by the app.  Child interpreters, `destroy`, variable traces, `fileevent`, socket/event dispatch and wrapper commands provide lifecycle and isolation patterns. |
| [rdbende/Sun-Valley-ttk-theme](https://github.com/rdbende/Sun-Valley-ttk-theme) | 2020s; 2025-06-08 | MIT; historical | Tcl/Tk 8.6 ttk theme | Scope theme Tcl and examples; exclude screenshots/assets.  Theme packages, style maps/layouts, element settings and dark/light switching are useful style-registry and option-database fixtures. |
| [sshlien/runabc](https://github.com/sshlien/runabc) | 1998 lineage; 2026-08-20 | GPL-2-or-later; active | Tcl/Tk 8.6-era classic application | Scope the application Tcl; exclude bundled music/fonts.  Monolithic editor structure, menus, text tags, keyboard bindings, dialogs and long-lived callback procedures represent an older but substantial GUI style. |
| [MaxPerl/ttk-Breeze](https://github.com/MaxPerl/ttk-Breeze) | 2020; 2020-06-28 | LGPL-2.1; historical | Tcl/Tk 8.6 ttk theme | Scope theme Tcl and package metadata; exclude binary/demo assets.  Theme element layouts, state maps and platform-specific colour options exercise declarative ttk styling. |
| [revarbat/TkCAD](https://github.com/revarbat/TkCAD) | 2000s lineage; 2026-08-24 | BSD-3-Clause; active candidate | Tcl/Tk 8.6 with CAD extensions | Scope Tcl application and canvas/UI scripts; exclude Critcl/C extensions and bundled tkpath.  Large canvas geometry, dynamic paths, font/glyph tools, extension fallbacks and tool-window callbacks provide a distinctive CAD workload. |
| [FRIBDAQ/SpecTcl](https://github.com/FRIBDAQ/SpecTcl) | 1990s lineage; 2026-08-24 | GPL; active scientific project | Tcl/Tk with Itcl/Itk/Tktable | Scope Tcl/Tk UI and focused examples; exclude generated/build products and vendored libraries.  Scientific control panels, Itcl classes, Itk components, table widgets, traces and acquisition callbacks cover a domain-specific desktop GUI. |
| [TkTable Redux canonical Fossil](https://core.tcl-lang.org/tktable/) | 1990s lineage; 2026-06-27 | Permissive Tcl-style licence; canonical source | Tcl/Tk 8.5–9 | Scope TkTable scripts/tests; exclude C/build output.  Table row/column options, bind classes, virtual events, `after` identifiers, option database and embedded windows exercise dense widget command structure. |
| [Scid vs PC](https://sourceforge.net/projects/scidvspc/) | 1990s lineage; 2026-08-01 | GPL-2.0; active | Tcl/Tk 8.5–8.6 plus C++ host commands | Scope Tcl GUI and tests; exclude engines, generated translations and large chess data.  Text-tag bindings, renames, docking/layout, themes, engine/fileevent callbacks and custom `sc_*` commands provide a mature mixed-language UI. |
| [Tkhtml3/Hv3 canonical Fossil](http://tkhtml.tcl.tk/fossil/) | 2000s lineage; 2024-04-16 mirror activity | LGPL-2.1; archival/canonical history | Tcl/Tk 8.6 HTML widget application | Scope Tkhtml/Hv3 Tcl wrappers and examples; exclude vendored Snit and browser data.  Renamed hull commands, megawidget forwarding, DOM callbacks, navigation events and asynchronous loading expose wrapper-heavy command resolution. |
| [Coccinella](https://github.com/coccinella/coccinella) | 2000s lineage; 2010-09-29 | GPL-3.0; historical | Tcl/Tk 8.5+ | Scope application Tcl; exclude bundled libraries and media.  `send -async`, whiteboard/canvas interaction, `fileevent`, aliases, XMPP callbacks, menus and theme integration capture a long-lived networked Tk application. |
| [TkTreeCtrl](https://tktreectrl.sourceforge.net/treectrl.html) | 2000s lineage; historical extension documentation checked 2026-08-25 | Tcl/Tk-style licence; verify the selected source release | Tk 8.x extension widget | `notify bind` is a distinct deferred callback language with event-specific `%` substitutions, global-scope execution, dynamic event installation and a callback that computes substitutions.  Use it as an extension/control fixture, not evidence for core Tk `bind`. |
| [Wcb and Tablelist](https://www.nemethi.de/) | 2000s lineage; Wcb 4.2 and Tablelist 7.10 site updated 2026-04-21 | Author-distributed Tcl packages; verify each archive's notice before ingestion | Pure Tcl/Tk 8.0+ with current Tcl 9 compatibility work | Widget-callback interception, validation wrapping, option forwarding and entry/combobox/spinbox overlays used for editable table cells.  This is a high-value megawidget callback source; keep Wcb and Tablelist version coupling visible in fixtures. |

Deferred after licence/provenance review: [PACKMOL-GUI](https://github.com/caiofreitasm/PACKMOL-GUI) is a useful current scientific Tk application but had no clear repository licence, while [reflectometry/reflpak](https://github.com/reflectometry/reflpak) mixes component notices without an unambiguous repository-wide grant.  Do not download either into a managed corpus unless that is resolved.

### Tk corpus-pattern mining

The following is a reproducible *pattern census*, not a benchmark workload.
On 2026-08-24, five research candidates (`tclapps`, TKE, Git GUI, PiAware GUI,
and TkPath) were fetched outside the repository and Tcl/TM files were searched
with anchored command-head patterns.  No upstream source, reduction, or index
was retained in this repository.  The conventional-constructor census found
**1,865** calls: **478 literal-path** constructors, **1,387
variable-derived-path** constructors.  The split is lexical (a pathname
beginning with `.` versus `$` at a recognised core/ttk constructor head), not a
claim that either form executes, and intentionally excludes custom factory
commands.

Other high-signal lexical counts were 182 `bindtags`, 145 `event generate`,
584 `after`, 68 `fileevent`, 536 `destroy`/withdraw/deiconify, 279 ttk style
layout/configure/map/element calls, 258 image/font create/delete calls, 494
`grid` configuration/query/removal forms, 54 non-placement `pack` forms, 43
non-placement `place` forms, and 83 direct geometry calls using `-in`.
Vendored and platform-specific copies in those source trees can contribute to
these counts, so they establish pattern presence rather than deployment scale.

The resulting implementation guidance is conservative: a static UI model may
show literal paths and direct geometry facts, but must report variable paths,
factory/wrapper DSLs, callback-built paths, lifecycle mutations, and
platform-conditional branches as uncertain rather than inventing a tree.  The
representative patterns to retain in focused fixtures are `grid .a .b -in .f`,
row/column configuration and propagate forms, `bindtags` insertion/removal,
virtual-event generate/consume pairs, `after`/`fileevent` cancellation,
post-constructor `configure`, named image/font/style lifetimes, custom
`class`/APave-like constructor indirection, and `destroy`/withdraw/recreate
dialog flow.  Registry descriptors—not consumer command-name branches—must
own any later modelling of those forms.

### iRulesLX paired Tcl/JavaScript candidates

iRulesLX needs a paired-language corpus: a Tcl iRule names a remote method in
`ILX::call` or `ILX::notify`, while JavaScript or TypeScript registers that
method on an `ILXServer`.  Harvesting only `.tcl` files loses the definition
side needed for cross-language navigation and arity checks.  These public
origins were verified on 2026-08-24; dates are GitHub creation and last-push
metadata, and licences are GitHub SPDX metadata unless stated otherwise.

| Source | Created; last push observed | Licence/status | Intended dialects | Distinctive paired patterns |
| --- | --- | --- | --- | --- |
| [ArtiomL/f5networks](https://github.com/ArtiomL/f5networks) | 2016; 2018-11-14 | MIT; historical | iRulesLX / Node.js, BIG-IP 12.1+ | Tutorial progression plus RADIUS, GraphQL and WebSocket examples; literal `ILX::call -timeout` names pair with `ILXServer.addMethod`. |
| [f5devcentral/f5-professional-services](https://github.com/f5devcentral/f5-professional-services) | 2022; 2026-08-06 | NOASSERTION; active vendor collection | iRules/iRulesLX / Node.js | Current risk-engine and Let's Encrypt pairs inside a much broader F5 corpus; select subtrees rather than benchmarking the whole repository. |
| [Netacea/f5-worker-template-typescript](https://github.com/Netacea/f5-worker-template-typescript) | 2020; 2026-06-30 | GPL-3.0; active | iRulesLX / TypeScript | Modern indirect handler registration, cached handles, repeated calls across HTTP events, and an explicit `RULE_INIT` availability caveat. |
| [f5se/bigip-ai-scenes-demo](https://github.com/f5se/bigip-ai-scenes-demo) | 2026; 2026-08-20 | NOASSERTION; active F5 example | Current iRulesLX / Node.js | The only real public Tcl `ILX::notify` call site found in this search: a static handle sends repeated one-way audit-log messages. |
| [Mikej81/f5-samlreplay](https://github.com/Mikej81/f5-samlreplay) | 2018; 2020-10-27 | NOASSERTION; historical | iRulesLX / Node.js | Multiple event handlers call hyphenated literal methods (`saml-request`, `saml-validate`) registered in the paired extension. |
| [Mikej81/f5-wsstar-ilx](https://github.com/Mikej81/f5-wsstar-ilx) | 2016; 2020-10-27 | NOASSERTION; historical | iRulesLX / Node.js | Several rule variants, one- and two-argument `ILX::init`, and hyphenated WS-Federation/WS-Trust method names. |
| [Mikej81/ILX-IDAM-AGS](https://github.com/Mikej81/ILX-IDAM-AGS) | 2016; 2016 | NOASSERTION; historical | iRulesLX / Node.js | LDAP RPC with three statically registered remote methods. |
| [Mikej81/f5-webcifs-ilx](https://github.com/Mikej81/f5-webcifs-ilx) | 2016; 2019 | NOASSERTION; historical | iRulesLX streaming / Node.js | Streaming-only `ILXPlugin`/flow example with no Tcl RPC method registration; useful negative control for cross-language RPC indexing. |
| [f5devcentral/f5-aws-lambda-proxy](https://github.com/f5devcentral/f5-aws-lambda-proxy) | 2019; 2021-03-30 | MIT; historical vendor example | iRulesLX / Node.js | Compact paired RPC example with literal plugin, extension and method names plus `catch`-based failure handling. |
| [gregcoward/f5-aws-apigw-proxy](https://github.com/gregcoward/f5-aws-apigw-proxy) | 2019; 2024 | MIT; maintained candidate | iRulesLX / Node.js | Dynamic REST proxy with asynchronous Node-side I/O; separates Tcl's blocking RPC boundary from Node's internal callbacks. |
| [akhmarov/f5_otp](https://github.com/akhmarov/f5_otp) | 2020; 2022-10-18 | Apache-2.0; historical | APM iRulesLX / Node.js | Plugin, extension, timeout and remote method can all come from `static::` variables; useful negative/unknown case for literal navigation. |
| [aknot242/apm-ilx-sql-challenge](https://github.com/aknot242/apm-ilx-sql-challenge) | 2019; 2019 | MIT; historical | APM iRulesLX / Node.js | Compact SQL-backed RPC and `catch` error path. |
| [islam-talaat/F5-IRule-LX-TCL-NodeJS](https://github.com/islam-talaat/F5-IRule-LX-TCL-NodeJS) | 2020; 2024 | NOASSERTION; maintained candidate | iRulesLX / Node.js | DNS response-bit manipulation with an uppercase/underscore remote method name. |
| [cloudadc/nodejs-honeypot](https://github.com/cloudadc/nodejs-honeypot) | 2020; 2021-03-23 | Apache-2.0; historical | iRulesLX / Node.js | Small raw-text and static-file workspaces; zero-user-argument calls and multiple registered response methods. |
| [cloudadc/nodejs-doh](https://github.com/cloudadc/nodejs-doh) | 2020; 2020 | Apache-2.0; historical | iRulesLX / Node.js | DNS-over-HTTPS resolver RPC; protocol payload handling around the method boundary. |
| [ximeng890726/DoHDotiRulesLX](https://github.com/ximeng890726/DoHDotiRulesLX) | 2021; 2021 | NOASSERTION; historical | iRulesLX / Node.js | Paired DoH GET, POST and UDP variants; literal mixed-case/underscore remote methods. |
| [johnalam/F5-iRules-LX-MSSQL](https://github.com/johnalam/F5-iRules-LX-MSSQL) | 2017; 2017 | NOASSERTION; historical | APM iRulesLX / Node.js | SQL RPC with duplicate Tcl variants; deduplicate before counting. |
| [steveh565/ilx-zlib](https://github.com/steveh565/ilx-zlib) | 2019; 2020-07-10 | NOASSERTION; historical | iRulesLX / Node.js | Calls embedded inside `lindex` and `catch`, multiple rule variants, and vendored `f5-nodejs`; exclude vendored runtime copies from counts. |
| [codygreen/F5-MFA](https://github.com/codygreen/F5-MFA) | 2016; 2017-12-07 | NOASSERTION; archived | APM iRulesLX / Node.js | Four literal remote methods spanning enrollment and verification, with handles created in different event lifetimes. |
| [bepsoccer/iRulesLX](https://github.com/bepsoccer/iRulesLX) | 2017; 2017-04-27 | MIT; historical | APM iRulesLX / Node.js | Route53 update RPC with a larger fixed argument vector and repeated call shape. |
| [colin-stubbs/f5-bigip-ilx-api](https://github.com/colin-stubbs/f5-bigip-ilx-api) | 2019; 2019-02-13 | Apache-2.0; archived | iRulesLX / Node.js | Deliberately dynamic remote method selected from an HTTP path; paired `addMethod` names look like URI paths and must not be mistaken for Tcl commands. |

The manifest-pinned optional private aggregate was also checked at its exact
manifest revision.  It contains 21 Tcl files mentioning ILX commands: 26
`ILX::init` occurrences, 33 `ILX::call` occurrences, and no `ILX::notify`
occurrence.  Those are physical occurrence counts, not distinct applications;
the aggregate contains public-source mirrors.  Its upstream inventory remains
private, and this document intentionally names only independently verified
public origins.  A paired research sweep should fetch the public repositories
above to a disposable location rather than copy their JavaScript into the
aggregate or this repository.

A separate public iRulesLX research sweep over 21 repositories found 28 Tcl
files and 38 non-vendored JavaScript files, with 33 `ILX::init`, 40
`ILX::call`, 17 `ILXServer`, and 29 `addMethod` occurrences.  Six calls used
`-timeout` and 15 were wrapped in `catch`; no real Tcl `ILX::notify` occurred
in that particular set.  Counts include variants and documentation snippets,
so they describe syntax coverage rather than deployments.  The separately
listed `f5se/bigip-ai-scenes-demo` adds the verified public notify pattern.

### Source collections and fixture references

These are sources for small, attributable fixtures and pattern taxonomy, not
repositories to benchmark wholesale.

| Collection | Status / age and recency observed | Pattern value |
| --- | --- | --- |
| [Tcl Wiki: callback](https://wiki.tcl-lang.org/page/callback), [command prefix](https://wiki.tcl-lang.org/page/command%2Bprefix), and [bindings and variable substitution](https://wiki.tcl-lang.org/page/Bindings%2Band%2Bvariable%2Bsubstitution) | Community pages; the search index observed updates in 2026, with material spanning older Tcl eras | Defines list-built prefixes, bind `+script`, appended arguments, and define-time vs fire-time substitution. |
| [CGI Coffee: Develop Cross-Platform CLI and GUI Tools With Tcl/Tk](https://cgicoffee.com/blog/2026/04/tcl-tk-develop-cross-platform-cli-gui-tools-tutorial-guide) | Tutorial published 2026-04-14; checked 2026-08-25 | Contemporary Tcl 9.0/Tk and ttk tutorial, with Tcl 8.6 compatibility context. Mine its mortgage-calculator modernization patterns—`grid`, validation, DPI-aware relative sizing, rate-limited canvas resize, namespaces/arrays and cross-platform layout—and its explicit code/data security warning. These motivate static preview uncertainty, callback timing/taint diagnostics and AI guidance; examples remain secondary evidence to official manuals and Tk source. |
| [TclOO callback discussion](https://stackoverflow.com/questions/47527830/tcloo-private-method-and-method-as-callback) | 2017 answer, checked 2026-08-24 | `namespace code [list my ...]`, public/exported-method boundary and object callback construction. |
| [TIP 379](https://core.tcl-lang.org/tips/doc/trunk/tip/379.md) and [TIP 419](https://core.tcl-lang.org/tips/doc/trunk/tip/419.md) | 2011/2012-era TIPs, current canonical TIP archive observed 2026-08-24 | Explicit command-prefix contracts and Tk binding substitution/append semantics. |
| [Tcl Wiki: grid](https://wiki.tcl-lang.org/page/grid), [place](https://wiki.tcl-lang.org/page/place), and [pack configure](https://wiki.tcl-lang.org/page/pack%2Bconfigure) | Community reference pages checked 2026-08-24; material spans classic and current Tk | Small fixtures for complete geometry-manager operation families, `-in` changing the effective master, grid row/column policy, and the important distinction that `place` does not propagate a requested size to its container. |
| [Tcl Wiki: Megawidget](https://wiki.tcl-lang.org/page/Megawidget), [Namespace MegaWidgets](https://wiki.tcl-lang.org/page/Namespace%2BMegaWidgets), and [oowidgets](https://wiki.tcl-lang.org/page/oowidgets) | Community pages; historical material checked 2026-08-24 | Primary fixture references for composite-widget construction, namespace/wrapper command forwarding, TclOO megawidget bodies, hull paths and option propagation. |
| [Tcl Wiki: ttk themes](https://wiki.tcl-lang.org/page/ttk+themes) | Community page; material checked 2026-08-24 | Theme/style fixture reference: `ttk::style`, element layouts, state maps, theme switching and classic Tk/ttk option-database boundaries. |
| [Tcl Wiki: send](https://wiki.tcl-lang.org/page/send) | Community page; material checked 2026-08-24 | Inter-interpreter and remote-command fixture: synchronous versus `-async` dispatch, script/list quoting, error boundaries and security-sensitive command transport. |
| [Tcl Wiki: dynnotebook](https://wiki.tcl-lang.org/page/dynnotebook) | Community page; material checked 2026-08-24 | Dynamic notebook fixture: tab creation/removal, widget-path variables, command-prefix callbacks and geometry/lifecycle mutations that defeat a purely lexical preview. |
| [Tcl Wiki: Tk & Ttk Integration](https://wiki.tcl-lang.org/page/Tk%2B%26%2BTtk%2BIntegration) | Community page updated 2011; checked 2026-08-24 | Cross-style fixture for option database versus ttk styles, `<<ThemeChanged>>` bindings, class-specific configuration and live theme propagation. |
| [TIP 21](https://core.tcl-lang.org/tips/doc/trunk/tip/21.md) and [TIP 518](https://core.tcl-lang.org/tips/doc/trunk/tip/518.md) | TIP 21 final for Tk 8.4; TIP 518 final from the Tk 8.7/9.0 development era; canonical archive checked 2026-08-24 | Geometry fixtures for asymmetric external padding and `<<NoManagedChild>>`, the lifecycle event produced when the last managed child is forgotten, removed or destroyed. |
| [TIP 248](https://core.tcl-lang.org/tips/doc/trunk/tip/248.md) | Final for Tcl/Tk 8.5; canonical archive checked 2026-08-24 | Primary Ttk integration and release-axis fixture, including the additional `::tk::*` names for classic widget constructors and the themed-widget command surface. |
| [TIP 556: Tko](https://core.tcl-lang.org/tips/doc/trunk/tip/556.md) and [TIP 560: TclOO megawidget configuration](https://core.tcl-lang.org/tips/doc/main/tip/560.md) | Deferred/draft Tk 9 proposals created 2019–2020; canonical branches and documents checked 2026-08-25 | Proposed Tk 9 / TclOO dialect, not shipped core behaviour | Research fixtures for OO-like widget constructors, class/object option mutation, wrapped hull commands, callback-valued options and the distinction between object command names and widget paths.  Keep these explicitly future/proposal-gated. |
| [TclOO: Past, Present and Future](https://www.tclcommunityassociation.org/wub/proceedings/Proceedings-2009/proceedings/tcloo/TclOO_Past_Present_Future.pdf) and [Adventures in TclOO](https://www.tclcommunityassociation.org/wub/proceedings/Proceedings-2010/DonalFellows/Adventures-in-TclOO.pdf) | 2009/2010 conference papers, archival sources | Historical TclOO idioms worth reducing to fixtures, not copying wholesale. |
| [F5 Community iRulesLX search](https://community.f5.com/search?q=iRulesLX+ILX+call), [Getting Started part 3](https://community.f5.com/kb/technicalarticles/getting-started-with-irules-lx-part-3-coding--exception-handling/276218), and [Twilio paired example](https://community.f5.com/kb/codeshare/send-an-one-time-password-otp-via-the-twilio-sms-gateway/291444) | Material from 2016 onward; search and pages checked 2026-08-24 | Real paired Tcl/Node snippets, timeout/error handling, the payload limit, and event-context usage.  Search hits mentioning that `ILXServer.listen()` receives both call and notify traffic are not evidence of a Tcl `ILX::notify` call. |
| [F5 iRulesLX API reference](https://clouddocs.f5.com/api/irules-lx/APIReference.html), [`ILX::call`](https://clouddocs.f5.com/api/irules/ILX__call.html), and [`ILX::notify`](https://clouddocs.f5.com/api/irules/ILX__notify.html) | Product documentation updated in 2026; commands introduced in BIG-IP 12.0/available generally in 12.1 and deprecated in BIG-IP Next 20.0.1 | Primary semantic oracle for RPC direction, timeout option, return shapes, payload limits, availability and deprecation; use Community/GitHub only for usage patterns. |

## Callback semantic taxonomy

Corpus searches must distinguish callback *semantics*, not merely search for a
word such as `command` or `script`.  The Tcl Wiki, core manuals, Tcl/Tk source,
tcllib, and iRules corpus show at least these families:

- **Complete scripts** such as `after`, `fileevent`, `chan event`, and
  `package ifneeded`.  They may be built with `list`, but the consumer executes
  a whole script and does not necessarily append arguments.
- **Command prefixes with appended arguments** such as `lsort -command`,
  `regsub -command`, `socket -server`, `fcopy -command`, Tk widget callbacks,
  and many trace registrations.  Their static contract includes the number or
  shape of arguments supplied by the consumer.
- **Namespace-wrapped or lambda prefixes**, commonly `namespace code [list
  ...]` and `apply`.  The wrapper affects command resolution and must not be
  flattened into an ordinary script by spelling alone.
- **Stored and composed callbacks** placed in variables, arrays or dicts and
  later invoked with `{*}`, `eval`, `uplevel`, or a package-specific dispatcher.
  These require data-flow proof rather than local syntax recognition.
- **Protocol callbacks** such as reflected channels (`chan create` / `chan
  push`), whose receiver supplies a method name and method-specific arguments.
- **Resumable commands** created by `coroutine`.  These participate in deferred
  and event-driven programs, but are continuations rather than ordinary
  fixed-arity callbacks.
- **Event handlers and event producers** in iRules.  `when EVENT` owns a
  structural event body; data collection and notification commands can cause a
  later event, but they do not receive a Tcl command prefix.
- **Remote calls and notifications** such as iRulesLX `ILX::call` and
  `ILX::notify`.  These target JavaScript extension methods, not Tcl callbacks,
  and need cross-language rather than TclOO method navigation.

The existing registry describes much of the structural surface with
`ArgRole::Body`, `ArgRole::CommandPrefix`, appended arity, and
builds/wraps-command-prefix traits.  Future registry work should preserve the
distinctions above and, where diagnostics need them, add typed timing,
execution-context, error-routing, repeatability/cancellation, or protocol
metadata.  It should not teach an analyser or LSP consumer a list of command
spellings.

### Normalized callback census

The 35 machine-pinned public repositories were scanned by source/test-like
file and content hash.  Of 3,763 files, 3,681 were unique within their corpus
family: 425/424 iRules, 473/456 EDA, 2,067/2,023 tcllib+tklib, 728/708 Tk and
applications, and 70/70 other Tcl.  The 2.2% duplicate rate is small, but raw
occurrence counts below remain lexical lower bounds rather than semantic call
counts.

Exact prefix-builder substitutions found were 3 `[callback ...]`, 361
`[mymethod ...]`, 33 `[myproc ...]`, 7 `[mytypemethod ...]`, 2 `[itcl::code
...]`, and 172 `[namespace code ...]`.  Anchored command-head searches found:

| Corpus family | `after` | `fileevent` | `trace` | `bind` | `interp alias` | `coroutine` | `fcopy` |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| iRules | 6 | 0 | 0 | 0 | 0 | 0 | 0 |
| EDA | 10 | 3 | 4 | 0 | 20 | 0 | 0 |
| tcllib + tklib | 284 | 118 | 129 | 1,368 | 451 | 22 | 42 |
| Tk + applications | 394 | 5 | 57 | 1,459 | 50 | 10 | 0 |
| other Tcl | 1 | 0 | 0 | 0 | 36 | 0 | 0 |

The census also found about 3,056 callback-shaped `{*}$value` expansions,
1,098 dynamic `uplevel` calls, and 196 dynamic `eval` calls.  Those figures
argue against treating every list or string as executable.  Analysis should
start at registry-declared sinks and builders, memoize decoded prefix spans,
and use a bounded backward slice for stored prefixes.

The most important registry gap is a typed prefix-result descriptor.  The
current `BUILDS_COMMAND_PREFIX` flag says that builder argument zero becomes
the returned command head; that is false for TclOO `callback`/`mymethod`, snit
`mymethod`/`myproc`/`mytypemethod`, and `itcl::code`, where an object or family
dispatcher is implicit.  A boolean flag would therefore create incorrect
navigation.  A future descriptor needs to distinguish direct head arguments,
current-object methods, snit instance/type methods, and Itcl object/method
forms.  Callback slots also need an invocation-phase axis separate from
argument shape: immediate/reentrant, stored/deferred, conditionally deferred,
blocking external RPC, and fire-and-forget external dispatch.

Additional high-confidence audits are HTTP/WebSocket callback options,
tcllib's cron/FTP/RC4/textutil-patch/uevent/namespacex handlers, Tk subcommand
deferral (`canvas bind`, `wm protocol`, `chan event`), and EDA `-rule_body` /
`-command` options.  iRulesLX commands must instead describe external method
dispatch: `ILX::init` is a one- or two-argument handle producer, while
`ILX::call` and `ILX::notify` require a handle and method after their options.
Their method slots are neither Tcl bodies nor Tcl command prefixes.

Two concrete audit leads came from this review: `interp bgerror` appears to
supply exactly two arguments to its handler on supporting Tcl releases, and
tcllib's `fileutil::updateInPlace` invokes a supplied command prefix after
appending the file contents.  Both require primary-source/release verification
before changing registry data.  Generated tcllib rows describing callbacks
(including websocket, HTML parsing, and channel utilities) also need a
descriptor-coverage audit rather than broad name-based inference.

The normalized iRules source corpus contains 742 unique `when EVENT` handlers,
seven `clientside` blocks, six anchored `after` calls, and 16 TCP/HTTP/SSL collect
operations that lead to later event handling.  It contains no `ILX::init`,
`ILX::call`, or `ILX::notify` example.  Raw counts are physical-file counts:
the one known duplicate is an SSL Orchestrator ICAP rule variant, and the
agility-lab ILX material is documentation rather than source.  Registry-owned
command-to-event provenance is tracked separately from Tcl callback-prefix
analysis.

## Callback-pattern coverage for #1701

The #1701 regression is a deliberately narrow, statically knowable
list-built command prefix whose receiver is `[self]` or `[self object]`.
It is recognised in registry-declared `ArgRole::Body` and
`ArgRole::CommandPrefix` positions.  The wider corpus taxonomy is role-based:
candidate callback positions must be declared as **`ArgRole::Body` or
`ArgRole::CommandPrefix`**, never inferred from a command spelling.  The
following matrix keeps future corpus searches honest about what has and has not
been covered; composed, wrapped, stored, quoted, and dynamically constructed
prefixes remain research targets unless a corresponding registry contract and
regression are landed.

| Pattern | Registry role / form | Fixture/corpus search status | Expected static treatment |
| --- | --- | --- | --- |
| `bind $w <Event> [list [self] method %x %y]` | `ArgRole::Body` | #1701 regression | Reference to a public method; rename/find-references must include it. |
| `[list [self object] method ...]` | `ArgRole::Body` | #1701 regression parity | Same as `[self]`. |
| `lsort -command [list [self] compare] ...` | `ArgRole::CommandPrefix` | #1701 regression parity | Same exact list-built object-method target; the consumer appends arguments. |
| Other generic command-prefix consumers (`socket -server`, widget `-command`, trace, hook) | `ArgRole::CommandPrefix`, as declared | Search candidates/Wiki/TIPs | The exact #1701 shape is recognised when both the role and builder trait prove it; audit registry coverage and composed forms separately. |
| `namespace code [list my method ...]` | Wrapper around Body/CommandPrefix semantics | TclOO callback source collection | Separate wrapper/prefix form; add a focused fixture when resolver semantics are modelled. |
| `callback` / `mymethod` helper APIs | Deferred-method builder semantics, not a consumer slot | Corpus search target | Do not infer from a helper name; require a registry-owned target descriptor or a proven wrapper model. |
| Quoted script, `bind ... +script`, or additive/concatenated prefix | Body or CommandPrefix, but not a single list-built prefix | Wiki/TIP 419 search target | Not equivalent to the #1701 form; remain conservative until represented. |
| Stored then later invoked/mutated prefix (`set cb [list ...]`, `lappend`, `eval`, `{*}$cb`) | Value/data-flow, not a direct role-local form | Corpus search target | Data-flow problem; not a direct reference unless a future analysis proves it. |
| Inert list value (`set x [list [self] method]`) | Not Body/CommandPrefix | #1701 negative regression | Not a callback reference. |
| Shadowed `list` command | Body/CommandPrefix position but wrong resolved builder | #1701 negative regression | Not a list-builder reference; command identity must be registry-resolved. |
| Unexported/private target captured through object command | `ArgRole::Body` | #1701 negative regression | Not externally dispatchable; do not claim it as a callback reference. |

When adding a result from any row above to #1181, state: canonical source,
commit/release used, licence check, intended dialect, discovered pattern,
whether it duplicates an existing source, and whether it is proposed as a
micro-fixture, a manual sweep input, or a benchmark-pin change.
