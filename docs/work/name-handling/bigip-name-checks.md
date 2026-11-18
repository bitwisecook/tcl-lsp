# BIG-IP name-handling checks

A work order for someone with shell and tmsh access to a real BIG-IP who can
load iRules and run tmsh, iApp and iCall Tcl. Each check is a small Tcl snippet
with the answer Tcl 8.4 gives. Run it, send back what the BIG-IP printed, and
say whether it matches. Nothing here needs knowledge of this repository's
history; file paths are given so that the result can be filed against the code
that depends on it.

## Start here

This document is a deliverable of its own: a work order for an agent with shell and `tmsh` access to a lab BIG-IP who has never seen this repository. Every check below is self-contained in the sense that it names the build and modules it needs, the objects to create, the rule or script text, the traffic to send, what to capture and the answer that confirms or refutes each test that waits on it. Read the two tables, pick a check, follow the section it points to.

**What you need.** A lab BIG-IP (the measurements on record are BIG-IP 21.1.0.1 build 0.0.26; record the build with `tmsh show sys version` and the platform with `tmsh show sys hardware`) with LTM and iRules provisioned (`tmsh list sys provision ltm`), shell and `tmsh` access, a free lab address and port 19912, and four TMMs where a check says "per TMM" (`tmsh show sys tmm-info`). The tmsh, iApp and iCall checks need no extra module. Never run `tmsh save sys config`. Delete everything you create.

**How to run one.** Checks `BIGIP-NAME-001` to `073` use the `BNC|` harness of section 2 (§2.1 gives the exact load, collect and clean-up commands, the `RULE_INIT` and `HTTP_REQUEST` wrappers and the virtual-server definition). The checks numbered 18 to 31 in section 6 are ordinary rules, not harness snippets: save the text as `ltm rule /Common/bnc_rN { ... }` in a file and load it exactly as §2.1 does (`tmsh load sys config merge file`); the answer to *does the loader accept it* is the loader's message, the answer to *what does it do* is `/var/log/ltm` (`log local0.`) and the traffic you send. Report in the form of section 5.

**Optional objects some checks need** (create, use, delete):

```sh
tmsh create ltm pool /Common/bnc_pool members add { <lab member>:80 } monitor none
tmsh create ltm data-group internal /Common/bnc_dg type string records add { abc { data one } /x { data two } }
tmsh create auth partition TenantA ; tmsh create auth partition TenantB      # TS-08 only
# virtual server with TCP and HTTP profiles: the stanza in section 2.1
# cleanup: tmsh modify ltm virtual /Common/bnc_vs rules none ; tmsh delete ltm virtual /Common/bnc_vs ;
#          tmsh delete ltm rule /Common/bnc_rN ; tmsh delete ltm data-group internal /Common/bnc_dg ;
#          tmsh delete ltm pool /Common/bnc_pool ; tmsh delete auth partition TenantA ; tmsh delete auth partition TenantB
```

### Index A: the 73 harness checks

"Tests waiting" counts the rows of the test-scrub ledgers (`docs/work/test-scrub/ledger/*.tsv`) that name the check.

| Check | Area | What it settles | Environment | On record | Tests waiting | Ledger |
|---|---|---|---|---|---|---|
| BIGIP-NAME-001 | Identity | Interpreter identity and execution frame | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-002 | Command name resolution | Current namespace before global; absolute name bypasses the current namespace | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-003 | Command name resolution | Global fallback; no walk through ancestor namespaces | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-004 | Command name resolution | Relative-qualified command names | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-005 | Command name resolution | Runs of colons inside command names | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-006 | Command name resolution | Trailing `::` names the empty-named command | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-007 | Command name resolution | Single colons are ordinary name characters; leading `:` in a namespace | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-008 | Command name resolution | `namespace which -command` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-009 | Command name resolution | `namespace origin` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-010 | Command name resolution | `info commands` with qualified patterns | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-011 | Command name resolution | `rename` to and from qualified names | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-012 | Command name resolution | `interp alias` names and target anchoring | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-013 | Command name resolution | Alias record after `rename` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-014 | Command name resolution | `unknown` and `namespace unknown` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-015 | Command name resolution | Command heads that are braced, substituted, computed or list-built | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-016 | Command name resolution | Unique-prefix abbreviation of built-in subcommands | TMM, tmsh, iApp, iCall | no | 0 | - |
| BIGIP-NAME-017 | Namespaces | `namespace eval` creation and name normalisation | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-018 | Namespaces | `namespace current`, `parent`, `children` and their missing-namespace messages | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-019 | Namespaces | `namespace qualifiers` and `namespace tail` on odd spellings | TMM, tmsh, iApp, iCall | no | 0 | - |
| BIGIP-NAME-020 | Namespaces | `namespace exists` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-021 | Namespaces | Deleting and recreating a namespace | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-022 | Namespaces | Tcl 8.5+ `namespace` subcommands are absent | TMM, tmsh, iApp, iCall | yes; rerun on other builds | 0 | - |
| BIGIP-NAME-023 | Namespaces | `namespace code` and `namespace inscope` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-024 | Namespaces | `namespace import`, `export`, `forget` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-025 | Variables and array elements | Qualified variable names: read, write, missing parent | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-026 | Variables and array elements | Unqualified variables inside `namespace eval`: global fallback and creation | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-027 | Variables and array elements | The `variable` command | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-028 | Variables and array elements | The `global` command with plain and qualified names | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-029 | Variables and array elements | `upvar` name and level forms | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-030 | Variables and array elements | `uplevel` level forms | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-031 | Variables and array elements | Array element names given to commands (`set name(index)`) | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-032 | Variables and array elements | Variable names with unusual bytes | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-033 | Variables and array elements | `info exists`, `info vars`, `info globals` with qualified names and patterns | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-034 | `$` substitution | `$name(index)`: where the index ends and what it may contain | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-035 | `$` substitution | `${name}` and `${name(index)}` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-036 | `$` substitution | Qualified names followed by `(` | TMM, tmsh, iApp, iCall | no | 0 | - |
| BIGIP-NAME-037 | `$` substitution | Which characters end an unbraced `$name` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-038 | `$` substitution | A lone `$`, `$$`, `$(`, and an unterminated index | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-039 | `$` substitution | Non-ASCII bytes after an unbraced `$name` | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-040 | Procedure naming | `proc` with a qualified name whose namespace does not exist | TMM, tmsh, iApp, iCall | no | 0 | - |
| BIGIP-NAME-041 | Procedure naming | `proc` with a relative name inside `namespace eval` | TMM, tmsh, iApp, iCall | yes; rerun on other builds | 0 | - |
| BIGIP-NAME-042 | Procedure naming | `info procs`, `info args`, `info body`, `info default` with qualified names | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-043 | Procedure naming | `proc` defined inside a proc; redefinition | TMM, tmsh, iApp, iCall | partly | 0 | - |
| BIGIP-NAME-044 | Procedure naming | Formal parameter names that are qualified or look like array elements | TMM, tmsh, iApp, iCall | no | 0 | - |
| BIGIP-NAME-045 | iRules in TMM | Where a top-level iRule `proc` lives | TMM RULE_INIT | partly | 0 | - |
| BIGIP-NAME-046 | iRules in TMM | `call` name forms across iRules | TMM RULE_INIT | yes; rerun on other builds | 0 | - |
| BIGIP-NAME-047 | iRules in TMM | Inside a called iRule proc | TMM RULE_INIT and HTTP_REQUEST | partly | 0 | - |
| BIGIP-NAME-048 | iRules in TMM | Is `static` a Tcl namespace? | TMM RULE_INIT | partly | 0 | - |
| BIGIP-NAME-049 | iRules in TMM | `static::` across events, procs and TMMs | TMM RULE_INIT + HTTP_REQUEST | partly | 2 | taint 2 |
| BIGIP-NAME-050 | iRules in TMM | Bare variables in `RULE_INIT` | TMM RULE_INIT | partly | 1 | taint 1 |
| BIGIP-NAME-051 | iRules in TMM | `namespace eval` plus `set` from an event frame | TMM RULE_INIT | partly | 0 | - |
| BIGIP-NAME-052 | iRules in TMM | Event-local variables against `::` globals | TMM HTTP_REQUEST | partly | 0 | - |
| BIGIP-NAME-053 | iRules in TMM | Are protocol commands real namespaced commands? | TMM RULE_INIT | partly | 0 | - |
| BIGIP-NAME-054 | iRules in TMM | Protocol commands from other namespaces and procs; user shadowing | TMM HTTP_REQUEST | partly | 0 | - |
| BIGIP-NAME-055 | iRules in TMM | `rename` of a protocol command (lab only) | TMM RULE_INIT | partly | 0 | - |
| BIGIP-NAME-056 | iRules in TMM | Subcommand abbreviation in iRules commands | TMM HTTP_REQUEST | no | 0 | - |
| BIGIP-NAME-057 | iRules in TMM | Load-time resolution of literal command names | TMM rule load | yes; rerun on other builds | 1 | server-e2e-db 1 |
| BIGIP-NAME-058 | iRules in TMM | `table` keys are not Tcl names | TMM HTTP_REQUEST | no | 0 | - |
| BIGIP-NAME-059 | iRules in TMM | Inventory of namespaces and commands in TMM | TMM RULE_INIT | partly | 0 | - |
| BIGIP-NAME-068 | iRules in TMM | `HTTP::header NAME VALUE` with no subcommand, before and after `HTTP::respond` | TMM rule load + HTTP_REQUEST | no | 0 | - |
| BIGIP-NAME-070 | iRules in TMM | A backslash inside a braced argument | TMM HTTP_REQUEST | no | 0 | - |
| BIGIP-NAME-071 | iRules in TMM | What runs after a response, a drop or a DNS answer, and payload without collect | TMM HTTP_REQUEST, DNS_REQUEST | no | 32 | taint 32 |
| BIGIP-NAME-072 | iRules in TMM | A plain variable across the events of one connection, and `global` | TMM CLIENT_ACCEPTED + HTTP_REQUEST | no | 8 | taint 8 |
| BIGIP-NAME-073 | iRules in TMM | Is there an `env` array in TMM | TMM RULE_INIT + HTTP_REQUEST | no | 1 | taint 1 |
| BIGIP-NAME-060 | tmsh, iApp and iCall | Execution namespace and frame of tmsh-hosted scripts | tmsh cli script | partly | 0 | - |
| BIGIP-NAME-061 | tmsh, iApp and iCall | The `tmsh::` command namespace | tmsh cli script | partly | 0 | - |
| BIGIP-NAME-062 | tmsh, iApp and iCall | Where procs defined in a cli script live | tmsh cli script | partly | 0 | - |
| BIGIP-NAME-063 | tmsh, iApp and iCall | cli script argument variables `tmsh::argc` and `tmsh::argv` | tmsh cli script | no | 0 | - |
| BIGIP-NAME-064 | tmsh, iApp and iCall | APL fields as Tcl variables in an iApp implementation | iApp implementation | no | 0 | - |
| BIGIP-NAME-065 | tmsh, iApp and iCall | `__` section separators, tables and name collisions in APL variables | iApp implementation | no | 0 | - |
| BIGIP-NAME-066 | tmsh, iApp and iCall | The `iapp::` procs | iApp implementation | partly | 0 | - |
| BIGIP-NAME-067 | tmsh, iApp and iCall | tmsh object field names with `tmsh::get_field_value` | tmsh cli script | no | 0 | - |
| BIGIP-NAME-069 | tmsh, iApp and iCall | Which `proc` body formations define a procedure outside TMM | tmsh cli script | no | 0 | - |

### Index B: the checks and decisions of section 6

| Item | What it settles | Kind | Tests waiting | Ledger |
|---|---|---|---|---|
| 1 | Which 8.4 is the baseline? | decision for a human | - | - |
| 2 | Which BIG-IP releases must be covered | decision for a human | - | - |
| 3 | May the destructive check run? | decision for a human | - | - |
| 4 | Relative `proc` placement in tmsh, iApp and iCall | decision for a human | - | - |
| 5 | The TMM `namespace eval … set` result | check on a BIG-IP | - | - |
| 6 | Rule procs and Tcl namespaces | check on a BIG-IP | - | - |
| 7 | APL Tcl callbacks | decision for a human | - | - |
| 8 | Filing | decision for a human | - | - |
| 9 | `HTTP::header NAME VALUE` after a response | check on a BIG-IP | - | - |
| 10 | Wide shift in an iRule expression | check on a BIG-IP | - | - |
| 11 | String character model | check on a BIG-IP | - | - |
| 12 | Prefix operator before an iRules comparison | check on a BIG-IP | - | - |
| 13 | Constant integer expressions | check on a BIG-IP | - | - |
| 14 | Double overflow | check on a BIG-IP | - | - |
| 15 | Word operators on literals | check on a BIG-IP | - | - |
| 16 | Sanitisers and getter colours the taint diagnostics trust | check on a BIG-IP | - | - |
| 17 | A source literal read numerically after another command read it | check on a BIG-IP | - | - |
| 18 | A top-level statement whose head is a rule procedure | check on a BIG-IP | 4 | analyser-binding (`rule_declaration` tests of `tcl-compiler`) |
| 31 | What `event … disable` touches | check on a BIG-IP | 0 | none yet: filed for the registry change of the 1712 audit |
| 19 | TS-01, per-TMM state | check on a BIG-IP | 17 | irules-registry-tooling 17 |
| 20 | TS-02, `class match` operators | check on a BIG-IP | 1 | irules-registry-tooling 1 |
| 21 | TS-03, request lifecycle | check on a BIG-IP | 6 | irules-registry-tooling 6 |
| 22 | TS-04, word boolean operators | check on a BIG-IP | 3 | irules-registry-tooling 3 |
| 23 | TS-05, string lengths and maths | check on a BIG-IP | 2 | irules-registry-tooling 2 |
| 24 | TS-06, loader | check on a BIG-IP | 5 | irules-registry-tooling 5 |
| 25 | TS-07, procs named like events | check on a BIG-IP | 2 | irules-registry-tooling 2 |
| 26 | TS-08, partition-aware reachability | check on a BIG-IP | 2 | irules-registry-tooling 2 |
| 27 | TS-09, the quick-fix claims | check on a BIG-IP | 12 | irules-registry-tooling 12 |
| 28 | TS-10, payload round trips | check on a BIG-IP | 3 | irules-registry-tooling 3 |
| 29 | CF-01, a valid `proc` after malformed declarations | check on a BIG-IP | 1 | class-facts 1 |
| 30 | CF-02, a `when` nested in a procedure | check on a BIG-IP | 1 | class-facts 1 |

### Overlaps and conflicts between checks

Checks the other sessions added were folded in without renumbering (the ids are cited by ledgers and code); the overlaps are noted here instead so that one rule load or one traffic run answers several.

- **One loader session: item 18, CF-01, CF-02, TS-06.** All four ask whether the rule loader accepts a rule and what it keeps. Load each rule on its own (`tmsh load sys config merge file`) and record ACCEPT or the loader's message; the same procedure answers all of them.
- **One per-TMM run: BIGIP-NAME-049, 072 and TS-01.** TS-01's `static::` half restates 049 and its plain-variable half restates 072; it adds `after` callbacks and re-saving the rule. Run 049 and 072 first; TS-01 only needs the extra traffic.
- **Item 18 extends BIGIP-NAME-057 (c); it is not a duplicate.** 057 (c) measured a direct `call` of a top-level proc from inside an event (rejected). Item 18 asks about a bare top-level statement, an unknown head, a `proc` named like a handler (`when`), and a variable-valued `priority`.
- **Items 15 and TS-04 ask about the same words** (`and`, `or`, `not`, `eq` in an expression) from two sides, folding and evaluation; one rule answers both.
- **One conflict between tests, decided by item 18 (c).** The passing test `rule_declaration::direct_loader_keeps_later_procedure_body_without_installation_proof` expects the loader to keep reading after an unknown top-level statement (`unresolvedOperation`) and to record the later `proc helper`. The rule-loader rejection model (`rule_loader_rejection` in `rule_declaration.rs`, written from F5 K36322151) treats such a statement as reported at load and reaching no handler. If the device rejects the whole rule on an unknown head, the first test's premise (a later proc is still inventoried) is moot; if it rejects only the statement, both stand. Check 18 (c) is the one measurement that decides it.
- No other pair of tests was found that expects opposite answers from one check: for TS-01 to TS-10 and CF-01, CF-02 every waiting test on a check expects the same answer.

## 1. Purpose and rule

The toolchain (language server, compiler, VM) resolves Tcl names through one
shared layer, `rust/tcl-syntax/src/naming.rs` and `rust/tcl-syntax/src/naming/`.
For C Tcl 8.4 to 9.1 and for Jim Tcl every rule in that layer is checked
against real interpreters. The F5 dialects cannot be checked that way: iRules
run inside TMM, and tmsh scripts, iApp implementations and iCall scripts run
inside the Tcl interpreters of tmsh and scriptd.

**Rule: F5 Tcl is treated as Tcl 8.4 unless a measurement on a BIG-IP says
otherwise.** F5's embedded Tcl forked from Tcl 8.4.6
(`Family::F5_FORK_POINT`, `rust/tcl-dialect/src/model/family.rs`). At present
`NativeNameProtocol::for_point` (`rust/tcl-syntax/src/naming/native.rs`)
returns no naming recipe at all for the F5 families; the checks below decide
where the 8.4 recipe can be used and where an F5-specific rule is needed.

"Name handling" covers:

- command lookup: relative and `::`-absolute names, lookup order, colon runs,
  `rename`, `interp alias`, `unknown`;
- namespaces: creation, `namespace current/parent/children/qualifiers/tail/exists`,
  deletion, import and export, which subcommands exist;
- variables: `variable`, `global`, `upvar`, `uplevel`, qualified names,
  fallback from a namespace to the global scope;
- array elements: where `name(index)` splits and what an index may contain;
- `$name` parsing: which characters belong to a name, `${…}`, `$name(index)`;
- procedure naming: where `proc` creates a command, formal parameter names;
- F5 name forms: iRule procs and `call`, `static::`, protocol commands such as
  `HTTP::header`, `tmsh::` commands, APL variables in iApps.

### Baselines

| Baseline | What it is |
|---|---|
| Tcl 8.4 column | Tcl 8.4.20 unless a row says otherwise. The repository's conformance vectors were run against tclsh 8.4.20, 8.5.19, 8.6.14, 9.0.4 and 9.1b0. |
| On-box control | `/usr/bin/tclsh8.4` on the appliance (8.4.13 on BIG-IP 21.1.0.1). A stock interpreter, useful for confirming the Tcl 8.4 column; it is not a BIG-IP execution context. |
| Embedded Tcl | What `info patchlevel` reports inside TMM, tmsh, iApp and iCall: 8.4.6 on BIG-IP 21.1.0.1. |

A result that differs from 8.4.20 but agrees with 8.4.6 is not an F5 change.
Where that is suspected the check says so; report the result either way.

### Basis codes in the expected-result tables

| Code | Meaning |
|---|---|
| `V-cmd`, `V-ns`, `V-var` | Stated by `rust/tcl-syntax/tests/data/command_resolution_vectors.txt`, `namespace_op_vectors.txt` or `variable_resolution_vectors.txt`; each row there was run on real tclsh 8.4.20. |
| `T84` | Recorded run of the appliance's own tclsh 8.4.13: `evidence/tcl/bigip-nh20261012stock2-stock-8.4-<case>.md` and its `stock3`, `pat1`, `pat3` siblings. The row names the case. |
| `N` | An 8.4-specific branch of the Rust name layer (`rust/tcl-syntax/src/naming/native.rs`, `rust/tcl-syntax/src/formal_params.rs`), written from the Tcl 8.4 C sources. |
| `S` | Standard Tcl 8.4 behaviour (8.4 manual pages and C sources); not separately pinned in this repository. Confirm on the on-box tclsh8.4 where there is one. |
| to be confirmed with tclsh8.4 | The value shown is the working assumption. Run the line on tclsh8.4 (the on-box one will do) and report that result too. |

### Verdicts

Give one verdict per check and environment:

- `matches 8.4`: every line equals the Tcl 8.4 column;
- `differs`: at least one line differs; say which;
- `command unavailable`: the command under test does not exist or is refused
  in this environment (give the message);
- `recorded`: only for checks marked "No Tcl 8.4 equivalent".

## 2. Environments to test

### 2.0 Rules for every environment

**Safety.**

- Use a lab unit. Never run `tmsh save sys config`.
- Before creating any object, check the name is free (`tmsh list <type> <name>`
  must fail). Delete every object afterwards and check it is gone.
- One check can damage a running system and is marked: BIGIP-NAME-055 renames
  a built-in iRules command and renames it back.
- Interpreters outlive the objects: TMM keeps procs, namespaces and global
  variables after a rule is reloaded or deleted. The harness removes what it
  created; do not skip that part.

**Names.**

| Thing | Name |
|---|---|
| Tcl names created by check N (namespaces, procs, variables) | start with `bnqN` |
| Harness variables | start with `bnc_` |
| iRule for check N | `/Common/bnc_rN` (a second, provider rule: `/Common/bnc_pN`) |
| cli script | `/Common/bnc_cN` |
| iApp template and service | `/Common/bnc_aN`, `/Common/bnc_aN_svc` |
| iCall script and handler | `/Common/bnc_iN`, `/Common/bnc_iN_h` |
| Lab virtual server | `/Common/bnc_vs` |

**The harness.** A check is two statements, `set bnc_id …` and
`set bnc_obs {…}`. `bnc_obs` is a list of label and script pairs. The harness
evaluates each script under `catch` and prints one line:

```text
BNC|<check id>|<environment>|<label>|<rc>|<result>
```

`<environment>` is `tmm<N>` (N is `TMM::cmp_unit`), `cli`, `iapp`, `icall` or
`tclsh`. `<rc>` is the `catch` code (0 success, 1 error). Line breaks inside a
result are replaced by ` / `. All lines can be collected with `grep 'BNC|'`.

There are two evaluation modes:

- **global mode**, `uplevel #0 $bnc_s`: the script runs at the global level in
  namespace `::`, which is what the Tcl 8.4 column assumes. This is the default.
- **frame mode**, `eval $bnc_s`: the script runs in the frame of the event
  body, of `script::run`, or of the implementation. Used where the question is
  about that frame. A check says when it needs frame mode.

Around the loop the harness runs a wipe that deletes every namespace, command
and global variable matching `::bnq*`:

```tcl
set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
```

**iRule loader rules** (measured on BIG-IP 21.1.0.1; they shape every iRule
snippet):

1. Every literal command name in a `when` or `proc` body must resolve when the
   rule is loaded, even inside `catch`. An unknown name rejects the whole rule.
2. Literal `namespace`, `rename`, `interp`, `package` and 27 other stock
   commands are refused at load (`command is disabled`), although several of
   them, these four included, exist at run time.
3. `proc` is refused inside a `when` body, including inside the braced script
   argument of `eval` or `uplevel`.

The snippets therefore keep all probe text as braced *data* in `bnc_obs` and
evaluate it from a variable; the loader sees only `set`, `foreach`, `catch`,
`uplevel`/`eval`, `regsub`, `log` and iRules commands such as `TMM::cmp_unit`. If a build rejects a rule because of text
inside `bnc_obs` or `bnc_w`, record the loader message, then carry that script
as hexadecimal instead (`set bnc_s [binary format H* <hex of the script>]`) and
say that you did.

**Logging.** syslog collapses identical consecutive lines and cuts long ones.
Lines carry the TMM unit so that the per-TMM lines differ; do not infer how
many times something ran from the number of lines. If expected lines are
missing, rerun the check with fewer observations per rule.

### 2.1 iRule in TMM

`RULE_INIT` runs once on every TMM when the rule is loaded. The rule does not
need to be attached to a virtual server. Expect one set of lines per TMM; if
the TMMs disagree, return all of them.

Put the snippet in a file as `ltm rule /Common/bnc_rN { <snippet> }` and run:

```sh
tmsh list ltm rule /Common/bnc_rN >/dev/null 2>&1 && { echo "name in use"; exit 2; }
SINCE=$(wc -l < /var/log/ltm)
tmsh load sys config merge file /var/tmp/bnc_rN.conf     # record ACCEPT or the loader message
sleep 1
tail -n +"$SINCE" /var/log/ltm | grep -o 'BNC|.*' | sort -u
tmsh delete ltm rule /Common/bnc_rN
tmsh list ltm rule /Common/bnc_rN >/dev/null 2>&1 && echo "STILL PRESENT"
```

`tmsh load sys config merge file` is the only loading path that works; there
is no `tmsh -f` and a rule cannot be piped to tmsh.

