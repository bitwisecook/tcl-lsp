# 12. Plan crosswalk

The source designs proposed plan numbers that collide with each other and
with existing items: 331a–d, 332a–e, 333's 336–344, 334's 340–346, 335's
345–355, 350's 356–365, 360a–f, 361a–d, 365a–f, 370's 370–378, 380's
380–386 and 393's 393a–j and 393x1–x5. Only 301–330 and 390–392 were plan
items. This chapter allocates fresh, contiguous plan numbers from 400, in
dependency order, merges duplicate work, and records the crosswalk. The items
are in [the plan](../../../work/name-handling/plan.md) after item 392, and each
source doc carries a "Plan numbers" note. The [integration design](../../compiler/compiler-analyser-spectcl-integration.md)'s
steps INT-a to INT-m became items 480–487 after the plan review of 26 November
2026, or folded into existing items; that review also split three items and
added two, and the deep review added 106a and 496–498 (*Review renumbering*, below).

## Allocation

| Block | Group | Runs after |
|---|---|---|
| 400–416 | the language: manifest and grammar, notices, hooks by ID, tier-gated spellings, the printer, the parity gate, the registry seam, image trust, identity and composition, dependencies, pack diagnostics, tests and proofs, content forms, load at scale | the registry and compiler splits; 305's reader |
| 417–435 | dialects: grammar profiles, D5, kernels, the writer, engines, conformance, portability, semantic axes, Jim, dialect-aware hooks, f5-tcl severities and rows, per-dialect consumer and codegen rules, the catalogue gate | 301, 303, 304, 418 after 400 |
| 436–441 | consumers: the VM and runtime from facts; name and frame facts, rename, extract and inline, edge cases | 311, 313, 437 after 400 |
| 442–451 | F5: iRules forms, pack, profiles, connection fields, test framework, release facets; iApps, tmsh, the config graph, the config-mode simulator | 304, 400, 118a; 443 performs 330's iRules upgrade |
| 452–457 | SslicTcl | 400; 454 after 307's images |
| 458–464 | eBPF | 307 |
| 465–479 | taint: static (465–474) and run-time (475–479) | 400, 391, 392, 313 |
| 480–487 | the [integration design](../../compiler/compiler-analyser-spectcl-integration.md#10-proposed-plan-steps)'s steps: `FactView`, every image in the catalogue, one `Certainty`, the summary framework, fact slices, grammar profile ids, consumer cones, § 9 as a row. 488–494 are unused; 496–498 are the deep review's new items (*Review renumbering*) | 262, 264, 305, 258, 286, 314 |

```mermaid
flowchart LR
  split["261 to 265 registry split"] --> model["301 to 304"]
  model --> s305["305 reader"]
  s305 --> lang["400 to 416 language"]
  lang --> dia["417 to 435 dialects"]
  lang --> cons["436 to 441 consumers"]
  lang --> f5["442 to 451 F5"]
  lang --> ssl["452 to 457 SslicTcl"]
  lang --> taint["465 to 479 taint"]
  s306["306 spike"] --> s307["307 libraries as data"]
  lang --> s306
  s307 --> ebpf["458 to 464 eBPF"]
  lang --> integ["480 to 487 integration"]
  integ --> taint
  s307 --> ssl
  cons --> waves["319 to 330 consumer waves"]
```

## Merges

| Proposed in two places | Became |
|---|---|
| 333's 336 (manifest, grammar completion) and 332c (reader coverage, grammar gaps) | 400 |
| 332b (tier-gated spellings) and 333's 340 gap 4 (native-contract rows from bundled images) | 403 |
| 333's 339 (`extend`, merge table) and 331d (`extend`, `ProviderRow`) | 409 |
| 333's 342 (`test`, `--prove`) and 335 work item 7 (`example` rows, `prove --consumer`) | 414 |
| 333's 338 `event`/`flow` and 360a | 442 |
| 333's 338 `channel-driver` and 391c | 391c (no new item) |
| 333's 338 `class` and 390a/391a | 391a (no new item) |
| 350's 359 `{*}` axis and character model, and 370's grammar profile | 417 |
| 334 work items 1 and 3 (`library_versions`, `implementation`) and 313 | 313 (no new item) |
| 335's `-appends N` and 390f's `-callback -appends` | 390f (no new item) |
| 390's `throws-prefix` and 392's `throws` | 392a (no new item) |
| INT-d and 308 | 308, as 308a and 308b |
| INT-f and 391b, 392b–c, 393f | 470, with 391b and 392b–c (no new item) |
| INT-i and 318 | 318's done-when |
| INT-k and 380's 386 | 464 |
| INT-l and 393a | 465 |
| 120 (one operation field, one effect descriptor) and 242 (native lowering) | 400 and 402, in the 3.0 vocabulary (D32) |
| 86's vocabulary (object-system profiles, lookup orders) | 391a and 402 (D32); 86 keeps its analysis orders |
| 210 (channel option tables) | 391c (D32) |
| 211 (channel handler protocols) | 390f, with `-callback -protocol` and `required` (D32) |
| 212 (channel instance values) | 391 (D-L); 84 keeps the binding walk, and 16's use-after-`destroy` row moves to 391d |
| 244a (taint sources lost under 9.0 and 9.1) | conformance rows of 465 and 466; verified by 466 |
| 79's grammar axes (operators, numerals, escapes, character model) and 202's generator | 417, in Now 6; 79 keeps the math-function and bundled-package rosters in stage 6 |

## Crosswalk

| Proposed ID | Design | Plan item | Title |
|---|---|---|---|
| 333's 336; 332c | [333](../model/333-spectcl-3-gap-analysis.md) | 400 | The vocabulary manifest and the closed SpecTcl 3 grammar |
| 333's 341 | [333](../model/333-spectcl-3-gap-analysis.md) | 401 | Reader notices with codes and spans, and the `.tclspec` editor |
| 332a | [332](../model/332-rust-spectcl-parity.md) | 402 | Native hooks by ID |
| 332b; 333's 340 (gap 4) | [332](../model/332-rust-spectcl-parity.md) | 403 | Tier-gated spellings |
| 332d | [332](../model/332-rust-spectcl-parity.md) | 404 | The printer over the model |
| 332e | [332](../model/332-rust-spectcl-parity.md) | 405 | The parity gate |
| 333's 344 | [333](../model/333-spectcl-3-gap-analysis.md) | 406 | The registry seam |
| 333's 340 | [333](../model/333-spectcl-3-gap-analysis.md) | 407 | Trust in images |
| 333's 339 (identity) | [333](../model/333-spectcl-3-gap-analysis.md) | 408 | Pack identity and file composition |
| 333's 339 (composition); 331d | [331](../model/331-library-dependencies.md) | 409 | `extend` and provider rows |
| 331a | [331](../model/331-library-dependencies.md) | 410 | Library dependencies |
| 331b | [331](../model/331-library-dependencies.md) | 411 | Dependency rows generated from `pkgIndex.tcl` guards, `ifneeded` chains and each source's top-level and `catch`-wrapped requires, with `--check`; it replaces `TcllibModule.core_floor` |
| 331c | [331](../model/331-library-dependencies.md) | 412 | Positioned dependency closures and diagnostics |
| 333's 337 | [333](../model/333-spectcl-3-gap-analysis.md) | 413 | Pack diagnostics |
| 333's 342; 335 work item 7 | [333](../model/333-spectcl-3-gap-analysis.md) | 414 | Tests in packs and the proof harness |
| 333's 338 (part) | [333](../model/333-spectcl-3-gap-analysis.md) | 415 | Library content forms |
| 333's 343 | [333](../model/333-spectcl-3-gap-analysis.md) | 416 | Load at scale |
| 370's 370; 350's 359 (character model, `{*}`) | [370](../model/370-dialect-grammars.md) | 417 | Grammar profiles in `tcl-dialect` |
| 370's 371 | [370](../model/370-dialect-grammars.md) | 418 | `grammar` blocks in reader3 |
| 370's 372 | [370](../model/370-dialect-grammars.md) | 419 | Pack dialects reach every parser (the redesign's §11 D5) |
| 370's 373 | [370](../model/370-dialect-grammars.md) | 420 | Kernels take their grammar |
| 370's 374 | [370](../model/370-dialect-grammars.md) | 421 | Lexer and segmentation |
| 370's 375 | [370](../model/370-dialect-grammars.md) | 422 | The writer takes the profile |
| 370's 376 | [370](../model/370-dialect-grammars.md) | 423 | Engines read the profile, and a compiled unit records its `GrammarProfileId`; an engine refuses a profile it cannot run instead of running it as Tcl 9 |
| 370's 377 | [370](../model/370-dialect-grammars.md) | 424 | The grammar conformance area (`grammar/<grammar>/<axis>/`) and `cargo xtask grammar-axes --check`, which fails on a value no real run discriminates |
| 370's 378 | [370](../model/370-dialect-grammars.md) | 425 | Portability diagnostics over any grammar axis across a target range |
| 350's 359 (semantic axes) | [350](../model/350-spectcl-across-dialects.md) | 426 | Dialect semantic axes |
| 350's 356 | [350](../model/350-spectcl-across-dialects.md) | 427 | Jim 0.84 full and minimal |
| 350's 357 | [350](../model/350-spectcl-across-dialects.md) | 428 | The Jim core surface generated from jimsh 0.84 probes, as inherited Tcl rows plus measured deltas (`-dialect {jim 0.84}` facets, `mode-default`) |
| 350's 358 | [350](../model/350-spectcl-across-dialects.md) | 429 | Jim extensions as tracks-host libraries with capability predicates, and the 31-name `Extension` |
| 350's 362 | [350](../model/350-spectcl-across-dialects.md) | 430 | Dialect-aware hooks |
| 350's 360 | [350](../model/350-spectcl-across-dialects.md) | 431 | f5-tcl load-time severities |
| 350's 361 | [350](../model/350-spectcl-across-dialects.md) | 432 | Versioned iRules command rows from evidence, starting with `proc` and `call`, for the BIG-IP releases in public support per K5902 only |
| 350's 363 | [350](../model/350-spectcl-across-dialects.md) | 433 | Consumer rules per dialect |
| 350's 364 | [350](../model/350-spectcl-across-dialects.md) | 434 | Codegen per dialect |
| 350's 365 | [350](../model/350-spectcl-across-dialects.md) | 435 | The catalogue refusal gate (an environment may not place a library with no row for its family), and the F5-bundled iApp packages as libraries with unknown versions (Q12) |
| 334 work items 2, 4, 5 and 6 | [334](../model/334-spectcl-codegen-vm-runtime.md) | 436 | The VM and runtime from facts |
| 335 work items 1–2 | [335](../model/335-spectcl-source-tools.md) | 437 | Name and frame facts in the registry and reader |
| 335 work item 3 | [335](../model/335-spectcl-source-tools.md) | 438 | Name and frame rows on the core and Tk surfaces |
| 335 work item 4 | [335](../model/335-spectcl-source-tools.md) | 439 | Rename on the word classifier; `prepareRename` returns `Abstain` with its reason |
| 335 work item 5 | [335](../model/335-spectcl-source-tools.md) | 440 | Extract and inline on `-scope caller` and `-frame`, then change signature (through `-appends`) and move to namespace |
| 335 work item 6 | [335](../model/335-spectcl-source-tools.md) | 441 | Formatter, minifier and optimiser edge cases, row by row from 335's table |
| 360a; 333's 338 (event, flow) | [360](../model/360-spectcl-f5-integration.md) | 442 | iRules forms in the reader and manifest |
| 360b | [360](../model/360-spectcl-f5-integration.md) | 443 | The iRules pack |
| 360c | [360](../model/360-spectcl-f5-integration.md) | 444 | Profile resolution through `schema-kind` in `tcl-bigip`, and `HostBinding` in the positioned context |
| 360d | [360](../model/360-spectcl-f5-integration.md) | 445 | Connection fields and form effects (`reads`/`writes connection`, `decision`, `ends`, `emits`), read by the flow walk, the diagram, `explain_flow` and MCP |
| 360e | [360](../model/360-spectcl-f5-integration.md) | 446 | The iRules test framework from facts |
| 360f | [360](../model/360-spectcl-f5-integration.md) | 447 | iRules release facets from the Q6 audit, one probe batch at a time, for K5902 lines only |
| 361a | [361](../model/361-spectcl-f5-iapps-tmsh.md) | 448 | iApps forms |
| 361b | [361](../model/361-spectcl-f5-iapps-tmsh.md) | 449 | The `tmsh-command` role, `ConfigEffect`, tmsh transactions, and the resolver contribution in `tcl-bigip` |
| 361c | [361](../model/361-spectcl-f5-iapps-tmsh.md) | 450 | The symbolic config graph, iApp-created `HostBinding`s, the missing-reference check and the cross-domain rename |
| 361d | [361](../model/361-spectcl-f5-iapps-tmsh.md) | 451 | The simulator's config mode |
| 365a | [365](../model/365-spectcl-sslictcl.md) | 452 | SslicTcl forms in the manifest |
| 365b | [365](../model/365-spectcl-sslictcl.md) | 453 | The generic declaration reader in `tcl-spectcl`, on `tcl-source` |
| 365c | [365](../model/365-spectcl-sslictcl.md) | 454 | `tcl-lib-sslictcl` |
| 365d | [365](../model/365-spectcl-sslictcl.md) | 455 | SslicTcl vocabulary 2 data, the Rust trust generator, and the compile step in `make update-source-data` and `check-source-data` |
| 365e | [365](../model/365-spectcl-sslictcl.md) | 456 | `tcl-sslictcl` builds `SslicModel` from `DeclDocument` and the images; documents migrate to vocabulary 2, and a lower header is `SSLIC1013` |
| 365f | [365](../model/365-spectcl-sslictcl.md) | 457 | SslicTcl consumers |
| 380 | [380](../model/380-spectcl-ebpf.md) | 458 | eBPF packs |
| 381 | [380](../model/380-spectcl-ebpf.md) | 459 | `compile ebpf` rows |
| 382 | [380](../model/380-spectcl-ebpf.md) | 460 | `EbpfFacts` and `KernelAbi` |
| 383 | [380](../model/380-spectcl-ebpf.md) | 461 | eBPF analysis |
| 384 | [380](../model/380-spectcl-ebpf.md) | 462 | eBPF `test` rows and the Tcl model as a runner, with privileged rows behind the ignore gate |
| 385 | [380](../model/380-spectcl-ebpf.md) | 463 | eBPF explorer views (`ebpf-ir`, `ebpf-asm`, `verifier`, `chain`) and CLI and MCP routing |
| 386 | [380](../model/380-spectcl-ebpf.md) | 464 | Drop `bpf-tcl-ir`'s `tcl-analysis` and `tcl-native-plan` edges; wire or delete the semantic bridge (open decision D19) |
| 393a | [393](../model/393-taint.md) | 465 | The taint colour table |
| 393b | [393](../model/393-taint.md) | 466 | Sink kinds |
| 393c | [393](../model/393-taint.md) | 467 | Origins and secrets |
| 393d | [393](../model/393-taint.md) | 468 | Taint spelling and parity |
| 393e | [393](../model/393-taint.md) | 469 | Taint validators |
| 393f | [393](../model/393-taint.md) | 470 | Gen/kill taint summaries and pack-declared transfers |
| 393g | [393](../model/393-taint.md) | 471 | Taint gates |
| 393h | [393](../model/393-taint.md) | 472 | The optimiser taint barrier |
| 393i | [393](../model/393-taint.md) | 473 | Package and F5 taint |
| 393j | [393](../model/393-taint.md) | 474 | Taint consumers |
| 393x1 | [393](../model/393-taint.md) | 475 | Run-time taint tag |
| 393x2 | [393](../model/393-taint.md) | 476 | Run-time taint sinks at dispatch |
| 393x3 | [393](../model/393-taint.md) | 477 | Run-time taint propagation |
| 393x4 | [393](../model/393-taint.md) | 478 | The taint warn-mode conformance lane |
| 393x5 | [393](../model/393-taint.md) | 479 | Taint oracles |
| INT-a | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 480 | `FactView` and `HookDispatch` in the registry core |
| INT-b | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 481 | Every fact an image, every image in the catalogue |
| INT-c | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 482 | One `Certainty`, the projection of item 78's `Knowledge<T>` ([chapter 3](03-semantics.md#certainty)) |
| INT-d | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 308 (308a, 308b) | The positioned context in `tcl-binding` |
| INT-e | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 483 | The summary framework |
| INT-f | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 470 | The new domains (with 391b and 392b–c) |
| INT-g | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 484 | Fact slices and pack inputs |
| INT-h | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 485 | Grammar profile ids |
| INT-i | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 318 | The VM boundary |
| INT-j | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 486 | Consumer cones |
| INT-k | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 464 | `bpf-tcl-ir` without `tcl-analysis` |
| INT-l | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 465 | One taint colour, one completion code value |
| INT-m | [integration](../../compiler/compiler-analyser-spectcl-integration.md) | 487 | § 9 as a conformance row |
| 333's 336–344, 334's 340–346, 335's 345–355, 350's 356–365, 370's 370–378, 380's 380–386 as numbers | — | retired | the numbers are not plan items; the rows above map their work |

