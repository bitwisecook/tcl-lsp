# Corpus expansion — external Tcl libraries for spec-DSL design

> Working note for [spec-packs.md](../../spec-packs.md) phase 3 ("external
> corpus": draft specs for uncovered libraries; every construct the DSL
> cannot express is a design bug filed against phase 1). Surfaced while
> designing the command-spec DSL, alongside a survey of new test-corpus
> candidates for [issue #1181](https://github.com/bitwisecook/tcl-lsp/issues/1181).
> Not committed/pushed — scratch design material.

Two deliverables:

- **Task A** — a table of 36 verified, real, licensed Tcl/Tcl-adjacent
  libraries not already in #1181's corpus, each tagged with the
  command-spec surface it stresses.
- **Task B** — 16 concrete hook-pattern exemplars pulled from cloned source
  and docs (mix of the new libraries above and the existing #1181 corpus),
  each a real public-API command that cannot be expressed as declarative
  spec data.

Repos referenced in Task B were shallow-cloned into
`scratchpad/corpus2/` for this survey (not part of the repo tree).

---

## Task A — new library candidates (verified)

Verification method: GitHub repository-search API (existence, license,
stars, size, language) plus WebSearch/WebFetch for anything not on GitHub.
"Approx size" is the GitHub-reported repo size where available, or working-
tree size (`du -sh`, `.git` excluded) for the subset cloned locally for
Task B (marked *cloned*). Licence is the GitHub-detected SPDX id; "not
SPDX-classified" means GitHub found a LICENSE file but couldn't map it to a
standard identifier (own-text/custom licences), not that the project is
unlicensed — check the repo's `LICENSE` file for the real terms before
relying on this table.

### apnadkarni (Ashok P. Nadkarni) — FFI, Windows systems programming, misc.

| Name | URL | Licence | Approx size | Command-spec surface stressed |
|---|---|---|---|---|
| tcl-cffi | https://github.com/apnadkarni/tcl-cffi | BSD-2-Clause | 2.6 MB *(cloned: 3.3 MB working tree)* | Huge per-parameter annotation surface (`in`/`out`/`inout`/`byref`/`nullok`/`chars[n]`…) on `cffi::Wrapper function`/`cffi::prototype`; varargs calling convention; cross-referencing param names in size annotations |
| tcl-promise | https://github.com/apnadkarni/tcl-promise | not SPDX-classified (no LICENSE detected) | 67 KB *(cloned: 136 KB)* | Callback command-prefixes with different appended arity per slot (`then`'s fulfil vs. reject callbacks); `promise::async` synthesises a new `proc` at call time |
| tarray | https://github.com/apnadkarni/tarray | BSD-2-Clause | 2.6 MB *(cloned: 4.9 MB)* | Typed bulk-column ops (`column search -eq/-gt/… -all/-inline/-bitmap`) where mode + shape flags combinatorially change the *return* format |
| iocp | https://github.com/apnadkarni/iocp | BSD-2-Clause | 1.5 MB | Channel-option surface for Windows IOCP-backed channels; version-gated (Windows-only) API |
| woof | https://github.com/apnadkarni/woof | not SPDX-classified | 7.5 MB | MVC web framework — convention-based dispatch (URL → controller/action method resolution), ensemble-heavy |
| tcl9-migrate | https://github.com/apnadkarni/tcl9-migrate | not SPDX-classified | 428 KB | Tcl-9 migration tooling (the exact "tcl9 migration tools" ask) — static scanners driven by version-gated command tables |
| tcl-libgit2 | https://github.com/apnadkarni/tcl-libgit2 | BSD-2-Clause | 600 KB | libgit2 binding — huge option surface (git plumbing flags), opaque handle commands (repo/commit/tree objects) |
| twapi | https://github.com/apnadkarni/twapi | not SPDX-classified | 21.6 MB | Very large Windows API surface — hundreds of thin C-level wrapper commands, many with structured-record in/out params |
| blt | https://github.com/apnadkarni/blt | not SPDX-classified (fork of BLT, own licence) | large (C, fork of SourceForge BLT) | Classic BLT graph/widget toolkit — huge megawidget option surfaces, `vector`/`graph` ensembles |
| tix | https://github.com/apnadkarni/tix | not SPDX-classified (fork of Tix, own licence) | large (C, fork of SourceForge Tix) | Legacy Tix widget set — huge composite-widget option surfaces |
| tktreectrl | https://github.com/apnadkarni/tktreectrl | not SPDX-classified (fork of TkTreeCtrl) | medium (C, fork of SourceForge TkTreeCtrl) | TreeCtrl widget — item/style/column sub-ensembles with per-column option surfaces |
| tcl-cmark | https://github.com/apnadkarni/tcl-cmark | BSD-3-Clause | 1.6 MB | CommonMark/GFM Markdown bindings — parser option surface, AST-node opaque-handle commands |

### rkeene (Roy Keene) — Tclkit / crypto / systems tooling

| Name | URL | Licence | Approx size | Command-spec surface stressed |
|---|---|---|---|---|
| TclTLS | https://github.com/rkeene/TclTLS | not SPDX-classified | 594 KB *(cloned: 512 KB)* | `tls::socket` — presence of `-server` changes the legal option set, positional arity (`host port` vs. just `port`), *and* wraps the caller's callback in an internal indirection; per-option routing table dispatches to `socket` vs. `tls::import` option buckets |
| KitCreator | https://github.com/rkeene/KitCreator | not SPDX-classified | 912 KB | Tclkit build system — shell/Tcl build-script DSL, not command-surface heavy but version-gated (8.4–8.6/9) build matrix |
| hunter2 | https://github.com/rkeene/hunter2 | MIT | — | Script-oriented password manager; small but real Tcl app corpus, `expr`/`vfs`-heavy |

### Build / C-embedding DSLs

| Name | URL | Licence | Approx size | Command-spec surface stressed |
|---|---|---|---|---|
| critcl | https://github.com/andreas-kupries/critcl | not SPDX-classified (own BSD-style licence) | 16 MB *(cloned: 11 MB)* | `critcl::cproc`/`critcl::cdata`/`critcl::argtype` — a whole embedded C-declaration DSL where argument-type tokens select C signature *and* Tcl marshalling code; 94★, active |

### tcltk org — natural extension of the already-tracked tcltk group (tcllib/tklib/tk)

| Name | URL | Licence | Approx size | Command-spec surface stressed |
|---|---|---|---|---|
| BWidget | https://github.com/tcltk/bwidget | not SPDX-classified (BSD-style) | 2.1 MB *(cloned)* | Megawidget option surface declared via `Widget::declare` (option *type* — TkResource/Int/Enum/String — drives per-option parsing); `Tree::insert index parent node ?-opt val…?` with `index` accepting `"end"` or an integer |
| tDOM | https://github.com/tDOM/tdom | not SPDX-classified (BSD-style) | 5.9 MB *(cloned: 6.4 MB)* | `$node selectNodes ?-namespaces …? ?-cache bool? xpathQuery ?typeVar?` — leading options + an embedded XPath sub-language + an optional trailing out-variable; `nodeValue ?newValue?` getter/setter overload |
| itcl (\[incr Tcl\]) | https://github.com/tcltk/itcl | not SPDX-classified (Tcl-style) | 8.7 MB | The third OO system alongside TclOO/snit already modelled by `definition_body`; public/protected/private wrapper nesting |

### Web / templating / data

| Name | URL | Licence | Approx size | Command-spec surface stressed |
|---|---|---|---|---|
| tclssg | https://github.com/tclssg/tclssg | MIT | 2.1 MB | Static site generator — 90★, active; template-driven page-build ensemble |
| wapp | https://github.com/lego12239/wapp | not SPDX-classified (no LICENSE detected) | 45 KB *(cloned: 68 KB)* | `wapp-page-$name` convention dispatch (command name synthesised from the URL path, not statically enumerable); `wapp-route` unpacks a variadic middle `args` where the *last* word is always the handler body regardless of count |
| retcl | https://github.com/gahr/retcl | BSD-2-Clause | 366 KB *(cloned: 220 KB)* | TclOO `unknown` method proxies **any** Redis command as a method call (fully dynamic arity, one per Redis command); `-sync`/`-cb callback` leading mode words |
| mustache.tcl | https://github.com/ianka/mustache.tcl | not SPDX-classified (see COPYING) | 68 KB *(cloned: 128 KB)* | Context values are polymorphic (plain data vs. "lambda" callable); appended callback arity (0 vs. 1 arg) depends on template-syntax position, not an explicit option |
| tanzer | https://github.com/AngryLawyer/tanzer | MIT | 518 KB | Web framework — route/middleware chaining; low activity (0★) but explicitly requested |
| tclcurl | https://github.com/jdc8/tclcurl | not confirmed (check repo) | 150 KB | libcurl binding — huge `CURLOPT_*`-mirroring `-flag value` option surface on `curl::transfer`/handle config |
| Apache Rivet (tcl-rivet) | https://github.com/apache/tcl-rivet | Apache-2.0 | large (C + Tcl) | Tcl-embedded-in-HTML (`<?tcl … ?>`) script-body DSL; ASF project, 35★, actively released — new find, not in the original candidate list |

### Networking / database bindings

| Name | URL | Licence | Approx size | Command-spec surface stressed |
|---|---|---|---|---|
| Pgtcl | https://github.com/flightaware/Pgtcl | BSD-3-Clause | 3.3 MB | PostgreSQL client — result-set/connection opaque-handle commands, dynamic column-typed result access |
| tclreadline | https://github.com/flightaware/tclreadline | not SPDX-classified | 1.0 MB | GNU readline binding — completion-callback command prefixes, 70★ |
| tclvfs | https://github.com/tcl-mirror/tclvfs (fossil mirror of core.tcl-lang.org/tclvfs) | not SPDX-classified | 1.1 MB *(cloned)* | `vfshandler subcmd root relative actualpath args…` — one callback command-prefix, ~9 wildly different arg shapes selected by `subcmd`, with `fileattributes` itself having a nested 0/1/2-arg get-list/get-value/set mode |

### Large public Tcl projects (servers, CMS, EDA, analysis)

| Name | URL | Licence | Approx size | Command-spec surface stressed |
|---|---|---|---|---|
| NaviServer | https://github.com/naviserver-project/naviserver | Mozilla Public License 1.1 (per project docs; not SPDX-classified on GitHub) | medium (C + Tcl core + Tcl modules) | Multithreaded web/app server; also ships small pure-Tcl modules worth their own corpus entries — `nsstats` (https://github.com/naviserver-project/nsstats) and `websocket` (https://github.com/naviserver-project/websocket) |
| OpenACS core | https://github.com/openacs/openacs-core | GPL-2.0 | 69.9 MB | Large CMS/web-app framework built on NaviServer/AOLserver + XOTcl; 50★. Bonus: `openacs/xotcl-core` and `openacs/xowiki` specifically stress **XOTcl** (mixins, dynamic per-object method injection) — a fourth OO family beyond TclOO/snit/itcl |
| OpenOCD | https://github.com/openocd-org/openocd | GPL-2.0-or-later (per COPYING; not SPDX-classified on GitHub) | 42.6 MB (mostly C) | Its `tcl/` tree is a huge proc-based configuration DSL (`target create`, `jtag newtap`, board/interface/target config scripts) — real-world Tcl-as-config-language corpus, distinct flavour from "library" surfaces; 2,290★ |
| awthemes | https://github.com/bll123/tcl-awthemes | not SPDX-classified | 4.5 MB | Scalable Tk ttk themes (awdark/awlight); archived but real, by BallroomDJ's author |
| nagelfar | https://github.com/euoia/nagelfar (fork; canonical home is sourceforge.net/projects/nagelfar, frozen ~2013) | not confirmed (fork; check repo) | small | Tcl static analyser/syntax checker — directly comparable prior art to our own analyser; 17★ on this fork |

**Total new, verified: 36** (12 apnadkarni + 3 rkeene + 1 critcl + 3 tcltk +
6 web/templating/data + 3 networking/db + 5 large public projects + 3
NaviServer sub-modules counted within the NaviServer row), against a
25+ target.

### Investigated but excluded (no usable git corpus entry)

- **tablelist / mentry / scrollutil / wcb / tsw** (Csaba Nemethi) — real,
  active, pure-Tcl/Tk megawidget packages, but distributed from
  `nemethi.de` (no git hosting), not GitHub/fossil. Older versions ship
  inside `tklib` (`modules/tablelist/`, already tracked as `tcltk/tklib`
  in #1181). Not addable as a distinct git URL.
- **Tcl3D** (Paul Obermeier) — real, BSD-3-Clause, but hosted on
  SourceForge (`sourceforge.net/projects/tcl3d`) with no maintained GitHub
  mirror found. Excluded from the GitHub-URL-based corpus/issue comment;
  worth a manual download if the project wants non-git sources.
- **apnadkarni/ruff** — real and active (22★), but it is the direct
  upstream fork source of `georgtree/ruff`, already in #1181's corpus.
  Re-suggesting it would just be the same codebase under the other name.
- **togl**, **tcllibc** — no actively maintained GitHub repository found
  with real stars/usage (only stale forks, packaging-spec mirrors, or
  RPM-spec-only repos); did not meet the "prefer active repos" bar.

---

## Task B — hook-pattern evidence (16 exemplars)

Each exemplar is a real public-API command, quoted from cloned source or
docs, that needs a **hook** (a native/VM function from words → data) rather
than a purely declarative `CommandSpec`. Categorised by the pattern
taxonomy from the task brief.

### 1. Dynamic arity — argument count/shape depends on earlier words

**`argparse` (georgtree/argparse, forked from Andy Goth's original) — the
`argparse` command itself.**
Source: `argparse.tcl:55-80`. The number of *trailing* words after any
global switches determines completely different behaviour:
```tcl
switch [expr {[llength $args]-$i}] {
0 { break }
1 { set definition [lindex $args end]; set argv [uplevel 1 {::set args}] }
2 { set definition [lindex $args end-1]; set argv [lindex $args end] }
}
```
With one trailing word it's the *definition* and the arguments are pulled
via `uplevel 1` from the caller's own `args` local; with two, the second
is the explicit argument list instead. Declarative data cannot express
"reach into the caller's frame to find the actual arguments," and the
arity-selecting logic is itself conditional on a runtime count.

**`wapp-route` (lego12239/wapp, `wapp-routes.tcl:19-31`).**
```tcl
proc wapp-route { method pathspec args } {
    set args [lreverse [lassign [lreverse $args] body]]
    ...
    proc wapp-page-$page-$method {} "\n$prefix $body"
}
```
`args` is a variadic run of local-variable names of *any* length, but the
**last** word is always the handler body regardless of how many names
precede it — "peel from the end" arity, plus the command synthesises a new
`proc` (`wapp-page-$page-$method`) from a string template at call time.

### 2. Options whose presence changes other options' meaning/arity/format

**`argparse` element-switch DSL (georgtree/argparse, `docs/argparse.n`
"ELEMENT SWITCHES"/"VALUE" sections).**
A real call:
```tcl
argparse {
    {-b!= -validate {[string is double $arg]}}
    {-elements= -catchall}
}
```
The **shorthand suffix characters** on the element name (`=`,`?`,`!`,`*`,
`^`) each imply different element switches, and those switches have
documented, cross-cutting exclusion/implication rules: "`-optional`,
`-required`, `-catchall`, and `-upvar` imply `-argument` when used with
`-switch`", "`-value` may not be used with `-argument`, `-optional`,
`-required`, or `-catchall`", "When `-switch` and `-optional` are both
used, `-catchall`, `-default`, and `-upvar` are disallowed." No flat
key/value spec captures a combinatorial legality table like this — it is
a constraint-checking function over the whole element definition.

**`tls::socket` (rkeene/TclTLS, `tls.tcl:19-52,195-245`).**
```tcl
variable socketOptionRules {
    {0 -async sopts 0}   {* -myaddr sopts 1}   {* -cafile iopts 1}
    {* -command iopts 1} {* -request iopts 1}  {* -require iopts 1}
    ...
}
```
Real calls: `tls::socket -cafile ca.pem host 443` vs.
`tls::socket -server accept_cmd -cafile ca.pem 8443`. Presence of
`-server` changes (a) the legal option set (`socketOptionsServer` vs.
`socketOptionsNoServer`), (b) how many trailing positionals are required
(`host port` vs. just `port`), and (c) it *rewrites* the caller-supplied
callback by wrapping it: `[list tls::_accept $iopts $callback]`. Each
option is also routed at runtime to one of two different downstream
commands' option sets (`sopts` for core `socket`, `iopts` for
`tls::import`) via the table above — a lookup + dispatch, not data.

**`cffi::Wrapper function`/`testDll function` varargs (apnadkarni/tcl-cffi,
`tests/varargs.test:14-40`).**
```tcl
testDll function formatVarargs int {buf {chars[n] out} n int fmt string ...}
formatVarargs buf 100 %d [list int 42]
```
The trailing `...` marker in the *parameter definition* changes the
calling convention of every call site: arguments after the fixed
parameters must come as `{type value}` pairs, and only a restricted set
of types is legal there ("Type not permitted for varargs"). Additionally
`chars[n]` cross-references another parameter's *name* in the same
definition list to size a buffer — a definition-list-internal reference,
not resolvable from one element in isolation.

### 3. Mode words selecting entirely different tails

**`tarray::column search` (apnadkarni/tarray,
`tests/column_search.test:20-50`).**
```tcl
tarray::column search -eq -all [newcolumn $type] [samplevalue $type]
tarray::column search -eq -inline -bitmap $col $val
```
Comparator mode words (`-eq`/`-gt`/`-lt`/…) combine with independent
shape flags (`-all`, `-inline`, `-bitmap`) to select one of several
mutually-exclusive *return* representations (single index, index column,
value column, or boolean bitmap column) — a combinatorial dispatch over
option presence, resolved once per call.

**`tclvfs` filesystem handler (`doc/vfs-fsapi.man:63-90,150-192`).**
```
[call vfshandler access root relative actualpath permissions]
[call vfshandler fileattributes root relative actualpath ?index? ?value?]
[call vfshandler matchindirectory root relative actualpath pattern types]
[call vfshandler open root relative actualpath mode permissions]
```
One callback command-prefix implements ~9 unrelated operations
(`access`, `createdirectory`, `deletefile`, `fileattributes`,
`matchindirectory`, `open`, `removedirectory`, `stat`, `utime`) selected
by its first argument, each with a completely different tail shape —
and `fileattributes` alone is *also* a nested 0/1/2-arg
list-names/get-value/set-value mode switch.

### 4. Interleaved name/value or var/list tails beyond a fixed stride

**`cffi` varargs pairs** (same call as above): `{type value} {type value}
…` — an unbounded run of 2-word groups after the fixed prototype, where
the first word of each pair is itself constrained to a type-name
sub-vocabulary that excludes several base types ("Type not permitted for
varargs").

**`retcl`'s proxied Redis commands (gahr/retcl, `unknown` method,
`retcl.tm:519-566`, called via e.g. `$r MSET k1 v1 k2 v2`).** Because
`unknown` proxies arbitrary Redis commands, retcl itself has *no* static
notion of which commands take interleaved key/value pairs (`MSET`,
`HSET`), score/member pairs (`ZADD`), or flat lists — that knowledge
lives entirely in the Redis protocol, not in retcl's Tcl source, and
would need to be supplied by a *separate* Redis-command hook table keyed
by the dynamically-dispatched command name.

### 5. Callbacks whose appended arity depends on options/position

**`Promise::then`/`done` (apnadkarni/tcl-promise, `src/promise.tcl:400,
437,525-560`).**
```tcl
method then {on_fulfill {on_reject {}}} { ... }
```
Documented: "Reactions are called with an additional argument which is
the value" for fulfilment, but the reject path calls with **two**
extra arguments (`reason edict` — see `_then_reaction`'s
`$target_promise reject $value $edict`). The same call's two callback
*parameters* get different, fixed-but-unequal appended arities — a hook
must know which slot it is filling, not just "append N args."

**mustache.tcl lambda sections (ianka/mustache.tcl, `mustache.tcl:160-267`).**
A context dict value can be plain data *or* a lambda (an `apply`-style
command). When it's used at a plain `{{tag}}` interpolation it is invoked
with **no** arguments and its result is re-parsed as a template; when the
same kind of value is used at a `{{#section}}…{{/section}}`, it is invoked
with **one** argument (the section's raw, unexpanded template text —
`[::lambda arg $body $sectioninput]`). Appended arity depends on where in
the template syntax the callable was found, which is not visible at the
call site that registers the context data at all.

### 6. Anything else code-needing

**Dynamically-generated `argparse` definitions (georgtree/SpiceGenTcl,
`src/generalClasses.tcl:126-148`, used e.g. by
`src/ngspice/specMiscClassesNgspice.tcl:29-45`).**
```tcl
method BuildArgStr {paramsNames} {
    foreach paramName $paramsNames { lappend paramDefList -${paramName}= }
    return [join $paramDefList \n]
}
...
set arguments [argparse -inline -help {...} "
    [my BuildArgStr $keyValParams]
    [my BuildSwArgStr $swParams]
"]
```
A real, shipping TclOO constructor (`OptionsNgspice new -klu -abstol
1e-10 -maxord 6`) whose entire legal-option set is *computed at runtime*
from two Tcl list variables via helper methods, then handed to the
already-hook-needing `argparse`. No static spec can enumerate this
class's constructor options without evaluating `BuildArgStr`/
`BuildSwArgStr` against the current `keyValParams`/`swParams` — a
textbook case of composed hook-need (an already-dynamic DSL, driven by
data that is itself computed).

**`tls::socket`'s callback rewriting** (category 2 above) also belongs
here in spirit: a hook doesn't just validate options, it *produces a new
command prefix* (`tls::_accept $iopts $callback`) that the runtime then
invokes instead of the literal one the user wrote.

**`$node selectNodes` embedded XPath (tDOM/tdom, `doc/domNode.n:410-472`).**
```
selectNodes ?-namespaces prefixUriList? ?-cache <boolean>? xpathQuery ?typeVar?
```
Real call: `$node selectNodes {chapter[3]//para[@type='warning']}`. The
mandatory `xpathQuery` argument is itself a full embedded expression
language (predicates, axes, functions) that must be understood to know
what the call does or returns, and the optional trailing `typeVar` is a
variable-**write** parameter whose presence changes the return contract
(single value vs. value-plus-type-out-var). Also `nodeValue ?newValue?`
is a classic 0-arg-getter/1-arg-setter overload on the same command name.

---

## Sources / method notes

- GitHub repo metadata (existence, stars, licence SPDX id, size, language)
  via the GitHub search API, cross-checked 2026-08-12.
- Non-GitHub facts (Nemethi's site, Tcl3D/SourceForge, Apache Rivet) via
  WebSearch/WebFetch, cross-checked 2026-08-12.
- Task B source citations are line numbers in the exact shallow clones
  taken for this survey (`git clone --depth=1`) — re-cloning may shift
  line numbers on tip-of-branch changes; the quoted snippets and command
  shapes are the load-bearing part, not the line numbers.
- Existing #1181 corpus (georgtree ×8, nico-robert ×6, tcltk ×3,
  aplsimple ×2, Xilinx/OSVVM ×2, plus the 14-repo iRules corpus pinned in
  `scripts/perf/MANIFEST.toml`) was treated as off-limits for Task A
  re-suggestion and cross-checked against both the issue body and the
  manifest before finalising the new-library list.