Global-mode `RULE_INIT` wrapper (the form every generic check is printed in):

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-NNN
    set bnc_obs {
        label {script}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

**Traffic events.** Checks that say `HTTP_REQUEST` need a virtual server with
TCP and HTTP profiles and one or more requests. The rule answers the request
itself, so no pool is needed:

```tcl
ltm virtual /Common/bnc_vs {
    destination <lab address>:19912
    mask 255.255.255.255
    ip-protocol tcp
    profiles { tcp {} http {} }
    rules { /Common/bnc_rN }
}
```

Send requests with `curl -s -H 'Connection: close' http://<lab address>:19912/`
until every TMM has logged (a request reaches one TMM; 32 requests are usually
enough on four TMMs). Delete the virtual server before the rule. Check that the
unit column shows more than one TMM: a rule that the loader sees using global
variables is demoted from CMP and then only one TMM answers.

Frame-mode `HTTP_REQUEST` wrapper:

```tcl
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-NNN
    set bnc_obs {
        label {script}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
}
```

`static::` variables are per TMM. A value written in `RULE_INIT` is present on
every TMM because `RULE_INIT` runs on every TMM; a value written in a traffic
event exists only on the TMM that handled that connection.

### 2.2 tmsh cli script

The script body is the proc `script::run`. Output written with `puts` goes to
the terminal.

```tcl
cli script /Common/bnc_cN {
proc script::run {} {
    set bnc_id BIGIP-NAME-NNN
    set bnc_obs {
        label {script}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        puts "BNC|$bnc_id|cli|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
}
```

```sh
tmsh list cli script /Common/bnc_cN >/dev/null 2>&1 && { echo "name in use"; exit 2; }
tmsh load sys config merge file /var/tmp/bnc_cN.conf
tmsh run cli script /Common/bnc_cN | grep '^BNC|'
tmsh delete cli script /Common/bnc_cN
```

Known on 21.1.0.1: `namespace current` inside `script::run` is `::script`, and
the variable `tcl_patchLevel` does not exist there (use `info patchlevel`).

### 2.3 iApp implementation

The implementation section of an iApp template is tmsh Tcl run by scriptd.
Loading the configuration creates the service without running the
implementation; force a run with `execute-action definition`. Log with
`tmsh::log err`: lower levels do not reach `/var/log/ltm`. (`puts` output goes
to `/var/tmp/scriptd.out`.)

```tcl
sys application template /Common/bnc_aN {
  actions { definition {
    implementation {
        set bnc_id BIGIP-NAME-NNN
        set bnc_obs {
            label {script}
        }
        set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
        catch {uplevel #0 $bnc_w}
        foreach {bnc_l bnc_s} $bnc_obs {
            set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
            tmsh::log err "BNC|$bnc_id|iapp|$bnc_l|$bnc_rc|$bnc_r"
        }
        catch {uplevel #0 $bnc_w}
    }
    presentation { }
  } }
}
sys application service /Common/bnc_aN_svc { template /Common/bnc_aN }
```

```sh
SINCE=$(wc -l < /var/log/ltm)
tmsh load sys config merge file /var/tmp/bnc_aN.conf
tmsh modify sys application service /Common/bnc_aN_svc execute-action definition
sleep 2
tail -n +"$SINCE" /var/log/ltm | grep -o 'BNC|.*' | sort -u
tmsh delete sys application service /Common/bnc_aN_svc
tmsh delete sys application template /Common/bnc_aN
```

The APL presentation section and Tcl callbacks inside it are separate
contexts. They are out of scope here except for how APL fields reach the
implementation (BIGIP-NAME-064, BIGIP-NAME-065).

### 2.4 iCall script

An iCall script is tmsh Tcl run by scriptd when a handler fires. Use a
triggered handler and generate its event by hand. Use the same body as the
iApp implementation, with `icall` in place of `iapp` in the log line.

```tcl
sys icall script /Common/bnc_iN {
  definition {
      <same statements as the iApp implementation, logging "BNC|$bnc_id|icall|…">
  }
}
sys icall handler triggered /Common/bnc_iN_h {
  script /Common/bnc_iN
  subscriptions { run { event-name BNC_IN } }
}
```

```sh
SINCE=$(wc -l < /var/log/ltm)
tmsh load sys config merge file /var/tmp/bnc_iN.conf
tmsh generate sys icall event name BNC_IN
sleep 4
tail -n +"$SINCE" /var/log/ltm | grep -o 'BNC|.*' | sort -u
tmsh delete sys icall handler triggered /Common/bnc_iN_h
tmsh delete sys icall script /Common/bnc_iN
```

Run every check whose environment list says "tmsh, iApp, iCall" here as well;
iCall has been measured separately from iApp before and the two are not
assumed equal.

### 2.5 On-box tclsh control

Where the appliance has `/usr/bin/tclsh8.4`, run the same `bnc_id` and
`bnc_obs` there. This confirms the Tcl 8.4 column on the spot and settles
every row marked "to be confirmed with tclsh8.4". Record its `info patchlevel`.

```tcl
set bnc_id BIGIP-NAME-NNN
set bnc_obs {
    label {script}
}
foreach {bnc_l bnc_s} $bnc_obs {
    set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
    puts "BNC|$bnc_id|tclsh|$bnc_l|$bnc_rc|$bnc_r"
}
```

```sh
/usr/bin/tclsh8.4 /var/tmp/bnc_NNN.tcl
```

Each run is a fresh process, so no wipe is needed. The iRules and tmsh checks
(areas 4.7 and 4.8) have no control.

### 2.6 Version and build capture

Record once per appliance, and repeat in every returned block:

```sh
tmsh show sys version        # Product, Version, Build, Edition
tmsh show sys hardware       # platform
tmsh list sys provision      # provisioned modules
ls -l /usr/bin/tclsh*        # stock interpreters present
```

Inside each environment run BIGIP-NAME-001 first. It records
`info patchlevel`, `info tclversion`, `namespace current`, `info level`, the
number of commands and the `namespace` subcommand table. Also save the full
command list of each environment once, in chunks that fit a log line:

```tcl
set bnc_all [lsort [info commands]]
for {set bnc_i 0} {$bnc_i < [llength $bnc_all]} {incr bnc_i 15} {
    log local0. "BNC|CMDS|tmm[TMM::cmp_unit]|$bnc_i|0|[lrange $bnc_all $bnc_i [expr {$bnc_i + 14}]]"
}
```

(Use `puts` or `tmsh::log err` and the matching environment word outside TMM.)

## 3. What is already measured

A good deal is on record, all of it from **one appliance release, BIG-IP
21.1.0.1** (the earlier runs record build 0.0.26; the 2026-10-12 runs record
only "21.1.0.1"). Nothing is measured on any other release. The records are
files under `evidence/tcl/` (one per executed script), summarised in proof
pages under `docs/design/analysis/name-resolution-proofs/bigip-*.md` and in
`docs/design/f5/bigip-irule-parser-measurements.md`.

| Evidence (path) | Build recorded | Environment and event | What was measured | Result |
|---|---|---|---|---|
| `docs/design/f5/bigip-irule-parser-measurements.md`; transcripts `scripts/dev/bigip-probes/results/10-context-parity.txt`, `11-proc-semantics.txt`, `12-dynamic-code.txt` | 21.1.0.1 build 0.0.26 | TMM `RULE_INIT` (unattached rule), tmsh cli script, iApp implementation, on-box tclsh 8.4 and 8.5 | Interpreter identity, 8.5 features, proc semantics, load-time rules | All three F5 contexts report 8.4.6 and have no 8.5 feature. iRules resolve literal command names at load; literal `namespace`, `rename`, `interp`, `package` are refused at load but work from evaluated text; `proc` is refused inside `when`; `call` exists only in TMM. |
| `evidence/tcl/bigip-r2286l-p-*.md`, `bigip-r2286l-s-*.md` | 21.1.0.1 | TMM `RULE_INIT` on four TMMs, tmsh cli script, iApp implementation, iCall, on-box tclsh 8.4.13 and 8.5.13 | `info patchlevel`, `tcl_patchLevel`, `tcl_platform`, `namespace current`, command surface | 8.4.6 in the four F5 contexts; `namespace current` is `::` in TMM; `apply`, `dict`, `lassign`, `lmap`, `try`, `throw` absent; `tmsh::version` absent in TMM and `21.1.0.1` in iApp and iCall. |
| `evidence/tcl/bigip-r2286c-dynamic-*.md` (26 files); `docs/design/analysis/name-resolution-proofs/bigip-dynamic-resolution-controls.md` | 21.1.0.1 build 0.0.26 | TMM `HTTP_REQUEST`, units 0 to 3 | Run-time `namespace eval`, `variable`, `global`, `upvar` (1, `#0`, 0), `uplevel`, `interp alias`, `rename`, `namespace import`, namespace delete and recreate, variable traces, `package`, `::ns:::x` | All as Tcl 8.4. `namespace path` gives the 8.4 `bad option` text. `::ns:::x` reads `::ns::x`. `global static::x` creates local `x`. An alias follows its target name after the target is renamed. |
| `evidence/tcl/bigip-log-nh20261012broad1-*.md`, `…broad2-*.md`, `bigip-nh20261012broad1-tmsh-cli-script-0.md`, `bigip-apl-<case>.md` | 21.1.0.1 | TMM `RULE_INIT`, TMM rule proc, TMM `HTTP_REQUEST`, iApp implementation, iCall, tmsh cli script, APL Tcl callback | About 60 name cases: lookup order, colon runs, empty and odd command heads, variable fallback, `global`/`variable`/`upvar` links, import and export, aliases, `rename`, `namespace code`, protocol-command shadowing, error messages | In the three TMM entry points every generic case agrees with Tcl 8.4 (the conformance vectors, where a vector exists). The four tmsh-hosted contexts agree on the cases that define no `proc` by relative name inside `namespace eval` and differ on those that do (next row); they have no iRules commands. |
| `evidence/tcl/bigip-log-nh20261012nsowner1-*.md`, `bigip-nh20261012nsowner1-tmsh-cli-script-0.md` | 21.1.0.1 | TMM (three entry points), iApp implementation, iCall, tmsh cli script | Where `namespace eval ::ns {proc p …}` creates `p` | TMM: `::ns::p` (as 8.4). iApp implementation, iCall and tmsh cli script: `::p`; `::ns::p` does not exist. The cli script's own namespace is `::script`. |
| `evidence/tcl/bigip-log-nh20261012exact1-*.md`, `…exact2-*.md`, `bigip-nh20261012exact1-tmsh-cli-script-*.md`, with on-box controls `bigip-nh20261012stock2-*`, `…stock3-*`, `…pat1-*`, `…pat3-*` | 21.1.0.1 | TMM (three entry points), iApp implementation, iCall, tmsh cli script, on-box tclsh 8.4.13 and 8.5.13 | 45 corpus cases: 15 direct cases (alias prefix after rename, computed command heads, proc factories, frame-forwarding wrappers, `namespace code` prefixes, list-built prefixes) and 30 cases written as `tcltest` tests | Direct cases: TMM equals tclsh 8.4.13 except one, where after `namespace eval ::nh {set prefix …}` the variable `::nh::prefix` did not exist in TMM (`namespace-captured-prefix`); the tmsh-hosted contexts fail the cases that rely on relative `proc` placement. The 30 `tcltest` cases produced no comparable BIG-IP result: TMM has no `tcltest` package, and in the tmsh-hosted contexts loading it stops with `can't read "debug": no such variable`. They ran on the on-box tclsh 8.4.13 and 8.5.13. |
| `evidence/tcl/bigip-log-nh20261012cap2-*.md`, `…cap3-*.md`, `…literal1-*.md`, `…nativecap1-*.md`, `…repair2-*.md` | 21.1.0.1 | TMM (three entry points), iApp implementation, iCall, tmsh cli script | Which `namespace` subcommands, `interp` forms, `trace`, `encoding`, `call`, TclOO, tcltest exist | `namespace ensemble`, `path`, `upvar`: 8.4 `bad option` text everywhere. `encoding` absent in TMM. `call` absent outside TMM. `oo::class` absent everywhere. `tcltest` 2.2.8 only in the tmsh-hosted contexts. |
| `evidence/tcl/bigip-log-nh20261012native1-*.md`, `…native2-*.md`, `…collision1-*.md`, `…persist1-*.md`; `docs/design/analysis/name-resolution-proofs/bigip-procedure-owner-route.md`, `bigip-activated-procedure-replacement.md` | 21.1.0.1 (build 0.0.26 in the proof pages) | TMM `RULE_INIT`, rule proc, `HTTP_REQUEST`; rules in `/Common`, a folder and a second partition | How `call` finds a rule proc and how long the proc lives | Bare name: the calling rule's proc. `rule::proc`: rule at the caller's partition root. `/partition/[folder/]rule::proc`: that rule. `::rule::proc`: refused (`invalid namespace`). `namespace current` inside the proc is `::`. `namespace which -command` is empty for every `call` target. A proc stays callable after its rule's configuration object is deleted. |
| `docs/design/analysis/name-resolution-proofs/bigip-static-state-tmm-locality.md`, `bigip-variable-static-event-currency.md` | 21.1.0.1 build 0.0.26 | `RULE_INIT` and traffic events on TMM 0:0 to 0:3 | `static::` seed, write, unset, recreate | The `RULE_INIT` value is seen on all four TMMs. A write, unset or recreate in an event affects only the TMM that ran it. |
| `evidence/tcl/bigip-r2286m-*.md`, `bigip-r2286p-*.md`, `bigip-r2286q-*.md`, `bigip-r2286s-*.md`, `bigip-r2286t-*.md`; proof pages `bigip-counted-name-producer-identity.md`, `bigip-counted-key-input-boundaries.md` | 21.1.0.1 build 0.0.26 | TMM `HTTP_REQUEST` (all); tmsh cli script, iApp implementation, iCall (the `p` and `s` runs); `RULE_INIT` and `CLIENT_ACCEPTED` (the `q` run) | Names containing NUL or non-ASCII bytes as scalars, array roots, indices and command names; `()`, `(k)`, `A(k(l))`, `A(k)tail`, `:`, `A:B`, `A::B` as variable names | Names are counted byte strings: a NUL does not cut a name and no normalisation happens. `A::B` with no namespace `A` is refused with the 8.4 messages. Any bytes are accepted in an array index. This is the measurement behind `ObservedBigIpNamePolicy` (`rust/tcl-syntax/src/naming/execution_policy.rs`). |
| `docs/design/analysis/name-resolution-proofs/bigip-lexical-byte-singleton-domain.md`, `bigip-cross-context-lexical-names.md`; `evidence/tcl/bigip-r2286m-lexical-*.md`, `bigip-r2286p-lexical-*.md` | 21.1.0.1 build 0.0.26 | TMM `HTTP_REQUEST`; tmsh cli script, iApp implementation, iCall | Which single byte continues an unbraced `$name`; braced names | TMM: only `0-9`, `A-Z`, `a-z`, `_`. tmsh, iApp and iCall: those plus bytes `aa b5 ba c0-d6 d8-f6 f8-ff`. `${…}` reads the full name in every context. `$name(` without `)` is `missing )`. |
| `evidence/tcl/bigip-r2286vl1-*.md` (63 files); proof pages `bigip-variable-read-destroy-recreate.md`, `bigip-variable-assignment-control-frontiers.md`, `bigip-cross-context-variable-links-traces.md` | 21.1.0.1 build 0.0.26 | TMM `HTTP_REQUEST`, units 0 to 3 | Variable traces and the variable-writing commands (`append`, `lappend`, `lset`, `scan`, `binary scan`, `catch`, `info default`) across plain, qualified, array-element, NUL-bearing and non-ASCII names | Trace callbacks receive the exact name bytes; a write through an `upvar` alias reports the alias name. `lassign` does not exist. |
| `evidence/tcl/bigip-r2286l-g-*.md`, `bigip-r2286l-n-*.md` | 21.1.0.1 | TMM `RULE_INIT`, tmsh cli script, iApp implementation, iCall, on-box tclsh | Word formation | F5 breaks a word after a closing brace or quote (`list {a}{b}` is two words) and lets a braced word continue on the next line; `{*}` is not expansion. `${zz}b` is unchanged from Tcl. |

### Differences from Tcl 8.4 already on record

Do not report these as new; do confirm them on other builds.

1. **tmsh, iApp, iCall, APL callback:** a `proc` given a relative name inside
   `namespace eval ::ns { … }` is created in `::`, not in `::ns`.
2. **TMM:** after `namespace eval ::nh {set prefix …}` evaluated from an event
   body, `::nh::prefix` did not exist. Cause not identified (BIGIP-NAME-051).
3. **Unbraced `$name`:** TMM stops at any byte outside `0-9A-Za-z_`; the
   tmsh-hosted contexts also consume Latin-1 letters.
4. **iRules:** command names are resolved when the rule loads; rule procs are
   reached through `call` with rule and partition paths, not through Tcl
   namespace lookup; `static::` values are per TMM.
5. **tmsh cli script:** the body runs in namespace `::script`;
   `tcl_patchLevel` is not set.
6. **Command surface:** TMM has no `unknown`, auto-loader, `encoding` or file
   commands; the tmsh-hosted contexts have no `call`.
7. **Word formation** (not name handling, but it changes what a word is): the
   implicit word break after `}` or `"`, and brace-line continuation.

### Not on record

No measurement exists for: any release other than 21.1.0.1; whether `static`,
`HTTP`, `IP`, `TMM`, `tmsh`, `iapp` are real Tcl namespaces; the spellings
returned by `info commands`, `info procs` and `info vars` patterns;
`namespace qualifiers` and `tail`; the wording of `namespace parent` and
`children` errors; `rename` or abbreviation of protocol commands; formal
parameter name errors; where a bare `RULE_INIT` variable lives; how APL fields
are published to an iApp implementation; `tmsh::argv`.

## 4. Check list

73 checks. By area: Identity 1, Command name resolution 15, Namespaces 8, Variables and array elements 9, `$` substitution 6, Procedure naming 5, iRules in TMM 20, tmsh, iApp and iCall 9.

Every check has: the question; why the toolchain needs the answer (Rust file
and symbol); what is already on record; the snippet; the result Tcl 8.4 gives,
with its basis, and the Tcl 8.6 / 9.0 result where it differs; and an empty
row to fill in. Unless a check says otherwise the snippet is the complete
`when RULE_INIT { … }` body for TMM; for tmsh, iApp and iCall take the
`set bnc_id` and `set bnc_obs` statements unchanged into the §2 wrapper.

In the expected tables, `rc` is the first field after the label and the result
is the last field of the `BNC|…` line. "empty" means the result field is empty.
"same" in the last column means Tcl 8.6 and 9.0 agree with 8.4.

| ID | Area | Check | Environment | On record for 21.1.0.1 |
|---|---|---|---|---|
| BIGIP-NAME-001 | Identity | Interpreter identity and execution frame | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-002 | Command name resolution | Current namespace before global; absolute name bypasses the current namespace | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-003 | Command name resolution | Global fallback; no walk through ancestor namespaces | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-004 | Command name resolution | Relative-qualified command names | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-005 | Command name resolution | Runs of colons inside command names | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-006 | Command name resolution | Trailing `::` names the empty-named command | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-007 | Command name resolution | Single colons are ordinary name characters; leading `:` in a namespace | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-008 | Command name resolution | `namespace which -command` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-009 | Command name resolution | `namespace origin` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-010 | Command name resolution | `info commands` with qualified patterns | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-011 | Command name resolution | `rename` to and from qualified names | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-012 | Command name resolution | `interp alias` names and target anchoring | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-013 | Command name resolution | Alias record after `rename` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-014 | Command name resolution | `unknown` and `namespace unknown` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-015 | Command name resolution | Command heads that are braced, substituted, computed or list-built | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-016 | Command name resolution | Unique-prefix abbreviation of built-in subcommands | TMM, tmsh, iApp, iCall | no |
| BIGIP-NAME-017 | Namespaces | `namespace eval` creation and name normalisation | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-018 | Namespaces | `namespace current`, `parent`, `children` and their missing-namespace messages | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-019 | Namespaces | `namespace qualifiers` and `namespace tail` on odd spellings | TMM, tmsh, iApp, iCall | no |
| BIGIP-NAME-020 | Namespaces | `namespace exists` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-021 | Namespaces | Deleting and recreating a namespace | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-022 | Namespaces | Tcl 8.5+ `namespace` subcommands are absent | TMM, tmsh, iApp, iCall | yes; rerun on other builds |
| BIGIP-NAME-023 | Namespaces | `namespace code` and `namespace inscope` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-024 | Namespaces | `namespace import`, `export`, `forget` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-025 | Variables and array elements | Qualified variable names: read, write, missing parent | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-026 | Variables and array elements | Unqualified variables inside `namespace eval`: global fallback and creation | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-027 | Variables and array elements | The `variable` command | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-028 | Variables and array elements | The `global` command with plain and qualified names | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-029 | Variables and array elements | `upvar` name and level forms | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-030 | Variables and array elements | `uplevel` level forms | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-031 | Variables and array elements | Array element names given to commands (`set name(index)`) | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-032 | Variables and array elements | Variable names with unusual bytes | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-033 | Variables and array elements | `info exists`, `info vars`, `info globals` with qualified names and patterns | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-034 | `$` substitution | `$name(index)`: where the index ends and what it may contain | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-035 | `$` substitution | `${name}` and `${name(index)}` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-036 | `$` substitution | Qualified names followed by `(` | TMM, tmsh, iApp, iCall | no |
| BIGIP-NAME-037 | `$` substitution | Which characters end an unbraced `$name` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-038 | `$` substitution | A lone `$`, `$$`, `$(`, and an unterminated index | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-039 | `$` substitution | Non-ASCII bytes after an unbraced `$name` | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-040 | Procedure naming | `proc` with a qualified name whose namespace does not exist | TMM, tmsh, iApp, iCall | no |
| BIGIP-NAME-041 | Procedure naming | `proc` with a relative name inside `namespace eval` | TMM, tmsh, iApp, iCall | yes; rerun on other builds |
| BIGIP-NAME-042 | Procedure naming | `info procs`, `info args`, `info body`, `info default` with qualified names | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-043 | Procedure naming | `proc` defined inside a proc; redefinition | TMM, tmsh, iApp, iCall | partly |
| BIGIP-NAME-044 | Procedure naming | Formal parameter names that are qualified or look like array elements | TMM, tmsh, iApp, iCall | no |
| BIGIP-NAME-045 | iRules in TMM | Where a top-level iRule `proc` lives | TMM RULE_INIT | partly |
| BIGIP-NAME-046 | iRules in TMM | `call` name forms across iRules | TMM RULE_INIT | yes; rerun on other builds |
| BIGIP-NAME-047 | iRules in TMM | Inside a called iRule proc | TMM RULE_INIT and HTTP_REQUEST | partly |
| BIGIP-NAME-048 | iRules in TMM | Is `static` a Tcl namespace? | TMM RULE_INIT | partly |
| BIGIP-NAME-049 | iRules in TMM | `static::` across events, procs and TMMs | TMM RULE_INIT + HTTP_REQUEST | partly |
| BIGIP-NAME-050 | iRules in TMM | Bare variables in `RULE_INIT` | TMM RULE_INIT | partly |
| BIGIP-NAME-051 | iRules in TMM | `namespace eval` plus `set` from an event frame | TMM RULE_INIT | partly |
| BIGIP-NAME-052 | iRules in TMM | Event-local variables against `::` globals | TMM HTTP_REQUEST | partly |
| BIGIP-NAME-053 | iRules in TMM | Are protocol commands real namespaced commands? | TMM RULE_INIT | partly |
| BIGIP-NAME-054 | iRules in TMM | Protocol commands from other namespaces and procs; user shadowing | TMM HTTP_REQUEST | partly |
| BIGIP-NAME-055 | iRules in TMM | `rename` of a protocol command (lab only) | TMM RULE_INIT | partly |
| BIGIP-NAME-056 | iRules in TMM | Subcommand abbreviation in iRules commands | TMM HTTP_REQUEST | no |
| BIGIP-NAME-057 | iRules in TMM | Load-time resolution of literal command names | TMM rule load | yes; rerun on other builds |
| BIGIP-NAME-058 | iRules in TMM | `table` keys are not Tcl names | TMM HTTP_REQUEST | no |
| BIGIP-NAME-059 | iRules in TMM | Inventory of namespaces and commands in TMM | TMM RULE_INIT | partly |
| BIGIP-NAME-068 | iRules in TMM | `HTTP::header NAME VALUE` with no subcommand, before and after `HTTP::respond` | TMM rule load + HTTP_REQUEST | no |
| BIGIP-NAME-070 | iRules in TMM | A backslash inside a braced argument | TMM HTTP_REQUEST | no |
| BIGIP-NAME-071 | iRules in TMM | What runs after a response, a drop or a DNS answer, and payload without collect | TMM HTTP_REQUEST, DNS_REQUEST | no |
| BIGIP-NAME-072 | iRules in TMM | A plain variable across the events of one connection, and `global` | TMM CLIENT_ACCEPTED + HTTP_REQUEST | no |
| BIGIP-NAME-073 | iRules in TMM | Is there an `env` array in TMM | TMM RULE_INIT + HTTP_REQUEST | no |
| BIGIP-NAME-060 | tmsh, iApp and iCall | Execution namespace and frame of tmsh-hosted scripts | tmsh cli script | partly |
| BIGIP-NAME-061 | tmsh, iApp and iCall | The `tmsh::` command namespace | tmsh cli script | partly |
| BIGIP-NAME-062 | tmsh, iApp and iCall | Where procs defined in a cli script live | tmsh cli script | partly |
| BIGIP-NAME-063 | tmsh, iApp and iCall | cli script argument variables `tmsh::argc` and `tmsh::argv` | tmsh cli script | no |
| BIGIP-NAME-064 | tmsh, iApp and iCall | APL fields as Tcl variables in an iApp implementation | iApp implementation | no |
| BIGIP-NAME-065 | tmsh, iApp and iCall | `__` section separators, tables and name collisions in APL variables | iApp implementation | no |
| BIGIP-NAME-066 | tmsh, iApp and iCall | The `iapp::` procs | iApp implementation | partly |
| BIGIP-NAME-067 | tmsh, iApp and iCall | tmsh object field names with `tmsh::get_field_value` | tmsh cli script | no |
| BIGIP-NAME-069 | tmsh, iApp and iCall | Which `proc` body formations define a procedure outside TMM | tmsh cli script | no |

### 4.1 Identity

Run this first in every environment and attach its lines to every returned block.

#### BIGIP-NAME-001 — Interpreter identity and execution frame

- **Question.** Which Tcl does this environment report, in which namespace and at which call level does the script body run, and what is the `namespace` subcommand table?
- **Why the toolchain needs it.** Every other result is filed against this identity. `Family::F5_FORK_POINT` (`rust/tcl-dialect/src/model/family.rs`) pins the fork at 8.4.6; `BigIpExecutionContext` (`rust/tcl-dialect/src/model/bigip_execution_context.rs`) records 152 commands for TMM and 95 for the tmsh-hosted contexts.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, frame mode (`eval $bnc_s`).
- **Already on record.** 21.1.0.1: `info patchlevel` 8.4.6 in TMM, tmsh cli script, iApp implementation and iCall; `tcl_patchLevel` unset in a cli script; `namespace current` is `::script` in a cli script and `::` elsewhere (`evidence/tcl/bigip-r2286l-p-patchlevel.md`, `evidence/tcl/bigip-nh20261012repair2-tmsh-cli-script-0.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-001
    set bnc_obs {
        ver   {list [info patchlevel] [info tclversion] [info exists ::tcl_patchLevel] [package provide Tcl]}
        frame {list [namespace current] [info level]}
        count {list [llength [info commands]] [llength [info procs]] [llength [namespace children ::]]}
        nssub {catch {namespace bnq1bogus} m; set m}
        plat  {lsort [array names ::tcl_platform]}
        tmm   {list [catch {set c TMM::cmp_count; $c} n] $n}
        tmsh  {list [catch {set c tmsh::version; $c} v] $v}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `ver` | `0` `8.4.20 8.4 1 8.4` | S (tclsh 8.4.20; a BIG-IP context is expected to say 8.4.6) | 8.6.x 8.6 1 8.6.x |
| `frame` | `0` `:: 0` | S | same |
| `count` | `0`, environment-specific | n/a | environment-specific |
| `nssub` | `0` `bad option "bnq1bogus": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which` | V-ns | longer option list |
| `plat` | `0` `byteOrder machine os osVersion platform threaded user wordSize` | T84 (`evidence/tcl/bigip-r2286l-p-platform-keys.md`) | adds pointerSize and others |
| `tmm` | `0` `1 {invalid command name "TMM::cmp_count"}` | S | same |
| `tmsh` | `0` `1 {invalid command name "tmsh::version"}` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

### 4.2 Command name resolution

All checks in this area define procs by absolute name, so that the known relative-`proc` difference in the tmsh-hosted contexts (BIGIP-NAME-041) does not hide the lookup rule under test.

#### BIGIP-NAME-002 — Current namespace before global; absolute name bypasses the current namespace

- **Question.** Does a bare command name resolve in the current namespace first and an absolute name only in the named namespace?
- **Why the toolchain needs it.** Selects the candidate order in `resolve_command_with` and `command_resolution_candidates` (`rust/tcl-syntax/src/naming.rs`). `NativeNameProtocol::for_point` (`rust/tcl-syntax/src/naming/native.rs`) returns `None` for `Family::F5Tcl` and `Family::F5Irules`, so the 8.4 order is an assumption for F5.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, all seven contexts: current namespace wins (`current-before-global` in `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-0.md` and siblings). Absolute bypass measured in TMM only (`…-RULE_INIT-1.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-002
    set bnc_obs {
        def  {namespace eval ::bnq2 {}; proc ::bnq2::bnq2p {} {return NS}; proc ::bnq2p {} {return GLOBAL}; list}
        rel  {namespace eval ::bnq2 {bnq2p}}
        abs  {namespace eval ::bnq2 {::bnq2p}}
        glob {bnq2p}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `rel` | `0` `NS` | V-cmd | same |
| `abs` | `0` `GLOBAL` | V-cmd | same |
| `glob` | `0` `GLOBAL` | V-cmd | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-003 — Global fallback; no walk through ancestor namespaces

- **Question.** When the current namespace lacks the command, is the global namespace the only fallback (no parent namespaces, no `namespace path`)?
- **Why the toolchain needs it.** Same resolver. 8.4 has no `namespace path`, so the candidate list is exactly current, then global (`command_resolution_vectors.txt`, rows `::a::b | - | ::a::helper | helper | -`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM (RULE_INIT, proc, HTTP_REQUEST): no ancestor walk. The tmsh-hosted contexts returned the parent's proc, but that probe defined it with a relative `proc` inside `namespace eval`, which those contexts place in `::` (see BIGIP-NAME-041); this check uses absolute `proc` names to separate the two effects.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-003
    set bnc_obs {
        def    {namespace eval ::bnq3::kid {}; proc ::bnq3::bnq3anc {} {return PARENT}; proc ::bnq3glob {} {return GLOBAL}; list}
        fallbk {namespace eval ::bnq3::kid {bnq3glob}}
        ancest {namespace eval ::bnq3::kid {bnq3anc}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `fallbk` | `0` `GLOBAL` | V-cmd | same |
| `ancest` | `1` `invalid command name "bnq3anc"` | V-cmd | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-004 — Relative-qualified command names

- **Question.** Is `ns::cmd` tried under the current namespace, then under `::`, and does an existing but empty qualifier namespace fail to commit the lookup?
- **Why the toolchain needs it.** Selects the two-base expansion in `command_lookup_bases` / `command_resolution_candidates` (`rust/tcl-syntax/src/naming.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: falls through to the global qualifier (`relative-qualifier-missing-target`, `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-0.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-004
    set bnc_obs {
        def  {namespace eval ::bnq4::bnq4in {}; namespace eval ::bnq4in {}; proc ::bnq4::bnq4in::p {} {return NESTED}; proc ::bnq4in::p {} {return ROOTED}; proc ::bnq4in::q {} {return ROOTQ}; list}
        near {namespace eval ::bnq4 {bnq4in::p}}
        far  {namespace eval ::bnq4 {bnq4in::q}}
        none {namespace eval ::bnq4 {bnq4in::z}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `near` | `0` `NESTED` | V-cmd | same |
| `far` | `0` `ROOTQ` | V-cmd | same |
| `none` | `1` `invalid command name "bnq4in::z"` | V-cmd | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-005 — Runs of colons inside command names

- **Question.** Does a run of two or more colons act as one separator wherever it appears in a command name?
- **Why the toolchain needs it.** Selects `written_separator_extent`, `written_command_parts` and `canonical_written_command` (`rust/tcl-syntax/src/naming.rs`), which collapse a written run to one separator.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: `::ns:::p` reaches `::ns::p` (`colon-run-and-empty-tail`, `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-1.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-005
    set bnc_obs {
        def    {namespace eval ::bnq5 {}; proc ::bnq5::p {} {return P}; list}
        triple {::bnq5:::p}
        many   {:::::bnq5::::p}
        relrun {bnq5:::p}
        innsev {namespace eval ::bnq5 {:::bnq5:::p}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `triple` | `0` `P` | V-cmd (`a:::b`) | same |
| `many` | `0` `P` | to be confirmed with tclsh8.4 | same |
| `relrun` | `0` `P` | V-cmd | same |
| `innsev` | `0` `P` | to be confirmed with tclsh8.4 | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-006 — Trailing `::` names the empty-named command

- **Question.** Does `ns::` (any number of trailing colons) name the command whose simple name is the empty string in `ns`?
- **Why the toolchain needs it.** Selects `ends_with_separator` and `written_command_tail` (`rust/tcl-syntax/src/naming.rs`) and the empty-tail slot in `NativeNameProtocol::command_lookup_slot` (`rust/tcl-syntax/src/naming/native.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: `::ns:::` reached the `{}` proc (`colon-run-and-empty-tail`); empty computed head reached `{}` in all seven contexts (`empty-command-head`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-006
    set bnc_obs {
        def   {namespace eval ::bnq6 {}; proc ::bnq6:: {} {return EMPTYTAIL}; list}
        two   {::bnq6::}
        three {::bnq6:::}
        rel   {bnq6::}
        which {namespace which -command ::bnq6::}
        info  {info commands ::bnq6::*}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | V-ns | same |
| `two` | `0` `EMPTYTAIL` | V-cmd (`::x::`) | same |
| `three` | `0` `EMPTYTAIL` | V-cmd (`::x:::`) | same |
| `rel` | `0` `EMPTYTAIL` | V-cmd (`x::`) | same |
| `which` | `0` `::bnq6::` | to be confirmed with tclsh8.4 | same |
| `info` | `0` `::bnq6::` | to be confirmed with tclsh8.4 | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-007 — Single colons are ordinary name characters; leading `:` in a namespace

- **Question.** Is a lone `:` an ordinary character in a command name, and is a `proc` whose name starts with `:` refused inside a non-global namespace?
- **Why the toolchain needs it.** The lone-colon rows of `command_resolution_vectors.txt` carry `8.4-8.5=!ERROR` for a leading-colon proc in a non-global namespace; F5 is assumed to take the 8.4 column. Publication goes through `NativeNameProtocol::command_publication_input` (`rust/tcl-syntax/src/naming/native.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 tmsh-hosted contexts only: a proc named `:` defined inside `namespace eval` was callable (`lone-colon-command`), consistent with those contexts creating it in `::`. No TMM result on record.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-007
    set bnc_obs {
        def   {namespace eval ::bnq7 {}; proc ::bnq7::a:b {} {return MID}; proc ::bnq7::c: {} {return END}; proc ::bnq7:g {} {return GLOBALCOLON}; list}
        mid   {::bnq7::a:b}
        end   {::bnq7::c:}
        glob  {bnq7:g}
        which {namespace eval ::bnq7 {namespace which -command c:}}
        lead  {set c [catch {namespace eval ::bnq7 {proc :bnq7l {} {return LEAD}}} m]; catch {rename :bnq7l {}}; list $c $m}
        leadg {proc :bnq7g {} {return LEADG}; set r [:bnq7g]; rename :bnq7g {}; set r}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `mid` | `0` `MID` | V-cmd | same |
| `end` | `0` `END` | V-ns (proc-1.6 rows) | same |
| `glob` | `0` `GLOBALCOLON` | V-cmd (`a:b`) | same |
| `which` | `0` `::bnq7::c:` | V-ns | same |
| `lead` | `0` `1 {can't create procedure ":bnq7l" in non-global namespace with name starting with ":"}` | V-cmd header (8.4 and 8.5 refuse) | 8.6+: `0 {}` |
| `leadg` | `0` `LEADG` | V-cmd | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-008 — `namespace which -command`

- **Question.** What does `namespace which` return for relative, fallback, absolute, missing and built-in names?
- **Why the toolchain needs it.** Drives `original_script_name_query_support` (`rust/tcl-syntax/src/naming/script_name_query.rs`) and every consumer that turns a written name into a fully qualified one.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1: `namespace which -command set` is `::set` in TMM, tmsh, iApp and iCall (`docs/design/analysis/name-resolution-proofs/bigip-namespace-command-currency.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-008
    set bnc_obs {
        def     {namespace eval ::bnq8 {}; proc ::bnq8::p {} {}; proc ::bnq8g {} {}; list}
        rel     {namespace eval ::bnq8 {namespace which -command p}}
        fall    {namespace eval ::bnq8 {namespace which -command bnq8g}}
        abs     {namespace which -command ::bnq8::p}
        miss    {namespace which -command ::bnq8::none}
        builtin {namespace eval ::bnq8 {namespace which -command set}}
        noflag  {namespace which bnq8g}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `rel` | `0` `::bnq8::p` | V-ns | same |
| `fall` | `0` `::bnq8g` | V-ns | same |
| `abs` | `0` `::bnq8::p` | V-ns | same |
| `miss` | `0`, empty | V-ns | same |
| `builtin` | `0` `::set` | V-ns (`which foreach`) | same |
| `noflag` | `0` `::bnq8g` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-009 — `namespace origin`

- **Question.** Does `namespace origin` follow an import to its source, return a plain command's own name, and fail with `invalid command name` for a missing one?
- **Why the toolchain needs it.** Import chains and origin reports are modelled from the import rows of `namespace_op_vectors.txt`; consumers use them to navigate from an imported name to its definition.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: origin of an import chain is the source command (`import-chain-origin`, `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-9.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-009
    set bnc_obs {
        def   {namespace eval ::bnq9a {namespace export p}; proc ::bnq9a::p {} {return A}; namespace eval ::bnq9b {namespace import ::bnq9a::p}; list}
        imp   {namespace origin ::bnq9b::p}
        plain {namespace origin ::bnq9a::p}
        built {namespace origin set}
        miss  {namespace origin ::bnq9b::none}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `imp` | `0` `::bnq9a::p` | V-ns | same |
| `plain` | `0` `::bnq9a::p` | V-ns | same |
| `built` | `0` `::set` | V-ns | same |
| `miss` | `1` `invalid command name "::bnq9b::none"` | to be confirmed with tclsh8.4 | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-010 — `info commands` with qualified patterns

- **Question.** Which spelling does `info commands` return for qualified, relative-qualified, rooted and bare patterns?
- **Why the toolchain needs it.** Drives `original_script_command_inventory_support` (`rust/tcl-syntax/src/naming/script_command_inventory.rs`) and `native_qualified_inventory_report_bytes` (`rust/tcl-syntax/src/naming/native.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: `info commands ::ns::name` returned the fully qualified name (`namespace-definition-owner`, `evidence/tcl/bigip-log-nh20261012nsowner1-tmm-RULE_INIT-0.md`). Pattern forms not otherwise measured.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-010
    set bnc_obs {
        def    {namespace eval ::bnq10 {}; proc ::bnq10::p {} {}; proc ::bnq10::q {} {}; proc ::bnq10g {} {}; list}
        qual   {lsort [info commands ::bnq10::*]}
        relq   {lsort [info commands bnq10::*]}
        bare   {info commands bnq10g}
        rooted {info commands ::bnq10g}
        inns   {namespace eval ::bnq10 {lsort [info commands bnq10*]}}
        innsp  {namespace eval ::bnq10 {info commands p}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `qual` | `0` `::bnq10::p ::bnq10::q` | V-ns (`commands ::test_ns_1::*`) | same |
| `relq` | `0` `::bnq10::p ::bnq10::q` | to be confirmed with tclsh8.4 | same |
| `bare` | `0` `bnq10g` | S | same |
| `rooted` | `0` `::bnq10g` | to be confirmed with tclsh8.4 | same |
| `inns` | `0` `bnq10g` | S (current namespace, then unshadowed globals) | same |
| `innsp` | `0` `p` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-011 — `rename` to and from qualified names

- **Question.** Does `rename` move a command between namespaces, create a missing destination namespace, and report the three failure messages?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::rename_source_input`, `rename_destination_slot` and `rename_diagnostic` (`rust/tcl-syntax/src/naming/native.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: a renamed proc runs in its destination namespace (`rename-procedure-namespace`); `can't rename to "…": command already exists` (`error-rename-conflict`). `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-12.md`, `…-18.md`.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-011
    set bnc_obs {
        def     {namespace eval ::bnq11a {variable v A}; namespace eval ::bnq11b {variable v B}; proc ::bnq11a::p {} {variable v; return $v}; proc ::bnq11a::s {} {return S}; list}
        move    {rename ::bnq11a::p ::bnq11b::q; list [::bnq11b::q] [namespace which -command ::bnq11a::p]}
        toexist {rename ::bnq11b::q ::bnq11a::s}
        nosrc   {rename ::bnq11a::none ::bnq11a::t}
        delmiss {rename ::bnq11a::none {}}
        tonew   {rename ::bnq11a::s ::bnq11new::r; list [namespace exists ::bnq11new] [namespace which -command ::bnq11new::r]}
        delete  {rename ::bnq11b::q {}; namespace which -command ::bnq11b::q}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `move` | `0` `B {}` | T84 (`wiki-rename-1`: a renamed proc runs in its destination namespace) | same |
| `toexist` | `1` `can't rename to "::bnq11a::s": command already exists` | N (`rename_diagnostic`) | same |
| `nosrc` | `1` `can't rename "::bnq11a::none": command doesn't exist` | N | same |
| `delmiss` | `1` `can't delete "::bnq11a::none": command doesn't exist` | N | same |
| `tonew` | `0` `1 ::bnq11new::r` | to be confirmed with tclsh8.4 | same |
| `delete` | `0`, empty | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-012 — `interp alias` names and target anchoring

- **Question.** Is an alias created at the written qualified name (creating a missing namespace), and is a relative target resolved from the global namespace when the alias is invoked?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::alias_publication_input`, `alias_publication_slot` and `alias_target_string_precedes_capture` (`rust/tcl-syntax/src/naming/native.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: relative target anchored at `::` (`alias-target-global-anchor`); missing destination namespace accepted and description returned in all seven contexts (`alias-destination-missing-parent`, `alias-describe-prefix`). `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-10.md`, `…-11.md`.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-012
    set bnc_obs {
        def    {namespace eval ::bnq12 {}; proc ::bnq12::bnq12t {args} {return [list LOCAL $args]}; proc ::bnq12t {args} {return [list GLOBAL $args]}; list}
        mk     {interp alias {} ::bnq12::a {} ::bnq12::bnq12t fixed}
        call   {::bnq12::a x}
        query  {interp alias {} ::bnq12::a}
        anchor {namespace eval ::bnq12 {interp alias {} ::bnq12::b {} bnq12t}; namespace eval ::bnq12 {::bnq12::b}}
        newns  {interp alias {} ::bnq12new::c {} ::bnq12t; list [namespace exists ::bnq12new] [::bnq12new::c]}
        gone   {interp alias {} ::bnq12::a {}; namespace which -command ::bnq12::a}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `mk` | `0` `::bnq12::a` | S | same |
| `call` | `0` `LOCAL {fixed x}` | S | same |
| `query` | `0` `::bnq12::bnq12t fixed` | S | same |
| `anchor` | `0` `GLOBAL {}` | S | same |
| `newns` | `0` `1 {GLOBAL {}}` | to be confirmed with tclsh8.4 | same |
| `gone` | `0`, empty | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-013 — Alias record after `rename`

- **Question.** After an alias command is renamed, does `interp alias {} <old name>` still describe it, and does the renamed command keep its fixed arguments?
- **Why the toolchain needs it.** `NativeNameProjection::alias_registration_replacement_key` and `alias_registration_key` (`rust/tcl-syntax/src/naming/native.rs`) keep 8.4's alias-record coordinate separate from the command's current slot.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM (RULE_INIT, proc, HTTP_REQUEST) agrees with on-box tclsh 8.4.13 (`alias-prefix-lifetime`, `evidence/tcl/bigip-log-nh20261012exact2-tmm-RULE_INIT-*.md`). tmsh-hosted contexts failed earlier for the proc-placement reason in BIGIP-NAME-041.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-013
    set bnc_obs {
        def  {namespace eval ::bnq13 {}; proc ::bnq13::t {a b} {list $a $b}; interp alias {} ::bnq13::a {} ::bnq13::t {a b}; rename ::bnq13::a ::bnq13::r; list}
        old  {interp alias {} ::bnq13::a}
        call {::bnq13::r {c d}}
        new  {list [catch {interp alias {} ::bnq13::r} m] $m}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `old` | `0` `::bnq13::t {a b}` | T84 (`alias-prefix-lifetime`) | same on tclsh 8.5.13 |
| `call` | `0` `{a b} {c d}` | T84 | same |
| `new` | `0`, to be recorded | to be confirmed with tclsh8.4 | to be recorded |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-014 — `unknown` and `namespace unknown`

- **Question.** Is there an `unknown` command and auto-loader in the interpreter, is `namespace unknown` absent, and does an `unknown` proc in a child interpreter receive unresolved names?
- **Why the toolchain needs it.** The 8.4 column of the `namespace unknown` rows in `namespace_op_vectors.txt` is a `bad option` error; `autoload_command_candidates` (`rust/tcl-syntax/src/naming/autoload.rs`) assumes an auto-loader exists only where `unknown` does.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1: `unknown`, `auto_load` and friends are absent from TMM (`docs/design/f5/bigip-irule-parser-measurements.md` §4b); a child-interpreter `unknown` handler works in all seven contexts (`unknown-isolated-child`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-014
    set bnc_obs {
        exists {info commands unknown}
        nsunk  {namespace unknown}
        child  {set i [interp create]; interp eval $i {proc unknown {args} {return [list UNK $args]}}; set r [interp eval $i {bnq14nosuch a b}]; interp delete $i; set r}
        auto   {list [info exists ::auto_path] [llength [info commands auto_load]]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `exists` | `0` `unknown` | S (stock tclsh with `init.tcl`) | same |
| `nsunk` | `1` `bad option "unknown": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which` | V-ns | `0 ::unknown` |
| `child` | `0` `UNK {bnq14nosuch a b}` | S | same |
| `auto` | `0` `1 1` | S (stock tclsh) | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-015 — Command heads that are braced, substituted, computed or list-built

- **Question.** Does a command name with a space or assembled at run time reach the same command as its literal spelling, and does string interpolation split it?
- **Why the toolchain needs it.** Selects `native_command_source_word` (`rust/tcl-syntax/src/naming/native.rs`) and `word_is_dynamic` (`rust/tcl-syntax/src/naming.rs`): the name is the whole first word after substitution.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, all seven contexts: braced, quoted, substituted and list-prefix heads reach the proc; an interpolated script string splits at the space (`name-braced-head`, `name-substituted-head`, `name-list-prefix`, `name-script-prefix-near-miss`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-015
    set bnc_obs {
        def    {namespace eval ::bnq15 {}; proc {::bnq15::two words} {x} {return $x}; proc ::bnq15::h {} {return H}; list}
        braced {{::bnq15::two words} A}
        subst  {set h {::bnq15::two words}; $h B}
        comp   {set n ::bnq15; ${n}::h}
        strng  {set h {::bnq15::two words}; eval "$h C"}
        listp  {set p [list {::bnq15::two words} D]; eval $p}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `braced` | `0` `A` | T84 (`list-prefix`) | same |
| `subst` | `0` `B` | T84 | same |
| `comp` | `0` `H` | T84 (`computed-qualified`) | same |
| `strng` | `1` `invalid command name "::bnq15::two"` | T84 (`string-prefix-near-miss`) | same |
| `listp` | `0` `D` | T84 (`list-prefix`) | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-016 — Unique-prefix abbreviation of built-in subcommands

- **Question.** Are unique prefixes of built-in subcommand names accepted, and is an ambiguous prefix refused?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::namespace_subcommand_prefix_input` and `interpreter_subcommand_input` (`rust/tcl-syntax/src/naming/native.rs`); the analyser must recognise `namespace cur` as `namespace current`.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Not measured on BIG-IP.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-016
    set bnc_obs {
        string {string len abc}
        ns     {namespace cur}
        info   {info exi bnq16none}
        array  {array nam bnq16none}
        ambig  {catch {namespace e} m; string range $m 0 20}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `string` | `0` `3` | S | same |
| `ns` | `0` `::` | S | same |
| `info` | `0` `0` | S | same |
| `array` | `0`, empty | S | same |
| `ambig` | `0` `ambiguous option "e":` | to be confirmed with tclsh8.4 | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

### 4.3 Namespaces

#### BIGIP-NAME-017 — `namespace eval` creation and name normalisation

- **Question.** Does `namespace eval` create every missing level, collapse colon runs and ignore trailing colons?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::namespace_address_input` and `namespace_creation_reuses_parent` (`rust/tcl-syntax/src/naming/native.rs`); the colon-normalisation rows of `namespace_op_vectors.txt` state the 8.4 answer.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1: dynamic `namespace eval ::a::b {}` works in all contexts; normalised spellings not measured.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-017
    set bnc_obs {
        deep  {namespace eval ::bnq17::a::b {namespace current}}
        kids  {lsort [namespace children ::bnq17]}
        run   {namespace eval :::bnq17x::::y {namespace current}}
        trail {namespace eval bnq17z::: {namespace current}}
        rel   {namespace eval ::bnq17 {namespace eval c::d {namespace current}}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `deep` | `0` `::bnq17::a::b` | V-ns | same |
| `kids` | `0` `::bnq17::a` | V-ns | same |
| `run` | `0` `::bnq17x::y` | V-ns (`:::test_ns_1::::foo`) | same |
| `trail` | `0` `::bnq17z` | V-ns (`test_ns_7:::`) | same |
| `rel` | `0` `::bnq17::c::d` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-018 — `namespace current`, `parent`, `children` and their missing-namespace messages

- **Question.** What do the three queries return, and which wording does a missing namespace produce?
- **Why the toolchain needs it.** `report_native_namespace_operation_error` (`rust/tcl-syntax/src/naming/native.rs`) has an 8.4-only `legacy` branch: `unknown namespace "…" in namespace <sub> command`. 8.5 and later say `namespace "…" not found`. F5 is assumed to use the 8.4 wording.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, all seven contexts: `unknown namespace "…" in inscope namespace command` (8.4 wording) for `namespace inscope` (`error-missing-namespace`). `parent` and `children` wording not measured.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-018
    set bnc_obs {
        def     {namespace eval ::bnq18::kid {}; list}
        cur     {namespace eval ::bnq18::kid {list [namespace current] [namespace parent]}}
        parg    {list [namespace parent ::bnq18::kid] [namespace parent ::bnq18] [namespace parent ::]}
        kids    {list [namespace children ::bnq18] [namespace children :: ::bnq18*] [namespace children ::bnq18::kid]}
        parmiss {namespace parent ::bnq18::none}
        kidmiss {namespace children ::bnq18::none}
        relmiss {namespace eval ::bnq18 {namespace children none}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `cur` | `0` `::bnq18::kid ::bnq18` | S | same |
| `parg` | `0` `::bnq18 :: {}` | S | same |
| `kids` | `0` `::bnq18::kid ::bnq18 {}` | V-ns | same |
| `parmiss` | `1` `unknown namespace "::bnq18::none" in namespace parent command` | N (`legacy` branch) | `namespace "::bnq18::none" not found` |
| `kidmiss` | `1` `unknown namespace "::bnq18::none" in namespace children command` | V-ns | `namespace "::bnq18::none" not found` |
| `relmiss` | `1` `unknown namespace "none" in namespace children command` | V-ns | `namespace "none" not found in "::bnq18"` |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-019 — `namespace qualifiers` and `namespace tail` on odd spellings

- **Question.** How do the two string operations treat colon runs, trailing colons and single colons?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::namespace_qualifier_bytes`, `namespace_tail_bytes` and `namespace_text_result` (`rust/tcl-syntax/src/naming/native.rs`); the last has an 8.4-specific result form.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Not measured on BIG-IP.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-019
    set bnc_obs {
        quals {list [namespace qualifiers ::a::b::c] [namespace qualifiers a:::b] [namespace qualifiers :::::] [namespace qualifiers foo:::] [namespace qualifiers a:b] [namespace qualifiers ::a]}
        tails {list [namespace tail ::a::b::c] [namespace tail a:::b] [namespace tail :::::] [namespace tail foo:::] [namespace tail a:b] [namespace tail ::]}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `quals` | `0` `::a::b a {} foo {} {}` | V-ns for `:::::` and `foo:::`; S for the rest | same |
| `tails` | `0` `c b {} {} a:b {}` | V-ns for `:::::`, `foo:::`, `::`; S for the rest | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-020 — `namespace exists`

- **Question.** Does `namespace exists` accept relative names, colon runs and trailing colons?
- **Why the toolchain needs it.** Namespace addresses go through `NativeNameProtocol::namespace_address_input` (`rust/tcl-syntax/src/naming/native.rs`); the probe harnesses in this document also rely on this command.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1: `namespace exists` is used by every recorded probe guard in all contexts; odd spellings not measured.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-020
    set bnc_obs {
        ex {list [namespace exists ::] [namespace exists ::bnq20] [namespace eval ::bnq20::k {}] [namespace exists ::bnq20::k] [namespace exists bnq20] [namespace exists ::bnq20:::k] [namespace exists ::bnq20::k::]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `ex` | `0` `1 0 {} 1 1 1 1` | V-ns (existence rows); S for the colon spellings | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-021 — Deleting and recreating a namespace

- **Question.** Do the namespace's commands and variables disappear on `namespace delete`, and does a recreated namespace start empty?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::namespace_command_teardown` and the `Operation::Delete` branch of `report_native_namespace_operation_error` (`rust/tcl-syntax/src/naming/native.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM HTTP_REQUEST (build 0.0.26): delete then recreate gives the new proc (`evidence/tcl/bigip-r2286c-dynamic-namespace-delete.md`); recreation while active measured in TMM (`active-namespace-delete-recreate`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-021
    set bnc_obs {
        def     {namespace eval ::bnq21 {variable v OLD}; proc ::bnq21::p {} {return OLD}; list}
        del     {namespace delete ::bnq21; list [namespace exists ::bnq21] [namespace which -command ::bnq21::p] [info exists ::bnq21::v]}
        gone    {::bnq21::p}
        again   {namespace eval ::bnq21 {variable v NEW}; proc ::bnq21::p {} {return NEW}; list [::bnq21::p] [set ::bnq21::v]}
        delmiss {namespace delete ::bnq21none}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `del` | `0` `0 {} 0` | V-ns | same |
| `gone` | `1` `invalid command name "::bnq21::p"` | V-ns | same |
| `again` | `0` `NEW NEW` | S | same |
| `delmiss` | `1` `unknown namespace "::bnq21none" in namespace delete command` | N | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-022 — Tcl 8.5+ `namespace` subcommands are absent

- **Question.** Are `namespace ensemble`, `namespace path` and `namespace upvar` rejected as unknown subcommands?
- **Why the toolchain needs it.** The `8.4=!ERROR` rows of `command_resolution_vectors.txt` and the `bad option` columns of `namespace_op_vectors.txt` / `variable_resolution_vectors.txt`; `NativeNameProtocol::namespace_upvar_local_input` returns `PurposeNotModelled` for 8.4.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, TMM (three entry points), iApp, iCall, tmsh cli: all three give the 8.4 `bad option` text (`namespace.ensemble`, `namespace.path`, `namespace.upvar` in `evidence/tcl/bigip-log-nh20261012cap2-*.md`, `…cap3-*.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-022
    set bnc_obs {
        ens   {namespace eval ::bnq22 {namespace ensemble create}}
        path  {namespace eval ::bnq22 {namespace path}}
        upvar {namespace eval ::bnq22 {namespace upvar :: bnq22a bnq22b}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `ens` | `1` `bad option "ensemble": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which` | V-ns | `0 ::bnq22` |
| `path` | `1` `bad option "path": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which` | V-ns | `0`, empty |
| `upvar` | `1` `bad option "upvar": must be children, code, current, delete, eval, exists, export, forget, import, inscope, origin, parent, qualifiers, tail, or which` | V-var | `0`, empty |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-023 — `namespace code` and `namespace inscope`

- **Question.** Does `namespace code` capture the defining namespace, and how does `namespace inscope` pass extra arguments and report a missing namespace?
- **Why the toolchain needs it.** Callback prefixes are resolved by the analyser from the captured namespace; the missing-namespace wording is the `Operation::Inscope` branch of `report_native_namespace_operation_error`.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, six contexts: `namespace code` returns `::namespace inscope <ns> {script}` (`namespace.code`); deleted target gives the 8.4 wording in all seven (`namespace-code-deleted`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-023
    set bnc_obs {
        def  {namespace eval ::bnq23 {}; proc ::bnq23::bnq23p {a} {return [list NS $a]}; proc ::bnq23p {a} {return [list GLOBAL $a]}; list}
        code {namespace eval ::bnq23 {namespace code {bnq23p X}}}
        run  {eval [namespace eval ::bnq23 {namespace code {bnq23p X}}]}
        insc {namespace inscope ::bnq23 bnq23p {Y Z}}
        miss {namespace inscope ::bnq23none {set x}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `code` | `0` `::namespace inscope ::bnq23 {bnq23p X}` | S | same |
| `run` | `0` `NS X` | S | same |
| `insc` | `0` `NS {Y Z}` | S | same |
| `miss` | `1` `unknown namespace "::bnq23none" in inscope namespace command` | N | `namespace "::bnq23none" not found` |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-024 — `namespace import`, `export`, `forget`

- **Question.** Do imports create commands in the importing namespace, is re-import of the same command a no-op, and are conflicts refused without `-force`?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::namespace_pattern_input`, `namespace_import_forces` and `namespace_export_clears` (`rust/tcl-syntax/src/naming/native.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM: import, `-force`, conflict message and origin all as 8.4 (`import-*` cases, `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-8.md`, `…-9.md`, `…-18.md`). On build 0.0.26 a second run of an import script in TMM answered `can't import command "p": already exists` (`evidence/tcl/bigip-r2286c-dynamic-namespace-import.md`); 8.4.20 treats re-import of the same origin as a no-op, so the `again` row may show 8.4.6 behaviour rather than an F5 change.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-024
    set bnc_obs {
        def    {namespace eval ::bnq24a {namespace export p}; proc ::bnq24a::p {} {return A}; proc ::bnq24a::q {} {return Q}; namespace eval ::bnq24b {}; list}
        imp    {namespace eval ::bnq24b {namespace import ::bnq24a::*}; lsort [info commands ::bnq24b::*]}
        call   {::bnq24b::p}
        again  {namespace eval ::bnq24b {namespace import ::bnq24a::p}}
        clash  {proc ::bnq24b::q {} {return LOCAL}; namespace eval ::bnq24a {namespace export q}; namespace eval ::bnq24b {namespace import ::bnq24a::q}}
        force  {namespace eval ::bnq24b {namespace import -force ::bnq24a::q}; ::bnq24b::q}
        forget {namespace eval ::bnq24b {namespace forget ::bnq24a::p}; namespace which -command ::bnq24b::p}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `imp` | `0` `::bnq24b::p` | V-ns | same |
| `call` | `0` `A` | V-ns | same |
| `again` | `0`, empty | V-ns (pinned on 8.4.20; confirm on 8.4.6) | same |
| `clash` | `1` `can't import command "q": already exists` | V-ns | same |
| `force` | `0` `Q` | V-ns | same |
| `forget` | `0`, empty | V-ns | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

### 4.4 Variables and array elements

#### BIGIP-NAME-025 — Qualified variable names: read, write, missing parent

- **Question.** Do `::ns::v`, `ns::v` and colon-run spellings address the namespace variable, and what are the missing-variable and missing-namespace messages?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::variable_root_input` and `report_native_variable_diagnostic` (`rust/tcl-syntax/src/naming/native.rs`) and `is_qualified` (`rust/tcl-syntax/src/naming.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26 TMM HTTP_REQUEST: `::ns:::x` equals `::ns::x` (`evidence/tcl/bigip-r2286c-dynamic-colon-names.md`). 21.1.0.1, all seven contexts: `can't set "absent::x": parent namespace doesn't exist`, `can't read "::ns::absent": no such variable` (`relative-variable-missing-parent`, `error-missing-variable`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-025
    set bnc_obs {
        def    {namespace eval ::bnq25 {variable v NSVAL}; set ::bnq25g GVAL; list}
        abs    {set ::bnq25::v}
        rel    {set bnq25::v}
        dollar {list $::bnq25::v $bnq25::v ${::bnq25::v} $::bnq25g}
        wr     {set ::bnq25::w 5; set bnq25::w}
        nopar  {set ::bnq25none::x 1}
        rdmiss {set ::bnq25::none}
        triple {set ::bnq25:::v}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `abs` | `0` `NSVAL` | V-var | same |
| `rel` | `0` `NSVAL` | V-var | same |
| `dollar` | `0` `NSVAL NSVAL NSVAL GVAL` | S | same |
| `wr` | `0` `5` | S | same |
| `nopar` | `1` `can't set "::bnq25none::x": parent namespace doesn't exist` | V-var | same |
| `rdmiss` | `1` `can't read "::bnq25::none": no such variable` | V-var | same |
| `triple` | `0` `NSVAL` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-026 — Unqualified variables inside `namespace eval`: global fallback and creation

- **Question.** Inside `namespace eval`, does an unqualified read fall back to the global variable, does a write to an existing global go through to it, and is a new variable created in the namespace?
- **Why the toolchain needs it.** This is the TIP 278 axis (`8.4-8.6` against `9.0+` columns in `variable_resolution_vectors.txt`), selected through `tcl_dialect::VariableLookupPolicy` and `source_variable_name_is_frame_local` (`rust/tcl-syntax/src/naming.rs`). F5 is assumed to keep the 8.x fallback.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, all seven contexts: read and `info exists` fall back to the global; a write did not create a namespace variable (`namespace-missing-read-global-fallback`, `namespace-exists-global-fallback`, `namespace-missing-write-local`, `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-3.md`, `…-4.md`). Creation of a new variable from an event frame differs in TMM; see BIGIP-NAME-051.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-026
    set bnc_obs {
        def    {set ::bnq26g GLOBAL; namespace eval ::bnq26 {}; list}
        read   {namespace eval ::bnq26 {set bnq26g}}
        exists {namespace eval ::bnq26 {info exists bnq26g}}
        which  {namespace eval ::bnq26 {namespace which -variable bnq26g}}
        write  {namespace eval ::bnq26 {set bnq26g CHANGED}; list $::bnq26g [info exists ::bnq26::bnq26g]}
        create {namespace eval ::bnq26 {set bnq26new 1}; list [info exists ::bnq26::bnq26new] [info exists ::bnq26new]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `read` | `0` `GLOBAL` | V-var | 9.0: `1` `can't read "bnq26g": no such variable` |
| `exists` | `0` `1` | V-var | 9.0: `0` |
| `which` | `0` `::bnq26g` | V-var | 9.0: empty |
| `write` | `0` `CHANGED 0` | V-var | 9.0: `GLOBAL 1` |
| `create` | `0` `1 0` | V-var | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-027 — The `variable` command

- **Question.** How does `variable` treat alternating name/value arguments, a trailing name without a value, qualified names and array-element names?
- **Why the toolchain needs it.** Declarations feed `NativeCompiledVariableRecipe` (`rust/tcl-syntax/src/naming/compiled_variables.rs`), which has 8.4-specific handling of declared names.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, all seven contexts: `variable a b` assigns `b` to `a` and declares nothing called `b` (`variable-alternating-arguments`). TMM: a proc's `variable x` links the namespace variable (`variable-declaration-link`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-027
    set bnc_obs {
        decl   {namespace eval ::bnq27 {variable a 1 b 2 c}; list [set ::bnq27::a] [set ::bnq27::b] [info exists ::bnq27::c] [namespace which -variable ::bnq27::c]}
        inproc {proc ::bnq27::p {} {variable a; incr a; return $a}; ::bnq27::p}
        qual   {proc ::bnq27q {} {variable ::bnq27::b; return $b}; ::bnq27q}
        undef  {proc ::bnq27::r {} {variable c; info exists c}; ::bnq27::r}
        alt    {namespace eval ::bnq27 {variable x y; list $x [info exists y]}}
        arr    {namespace eval ::bnq27 {variable e(k) 1}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `decl` | `0` `1 2 0 ::bnq27::c` | S | same |
| `inproc` | `0` `2` | V-var | same |
| `qual` | `0` `2` | V-var | same |
| `undef` | `0` `0` | S | same |
| `alt` | `0` `y 0` | S | same |
| `arr` | `1` `can't define "e(k)": name refers to an element in an array` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-028 — The `global` command with plain and qualified names

- **Question.** Does `global` link a local to the global variable, and does a qualified argument create a local named after the tail?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::variable_alias_local_input` (`rust/tcl-syntax/src/naming/native.rs`); iRules rely on `global static::x`-style declarations.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26 TMM HTTP_REQUEST: `global name` links (`evidence/tcl/bigip-r2286c-dynamic-global-alias.md`); `global static::x` creates local `x` (`…-dynamic-static-global-link.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-028
    set bnc_obs {
        def      {set ::bnq28g G; namespace eval ::bnq28 {variable v NS}; list}
        plain    {proc ::bnq28::p {} {global bnq28g; return $bnq28g}; ::bnq28::p}
        qual     {proc ::bnq28q {} {global ::bnq28::v; return $v}; ::bnq28q}
        relq     {proc ::bnq28r {} {global bnq28::v; return $v}; ::bnq28r}
        new      {proc ::bnq28s {} {global bnq28new; set bnq28new 1}; ::bnq28s; info exists ::bnq28new}
        toplevel {global bnq28g; set bnq28g}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `plain` | `0` `G` | V-var | same |
| `qual` | `0` `NS` | S | same |
| `relq` | `0` `NS` | S | same |
| `new` | `0` `1` | S | same |
| `toplevel` | `0` `G` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-029 — `upvar` name and level forms

- **Question.** Do `upvar 1`, the default level, `#0`, `0`, a qualified source and an array-element source link the expected variable, and may the local name look like an array element?
- **Why the toolchain needs it.** Selects `variable_alias_local_input`; the vector row `upvar(#0 aaaaa foo(bar))` is `8.4=0 …;8.5+=1 {bad variable name …}`, so F5 is assumed to allow the 8.4 form.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26 TMM HTTP_REQUEST: `upvar 1`, `upvar #0`, `upvar 0`, unset through a link (`evidence/tcl/bigip-r2286c-dynamic-upvar*.md`). The array-looking local name is not measured.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-029
    set bnc_obs {
        lvl1   {proc ::bnq29set {n} {upvar 1 $n v; set v SET}; proc ::bnq29c {} {set x 0; ::bnq29set x; return $x}; ::bnq29c}
        deflt  {proc ::bnq29set2 {n} {upvar $n v; set v DEF}; proc ::bnq29c2 {} {set x 0; ::bnq29set2 x; return $x}; ::bnq29c2}
        abs0   {set ::bnq29g 0; proc ::bnq29a {} {upvar #0 bnq29g v; set v ABS}; ::bnq29a; set ::bnq29g}
        zero   {proc ::bnq29z {} {set x 0; upvar 0 x y; set y ZERO; return $x}; ::bnq29z}
        qual   {namespace eval ::bnq29 {variable q 0}; proc ::bnq29q {} {upvar ::bnq29::q v; set v QUAL}; ::bnq29q; set ::bnq29::q}
        elem   {proc ::bnq29e {} {set a(k) 0; upvar 0 a(k) e; set e ELEM; return $a(k)}; ::bnq29e}
        arrloc {set i [interp create]; set c [catch {interp eval $i {set g 1; upvar #0 g l(bar); set g}} m]; interp delete $i; list $c $m}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `lvl1` | `0` `SET` | S | same |
| `deflt` | `0` `DEF` | S | same |
| `abs0` | `0` `ABS` | V-var | same |
| `zero` | `0` `ZERO` | S | same |
| `qual` | `0` `QUAL` | V-var | same |
| `elem` | `0` `ELEM` | V-var | same |
| `arrloc` | `0` `0 1` | V-var | 8.5+: `1 {bad variable name "l(bar)": can't create a scalar variable that looks like an array element}` |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-030 — `uplevel` level forms

- **Question.** Do `uplevel 1`, the default level and `#0` evaluate in the expected frame and namespace, and what does `info level` report?
- **Why the toolchain needs it.** Frame targets for forwarded names come from the frame kinds in `variable_resolution_vectors.txt`; the harness in this document also depends on `uplevel #0` reaching the global frame.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26 TMM HTTP_REQUEST: `uplevel 1` and `uplevel #0` as 8.4 (`evidence/tcl/bigip-r2286c-dynamic-uplevel.md`, `…-uplevel-absolute.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-030
    set bnc_obs {
        one   {proc ::bnq30u {} {uplevel 1 {set x UP}}; proc ::bnq30c {} {set x 0; ::bnq30u; return $x}; ::bnq30c}
        deflt {proc ::bnq30u2 {} {uplevel {set x DEF}}; proc ::bnq30c2 {} {set x 0; ::bnq30u2; return $x}; ::bnq30c2}
        abs   {proc ::bnq30u3 {} {uplevel #0 {set bnq30g ABS}}; ::bnq30u3; set ::bnq30g}
        ns    {proc ::bnq30u4 {} {uplevel 1 {namespace current}}; namespace eval ::bnq30 {::bnq30u4}}
        level {proc ::bnq30l {} {info level}; proc ::bnq30l2 {} {::bnq30l}; list [info level] [::bnq30l] [::bnq30l2]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `one` | `0` `UP` | S | same |
| `deflt` | `0` `DEF` | S | same |
| `abs` | `0` `ABS` | S | same |
| `ns` | `0` `::bnq30` | S | same |
| `level` | `0` `0 1 2` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-031 — Array element names given to commands (`set name(index)`)

- **Question.** Where does a combined `name(index)` operand split, and which bytes may the index contain?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::combined_variable_input` (`rust/tcl-syntax/src/naming/native.rs`) and `split_array_name` / `split_element_ref` (`rust/tcl-syntax/src/naming.rs`): first `(` to final `)`.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1, all seven contexts: `arr(a::b(c))` has index `a::b(c)` (`array-index-colons-parentheses`). Build 0.0.26 TMM HTTP_REQUEST: `()`, `(k)`, `A()`, `A(k(l))`, `A(k)tail` all accepted (`evidence/tcl/bigip-r2286t-counted-input-boundaries.md`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-031
    set bnc_obs {
        basic   {set bnq31a(k) 1; list [array exists bnq31a] [array names bnq31a]}
        nested  {set {bnq31a(x(y))} 2; lsort [array names bnq31a]}
        space   {set {bnq31a(a b)} 3; set {bnq31a(a b)}}
        empty   {set bnq31a() 4; set bnq31a()}
        colons  {set {bnq31a(p::q)} 5; set {bnq31a(p::q)}}
        nsarr   {namespace eval ::bnq31 {}; set ::bnq31::arr(k) 6; list [array exists ::bnq31::arr] [set ::bnq31::arr(k)]}
        tail    {set {bnq31b(k)x} 7; list [array exists bnq31b] [info exists {bnq31b(k)x}]}
        scalarr {set bnq31s 1; set bnq31s(k) 2}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `basic` | `0` `1 k` | S | same |
| `nested` | `0` `k x(y)` | S | same |
| `space` | `0` `3` | S | same |
| `empty` | `0` `4` | S | same |
| `colons` | `0` `5` | S | same |
| `nsarr` | `0` `1 6` | S | same |
| `tail` | `0` `0 1` | S | same |
| `scalarr` | `1` `can't set "bnq31s(k)": variable isn't array` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-032 — Variable names with unusual bytes

- **Question.** Are the empty name, names containing `$` or braces, and names or indices containing a NUL byte distinct variables?
- **Why the toolchain needs it.** `NativeNameProtocol::variable_root_input` selects the C-string extent (cut at NUL) for `C(V8_4)`, while `ObservedBigIpNamePolicy::variable_input` (`rust/tcl-syntax/src/naming/execution_policy.rs`) keeps the full counted key for the one measured scope. This check decides which holds elsewhere.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26 TMM HTTP_REQUEST and, for the portable variants, tmsh/iApp/iCall: NUL-bearing scalar, array-root and index names are distinct from their short forms (`evidence/tcl/bigip-r2286m-nul-*.md`, `bigip-r2286p-nul-*.md`, `bigip-r2286t-*.md`). 21.1.0.1 all seven contexts: `{}`, `{$n}`, `{{a}}` are ordinary names (`literal-variable-name-data`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-032
    set bnc_obs {
        empty  {set {} E; set r [set {}]; unset {}; set r}
        dollar {set {$bnq32} D; set r [set {$bnq32}]; unset {$bnq32}; set r}
        brace  {set {bnq32{a}} B; set {bnq32{a}}}
        nul    {set n bnq32[binary format H* 00]x; set $n NUL; set ::bnq32 PLAIN; set r [list [string length $n] [set $n] [set ::bnq32] [info exists $n]]; unset $n; set r}
        nulidx {set k a[binary format H* 00]b; set bnq32arr($k) NULIDX; set bnq32arr(a) SHORT; list [string length $k] [set bnq32arr($k)] [set bnq32arr(a)] [array size bnq32arr]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `empty` | `0` `E` | S | same |
| `dollar` | `0` `D` | S | same |
| `brace` | `0` `B` | S | same |
| `nul` | `0`, to be recorded: `7 NUL PLAIN 1` if names are counted, `7 PLAIN PLAIN 1` if cut at the NUL | to be confirmed with tclsh8.4 (BIG-IP evidence so far: counted) | to be recorded |
| `nulidx` | `0`, to be recorded: `3 NULIDX SHORT 2` if counted, `3 SHORT SHORT 1` if cut | to be confirmed with tclsh8.4 (BIG-IP evidence so far: counted) | to be recorded |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-033 — `info exists`, `info vars`, `info globals` with qualified names and patterns

- **Question.** Which spellings do the variable inventory commands accept and return?
- **Why the toolchain needs it.** Drives `NativeNameProtocol::namespace_variable_query_input` (`rust/tcl-syntax/src/naming/native.rs`) and `original_namespace_variable_snapshot_support` (`rust/tcl-syntax/src/naming/script_namespace_snapshot.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1: `info vars ::prefix*` and `info exists` on qualified names are used by every recorded probe guard; returned spellings not recorded as a result.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-033
    set bnc_obs {
        def     {namespace eval ::bnq33 {variable bnq33a 1; variable bnq33arr; set bnq33arr(k) 1}; set ::bnq33g 1; list}
        exists  {list [info exists ::bnq33::bnq33a] [info exists bnq33::bnq33a] [info exists ::bnq33::bnq33arr(k)] [info exists ::bnq33::bnq33arr(z)] [info exists ::bnq33::none] [info exists ::bnq33none::x]}
        vars    {lsort [info vars ::bnq33::*]}
        relv    {lsort [info vars bnq33::*]}
        glob    {list [info vars bnq33g] [info vars ::bnq33g]}
        inns    {namespace eval ::bnq33 {lsort [info vars bnq33*]}}
        globals {list [info globals bnq33g] [info globals ::bnq33g] [info globals :bnq33g]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `exists` | `0` `1 1 1 0 0 0` | S | same |
| `vars` | `0` `::bnq33::bnq33a ::bnq33::bnq33arr` | to be confirmed with tclsh8.4 | same |
| `relv` | `0` `::bnq33::bnq33a ::bnq33::bnq33arr` | to be confirmed with tclsh8.4 | same |
| `glob` | `0` `bnq33g ::bnq33g` | to be confirmed with tclsh8.4 | same |
| `inns` | `0` `bnq33a bnq33arr bnq33g` | to be confirmed with tclsh8.4 | same |
| `globals` | `0` `bnq33g bnq33g {}` | V-ns (`globals` rows) | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

### 4.5 `$` substitution

#### BIGIP-NAME-034 — `$name(index)`: where the index ends and what it may contain

- **Question.** In a `$name(index)` substitution, is the index everything up to the first `)`, with variable and command substitution inside it and spaces allowed?
- **Why the toolchain needs it.** `GRAMMAR_F5_TCL` (`rust/tcl-dialect/src/model/family.rs`) inherits `array_index: ArrayIndexSyntax::Tcl8` and `var_syntax: VarSyntax::Tcl` from the 8.4 grammar (`rust/tcl-dialect/src/grammar.rs`); the scanner is `scan_var_ref` (`rust/tcl-lexer/src/word_parts.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26, TMM and tmsh-hosted contexts: an unterminated index is `missing )` (lexical byte scan, `docs/design/analysis/name-resolution-proofs/bigip-lexical-byte-singleton-domain.md`). Index content rules not measured through `$`.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-034
    set bnc_obs {
        def   {array set bnq34 {k K {x(y} XY {a b} AB p::q PQ {} EMPTY}; set bnq34i k; list}
        plain {set r $bnq34(k)}
        var   {set r $bnq34($bnq34i)}
        cmd   {set r $bnq34([string tolower K])}
        space {set r $bnq34(a b)}
        nest  {set r $bnq34(x(y))}
        empty {set r $bnq34()}
        colon {set r $bnq34(p::q)}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `plain` | `0` `K` | S | same |
| `var` | `0` `K` | S | same |
| `cmd` | `0` `K` | S | same |
| `space` | `0` `AB` | S | same |
| `nest` | `0` `XY)` | S (first `)` closes; `ArrayIndexSyntax::Tcl8`) | 8.6 same; 9.0 rejects `(` in an index (`ArrayIndexSyntax::Tcl9`) |
| `empty` | `0` `EMPTY` | S | same |
| `colon` | `0` `PQ` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-035 — `${name}` and `${name(index)}`

- **Question.** Does a braced reference end at the first `}`, accept any characters, and treat a trailing `(index)` as an array element?
- **Why the toolchain needs it.** `GRAMMAR_F5_TCL` inherits `braced_var: BracedVarStyle::FirstClose` (`rust/tcl-dialect/src/grammar.rs`); `split_array_name_braced_for_style` (`rust/tcl-syntax/src/naming.rs`) splits the element.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 all four parser contexts: `${zz}b` is `XXb` (`evidence/tcl/bigip-r2286l-g-dollarbrace.md`). Build 0.0.26: braced references read the full byte-valued name in every context (`bigip-r2286m-lexical-*-braced.md`). First-close and element forms not measured.
- **Note.** `set bnq35d\x7bx FIRST` creates the variable named `bnq35d{x` without writing an unbalanced brace in the source.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-035
    set bnc_obs {
        def   {set bnq35a(k) ELEM; set {bnq35 b} SPACE; set bnq35d\x7bx FIRST; set bnq35e ESC; list}
        elem  {set r ${bnq35a(k)}}
        space {set r ${bnq35 b}}
        close {set r ${bnq35d{x}y}}
        adj   {set r ${bnq35e}tail}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `elem` | `0` `ELEM` | S | same |
| `space` | `0` `SPACE` | S | same |
| `close` | `0` `FIRSTy}` | S (`BracedVarStyle::FirstClose`) | 8.6 same; 9.0: `1` `can't read "bnq35d{x}y": no such variable` |
| `adj` | `0` `ESCtail` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-036 — Qualified names followed by `(`

- **Question.** Does `$ns::arr(k)` read an element of a namespace array, and does `$ns::(k)` read the array whose simple name is empty?
- **Why the toolchain needs it.** Selects `variable_reference_root_bytes` (`rust/tcl-syntax/src/naming.rs`) and `scan_var_ref`: the name part includes the `::` runs, the index starts at `(`.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Not measured on BIG-IP.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-036
    set bnc_obs {
        def    {namespace eval ::bnq36 {variable arr; set arr(k) NSARR; variable {}; set (e) EMPTYNAME}; set bnq36i k; list}
        qual   {set r $::bnq36::arr(k)}
        rel    {set r $bnq36::arr(k)}
        idx    {set r $::bnq36::arr($bnq36i)}
        braced {set r ${::bnq36::arr(k)}}
        etail  {set r $::bnq36::(e)}
        clean  {catch {unset {}}; list}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `qual` | `0` `NSARR` | S | same |
| `rel` | `0` `NSARR` | S | same |
| `idx` | `0` `NSARR` | S | same |
| `braced` | `0` `NSARR` | S | same |
| `etail` | `0` `EMPTYNAME` | to be confirmed with tclsh8.4 | same |
| `clean` | `0`, empty | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-037 — Which characters end an unbraced `$name`

- **Question.** Does an unbraced name consist of ASCII letters, digits, underscore and runs of two or more colons, with a single colon ending it?
- **Why the toolchain needs it.** `VarSyntax::Tcl` (`rust/tcl-dialect/src/grammar.rs`) and `is_bare_var_name` (`rust/tcl-syntax/src/naming.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26: complete single-byte scan. TMM consumes only `0-9A-Za-z_`; tmsh, iApp and iCall also consume bytes `aa b5 ba c0-d6 d8-f6 f8-ff` (`docs/design/analysis/name-resolution-proofs/bigip-lexical-byte-singleton-domain.md`). Colon runs inside `$name` not measured.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-037
    set bnc_obs {
        def    {set bnq37 V; set bnq37_x U; namespace eval ::bnq37n {variable v NV; variable w NW}; list}
        under  {set r "$bnq37_x,$bnq37-x,$bnq37.x"}
        colon1 {set r "$bnq37:b"}
        colon2 {set r "$bnq37n::v"}
        colon3 {set r "$bnq37n:::v"}
        trail  {set r "$bnq37n::"}
        lead   {set r "$::bnq37,${::bnq37}"}
        chain  {set r "$::bnq37n::v$::bnq37n::w"}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `under` | `0` `U,V-x,V.x` | S | same |
| `colon1` | `0` `V:b` | S | same |
| `colon2` | `0` `NV` | S | same |
| `colon3` | `0` `NV` | S | same |
| `trail` | `1` `can't read "bnq37n::": no such variable` | S | same |
| `lead` | `0` `V,V` | S | same |
| `chain` | `0` `NVNW` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-038 — A lone `$`, `$$`, `$(`, and an unterminated index

- **Question.** Is a `$` that is not followed by a name left as a literal, is `$(k)` an element of the empty-named array, and is `$name(` without `)` an error?
- **Why the toolchain needs it.** `scan_var_ref` (`rust/tcl-lexer/src/word_parts.rs`); `VarSyntax::has_expr_sugar` is false for the Tcl family, so `$(…)` is an empty-name array read, not an expression.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26: `$name(` gives `missing )` in TMM, tmsh, iApp and iCall (lexical byte scan).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-038
    set bnc_obs {
        def    {set bnq38 V; list}
        lone   {set r "a$"}
        double {set r "$$bnq38"}
        space  {set r "$ x"}
        paren  {catch {eval {set r "$bnq38(k"}} m; set m}
        noname {set (k) E; set r $(k); unset {}; set r}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `lone` | `0` `a$` | S | same |
| `double` | `0` `$V` | S | same |
| `space` | `0` `$ x` | S | same |
| `paren` | `0` `missing )` | S | same |
| `noname` | `0` `E` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-039 — Non-ASCII bytes after an unbraced `$name`

- **Question.** Does the unbraced scanner stop before a byte above 0x7f?
- **Why the toolchain needs it.** `VarSyntax::name_allows_high_bytes` is false for every Tcl-family grammar including `GRAMMAR_F5_TCL`, so the lexer stops; the tmsh-hosted contexts were measured consuming Latin-1 letters, which no grammar value models.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Build 0.0.26: TMM stops before `e9` (result is the short variable plus the byte); tmsh, iApp and iCall consume `e9` and read the long name (`evidence/tcl/bigip-r2286m-lexical-format-unbraced.md`, `bigip-r2286p-lexical-format-unbraced.md`).
- **Note.** `53484f5254e9` is `SHORT` followed by the byte; `46554c4c` is `FULL`.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-039
    set bnc_obs {
        def  {set bnq39 SHORT; set n bnq39[binary format H* e9]; set $n FULL; list}
        unbr {set n bnq39[binary format H* e9]; set c [catch {subst "\$$n"} m]; binary scan $m H* h; list $c $h}
        brcd {set n bnq39[binary format H* e9]; set c [catch {subst "\${$n}"} m]; binary scan $m H* h; list $c $h}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `unbr` | `0`, `0 53484f5254e9` if the scanner stops (as `VarSyntax::Tcl` models), `0 46554c4c` if it consumes the byte | to be confirmed with tclsh8.4 (8.4 uses the C library `isalnum`, so the answer may depend on the locale) | to be recorded |
| `brcd` | `0` `0 46554c4c` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

### 4.6 Procedure naming

#### BIGIP-NAME-040 — `proc` with a qualified name whose namespace does not exist

- **Question.** Does `proc` refuse to create a command in a missing namespace (it never creates namespaces)?
- **Why the toolchain needs it.** Selects `NativeNameProtocol::command_publication_slot` (`rust/tcl-syntax/src/naming/native.rs`); definition sites in a missing namespace are diagnosed by the analyser.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Not measured on BIG-IP.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-040
    set bnc_obs {
        missing {proc ::bnq40none::p {} {return X}}
        relmiss {proc bnq40none::p {} {return X}}
        created {namespace exists ::bnq40none}
        ok      {namespace eval ::bnq40 {}; proc ::bnq40::p {} {return OK}; ::bnq40::p}
        relok   {proc bnq40::q {} {return REL}; ::bnq40::q}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `missing` | `1` `can't create procedure "::bnq40none::p": unknown namespace` | S | same |
| `relmiss` | `1` `can't create procedure "bnq40none::p": unknown namespace` | S | same |
| `created` | `0` `0` | S | same |
| `ok` | `0` `OK` | S | same |
| `relok` | `0` `REL` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-041 — `proc` with a relative name inside `namespace eval`

- **Question.** Is a relative `proc` name created in the namespace the `namespace eval` entered?
- **Why the toolchain needs it.** This is the definition-site rule every consumer uses to qualify a proc (`NativeNameProtocol::command_publication_input`). It is already known to differ in the tmsh-hosted contexts, which `BigIpExecutionContext` does not yet express.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1: TMM (RULE_INIT, proc, HTTP_REQUEST) creates `::ns::p` as Tcl 8.4 does. iApp implementation, iCall and tmsh cli script create `::p` in the global namespace instead, and `::ns::p` does not exist (`namespace-definition-owner`: `evidence/tcl/bigip-log-nh20261012nsowner1-*.md`, `evidence/tcl/bigip-nh20261012nsowner1-tmsh-cli-script-0.md`). Rerun to confirm on other builds and to record the qualified-relative and absolute forms.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-041
    set bnc_obs {
        def    {namespace eval ::bnq41 {proc bnq41p {} {return [namespace current]}}; list}
        where  {list [namespace which -command ::bnq41::bnq41p] [namespace which -command ::bnq41p]}
        call   {list [catch {::bnq41::bnq41p} a] $a [catch {::bnq41p} b] $b}
        qualin {namespace eval ::bnq41 {namespace eval bnq41sub {}; proc bnq41sub::bnq41q {} {return Q}}; list [namespace which -command ::bnq41::bnq41sub::bnq41q] [namespace which -command ::bnq41sub::bnq41q] [namespace which -command ::bnq41q]}
        absin  {namespace eval ::bnq41 {proc ::bnq41r {} {return [namespace current]}}; ::bnq41r}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `where` | `0` `::bnq41::bnq41p {}` | T84 (`computed-qualified`, `finite-factory` on tclsh 8.4.13) | same |
| `call` | `0` `0 ::bnq41 1 {invalid command name "::bnq41p"}` | S | same |
| `qualin` | `0` `::bnq41::bnq41sub::bnq41q {} {}` | S | same |
| `absin` | `0` `::` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-042 — `info procs`, `info args`, `info body`, `info default` with qualified names

- **Question.** Do the proc introspection commands accept qualified names and patterns, and which spelling does `info procs` return?
- **Why the toolchain needs it.** Drives `original_script_command_inventory_support` (`rust/tcl-syntax/src/naming/script_command_inventory.rs`).
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1: `info procs ::ns::name` and `info body ::ns::name` work in TMM; a missing proc gives `"…" isn't a procedure` in all contexts (`namespace-definition-owner`).

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-042
    set bnc_obs {
        def     {namespace eval ::bnq42 {}; proc ::bnq42::p {a {b 2} args} {return body}; proc ::bnq42g {x} {return g}; list}
        procs   {lsort [info procs ::bnq42::*]}
        bare    {list [info procs bnq42g] [info procs ::bnq42g]}
        inns    {namespace eval ::bnq42 {info procs p}}
        args    {info args ::bnq42::p}
        body    {info body ::bnq42::p}
        deflt   {list [info default ::bnq42::p b d] $d}
        notproc {info body ::bnq42::none}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `def` | `0`, empty | S | same |
| `procs` | `0` `::bnq42::p` | to be confirmed with tclsh8.4 | same |
| `bare` | `0` `bnq42g ::bnq42g` | to be confirmed with tclsh8.4 | same |
| `inns` | `0` `p` | S | same |
| `args` | `0` `a b args` | S | same |
| `body` | `0` `return body` | S | same |
| `deflt` | `0` `1 2` | S | same |
| `notproc` | `1` `"::bnq42::none" isn't a procedure` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-043 — `proc` defined inside a proc; redefinition

- **Question.** Is a relative `proc` name defined while a proc body runs created in that proc's namespace, and does redefinition replace the command?
- **Why the toolchain needs it.** Same publication rule as BIGIP-NAME-041, reached from a call frame instead of `namespace eval`.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** 21.1.0.1 TMM, tmsh, iApp: a proc defined inside a proc is callable and redefinition wins (`scripts/dev/bigip-probes/results/11-proc-semantics.txt`). The namespace it lands in is not recorded.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-043
    set bnc_obs {
        inner    {proc ::bnq43o {} {proc ::bnq43i {} {return IN}; ::bnq43i}; ::bnq43o}
        innerrel {namespace eval ::bnq43 {}; proc ::bnq43::o {} {proc bnq43d {} {return [namespace current]}; bnq43d}; list [::bnq43::o] [namespace which -command ::bnq43::bnq43d] [namespace which -command ::bnq43d]}
        redef    {proc ::bnq43r {} {return one}; proc ::bnq43r {} {return two}; ::bnq43r}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `inner` | `0` `IN` | S | same |
| `innerrel` | `0` `::bnq43 ::bnq43::bnq43d {}` | S | same |
| `redef` | `0` `two` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-044 — Formal parameter names that are qualified or look like array elements

- **Question.** Which formal parameter names does `proc` refuse, and with which message?
- **Why the toolchain needs it.** `FormalParameterError::message_for_definition_bytes` (`rust/tcl-syntax/src/formal_params.rs`) has an 8.4-only wording that includes the procedure name; `vendor_source_formal_extents` (`rust/tcl-syntax/src/naming/vendor_source.rs`) reads F5 formal lists with the Tcl grammar.
- **Environments.** TMM `RULE_INIT` (complete body below); tmsh cli script, iApp implementation and iCall with the §2 wrappers and the same `bnc_id` / `bnc_obs` lines, global mode.
- **Already on record.** Not measured on BIG-IP.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-044
    set bnc_obs {
        qualf  {proc ::bnq44a {x::y} {}}
        arrf   {proc ::bnq44b {x(y)} {}}
        emptyf {proc ::bnq44c {{}} {}}
        colonf {proc ::bnq44d {x:y} {return ${x:y}}; ::bnq44d V}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `qualf` | `1` `procedure "bnq44a" has formal parameter "x::y" that is not a simple name` | N (8.4 wording; whether the name is reported with or without qualifiers is to be confirmed with tclsh8.4) | `formal parameter "x::y" is not a simple name` |
| `arrf` | `1` `procedure "bnq44b" has formal parameter "x(y)" that is an array element` | N (same caveat) | `formal parameter "x(y)" is an array element` |
| `emptyf` | `1` `procedure "bnq44c" has argument with no name` | N (same caveat) | `argument with no name` |
| `colonf` | `0` `V` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

### 4.7 iRules in TMM

These checks have no tclsh form. Where a "Tcl 8.4" column is given it states what ordinary 8.4 naming rules would produce if the F5 object were an ordinary Tcl namespace, command or variable; a `differs` verdict there is information, not a fault. Checks that need traffic say so.

#### BIGIP-NAME-045 — Where a top-level iRule `proc` lives

- **Question.** Is a `proc` written at the top level of an iRule an ordinary Tcl command, and in which namespace? Can it be invoked without `call`, and does `call` see procs defined at run time?
- **Why the toolchain needs it.** `docs/design/contracts/namespace-model.md` §3 states that a proc in rule `my_irule` becomes `::my_irule::my_helper`; the `call` spec (`rust/tcl-registry/src/commands/irules/call.rs`) and the workspace proc index are documented against that model. Earlier measurements point the other way.
- **Environments.** TMM `RULE_INIT`.
- **Already on record.** 21.1.0.1 build 0.0.26: `namespace current` inside a rule proc is `::`; `namespace which -command` is empty for every `call` target; `call` does not find a proc defined at run time (`proc … not found`) (`docs/design/analysis/name-resolution-proofs/bigip-procedure-owner-route.md`, `docs/design/f5/bigip-irule-parser-measurements.md` §4b).

Rule `/Common/bnc_r45` (complete rule body shown inside its `ltm rule` stanza):

```tcl
ltm rule /Common/bnc_r45 {
proc bnq45who {} { return [list [info level] [llength [info commands bnq45who]]] }
when RULE_INIT {
    set bnc_id BIGIP-NAME-045
    set bnc_obs {
        viacall {call bnq45who}
        which   {list [namespace which -command bnq45who] [namespace which -command ::bnq45who] [namespace which -command ::bnc_r45::bnq45who]}
        listed  {list [info commands bnq45who] [info procs bnq45who] [info commands ::bnc_r45::*] [info procs ::bnc_r45::*]}
        nsex    {list [namespace exists ::bnc_r45] [namespace exists ::Common] [namespace exists ::Common::bnc_r45]}
        direct  {set h bnq45who; $h}
        qualdir {set h ::bnc_r45::bnq45who; $h}
        dyn     {set d {proc ::bnq45dyn {} {return DYN}}; uplevel #0 $d; set r [list [catch {call bnq45dyn} m] $m [catch {set h bnq45dyn; $h} m2] $m2]; rename ::bnq45dyn {}; set r}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
}
```

**Expected.** No Tcl 8.4 equivalent. If rule procs were ordinary procs in a namespace named after the rule (the `namespace-model.md` reading), `which` would end with `::bnc_r45::bnq45who`, `nsex` would start with `1`, and `qualdir` would succeed. The earlier measurements predict `which` = `{} {} {}`, and `dyn` = `1 {proc bnq45dyn not found …} 0 DYN`. Record all seven lines.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-046 — `call` name forms across iRules

- **Question.** Which target spellings does `call` accept: bare, `rule::proc`, `/partition/rule::proc`, a Tcl-rooted `::rule::proc`, a computed target?
- **Why the toolchain needs it.** The `call` spec gives its first argument `ArgRole::Name`; the resolver must know which spellings name a rule proc and which are refused (`rust/tcl-registry/src/commands/irules/call.rs`, `rust/tcl-bigip` object-name policies in `docs/design/contracts/bigip-object-name-resolution.md`).
- **Environments.** TMM `RULE_INIT` (two rules; load the provider first).
- **Already on record.** 21.1.0.1 (build 0.0.26 for the r2286 runs): bare name selects the calling rule's proc; `rule::proc` resolves at the caller's partition root; `/partition/[folder/]rule::proc` reaches the named rule; `::rule::proc` fails with `… - invalid namespace`; a rule in a folder is not found by leaf name; computed targets behave like literal ones (`docs/design/analysis/name-resolution-proofs/bigip-procedure-owner-route.md`; `evidence/tcl/bigip-log-nh20261012native1-*.md`, `…native2-*.md`, `…pat1-*.md`).

Provider and caller:

```tcl
ltm rule /Common/bnc_p46 {
proc bnq46id {args} { return [list PROVIDER $args] }
}
ltm rule /Common/bnc_r46 {
proc bnq46id {args} { return [list CALLER $args] }
when RULE_INIT {
    set bnc_id BIGIP-NAME-046
    set bnc_obs {
        bare     {call bnq46id x}
        rel      {call bnc_p46::bnq46id x}
        abs      {call /Common/bnc_p46::bnq46id x}
        tclabs   {call ::bnc_p46::bnq46id x}
        selfrel  {call bnc_r46::bnq46id x}
        computed {set t /Common/bnc_p46::bnq46id; call $t {a b}}
        missing  {call bnc_p46::bnq46none}
        norule   {call bnc_none46::bnq46id}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
}
```

**Expected.** No Tcl 8.4 equivalent. From the 21.1.0.1 measurements: `bare` `0|CALLER x`; `rel`, `abs` `0|PROVIDER x`; `tclabs` `1|proc ::bnc_p46::bnq46id - invalid namespace …`; `selfrel` `0|CALLER x`; `computed` `0|PROVIDER {{a b}}`; `missing` and `norule` `1|proc … not found …`. Delete the caller before the provider: a literal cross-rule `call` creates a configuration dependency.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-047 — Inside a called iRule proc

- **Question.** What do `namespace current`, `info level`, `namespace which` and `info procs` report inside a proc reached through `call`, and which frame does `upvar 1` reach?
- **Why the toolchain needs it.** Frame and namespace of a called proc decide how `upvar`/`uplevel` names in library rules are resolved (`MeasuredBigIpNameScope` in `rust/tcl-syntax/src/naming/execution_policy.rs` covers only `HTTP_REQUEST` bodies).
- **Environments.** TMM `RULE_INIT` and `HTTP_REQUEST` (two rules; the caller needs a lab virtual server for the second line).
- **Already on record.** 21.1.0.1 build 0.0.26: `namespace current` is `::` in the provider proc for local and cross-rule calls; `upvar 1` changes the caller's event-local variable (`bigip-procedure-owner-route.md`). `info level` values recorded in `evidence/tcl/bigip-log-nh20261012native2-*.md` (`provider-local-upvar-write`).

The proc body keeps `namespace` out of the rule source by holding the query as data:

```tcl
ltm rule /Common/bnc_p47 {
proc bnq47in {name} {
    upvar 1 $name v
    set v CHANGED
    set q {list [namespace current] [info level] [namespace which -command bnq47in] [info procs bnq47*] [uplevel 1 {namespace current}] [uplevel 1 {info level}]}
    return [eval $q]
}
}
ltm rule /Common/bnc_r47 {
when RULE_INIT {
    set bnq47x BEFORE
    set bnc_rc [catch {call bnc_p47::bnq47in bnq47x} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
    log local0. "BNC|BIGIP-NAME-047|tmm[TMM::cmp_unit]|init|$bnc_rc|$bnc_r|$bnq47x"
}
when HTTP_REQUEST {
    set bnq47y BEFORE
    set bnc_rc [catch {call bnc_p47::bnq47in bnq47y} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
    log local0. "BNC|BIGIP-NAME-047|tmm[TMM::cmp_unit]|event|$bnc_rc|$bnc_r|$bnq47y"
    HTTP::respond 200 content "BNC [TMM::cmp_unit]" Connection close
}
}
```

**Expected.** No Tcl 8.4 equivalent. An ordinary 8.4 proc called from the global level would print `:: 1 ::bnq47in bnq47in :: 0` and leave the caller variable `CHANGED`. Record both lines for every TMM.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-048 — Is `static` a Tcl namespace?

- **Question.** Is `static::x` a variable in a real namespace `::static`: does `namespace exists ::static` hold, do `::static::x` and `static::x` name the same cell, and is it visible to `info vars` and `namespace which`?
- **Why the toolchain needs it.** The simulator stores `static::` cells through namespace machinery (`AuthoredTmmStaticPolicy` in `rust/tcl-runtime-api/src/authored_tmm.rs`, `rust/tcl-vm/src/interp/authored_tmm_static.rs`), and `ObservedBigIpNamePolicy` refuses every qualified name including `static::x`. Whether TMM treats `static` as an ordinary namespace is not measured.
- **Environments.** TMM `RULE_INIT`.
- **Already on record.** Build 0.0.26 TMM HTTP_REQUEST: `set static::x` works from evaluated text; `global static::x` gives a local `x`; a variable trace reports the name as written (`static::…`) (`evidence/tcl/bigip-r2286c-dynamic-static-global-link.md`, `…-trace-static.md`). `namespace exists ::static`, the `::static::` spelling and `info vars` are not measured.

```tcl
when RULE_INIT {
    set static::bnq48 INIT
    set bnc_id BIGIP-NAME-048
    set bnc_obs {
        nsex  {list [namespace exists ::static] [namespace exists static] [expr {[lsearch -exact [namespace children ::] ::static] >= 0}]}
        exist {list [info exists static::bnq48] [info exists ::static::bnq48] [info exists :::static::bnq48]}
        read  {list [catch {set ::static::bnq48} a] $a}
        vars  {list [info vars ::static::bnq48*] [info vars static::bnq48*] [namespace which -variable static::bnq48]}
        write {set ::static::bnq48w ABS; list [info exists static::bnq48w] [set static::bnq48w]}
        link  {proc ::bnq48p {} {global static::bnq48; return $bnq48}; ::bnq48p}
        other {namespace eval ::bnq48ns {list [info exists static::bnq48] [catch {set static::bnq48} m] $m}}
        clean {unset static::bnq48; catch {unset static::bnq48w}; info exists static::bnq48}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `nsex` | `0` `1 1 1` | S if `::static` is an ordinary namespace | same |
| `exist` | `0` `1 1 1` | S, same condition | same |
| `read` | `0` `0 INIT` | S, same condition | same |
| `vars` | `0` `::static::bnq48 ::static::bnq48 ::static::bnq48` | to be confirmed with tclsh8.4, same condition | same |
| `write` | `0` `1 ABS` | S, same condition | same |
| `link` | `0` `INIT` | S (measured on build 0.0.26) | same |
| `other` | `0` `1 0 INIT` | S (relative-qualified names fall back to `::`) | 9.0: variables have no global fallback |
| `clean` | `0` `0` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-049 — `static::` across events, procs and TMMs

- **Question.** Is a `static::` variable set in `RULE_INIT` readable in a traffic event and inside a proc on every TMM, under each spelling?
- **Why the toolchain needs it.** CMP behaviour of `static::` is modelled by `AuthoredTmmStaticPolicy::RuleInitPublication`; the spellings accepted by `$` substitution decide what the analyser links to the `RULE_INIT` definition.
- **Environments.** TMM `RULE_INIT` + `HTTP_REQUEST` (lab virtual server; send enough requests to reach every TMM).
- **Already on record.** Build 0.0.26: a `RULE_INIT` seed is visible on all four TMMs; a write, unset or recreate in an event stays on the TMM that executed it (`docs/design/analysis/name-resolution-proofs/bigip-static-state-tmm-locality.md`, `bigip-variable-static-event-currency.md`). The `::static::` and `${static::x}` spellings are not measured.

```tcl
when RULE_INIT { set static::bnq49 INIT }
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-049
    set bnc_obs {
        event  {set static::bnq49}
        abs    {set ::static::bnq49}
        inproc {proc ::bnq49p {} {return $static::bnq49}; set r [::bnq49p]; rename ::bnq49p {}; set r}
        dollar {set r "$static::bnq49,${static::bnq49},$::static::bnq49"}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
}
```

**Expected.** No Tcl 8.4 equivalent for the per-TMM part. Under 8.4 naming rules with `::static` an ordinary namespace: `event`, `abs`, `inproc` `0|INIT`; `dollar` `0|INIT,INIT,INIT`, on every TMM. Remove the variable afterwards by deleting the rule and reloading one whose `RULE_INIT` runs `unset static::bnq49`.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-050 — Bare variables in `RULE_INIT`

- **Question.** Does the `RULE_INIT` body run at the global level, and is a bare variable set there a global variable?
- **Why the toolchain needs it.** Decides whether names in `RULE_INIT` are frame-local or global for the analyser (`source_variable_name_is_frame_local`, `rust/tcl-syntax/src/naming.rs`) and whether `MeasuredBigIpNameScope` needs a `RULE_INIT` member.
- **Environments.** TMM `RULE_INIT`.
- **Already on record.** 21.1.0.1: `namespace current` is `::` in `RULE_INIT` (`evidence/tcl/bigip-r2286l-s-namespace.md`). Frame level and global visibility not recorded as a result.

```tcl
when RULE_INIT {
    set bnq50bare BARE
    set bnc_id BIGIP-NAME-050
    set bnc_obs {
        frame  {list [info level] [namespace current]}
        local  {list [info exists bnq50bare] [info locals bnq50*] [info vars bnq50*] [info globals bnq50*]}
        global {uplevel #0 {list [info exists bnq50bare] [info exists ::bnq50bare] [info level]}}
        upl    {uplevel #0 {set bnq50g G}; list [info exists bnq50g] [info exists ::bnq50g]}
        clean  {catch {unset bnq50bare}; catch {unset ::bnq50g}; list}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `frame` | `0` `0 ::` | S (script evaluated at the global level) | same |
| `local` | `0` `1 {} bnq50bare bnq50bare` | S, same condition | same |
| `global` | `0` `1 1 0` | S, same condition | same |
| `upl` | `0` `1 1` | S, same condition | same |
| `clean` | `0`, empty | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-051 — `namespace eval` plus `set` from an event frame

- **Question.** When evaluated from an event body (not through `uplevel #0`), does an unqualified `set` of a new variable inside `namespace eval` create a namespace variable?
- **Why the toolchain needs it.** Tcl 8.4 creates the variable in the namespace (`variable_resolution_vectors.txt`, creation rows). An earlier TMM measurement did not find it there; the cause is not identified, and the naming layer currently has no TMM rule for it.
- **Environments.** TMM `RULE_INIT`; repeat in `HTTP_REQUEST` with the §2.1 traffic wrapper.
- **Already on record.** 21.1.0.1 TMM (RULE_INIT, proc, HTTP_REQUEST): after `namespace eval ::nh {set prefix …}`, `$::nh::prefix` was `no such variable`, while on-box tclsh 8.4.13 and 8.5.13 created it (`namespace-captured-prefix`: `evidence/tcl/bigip-log-nh20261012exact2-tmm-RULE_INIT-10.md` and `evidence/tcl/bigip-nh20261012stock2-stock-8.4-namespace-captured-prefix.md`). A pre-existing global of the same name would explain it under 8.4 rules; the `confound` line tests that.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-051
    set bnc_obs {
        confound {list [info exists ::bnq51v] [info exists bnq51v] [info exists ::prefix] [info exists prefix]}
        create   {namespace eval ::bnq51 {set bnq51v 1}; list [info exists ::bnq51::bnq51v] [info exists ::bnq51v] [info exists bnq51v]}
        declared {namespace eval ::bnq51 {variable bnq51w 1}; list [info exists ::bnq51::bnq51w] [info exists bnq51w]}
        reread   {namespace eval ::bnq51 {list [info exists bnq51v] [catch {set bnq51v} m] $m}}
        clean    {catch {unset bnq51v}; catch {unset ::bnq51v}; list}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `confound` | `0` `0 0 0 0` | S (clean tclsh at the global level) | same |
| `create` | `0` `1 0 0` | V-var; T84 (`namespace-captured-prefix` on tclsh 8.4.13) | same |
| `declared` | `0` `1 0` | V-var | same |
| `reread` | `0` `1 0 1` | S | same |
| `clean` | `0`, empty | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-052 — Event-local variables against `::` globals

- **Question.** In a traffic event, is a bare variable a different cell from the global of the same name, and at which level does the event body run?
- **Why the toolchain needs it.** `AuthoredObservedFrameStorageContext` (`rust/tcl-runtime-api/src/authored_tmm.rs`) models event-frame storage for the measured scope only; the analyser must not link `$x` and `$::x` in an event if they are different cells.
- **Environments.** TMM `HTTP_REQUEST` (lab virtual server).
- **Already on record.** Build 0.0.26: a variable set in `CLIENT_ACCEPTED` is readable in later events of the connection; ordinary globals are per TMM and demote the virtual server from CMP (`bigip-procedure-owner-route.md`, `bigip-static-state-tmm-locality.md`). The local/global identity question is not recorded as a result.
- **Note.** iRules documentation describes event variables as connection-local, which predicts `same` = `1 0`, `glob` = `LOCAL GLOBAL`, `up0` = `0`. That outcome is a `differs` verdict against the 8.4 column and is the expected one.

```tcl
when HTTP_REQUEST {
    set bnq52 LOCAL
    set bnc_id BIGIP-NAME-052
    set bnc_obs {
        frame {list [info level] [namespace current]}
        same  {list [info exists bnq52] [info exists ::bnq52]}
        up0   {uplevel #0 {info exists bnq52}}
        vars  {list [info locals bnq52*] [info vars bnq52*] [info globals bnq52*]}
        glob  {set ::bnq52 GLOBAL; set r [list $bnq52 $::bnq52]; catch {unset ::bnq52}; set r}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `frame` | `0` `0 ::` | S (tclsh global level) | same |
| `same` | `0` `1 1` | S (at the global level a bare name is the global) | same |
| `up0` | `0` `1` | S | same |
| `vars` | `0` `{} bnq52 bnq52` | S | same |
| `glob` | `0` `GLOBAL GLOBAL` | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-053 — Are protocol commands real namespaced commands?

- **Question.** Do `HTTP::header`, `IP::client_addr`, `TMM::cmp_unit` live in Tcl namespaces `::HTTP`, `::IP`, `::TMM`, and are they visible to `namespace which`, `info commands` and `namespace origin`?
- **Why the toolchain needs it.** `docs/design/contracts/namespace-model.md` §2 and `ProtocolNamespaceSpec` (`rust/tcl-registry/src/profiles.rs`) treat `HTTP::` and friends as command prefixes, not Tcl namespaces. A user namespace that shadows them, and `::HTTP::method` working, suggest real namespaces.
- **Environments.** TMM `RULE_INIT`.
- **Already on record.** 21.1.0.1 TMM: a user proc `::ns::HTTP::method` shadows `HTTP::method` when called from `::ns`; `::HTTP::method` is found (`Invalid context` in `RULE_INIT`, `GET` in `HTTP_REQUEST`) (`protocol-relative-user-shadow`, `protocol-absolute-native`: `evidence/tcl/bigip-log-nh20261012broad1-tmm-RULE_INIT-14.md`, `…-15.md`, `…broad2-tmm-traffic-event-*.md`). Namespace existence and inventories not measured.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-053
    set bnc_obs {
        nsex   {list [namespace exists ::HTTP] [namespace exists ::IP] [namespace exists ::TCP] [namespace exists ::TMM]}
        which  {list [namespace which -command HTTP::header] [namespace which -command ::HTTP::header] [namespace which -command IP::client_addr] [namespace which -command TMM::cmp_unit]}
        count  {list [llength [info commands]] [llength [info commands HTTP::*]] [llength [info commands ::HTTP::*]] [llength [info procs ::HTTP::*]]}
        names  {lrange [lsort [info commands ::HTTP::*]] 0 7}
        origin {list [catch {namespace origin HTTP::header} m] $m}
        fromns {namespace eval ::bnq53 {namespace which -command HTTP::header}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `nsex` | `0` `1 1 1 1` | S if the commands were registered with qualified names (a C command created as `HTTP::header` lives in namespace `::HTTP`) | same |
| `which` | `0` `::HTTP::header ::HTTP::header ::IP::client_addr ::TMM::cmp_unit` | S, same condition | same |
| `count` | `0`, to be recorded | n/a | n/a |
| `names` | `0`, to be recorded | n/a | n/a |
| `origin` | `0` `0 ::HTTP::header` | S, same condition | same |
| `fromns` | `0` `::HTTP::header` | S, same condition | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-054 — Protocol commands from other namespaces and procs; user shadowing

- **Question.** Is `HTTP::method` reachable from a user namespace and from a proc, does a user proc `::ns::HTTP::method` shadow it from inside `::ns`, and does the absolute name bypass the shadow?
- **Why the toolchain needs it.** If protocol commands follow ordinary relative-qualified lookup, the resolver must apply `command_resolution_candidates` to them too, and a same-named user namespace changes the target.
- **Environments.** TMM `HTTP_REQUEST` (lab virtual server, send a `GET`).
- **Already on record.** 21.1.0.1 TMM HTTP_REQUEST: shadow returns the user proc; `::HTTP::method` and a computed `::HTTP::method` return `GET` (`evidence/tcl/bigip-log-nh20261012broad2-tmm-traffic-event-*.md`).

```tcl
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-054
    set bnc_obs {
        plain  {HTTP::method}
        abs    {::HTTP::method}
        inns   {namespace eval ::bnq54 {HTTP::method}}
        inproc {proc ::bnq54::bnq54p {} {HTTP::method}; ::bnq54::bnq54p}
        shadow {namespace eval ::bnq54::HTTP {}; proc ::bnq54::HTTP::method {} {return SHADOW}; namespace eval ::bnq54 {HTTP::method}}
        absns  {namespace eval ::bnq54 {::HTTP::method}}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `plain` | `0` `GET` | S (ordinary lookup of a command in `::HTTP`) | same |
| `abs` | `0` `GET` | S | same |
| `inns` | `0` `GET` | V-cmd (relative-qualified, global fallback) | same |
| `inproc` | `0` `GET` | V-cmd | same |
| `shadow` | `0` `SHADOW` | V-cmd | same |
| `absns` | `0` `GET` | V-cmd | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-055 — `rename` of a protocol command (lab only)

- **Question.** Can a built-in iRules command be renamed, and does the old name stop resolving?
- **Why the toolchain needs it.** If built-ins can be renamed at run time, a `rename` in rule text invalidates the registry's static binding for that name on that TMM; the analyser needs to know whether to honour or refuse such renames.
- **Environments.** TMM `RULE_INIT`, disposable lab unit only.
- **Already on record.** 21.1.0.1: `rename` works for procs when reached through evaluated text; a literal `rename` in rule source is rejected at load (`docs/design/f5/bigip-irule-parser-measurements.md` §4b, §5). Renaming a built-in is not measured.
- **Note.** **Risk.** The script renames `IP::tos` and renames it straight back, on every TMM. If the last element of the result is empty the command was not restored on that TMM: restart TMM (`bigstart restart tmm`) before any other test. Do not run on a unit that carries traffic. The temporary name deliberately does not start with `bnq`, so the harness wipe cannot delete it.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-055
    set bnc_obs {
        ren {set c [catch {rename ::IP::tos ::bnc_keep55} m]; set w [list [namespace which -command ::IP::tos] [namespace which -command ::bnc_keep55]]; set b [catch {rename ::bnc_keep55 ::IP::tos} m2]; list $c $m $w $b $m2 [namespace which -command ::IP::tos]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `ren` | `0` `0 {} {{} ::bnc_keep55} 0 {} ::IP::tos` | S if `IP::tos` is an ordinary command in `::IP` | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-056 — Subcommand abbreviation in iRules commands

- **Question.** Do iRules commands accept unique prefixes of their subcommands, and can a command name itself be abbreviated?
- **Why the toolchain needs it.** The registry stores subcommands by full name (`rust/tcl-registry/src/commands/irules/`); if TMM accepts prefixes the analyser must not flag them, and if it does not, `OptionSpec::min_abbrev` must stay unset for them.
- **Environments.** TMM `HTTP_REQUEST` (lab virtual server; send a request with a `Host` header).
- **Already on record.** Not measured on BIG-IP.

```tcl
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-056
    set bnc_obs {
        full    {HTTP::header value Host}
        abbr    {HTTP::header val Host}
        names   {llength [HTTP::header names]}
        nam     {llength [HTTP::header nam]}
        cmdabbr {set h HTTP::head; $h value Host}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `full` | `0`, the request's Host value | n/a | n/a |
| `abbr` | `0`, to be recorded | n/a | n/a |
| `names` | `0`, header count | n/a | n/a |
| `nam` | `0`, to be recorded | n/a | n/a |
| `cmdabbr` | `1` `invalid command name "HTTP::head"` | S (command names are never abbreviated outside interactive `unknown`) | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-057 — Load-time resolution of literal command names

- **Question.** Does the rule loader reject a literal unknown command, a literal `namespace`, and a direct call of a top-level proc, while accepting the same name when it is computed?
- **Why the toolchain needs it.** `Family::closed_load_time_resolution` and `Family::declaration_only_top_level` (`rust/tcl-dialect/src/model/family.rs`) are true only for `F5Irules`; they turn unknown-command findings into errors for iRules.
- **Environments.** TMM rule load (four separate rules; record ACCEPT or REJECT and the loader message for each).
- **Already on record.** 21.1.0.1 build 0.0.26: (a) rejected `undefined procedure`; (b) rejected `command is disabled: "namespace"`; (c) rejected `undefined procedure: <proc>`; (d) accepted, run-time `invalid command name` (`docs/design/f5/bigip-irule-parser-measurements.md` §4a–§4c; `docs/design/analysis/name-resolution-proofs/bigip-literal-head-loader-boundary.md`).

(a) literal unknown command inside `catch`:

```tcl
when RULE_INIT { catch { bnq57nosuch } }
```

(b) literal `namespace`:

```tcl
when RULE_INIT { catch { namespace current } }
```

(c) top-level proc invoked without `call`:

```tcl
proc bnq57p {} { return 1 }
when RULE_INIT { set bnq57x [bnq57p] }
```

(d) the same unknown name, computed:

```tcl
when RULE_INIT {
    set bnq57c bnq57nosuch
    set bnc_rc [catch {eval $bnq57c} bnc_r]
    log local0. "BNC|BIGIP-NAME-057|tmm[TMM::cmp_unit]|dyn|$bnc_rc|$bnc_r"
}
```

**Expected.** Tcl 8.4 has no load phase: all four scripts are accepted and fail or succeed when run ((a) and (b) are caught, (c) returns `1`, (d) logs `1|invalid command name "bnq57nosuch"`). Expected on BIG-IP from the measurements above: REJECT, REJECT, REJECT, ACCEPT with `dyn|1|invalid command name "bnq57nosuch"`.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-058 — `table` keys are not Tcl names

- **Question.** Are `table` keys and subtable names opaque strings that create no Tcl variable or namespace, whatever characters they contain?
- **Why the toolchain needs it.** Keeps the variable-name layer out of `table` operands: the registry `table` spec (`rust/tcl-registry/src/commands/irules/table.rs`) must not give its key an `ArgRole` that the naming layer parses as `name(index)` or as a qualified name.
- **Environments.** TMM `HTTP_REQUEST` (lab virtual server; `table` is not valid in `RULE_INIT`).
- **Already on record.** Not measured.

```tcl
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-058
    set bnc_obs {
        set    {table set -subtable bnq58 {a::b(c) d} V 60; table lookup -subtable bnq58 {a::b(c) d}}
        keys   {table keys -subtable bnq58}
        notvar {list [info exists {a::b(c) d}] [info exists ::bnq58] [namespace exists ::bnq58]}
        del    {table delete -subtable bnq58 -all; table keys -subtable bnq58 -count}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `set` | `0` `V` | n/a | n/a |
| `keys` | `0` `{a::b(c) d}` | n/a | n/a |
| `notvar` | `0` `0 0 0` | n/a | n/a |
| `del` | `0` `0` | n/a | n/a |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-059 — Inventory of namespaces and commands in TMM

- **Question.** Which child namespaces of `::` exist in the TMM interpreter, and how many commands and procs does it hold?
- **Why the toolchain needs it.** Cross-checks the registry's protocol namespace list (`ProtocolNamespaceSpec`, `rust/tcl-registry/src/profiles.rs`) against the interpreter, from the interpreter's side.
- **Environments.** TMM `RULE_INIT`.
- **Already on record.** 21.1.0.1: 152 commands from `info commands` in TMM (`BigIpExecutionContext::TmmIRule` documentation). The namespace list has not been enumerated (`docs/design/f5/bigip-irule-parser-measurements.md` §12).
- **Note.** If `ns4` is long enough to be cut by syslog, split it further.

```tcl
when RULE_INIT {
    set bnc_id BIGIP-NAME-059
    set bnc_obs {
        total {list [llength [info commands]] [llength [info procs]] [llength [namespace children ::]]}
        ns0   {lrange [lsort [namespace children ::]] 0 11}
        ns1   {lrange [lsort [namespace children ::]] 12 23}
        ns2   {lrange [lsort [namespace children ::]] 24 35}
        ns3   {lrange [lsort [namespace children ::]] 36 47}
        ns4   {lrange [lsort [namespace children ::]] 48 end}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `total` | `0`, to be recorded | n/a | n/a |
| `ns0` | `0`, to be recorded (stock tclsh 8.4 has only `::tcl`) | n/a | n/a |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-068 — `HTTP::header NAME VALUE` with no subcommand, before and after `HTTP::respond`

- **Question.** Does TMM accept `HTTP::header NAME VALUE` (first word a header name, not a subcommand), and as what: a read of `NAME`, an insert, or an error? Is it refused when the rule loads or when the event runs? Does it raise after `HTTP::respond` in the same event?
- **Why the toolchain needs it.** `rust/tcl-compiler/src/irules_checks.rs` test `irule1201_http_after_respond_fires` uses `HTTP::respond 200 content ok; HTTP::header Cache-Control no-cache` and expects IRULE1201. The registry (`rust/tcl-registry/src/commands/irules/`) describes no `HTTP::header` form whose first word is a header name followed by a value, so the command selects no subcommand and the check stays silent. The answer decides whether the registry gains that form or the test's source becomes `HTTP::header insert`. This is not a naming question; it is filed here because it needs a device.
- **Environments.** TMM rule load, then `HTTP_REQUEST` (lab virtual server; send a request with a `Host` header). Load the three rules one at a time and record whether each loads.
- **Already on record.** Not measured on BIG-IP.

```tcl
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-068
    set bnc_obs {
        bare    {HTTP::header Host}
        pair    {HTTP::header X-Bnc one}
        seen    {HTTP::header exists X-Bnc}
        insert  {HTTP::header insert X-Bnc2 two}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
    set bnc_obs {
        after_pair   {HTTP::header X-Bnc3 three}
        after_insert {HTTP::header insert X-Bnc4 four}
        after_read   {HTTP::header value Host}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|$bnc_l|$bnc_rc|$bnc_r"
    }
}
```

Then load each of these as its own rule, with the words literal, and record the load result and the `/var/log/ltm` lines for one request:

```tcl
when HTTP_REQUEST { HTTP::header Cache-Control no-cache }
```

```tcl
when HTTP_REQUEST { HTTP::respond 200 content ok; HTTP::header Cache-Control no-cache }
```

There is no Tcl baseline: `HTTP::header` is a TMM command.

| Label | Expected | Basis |
|---|---|---|
| `bare` | to be recorded | n/a |
| `pair` | to be recorded | n/a |
| `seen` | to be recorded (`1` if `pair` inserted) | n/a |
| `insert` | `0`, empty | n/a |
| `after_pair` | to be recorded | n/a |
| `after_insert` | to be recorded | n/a |
| `after_read` | to be recorded | n/a |
| literal rule 1 load | to be recorded | n/a |
| literal rule 2 load and run | to be recorded | n/a |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-070 — A backslash inside a braced argument

- **Question.** Does TMM pass the backslashes of a braced argument through unchanged, as Tcl 8.4 does (in a brace group only backslash-newline is replaced)?
- **Why the toolchain needs it.** `vendor_source_literal_units` (`rust/tcl-syntax/src/naming/vendor_source.rs`) reads a braced word that holds a backslash and no backslash-newline as its content bytes, so a handler's `regsub -all {[][{}()*+?.\\^$|]} $s {\\&}` is recognised as the regex-escaping form (the `REGEX_LITERAL` colour behind T103 suppression and T106) and a braced pattern ends the T102 option scan. Read it together with item 11 of §6 (issue #2151, the F5 string character model): both concern the bytes TMM keeps for a word.
- **Environments.** TMM `HTTP_REQUEST` (lab virtual server, any request).
- **Already on record.** Not measured on BIG-IP.

```tcl
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-070
    log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|len|[string length {a\\b}]"
    binary scan {a\\b} H* bnc_h
    log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|hex|$bnc_h"
    log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|quote|[regsub -all {[][{}()*+?.\\^$|]} a.b {\\&}]"
    log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|cont|[string length {a\
b}]"
    HTTP::respond 200 content "BNC $bnc_id [TMM::cmp_unit]" Connection close
}
```

| Label | Tcl 8.4 | Basis | Tcl 8.5 to 9.1 |
|---|---|---|---|
| `len` | `4` | tclsh 8.4.20 | same |
| `hex` | `615c5c62` | tclsh 8.4.20 | same |
| `quote` | `a\.b` | tclsh 8.4.20 | same |
| `cont` | `3` (backslash-newline becomes a space) | tclsh 8.4.20 | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-071 — What runs after a response, a drop or a DNS answer, and payload without collect

- **Question.** After `HTTP::respond` in `HTTP_REQUEST`, does a later `HTTP::header insert` or a second `HTTP::respond`/`HTTP::redirect` raise or log an error, and do the commands after `drop` or `DNS::return` in the same event still run? Does `HTTP::payload` return data in `HTTP_REQUEST` with no `HTTP::collect`?
- **Why the toolchain needs it.** The iRules flow checks claim each of these: IRULE1201 (header change after a response), IRULE1202 (a second response), IRULE5002 (commands after `drop` still run, so the drop needs a `return`), IRULE5004 (the same after `DNS::return`), IRULE1005/IRULE1006 (payload needs a collect). The checks are `rust/tcl-compiler/src/irules_checks/`; the examples are in `docs/design/compiler/value-transfers-examples.md`. The test-scrub ledger (`docs/work/test-scrub/ledger/taint.tsv`, cause `irules-checks-dialect-blind-unit`) keeps those tests on this check. None is measured.
- **Environments.** TMM `HTTP_REQUEST` on an HTTP virtual server (a GET and a POST with a 10-byte body); TMM `DNS_REQUEST` on a DNS virtual server for the second rule.

```tcl
when HTTP_REQUEST {
    set bnc_id BIGIP-NAME-071
    log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|payload|[string length [HTTP::payload]]"
    HTTP::respond 200 content "BNC $bnc_id" Connection close
    set bnc_rc [catch {HTTP::header insert X-Bnc71 yes} bnc_r]
    log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|after_respond_header|$bnc_rc|$bnc_r"
    set bnc_rc [catch {HTTP::respond 200 content again} bnc_r]
    log local0. "BNC|$bnc_id|tmm[TMM::cmp_unit]|second_respond|$bnc_rc|$bnc_r"
}
```

Load the next two as separate rules (the first on the HTTP virtual server, the second on the DNS virtual server) and record whether the `log` after the terminating command appears:

```tcl
when HTTP_REQUEST { drop; log local0. "BNC|BIGIP-NAME-071|after_drop" }
```

```tcl
when DNS_REQUEST { DNS::return; log local0. "BNC|BIGIP-NAME-071|after_dns_return" }
```

There is no Tcl baseline: these are TMM commands.

| Label | Expected by the checks | Basis |
| --- | --- | --- |
| `payload` | `0` on the POST without a collect | IRULE1006 claim |
| `after_respond_header` | an error or a logged TCL error | IRULE1201 claim |
| `second_respond` | an error or a logged TCL error | IRULE1202 claim |
| `after_drop` | logged (execution continues) | IRULE5002 claim |
| `after_dns_return` | logged (execution continues) | IRULE5004 claim |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-072 — A plain variable across the events of one connection, and `global`

- **Question.** Is a plain variable set in `CLIENT_ACCEPTED` (or `RULE_INIT`) readable in a later `HTTP_REQUEST` of the same connection, and gone on a new connection? Does `global x` in an event link to a variable shared by every connection on the TMM?
- **Why the toolchain needs it.** IRULE4003 (a variable read in one event and set in another), IRULE5001 (a debug flag set in a setup event is not unset) and IRULE6001 (`global` and `::` names are shared across connections) rest on these, through the connection scope (`rust/tcl-compiler/src/connection_scope.rs`). The test-scrub ledger keeps those tests on this check. Not measured.
- **Environments.** TMM, one virtual server, two HTTP requests on one keep-alive connection, then one request on a new connection.

```tcl
when CLIENT_ACCEPTED {
    if {![info exists bnq72conn]} { set bnq72conn [TCP::client_port] }
    global bnq72glob
    if {![info exists bnq72glob]} { set bnq72glob 0 }
    incr bnq72glob
}
when HTTP_REQUEST {
    global bnq72glob
    log local0. "BNC|BIGIP-NAME-072|tmm[TMM::cmp_unit]|conn|[info exists bnq72conn]|[expr {[info exists bnq72conn] ? $bnq72conn : {}}]|glob|$bnq72glob"
}
```

| Label | Expected by the checks | Basis |
| --- | --- | --- |
| `conn` on both requests of the first connection | `1` and that connection's client port | IRULE4003/IRULE5001 claim |
| `conn` on the second connection | `1` and the new connection's port | IRULE4003 claim |
| `glob` | counts every connection on the TMM | IRULE6001 claim |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-073 — Is there an `env` array in TMM

- **Question.** Does `::env` exist in TMM, and if so what does it hold?
- **Why the toolchain needs it.** `env` is a taint source under the Tcl releases (tclsh 8.4.20 to 9.1.0 print `1` for `info exists env` at file level); the test `it_3 taint::sources::special_variable_sources::env_is_not_a_source_in_irules` claims it is not one under f5-irules. Not measured.
- **Environments.** TMM `RULE_INIT` and `HTTP_REQUEST`.

```tcl
when RULE_INIT { log local0. "BNC|BIGIP-NAME-073|init|[info exists ::env]|[expr {[info exists ::env] ? [array size ::env] : {}}]" }
when HTTP_REQUEST { log local0. "BNC|BIGIP-NAME-073|req|[info exists ::env]" }
```

| Label | Tcl 8.4 | Basis |
| --- | --- | --- |
| `init`, `req` | `1` and the process environment | tclsh 8.4.20 |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

### 4.8 tmsh, iApp and iCall

The tmsh-hosted contexts share an interpreter build but are entered differently; run each check in every context its environment line lists.

#### BIGIP-NAME-060 — Execution namespace and frame of tmsh-hosted scripts

- **Question.** In which namespace and at which call level do a cli script's `script::run`, an iApp implementation and an iCall script run, and where does a bare variable go?
- **Why the toolchain needs it.** `BigIpExecutionContext` distinguishes `TmshCliScript`, `IAppImplementation` and `ICallScript`; `VendorSourceNamePolicy::authored` (`rust/tcl-syntax/src/naming/vendor_source.rs`) gives each only source-level advice. The frame decides whether bare names are locals.
- **Environments.** tmsh cli script (shown); iApp implementation and iCall with the §2 wrappers, using `eval $bnc_s`.
- **Already on record.** 21.1.0.1: `namespace current` is `::script` in a cli script and `::` in iApp implementation and iCall (`evidence/tcl/bigip-nh20261012repair2-tmsh-cli-script-0.md`, `bigip-namespace-command-currency.md`). Call level and variable placement not recorded.

```tcl
cli script /Common/bnc_c60 {
proc script::run {} {
    set bnc_id BIGIP-NAME-060
    set bnc_obs {
        ns    {list [namespace current] [namespace parent]}
        level {info level}
        frame {set bnq60l 1; list [info exists bnq60l] [info exists ::bnq60l] [info locals bnq60*] [info globals bnq60*]}
        ident {list [info exists ::tcl_patchLevel] [info patchlevel] [llength [info commands]]}
        clean {catch {unset bnq60l}; list}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        puts "BNC|$bnc_id|cli|$bnc_l|$bnc_rc|$bnc_r"
    }
}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `ns` | `0` `:: {}` | S (tclsh global level; a cli script is expected to say `::script ::`) | same |
| `level` | `0` `0` | S (a cli script runs inside proc `script::run`, so 1 is expected there) | same |
| `frame` | `0` `1 1 {} bnq60l` | S (global level) | same |
| `ident` | `0` `1 8.4.20 …` | S | n/a |
| `clean` | `0`, empty | S | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-061 — The `tmsh::` command namespace

- **Question.** Is `tmsh` a real Tcl namespace, are its commands visible to `namespace which` and `info commands`, and are they reachable from another namespace?
- **Why the toolchain needs it.** The registry lists `tmsh::*` commands by qualified name (`rust/tcl-registry/src/commands/iapps/tmsh__*.rs`); the resolver must know whether they follow ordinary relative-qualified lookup.
- **Environments.** tmsh cli script (shown); iApp implementation and iCall with the §2 wrappers.
- **Already on record.** 21.1.0.1: `tmsh::version` returns `21.1.0.1` in iApp implementation and iCall and is `invalid command name` in TMM (`evidence/tcl/bigip-r2286l-p-tmsh-version.md`); 95 commands in the tmsh-hosted interpreters. Namespace existence and inventories not measured.

```tcl
cli script /Common/bnc_c61 {
proc script::run {} {
    set bnc_id BIGIP-NAME-061
    set bnc_obs {
        nsex  {list [namespace exists ::tmsh] [namespace exists ::script] [namespace exists ::iapp]}
        which {list [namespace which -command tmsh::version] [namespace which -command ::tmsh::version] [namespace which -command tmsh::get_field_value]}
        count {list [llength [info commands]] [llength [info commands ::tmsh::*]] [llength [info procs ::tmsh::*]]}
        names {lrange [lsort [info commands ::tmsh::*]] 0 7}
        rel   {namespace eval ::bnq61 {namespace which -command tmsh::version}}
        call  {set c tmsh::version; $c}
        kids  {lsort [namespace children ::]}
    }
    set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
    catch {uplevel #0 $bnc_w}
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        puts "BNC|$bnc_id|cli|$bnc_l|$bnc_rc|$bnc_r"
    }
    catch {uplevel #0 $bnc_w}
}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `nsex` | `0`, to be recorded (`1 …` if `tmsh::` commands are registered under a real namespace) | n/a | n/a |
| `which` | `0` `::tmsh::version ::tmsh::version ::tmsh::get_field_value` | S if they are ordinary commands in `::tmsh` | same |
| `count` | `0`, to be recorded | n/a | n/a |
| `names` | `0`, to be recorded | n/a | n/a |
| `rel` | `0` `::tmsh::version` | V-cmd (relative-qualified, global fallback), same condition | same |
| `call` | `0`, the BIG-IP version | n/a | n/a |
| `kids` | `0`, to be recorded | n/a | n/a |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-062 — Where procs defined in a cli script live

- **Question.** In which namespace are a cli script's top-level `proc name`, `proc script::name` and a proc defined at run time inside `script::run` created?
- **Why the toolchain needs it.** The registry models `script::run`, `script::init`, `script::help`, `script::tabc` as entry points (`rust/tcl-registry/src/commands/iapps/script__run.rs`); helper procs must be qualified the way tmsh creates them. The measured relative-`proc` behaviour (BIGIP-NAME-041) makes the 8.4 reading doubtful here.
- **Environments.** tmsh cli script.
- **Already on record.** 21.1.0.1: inside `script::run` (namespace `::script`), `namespace eval ::ns {proc p …}` created `::p`, not `::ns::p` and not `::script::p` (`evidence/tcl/bigip-nh20261012nsowner1-tmsh-cli-script-0.md`). Top-level helper placement not measured.

```tcl
cli script /Common/bnc_c62 {
proc bnq62h {} { return [namespace current] }
proc script::bnq62s {} { return [namespace current] }
proc script::run {} {
    set bnc_id BIGIP-NAME-062
    set bnc_obs {
        helper {list [namespace which -command bnq62h] [namespace which -command ::bnq62h] [namespace which -command ::script::bnq62h]}
        scoped {list [namespace which -command bnq62s] [namespace which -command ::script::bnq62s] [namespace which -command ::bnq62s]}
        call   {list [catch {bnq62h} a] $a [catch {bnq62s} b] $b}
        procs  {lsort [info procs ::script::*]}
        inrun  {proc bnq62d {} {namespace current}; list [namespace which -command ::script::bnq62d] [namespace which -command ::bnq62d] [catch {bnq62d} c] $c}
        clean  {foreach c {::bnq62h ::bnq62d ::script::bnq62h ::script::bnq62d ::script::bnq62s} {catch {rename $c {}}}; list}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        puts "BNC|$bnc_id|cli|$bnc_l|$bnc_rc|$bnc_r"
    }
}
}
```

**Expected.** Under Tcl 8.4 rules, with the script text evaluated in `::` and `script::run` running in `::script`: `helper` `0|::bnq62h ::bnq62h {}`; `scoped` `0|::script::bnq62s ::script::bnq62s {}`; `call` `0|0 :: 0 ::script`; `inrun` `0|::script::bnq62d {} 0 ::script`. The measured tmsh behaviour predicts `inrun` = `{} ::bnq62d 0 ::`.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-063 — cli script argument variables `tmsh::argc` and `tmsh::argv`

- **Question.** Are the script arguments ordinary namespace variables `::tmsh::argc` / `::tmsh::argv`, and which variables exist in `::tmsh` and `::script`?
- **Why the toolchain needs it.** The analyser resolves `$tmsh::argv` as a qualified variable; it needs to know the namespace exists and the variables are real (`NativeNameProtocol::variable_root_input` qualified branch).
- **Environments.** tmsh cli script, run as `tmsh run cli script /Common/bnc_c63 one "two words"`.
- **Already on record.** Not measured.

```tcl
cli script /Common/bnc_c63 {
proc script::run {} {
    set bnc_id BIGIP-NAME-063
    set bnc_obs {
        argv  {list [info exists tmsh::argc] [info exists ::tmsh::argv] [catch {set tmsh::argv} m] $m}
        vars  {lsort [info vars ::tmsh::*]}
        svars {lsort [info vars ::script::*]}
        which {list [namespace which -variable tmsh::argv] [namespace which -variable ::tmsh::argv]}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        puts "BNC|$bnc_id|cli|$bnc_l|$bnc_rc|$bnc_r"
    }
}
}
```

**Expected.** No Tcl 8.4 equivalent. If they are ordinary namespace variables: `argv` `0|1 1 0 {<script name> one {two words}}` (whether the script name is element 0 is to be recorded); `which` `0|::tmsh::argv ::tmsh::argv`.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-064 — APL fields as Tcl variables in an iApp implementation

- **Question.** Under which names does an iApp implementation see an APL field `section.field`: `$::section__field`, the bare name, colon-run spellings? Is it a global variable?
- **Why the toolchain needs it.** `apl_name_to_tcl_var` and `tcl_var_to_apl_name` (`rust/tcl-bigip/src/apl/variable_names.rs`) map `section.field` to `::section__field`; iApp diagnostics for undefined and unused fields depend on it. The mapping is a convention in the code, not a measurement (`experiments/name-handling/apl-source-variable-geometry/f5-questions.md`).
- **Environments.** iApp implementation (template with a presentation section and a service that supplies the value).
- **Already on record.** Not measured. iApp implementation runs in namespace `::` on 21.1.0.1.

Adjust the presentation and `variables` syntax if this build rejects them, and return the exact text that loaded:

```tcl
sys application template /Common/bnc_a64 {
  actions { definition {
    implementation {
        set bnc_id BIGIP-NAME-064
        set bnc_obs {
            abs    {set ::basic__addr}
            triple {set :::basic__addr}
            bare   {set basic__addr}
            dollar {set r "$::basic__addr,${::basic__addr}"}
            vars   {list [info vars ::basic__*] [info globals basic__*] [info locals basic__*]}
            where  {list [namespace which -variable ::basic__addr] [namespace exists ::basic] [namespace current] [info level]}
            inproc {proc ::bnq64p {} {return $::basic__addr}; set r [::bnq64p]; rename ::bnq64p {}; set r}
            absent {info exists ::basic__nosuch}
        }
        foreach {bnc_l bnc_s} $bnc_obs {
            set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
            tmsh::log err "BNC|$bnc_id|iapp|$bnc_l|$bnc_rc|$bnc_r"
        }
    }
    presentation {
        section basic {
            string addr
        }
    }
  } }
}
sys application service /Common/bnc_a64_svc {
  template /Common/bnc_a64
  variables { basic__addr { value ROOT } }
}
```

**Expected.** No Tcl 8.4 equivalent. The convention in the code predicts `abs` `0|ROOT`; if the variable is an ordinary global and the implementation runs at the global level, `triple`, `bare` and `inproc` are also `ROOT`, `vars` lists `::basic__addr basic__addr {}`, and `where` starts with `::basic__addr 0 ::`.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-065 — `__` section separators, tables and name collisions in APL variables

- **Question.** How are fields with underscores, nested tables and colliding names published as Tcl variables?
- **Why the toolchain needs it.** `tcl_var_to_apl_name` splits at `__`; a field that itself contains `__` or a table makes the reverse mapping ambiguous, and the code declines some of these (`rust/tcl-bigip/src/apl/variable_names.rs`).
- **Environments.** iApp implementation.
- **Already on record.** Not measured. The question list is in `experiments/name-handling/apl-source-variable-geometry/f5-questions.md`.

Adjust the presentation, `variables` and `tables` syntax if this build rejects them, and return the exact text that loaded:

```tcl
sys application template /Common/bnc_a65 {
  actions { definition {
    implementation {
        set bnc_id BIGIP-NAME-065
        set bnc_obs {
            netvars {lsort [info vars ::net__*]}
            under   {list [info exists ::net__member_addr] [info exists ::net__member__addr]}
            values  {list [catch {set ::net__member_addr} a] $a [catch {set ::net__member__addr} b] $b}
            tblvars {lsort [info vars ::tls__*]}
            tbl     {list [info exists ::tls__members] [array exists ::tls__members] [catch {set ::tls__members} m] $m}
            tblcol  {list [info exists ::tls__members__port] [catch {set ::tls__members__port} m] $m}
        }
        foreach {bnc_l bnc_s} $bnc_obs {
            set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
            tmsh::log err "BNC|$bnc_id|iapp|$bnc_l|$bnc_rc|$bnc_r"
        }
    }
    presentation {
        section net {
            string member_addr
            string member__addr
        }
        section tls {
            table members {
                string port
            }
        }
    }
  } }
}
sys application service /Common/bnc_a65_svc {
  template /Common/bnc_a65
  variables {
    net__member_addr { value ONE }
    net__member__addr { value TWO }
  }
  tables {
    tls__members {
      column-names { port }
      rows { { row { 443 } } }
    }
  }
}
```

**Expected.** No Tcl 8.4 equivalent and no prediction: record whether the template loads, every line, and the exact template and service text.

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-066 — The `iapp::` procs

- **Question.** Are the `iapp::` helper commands Tcl procs in a real `::iapp` namespace supplied by a package, and are they reachable from another namespace?
- **Why the toolchain needs it.** The registry lists `iapp::*` commands (`rust/tcl-registry/src/commands/iapps/iapp__*.rs`). If they are procs defined with relative names inside `namespace eval`, the tmsh proc-placement behaviour (BIGIP-NAME-041) decides where they really are.
- **Environments.** iApp implementation.
- **Already on record.** 21.1.0.1: `package names` in iApp implementation and iCall includes `iapp` (`evidence/tcl/bigip-r2286l-s-package.md`). Nothing else measured.

```tcl
sys application template /Common/bnc_a66 {
  actions { definition {
    implementation {
        set bnc_id BIGIP-NAME-066
        set bnc_obs {
            pkg   {list [catch {package present iapp} m] $m [namespace exists ::iapp]}
            procs {list [llength [info procs ::iapp::*]] [llength [info commands ::iapp::*]] [lrange [lsort [info procs ::iapp::*]] 0 5]}
            which {list [namespace which -command iapp::conf] [namespace which -command ::iapp::conf] [namespace which -command ::conf]}
            rel   {namespace eval ::bnq66 {namespace which -command iapp::conf}}
            vars  {lrange [lsort [info vars ::iapp::*]] 0 7}
        }
        set bnc_w {foreach n [namespace children :: ::bnq*] {namespace delete $n}; foreach c [info commands ::bnq*] {rename $c {}}; foreach v [info vars ::bnq*] {catch {unset $v}}}
        catch {uplevel #0 $bnc_w}
        foreach {bnc_l bnc_s} $bnc_obs {
            set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
            tmsh::log err "BNC|$bnc_id|iapp|$bnc_l|$bnc_rc|$bnc_r"
        }
        catch {uplevel #0 $bnc_w}
    }
    presentation { }
  } }
}
sys application service /Common/bnc_a66_svc { template /Common/bnc_a66 }
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `pkg` | `0`, to be recorded | n/a | n/a |
| `procs` | `0`, to be recorded | n/a | n/a |
| `which` | `0` `::iapp::conf ::iapp::conf {}` | S if `iapp::conf` is a proc in `::iapp` | same |
| `rel` | `0` `::iapp::conf` | V-cmd, same condition | same |
| `vars` | `0`, to be recorded | n/a | n/a |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-067 — tmsh object field names with `tmsh::get_field_value`

- **Question.** Are tmsh object and field names opaque strings (hyphens, slashes) rather than Tcl names, and does the optional variable argument create a variable in the caller's frame?
- **Why the toolchain needs it.** The registry gives `tmsh::get_field_value <object> <field_name>` (`rust/tcl-registry/src/commands/iapps/tmsh__get_field_value.rs`) no variable-name role; a third, variable-name argument would need one.
- **Environments.** tmsh cli script (shown); iApp implementation and iCall with the §2 wrappers, using `eval $bnc_s`.
- **Already on record.** Not measured.
- **Note.** `tmsh::get_config` is given `/sys global-settings hostname`; if this build wants another component spelling, change it and return the text that ran.

```tcl
cli script /Common/bnc_c67 {
proc script::run {} {
    set bnc_id BIGIP-NAME-067
    set bnc_obs {
        obj     {set o [lindex [tmsh::get_config /sys global-settings hostname] 0]; list [tmsh::get_type $o] [tmsh::get_name $o]}
        field   {set o [lindex [tmsh::get_config /sys global-settings hostname] 0]; tmsh::get_field_value $o hostname}
        missing {set o [lindex [tmsh::get_config /sys global-settings hostname] 0]; tmsh::get_field_value $o bnq67-none}
        intovar {set o [lindex [tmsh::get_config /sys global-settings hostname] 0]; set n [tmsh::get_field_value $o hostname bnq67v]; list $n [info exists bnq67v] [info exists ::bnq67v]}
        clean   {catch {unset bnq67v}; catch {unset o}; catch {unset n}; list}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {eval $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        puts "BNC|$bnc_id|cli|$bnc_l|$bnc_rc|$bnc_r"
    }
}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `obj` | `0`, to be recorded | n/a | n/a |
| `field` | `0`, the configured hostname | n/a | n/a |
| `missing` | `1`, to be recorded | n/a | n/a |
| `intovar` | `0`, to be recorded (three-argument form; record the error if it does not exist) | n/a | n/a |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

#### BIGIP-NAME-069 — Which `proc` body formations define a procedure outside TMM

- **Question.** In a tmsh cli script and an iApp implementation, is `proc` the stock handler, which takes its body as an evaluated value (quoted, bare, computed, built with `list`), or does the host restrict the body to a braced literal as the TMM rule loader does?
- **Why the toolchain needs it.** `procedure_definition_body_policy` (`rust/tcl-registry/src/native_procedure.rs`) answers `BracedLiteralEstablished` for `Family::F5Tcl`: a braced body defines the procedure and its body is analysed; any other formation is undecided, so the definition completes unknown and nothing after it in the script is proved. The measured answer replaces that row with `EvaluatedValue` or `OriginalBracedLiteral`.
- **Environments.** tmsh cli script (shown); iApp implementation and iCall with the §2 wrappers, using `uplevel #0 $bnc_s`.
- **Already on record.** Not measured. `info patchlevel` is 8.4.6 in all three contexts on 21.1.0.1.

```tcl
cli script /Common/bnc_c69 {
proc script::run {} {
    set bnc_id BIGIP-NAME-069
    set bnc_obs {
        braced   {proc ::bnq69a {} {return a}; ::bnq69a}
        quoted   {proc ::bnq69b {} "return b"; ::bnq69b}
        bare     {proc ::bnq69c {} list; ::bnq69c}
        computed {set bnq69body {return d}; proc ::bnq69d {} $bnq69body; ::bnq69d}
        built    {proc ::bnq69e {} [list return e]; ::bnq69e}
        bodyof   {list [info body ::bnq69b] [info body ::bnq69d]}
        clean    {foreach c [info commands ::bnq69*] {rename $c {}}; catch {unset ::bnq69body}; list}
    }
    foreach {bnc_l bnc_s} $bnc_obs {
        set bnc_rc [catch {uplevel #0 $bnc_s} bnc_r]; regsub -all {[\r\n]+} $bnc_r { / } bnc_r
        puts "BNC|$bnc_id|cli|$bnc_l|$bnc_rc|$bnc_r"
    }
}
}
```

| Label | Tcl 8.4 (`rc`, result) | Basis | Tcl 8.6 / 9.0 |
|---|---|---|---|
| `braced` | `0` `a` | tclsh 8.4.20 | same |
| `quoted` | `0` `b` | tclsh 8.4.20 | same |
| `bare` | `0`, empty | tclsh 8.4.20 | same |
| `computed` | `0` `d` | tclsh 8.4.20 | same |
| `built` | `0` `e` | tclsh 8.4.20 | same |
| `bodyof` | `0` `{return b} {return d}` | tclsh 8.4.20 | same |

| Observed result | BIG-IP build | Environment | Date | Verdict |
|---|---|---|---|---|
| | | | | |

## 5. Recording results

### What to send back

One fenced block per check and environment, in this form. Keep the `BNC|…`
lines exactly as printed (all TMMs, duplicates removed).

````text
CHECK: BIGIP-NAME-012
BUILD: BIG-IP 21.1.0.1 build 0.0.26 Point Release 1   (from tmsh show sys version)
PLATFORM: <tmsh show sys hardware platform>, 4 TMMs
ENVIRONMENT: TMM RULE_INIT, unattached rule /Common/bnc_r12
TCL: info patchlevel 8.4.6   (from BIGIP-NAME-001 in this environment)
DATE: 2026-11-20
SNIPPET: as written   (or: the exact text that ran, if anything was changed, and why)
LOAD: ACCEPT          (or the loader's message)
OUTPUT:
BNC|BIGIP-NAME-012|tmm0|def|0|
BNC|BIGIP-NAME-012|tmm0|mk|0|::bnq12::a
…
CONTROL (optional): the same lines from /usr/bin/tclsh8.4, with its info patchlevel
VERDICT: matches 8.4   (or: differs: <labels>; command unavailable: <message>; recorded)
NOTES: anything that did not go to plan (missing lines, cleanup that failed, objects left behind)
````

Rules for the report:

- Report what ran. If a snippet had to be changed, send the changed text.
- Keep load rejection separate from run-time results. A rejected rule has a
  `LOAD:` message and no `OUTPUT:`.
- Do not fill in a line that did not appear, and do not carry a result from
  one environment or build to another.
- An unavailable context or command is a result; say how you know.
- Send the BIGIP-NAME-001 block for each environment and build once.

### Where results are filed

The repository's evidence standard is
`docs/design/contracts/tcl-execution-evidence.md`: evidence is a script that
actually ran, the environment it ran in, the question it answers and the
measured result. The returned blocks map onto it directly.

1. The returned blocks are filed unchanged under
   `experiments/name-handling/f5/`, one file per appliance build and
   environment.
2. Each check whose result the code will rely on becomes one file under
   `evidence/tcl/`, named `bigip-name-<NNN>-<environment>.md`, using
   `experiments/name-handling/f5/ANSWER-TEMPLATE.md`: **Question** (the check's
   question and why), **Assumptions** (build, environment, anything changed),
   **Script** (the exact snippet), **Results** (one subsection per environment
   with the `BNC|…` lines). `make check-tcl-evidence` checks the format.
3. Fill in the check's row in this document (observed result, build,
   environment, date, verdict).

### Who updates the code when a check differs from 8.4

| Area | Rust owner to update | Test data to extend |
|---|---|---|
| Naming recipe selection for F5 | `NativeNameProtocol::for_point` (`rust/tcl-syntax/src/naming/native.rs`); `ObservedBigIpNamePolicy` and `MeasuredBigIpNameScope` (`rust/tcl-syntax/src/naming/execution_policy.rs`); `observed_name_policy` (`rust/tcl-registry/src/f5/naming.rs`); `VendorSourceNamePolicy` (`rust/tcl-syntax/src/naming/vendor_source.rs`) | a measured scope per build, context and event |
| Command name resolution | `resolve_command_with`, `command_resolution_candidates`, `written_command_parts` (`rust/tcl-syntax/src/naming.rs`); `rename_*`, `alias_*` methods of `NativeNameProtocol` | `rust/tcl-syntax/tests/data/command_resolution_vectors.txt` |
| Namespaces | `namespace_address_input`, `namespace_tail_bytes`, `namespace_qualifier_bytes`, `report_native_namespace_operation_error` (`rust/tcl-syntax/src/naming/native.rs`) | `rust/tcl-syntax/tests/data/namespace_op_vectors.txt` |
| Variables and array elements | `variable_root_input`, `combined_variable_input`, `separate_variable_input` (`rust/tcl-syntax/src/naming/native.rs`); `rust/tcl-syntax/src/naming/variable_geometry.rs`, `variable_spelling.rs`, `compiled_variables.rs` | `rust/tcl-syntax/tests/data/variable_resolution_vectors.txt` |
| `$` substitution | `GRAMMAR_F5_TCL` (`rust/tcl-dialect/src/model/family.rs`); `VarSyntax`, `BracedVarStyle`, `ArrayIndexSyntax` (`rust/tcl-dialect/src/grammar.rs`); `scan_var_ref` (`rust/tcl-lexer/src/word_parts.rs`) | lexer tests in `rust/tcl-lexer` |
| Procedure naming | `command_publication_input`, `command_publication_slot` (`rust/tcl-syntax/src/naming/native.rs`); `FormalParameterError` (`rust/tcl-syntax/src/formal_params.rs`) | `rust/tcl-syntax/tests/data/native_procedure_name/` |
| iRules in TMM | `rust/tcl-registry/src/commands/irules/call.rs`; `ProtocolNamespaceSpec` (`rust/tcl-registry/src/profiles.rs`); `Family::closed_load_time_resolution` (`rust/tcl-dialect/src/model/family.rs`); `AuthoredTmmStaticPolicy` (`rust/tcl-runtime-api/src/authored_tmm.rs`), `rust/tcl-vm/src/interp/authored_tmm_static.rs`; `docs/design/contracts/namespace-model.md` | `rust/tcl-irule-test` scenarios |
| tmsh, iApp, iCall | `BigIpExecutionContext` (`rust/tcl-dialect/src/model/bigip_execution_context.rs`); `rust/tcl-registry/src/commands/iapps/`; `apl_name_to_tcl_var` (`rust/tcl-bigip/src/apl/variable_names.rs`) | iApp diagnostics tests in `rust/tcl-bigip` |

A `differs` verdict does not change code by itself. It becomes an
`evidence/tcl/` file first; the owner then adds the F5 rule with that file as
its citation and a regression that reads the recorded lines.

## 6. Open questions that need a human

1. **Which 8.4 is the baseline?** The vectors are pinned on 8.4.20, the
   on-box control is 8.4.13 and the embedded Tcl is 8.4.6. Re-importing an
   already imported command (BIGIP-NAME-024) may be one place where they
   differ. Should "matches 8.4" mean 8.4.6 where the patch levels disagree?
2. **Which BIG-IP releases must be covered**, and is one result per release
   enough or is it needed per build? Everything on record is 21.1.0.1.
3. **May the destructive check run?** BIGIP-NAME-055 renames a built-in iRules
   command and renames it back. It needs explicit permission and a disposable
   unit.
4. **Relative `proc` placement in tmsh, iApp and iCall** is measured and is not
   Tcl 8.4. Should those contexts get their own rule now, or wait for a second
   release?
5. **The TMM `namespace eval … set` result** (BIGIP-NAME-051) contradicts 8.4
   and has no explanation yet. Does it block using the 8.4 variable recipe for
   iRules?
6. **Rule procs and Tcl namespaces.** `docs/design/contracts/namespace-model.md`
   §3 says a proc in rule `my_irule` becomes `::my_irule::my_helper`; the
   measurements say the proc runs in `::` and is invisible to
   `namespace which`. Which statement should the code follow until
   BIGIP-NAME-045 is run?
7. **APL Tcl callbacks.** `BigIpExecutionContext::IAppPresentationTclCallback`
   is documented as never measured, yet `evidence/tcl/bigip-apl-*.md` records
   about 150 callback executions on 21.1.0.1. Is that context now in scope?
8. **Filing.** This document files returned blocks under
   `experiments/name-handling/f5/` and converted evidence under
   `evidence/tcl/`. Confirm that split, or name the single place wanted.
9. **`HTTP::header NAME VALUE` after a response** is now check
   BIGIP-NAME-068. Until it is run,
   `irules_checks::tests::irule1201_http_after_respond_fires` keeps its
   expectation and stays failing.
10. **Wide shift in an iRule expression.** Not a naming question, filed here
    because the constant folder now answers it from the 8.4 rule: under
    `f5-irules`, `expr {1 << 70}` folds to 0, `expr {(1 << 70) == 0}` to 1 and
    `expr {-(1 << 70)}` to 0, which is what tclsh 8.4.20 prints
    (`tcl_expr_eval::tests::the_old_folder_stays_within_the_targets_tower`).
    Run `when RULE_INIT { log local0. "[expr {1 << 70}] [expr {(1 << 70) == 0}] [expr {-(1 << 70)}]" }`
    on TMM; anything other than `0 1 0` means the F5 profile needs its own
    arithmetic model.
11. **String character model** (issue #2151). Not a naming question; filed
    here so it is measured on the same visit. `Family::F5Tcl` and
    `Family::F5Irules` answer `StringCharacterModel::Utf16CodeUnits`
    (`character_model`, `rust/tcl-dialect/src/model/family.rs`), an
    inference with no measurement behind it. Run in both TMM and a tmsh cli
    script:
    `set s [encoding convertfrom utf-8 "\xF0\x9F\x98\x80"]; puts "[string length $s] [scan [string index $s 0] %c]"`
    (in TMM, `log local0.` the same string from `RULE_INIT`). Do not put a
    hex digit after the last `\x80`: in 8.4 `\x` consumes every following hex
    digit. `4 240` means `BmpCharsElseUtf8Bytes`, as stock 8.4.20 prints;
    `2 55357` means `Utf16CodeUnits`, the current value; `1 128512` means
    neither. Record the result and the build; the arm then cites it like its
    neighbours in `canonical_builds_resolve_measured_capabilities`.
12. **Prefix operator before an iRules comparison** (issue #2165). Not a
    naming question. The compiler groups `! A contains B` as
    `(! A) contains B` (`UNARY_BP` in `rust/tcl-syntax/src/expr/parser.rs`
    against the F5 comparison rows in `rust/tcl-dialect/src/model/expr_grammar.rs`);
    `rust/tcl-irule-test/tcl/expr_ops.tcl` groups it as `! (A contains B)`.
    Run in TMM `RULE_INIT`:
    `log local0. "[catch {expr {! "abc" contains "z"}} r1] $r1 | [catch {expr {not 1 starts_with ""}} r2] $r2"`.
    If TMM groups the comparison tighter than the prefix operator, `UNARY_BP`
    is wrong for F5's custom comparisons and the compiler needs the fix; the
    harness's refusal of `not` (`_refuse_unmeasured_prefix_grouping`) can
    then be replaced by that grouping.
13. **Constant integer expressions.** The optimiser folds a constant
    expression in an F5 unit (iRule, iApp implementation, tmsh script) by the
    8.4 rule: `expr {1 + 2}` is 3 and `expr {010 + 1}` is 9 (a leading zero
    reads as octal). Run
    `log local0. "[expr {1 + 2}] [expr {010 + 1}] [expr {-010}]"` in each of
    TMM, tmsh and an iApp implementation; anything other than `3 9 -8` means
    the fold must stop for that host
    (`optimiser_coverage::an_f5_unit_folds_a_constant_expression_as_tcl_8_4_does`).
14. **Double overflow.** tclsh 8.4.20 raises `floating-point value too large
    to represent` for `expr {1e308 * 10}`, so the fold leaves it as written
    under the F5 profiles. Run `catch {expr {1e308 * 10}} r; log local0. $r`;
    if TMM prints `Inf`, the F5 profile widens like Tcl 8.5 and the fold may
    take it.
15. **Word operators on literals.** `if {"abcde" contains "cd"} {…}` in an
    iRule is folded to its body. Run
    `log local0. "[expr {"abcde" contains "cd"}] [expr {"abcde" starts_with "ab"}] [expr {"abcde" ends_with "de"}] [expr {"abc" equals "abc"}]"`
    on TMM; expected `1 1 1 1`. Also say whether tmsh and iApp Tcl accept
    these operators at all: the optimiser assumes only iRules has them.
16. **Sanitisers and getter colours the taint diagnostics trust.** Not a
    naming question. In a `when` handler the taint diagnostics (IRULE3001 to
    IRULE3004, IRULE3101, T106) stay silent when a value went through a
    command the registry marks as a sanitiser, or came from a getter whose
    result has a fixed shape (`classify_substitution_taint`,
    `rust/tcl-compiler/src/taint.rs`; the rows are in
    `rust/tcl-registry/src/commands/irules/`). None of these claims is
    measured. Run in `when HTTP_REQUEST` against a request for
    `/a%0d%0aX:%20y?q=<b>"&'%0d%0a` with the header
    `X-T: <script>"&'` and log each result with `binary scan $v H* hex`
    so control characters show:
    - `HTML::encode [HTTP::header value X-T]`: the registry claims no `<`,
      `>`, `"`, `&` and no CR or LF remain (`HTML_ESCAPED`, `CRLF_FREE`).
      Say also whether `HTML::encode` exists on the build at all.
    - `URI::encode [HTTP::query]`, `URI::encode_component [HTTP::query]` and
      `URI::escape [HTTP::query]`: claimed percent-encoded with no CR or LF
      (`URL_ENCODED`, `CRLF_FREE`).
    - `HTTP::uri` and `HTTP::path`, bare and with `-normalized`: claimed
      always to begin with `/` (`PATH_PREFIXED`). Send also a request line with an absolute URI
      (`GET http://h/x HTTP/1.1`) and one with `*` (`OPTIONS *`).
      IRULE3004 treats `PATH_PREFIXED` as a same-origin redirect target, so
      `HTTP::redirect [HTTP::uri]` is silent. Send also
      `GET //evil.example/x HTTP/1.1` and `GET /\evil.example/x HTTP/1.1`,
      log `[HTTP::uri]`, `[HTTP::path]` and `[HTTP::path -normalized]`, and
      run `HTTP::redirect [HTTP::uri]` with the response's `Location`
      captured. If TMM passes a leading `//` or `/\` through, the value is a
      protocol-relative redirect to another host and `PATH_PREFIXED` must
      stop clearing IRULE3004.
    - `IP::client_addr`, `IP::remote_addr`: claimed an address literal only
      (`IP_ADDRESS`); include an IPv6 client with a route domain (`%1`).
    - `TCP::client_port`, `TCP::remote_port`: claimed decimal digits only
      (`PORT`).
    - `SSL::sni` (in `CLIENTSSL_CLIENTHELLO`, with a client that sends the
      server name `a b\r\nc`): claimed a DNS name with no whitespace or
      control characters (`FQDN`). If TMM passes the bytes through, the
      colour has to go.
17. **A source literal read numerically after another command read it.** Not
    a naming question; the optimiser's (`expr_operand_literal_reading` in
    `rust/tcl-compiler/tests/it_5/optimiser_coverage/constant_folding.rs`).
    On every tclsh from 8.4.20 to 9.1.0 the numeric reading of a variable
    holding a source literal follows its string whatever read the shared
    object first, so the optimiser folds `$a` into an expression there. F5
    units stay declined until TMM answers. In `when HTTP_REQUEST`, log the
    result of each:
    - `set a 1; format %f $a; log local0. [expr {$a + 1}]`
    - `set a 1; format %e $a; log local0. [expr {$a + 1}]`
    - `set a 1; lsort -real [list $a 0]; log local0. [expr {$a + 1}]`
    - `set a 1; binary format d $a; log local0. [expr {$a + 1}]`
    - `set a 1; expr {double($a)}; log local0. [expr {$a + 1}]`
    - `set a 010; format %f $a; log local0. [expr {$a + 1}]`
    - and the same with a `proc g {} {format %f 1}` defined in the rule and
      `call g` before `set a 1`.
    tclsh 8.4.20 prints `2` for each of the first five and `9` for the
    sixth. If TMM prints `2.0` for any, F5 must keep declining; if it
    prints what 8.4 does, F5 units may take the 8.4 fold once it exists.
18. **A top-level statement whose head is a rule procedure.** Not on record.
    BIGIP-NAME-057 (c) measured a direct call of a top-level proc from inside
    an event (rejected, `undefined procedure`); a bare statement at the top
    level of the rule was not measured. The rule loader's refusal
    (`rule_loader_rejection` in `rust/tcl-compiler/src/command_binding/rule_declaration.rs`)
    covers a registry-backed head the loader refuses and an unknown literal
    head, not a head bound to the rule's own proc, so the walk enters such a
    call. Load each as its own rule and record ACCEPT or REJECT with the
    loader message:
    - `proc bnq18p {} { return 1 }\nbnq18p\nwhen RULE_INIT { log local0. x }`
    - `proc bnq18p {} { return 1 }\nwhen RULE_INIT { log local0. [call bnq18p] }`
      (the control that must be accepted)
    - `bnq18nosuch\nwhen RULE_INIT { log local0. x }`
    Until answered, the walk keeps entering the call and the consumers that
    need a refusal stay conservative.
    Also load, each as its own rule (tests: `rule_declaration`
    `rule_loader_inventory_declines_unbraced_and_replaced_event_handlers`,
    `frozen_priority_retains_only_the_rule_loader_file_policy_value`,
    `direct_loader_keeps_later_procedure_body_without_installation_proof`):
    - (c) `bnq18nosuch\nproc bnq18q {} { return 1 }\nwhen HTTP_REQUEST { log local0. [call bnq18q] }`: is the
      whole rule refused, or only the statement? If accepted, send a request
      and record whether `bnq18q` runs.
    - (d) `proc when {args} { log local0. CUSTOM }\nwhen HTTP_REQUEST { log local0. handler }`: ACCEPT or REJECT. If accepted,
      send a request and record `handler`, `CUSTOM` or nothing.
    - (e) `set bnq18p 700\npriority $bnq18p\nwhen HTTP_REQUEST { log local0. one }` and a second rule
      `when HTTP_REQUEST priority 600 { log local0. two }` on the same virtual
      server: ACCEPT or REJECT for the first, and if accepted the order of
      `one` and `two`.
    Answers: (d) REJECT confirms that no handler can be replaced by a `proc`;
    ACCEPT with `handler` logged means a `proc when` does not replace the
    keyword. (e) is the evidence for whether a top-level variable can feed
    `priority`; without it the tests keep treating `priority $var` as not a
    declaration value.
31. **What `event … disable` touches** (issue #1712 audit). Not a naming
    question; filed here because it needs a device. The registry stated it
    as a `ConnectionControl` write, the target the connection-terminating
    commands (`drop`, `reject`, `discard`) use, and now states its side
    effects as `Unknown` (`rust/tcl-registry/src/commands/irules/event.rs`).
    On one virtual server with one HTTP request per connection, record what
    each rule does and whether the connection survives:
    - `when HTTP_REQUEST { event disable; log local0. "after" }` and a second
      rule `when HTTP_REQUEST priority 600 { log local0. "second" }`
    - `when HTTP_REQUEST { event HTTP_RESPONSE disable }` with
      `when HTTP_RESPONSE { log local0. "response" }`
    - `when CLIENT_ACCEPTED { event disable all }` with
      `when HTTP_REQUEST { log local0. "request" }`
    If every connection completes and only the named events stop firing,
    the effect is event control and gets its own target; until answered it
    stays `Unknown`.

#### Checks for tests that hold iRules behaviour no Tcl core shows (test scrub, `irules-registry-tooling`)

Each row says what to run on a BIG-IP and what answer confirms the test. The tests are in `docs/work/test-scrub/ledger/irules-registry-tooling.tsv`, marked `check TS-nn`. Until a row is answered the test stays, and the simulator or translator keeps declining where it cannot be sure.

19. **TS-01, per-TMM state.** Tests: `tcl-irule-test` `live::tests::{worker_*, rule_init_uses_each_workers_global_cells_once, rule_creation_and_recreation_initialise_only_the_owned_rule_on_four_workers, scheduled_callback*, event_errors_preserve_cells_and_restore_worker_rule_activation, event_frames_preserve_cells_aliases_and_traces, retained_event_frame_preserves_native_completion_options, measured_rule_lookup_uses_partition_root_and_keeps_activated_callables, named_rules_keep_procedures_owned_and_route_call_without_extra_frames, measured_event_names_enter_the_actual_connection_frame_and_restore_policy}`; server `irules::irule6001_*`. On a BIG-IP with at least two TMMs (`tmsh show sys tmm-info`), attach to a virtual server:
    ```tcl
    when RULE_INIT { set static::n 1; log local0. "init tmm=[TMM::cmp_unit] n=$static::n" }
    when CLIENT_ACCEPTED { incr static::n; set ::g $static::n; log local0. "conn tmm=[TMM::cmp_unit] n=$static::n g=$::g"
        after 100 { log local0. "later tmm=[TMM::cmp_unit] n=$static::n" } }
    ```
    Open six connections spread over the TMMs, then re-save the rule and open six more. Confirms the tests: `init` appears once per TMM; each TMM's `n` counts on its own and is never shared; the `later` line runs in the TMM that scheduled it; after the rule is re-saved every TMM logs `init` again and `n` starts from 1.
20. **TS-02, `class match` operators.** Test: `live_session_class_match_honours_every_operator`. Create a string data group `dg` with the records the test names (a hit and a miss), and log `[class match $v equals dg]`, `starts_with`, `ends_with`, `contains`, and `[class match -- $v equals dg]` for a `$v` that begins with `-`. Confirms: each call returns the 0 or 1 the test asserts, and `--` ends the option scan.
21. **TS-03, request lifecycle.** Tests: `live_session_drives_the_request_lifecycle`, `embedded_session_scenarios`, `harness_runs_the_tmm_84_surface`, `tmm_select_mode_survives_the_per_test_reset`; `f5-cli` `explain_flow::simulate_*`. Rule: `when HTTP_REQUEST { if {[HTTP::uri] eq "/r"} { reject } elseif {[HTTP::uri] eq "/h"} { HTTP::respond 200 content ok; pool never_used } else { pool api_pool } }`. Request `/r`, `/h` and `/x`. Confirms: `/r` resets the connection, `/h` answers 200 `ok` and the `pool` after it has no effect, `/x` is balanced to `api_pool`.
22. **TS-04, word boolean operators.** Tests: `tmm_word_boolean_operators_are_rewritten_to_their_tcl_equivalents`; `tcl-cli` `opt_detects_the_irules_dialect_without_the_flag`, `dis_lowers_word_operators_for_the_detected_dialect`. In an event log `[expr {1 and 1}]`, `[expr {1 and 0}]`, `[expr {not 0}]`, `[expr {1 or 0}]`, `[expr {"a" eq "a"}]`. Confirms: 1, 0, 1, 1, 1 (tclsh 8.4 to 9.1 reject the first four as a syntax error or invalid bareword, which is why the rule loader's rewrite exists), so a condition built from them may be folded by the optimiser.
23. **TS-05, string lengths and maths.** Tests: `measured_counted_lengths_keep_dynamic_bytes_and_literal_strings_separate_from_host`, `authored_functions_keep_independent_worker_random_and_precision_contexts`. Log `[string length "😀"]`, `[string length $x]` for the byte strings the test builds, `[expr {abs(077)}]`, `[expr {sqrt(2)}]` per TMM. Confirms: the counts and the doubles the tests assert (the static `4 4 8 3` row for the lengths), the same on every TMM.
24. **TS-06, loader.** Tests: `itest_loader_event_registration_matches_rust_when_blocks`, `measured_loader_checks_original_source_and_keeps_dynamic_runtime_commands`, `scf_loader_stores_parsed_snatpool_members`. Load a rule with `when HTTP_REQUEST priority 100 timing off { ... }` and one with `when CLIENT_ACCEPTED { ... }` plus a computed command head; load an SCF with a `ltm snatpool` block. Confirms: `priority` and `timing` are loader modifiers (not events), the registered events are exactly the `when` events, a computed head is accepted and fails at run time, the snatpool members are stored.
25. **TS-07, procs named like events.** Tests: `f5-xc` `event_shaped_procedure_names_do_not_select_response_event_semantics`; `tcl-diagram` `hosted_diagram_procedures_require_the_current_body_card`. Rule: `proc HTTP_RESPONSE {} { log local0. proc-ran }` and `when HTTP_REQUEST { call HTTP_RESPONSE }`. Confirms: `proc-ran` is logged on each request and no response-event handler is installed by the proc.
26. **TS-08, partition-aware reachability.** Tests: `bigip-report-gen-rust` `certs::orphan_reachability_is_partition_aware`, `constrained_pattern_filters_orphans_by_name`. In `/TenantA` a rule referencing `pool web_b` where `web_b` exists only in `/TenantB`; save and load it. Confirms: the load fails (or the reference is unresolved), so a pool in another partition is not reachable from a rule in `/TenantA` without its full path, and a pool referenced only from a rule of its own partition is not an orphan.
27. **TS-09, the quick-fix claims.** Tests: server `irules::{irule3001_*, irule3002_wrap_uri_encode, irule3004_no_autofix, t101_*, t102_*, t103_*, t106_*}`, `tcl-mcp` `review_reports_the_compiler_check_families`. With `Host: "><b>x` log the response body of `HTTP::respond 200 content "<h1>[HTTP::host]</h1>"` and of the same with an HTML-encoding proc; a redirect with `HTTP::redirect "http://x/[HTTP::uri]"` against `[URI::encode [HTTP::uri]]`; `HTTP::header insert X "a\r\nb"` against the same with CRLF stripped; `regexp [HTTP::uri]` against a quoted pattern; `set static::debug 0` in `RULE_INIT` against `HTTP_REQUEST`. Confirms: the unencoded forms echo the raw value and the encoded forms do not, a CRLF in a header value splits the header, and a `static::` write outside `RULE_INIT` is visible per TMM only, each as the fix text claims.
28. **TS-10, payload round trips.** Tests: server `irules::{payload_string_roundtrip_fires_s110, mqtt_payload_roundtrip_fires_s110, diameter_payload_roundtrip_fires_s110_without_collect_warning}`. Send a body of non-UTF-8 bytes (`\xff\xfe`) through `set p [HTTP::payload]; HTTP::payload replace 0 [string length $p] $p`, and the MQTT and Diameter equivalents. Confirms: the bytes seen by the server differ from what was sent (the round trip through a Tcl string is lossy), which is what S110 warns about; for Diameter, `DIAMETER::payload` needs no `collect` first.

#### Checks for tests that hold iRules rule-load behaviour (test scrub, `class-facts`)

The tests are in `docs/work/test-scrub/ledger/class-facts.tsv`, marked `check CF-nn`. Until a row is answered the test stays and the loader simulation keeps its current answer.

29. **CF-01, a valid `proc` after malformed declarations.** Test: `tcl-compiler` `lowering::tests::irules_events::irules_declaration_body_shape_gates_lowered_regions`. Load as one rule: a `when` with no body, a `proc` with a missing argument list, then `proc valid_proc {} { return ok }` and `when HTTP_REQUEST { log local0. [call valid_proc] }`. Record ACCEPT or REJECT with the loader message; if accepted, send one request. Confirms the test if the rule is rejected as a whole, or if it loads and `call valid_proc` logs `ok` (the test expects the valid procedure to be registered).
30. **CF-02, a `when` nested in a procedure.** Test: `tcl-compiler` `lowering::tests::irules_events::irules_declarations_only_lower_at_the_file_surface`. Load as one rule: `proc helper {} { when HTTP_REQUEST { log local0. nested } ; return ok }` and `when CLIENT_ACCEPTED { log local0. [call helper] }`. Record ACCEPT or REJECT with the loader message; if accepted, open a connection and send a request. Confirms the test if `helper` is registered (CLIENT_ACCEPTED logs `ok`) and the nested `when` registers no HTTP_REQUEST handler (no `nested` line).