The model documents keep their own numbers (`331-library-dependencies.md`
is still "331"); only proposed plan numbers are retired. Items 301–330 and
390–392 keep their numbers. Item 305's plan text stays, marked as superseded in
detail by this specification.

## Review renumbering (26 November 2026)

The plan review split three items so that their build-time halves do not wait
for slow work, and added two. The lettered halves are bullets inside their
item, as 117a and 117b are, except 244a, which is its own row.

| Before | After | Why |
|---|---|---|
| 118 | 118a (the data move, with the registry split), 118b (the generated model and the 216 comparisons, with 119) | the build win does not wait for the generator |
| 308 | 308a (whole-file selection, after 253, 302 and 303), 308b (per position, after item 84) | 308 had been scheduled before item 84, which it needs |
| 318 | 318a (`tcl-spec-hooks` behind a non-default feature, now), and the rest with 305–307 | a build win that needs no reader |
| none | 244a | 393's bug 1: taint sources lost under `tcl9.0` and `tcl9.1` |
| item 90's step 14 | 495 | the caching step had no item number |
| INT-a to INT-m | 480–487, or folded (table above) | the integration design's steps |
| 106 | 106 (the single budget table and its CI gate, stage 13), 106a (its harness, Now 1 beside 259) | the deep review's F1: every split step reports latency against one baseline, and the absolute gate does not hold Now 7 |
| none | 496 | packaging on SpecTcl 3 facts (319's consumer 14), Now 8 wave 7 |
| none | 497 | AGENTS.md § Invariants for the target (deep review O13), Now 6, gating nothing |
| none | 498 | the `tcl-bytecode` split into `tcl-bytecode-tables` and `tcl-bytecode`, Now 3 beside 252 (a build win) |

## Example: one plan row and its crosswalk entry

**Source:**

```text
413. **[designed]** (with 305's `check`; blocked by D9 and D10 ([chapter 11](../../design/registry/spectcl-3/11-open-decisions.md)); assumes their recommendations; …; was 333's 337; [design](../../design/registry/model/333-spectcl-3-gap-analysis.md), [spec](../../design/registry/spectcl-3/05-checks-hooks-abi.md)) Pack diagnostics. …
   - Done when: 305's four example rules load with pack codes, a user setting changes one's severity, and the editor catalogues list pack codes at run time.
```

**In memory:**

```rust
// rust/xtask/src/plan_rows.rs parses each row into its Item (real fields)
Item { number: 413, status: "designed", text: "413. **[designed]** (with 305's `check`; blocked by D9 and D10 …" }
// the crosswalk row, as this chapter's table states it
("333's 337", "333-spectcl-3-gap-analysis.md", 413, "Pack diagnostics")
```

`cargo xtask plan-rows` reads every row with the pattern `^(\d+)\.
\*\*\[([a-z ]+)\]\*\*` (`rust/xtask/src/plan_rows.rs`); the new items sit
outside the working order's migration stages (stages 6–13), so it asks no
conformance rows of them yet. A lettered row such as `244a.` does not match the
pattern, so the tool skips it. A plan row has no bytecode or Tcl result.

## Where they run in the working order

The plan's working order has eight Now stages, ahead of its stages 1–13:
build and iteration wins; the registry split; the compiler split's lower
crates; binding and the passes; the top of the stack and the rest of the
workspace; the dialect and library model with SpecTcl 3; the analysis
framework; the consumers. The SpecTcl 3 items run in the last three:

- **Now 6:** 400–411, 413, 415 and 416 with 305's reader and the
  expect spike, 400 and 402 carrying 120's and 242's vocabulary (D32); 465–468; 482; 481 with 307; the dialect items 417–420,
  424–432 and 435, with 202's generator and 79's grammar axes before 301
  and 417. None of these gates a crate extraction in Now 1–5.
- **Now 7:** 469–471 with 483, 484, 485 and 487.
- **Now 8:** 414, 433, 436, 392f, 441 and 472 beside 313; 496 in the last wave; 437–440; 442–451 (443
  performs 330's iRules upgrade); 452–457; 458–464; 473 and 474; 486.
- **Waiting:** 412 after 308b, so after item 84; 421 with item 41 and 422 with
  item 39.
- **Deferred with the engine items:** 423, 434 and 475–479.

## Review fixes (26 November 2026)

- **P7:** INT-a, b, c, e, g, h, j and m allocated as 480–487; INT-d, f, i, k
  and l folded into 308, 470, 318, 464 and 465; the "not yet committed" text
  is gone.
- **P1, P10, P11, P12:** *Review renumbering* records 118a/118b, 308a/308b,
  318a, 244a and 495; 452–464 no longer name a "DSL step" 307 does not have.
- **P2, P5:** 443 performs 330's iRules upgrade; *Where they run* follows the
  plan's Now stages.
- **P9:** 419's D5 is the redesign's §11 D5, not chapter 11's.

## Review fixes (26 November 2026, deep review)

- **M4.** INT-c's `Certainty` is `Knowledge<T>`'s projection.
- **W2, W3, W4, W10, W12.** Merges for 120/242, 86, 210, 211, 212, 244a and 79/202; Now 6 carries D32 and the generator, and none of it gates Now 1–5.
- **Plan pass.** *Review renumbering* adds 106a, 496, 497 and 498; *Where they run* puts 392f beside 313 (D-J) and 496 in Now 8.
