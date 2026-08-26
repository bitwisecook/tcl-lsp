# BIG-IP evidence review: iRules, tmsh, iApps, and APL ([#1631](https://github.com/bitwisecook/tcl-lsp/issues/1631))

> **Review target:**
> [`dialect-and-package-registry-redesign.md`](dialect-and-package-registry-redesign.md)
> revision 2 at `150338a2883857f5d4cbdd8cd924deed32c77bc8`, supplemented by the
> [general adversarial review](dialect-and-package-registry-redesign-adversarial-review.md).
> Revision 2 accepts that review's thirteen blocking findings; this companion
> reviews the residual F5 assumptions that survived those corrections.
>
> **Verdict:** request changes to every fixed F5 Tcl-version claim. The proposed
> separation of dialect, package, and environment is useful, but its F5 rows
> presently turn coarse documentation and undocumented implementation
> assumptions into permanent type facts. They also treat six distinct language
> contexts as though one observed `tclsh` could identify all of them.

## Executive finding

The F5 model needs another key before it can be ratified:

```text
BigIpExecutionContext
  ├── TmmIRule
  ├── TmshCliScript
  ├── IAppImplementation
  ├── IAppPresentationApl
  ├── IAppPresentationTclCallback
  └── HostShellTcl
```

That key is not cosmetic. These contexts have different parsers, commands,
variables, package paths, security policies, lifetimes, and execution engines.
Revision 2 still classifies `f5-irules` as “8.4-based”, fixes both `f5-iapps`
and `f5-tmsh` at Tcl 8.5, and asserts that its lexer-wide ghost separator is
why `if {1}{...}` works on BIG-IP. Its illustrative `CoreProfileId` leaves the
iRules release as “TMOS-keyed or single”, but neither choice is made or backed
by an appliance transcript, binary provenance, F5 source, or release-indexed
conformance data.

The strongest counterexample is the iRules brace chain. Stock Tcl 8.6.18 and
9.0.4 reject `if {1}{...}` with `extra characters after close-brace`, while F5
publishes iRules examples containing the form. The repository models this by
inserting a zero-width separator after **every** braced word followed by `{`.
Its tests deliberately assert both the generic `a {b}{c}` split and `if`
highlighting, but neither test cites an appliance result. The implementation
therefore asserts much more than the external evidence: it predicts that
`list {a}{b}` is a two-argument command. Combined with iRules' disabled
expansion flag, it further predicts that `{*}{a b}` is two ordinary braced
words instead of either TIP 157 expansion or a syntax error. The live matrix
below tests these generic and cross-axis predictions instead of treating an
implementation-expectation test as a runtime oracle.

The correct registry boundary depends on the result:

- if `}{` separates words for every iRules command, it is a lexer-grammar fact;
- if it is accepted only by `if`, `when`, or another command family, it belongs
  in those commands' declarative argument grammar; and
- if acceptance varies by BIG-IP release, the fact must also carry a BIG-IP
  lifecycle.

In no case should a single `bool` inferred from one `if` example silently alter
every command.

## Evidence discipline

This review keeps five kinds of evidence separate:

| Evidence | What it can establish | What it cannot establish |
|---|---|---|
| Upstream Tcl C and tests | Standard Tcl parser and command behaviour at a pinned revision | F5's private TMM patches or preprocessors |
| F5 manuals | Supported interfaces, execution boundaries, and documented policy | Exact behaviour on a later release unless the page covers it |
| F5 API-reference examples | Useful candidate syntax and commands | A warranty of correctness; those pages carry an explicit community-content disclaimer |
| A live BIG-IP build | Behaviour of one execution context on one exact build | “Forever”, all platforms, or a different execution context |
| Host `tclsh` | The appliance operating system's shell interpreter | The Tcl embedded in TMM, tmsh `scriptd`, or iApp execution |

The intended live target is a lab BIG-IP VE appliance whose control-plane
inventory identifies it as 21.1.0.1. That inventory label is not runtime
evidence: the build and every Tcl identity must still be recorded inside the
relevant execution context. Private management addresses, host names,
credentials, and key material are intentionally omitted. The probe is designed
so that no iRule is attached to a virtual server, no data-plane traffic is
generated, and no configuration is saved. Disposable objects use a unique
`/Common/__tcl_lsp_probe_*` prefix, are checked for collision before creation,
and are deleted by an exit trap. The final transcript must include absence
checks before it can be accepted as evidence.

One current build is still valuable: it can falsify a universal claim and
define one measured catalogue row. It cannot justify “pinned forever”.

## Findings

### F1. The proposal conflates six language contexts

F5's own manuals describe different entry points:

- [`cli script`](https://clouddocs.f5.com/cli/tmsh-reference/v14/modules/cli/cli_script.html)
  says tmsh invokes `script::run`, exposes `tmsh::*` commands and variables,
  applies role-dependent command disabling, and can load supported packages
  from `/usr/share/compat-tcl8.4`.
- [`sys application template`](https://clouddocs.f5.com/cli/tmsh-reference/latest/modules/sys/sys_application_template.html)
  stores separate `presentation` and `implementation` strings.
- [iApp implementation details](https://clouddocs.f5.com/api/iapps/implementation.html)
  identify the implementation as Tcl, provide `$tmsh::app_name` and form-value
  variables, and send `puts` output to `/var/tmp/scriptd.out`.
- [APL shared code](https://clouddocs.f5.com/api/iapps/APL-Shared-Code.html)
  demonstrates `define`, `section`, `choice`, `optional`, and embedded `tcl`
  clauses. APL is a presentation DSL containing Tcl callbacks; it is not a Tcl
  dialect.
- [`RULE_INIT`](https://clouddocs.f5.com/api/irules/RULE_INIT.html) executes in
  TMM when an iRule is saved, at boot, or when software restarts, without a
  virtual-server connection.

These are not aliases for one environment. A file router may associate an iApp
template's presentation field with APL and its implementation field with iApp
Tcl, but the registry must preserve the boundary after routing. In particular,
APL keywords must never become Tcl commands, and TMM-only iRule commands must
never leak into an iApp implementation.

Revision 2's statement that APL container routing is merely a language-ID fact
is too weak. F5's tmsh manual shows a `choice ... tcl { ... }` script nested
inside the APL presentation string and calls it embedded Tcl. One document
therefore has at least three nested ranges: the template container, APL, and a
Tcl callback inside APL, in addition to the separate implementation Tcl field.
A language ID classifies a whole document; it cannot assign the callback's
grammar, surface, variables, policy, or execution evidence. Nor may the callback
be assumed equivalent to implementation Tcl merely because both use Tcl
syntax.

**Required change.** Add the execution-context key above to every F5 grammar,
command, variable, package, policy, and evidence record. Add a typed embedded-
language range descriptor for the APL `tcl` clause; do not solve it with a
whole-document language ID. Remove `f5-bigip` as a catch-all Tcl identity:
BIG-IP configuration, APL, APL's Tcl callbacks, iRules, tmsh scripts, iApp
implementation Tcl, and host Tcl are independently routed surfaces.

### F2. The Tcl release defaults have no adequate provenance

The shipping model states all of the following as total facts:

- iRules embeds “a genuine Tcl 8.4.6” forever;
- iApps runs “a real Tcl 8.5.13 host”; and
- tmsh uses Tcl 8.5.

Those statements appear in
[`dialect-profile-model.md`](dialect-profile-model.md) and, for iApps, in the
shipping comment and tests in
[`special_vars.rs`](../../rust/tcl-registry/src/special_vars.rs). None cites an
F5 build manifest, source package, binary dependency, appliance transcript, or
version matrix. Revision 2 correctly distinguishes intended from shipping
architecture, but carries the same unmeasured defaults into its classification
table: iRules is “8.4-based”, and the iApps and tmsh environments select
`tcl@8.5`. Writing “TMOS-keyed or single” beside the iRules release field
records the unresolved choice; it does not supply the missing key or evidence.

There is official support for the coarsest iRules claim: F5's
[GTM certification study guide](https://techdocs.f5.com/content/dam/f5/kb/global/solutions/K29900360.html/Certification_Study_Guide_302.pdf)
calls iRules “based on Tcl 8.4”. That wording does not identify 8.4.6, promise
no backports, specify the parser that accepts `}{`, or establish one invariant
surface across BIG-IP releases. The design should record it as documentary
evidence for a baseline hypothesis, not promote it to all of those stronger
facts.

The tmsh manual makes the 8.5 default particularly suspect. Its supported
third-party packages live under a directory named `compat-tcl8.4`, and its
example deliberately prints `[info patch]`. The directory name is not proof of
the interpreter version—it might be an ABI-compatibility path—but it is direct
evidence that the answer must be measured instead of guessed.

Even `[info patchlevel]` is not a complete semantic profile. F5 can patch or
backport individual parser and command changes without changing that string.
The release value is one observation alongside syntax, command-presence,
expression, package, and policy probes.

**Required change.** Replace the bare release fields with versioned,
provenance-bearing measurements:

```rust,ignore
struct EmbeddedRuntimeEvidence {
    bigip_release: BigIpRelease,
    build: Arc<str>,
    context: BigIpExecutionContext,
    reported_patchlevel: Option<Arc<str>>,
    grammar_probe_set: ProbeSetId,
    command_probe_set: ProbeSetId,
    source: EvidenceSource,
}
```

`reported_patchlevel` may select a baseline, but probe results override the
baseline. An unmeasured BIG-IP release resolves to `Unknown` or to an explicitly
labelled nearest-known assistance profile; it must not silently inherit
“forever”.

### F3. The current `}{` implementation overfits one command and overclaims all commands

Standard Tcl's parser requires whitespace after a braced word. At the pinned Tcl
9.0.4 revision, [`tclParse.c` lines 278–298](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclParse.c#L278-L298)
raises `TCL_PARSE_BRACE_EXTRA` when no whitespace was scanned, and
[`parse.test` lines 150–174](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/tests/parse.test#L150-L174)
locks in the error.

F5-hosted examples nevertheless contain forms such as:

```tcl
if {[HTTP::path] eq "/"}{
    log local0. "redirecting"
}
```

The example appears in [iRules Common Concepts](https://clouddocs.f5.com/api/irules/iRulesCommonConcepts.html),
and other API-reference pages contain still tighter forms such as
`if {[HTTP::method] ne "GET"}{return}`. Because F5 marks the API-reference
content as community-contributed, these examples are a reason to probe, not a
substitute for probing.

The repository's implementation in
[`Lexer::parse_brace`](../../rust/tcl-lexer/src/lexer.rs) checks only two facts:
the iRules flag is on, and the next byte is `{`. It then injects a pending
separator. It does not know the command, argument position, or surrounding
grammar. [`lexer_depth.rs`](../../rust/tcl-lexer/tests/lexer_depth.rs) locks in
the generic `a {b}{c}` split, while the focused test in
[`highlight.rs`](../../rust/tcl-lexer/src/highlight.rs) covers
`if {1}{ pool p }`. Those tests accurately describe the local implementation;
they do not establish that TMM accepts the generic case, and neither covers the
expansion/separator interaction.

The discriminating matrix is:

| Case | Stock Tcl 8.6.18 | Stock Tcl 9.0.4 | What it distinguishes on TMM |
|---|---:|---:|---|
| `if {1} {expr {6*7}}` | returns `42` | returns `42` | control |
| `if {1}{expr {6*7}}` | parse error | parse error | documented iRules oddity |
| `list {a}{b}` | parse error | parse error | generic separator vs `if` grammar |
| `set x {a}{b}` | parse error | parse error | parser acceptance plus ordinary arity checking |
| `if{1}{expr {6*7}}` | invalid command | invalid command | proves no separator before the first `{` |
| `list {*}{a b}` | expands to `a b` | expands to `a b` | interaction between expansion and the ghost separator |

**Required change.** Retain a dialect-level separator only if the live generic
cases establish it. Otherwise move the rule into declarative argument grammar
for the commands and positions that accept it. Add lexer/parser tests for every
row and for the expansion/separator interaction, not just highlighter recovery
for `if`.

### F4. tmsh policy is not core Tcl availability

The tmsh manual's “Disabled Commands” section makes command visibility depend
on the executing user's role. It lists file, process, socket, interpreter,
package, and event-loop commands that are disabled for users without
Administrator or Resource Administrator privileges. The same binary and
reported Tcl patch level can therefore expose different command tables.

This is the same catalogue-versus-live-binding distinction identified in the
general adversarial review, now inside one F5 execution context. A registry row
such as “tmsh has `package`” is false without a policy qualifier. Conversely,
“tmsh lacks `package`” is false for an administrator script.

**Required change.** Model the tmsh role/sandbox policy separately from the Tcl
baseline and the ambient `tmsh::*` package. Completion may show policy-disabled
commands with an explanation; compiler and analyser hooks must not treat them
as callable.

### F5. `tcl_platform` has iRules-specific operational semantics

F5's [`tcl_platform` iRules reference](https://clouddocs.f5.com/api/irules/tcl_platform.html)
adds `osVersion` and `tmmVersion`, but also warns that reading the ordinary
global can make a virtual server incompatible with clustered multiprocessing.
The `static::tcl_platform` copy is the CMP-compatible route.

That is not merely a predefined variable description. It is an execution-cost
and placement effect attached to a particular variable binding in a particular
context. Treating every `tcl_platform` as the stock Tcl variable loses a
performance-significant iRules diagnostic; attaching that effect to stock Tcl
would be equally wrong.

**Required change.** Allow environment/context overlays to refine the effects
of a core variable binding. The iRules overlay should distinguish the ordinary
global from the `static::` alias, carry the F5 source, and be lifecycle-keyed if
the behaviour changes by BIG-IP release.

### F6. BIG-IP release and tmsh syntax release are separate axes

The tmsh manual says that, starting in BIG-IP 11.5.0, tmsh commands are
versioned and a CLI script may select an active tmsh syntax version. A script
running on one BIG-IP build can therefore request an older tmsh command grammar.
That axis is independent of the embedded Tcl parser and of the installed
BIG-IP build.

It is also not necessarily document-constant. F5's own multi-version example
calls `tmsh::modify cli version active 11.5.0`, executes a command, changes the
active version to 11.6.0, and executes another command in the same
`script::run`. Whatever the runtime scope of that setting proves to be, the
language model must distinguish statements before and after the transition.
An environment-level version selected once for the file cannot do so.

The proposal's single `Keyed(BigipVersion)` package pin cannot express all
three facts:

```text
BIG-IP software/build
  × embedded Tcl runtime evidence
  × selected tmsh command-syntax version
```

**Required change.** Give tmsh syntax its own typed version axis and a
registry-declared state transition for `tmsh::modify cli version active`.
A constant argument updates subsequent tmsh-command resolution; a dynamic or
unsupported argument widens it to `Unknown`. Probe whether the state is local
to the script, tmsh process, user session, or system before assigning a realm
scope. Do not compare or intersect it with Tcl package versions or Tcl core
releases merely because all three happen to use dotted numbers.

### F7. iApp target and execution policy are action-local data

The tmsh application-template schema carries facts that the proposed single
`f5-iapps` environment does not preserve:

- `requires-bigip-version-min` and `requires-bigip-version-max` declare a
  template's own BIG-IP compatibility interval;
- `role-acl` controls which roles may run each template action; and
- `run-as` selects the account used for the implementation, with an omitted
  account meaning the calling user.

Consequently, two implementation scripts on the same appliance and reported
Tcl release may have different target ranges and principals. Treating the iApp
surface as one fixed environment either loses those restrictions or wrongly
applies one template's authority to another. The version bounds are also
positive source evidence already present in the artefact being analysed; a
workspace-wide BIG-IP default must not override a narrower template range.

The principal is not the last policy axis. F5's
[Bug ID 589374](https://cdn.f5.com/product/bugtracker/ID589374.html) documents
iApp Tcl in which shell-like file operations fail when the appliance setting
`systemauth.disablebash` is enabled. That is a same-product capability change
driven by live system policy, not by Tcl release or package version.

**Required change.** Parse the template and action metadata into typed document
overlays. Intersect the declared BIG-IP interval with the configured target set,
attach `role-acl`/`run-as` to the implementation realm and its effects, and
keep relevant appliance security settings in a distinct live-policy overlay.
Unknown or dynamic principals/policy must widen authorisation-sensitive
analysis; they must not silently inherit an administrator command surface.

### F8. A one-build success must become a fixture, not tribal knowledge

The repository currently has a hand-written boolean, a structural test that
asserts the generic split, and a highlighter test for `if`. The experiment below
is more useful only if its input, expected output, execution context, BIG-IP
build, and provenance become durable data.

**Required change.** Add a checked-in F5 conformance corpus with:

- one manifest per measured BIG-IP build and execution context;
- exact scripts and normalised, privacy-safe outputs;
- positive and negative grammar cases;
- command, variable, expression, package, and policy probes;
- explicit `unknown` cells rather than inferred answers; and
- a refresh tool that never runs in ordinary CI, but validates a newly supplied
  transcript against the schema.

Generated registry rows may consume that corpus. Hand-authored “forever” prose
must not outrank it.

## Experiments

### E1. Stock Tcl controls

The same script was run with Homebrew Tcl 8.6.18 and Tcl 9.0.4 on macOS. It
uses `catch` so a syntax failure is observed without stopping the remaining
cases.

```tcl
proc run_case {name script} {
    set ::marker unset
    set rc [catch {uplevel #0 $script} value options]
    set errorcode {}
    if {$rc && [dict exists $options -errorcode]} {
        set errorcode [dict get $options -errorcode]
    }
    puts [list $name rc=$rc value=$value errorcode=$errorcode marker=$::marker]
}

run_case if_control {if {1} {set ::marker yes}}
run_case if_brace_chain {if {1}{set ::marker yes}}
run_case list_brace_chain {list {a}{b}}
run_case set_brace_chain {set x {a}{b}}
run_case command_glued {if{1}{set ::marker yes}}
run_case expansion_chain {list {*}{a b}}
```

Observed output, normalised to remove Tcl's list-escaping noise; the case results
were identical on both interpreters:

```text
patchlevel=8.6.18 tcl_version=8.6
if_control rc=0 value=yes errorcode= marker=yes
if_brace_chain rc=1 value={extra characters after close-brace} errorcode=NONE marker=unset
list_brace_chain rc=1 value={extra characters after close-brace} errorcode=NONE marker=unset
set_brace_chain rc=1 value={extra characters after close-brace} errorcode=NONE marker=unset
command_glued rc=1 value={invalid command name "if{1}{set"} errorcode=[TCL,LOOKUP,COMMAND,"if{1}{set"] marker=unset
expansion_chain rc=0 value={a b} errorcode= marker=unset

patchlevel=9.0.4 tcl_version=9.0
if_control rc=0 value=yes errorcode= marker=yes
if_brace_chain rc=1 value={extra characters after close-brace} errorcode=NONE marker=unset
list_brace_chain rc=1 value={extra characters after close-brace} errorcode=NONE marker=unset
set_brace_chain rc=1 value={extra characters after close-brace} errorcode=NONE marker=unset
command_glued rc=1 value={invalid command name "if{1}{set"} errorcode=[TCL,LOOKUP,COMMAND,"if{1}{set"] marker=unset
expansion_chain rc=0 value={a b} errorcode= marker=unset
```

This agrees with the upstream C parser and gives the live TMM probe an
independent control. It does not say whether TMM implements the divergence in a
patched Tcl parser, an iRules pre-parser, or another layer.

### E2. Repository-model controls

The two focused tests were run at the reviewed tree:

```text
cargo test -p tcl-lexer irules_brace_separator_splits_adjacent_braced_words -- --exact
cargo test -p tcl-lexer --features html f5irules_config_highlights_body_after_brace_chain
```

Both pass. The first asserts that iRules tokenises `a {b}{c}` with an empty
separator at the `}{` seam; the second asserts that `pool` inside
`if {1}{ pool p }` receives command highlighting. This establishes the exact
local prediction and prevents this review from arguing against a weaker model
than the repository implements. It is not evidence that a BIG-IP runtime
accepts either input.

### E3. Live BIG-IP transcript

<!-- BIGIP_LIVE_RESULTS_START -->
**Evidence status (2026-08-26): pending.** The appliance was reachable, but the
authenticated execution session was not approved while this review checkpoint
was prepared. No version, command-surface, or syntax conclusion is drawn from
reachability or from the control-plane inventory label. Replace this paragraph
with the normalised transcript and cleanup proof after the probe in E4 has run.
<!-- BIGIP_LIVE_RESULTS_END -->

### E4. Probe and cleanup contract

The appliance run follows this order:

1. Record `show sys version`, installed Tcl packages/binaries, and relevant
   process/library provenance without changing configuration.
2. Run host `tclsh` introspection and label it **host only**.
3. Create a uniquely named `cli script`, run `script::run`, delete it, and prove
   it is absent.
4. Create an unattached iRule whose `RULE_INIT` records patch, command, variable,
   expression, and brace-chain results; delete it, and prove it is absent.
5. Create the smallest valid iApp template and service with implementation-only
   output, capture only marker-tagged lines from `/var/tmp/scriptd.out`, delete
   the service and template, and prove both are absent.
6. If a non-interactive presentation renderer is available, exercise a
   separately marked APL `tcl` callback. Otherwise record that context as
   `Unknown`; never copy the implementation result into it.
7. Confirm no probe object is attached to a virtual server and do not run
   `save sys config`.

Every create is preceded by an exact-name absence check. A shell `EXIT` trap
deletes only the exact probe names. A collision aborts rather than modifying an
existing object. This is deliberately more stringent than `tmsh::stateless`,
which can hide a name collision.

The runtime probe records both reported identity and behaviour:

```text
reported: info patchlevel, tcl_version, tcl_patchLevel
grammar:  }{ matrix, {*} expansion, numeral and expr operators
surface:  dict, lassign, lmap, try, throw, yieldto, package, interp
F5:       tmsh::version, selected tmsh syntax, tcl_platform(osVersion/tmmVersion)
policy:   current role and command visibility, recorded separately
system:   relevant policy settings such as systemauth.disablebash
```

No conclusion about a missing command is made from one `info commands` result
unless the execution role and context are also recorded.

## Required changes to #1631

The proposal now marks implementation as building. Before any F5 row is
migrated into the new source of truth:

1. Add `BigIpExecutionContext`, route APL separately from Tcl, and describe the
   embedded Tcl ranges inside APL.
2. Remove “pinned forever” and the unreferenced 8.4.6/8.5.13 assertions.
3. Introduce provenance-bearing, BIG-IP-build-indexed runtime evidence with
   `Unknown` for unmeasured releases.
4. Give tmsh command syntax its own version axis and temporal transition.
5. Make tmsh policy/role a separate command-visibility overlay.
6. Import iApp BIG-IP bounds, `role-acl`, and `run-as` as action-local overlays;
   keep appliance security settings as separate live policy.
7. Resolve the `}{` scope from the generic live matrix; encode it as lexer data
   only if it is genuinely command-independent.
8. Attach iRules-specific effects to `tcl_platform` and its concrete binding.
9. Generate measured F5 defaults from a conformance corpus, and fail the drift
   gate if prose, registry rows, or tests disagree with that corpus.

## Acceptance matrix

The redesigned model is not ready until the same probe schema has at least the
following coverage:

| Context | Current lab build | One supported 17.x build | One older build | Required discriminator |
|---|---:|---:|---:|---|
| TMM iRules | required | required | required | generic vs command-specific `}{` |
| tmsh CLI script, Administrator | required | required | required | reported patch plus role-visible commands |
| tmsh CLI script, restricted role | required | desirable | desirable | policy overlay |
| iApp implementation, explicit/calling principal | required | required | desirable | direct runtime identity plus action policy |
| iApp presentation APL | routing fixture | routing fixture | routing fixture | APL tokens never enter Tcl registry |
| iApp presentation Tcl callback | required or `Unknown` | required or `Unknown` | desirable | nested range and callback-specific surface |
| host `tclsh` | provenance only | provenance only | provenance only | never used as embedded-runtime proof |

For every measured row, the registry must predict the observed grammar and
surface without a command-name branch in a consumer. For every unmeasured row,
strong semantic hooks must abstain.

## Source index

- Tcl 9.0.4 parser: [`generic/tclParse.c`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/generic/tclParse.c#L250-L299)
  and [`tests/parse.test`](https://github.com/tcltk/tcl/blob/c655b4770b1d6d32a8cbffd6cef59db6029fe19e/tests/parse.test#L140-L175).
- Tcl language syntax: [Tcl manual](https://www.tcl-lang.org/man/tcl8.0/TclCmd/Tcl.htm),
  especially the rule that whitespace separates command words.
- F5 tmsh scripting: [`cli script`](https://clouddocs.f5.com/cli/tmsh-reference/v14/modules/cli/cli_script.html).
- F5 iApp structure: [`sys application template`](https://clouddocs.f5.com/cli/tmsh-reference/latest/modules/sys/sys_application_template.html),
  [implementation details](https://clouddocs.f5.com/api/iapps/implementation.html),
  and [APL shared code](https://clouddocs.f5.com/api/iapps/APL-Shared-Code.html).
- F5 iRules execution and variables: [`RULE_INIT`](https://clouddocs.f5.com/api/irules/RULE_INIT.html)
  and [`tcl_platform`](https://clouddocs.f5.com/api/irules/tcl_platform.html).
- F5's coarse iRules baseline: [GTM certification study guide](https://techdocs.f5.com/content/dam/f5/kb/global/solutions/K29900360.html/Certification_Study_Guide_302.pdf).
- F5 iApp policy variation: [Bug ID 589374](https://cdn.f5.com/product/bugtracker/ID589374.html).
- F5 brace-chain examples: [iRules Common Concepts](https://clouddocs.f5.com/api/irules/iRulesCommonConcepts.html)
  and [`HTTP::request`](https://clouddocs.f5.com/api/irules/HTTP__request.html).
- Repository implementation under review:
  [`Lexer::parse_brace`](../../rust/tcl-lexer/src/lexer.rs),
  [the one focused highlighter test](../../rust/tcl-lexer/src/highlight.rs),
  [the proposed grammar table](dialect-and-package-registry-redesign.md), and
  [the fixed profile table](dialect-profile-model.md).
