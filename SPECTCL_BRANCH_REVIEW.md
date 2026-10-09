REWORK

# Temporary branch review report

**Worker instruction: delete `SPECTCL_BRANCH_REVIEW.md` from this branch before opening the PR to `rust`. Commit the deletion so this report is absent from the final PR diff.** This file is a temporary hand-off, not repository documentation.

Reviewed branch: `claude/spectcl-optimization-discussion-5qhf42` in `bitwisecook/tcl-lsp`. Reviewed source tip: `f82eaefbdc5bce8b817f62bf67b900300fb096c8`. Merge base and refreshed `origin/rust`: `6d214350b9f899a1399fa841be7f6fc5edbf225a`. The report commit is later than the reviewed source tip and changes only this file. Review date: 9 October 2026.

The branch needs rework: three declared-implementation boundaries publish incorrect facts and change deterministic output, the required Spec Studio formatting gate fails, and the new contract pages contain false statements about the current implementation. The runtime API discrepancies and surviving guard mutation should also be addressed. Source fixes, pushes, issue writes and PR writes were outside this review. The user's final hand-off instruction expressly authorised committing this report despite the pasted brief's earlier review-only/no-commit restriction.

The report follows the requested categories: B is blocking, S should be fixed before merging, N is a nit, and P is pre-existing. Known issues #2381–#2441 (with #2409 withdrawn), plus the specified older exclusions, were read before triage. A different declared-capability trigger of a known error class is explicitly labelled as a new angle. No excluded original trigger is repeated as a new defect. Documentary findings do not imply that the corresponding newly implemented folding behaviour is unsound.

Paths and source line numbers below refer to the reviewed source tip unless a base path is named. Scratch paths show the exact executed commands and are not committed artefacts. Each substantive finding contains its inputs and evidence here so the recipient does not need access to the review machine. Quoted tool output retains its original spelling. The review is finite adversarial evidence, not a proof that every invocation is sound.

## B — Blocking findings

| ID | Confirmed problem | Base attribution |
|---|---|---|
| B1 | Declared and reference-derived implementations accept extra arguments | New implementation route |
| B2 | A raising unread argument is turned into successful catch facts | New declared-route angle on #2389 |
| B3 | A declared binding dependency is used after rebinding | New declared-route angle on #2423/#2398 |
| B4 | Spec Studio's added editor code fails pinned Prettier | Base file passes |
| B5 | Contract, examples and migration pages describe a different tree | New pages |
| B6 | I230 note says a certain assignment prevents the fold | Added sentence; tip performs the fold |
| B7 | Suppression pages promise retained W305 evidence which is discarded | New reporting contract; filter pre-exists |
| B8 | Regexp KCS misstates C Tcl's algorithm and named cost witness | New KCS page |

For every pack-probe table in B1–B3 and S1, all ten recorded CLI invocations and all 75 interpreter executions exit 0 with empty stderr. The explicit Full repeat described in the run log gives identical output and status.


### B1 — Declared implementations ignore resolved maximum arity

**Reject invalid resolved arity before evaluating declared implementations.** For both an explicit implementation and a reference-body-derived implementation, `review::label acme extra` violates `arity 1`. Tip nevertheless proves the catch status `0` and result `acme`, and O100 rewrites the reads after the catch. Every reference shell and base output report status `1` and `wrong # args: should be "review::label name"`. Corrective intent: use the resolved command/subcommand/form’s effective `Arity::accepts` before publishing an evaluator answer; preserve the conservative/raised completion path for invalid arity, including maximum, stepped, and selected-form constraints. This should cover both authored and derived capabilities.

   The resolver already carries effective arity in [resolved_invocation.rs](/workspace/tcl-lsp/rust/tcl-registry/src/resolved_invocation.rs:617). Compiler [view_of](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:5816) projects a [ResolvedInvocationView](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/inputs.rs:265) with operands and argument offset but no arity. [run_command](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:3897) resolves the call and invokes [route_answer](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:3978) without checking that arity. Registry [evaluate_implementation](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:394) checks expansion and only resolves declared inputs at line 421: [input_text](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:328) rejects a missing requested argument, but does not reject additional arguments. [outcome](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:553) publishes `CompletionOutcome::Normal` at line 615. The derived path [declared_for](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/reference_body.rs:826) creates one exact operand input per reference parameter at line 840, so it shares this omission.

`/tmp/spectcl-review/registry-repros/arity/.tcl-lsp/arity.tclspec`:

```tcl
speclib review 2.2 {
  command review::label {
    arity 1
    semantics { effects {no_store_writes no_external_io}; result -semantic string }
    evaluate -implementation review.label.v1 -host bounded_tcl {
      inputs {arg 0 exact}
      depends {tcl_profile implementation_identity}
      body {name} {fold $name}
    }
  }
}
```

`/tmp/spectcl-review/registry-repros/arity/caught.tcl`:

```tcl
set code [catch {review::label acme extra} message]
puts $code
puts $message
```

`/tmp/spectcl-review/registry-verification/arity-harness.tcl`:

```tcl
namespace eval review {}
proc review::label {name} {return $name}
source [lindex $argv 0]
```

```bash
cd /tmp/spectcl-review/registry-repros/arity
/tmp/spectcl-review/binaries/tip-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/arity/caught.tcl > /tmp/spectcl-review/registry-verification/arity-tip.tcl
/tmp/spectcl-review/binaries/base-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/arity/caught.tcl > /tmp/spectcl-review/registry-verification/arity-base.tcl
```

Tip tool stdout:

```tcl
set code [catch {review::label acme extra} message]
puts 0
puts acme


# -------------
# optimised: 2 rewrite(s)
# O100  Inline the constant value of 'code' proved at this read
# O100  Inline the constant value of 'message' proved at this read
```

Base tool stdout:

```tcl
set code [catch {review::label acme extra} message]
puts $code
puts $message
```

```bash
for version in 8.4 8.5 8.6 9.0 9.1; do
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/arity-harness.tcl /tmp/spectcl-review/registry-repros/arity/caught.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/arity-harness.tcl /tmp/spectcl-review/registry-verification/arity-tip.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/arity-harness.tcl /tmp/spectcl-review/registry-verification/arity-base.tcl
done
```

| C Tcl | Original stdout | Tip output stdout | Base output stdout |
|---|---|---|---|
| 8.4 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 8.5 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 8.6 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 9.0 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 9.1 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |

Base attribution: this input’s source rewrite and resulting output discrepancy are absent in the compared base. The new `declared.rs`, `reference_body.rs`, and compiler `value_transfer.rs` are added files in `git diff --name-status 6d214350b9f899a1399fa841be7f6fc5edbf225a` at the reviewed tip.

#### derived-arity: Reference-derived implementation ignores maximum arity

`/tmp/spectcl-review/registry-repros/derived-arity/.tcl-lsp/derived.tclspec`:

```tcl
speclib review 2.2 {
  command review::label {
    arity 1
    runtime_backing tcl-body {-pack-text {proc review::label {name} {return $name}} -evaluate}
  }
}
```

`/tmp/spectcl-review/registry-repros/derived-arity/caught.tcl`:

```tcl
set code [catch {review::label acme extra} message]
puts $code
puts $message
```

`/tmp/spectcl-review/registry-verification/derived-arity-harness.tcl`:

```tcl
namespace eval review {}
proc review::label {name} {return $name}
source [lindex $argv 0]
```

```bash
cd /tmp/spectcl-review/registry-repros/derived-arity
/tmp/spectcl-review/binaries/tip-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/derived-arity/caught.tcl > /tmp/spectcl-review/registry-verification/derived-arity-tip.tcl
/tmp/spectcl-review/binaries/base-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/derived-arity/caught.tcl > /tmp/spectcl-review/registry-verification/derived-arity-base.tcl
```

Tip tool stdout:

```tcl
set code [catch {review::label acme extra} message]
puts 0
puts acme


# -------------
# optimised: 2 rewrite(s)
# O100  Inline the constant value of 'code' proved at this read
# O100  Inline the constant value of 'message' proved at this read
```

Base tool stdout:

```tcl
set code [catch {review::label acme extra} message]
puts $code
puts $message
```

```bash
for version in 8.4 8.5 8.6 9.0 9.1; do
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/derived-arity-harness.tcl /tmp/spectcl-review/registry-repros/derived-arity/caught.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/derived-arity-harness.tcl /tmp/spectcl-review/registry-verification/derived-arity-tip.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/derived-arity-harness.tcl /tmp/spectcl-review/registry-verification/derived-arity-base.tcl
done
```

| C Tcl | Original stdout | Tip output stdout | Base output stdout |
|---|---|---|---|
| 8.4 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 8.5 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 8.6 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 9.0 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |
| 9.1 | `"1\nwrong # args: should be \"review::label name\"\n"` | `"0\nacme\n"` | `"1\nwrong # args: should be \"review::label name\"\n"` |

Base attribution: this input’s source rewrite and resulting output discrepancy are absent in the compared base. The new `declared.rs`, `reference_body.rs`, and compiler `value_transfer.rs` are added files in `git diff --name-status 6d214350b9f899a1399fa841be7f6fc5edbf225a` at the reviewed tip.


### B2 — A raising unread argument becomes a successful invocation

**Carry argument-substitution failure through the implementation route even when the implementation does not read that operand.** `review::constant [error boom]` cannot invoke the command: Tcl evaluates the argument first. With `inputs {}`, tip proves a successful catch and result `answer`, while every shell and base output print catch status `1` and `boom`. Corrective intent: validate the completion of all Tcl words before invoking any declared evaluator; a capability’s input subset controls value reads, not whether Tcl argument evaluation can be omitted. Preserve the pre-invocation completion and its prefix writes, or decline conservatively before accepting `Normal`.

   The source has an important nuance: [finite_inputs](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/lift.rs:131) queries every operand at lines 143–145 but its `note` closure records only finite facts, discarding `Top`/`Pending`. [evaluate_lifted](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/lift.rs:174) then calls the evaluator. The implementation’s input loop at [declared.rs:421](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:421) is empty in this case and [outcome:615](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:615) returns `Normal`. The driver recognizes a raising word in [substituted_in](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:6379) at lines 6411–6417, but [carrying_word_writes](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:6105) carries state writes and binding evidence rather than enforcing the word’s failed completion; [call_defs](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:2648) also only converts `word_error` to raised definitions when the answer itself declined at lines 2683–2685. The observed error is therefore loss of the argument completion, not a claim that no component ever queried the argument.

   This is a new declared-implementation/O100 catch-result angle on the known argument-evaluation class in [#2389](https://github.com/bitwisecook/tcl-lsp/issues/2389), whose recorded trigger is O103’s procedure-summary path. It is not evidence that the excluded O103 issue was newly introduced.

`/tmp/spectcl-review/registry-repros/ignored-argument/.tcl-lsp/ignore.tclspec`:

```tcl
speclib review 2.2 {
  command review::constant {
    arity 1
    semantics {effects {no_store_writes no_external_io}; result -semantic string}
    evaluate -implementation review.constant.v1 -host bounded_tcl {
      inputs {}
      depends {tcl_profile implementation_identity}
      body {} {fold answer}
    }
  }
}
```

`/tmp/spectcl-review/registry-repros/ignored-argument/caught.tcl`:

```tcl
set code [catch {review::constant [error boom]} message]
puts $code
puts $message
```

`/tmp/spectcl-review/registry-verification/ignored-argument-harness.tcl`:

```tcl
namespace eval review {}
proc review::constant {value} {return answer}
source [lindex $argv 0]
```

```bash
cd /tmp/spectcl-review/registry-repros/ignored-argument
/tmp/spectcl-review/binaries/tip-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/ignored-argument/caught.tcl > /tmp/spectcl-review/registry-verification/ignored-argument-tip.tcl
/tmp/spectcl-review/binaries/base-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/ignored-argument/caught.tcl > /tmp/spectcl-review/registry-verification/ignored-argument-base.tcl
```

Tip tool stdout:

```tcl
set code [catch {review::constant [error boom]} message]
puts 0
puts answer


# -------------
# optimised: 2 rewrite(s)
# O100  Inline the constant value of 'code' proved at this read
# O100  Inline the constant value of 'message' proved at this read
```

Base tool stdout:

```tcl
set code [catch {review::constant [error boom]} message]
puts $code
puts $message
```

```bash
for version in 8.4 8.5 8.6 9.0 9.1; do
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/ignored-argument-harness.tcl /tmp/spectcl-review/registry-repros/ignored-argument/caught.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/ignored-argument-harness.tcl /tmp/spectcl-review/registry-verification/ignored-argument-tip.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/ignored-argument-harness.tcl /tmp/spectcl-review/registry-verification/ignored-argument-base.tcl
done
```

| C Tcl | Original stdout | Tip output stdout | Base output stdout |
|---|---|---|---|
| 8.4 | `"1\nboom\n"` | `"0\nanswer\n"` | `"1\nboom\n"` |
| 8.5 | `"1\nboom\n"` | `"0\nanswer\n"` | `"1\nboom\n"` |
| 8.6 | `"1\nboom\n"` | `"0\nanswer\n"` | `"1\nboom\n"` |
| 9.0 | `"1\nboom\n"` | `"0\nanswer\n"` | `"1\nboom\n"` |
| 9.1 | `"1\nboom\n"` | `"0\nanswer\n"` | `"1\nboom\n"` |

Base attribution: this input’s source rewrite and resulting output discrepancy are absent in the compared base. The new `declared.rs`, `reference_body.rs`, and compiler `value_transfer.rs` are added files in `git diff --name-status 6d214350b9f899a1399fa841be7f6fc5edbf225a` at the reviewed tip.


### B3 — Declared binding dependencies are not checked against the application

**Validate declared binding dependencies before using an implementation result.** The implementation declares `depends {tcl_profile implementation_identity binding string}`. The analysed application renames `string` and installs a procedure returning `rebound`. At tip the bounded implementation still computes builtin length `3`; O100 rewrites `puts $x` to `puts 3`. Every reference shell and base output print `rebound`. Corrective intent: check each declared binding against the application’s command-trust snapshot and resolution namespace before evaluating or accepting cached evidence; refuse a dependency whose expected binding no longer holds, with its actual identity in invalidation/evidence. Checking only the invoked private command is insufficient. This is the binding-validity contract that ruling 3 explicitly retains for authoritative workspace facts; no provenance cap or certification requirement is needed.

   Loader [depends_row](/workspace/tcl-lsp/rust/tcl-spectcl/src/loader/semantics.rs:926) constructs a literal `BindingIdentity` from the declared name at lines 933–936. [evaluate_implementation](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:394) has no binding-admission check. [outcome](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:606) copies those declared bindings into evidence, and compiler [run_command](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:3906) checks only `head` through [trusted](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:2550). [route_answer](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:4018) accepts the implementation’s lifted outcome; [fold_cmd_subst_routes](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:3804) turns its exact normal result into a lattice constant without validating those declared dependencies. Hook cache [CallContent](/workspace/tcl-lsp/rust/tcl-registry/src/pack_hooks.rs:1323) retains and compares the declaration’s dependency literals at lines 1368 and 1405; that is not a check of their current application binding.

   This is a new declared-capability angle on the known binding-validity class around [#2423](https://github.com/bitwisecook/tcl-lsp/issues/2423) (ensemble implementation rebinding through O129) and [#2398](https://github.com/bitwisecook/tcl-lsp/issues/2398) (definition order through O103). These known records have different triggers and compiler routes. The comparison establishes that this particular declared-implementation rewrite is absent in base; it does not claim that all binding bugs are new.

`/tmp/spectcl-review/registry-repros/binding/.tcl-lsp/binding.tclspec`:

```tcl
speclib review 2.2 {
  command review::length {
    arity 1
    semantics {effects {no_store_writes no_external_io}; result -semantic int}
    evaluate -implementation review.length.v1 -host bounded_tcl {
      inputs {arg 0 exact}
      depends {tcl_profile implementation_identity binding string}
      body {value} {fold [string length $value]}
    }
  }
}
```

`/tmp/spectcl-review/registry-repros/binding/repro.tcl`:

```tcl
rename string saved_string
proc string args {return rebound}
set x [review::length abc]
puts $x
```

`/tmp/spectcl-review/registry-verification/binding-harness.tcl`:

```tcl
namespace eval review {}
proc review::length {value} {return [string length $value]}
source [lindex $argv 0]
```

```bash
cd /tmp/spectcl-review/registry-repros/binding
/tmp/spectcl-review/binaries/tip-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/binding/repro.tcl > /tmp/spectcl-review/registry-verification/binding-tip.tcl
/tmp/spectcl-review/binaries/base-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/binding/repro.tcl > /tmp/spectcl-review/registry-verification/binding-base.tcl
```

Tip tool stdout:

```tcl
rename string saved_string
proc string args {return rebound}
set x [review::length abc]
puts 3


# -------------
# optimised: 1 rewrite(s)
# O100  Inline the constant value of 'x' proved at this read
```

Base tool stdout:

```tcl
rename string saved_string
proc string args {return rebound}
set x [review::length abc]
puts $x
```

```bash
for version in 8.4 8.5 8.6 9.0 9.1; do
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/binding-harness.tcl /tmp/spectcl-review/registry-repros/binding/repro.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/binding-harness.tcl /tmp/spectcl-review/registry-verification/binding-tip.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/binding-harness.tcl /tmp/spectcl-review/registry-verification/binding-base.tcl
done
```

| C Tcl | Original stdout | Tip output stdout | Base output stdout |
|---|---|---|---|
| 8.4 | `"rebound\n"` | `"3\n"` | `"rebound\n"` |
| 8.5 | `"rebound\n"` | `"3\n"` | `"rebound\n"` |
| 8.6 | `"rebound\n"` | `"3\n"` | `"rebound\n"` |
| 9.0 | `"rebound\n"` | `"3\n"` | `"rebound\n"` |
| 9.1 | `"rebound\n"` | `"3\n"` | `"rebound\n"` |

Base attribution: this input’s source rewrite and resulting output discrepancy are absent in the compared base. The new `declared.rs`, `reference_body.rs`, and compiler `value_transfer.rs` are added files in `git diff --name-status 6d214350b9f899a1399fa841be7f6fc5edbf225a` at the reviewed tip.


### B4 — Spec Studio fails the required formatting gate

The branch's added `optionEffectEditor` is not formatted with the pinned Prettier, so the `web-frontends` job fails before its production build and unit tests.

Location: `rust/tcl-spec-studio/web/src/editors.ts`, `makeEditors` / nested `optionEffectEditor`, line 685 and lines 692–694 at the reviewed tip. The first call and the array `.map` need the wrapping the repository's formatter emits. No source fix was applied.

Exact reproduction, with Node `v24.19.0` and npm `12.2.0` (Corepack `0.34.6`, using each package's committed `packageManager` pin):

```sh
cd rust/tcl-spec-studio/web
npm ci
cd ../../..
mkdir -p build/stamps
touch build/stamps/spec-studio-npm-install
make typecheck-spec-studio-ts lint-spec-studio-ts spec-studio-assets spec-studio-test
```

Result: `npm ci` succeeds, type-check succeeds, ESLint succeeds, Prettier fails, Make exits 2. Exact gate excerpt:

```text
cwd: /workspace/tcl-node-ci
command: make typecheck-spec-studio-ts lint-spec-studio-ts spec-studio-assets spec-studio-test
==> Type-checking spec studio front-end with tsc
cd /workspace/tcl-node-ci/rust/tcl-spec-studio/web && npm run typecheck
npm notice run @tcl-spec-studio/web@0.0.0 typecheck
npm notice run tsc --noEmit -p tsconfig.json && tsc --noEmit -p tsconfig.test.json
==> Linting spec studio front-end (ESLint + Prettier check)
cd /workspace/tcl-node-ci/rust/tcl-spec-studio/web && npm run lint
npm notice run @tcl-spec-studio/web@0.0.0 lint
npm notice run eslint src test --ext .ts && prettier --check "src/**/*.ts" "test/**/*.ts"
Checking formatting...
[warn] src/editors.ts
[warn] Code style issues found in the above file. Run Prettier with --write to fix.
make: *** [Makefile:776: lint-spec-studio-ts] Error 1
```

Attribution: introduced by the branch. `git diff 6d214350b..f82eaefbd -- rust/tcl-spec-studio/web/src/editors.ts` shows that `optionEffectEditor` is added by the branch. `git diff` for the Studio `.prettierrc.json` and `package-lock.json` is empty, so the formatter version and configuration are identical at base and tip. Pinned Prettier `3.9.9` checked the real base file using its own committed configuration:

```sh
cd /workspace/tcl-lsp-review-base
node /workspace/tcl-node-ci/rust/tcl-spec-studio/web/node_modules/prettier/bin/prettier.cjs --check rust/tcl-spec-studio/web/src/editors.ts
```

This command exits 0:

```text
Checking formatting...
All matched files use Prettier code style!
```

The base file was also extracted with `git show 6d214350b:rust/tcl-spec-studio/web/src/editors.ts` and checked with the tip's identical explicit `--config`; that independently exits 0. Formatting the tip to `/tmp/spectcl-review/node-tip-editors-formatted.ts` without editing the checkout produces precisely the two wrapping differences logged in `node-logs/tip-editors-format-diff.log`.

Known-issue comparison: searched the collected titles in `/tmp/spectcl-review/known-issues.md` (issues 2381–2441 and cited older issues) and `/tmp/spectcl-review/older-known-issues.json` for `prettier|format|editors.ts|studio|front`; no matching title. This failure is not a duplicate of an identified standing issue. Tcl reference outputs and CLI binaries are not applicable to this formatting gate.


### B5 — Contract and worked-example pages describe an obsolete or different tree

The value-transfer design pages are new on this branch but contain current-state claims contradicted by the tip's code and, in several cases, by another section of the same page. These are documentary reproductions: read the quoted lines and the named definitions; no speculative runtime claim is required.

| Claim at tip | Reproduction against current code/page | Attribution |
|---|---|---|
| `docs/design/compiler/value-transfers.md:1357–1366`, Existence: “Today the fact is two whole-body scans — `scan_defined_and_unset` … and `existence_constant_branches` … outside the fixed point”, and any assigned name never folds. | `rg -n -e scan_defined_and_unset -e existence_constant_branches rust/tcl-compiler/src` finds neither. `sccp.rs:1082–1099` constructs the existence rung beside the value fixed point, and `SccpResult` holds existence maps at lines335–381. The same page says those two functions “are gone” at1605–1607. | Page absent at base. The named obsolete functions exist in base `sccp.rs:1024,1188`; branch changed implementation without removing its old description. |
| `docs/design/compiler/value-evaluation.md:187–200`, What exists: `fold_range` is an ASCII-only second implementation and there is “no live” string-backed `ValueOps`. | `commands/tcl/string_.rs:243–251`, `fold_range`, delegates to `value_transfer::evaluate_literal(&builtins::STRING_RANGE,...)`; `value_transfer/const_ops.rs:400` defines live `ConstOps`, and840 implements `ValueOps` for it. | Page absent at base; base `fold_range` contains the ASCII algorithm the page describes. |
| `value-evaluation.md:504–507`: `tcl opt` folds nothing for `string range abcdefghijkl 010 end` “under every dialect”. | Exact repro below: tip writes `puts ijkl` for tcl8.4/8.5/8.6 and `puts kl` for tcl9.0/9.1. Every oracle prints that same corresponding string. Base leaves the call unchanged for all five. | Branch-introduced false documentation; branch's new folding behaviour is sound in the measured program. |
| `value-evaluation.md:935–937`: `TclVmEngine` is the only `Engine` implementation and exposes no release setter. | `tcl-engine-tclvm/src/lib.rs:514,657`, `impl Engine for TclVmEngine` / `set_release`; `tcl-engine-wasm/src/lib.rs:371,502`, `impl Engine for WasmEngine` / `set_release`. Page's own1240ff documents `Engine::set_release`. | Page absent base; neither the WASM crate nor `Engine::set_release` exists at base. |
| `value-evaluation.md:850–858,1674,1696–1698,1838–1845,1855–1857`: core refuses `regexp -about`; route declines it. | `tcl-cmd-core/src/regex.rs:829–850` handles `if about` and returns `RegexpResult::Inline([count,flags])`; `regexp_about_reports_the_subexpression_count_and_info_list` at1880 pins `1 {}` for `a(b)c`; `value_transfer/regex.rs:237–268`, `RegexpSemantics::evaluate_regexp`, explicitly consumes `-about` through `regexp_analysis`. Page's core-table row561 accurately says it evaluates through the core. | Page absent base. Core `-about` support already exists at base `regex.rs:390` with its test1144, so this is an introduced false statement about an existing capability. |
| `value-evaluation.md:1935,1942–1949`: `bounded_tcl` is the only host word; an implementation has four rows and no others. | `value_transfer/route.rs:192–215`, `HostKind`, has `BoundedTcl` and `WasmExtension`; `loader/semantics.rs:702–719` resolves both, and779–786 parses the additional `extension FILE PREFIX` row. `a_wasm_extension_implementation_names_its_artefact` at1314 pins it. Page's own1200ff extension-host section documents this exact grammar. | Page absent base; extension host grammar new on branch. |
| `value-transfers.md:2788–2795`: “Today each surface assembles its own subset” and the common policy pipeline is a remaining gap. | `rust/tcl-lsp-core/src/diagnostic_policy.rs:1582`, `apply`, is the current common policy owner; `diagnostic_report.rs:137` delegates to it; CLI `commands/diag.rs:342` calls `standalone_findings`; server `lib.rs:18202,32253` calls `core_policy::apply`. `diagnostic-policy.md` declares this implementation built. | Page absent base; common owner is new on branch. |

These locations establish one blocking documentation finding; they do not assert implementation defects. Replace the obsolete current-state paragraphs with the actual owner/API and retain only current limitations.

#### Exact CLI/oracle repro for the `010` statement

`/tmp/spectcl-review/docs-old-index.tcl`:

```tcl
set s [string range abcdefghijkl 010 end]
puts $s
```

For each `D` in `tcl8.4 tcl8.5 tcl8.6 tcl9.0 tcl9.1`:

```sh
/workspace/tcl-lsp/target/debug/tcl opt /tmp/spectcl-review/docs-old-index.tcl --profile full --dialect D
/workspace/tcl-lsp-review-base/target/debug/tcl opt /tmp/spectcl-review/docs-old-index.tcl --profile full --dialect D
/tmp/spectcl-review/tcl-reference-bin/tclshRELEASE /tmp/spectcl-review/docs-old-index.tcl
```

| Target/oracle patchlevel | Tip first source line | Base first source line | Original oracle output |
|---|---|---|---|
| 8.4 /8.4.20 | `puts ijkl` | `set s [string range abcdefghijkl 010 end]` | `ijkl` |
| 8.5 /8.5.19 | `puts ijkl` | same unchanged call | `ijkl` |
| 8.6 /8.6.18 | `puts ijkl` | same unchanged call | `ijkl` |
| 9.0 /9.0.4 | `puts kl` | same unchanged call | `kl` |
| 9.1 /9.1.0 | `puts kl` | same unchanged call | `kl` |

Tip complete9.0 rewrite is `puts kl`, followed by summary “optimised: 2 rewrite(s)”, O109 eliminate dead store/O100 inline constant `s`. No miscompile.

The registry-consumer design page contains the following confirmed contradictions. These locations establish a shared stale-current-state finding.

| Page location | False statement | Current tree and reproduction | Base attribution |
|---|---|---|---|
| `registry-consumer-contracts.md:352–355`, repeated `2130–2136` | The intrinsic and ABI descriptor catalogues are generated from the registry by an `xtask` build task. | `rust/tcl-runtime-api/src/codegen_abi.rs:96`, `:307`, `:371` author `CodegenAbiImportId`, `ALL`, and the descriptor match directly; `:537–546` builds an array from those descriptors at const evaluation. `rust/tcl-registry/src/intrinsic.rs:61`, `:102`, `:229` author the intrinsic revision table, enum and `ALL`. `rg -n -e intrinsic -e codegen_abi -e codegen-abi -e Intrinsic rust/xtask/src` returns no matches. `rust/xtask/src/command_backing.rs:60`, `:408`, `:555` generates only the Markdown backing report. | Page is entirely new on this branch (`git diff --numstat` says 2607 added, 0 removed). The manually authored ABI table also exists at the base (`CodegenAbiImportId:96`, `descriptor:302`). |
| `registry-consumer-contracts.md:539–543` | `runtime/rust/src/capi.rs` has 20 no-mangle C functions. | The repository's own `find_capi_exports` finds **41** no-mangle functions; its C-API classification finds **35**. `check_c_extension_wasm.header_legs` finds 33 WASM extern declarations plus the two refcount macros; native exports are 36. | New claim on branch, line last touched `2d570cf7bd`. |
| `registry-consumer-contracts.md:607` | There are 32 `AnalyserHookId` variants and `Set` remains among three residual hooks. | Enum in `rust/tcl-registry/src/hooks.rs:292–427` has **31** variants and no `Set`. `rust/tcl-registry/tests/analyser_hooks.rs:102–106` explicitly records 31 variants/42 stamp rows. Count reproduced with a Python extraction of the enum's comma-terminated variants. | Base has 43 variants and `Set`; retirement commit `c387a2ce5` removed `Set`. New page claim, line last touched `8c6d37d5d6`. |
| `registry-consumer-contracts.md:1772` | “evaluators exist; the answer protocol does not”. | `rust/tcl-registry/src/value_transfer/answers.rs:1011` defines `EvalAnswer::{Pending, Declined, Evaluated}`; `ValueSemantics::evaluate` in `value_transfer/mod.rs:192` returns it; the implementations and transfer driver use it. Page's own lines269–271 also name this existing type. | Page absent at base; statement last touched `fa9a420156`. |
| `registry-consumer-contracts.md:2425–2432` | Nothing derives command facts from C source and users describe native commands manually. | `scan_c_source` in `rust/tcl-spec-studio/src/infer/c_scan.rs:279` and `import_c_sources` read registrations and evidence; CLI route `rust/tcl-cli/src/commands/spec.rs:649`, `:722–753` renders the pack. **Repro:** `target/debug/tcl spec import --c-source rust/tcl-cshim/tests/c --entry Pkga --out /tmp/spectcl-review/doc-pkga.tclspec` exits 0 and says `pkga: 5 command(s) described (5 from the C source, 0 from the probe); 0 computed registration(s), 0 call(s) the scan cannot read`. `Loaded::declared_surface` also exists at `rust/tcl-cshim/src/lib.rs:95`. | New page paragraph, last touched `b0f0b432b1`; scanner/import added by `ca5cc7295`. |
| `registry-consumer-contracts.md:2447`, `:2564` | Native shim has 34 / 32 exported symbols; the file anchor says the engine interface has one implementation. | `check_c_extension_wasm.shim_exports` extracts **36** `export_name` functions from `rust/tcl-cshim/src/ffi.rs`; the page itself correctly says36 at line525. Interface implementations include `TclVmEngine`, `tcl_runtime::engine::RuntimeEngine` and `tcl_engine_wasm::WasmEngine`, all also described on this page. | New page claims; diagram last touched `822db6ee63`; file anchor `2d570cf7bd`. |

Suggested correction: state these current facts directly, remove the stale status/counter snapshots, and restrict the generator claim to the generated backing Markdown report.

The page tells contributors that every proposed block is merely a sketch and “none of it is loader syntax yet” (`value-transfers-examples.md:1419–1421`; also :18–19). In the current tree, `semantics`, `evaluate` and `facts` are loader statements, and these routes are implemented. Consequently several purported current-state descriptions are false:

- :1444–1446: “what is missing is the specialisation that turns it into a value transfer”. `rust/tcl-registry/src/value_transfer/cell_update.rs:280–334` implements `CommandSemantics`, reports a direct route, produces `PlanAnswer::CellReadModifyWrite`, type/range/existence facts, and evaluates the update. `incr_.rs:97–103` also has no `analyser_hook: Some(AnalyserHookId::Incr)` that the claimed shipped spec excerpt at :1438–1439 includes.
- :1541–1544: the page calls `string range`'s `const_fold` “an ASCII-only re-implementation beside `tcl_cmd_core::string::range`” and says `binary format` has “no evaluator”. `string_.rs:243–256` instead calls `value_transfer::evaluate_literal` over `builtins::STRING_RANGE`; its subcommand at :1458–1460 declares that semantics and `fold_range_unanimous`. `binary_.rs:216` declares `builtins::BINARY_FORMAT`.
- :1628–1629 says expression evaluation lives in the compiler's `cmd == "expr"` arm and numeric-only `FoldOps`. `expr_.rs:50` declares `builtins::EXPR`; `sccp.rs:4551–4565` delegates expression statements to the value-transfer driver. The named compiler name recogniser is absent.
- :1664–1665 says every `regexp`, `scan` and `lassign` target is a definite definition, presenting the already repaired #2051 as current. `regexp_.rs:307` declares the registry regexp owner, and the value-transfer outcome models `Preserve` (`value_transfer/regex.rs:275–278` calls `Publication::preserving` on no-match); current CLI evidence is recorded below.
- :1812–1813 says switch selection semantics are implemented three times in the compiler. The named private O112 matchers `resolve_subject` / `pattern_matches` are absent from `optimiser/structure_elimination.rs`, which reads the solver's selected/applied facts.
- :1925–1940 says `subst_.rs` still declares `substitution_resolver: Some(crate::substitution::subst_substitutions)` and only proposes the template plan. That field is absent from the spec; :329 declares `SemanticsDeclaration::Declared(&TEMPLATE)`, with authored option-effect families at :295 and the reserved trailing operand at :311.

Independent loader reproduction: `rust/tcl-spectcl/src/loader/semantics.rs:126–152` explicitly reads the three keywords. The shipped `tenant.tclspec:22–37` and `specs/sdc_base.tclspec:244–255` already author these declarations. The latter is even printed as “Built” later on the same examples page (:2002 onward), contradicting the blanket introduction.

Attribution: both design pages were added by this branch (`git cat-file -e BASE:PATH` fails for both), so these false descriptions are branch-introduced. No Tcl oracle can adjudicate whether an identifier exists in Rust; this is reproduced by exact tree/code inspection.

#### The migration page presents removed compiler functions and deleted-but-live modules as the current tree

The current inventory and structural-gap discussion contain several reproducible contradictions:

- :54–64 and :92–100 describe `try_fold_cmd_subst`'s name arms, `scan_defined_and_unset`, a lattice in which every call other than `foreach`/`lmap` is `Overdefined`, a shared lattice built without `BuiltinFoldInputs`, and a whole-variable `Raw` subject which cannot be evaluated. The same stale function identifiers recur in the file-path anchor at :714. `rg -n 'try_fold_cmd_subst|scan_defined_and_unset|existence_constant_branches' rust/tcl-compiler/src` yields no match. `sccp.rs:4562–4563` dispatches every call to `driver.evaluate_call`; :4565–4578 delegates a typed increment to the driver. `compilation_unit.rs:959–967` supplies `BuiltinFoldInputs` under `ObservedBindings` to the shared unit run. This intentionally has `registry_engine: false`, but the declared routes are on; the page's blanket absence claim and diagram are wrong.
- :57 and :714 name `scan_defined_and_unset`; :54 and :714 name `try_fold_cmd_subst`; the latter anchor also names `existence_constant_branches`. All are absent. These are not labelled removed-history anchors; they are offered as file-path anchors to the built implementation.
- :106 calls `folded_types` “the proposed side map”. `sccp.rs:312` already declares the field.
- :291–292 says the sixty-two hand-written sites become consumers “once [the interface] exists”. `rust/tcl-registry/src/value_transfer/mod.rs` and the driver already exist.
- :429–431 says `var_escape/handlers.rs` and the test-only half of `var_escape/cfg_propagation/handlers.rs` “are deleted, not migrated”. Both files exist (601 and 565 lines). `var_escape/mod.rs:40` exports `handlers`, and `var_escape/walker.rs:35` imports it; `cfg_propagation/mod.rs:31` exports its handlers and its walker at :40 imports them. The same page's own ratchet row :662 expressly admits “the walker still calls the file, so it is reviewed, not deleted”.

Attribution: branch-introduced page, absent at the merge base. The 198-code catalogue count was checked and is correct; the ratchet table is not being alleged wrong on an unmeasured count.

Five independent negative claims now contradict the CLI at the reviewed tip. These are documentary defects, not new compiler soundness defects. All runs below use `--dialect tcl8.6`; every reference shell was run on the unchanged source, and the reported facts agree with those shells. The two pages do not exist at the base, but the base CLI has the claimed old behaviour, showing that the examples failed to follow the implementation.

#### `examples:875` — “today: nothing” after the computed invalid IP

```tcl
set addr 10.0.0
append addr .256
puts $addr
```

Exact command: `/workspace/tcl-lsp/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-computed-ip.tcl --dialect tcl8.6 --json`. Exit 1.

Tip findings:

- `W124` line 2, column 1: IPv4 octet 4 (256) exceeds 255 — this is not a valid IP address.

Base command: `/workspace/tcl-lsp-review-base/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-computed-ip.tcl --dialect tcl8.6 --json`. No diagnostics.

Oracle commands are `/tmp/spectcl-review/tcl-reference-bin/tclshVERSION /tmp/spectcl-review/docs-repros/examples-computed-ip.tcl` for VERSION 8.4, 8.5, 8.6, 9.0 and 9.1. All exit 0, with empty stderr. All give exactly the same stdout:

```text
10.0.0.256
```

Optimiser command: `/workspace/tcl-lsp/target/debug/tcl opt /tmp/spectcl-review/docs-repros/examples-computed-ip.tcl --dialect tcl8.6 --profile full`, exit 0. Full output is in `docs-repros/results.json`. This finding concerns the diagnostic claimed absent in the page; no unsound rewrite is alleged.

#### `examples:688,691` — “today: nothing — the enumeration runs no `[…]` amount”

```tcl
set x 0
foreach {a b} {1 10 2 20} { incr x [expr {$b / $a}] }
if {$x == 20} {puts twenty} else {puts other}
set y 0
foreach {a b} {1 20 2 10} { incr y [expr {$b / $a}] }
if {$y == 25} {puts twentyfive} else {puts other}
```

Exact command: `/workspace/tcl-lsp/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-correlated-loops.tcl --dialect tcl8.6 --json`. Exit 0.

Tip findings:

- `I230` line 3, column 4: Condition '$x == 20' is always true; the alternate branch is unreachable
- `I230` line 6, column 4: Condition '$y == 25' is always true; the alternate branch is unreachable

Base command: `/workspace/tcl-lsp-review-base/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-correlated-loops.tcl --dialect tcl8.6 --json`. No diagnostics.

Oracle commands are `/tmp/spectcl-review/tcl-reference-bin/tclshVERSION /tmp/spectcl-review/docs-repros/examples-correlated-loops.tcl` for VERSION 8.4, 8.5, 8.6, 9.0 and 9.1. All exit 0, with empty stderr. All give exactly the same stdout:

```text
twenty
twentyfive
```

Optimiser command: `/workspace/tcl-lsp/target/debug/tcl opt /tmp/spectcl-review/docs-repros/examples-correlated-loops.tcl --dialect tcl8.6 --profile full`, exit 0. Full output is in `docs-repros/results.json`. The tip changes each proven condition to `0` or `1` (O101) and removes its dead arm (O107); the base emits the original source with no such rewrite.

#### `examples:1020,1024–1027` — “today: nothing” and “nothing decides the condition” after an unset; named old scan is absent

```tcl
proc p {} {
    set x 1
    unset x
    if {[info exists x]} {puts yes} else {puts no}
}
p
```

Exact command: `/workspace/tcl-lsp/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-exists-unset.tcl --dialect tcl8.6 --json`. Exit 0.

Tip findings:

- `I230` line 4, column 8: Condition '[info exists x]' is always false; the alternate branch is unreachable

Base command: `/workspace/tcl-lsp-review-base/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-exists-unset.tcl --dialect tcl8.6 --json`. No diagnostics.

Oracle commands are `/tmp/spectcl-review/tcl-reference-bin/tclshVERSION /tmp/spectcl-review/docs-repros/examples-exists-unset.tcl` for VERSION 8.4, 8.5, 8.6, 9.0 and 9.1. All exit 0, with empty stderr. All give exactly the same stdout:

```text
no
```

Optimiser command: `/workspace/tcl-lsp/target/debug/tcl opt /tmp/spectcl-review/docs-repros/examples-exists-unset.tcl --dialect tcl8.6 --profile full`, exit 0. Full output is in `docs-repros/results.json`. The tip changes each proven condition to `0` or `1` (O101) and removes its dead arm (O107); the base emits the original source with no such rewrite.

#### `examples:913` — “today: no W233 … the diagnostic lattice does not [fold z]”

```tcl
proc p {x} {
    set z [string range 1000 3 3]
    return [expr {$x / $z}]
}
puts [catch {p 1} msg]
puts $msg
```

Exact command: `/workspace/tcl-lsp/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-refined-divisor.tcl --dialect tcl8.6 --json`. Exit 1.

Tip findings:

- `W233` line 3, column 5: Division by a provably-zero divisor — raises 'divide by zero' at runtime.

Base command: `/workspace/tcl-lsp-review-base/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-refined-divisor.tcl --dialect tcl8.6 --json`. Only S100: variable 'z' has string intrep used in arithmetic expression (operand of '/').

Oracle commands are `/tmp/spectcl-review/tcl-reference-bin/tclshVERSION /tmp/spectcl-review/docs-repros/examples-refined-divisor.tcl` for VERSION 8.4, 8.5, 8.6, 9.0 and 9.1. All exit 0, with empty stderr. All give exactly the same stdout:

```text
1
divide by zero
```

Optimiser command: `/workspace/tcl-lsp/target/debug/tcl opt /tmp/spectcl-review/docs-repros/examples-refined-divisor.tcl --dialect tcl8.6 --profile full`, exit 0. Full output is in `docs-repros/results.json`. This finding concerns the diagnostic claimed absent in the page; no unsound rewrite is alleged.

#### `examples:957` — “today: nothing — the prover bails on metacharacters”

```tcl
proc p {} {
    regexp {(x)(y)} zz a b
    puts "$a $b"
}
puts [catch {p} msg]
puts $msg
```

Exact command: `/workspace/tcl-lsp/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-regexp.tcl --dialect tcl8.6 --json`. Exit 1.

Tip findings:

- `W210` line 3, column 11: Variable 'a' is read before it is set
- `W210` line 3, column 14: Variable 'b' is read before it is set

Base command: `/workspace/tcl-lsp-review-base/target/debug/tcl diag /tmp/spectcl-review/docs-repros/examples-regexp.tcl --dialect tcl8.6 --json`. No diagnostics.

Oracle commands are `/tmp/spectcl-review/tcl-reference-bin/tclshVERSION /tmp/spectcl-review/docs-repros/examples-regexp.tcl` for VERSION 8.4, 8.5, 8.6, 9.0 and 9.1. All exit 0, with empty stderr. All give exactly the same stdout:

```text
1
can't read "a": no such variable
```

Optimiser command: `/workspace/tcl-lsp/target/debug/tcl opt /tmp/spectcl-review/docs-repros/examples-regexp.tcl --dialect tcl8.6 --profile full`, exit 0. Full output is in `docs-repros/results.json`. This finding concerns the diagnostic claimed absent in the page; no unsound rewrite is alleged.


#### Corpus outcome exclusions

The remaining cases were checked without inventing new findings: the fused-length and string-building-proc examples still retain their documented limitation; the list/lindex results gain facts or hint counts while their source substitutions remain, overlapping the already known unapplied-rewrite problem (#2406/#2439). The binary example still emits the original source despite its declared evaluator existing; No additional outcome defect was reproduced for that fixture.


### B6 — The I230 code note contradicts certain preceding assignments

`docs/kcs/codes/kcs-diagnostic-i230-constant-existence-check.md:61–65` states: “What still stops the fold is an assignment that can actually reach the check on some path: a set that runs before it with nothing between them that undoes it”. A certain preceding assignment makes `info exists` certainly true; it does not stop constant folding. The first half of this paragraph should distinguish an uncertain assignment on only some paths from a definite assignment.

Exact reproduction (`/tmp/spectcl-review/docs-repros/kcs-exists-set.tcl`):

```tcl
proc p {} {
    set handle 1
    if {[info exists handle]} {puts yes} else {puts no}
}
p
```

Command: `target/debug/tcl diag /tmp/spectcl-review/docs-repros/kcs-exists-set.tcl --dialect tcl8.6 --json`.

Actual current output: I230 at line 3, column 8, “Condition '[info exists handle]' is always true; the alternate branch is unreachable”. Every reference Tcl prints `yes`. Base CLI reports no diagnostics on the same program. Every pinned reference shell from 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 exits 0, stdout `yes\n`, empty stderr. Full tip/base/oracle outputs are in the completed corpus JSON.

Attribution: this sentence is added by the branch's diff. The preceding-set flow-sensitive implementation is also branch work; the KCS paragraph must describe that present behaviour.


### B7 — The suppression contract promises evidence which W305 discards

`docs/kcs/kcs-qa-where-is-diagnostic-policy-applied.md:18–24` says every producer filters nothing and nothing is deleted; `:54–56` promises every suppressed finding in `--show-suppressed`. `docs/design/compiler/diagnostic-policy.md:50–58`, `:956–960` state the same absolute contract. Yet the analyser applies W305 suppression before it constructs the report. `rust/tcl-compiler/src/analyser/source_integrity.rs::bidi_control_diagnostics_with_suppressions` (`:57–75`) filters a `# noqa`-suppressed W305 away; the report cannot record a finding or a gap for that line. The design page itself accurately acknowledges this exception at `:224–225`, but its absolute rules and KCS answer contradict it.

Create the exact input with a literal U+202E:

```sh
python3 - <<'PYCODE'
from pathlib import Path
p = Path('/tmp/spectcl-review')
p.mkdir(exist_ok=True)
(p / 'bidi-noqa.tcl').write_text('# noqa: W305\nputs "' + chr(0x202e) + 'x"\n')
(p / 'bidi-unsuppressed.tcl').write_text('puts "' + chr(0x202e) + 'x"\n')
PYCODE
```

The resulting source is:

```tcl
# noqa: W305
puts "‮x"
```

Tip `tcl diag /tmp/spectcl-review/bidi-noqa.tcl --dialect tcl9.0 --json --show-suppressed` exits0 with `diagnostics: []` and a `suppressed` array containing only the optimiser's not-run row. **No W305 entry or gap**. Remove the first line: `bidi-unsuppressed.tcl` exits1 and reports W305 at1:7. This is a policy/reporting contract check; reference execution is immaterial because the source control itself, rather than Tcl runtime semantics, is the producer's subject.

Base: `--show-suppressed` is unavailable (exit2, unexpected argument), so the exact reporting promise is new. Without that flag, the base also suppresses W305 (exit0/empty diagnostics) and reports it in the control (exit1/W305), and its identical source_integrity filter is present. Suggested correction for a documentation-only remedy: qualify the universal promise and name the current W305 exception consistently; the implementation remedy is to emit the raw finding and let policy own it.


### B8 — The regexp KCS attributes catastrophic backtracking to C Tcl incorrectly

`docs/kcs/compiler/kcs-qa-why-does-a-regexp-sometimes-not-fold.md:20–24` says regular-expression cost is exponential on every Tcl release, including the engine tclsh links against, and `(a+)+b` on text without `b` is its catastrophic-backtracking example. The actual C Tcl extent search is a lazy DFA (`tmp/tcl9.0.4/generic/regexec.c:34,330–336`; `rege_dfa.c`), whereas our own matcher explicitly distinguishes regular NFA-set extent search from its backreference backtracking path (`rust/tcl-regex/src/exec.rs:19–34`). The page therefore attributes the custom engine's bounded-cost limitation to a different matcher. The evaluation design's own witness at827–838 also says the corresponding anchored no-match returns in under2ms on all five Tcl versions.

Exact measured programme `/tmp/spectcl-review/docs-regexp.tcl`:

```tcl
puts "patchlevel [info patchlevel]"
foreach count {300 10000} {
 set s [string repeat a $count]
 puts "count $count result [regexp {(a+)+b} $s] timing [time {regexp {(a+)+b} $s} 10]"
}
```

Command: `for release in 8.4 8.5 8.6 9.0 9.1; do timeout 10 /tmp/spectcl-review/tcl-reference-bin/tclsh$release /tmp/spectcl-review/docs-regexp.tcl; done` exits0.

All results are0. Microseconds per iteration for300/10,000 `a` characters: 8.4.20 **1.3/55.5**,8.5.19 **1.3/28.4**,8.6.18 **1.5/28.2**,9.0.4 **1.6/36.0**,9.1.0 **1.9/41.2**. This finite run is not a general complexity theorem; the claim is contradicted by the matcher implementation and the named concrete witness. Fix the explanation to identify tcl-regex's work cap separately from C Tcl's algorithm. The KCS page is absent at base (introduced by branch).

Additional tip CLI probe on the literal 300-a programme `/tmp/spectcl-review/docs-regexp-program.tcl` with `--profile full --dialect tcl9.0` leaves `[regexp {(a+)+b} $s]` in the source. `tcl explore /tmp/spectcl-review/docs-regexp-program.tcl --show sccp --text` exited 0 and reports `route regexp: direct regexp-match (registry)` / `answer: declined: approximate`. Thus the issue is the page's stated C Tcl explanation; the custom analyser limitation itself is accurately observed. Do not report its O102 summary/text discrepancy: the exclusion ledger already covers optimiser candidate-count issues.


## S — Should fix before merging


### S1 — Declared direct IDs are advertised but never dispatched

**Dispatch a declared direct ID to its named registry evaluator.** The accepted `evaluate -direct ListOfArgs` declaration is explained as `direct list-of-args (registry)` but declines `unsupported` for two exact literal arguments. Tip and base both leave the application source unchanged; all shells produce `selected` and `a b`. This is a capability/implementation mismatch and missed fold, with no output discrepancy observed. Corrective intent: route a declared direct ID to its registry-owned specialisation, while preserving declaration option declines, selected-form argument offsets and structural facts.

   [DeclaredSemantics::route](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:695) returns the authored route, but [DeclaredSemantics::evaluate](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/declared.rs:711) returns `Unsupported` for every declared named route except `None` at line 724. The comment at line 719 says the driver runs a named route itself. Driver [route_answer](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:3985) instead passes the same declaration into `evaluate_lifted`, so the ID is not used to select [LIST_OF_ARGS](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/builtins.rs:278), whose [evaluate](/workspace/tcl-lsp/rust/tcl-registry/src/value_transfer/builtins.rs:305) implements canonical list rendering. The store-definition dispatch [call_defs](/workspace/tcl-lsp/rust/tcl-compiler/src/value_transfer.rs:2656) follows the same interface shape.

`/tmp/spectcl-review/registry-repros/direct/.tcl-lsp/direct.tclspec`:

```tcl
speclib review 2.2 {
  command review::list {
    arity 0..
    semantics { effects {no_store_writes no_external_io}; result -semantic list }
    evaluate -direct ListOfArgs
  }
}
```

`/tmp/spectcl-review/registry-repros/direct/repro.tcl`:

```tcl
proc direct_test {} {
  set x [review::list a b]
  if {$x eq {a b}} {puts selected} else {puts unreachable}
  return $x
}
```

`/tmp/spectcl-review/registry-verification/direct-harness.tcl`:

```tcl
namespace eval review {}
proc review::list args {return $args}
source [lindex $argv 0]
puts [direct_test]
```

```bash
cd /tmp/spectcl-review/registry-repros/direct
/tmp/spectcl-review/binaries/tip-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/direct/repro.tcl > /tmp/spectcl-review/registry-verification/direct-tip.tcl
/tmp/spectcl-review/binaries/base-dev opt --dialect tcl8.6 --no-colour /tmp/spectcl-review/registry-repros/direct/repro.tcl > /tmp/spectcl-review/registry-verification/direct-base.tcl
```

Tip tool stdout:

```tcl
proc direct_test {} {
  set x [review::list a b]
  if {$x eq {a b}} {puts selected} else {puts unreachable}
  return $x
}
```

Base tool stdout:

```tcl
proc direct_test {} {
  set x [review::list a b]
  if {$x eq {a b}} {puts selected} else {puts unreachable}
  return $x
}
```

```bash
for version in 8.4 8.5 8.6 9.0 9.1; do
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/direct-harness.tcl /tmp/spectcl-review/registry-repros/direct/repro.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/direct-harness.tcl /tmp/spectcl-review/registry-verification/direct-tip.tcl
  /tmp/spectcl-review/tcl-reference-bin/tclsh"$version" /tmp/spectcl-review/registry-verification/direct-harness.tcl /tmp/spectcl-review/registry-verification/direct-base.tcl
done
```

| C Tcl | Original stdout | Tip output stdout | Base output stdout |
|---|---|---|---|
| 8.4 | `"selected\na b\n"` | `"selected\na b\n"` | `"selected\na b\n"` |
| 8.5 | `"selected\na b\n"` | `"selected\na b\n"` | `"selected\na b\n"` |
| 8.6 | `"selected\na b\n"` | `"selected\na b\n"` | `"selected\na b\n"` |
| 9.0 | `"selected\na b\n"` | `"selected\na b\n"` | `"selected\na b\n"` |
| 9.1 | `"selected\na b\n"` | `"selected\na b\n"` | `"selected\na b\n"` |

Base attribution: both tools preserve source and all executions agree. Tip newly reports a named declared direct route but fails to execute it. The recorded explorer `/tmp/spectcl-review/registry-repros/direct/explore.json` contains `review::list`, `direct list-of-args (registry)`, `declined: unsupported`; the subsequent condition declines `not-exact`. The source-output comparison and an independent explicit-SCCP Explorer repeat support this result; the repeated command and exact text appear below.

Exact route inspection (exit 0):

```sh
cd /tmp/spectcl-review/registry-repros/direct
/tmp/spectcl-review/binaries/tip-ci explore /tmp/spectcl-review/registry-repros/direct/repro.tcl --dialect tcl8.6 --show sccp --text --no-colour
```

The relevant exact output is:

```text
    ├── route review::list: direct list-of-args (registry)
    │   · answer: declined: unsupported
    │   · line: 2
```

This was independently repeated using the saved fast binary; preserved original/rewritten output is shown above. An earlier default JSON invocation did not expose this trace; the working-directory/explicit-SCCP text invocation above is the evidence for the quoted route.


### S2 — The new package-provide C API loses the caller's result

At `runtime/rust/src/capi.rs:731`, `Tcl_PkgProvideEx` delegates to the Tcl command helper `provide_package`. Its two successful paths clear the result (`runtime/rust/src/cmd_package.rs:204` and `:212`). C Tcl's C API preserves the current interpreter result on success.

A fixed native C API probe sets the result to `preserved`, successfully provides package `review 1.0`, then repeats with result `again`. Every available C Tcl reference (8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0) answers `0 | preserved` and `0 | again`. This branch's runtime answers status 0 with zero result bytes both times. The verified operation loses the result the caller set before providing the package. This is an API consistency finding; no Tcl optimiser miscompile is asserted. The command-level `package provide` still should return empty; preserve the existing result at the C API boundary or split the shared helper's result policy.

Base attribution: `Tcl_PkgProvideEx` is absent from base `runtime/rust/src/capi.rs`; this is a defect in the newly implemented surface. No matching known issue was present in `/tmp/spectcl-review/known-issues.json`.

The complete native probe, C Tcl harness, build commands and five-release results are in the runtime reproduction appendix below.


### S3 — The new confinement capability permits global array creation and unsets

The new interface promises to refuse every store outside the procedure's frame (`rust/tcl-engine-api/src/lib.rs:531`). Both implementations return `Ok(())` for this capability but allow a global array to be created and a host-provided global scalar to be deleted.

The fixed probe first seeds `::seed`, applies `restrict_commands(["set", "array", "unset", "return", "info"])`, then calls `confine_stores`. `array set ::fresh {}` returns normally and an independent next invocation observes `info exists ::fresh` as 1. `unset ::seed` returns normally and the next invocation observes 0. The probe therefore follows the documented restriction-before-confinement sequence.

The native runtime's array materialisation goes directly through `Interp::ensure_array` (`runtime/rust/src/interp.rs:4220`) and `array_set` (`runtime/rust/src/cmd_array.rs:419`), bypassing the new store check. `unset` calls `var_unset*` without that check (`runtime/rust/src/builtins.rs:666`). The VM reproduces both operations as well.

Scope: this is a general Engine capability-contract failure on the caller's explicitly allowed command surface. The pack host's current `SANDBOX_COMMANDS` excludes `array` and `unset`; this witness does **not** show a leak through that pack whitelist or a subject-program miscompile.

Base attribution: `confine_stores` is absent from the base Engine trait and the base TclVmEngine implementation. Existing array/unset code becomes incorrectly advertised as confined through the new capability. No matching known issue was present.

The complete capability probe and measured outputs are in the runtime reproduction appendix below.


### S4 — A declared-write guard is not pinned by the registry suite

A mutation removing the rejection of `OutcomeKind::Write` without a published value survives all 1,048 registry library tests and all 80 `value_transfers` integration tests. The production guard is present; this is a confirmed test-coverage gap, not a claim that the unmutated tree accepts that invalid outcome.

Location: `rust/tcl-registry/src/value_transfer/declared.rs`, `outcome`. In the isolated exact-tip worktree, replace:

```rust
(_, Some(OutcomeKind::Unbind) | None) | (None, Some(OutcomeKind::Write)) => {
```

with:

```rust
(_, Some(OutcomeKind::Unbind) | None) => {
```

Commands (incremental disabled, own target):

```sh
cd /workspace/tcl-lsp-registry-review
source /tmp/spectcl-review/build-env.sh
cargo test -p tcl-registry --lib --test value_transfers -- --nocapture
```

Both baseline and mutant exit 0; all 1,128 tests pass in each. Restore the original file; the focused declared-unit baseline passes again. The second sampled mutation, changing the spec-hooks silent-target `if silent || unstated` guard to `if silent && unstated`, is killed by `a_silent_target_declines_the_whole_answer` on both engines, exits 101, and passes after restoration. `git diff --exit-code` verifies the isolated worktree is clean afterwards. Further targeted mutation results are recorded in the contract appendix.

Base attribution: the declared implementation surface and guard are branch additions; no equivalent base pin exists. A Tcl interpreter cannot judge a deliberately mutated Rust structural-validation guard. The exact mutation diff and full logs were retained in the review workspace; the reproduced change and suite outcomes above are self-contained.


### S5 — Current-state documentation retains development and proposal narration

The brief explicitly makes development/history/revision/before-after narration a finding. The locations below cover the required pages.

Confirmed locations in the two value-contract design pages (line ranges identify paragraphs, not runtime sequencing language):

- `value-transfers.md:15–20` still calls companion a migration plan and “what changes”;50–88 owner decisions include “First delivery”;218–224 thread-local availability “is replaced”;1357–1366 old current state;1434–1450 old cross-event post-pass and “today both blind”;1496–1500 paths “become one”;1536–1540 “Until #2220” before/after bug narrative;1562–1563 retirement;1598–1607 “fixed additions” plus removed functions;1611–1626 inventories formerly independent consumers;1720–1727 “What it stops doing” table;1734–1742 “fixed additions”;1762–1777 adapter “limits … corrected rather than inherited”;2028–2044 “Two gaps close”;2574–2581 “fixed additions”;2746–2747 “boundaries strengthened, not replaced”;2788–2795 obsolete remaining gap;2799–2824 producer recognised names/private prover “gone”/O111 “no longer”;3042–3050 semantic sites “become consumers”;3105 entire “fixed witnesses to add” bullet.
- `value-evaluation.md:35–36` no builtin “has been moved”;43–56 “not built”/shape “tree built”/`ActivationStore` “superseded before it was built”;61–64 revision-checked identifiers;201–203 folds codegen “once held”;230 Mermaid `ConstOps (proposed)` despite live400/840 type;248–259 `fold_range becomes`/helpers “retire”/tests “replaced”;603–612 proposed-core table includes built `binary::format_size_bound`;629–630 “Three changes made”;668–674 table “went onto”;747–818 regexp result “becomes”/“gains”/provider “to update”/“contract change … precedes”;984–1019 isolation alternatives rejected/measured prior cached answer/new trait method;1021–1062 “needed their own rule”, VM “now”, contract “originally read” quoted prior description;1264–1289 original argument type/byte-identical old families;1394–1458 “closes both gaps”, “new for evaluate”, proc key “gains”;1522–1524 “three separate types did not survive contact with tree”;1609–1619 measured prior hook workloads/unchanged-tree acceptance;1720–1741 loader/renderer “moved together”/“minor after2.1”;2016–2019 prior spelling ambiguity/example “now spells”;2059–2069 plan sentence not what built;2104–2121 renderer gap and fields “new”;2130–2145 carry-forward closure/try-it “not built”;2173 codegen folders “to retire”;2193 tests “gains”;2202–2209 fixed witnesses “to add”.
- KCS `compiler/kcs-qa-why-does-a-constant-fold-depend-on-the-dialect.md:75–77`: “Before the declared base reached the direct routes, incr … declined there while expr folded”.

Ordinary source `Before`/`After` examples describe current input/output and are not development history. New design docs absent at base; KCS dialect page absent base, so these are branch-introduced.

The brief explicitly requires current-state documentation, “never history”. These pages retain extensive development narrative and proposal/delivered framing for existing code. This is one coherent finding, rather than one finding for each status sentence. Rewrite the listed prose as present-tense contracts and move any desired history to commit descriptions.

Confirmed locations:

- `registry-consumer-contracts.md`:12–19,21–25,46–47,57,82,88,114 (retirement/build/ruling status);327–348 (proposal/owner decisions/documents repaired);350–600 (Ruling/Rationale/Consequences/**Decided with the build**, including documents/code that “gain”, “grow”, “drop” and the old flag behaviour);607–636 (down-from43, retired handlers, proposed migration);647–650,818–820,839–878 (adaptations/former checker/consumer changes plus “As built” narration);904–922 (tests “widens”, “gains”);929–946,1065–1142 (member descriptor “Proposed” at946, sites it retires, studio field moved, tests gain/rebaseline);1295–1325 (fifth ruling and consequences, drop-list loses/resolver gains);1358–1376,1478,1586–1608 (option walk retired/as built/GAPS rows lost/tests gain);1809–1829,1947–1962 (rungs built/proposed statuses);2027–2043 (“Rules1 to4 are built”, rung2 needed corrections, identity used to record);2087,2125–2136 (reference body is built, mock table dropped46 entries, ruling/generator status);2138–2140 (“Consequences” and runtime programme);2306 (current `tcl spec test` and reference-body capability as future “Fix”);2510–2530 (registration/host side built);2540,2573,2580–2581,2602 (anchors describe replaced resolvers, rebaselining/round-trip fields lost/form gains/ruling changes).
- `diagnostic-policy.md`:26–41 (built status and revision`3b5eba8a`);76–147 (**Before the policy step** matrix, **Why the copies existed**, explicit before/after story);328–330 (issue's omitted list);349–352 (codes “were strings too”);452–461 (`diagnostics.exclude` used to empty report);624–634 (module moved down from old nonexistent server path);724–725 (default-off seed “stopped” earlier CLI mismatch);843–893 (**Producers that change**, server used to create O111, parameters lost, old lifts, what changed);938–942 (four old unit tests);983–987,1123–1125 (owner-doc changes/consumer list design reduces).
- `c-extension-abi.md`:8–18 (**Implementation state**/partly-shipped status on the authored existing header/runtime exports);447–461 (**The seam, proven**, seam “was never exercised ... by anything but a stand-in” before current test). Future work for APIs that do not exist is not being claimed here as a factual contradiction.
- Changed KCS: `compiler/kcs-qa-where-does-a-clause-shape-come-from.md:41–51` narrates the former hand-written checker retiring and the migration; `compiler/kcs-qa-what-does-a-member-effect-say.md:55` describes the sites the descriptor retired.

Attribution: the two compiler design pages are entirely introduced by this branch; their history prose has no base-page equivalent. The ABI seam narrative was added by the branch. The ABI implementation-state introduction was amended by the branch (the authored two-host header is new); the forbidden new KCS narrative pages are likewise absent at base. These are documentation-maintenance findings, not new soundness claims.

The brief explicitly requires current-state documentation and forbids process/history narration, revision hashes and proposed/delivered status on implemented functionality. These are confirmed text findings, independent of the semantic failures above. The listed locations share one documentation-maintenance cause.

`value-transfers-examples.md` locations:

- Historical/proposal frame: :3–26, :29–51, :53–79 (the sixteen-issue “what running the corpus found” history and the subsequent fix history), :98–102, :130–138, :161–166, :184–186, :195–199, :210–212, :273–280, :296–299, :309–316 (“no longer depends”), :330–332, :379–381, :406–407, :553–555, :698–701, :879–882, :900–902, :917–920, :943–945, :971–973, :991–994, :1024–1031, :1055–1064, :1088–1093, :1158–1161, :1179–1187, :1203–1213, :1233–1237, :1306–1308, :1337–1339, :1364–1365, :1384, :1413–1421.
- The declaration section :1423–1981 is framed as current “Today” specs versus “Proposed” Rust/DSL. Specific proposal labels: :1448, :1490, :1512, :1562, :1585, :1602, :1631, :1646, :1667, :1760, :1815, :1847, :1931, :1940. Related-doc link :2104 calls the old DSL examples “today's authorable spellings”.
- Revision hashes appear at :16, :22, :78, :98, :184, :273, :278, :1056, :1089.
- Complete historical `today`/`merged` annotation line list follows below. The page's status block explicitly anchors those markers to earlier revisions rather than current tip observations.

`value-transfers-migration.md` locations:

- :1–15 (“migration plan”, “Status — delivered”), :17–41 (two sweep revisions, re-check/merge history), :43–102 (old tree inventory, gap list and old/new consumer diagram), :104–151 (analysis table's current/proposed/under-this-design columns), :153–185 (optimisation comparison table), :189–230 (diagnostic old/gain tables), :289–358 (sites awaiting an interface which exists; object bindings “recognised”; editor consumers “emitted only”, “now”, “first editor consumers”), :384–399 (survey history and “when the interface lands”), :411–435 (inventory's migration plan and dead-sites-first narration), :437–452 (pack inventory at an earlier revision), :747–750 (contracts the plan “lands” and today's observations).
- Explicit revision hashes at :19 and :27; proposed side map at :106; editor “were the first” at :354.

KCS: `kcs-diagnostic-i230-constant-existence-check.md:42–46` says a prior assignment “no longer blocks” the fold.

The standard KCS optimiser headings `Before`/`After` describe a present source rewrite rather than development history; they have deliberately not been called a defect.

#### Complete historical annotation locations in examples

4, 13, 21, 33, 36, 47, 55, 79, 88, 95, 106, 118, 124, 125, 126, 143, 153, 158, 172, 178, 219, 246, 247, 255, 268, 285, 296, 321, 327, 338, 350, 360, 370, 376, 386, 396, 403, 415, 427, 440, 444, 447, 450, 457, 470, 481, 492, 503, 514, 515, 526, 530, 544, 548, 652, 678, 688, 691, 707, 723, 767, 773, 775, 786, 869, 875, 887, 895, 907, 913, 924, 926, 927, 932, 934, 940, 950, 957, 966, 979, 980, 982, 983, 984, 987, 999, 1003, 1006, 1020, 1035, 1039, 1043, 1044, 1050, 1123, 1125, 1133, 1136, 1141, 1143, 1146, 1152, 1155, 1165, 1166, 1167, 1169, 1170, 1171, 1176, 1191, 1196, 1208, 1219, 1221, 1223, 1225, 1228, 1230, 1242, 1244, 1246, 1248, 1250, 1252, 1253, 1258, 1264, 1265, 1269, 1273, 1280, 1291, 1294, 1297, 1301, 1312, 1313, 1314, 1315, 1316, 1319, 1321, 1322, 1323, 1324, 1330, 1333, 1345, 1354, 1360, 1369, 1370, 1373, 1379, 1381, 1389, 1390, 1391, 1392, 1393, 1394, 1395, 1396, 1397, 1398, 1402, 1403, 1404, 1405, 1406, 1407, 1408, 1409, 1410, 1419, 1544, 1665, 1811, 1936, 1944, 2104


#### Complete proposal/comparison marker locations in examples

19, 98, 111, 130, 134, 146, 161, 197, 210, 261, 277, 297, 330, 343, 353, 365, 379, 391, 406, 420, 432, 462, 474, 486, 497, 508, 520, 534, 553, 698, 778, 879, 900, 917, 943, 971, 991, 1027, 1056, 1057, 1090, 1128, 1158, 1183, 1233, 1285, 1306, 1337, 1348, 1364, 1384, 1413, 1420, 1448, 1490, 1512, 1562, 1585, 1602, 1631, 1646, 1667, 1760, 1815, 1847, 1931, 1940


## N — Nits

No separate nits are reported.

## P — Pre-existing findings


### P1 — O100 leaves the closer of a braced variable reference

The original input prints `3` successfully under every reference shell. Both builds rewrite the return into `return 3}}`, leaving one extra `}` and causing `extra characters after close-brace` before any output. Tip also folds `[p]` to `3`, but base already emits the same invalid procedure definition. This is pre-existing, not introduced by the reviewed branch.

Exact input at [proc_return_varname.tcl](/tmp/spectcl-review/compiler-cases/proc_return_varname.tcl):

```tcl
proc p {} {set {a b} 3; return ${a b}}
puts [p]
```

Tip complete optimiser stdout (identical under all five dialect flags):

```tcl
proc p {} {set {a b} 3; return 3}}
puts 3


# -------------
# optimised: 2 rewrite(s)
# O100  Fold return of constant variable
# O103  Fold pure-proc call to '::p' to its constant return
```

Base complete optimiser stdout (identical under all five dialect flags):

```tcl
proc p {} {set {a b} 3; return 3}}
puts [p]


# -------------
# optimised: 1 rewrite(s)
# O100  Fold return of constant variable
```

Implementing path:

- Tip [propagation.rs:1915](/workspace/tcl-lsp/rust/tcl-compiler/src/optimiser/propagation.rs:1915), `try_fold_return_terminator`, recognises `${…}` at line 1988 and reports O100 with the unchanged statement `span` at lines 2002–2007. Base has the same function at [propagation.rs:2114](/workspace/tcl-lsp-review-base/rust/tcl-compiler/src/optimiser/propagation.rs:2114), `${…}` handling at line 2187 and report at lines 2201–2206.
- [segmenter.rs:346](/workspace/tcl-lsp/rust/tcl-compiler/src/segmenter.rs:346), `widen_word_end`, returns the inner token end when `group_closer()` is absent; the ending variable token in this witness does not widen through its `}`. The inherited span reaches `try_lower_return` ([control.rs:213](/workspace/tcl-lsp/rust/tcl-compiler/src/lowering/hooks/control.rs:213)) and then O100.
- [manager.rs:818](/workspace/tcl-lsp/rust/tcl-compiler/src/optimiser/manager.rs:818), `apply_optimisations`, applies exactly the half-open reported byte range. Base uses [manager.rs:809](/workspace/tcl-lsp-review-base/rust/tcl-compiler/src/optimiser/manager.rs:809).

A small exact CLI inspection confirms the span, rather than inferring it from the parser source. Both builds’ `explore --dialect tcl8.6 --json` outputs report O100 range `[24,36)` and replacement `return 3`. In the original input, byte 36 is the closing `}` of `${a b}` and byte 37 is the procedure body’s closing `}`. The rewrite therefore retains both braces. Raw inspection evidence is in [tip JSON](/tmp/spectcl-review/proc-return-varname-explore.json) and [base JSON](/tmp/spectcl-review/proc-return-varname-base-explore.json).

Exact inspection commands:

```sh
/workspace/tcl-lsp-review-fast/target/ci/tcl explore --dialect tcl8.6 --json /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-base/target/ci/tcl explore --dialect tcl8.6 --json /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
```

#### Exact optimiser commands

Each command exited 0 with empty stderr and the full tip/base stdout shown above. The corpus runner wrote that stdout verbatim to the corresponding `compiler-results/proc_return_varname.<release>.<build>.opt.tcl` file. No `--profile` flag was passed.

```sh
/workspace/tcl-lsp-review-fast/target/ci/tcl opt --dialect tcl8.4 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-base/target/ci/tcl opt --dialect tcl8.4 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-fast/target/ci/tcl opt --dialect tcl8.5 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-base/target/ci/tcl opt --dialect tcl8.5 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-fast/target/ci/tcl opt --dialect tcl8.6 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-base/target/ci/tcl opt --dialect tcl8.6 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-fast/target/ci/tcl opt --dialect tcl9.0 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-base/target/ci/tcl opt --dialect tcl9.0 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-fast/target/ci/tcl opt --dialect tcl9.1 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
/workspace/tcl-lsp-review-base/target/ci/tcl opt --dialect tcl9.1 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
```

#### Exact five-release shell evidence

Every recorded command, exit status and stream is reproduced below. Stream strings use JSON escaping to preserve newlines, quotes and the Tcl 8.4/8.5 traceback’s multiline command excerpt exactly.

#### Tcl 8.4.20

Original:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.4 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
exit: 0
stdout: "3\n"
stderr: ""
```

Tip rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.4 /tmp/spectcl-review/compiler-results/proc_return_varname.8.4.tip.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\nputs 3\n\n\n# -------------\n# optimised: 2 rewrite(s)\n# O100  Fold return of constant variable\n# O103  Fold pure-proc ...\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.8.4.tip.opt.tcl\" line 1)\n"
```

Base rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.4 /tmp/spectcl-review/compiler-results/proc_return_varname.8.4.base.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\nputs [p]\n\n\n# -------------\n# optimised: 1 rewrite(s)\n# O100  Fold return of constant variable\n\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.8.4.base.opt.tcl\" line 1)\n"
```


#### Tcl 8.5.19

Original:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.5 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
exit: 0
stdout: "3\n"
stderr: ""
```

Tip rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.5 /tmp/spectcl-review/compiler-results/proc_return_varname.8.5.tip.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\nputs 3\n\n\n# -------------\n# optimised: 2 rewrite(s)\n# O100  Fold return of constant variable\n# O103  Fold pure-proc ...\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.8.5.tip.opt.tcl\" line 1)\n"
```

Base rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.5 /tmp/spectcl-review/compiler-results/proc_return_varname.8.5.base.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\nputs [p]\n\n\n# -------------\n# optimised: 1 rewrite(s)\n# O100  Fold return of constant variable\n\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.8.5.base.opt.tcl\" line 1)\n"
```


#### Tcl 8.6.18

Original:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.6 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
exit: 0
stdout: "3\n"
stderr: ""
```

Tip rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.6 /tmp/spectcl-review/compiler-results/proc_return_varname.8.6.tip.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.8.6.tip.opt.tcl\" line 1)\n"
```

Base rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh8.6 /tmp/spectcl-review/compiler-results/proc_return_varname.8.6.base.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.8.6.base.opt.tcl\" line 1)\n"
```


#### Tcl 9.0.4

Original:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh9.0 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
exit: 0
stdout: "3\n"
stderr: ""
```

Tip rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh9.0 /tmp/spectcl-review/compiler-results/proc_return_varname.9.0.tip.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.9.0.tip.opt.tcl\" line 1)\n"
```

Base rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh9.0 /tmp/spectcl-review/compiler-results/proc_return_varname.9.0.base.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.9.0.base.opt.tcl\" line 1)\n"
```


#### Tcl 9.1.0

Original:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh9.1 /tmp/spectcl-review/compiler-cases/proc_return_varname.tcl
exit: 0
stdout: "3\n"
stderr: ""
```

Tip rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh9.1 /tmp/spectcl-review/compiler-results/proc_return_varname.9.1.tip.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.9.1.tip.opt.tcl\" line 1)\n"
```

Base rewritten:

```text
command: /tmp/spectcl-review/tcl-reference-bin/tclsh9.1 /tmp/spectcl-review/compiler-results/proc_return_varname.9.1.base.opt.tcl
exit: 1
stdout: ""
stderr: "extra characters after close-brace\n    while executing\n\"proc p {} {set {a b} 3; return 3}}\"\n    (file \"/tmp/spectcl-review/compiler-results/proc_return_varname.9.1.base.opt.tcl\" line 1)\n"
```

Known-class comparison: #2391 concerns `return {$x}` treating a literal braced word as a substitution. This witness uses a legal `${a b}` variable substitution and leaves its closer outside the replacement span. It is a distinct source-span angle in the same O100 function; both base and tip fail, so it is not a branch regression.


### P2 — The VM forgets a caught command or value-size budget failure

The host command `exhausted` returns `EngineError::BudgetExceeded(Commands)`. Body `catch {exhausted}; return exact` returns `Ok("exact")` on both tip and base TclVmEngine. So does `catch {string repeat abcdefgh 100}; return exact` under `max_value_bytes = 64`, where the repeat would build 800 bytes. The newly added RuntimeEngine correctly reports Commands / ValueSize budget errors for these bodies even after the catch.

The VM adapter decides budget failure from the final completion's message (`rust/tcl-engine-tclvm/src/lib.rs:493`); it accepts a normal final completion before reading anything else. Its host-command budget case makes an ordinary error completion (`:351`) rather than recording an invocation-wide exceeded state. This is a pre-existing failure, not a branch-introduced regression. `catch` is outside the current pack whitelist, so these witnesses establish the Engine contract discrepancy, not a current pack-host miscompile.

The base-compatible harness and exact base results are in the runtime appendix.


### P3 — The value-size budget is enforced only by some value builders

Under a 64-byte cap, `return [string cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh]` succeeds with a 72-byte result on tip and base TclVmEngine and on the newly added RuntimeEngine. `return [lrepeat 100 abcdefgh]` similarly succeeds with 899 bytes. The first command is on the actual pack whitelist.

The API states “Maximum byte length of any single value the body may build” (`rust/tcl-engine-api/src/lib.rs:316`). Native runtime enforcement measures `string repeat` only (`runtime/rust/src/cmd_string.rs:148`; `Budget::charge_allocation` at `runtime/rust/src/budget.rs:226`). The VM had the same partial cap at base. The new RuntimeEngine repeats this limitation, so this is a pre-existing API promise that the new adapter also does not meet. These fixed-size probes show accepted over-budget values, without attempting a large allocation or asserting a crash. The WASM host also limits memory growth; that extra bound was reviewed from source but was not exercised by this standalone native probe.

The base-compatible harness and exact base results are in the runtime appendix.


### P4 — The ABI page claims shipped stubs and compile coverage which do not exist

`docs/design/runtime/c-extension-abi.md:276–277` says “We ship the `TclOOStubs` / `TclStubs` struct shapes” and that `pkgooa.c` compiles in the compile-check. Neither authored type exists in `runtime/rust/include/`; that directory contains only `tcl.h` and `tcl_regex_capi.h`. The canonical `tmp/tcl9.0.4/unix/dltest/pkgooa.c:14` includes `tclOO.h`, which this runtime does not provide. The current `scripts/check_c_extension_wasm.py::check_compiles` at line314 checks `layout.c`, `pkga.c`, `doors.c` and its two function probes; it does not compile `pkgooa.c` or the nine-extension corpus. The opening of this same page correctly says there is no authored `tclOO.h` (`:11`) and section4.1 repeats that (`:112–114`).

Reproduction:

```sh
cc -fsyntax-only -std=c99 -DTCL_HOST_WASM -Iruntime/rust/include tmp/tcl9.0.4/unix/dltest/pkgooa.c
```

Exit1: `fatal error: tclOO.h: No such file or directory`. The same canonical source compiled with `-I/workspace/tcl-lsp-review-base/runtime/rust/include` fails identically. This is a documentation contradiction, not a report that supported extensions fail their actual gate.

Base: both false sentences already exist verbatim in the base page; blame is pre-branch `4e403da805`. Therefore this is pre-existing documentary debt (P), with no relation to the listed known issues.


### P5 — The annotation reference promises stub arity checks

`docs/kcs/kcs-qa-tcl-lsp-annotations.md:75–79`, `:105–108` says a stub's argument count is checked. The current `kcs-howto-annotate-commands-with-stubs.md:25–26`, `:249–251` correctly says it is not. `rust/tcl-compiler/src/analyser/diagnostics/validity.rs::flush_arity_diagnostics` at `:2652–2656` explicitly skips a candidate whose name is stubbed.

Exact fixture `/tmp/spectcl-review/stub-arity.tcl`:

```tcl
# tcl-lsp: stubs-begin
# tcl-lsp: stub vend {x}
# tcl-lsp: stubs-end
proc vend {x} {return $x}
catch {vend} a
catch {vend 1 2} b
puts "$a|$b"
```

Both tip and base `tcl diag FILE --dialect tcl9.0 --json` exit0 with `diagnostics: []`. Every reference wrapper, Tcl8.4.20/8.5.19/8.6.18/9.0.4/9.1.0, prints exactly:

```text
wrong # args: should be "vend x"|wrong # args: should be "vend x"
```

This is a false documented capability, not an assertion that intentionally conservative stub analysis must emit arity errors. Both false sentences are unchanged from base (`61b013af66`); base validity function similarly skips stub names at its line2520. Correct the annotation reference to match the how-to.


### P6 — The suppression guide reverses configuration reload advice

`docs/kcs/kcs-howto-suppress-diagnostics.md:170–175` tells users to restart after a project `.tcl-lsp.ini` edit and says editor **or global** config edits refresh automatically. In the current server the project file is watched; global config is not watched. `kcs-qa-how-tcl-lsp-loads-configuration.md:222–230` states this correctly.

Proof: `rust/tcl-lsp-server/src/lib.rs::register_file_watchers` (`:21148–21192`) registers `PROJECT_CONFIG_GLOB`; `did_change_watched_files` at `:24280–24288` re-pulls layers, scans and reschedules open and closed documents on `config_changed`; no user-config directory watcher is registered. Regression `watcher_registration_covers_sources_and_project_config` at `:43132` names the project live-reload branch. Base has the same watcher test and live-reload branch (`:23948–23955`); the wrong KCS advice is unchanged at base, blame `fd495dd7d8`.

Suggested correction: project config changes reload through watched-file notifications; manual global-file changes require restart unless some separate editor config event causes a fresh read.


### P7 — The configuration answer recommends a nonexistent ignore list

`docs/kcs/kcs-qa-how-tcl-lsp-loads-configuration.md:57–61` says an editor override can beat a project key by adding that key to the project's “local-only ignore list”. No such facility is implemented. `tcl_lsp_core::config_ini::settings_from_ini` (`:178–291`) reads the complete known section/key schema, with no ignore-list key; `merge_settings` (`:622`) merges project keys normally; the project's highest priority is enforced by `PolicyBuilder::build` too. `rg -n 'local.only ignore|ignore list|ignore_list|ignoredKeys|ignoreKeys' docs rust/tcl-lsp-server rust/tcl-lsp-core` finds this page's own sentence as the only relevant hit. The complete configuration-schema KCS page defines no such list.

Base: same paragraph exists verbatim (`7910917607`), and base parser/merge likewise have no list. There is no prescribed syntax to exercise because the claimed feature is absent; this is confirmed by the complete parser and schema, not a guessed failing key. Remove the nonexistent workaround.


### P8 — The pack quickstart calls an older minor the current vocabulary

`docs/kcs/kcs-howto-write-a-tclspec-pack.md:45–46` calls2.0 the current vocabulary. Tip `rust/tcl-spectcl/src/loader.rs:1456` sets `NEWEST_VOCABULARY_VERSION = "2.2"`; base loader1363 sets2.1; the sentence is identical at base. The quickstart's separate assertion that `tcl spec upgrade` outputs2.0 remains true: tip upgraded a minimal1.0 pack to `speclib probe 2.0`, so flag only the “current one” claim. Pre-existing documentary defect, not a branch regression. No base-binary semantics claim needed for this textual contradiction.


### P9 — The existing W210 note also retains development history

`docs/kcs/codes/kcs-diagnostic-w210-variable-read-before-set.md:120–121` recounts what the code did “Before this was registry-driven” and issue #2117. The same paragraph is present at base lines 112–113, verified with `git show 6d214350b:docs/kcs/codes/kcs-diagnostic-w210-variable-read-before-set.md`. This is pre-existing narration under the brief's current-state-only documentation rule, separate from the branch-added S5 locations. It is a textual finding; Tcl runtime execution is not relevant.


## Known exclusions and corrected witnesses

The title/body ledger covers all required issue numbers. #2409 remains excluded as withdrawn. Standing `dialect-drift` exits 1 at eight base-existing sites (#2253). Locale-specific UTF-8 assertions remain the known #2271 class; CI is run with `LANG=C.UTF-8`. The fixed corpus's shadow-after-call, conditional-procedure closer and forward-called-procedure failures overlap #2398, #2383 and the related #2390 completion class and are excluded as new findings.

Tip/base execution under all five reference interpreters confirms that the selected witnesses for #2388 (double and padded return spelling), #2389 (canonical rejected arity and substituted argument side effect), #2392 (padded set), #2393 (early return in switch), and #2418 (empty split) are corrected at this tip. This statement concerns the tested witnesses, not every possible variant of those error classes.


| Corpus input | Classification | Observed behaviour on both builds, all five releases |
|---|---|---|
| `builtin_before_shadow` | Direct repeat of [#2398](https://github.com/bitwisecook/tcl-lsp/issues/2398), O103 folding before the procedure definition dominates a call | Original prints `3\n7\n`; optimised prints `7\n7\n`. |
| `proc_conditional_define` | Direct repeat of [#2383](https://github.com/bitwisecook/tcl-lsp/issues/2383), O112 dropping a trailing braced word’s closer | Original prints `1\n`; optimised source raises `missing close-brace`. |
| `proc_called_before_define` | New forward-binding witness of #2398’s dominance gap; [#2390](https://github.com/bitwisecook/tcl-lsp/issues/2390) is related completion loss | Original prints `1\n1\n`; both optimised versions print `0\n1\n`. Tip O103 replaces `[label]` inside `p` before its first caught invocation; base O126 already deletes the assignment containing that raising call. No behavioural regression. |

The forward-binding witness is broader than #2398’s builtin-shadow example and can inform that existing issue’s coverage. It is not retained as a separate new report. The `${a b}` finding below is a different span defect from [#2391](https://github.com/bitwisecook/tcl-lsp/issues/2391)’s treatment of braced literal words and [#2340](https://github.com/bitwisecook/tcl-lsp/issues/2340)’s loss of other text from a composite return word: this return word consists entirely of one valid variable substitution and its value really is 3.

### Corrected base nested-read cases

### `terminator_return_lindex_before_write`

Input (also the tip’s complete emitted source, with no rewrites):

```tcl
proc p {} {set l {a b c}; set i 0; return [list [lindex $l $i] [set i 9]]}
puts [p]
```

Base complete optimiser stdout:

```tcl
proc p {} {set l {a b c}; ; return [list [lindex $l $i] [set i 9]]}
puts [p]


# -------------
# optimised: 1 rewrite(s)
# O109  Eliminate dead store
```

All five original and tip executions exit 0 with stdout `a 9\n` and empty stderr. All five base executions exit 1 with empty stdout and `can't read "i": no such variable` from the nested read. Base O109 removes `set i 0` even though the earlier substitution reads it before `[set i 9]`; tip keeps the store. On Tcl 8.6, base also emits W220 for that initial assignment, while tip emits no diagnostic.

### `terminator_return_string_before_write`

Input (also the tip’s complete emitted source, with no rewrites):

```tcl
proc p {} {set s abc; set i 0; return [list [string index $s $i] [set i 9]]}
puts [p]
```

Base complete optimiser stdout:

```tcl
proc p {} {set s abc; ; return [list [string index $s $i] [set i 9]]}
puts [p]


# -------------
# optimised: 1 rewrite(s)
# O109  Eliminate dead store
```

All five original and tip executions exit 0 with stdout `a 9\n` and empty stderr. All five base executions exit 1 with empty stdout and `can't read "i": no such variable` from the nested read. Base O109 removes `set i 0` even though the earlier substitution reads it before `[set i 9]`; tip keeps the store. On Tcl 8.6, base also emits W220 for that initial assignment, while tip emits no diagnostic.

## Contract ownership and verification coverage

All eight ownership rulings and the seven target-axis rows were mapped to source and named tests. The table states limits where an operation-specific test does not establish an entire route's contract. The missing pin descriptions below are coverage observations; they are not additional reproduced implementation findings.


| Ruling and contract anchor | Production anchors | Existing named tests and precise evidence limit |
|---|---|---|
| **1. Ownership boundary** — `docs/design/compiler/value-transfers.md:51` | Registry interface: `rust/tcl-registry/src/value_transfer/mod.rs:113` and read-only inputs at `rust/tcl-registry/src/value_transfer/inputs.rs:451`. Declaration resolution: `rust/tcl-registry/src/value_transfer/declaration.rs:278`. Compiler generic fact application: `rust/tcl-compiler/src/value_transfer.rs:2648` (`call_defs`), `:2915` (`apply_outcome`), `:3978` (`route_answer`). The inspected route dispatch branches on route family and registry-owned evaluator identity. | `rust/tcl-registry/tests/value_transfers.rs:1592` **`every_cell_update_has_a_registry_owned_route`**; `:5102` **`validate_outcome_rejects_a_store_to_a_non_target`**; `rust/tcl-compiler/tests/value_transfer_witnesses.rs:2042` **`the_completion_test_needs_no_consumer_edit`**. These pin listed cell-update owners, representative structural validation, and a private command reaching generic consumers. They do not prove repository-wide absence of every command-name/command-ID arm, or completeness of the migration ledger. The source-location and no-new-arms clauses are structural review obligations, not one boolean runtime guard. |
| **2. Executable backing declared; purity supplies none** — `docs/design/compiler/value-transfers.md:57` | `rust/tcl-registry/src/value_transfer/route.rs:34` defines `Direct`, `Expression`, `Implementation`, and `None`. `:156` defines `EvaluatorCapability` with identity, host, target axes, inputs, dependencies, budget, and completion. `rust/tcl-compiler/src/value_transfer.rs:4015` unconditionally returns the typed `NoRoute` decline for `None`; declared evaluation also declines explicit `None` at `rust/tcl-registry/src/value_transfer/declared.rs:721`. | `rust/tcl-registry/tests/value_transfers.rs:4427` **`the_capability_is_part_of_the_route_identity`** varies pack/id/hash/target/inputs/dependencies/budget, but not host or completion; `:2825` **`the_resolver_projects_the_declaration_state`** pins declaration resolution states; `rust/tcl-compiler/tests/value_transfer_witnesses.rs:1227` **`route_entries_are_counted_per_family`** checks Direct/Expression accounting and a rebound expression's zero entries. The capability fields and inspected `None` arm support the architecture; these tests do not independently assert that purity never supplies an evaluator in every consumer. |
| **3. Workspace facts authoritative** — `docs/design/compiler/value-transfers.md:64` | Loaded declarations use `rust/tcl-registry/src/value_transfer/declared.rs:711` → `:394` → `:553`; compiler route application is `rust/tcl-compiler/src/value_transfer.rs:3978`. The inspected evaluator/application path has no pack-provenance precision cap. `rust/tcl-registry/src/value_transfer/context.rs:68` carries registry/overlay generations, bindings, target profile, and evaluator revisions. | `rust/tcl-compiler/tests/value_transfer_witnesses.rs:2042` **`the_completion_test_needs_no_consumer_edit`** loads the Workspace fixture and checks exact values and real O100 forwarding through its helper at `:2134`; `:2237` **`a_pack_write_through_an_incoming_target_reaches_the_driver`** pins pack writes. This is representative authority evidence for results and stores. No test in the inspected set separately asserts the entire false-workspace-fact chain of edge pruning, diagnostic suppression, and code elimination. Do not label that full chain mutation-tested from these pins. |
| **4. One regexp owner** — `docs/design/compiler/value-transfers.md:77` | `rust/tcl-registry/src/value_transfer/regex.rs:40` imports `tcl_regex::cmd_core::AreEngine`; concrete registry calls are `:267` (`regexp_analysis`) and `:382` (`regsub_analysis`). Precision refusals map through `:171`; command errors map to raised completion at `:197`. VM plumbing uses the same engine at `rust/tcl-vm/src/cmd_regexp.rs:56`, `:97`, and `:123`; runtime plumbing does so at `runtime/rust/src/cmd_regex.rs:58`, `:70`, and `:116`. | `rust/tcl-registry/tests/value_transfers.rs:5367` **`regexp_writes_or_preserves_its_match_variables`**; `:5481` **`a_regexp_that_established_nothing_declines`**; `:5532` **`regsub_writes_its_variable_and_declines_its_callback`**; `rust/tcl-registry/tests/differential_fold.rs:1340` **`regexp_witnesses_match_every_release_on_path`**. They pin match/no-match stores, exhausted-search refusal, malformed-pattern completion, callback refusal, and selected C Tcl witnesses when interpreters are present. One-engine ownership is also a structural source fact; these tests do not scan for a second matcher. |
| **5. Shipped specialisations remain Rust; pack calculations share interface** — `docs/design/compiler/value-transfers.md:80` | Rust builtin implementations of the registry trait include `rust/tcl-registry/src/value_transfer/builtins.rs:240`, `:1057`, and `:1206`. DSL declarations implement that trait at `rust/tcl-registry/src/value_transfer/declared.rs:690`. | `rust/tcl-registry/tests/value_transfers.rs:4265` **`route_stamps_match_the_pinned_set`**; `:4284` **`shipped_builtins_stay_on_the_direct_route`**. The latter checks the pinned shipped catalogue remains unchanged while exactly three tenant implementation spellings are added. Its name does **not** mean every builtin is in the Direct family: `expr` is Expression and the pinned set also records None routes. Rust source ownership is structural; catalogue route stamps are a testable subset. |
| **6. First-delivery scope** — `docs/design/compiler/value-transfers.md:83` | All four routes dispatch at `rust/tcl-compiler/src/value_transfer.rs:3978`; shared analysis identity is `rust/tcl-registry/src/value_transfer/context.rs:68`; private tenant fixture/host integration is exercised through `rust/tcl-compiler/tests/value_transfer_witnesses.rs:2042`. | `rust/tcl-compiler/tests/value_transfer_witnesses.rs:445` **`program_three_folds_in_every_consumer`**; `:1883` **`program_two_folds_and_is_a_byte_array`**; `:2042` **`the_completion_test_needs_no_consumer_edit`** pin delivered examples. The phase/defer list is a historical scope ruling, not a prohibition against subsequent work or a meaningful runtime condition to mutate. Current mock extension-host test `rust/tcl-registry/tests/value_transfers.rs:8192` **`a_wasm_extension_implementation_runs_on_the_extension_host`** does not turn the initial phase wording into a failed contract. |
| **7. Release-less values require unanimity** — `docs/design/compiler/value-transfers.md:87` | Numerals: `rust/tcl-registry/src/value_transfer/const_ops.rs:791` uses `NumberSyntax::unanimous`; index pre-resolution: `:645` / `:661`; integer overflow disagreement: `:950` / `:969`; character counts: `:894`. Expression numeral/platform final checks are `rust/tcl-compiler/src/tcl_expr_eval.rs:2073` and `:2076`; integer-tower checks are `:1723`. Declared implementations conservatively decline a target naming no release at `rust/tcl-registry/src/value_transfer/declared.rs:429`, rather than executing engines under every release. | Unit tests in **`rust/tcl-registry/src/value_transfer/const_ops.rs`**: `:1131` **`numerals_read_under_the_named_grammar_or_the_unanimous_one`**; `:1177` **`the_integer_tower_widens_from_8_5_and_declines_elsewhere`**; `:1239` **`an_index_is_pre_resolved_under_the_admitted_grammar`**; `:1409` **`character_counts_need_a_release_only_off_the_basic_plane`**; `:1489` **`non_ascii_text_is_admissible_only_where_source_is_utf8`**. Compiler route pin: `rust/tcl-compiler/tests/value_transfer_witnesses.rs:1284` **`expr_acceptance_list`** includes unanimous cases and release-less refusals. These are named-axis/operation pins, not an exhaustive all-route unanimity proof. Completion fields have separate partial-evidence behaviour: `const_ops.rs:471` keeps an agreed field exact and a disagreeing field unavailable; `rust/tcl-registry/tests/value_transfers.rs:7589` **`a_write_to_an_array_is_proven_only_where_every_release_agrees`** pins that distinction. |
| **8. Declared base governs; declared divergence blocks it** — `docs/design/compiler/value-transfers.md:91` | `rust/tcl-registry/src/value_transfer/const_ops.rs:341` obtains `DialectProfile::runtime_version` and filters numeral/character assumptions against explicit profile grammar/model. Engine pinning is `rust/tcl-engine-tclvm/src/lib.rs:657`; the host pins before compilation at `rust/tcl-spec-hooks/src/host.rs:222`. | Unit tests: `rust/tcl-registry/src/value_transfer/const_ops.rs:1448` **`a_declared_base_release_is_the_release`**, `:1409` **`character_counts_need_a_release_only_off_the_basic_plane`** (F5 divergence); `rust/tcl-compiler/src/tcl_expr_eval.rs:3013` **`leading_zero_octal_policy_follows_the_runtime_base`**; `rust/tcl-engine-tclvm/src/lib.rs:1042` **`set_release_pins_the_numeral_grammar`**. Integration/oracle pin: `rust/tcl-spectcl/tests/pack_source_e2e.rs:470` **`a_derived_body_runs_under_the_release_the_call_is_analysed_under`**. The engine unit test covers the numeral grammar for 8.6/9.0/iRules plus repinning rules; it is not a full per-axis C Tcl certificate. The integration test covers a selected operator/availability/character table and skips missing reference shells. |

### Route/axis matrix

Common route anchors: `rust/tcl-registry/src/value_transfer/route.rs:34` defines the four families; `rust/tcl-compiler/src/value_transfer.rs:3978` dispatches them. The **None column for every axis** has the same unconditional `NoRoute` guard at `:4015`. **`route_entries_are_counted_per_family`** at `rust/tcl-compiler/tests/value_transfer_witnesses.rs:1227` checks Direct/Expression accounting and a rebound expression's zero entries; it does not assert every None-axis result. None behaviour here is source-inspected; no separate per-axis None execution test is claimed.

Direct routes open `ConstOps` at `rust/tcl-registry/src/value_transfer/const_ops.rs:509`; this rejects Platform/WallClock immediately, while other axis checks occur at the actual operations. Expression evaluation uses the one tree walk at `rust/tcl-compiler/src/tcl_expr_eval.rs:2059`. Declared implementations resolve their inputs/targets at `rust/tcl-registry/src/value_transfer/declared.rs:416` and `:421`, require a named runtime release at `:429`, admit the capability's axes at `:432`, and dispatch the declared host at `:435` / `:438`.

### Numeric syntax, integer tower, operator set — matrix `:1672`

- **Direct:** `rust/tcl-registry/src/value_transfer/const_ops.rs:791` reads the named numeral grammar or its unanimous answer; `:950` models `incr` widening. Existing pins: unit tests `:1131` **`numerals_read_under_the_named_grammar_or_the_unanimous_one`** and `:1177` **`the_integer_tower_widens_from_8_5_and_declines_elsewhere`**; route test `rust/tcl-registry/tests/value_transfers.rs:2314` **`the_increment_route_reads_numerals_under_the_target_release`**.
- **Expression:** `rust/tcl-compiler/src/tcl_expr_eval.rs:1689` configures policy, `:1723` rejects beyond-target operands/results, and `:1823` configures the shared fold services. Existing pins: `rust/tcl-compiler/tests/value_transfer_witnesses.rs:1284` **`expr_acceptance_list`**, `:1573` **`o101_rewrites_only_what_the_route_proves`**, and `:1501` **`expression_witnesses_match_every_release_on_path`**. The latter is a selected 13-case oracle table, not every operator.
- **Declared:** `rust/tcl-engine-tclvm/src/lib.rs:657` pins a catalogue profile; `rust/tcl-registry/src/value_transfer/declared.rs:429` refuses release-less execution. Existing pins: engine unit test `:1042` **`set_release_pins_the_numeral_grammar`** and `rust/tcl-spectcl/tests/pack_source_e2e.rs:470` **`a_derived_body_runs_under_the_release_the_call_is_analysed_under`** (availability, octal, digit separator, formatting and selected conversions). A pin alone does not prove every engine numeric operation matches C Tcl.

### Character/index/source-encoding model — matrix `:1673`

- **Direct:** `rust/tcl-registry/src/value_transfer/const_ops.rs:341` selects target assumptions, `:618` admits source text, `:645` pre-resolves indices, `:894` counts characters. `rust/tcl-registry/src/value_transfer/builtins.rs:168` declares StringRange's axes and `:189`–`:197` applies admission/index pre-resolution. Existing pins: const-ops unit tests `:1239` **`an_index_is_pre_resolved_under_the_admitted_grammar`**, `:1409` **`character_counts_need_a_release_only_off_the_basic_plane`**, `:1489` **`non_ascii_text_is_admissible_only_where_source_is_utf8`**; registry route test `rust/tcl-registry/tests/value_transfers.rs:4645` **`string_range_runs_the_shared_core_with_the_index_grammar`**; oracle test `rust/tcl-registry/tests/differential_fold.rs:858` **`an_index_reads_as_each_release_reads_it`**.
- **Expression:** `rust/tcl-compiler/src/tcl_expr_eval.rs:1736` converts already-exact operands, `:1915` consumes nested route answers, and `:1884` refuses non-text command script bytes. `rust/tcl-compiler/tests/value_transfer_witnesses.rs:1284` **`expr_acceptance_list`** includes nested `string length`. These inspected anchors do not themselves constitute a complete expression source-encoding admission guard or an all-release character-index test.
- **Declared:** `rust/tcl-registry/src/value_transfer/declared.rs:432` admits declared target axes; `rust/tcl-spec-hooks/src/host.rs:60` caps engine values at 16 MiB. `rust/tcl-spectcl/tests/pack_source_e2e.rs:470` **`a_derived_body_runs_under_the_release_the_call_is_analysed_under`** includes astral `string length` (`:526`) against available shells. No full declared-route character/index/source-encoding conformance matrix is established by the inspected tests; the value cap is a containment guard, not semantic evidence.

### Regexp features and precision limits — matrix `:1674`

- **Direct:** `rust/tcl-registry/src/value_transfer/regex.rs:267`, `:382` call the shared owner, `:136` meters work, `:171` maps exhausted/deep/approximate search to decline, and `:355` rejects `regsub -command`. Existing pins: `rust/tcl-registry/tests/value_transfers.rs:5367` **`regexp_writes_or_preserves_its_match_variables`**, `:5481` **`a_regexp_that_established_nothing_declines`**, `:5532` **`regsub_writes_its_variable_and_declines_its_callback`**; `rust/tcl-registry/tests/differential_fold.rs:1340` **`regexp_witnesses_match_every_release_on_path`**.
- **Expression:** already-exact nested command answers cross `rust/tcl-compiler/src/tcl_expr_eval.rs:1915`. The compiler's own iRules `matches_regex` fold is deliberately refused by `apply_irules_string_op` at `:1557` / `:1563`; unit test `:3395` **`irules_matches_regex_is_not_folded`** pins that refusal. Thus “no regexp operator” should be read narrowly as no concrete matching in this folding path, not absence of a parsed iRules regexp operator.
- **Declared:** a body's VM command uses `rust/tcl-vm/src/cmd_regexp.rs:97` and `:123` with the same engine. Sharing the owner does not preserve the direct route's typed precision decline across the script-error/host-abstention boundary. No inspected declared-regexp precision oracle table is claimed.
- **Confirmed document drift:** matrix `docs/design/compiler/value-evaluation.md:1674` still says `-about` declines. Registry test `rust/tcl-registry/tests/value_transfers.rs:5367` explicitly expects `0 REG_UNONPOSIX` and `1 {}` at `:5447`; core unit test `rust/tcl-cmd-core/src/regex.rs:1880` **`regexp_about_reports_the_subexpression_count_and_info_list`** pins support. This is source-confirmed wording drift, not an execution finding from this subagent.

### Binary fields and byte representation — matrix `:1675`

- **Direct:** `rust/tcl-registry/src/value_transfer/builtins.rs:940` gates field-letter availability, `:1012` applies field refusal, `:1029` charges the output bound before packing, and `:1040` publishes a ByteArray construction. `rust/tcl-registry/src/value_transfer/const_ops.rs:204` separates exact bytes from representation. Existing pins: `rust/tcl-compiler/tests/value_transfer_witnesses.rs:1883` **`program_two_folds_and_is_a_byte_array`** (representation, diagnostics, and no source materialisation), `:1945` **`folded_types_state_what_each_route_constructed`**; `rust/tcl-registry/tests/differential_fold.rs:1807` **`binary_format_witnesses_match_every_release_on_path`**. Core unit tests `rust/tcl-cmd-core/src/binary.rs:1064` **`specifier_min_version_matches_measured_releases`** and `:1089` **`shared_specifier_table_covers_q_and_q_and_release_gate`** pin field ownership. The direct format evaluator calls `specifiers(format, false)` at `builtins.rs:939`; do not claim that this evaluator invokes `signedness_available` just because that helper exists at `binary.rs:358`.
- **Expression:** `FoldValue` has no ByteArray rung (`rust/tcl-compiler/src/tcl_expr_eval.rs:571`); the inspected expression services convert exact textual/numeric operands at `:1736`. This is a structural model limitation, not an identified independent byte-axis conditional guard.
- **Declared:** `rust/tcl-engine-tclvm/src/lib.rs:394` returns `Value::string(value.to_str())`; `rust/tcl-registry/src/value_transfer/declared.rs:594` publishes that protocol text with `ExactValue::from_literal`. The inspected boundary does not establish the matrix's bytes-or-decline promise. `folded_types_state_what_each_route_constructed` explicitly states pack semantic type without construction representation. No representation-preserving declared-binary boundary test was identified in the inspected set. This evidence gap is **not** promoted here to a demonstrated optimiser regression or a finding; execution/witness work would be required for that claim.

### Platform behaviour — matrix `:1676`

- **Direct:** `rust/tcl-registry/src/value_transfer/const_ops.rs:519` rejects Platform; `:845` also poisons host-long reads. Unit test `:1096` **`the_two_host_axes_never_admit`** pins refusal, although its assertion accepts either Platform or WallClock variant for either input, so it does not pin exact axis identity.
- **Expression:** `rust/tcl-compiler/src/tcl_expr_eval.rs:2076` refuses a platform-dependent answer. The direct index oracle `rust/tcl-registry/tests/differential_fold.rs:858` **`an_index_reads_as_each_release_reads_it`** exercises host-long-sensitive refusals, but is not an expression platform oracle.
- **Declared:** `rust/tcl-spec-hooks/src/host.rs:241` restricts commands and `:246` confines stores; `rust/tcl-engine-tclvm/src/lib.rs:677` enables confinement. Engine unit tests `:1258` **`a_confined_engine_reads_no_host_environment`** and `:1110` **`confine_stores_refuses_every_store_outside_the_activation`** pin negative capability and activation isolation. They do not certify unrelated semantic axes.

### Completion semantics and prefix rule — matrix `:1677`

- **Direct:** `rust/tcl-registry/src/value_transfer/answers.rs:729` defines completion outcomes; `:965` validates answer structure. Registry regex emits raised completion at `rust/tcl-registry/src/value_transfer/regex.rs:197`. Named pins: `rust/tcl-registry/tests/value_transfers.rs:7316` **`a_route_error_is_a_completion`**, `:7426` **`writing_to_an_array_is_the_commands_error_with_its_prefix`**, and `rust/tcl-registry/tests/differential_fold.rs:3227` **`the_prefix_rule_over_the_routes_matches_every_release_on_path`**.
- **Expression:** `rust/tcl-compiler/src/tcl_expr_eval.rs:1959` identifies integer division by zero; `:1921` propagates ended nested completion. `rust/tcl-compiler/src/value_transfer.rs:5412` constructs the expression's error completion with its write prefix, then `:5453` publishes completion and `:5454` nested writes. Existing pins include **`expr_acceptance_list`** at `rust/tcl-compiler/tests/value_transfer_witnesses.rs:1284` (short circuit, ternary, `1/0`) and the cross-route prefix oracle above. **`a_closed_catch_script_gives_its_result_variable_what_it_returned`** at `rust/tcl-compiler/tests/value_transfer_witnesses.rs:4297` is stronger result-message evidence: its `expr {1/0}` witness at `:4276` checks `divide by zero` in the lattice, O100 rewrite, and available runtimes. Merely asserting no lattice value for `1/0` is weaker than pinning its exact completion fields.
- **Declared:** `rust/tcl-registry/src/value_transfer/route.rs:318` declares `CompletionSupport::NormalOnly`; `rust/tcl-registry/src/value_transfer/declared.rs:615` publishes Normal only. `rust/tcl-engine-tclvm/src/lib.rs:852` **`an_error_in_the_body_is_reported_not_propagated`** is an engine unit pin; `rust/tcl-spec-hooks/tests/containment_e2e.rs:217` **`an_erroring_hook_abstains_forever_but_is_logged_once`** uses a test engine to pin host Abstain/log-once/no-quarantine behaviour. The declared matrix column supports Normal and treats Script errors as declines; it does not inherit direct prefix-completion support.

### Wall clock and locale — matrix `:1678`

- **Direct:** `rust/tcl-registry/src/value_transfer/const_ops.rs:519` rejects WallClock; unit test `:1096` **`the_two_host_axes_never_admit`** pins refusal, with the exact-axis limitation described above.
- **Expression:** `rust/tcl-compiler/src/tcl_expr_eval.rs:1935` refuses generator math functions. Nested ambient calls use their declared route and cannot supply an exact unsupported result. No standalone expression locale-conformance certificate is claimed by the inspected pins.
- **Declared:** `rust/tcl-spec-hooks/src/sandbox.rs:37` is the allowlist, which omits ambient `clock` and `after`; `rust/tcl-spec-hooks/src/host.rs:249` arms execution budgets. Generator refusal comes from `rust/tcl-vm/src/interp.rs:4409` (`confine_generator`), enabled by confinement at `rust/tcl-engine-tclvm/src/lib.rs:677`: command restrictions alone cannot remove Tcl 8.4's expression builtins. `rust/tcl-spec-hooks/tests/containment_e2e.rs:296` **`the_generator_is_refused_under_every_pinned_release`** pins the four profiles actually listed; engine unit test `rust/tcl-engine-tclvm/src/lib.rs:928` **`the_wall_clock_budget_stops_a_loop_the_command_budget_cannot_see`** pins timeout containment. A timeout that bounds computation does not expose a wall-clock fact to analysis; the timeout test is not a locale oracle.

### Execution and mutation limits

- This map states the source-inspected assertion scope. Actual execution and mutation results are in the command ledger below; an existing test name alone is not evidence of successful execution.
- `rust/tcl-registry/tests/differential_fold.rs:42` states the missing-interpreter policy: releases are skipped unless required by `TCL_LSP_REQUIRE_TCLSH`. The compiler witness helper likewise enumerates interpreters on `PATH`; the SpecTcl derived-body oracle explicitly skips missing shells at `rust/tcl-spectcl/tests/pack_source_e2e.rs:545`. A test's name cannot establish which releases actually executed in the current review.
- The design explicitly says **`direct_route_needs_match_their_cores` is not built** at `docs/design/compiler/value-evaluation.md:42`. Existing **`the_cores_the_routes_call_read_only_admitted_axes`**, `rust/tcl-registry/tests/value_transfers.rs:4993`, probes incr/list-append/index/append/dict/list-length/string-length and several Needs sets. It does not enumerate every direct evaluator in the catalogue.
- Meaningful conditional guards exist for concrete admission, target selection, route dispatch, resource refusal, answer validation, and completion handling. A source-inspected test still requires a mutation execution before its ability to detect removal of that guard is established.
- No single meaningful runtime guard encodes the full structural clauses “command-specific code stays in registry,” “no new command-name/ID arms,” “shipped specialisations are Rust,” or migration-ledger completeness. Ruling 6's historical phase/defer list has no meaningful guard to mutate. Lack of a full conformance table also supplies no specific guard to mutate.
- This is an evidence map, not an additional findings list. Source-confirmed documentation drift and uncovered assertion scope are recorded as limits; unexecuted suspicions are not presented as demonstrated behavioural defects.

### Targeted guard mutation results

Thirteen distinct mutations were executed: twelve were caught by successfully built tests and one survived (S4). The final regexp-failure mutation was caught by a secondary result-work budget refusal, so that result does not prove that its precision guard alone is pinned. All temporary source edits were restored and verified against HEAD.

#### Mutation run log

A failed mutant is counted as killed only when it built successfully and the named test assertion failed. The run log retains baseline and restored-source results; a zero-test run or a compiler error is not evidence of a killed mutation.

| Step | Exact command | Exit | Test result | Seconds |
|---|---|---:|---|---:|
| mutation-outcome-write | `cargo test -p tcl-registry --lib --test value_transfers -- --nocapture` | 0 | ok. 1048 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 25.49s<br>ok. 80 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.21s | — |
| mutation-outcome-restored | `cargo test -p tcl-registry --lib value_transfer::declared -- --nocapture` | 0 | ok. 2 passed; 0 failed; 0 ignored; 0 measured; 1046 filtered out; finished in 0.00s | — |
| mutation-target-baseline | `cargo test -p tcl-spec-hooks --test families_e2e a_silent_target_declines_the_whole_answer -- --nocapture` | 0 | ok. 2 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out; finished in 0.85s | — |
| mutation-silent-target | `cargo test -p tcl-spec-hooks --test families_e2e a_silent_target_declines_the_whole_answer -- --nocapture` | 101 | FAILED. 0 passed; 2 failed; 0 ignored; 0 measured; 38 filtered out; finished in 0.48s | — |
| mutation-target-restored | `cargo test -p tcl-spec-hooks --test families_e2e a_silent_target_declines_the_whole_answer -- --nocapture` | 0 | ok. 2 passed; 0 failed; 0 ignored; 0 measured; 38 filtered out; finished in 0.51s | — |
| mutation-worktree-diff-check | `git diff --exit-code` | 0 | clean source diff | — |
| numeric-unanimity-baseline | `cargo test -p tcl-registry --lib numerals_read_under_the_named_grammar_or_the_unanimous_one -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.01s | 49.596 |
| numeric-unanimity-mutant | `cargo test -p tcl-registry --lib numerals_read_under_the_named_grammar_or_the_unanimous_one -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.01s | 148.52 |
| numeric-unanimity-restored | `cargo test -p tcl-registry --lib numerals_read_under_the_named_grammar_or_the_unanimous_one -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.00s | 50.949 |
| numeric-unanimity-clean | `git diff --exit-code` | 0 | clean source diff | 0.029 |
| character-divergence-baseline | `cargo test -p tcl-registry --lib a_declared_base_release_is_the_release -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.00s | 0.224 |
| character-divergence-mutant | `cargo test -p tcl-registry --lib a_declared_base_release_is_the_release -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.00s | 107.137 |
| character-divergence-restored | `cargo test -p tcl-registry --lib a_declared_base_release_is_the_release -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.00s | 96.676 |
| character-divergence-clean | `git diff --exit-code` | 0 | clean source diff | 0.084 |
| writes-first-baseline | `cargo test -p tcl-registry --test value_transfers the_ordered_state_reads_its_own_writes_first -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.00s | 6.613 |
| writes-first-mutant | `cargo test -p tcl-registry --test value_transfers the_ordered_state_reads_its_own_writes_first -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.00s | 107.362 |
| writes-first-restored | `cargo test -p tcl-registry --test value_transfers the_ordered_state_reads_its_own_writes_first -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.00s | 70.172 |
| writes-first-clean | `git diff --exit-code` | 0 | clean source diff | 0.04 |
| binding-admission-baseline | `cargo test -p tcl-compiler --test value_transfer_witnesses a_rebound_nested_head_declines_the_expression -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 1.56s | 77.303 |
| binding-admission-baseline | `cargo test -p tcl-compiler --test value_transfer_witnesses a_rebound_nested_head_declines_the_expression -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 2.80s | 3.142 |
| binding-admission-mutant | `cargo test -p tcl-compiler --test value_transfer_witnesses a_rebound_nested_head_declines_the_expression -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.21s | 74.569 |
| binding-admission-restored | `cargo test -p tcl-compiler --test value_transfer_witnesses a_rebound_nested_head_declines_the_expression -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 1.59s | 76.271 |
| binding-admission-clean | `git diff --exit-code` | 0 | clean source diff | 0.034 |
| workspace-authority-baseline | `cargo test -p tcl-compiler --test value_transfer_witnesses the_completion_test_needs_no_consumer_edit -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 2.39s | 2.563 |
| workspace-authority-mutant | `cargo test -p tcl-compiler --test value_transfer_witnesses the_completion_test_needs_no_consumer_edit -- --nocapture` | 101 | See run output; no test-result line | 34.469 |
| workspace-authority-restored | `cargo test -p tcl-compiler --test value_transfer_witnesses the_completion_test_needs_no_consumer_edit -- --nocapture` | 101 | See run output; no test-result line | 11.894 |
| workspace-authority-baseline | `cargo test -p tcl-compiler --test value_transfer_witnesses the_completion_test_needs_no_consumer_edit -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 1.69s | 33.955 |
| workspace-authority-mutant | `cargo test -p tcl-compiler --test value_transfer_witnesses the_completion_test_needs_no_consumer_edit -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.19s | 31.309 |
| workspace-authority-restored | `cargo test -p tcl-compiler --test value_transfer_witnesses the_completion_test_needs_no_consumer_edit -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 1.68s | 34.074 |
| workspace-authority-clean | `git diff --exit-code` | 0 | clean source diff | 0.013 |
| builtin-route-baseline | `cargo test -p tcl-registry --test value_transfers shipped_builtins_stay_on_the_direct_route -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 2.21s | 33.025 |
| builtin-route-mutant | `cargo test -p tcl-registry --test value_transfers shipped_builtins_stay_on_the_direct_route -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 2.60s | 68.815 |
| builtin-route-restored | `cargo test -p tcl-registry --test value_transfers shipped_builtins_stay_on_the_direct_route -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 3.20s | 68.761 |
| builtin-route-clean | `git diff --exit-code` | 0 | clean source diff | 0.051 |
| binary-release-baseline | `cargo test -p tcl-registry --test differential_fold binary_format_witnesses_match_every_release_on_path -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 26 filtered out; finished in 2.40s | 4.231 |
| binary-release-mutant | `cargo test -p tcl-registry --test differential_fold binary_format_witnesses_match_every_release_on_path -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 26 filtered out; finished in 0.11s | 73.42 |
| binary-release-restored | `cargo test -p tcl-registry --test differential_fold binary_format_witnesses_match_every_release_on_path -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 26 filtered out; finished in 1.60s | 65.43 |
| binary-release-clean | `git diff --exit-code` | 0 | clean source diff | 0.04 |
| regex-precision-baseline | `cargo test -p tcl-registry --test value_transfers a_regexp_that_established_nothing_declines -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.01s | 2.938 |
| regex-precision-mutant | `cargo test -p tcl-registry --test value_transfers a_regexp_that_established_nothing_declines -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.02s | 70.672 |
| regex-precision-restored | `cargo test -p tcl-registry --test value_transfers a_regexp_that_established_nothing_declines -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.03s | 69.888 |
| regex-precision-clean | `git diff --exit-code` | 0 | clean source diff | 0.017 |
| host-axes-baseline | `cargo test -p tcl-registry --lib the_two_host_axes_never_admit -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.00s | 25.108 |
| host-axes-mutant | `cargo test -p tcl-registry --lib the_two_host_axes_never_admit -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.01s | 96.695 |
| host-axes-restored | `cargo test -p tcl-registry --lib the_two_host_axes_never_admit -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.00s | 97.079 |
| host-axes-clean | `git diff --exit-code` | 0 | clean source diff | 0.019 |
| concrete-last-write-baseline | `cargo test -p tcl-compiler --test value_transfer_witnesses the_repeated_target_witness -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.98s | 68.797 |
| concrete-last-write-mutant | `cargo test -p tcl-compiler --test value_transfer_witnesses the_repeated_target_witness -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.10s | 46.19 |
| concrete-last-write-restored | `cargo test -p tcl-compiler --test value_transfer_witnesses the_repeated_target_witness -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.96s | 64.817 |
| concrete-last-write-clean | `git diff --exit-code` | 0 | clean source diff | 0.021 |
| final-clean | `git diff --exit-code` | 0 | clean source diff | 0.033 |
| regex-answer-on-failure-baseline | `cargo test -p tcl-registry --test value_transfers a_regexp_that_established_nothing_declines -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.01s | 47.097 |
| regex-answer-on-failure-mutant | `cargo test -p tcl-registry --test value_transfers a_regexp_that_established_nothing_declines -- --nocapture` | 101 | FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.01s | 76.758 |
| regex-answer-on-failure-restored | `cargo test -p tcl-registry --test value_transfers a_regexp_that_established_nothing_declines -- --nocapture` | 0 | ok. 1 passed; 0 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.01s | 67.476 |
| regex-answer-on-failure-clean | `git diff --exit-code` | 0 | clean source diff | 0.022 |
| final-clean | `git diff --exit-code` | 0 | clean source diff | 0.008 |

#### Exact expanded mutations and observed failures

#### Host failure and recovery

The first workspace-authority mutant and restored builds failed before executing tests because rustc could not create temporary directories (`No space left on device`). These are host failures in the run log, not killed mutations or branch test failures. The source had already been restored; `git diff --exit-code` returned 0. After filesystem recovery, the affected compiler and BIG-IP artefacts/fingerprints were cleaned with:

```sh
cargo clean -p tcl-compiler -p tcl-bigip
```

The clean removed 58 files / 866.2 MiB. The authority baseline, mutant and restored tests were then rerun; the later rows are the usable evidence.

#### binary-release

```diff
diff --git a/rust/tcl-registry/src/value_transfer/builtins.rs b/rust/tcl-registry/src/value_transfer/builtins.rs
index ca33e9c21..507204834 100644
--- a/rust/tcl-registry/src/value_transfer/builtins.rs
+++ b/rust/tcl-registry/src/value_transfer/builtins.rs
@@ -938,7 +938,7 @@ impl BinaryFormatSemantics {
         let mut next = args.iter();
         for field in tcl_cmd_core::binary::specifiers(format.as_bytes(), false) {
             if let Some(floor) = tcl_cmd_core::binary::specifier_min_version(field.letter)
-                && release.is_none_or(|release| release < floor)
+                && false && release.is_none_or(|release| release < floor)
             {
                 return Some(DeclineReason::ReleaseAmbiguous(Axis::Availability(
                     tcl_dialect::model::SpecSurface::TCL85_PLUS[0],
```

Observed mutant output:

```text
thread 'binary_format_witnesses_match_every_release_on_path' (193187) panicked at rust/tcl-registry/tests/differential_fold.rs:1826:38:
tclsh8.4 raises on binary format ["t n m", "5", "5", "5"], the route answered 0500050000000500000000000000
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test binary_format_witnesses_match_every_release_on_path ... FAILED

failures:

failures:
    binary_format_witnesses_match_every_release_on_path

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 26 filtered out; finished in 0.11s

error: test failed, to rerun pass `-p tcl-registry --test differential_fold`
```

#### binding-admission

```diff
diff --git a/rust/tcl-compiler/src/value_transfer.rs b/rust/tcl-compiler/src/value_transfer.rs
index e3456db34..a9f521ff9 100644
--- a/rust/tcl-compiler/src/value_transfer.rs
+++ b/rust/tcl-compiler/src/value_transfer.rs
@@ -2550,7 +2550,7 @@ impl<'a> LatticeDriver<'a> {
     pub(crate) fn trusted(&self, head: &str) -> bool {
         self.folds.is_some_and(|f| match f.trust {
             FoldTrust::WholeModule => f.mutations.trusts(head),
-            FoldTrust::ObservedBindings => f.mutations.observed_binding_is_the_builtin(head),
+            FoldTrust::ObservedBindings => true,
         })
     }
```

Observed mutant output:

```text
thread 'a_rebound_nested_head_declines_the_expression' (179098) panicked at rust/tcl-compiler/tests/value_transfer_witnesses.rs:1089:9:
assertion `left == right` failed: tcl8.4
  left: Some(Const(Int(4)))
 right: Some(Overdefined)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test a_rebound_nested_head_declines_the_expression ... FAILED

failures:

failures:
    a_rebound_nested_head_declines_the_expression

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.21s

error: test failed, to rerun pass `-p tcl-compiler --test value_transfer_witnesses`
```

#### builtin-route

```diff
diff --git a/rust/tcl-registry/src/value_transfer/builtins.rs b/rust/tcl-registry/src/value_transfer/builtins.rs
index ca33e9c21..2e8595c80 100644
--- a/rust/tcl-registry/src/value_transfer/builtins.rs
+++ b/rust/tcl-registry/src/value_transfer/builtins.rs
@@ -243,9 +243,7 @@ impl CommandSemantics for StringRangeSemantics {
     }

     fn route(&self) -> EvalRoute {
-        EvalRoute::Direct {
-            id: NativeEvalId::StringRange,
-        }
+        EvalRoute::None { reason: super::decline::NoRouteReason::Declared }
     }

     fn transfer(
```

Observed mutant output:

```text
thread 'shipped_builtins_stay_on_the_direct_route' (186790) panicked at rust/tcl-registry/tests/value_transfers.rs:4304:5:
a shipped route moved: [("string range", "direct:string-range", "registry")]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test shipped_builtins_stay_on_the_direct_route ... FAILED

failures:

failures:
    shipped_builtins_stay_on_the_direct_route

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 2.60s

error: test failed, to rerun pass `-p tcl-registry --test value_transfers`
```

#### character-divergence

```diff
diff --git a/rust/tcl-registry/src/value_transfer/const_ops.rs b/rust/tcl-registry/src/value_transfer/const_ops.rs
index d4810012d..a16bb7ac4 100644
--- a/rust/tcl-registry/src/value_transfer/const_ops.rs
+++ b/rust/tcl-registry/src/value_transfer/const_ops.rs
@@ -345,7 +345,7 @@ impl TargetSemantics {
             .filter(|numbers| Some(*numbers) == profile.map(|p| p.grammar.numbers));
         let character_model = release
             .map(TclVersion::string_character_model)
-            .filter(|model| Some(*model) == profile.and_then(DialectProfile::character_model));
+            .filter(|_model| true);
         Self {
             release,
             numerals,
```

Observed mutant output:

```text
thread 'value_transfer::const_ops::tests::a_declared_base_release_is_the_release' (157583) panicked at rust/tcl-registry/src/value_transfer/const_ops.rs:1477:9:
assertion `left == right` failed: the F5 character model is declared, and not 8.4's
  left: Some(BmpCharsElseUtf8Bytes)
 right: None
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test value_transfer::const_ops::tests::a_declared_base_release_is_the_release ... FAILED

failures:

failures:
    value_transfer::const_ops::tests::a_declared_base_release_is_the_release

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p tcl-registry --lib`
```

#### concrete-last-write

```diff
diff --git a/rust/tcl-compiler/src/value_transfer.rs b/rust/tcl-compiler/src/value_transfer.rs
index e3456db34..6563114cf 100644
--- a/rust/tcl-compiler/src/value_transfer.rs
+++ b/rust/tcl-compiler/src/value_transfer.rs
@@ -2962,7 +2962,7 @@ impl<'a> LatticeDriver<'a> {
                     };
                 }
                 let mut held: Option<LatticeValue> = None;
-                for (_, store) in &named {
+                for (_, store) in named.iter().rev() {
                     match store {
                         StoreOutcome::Write { value, .. }
                         | StoreOutcome::WriteElement { value, .. } => {
```

Observed mutant output:

```text
thread 'the_repeated_target_witness' (215632) panicked at rust/tcl-compiler/tests/value_transfer_witnesses.rs:2570:9:
assertion `left == right` failed: tcl8.4: the second position's capture is the one that survives
  left: Const(String("a"))
 right: Const(String("b"))
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test the_repeated_target_witness ... FAILED

failures:

failures:
    the_repeated_target_witness

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.10s

error: test failed, to rerun pass `-p tcl-compiler --test value_transfer_witnesses`
```

#### host-axes

```diff
diff --git a/rust/tcl-registry/src/value_transfer/const_ops.rs b/rust/tcl-registry/src/value_transfer/const_ops.rs
index d4810012d..bbd47bc26 100644
--- a/rust/tcl-registry/src/value_transfer/const_ops.rs
+++ b/rust/tcl-registry/src/value_transfer/const_ops.rs
@@ -520,7 +520,7 @@ impl<'ctx> ConstOps<'ctx> {
             (Needs::PLATFORM, Axis::Platform),
             (Needs::WALL_CLOCK, Axis::WallClock),
         ] {
-            if needs.contains(bit) {
+            if false && needs.contains(bit) {
                 return Err(DeclineReason::ReleaseAmbiguous(axis));
             }
         }
```

Observed mutant output:

```text
thread 'value_transfer::const_ops::tests::the_two_host_axes_never_admit' (204487) panicked at rust/tcl-registry/src/value_transfer/const_ops.rs:1100:13:
Needs(16384): None
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test value_transfer::const_ops::tests::the_two_host_axes_never_admit ... FAILED

failures:

failures:
    value_transfer::const_ops::tests::the_two_host_axes_never_admit

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p tcl-registry --lib`
```

#### numeric-unanimity

```diff
diff --git a/rust/tcl-registry/src/value_transfer/const_ops.rs b/rust/tcl-registry/src/value_transfer/const_ops.rs
index d4810012d..8787d6fe9 100644
--- a/rust/tcl-registry/src/value_transfer/const_ops.rs
+++ b/rust/tcl-registry/src/value_transfer/const_ops.rs
@@ -801,7 +801,7 @@ impl<'ctx> ConstOps<'ctx> {
         };
         if let Some(numbers) = self.target.numerals {
             parse(numbers)
-        } else if let Some(answer) = NumberSyntax::unanimous(parse) {
+        } else if let Some(answer) = Some(parse(TclVersion::V9_1.number_syntax())) {
             answer
         } else {
             self.poison(DeclineReason::ReleaseAmbiguous(Axis::NumeralGrammar));
```

Observed mutant output:

```text
thread 'value_transfer::const_ops::tests::numerals_read_under_the_named_grammar_or_the_unanimous_one' (149030) panicked at rust/tcl-registry/src/value_transfer/const_ops.rs:1153:13:
None
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test value_transfer::const_ops::tests::numerals_read_under_the_named_grammar_or_the_unanimous_one ... FAILED

failures:

failures:
    value_transfer::const_ops::tests::numerals_read_under_the_named_grammar_or_the_unanimous_one

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1047 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p tcl-registry --lib`
```

#### regex-answer-on-failure

```diff
diff --git a/rust/tcl-registry/src/value_transfer/regex.rs b/rust/tcl-registry/src/value_transfer/regex.rs
index 20d448f32..69a4d185d 100644
--- a/rust/tcl-registry/src/value_transfer/regex.rs
+++ b/rust/tcl-registry/src/value_transfer/regex.rs
@@ -268,7 +268,7 @@ impl RegexpSemantics {
         });
         let publication = match run {
             Err(reason) => return EvalAnswer::Declined(reason),
-            Ok(Err(failure)) => return failed(&ops, &failure, NativeEvalId::RegexpMatch),
+            Ok(Err(_failure)) => Publication::preserving(ConstValue::int(0), TclType::Int, &targets),
             Ok(Ok(RegexpResult::Inline(list))) => {
                 Publication::preserving(list, TclType::List, &targets)
             }
```

Observed mutant output:

```text
thread 'a_regexp_that_established_nothing_declines' (228276) panicked at rust/tcl-registry/tests/value_transfers.rs:5490:5:
assertion `left == right` failed
  left: Declined(Budget(Fuel))
 right: Declined(Approximate)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test a_regexp_that_established_nothing_declines ... FAILED

failures:

failures:
    a_regexp_that_established_nothing_declines

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.01s

error: test failed, to rerun pass `-p tcl-registry --test value_transfers`
```

#### regex-precision

```diff
diff --git a/rust/tcl-registry/src/value_transfer/regex.rs b/rust/tcl-registry/src/value_transfer/regex.rs
index 20d448f32..9a70c8d3e 100644
--- a/rust/tcl-registry/src/value_transfer/regex.rs
+++ b/rust/tcl-registry/src/value_transfer/regex.rs
@@ -180,7 +180,7 @@ pub(super) fn failure_reason(ops: &ConstOps<'_>, failure: &RegexFailure) -> Decl
             PrecisionDecline::FuelExhausted { .. }
             | PrecisionDecline::DepthExhausted { .. }
             | PrecisionDecline::ApproximateCapture { .. },
-        ) => DeclineReason::Approximate,
+        ) => DeclineReason::Unsupported,
         RegexFailure::Declined(PrecisionDecline::FormUnsupported { .. }) => {
             DeclineReason::Unsupported
         }
```

Observed mutant output:

```text
thread 'a_regexp_that_established_nothing_declines' (198516) panicked at rust/tcl-registry/tests/value_transfers.rs:5490:5:
assertion `left == right` failed
  left: Declined(Unsupported)
 right: Declined(Approximate)
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test a_regexp_that_established_nothing_declines ... FAILED

failures:

failures:
    a_regexp_that_established_nothing_declines

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.02s

error: test failed, to rerun pass `-p tcl-registry --test value_transfers`
```

#### workspace-authority

```diff
diff --git a/rust/tcl-compiler/src/value_transfer.rs b/rust/tcl-compiler/src/value_transfer.rs
index e3456db34..3089d26d3 100644
--- a/rust/tcl-compiler/src/value_transfer.rs
+++ b/rust/tcl-compiler/src/value_transfer.rs
@@ -4017,7 +4017,7 @@ impl<'a> LatticeDriver<'a> {
             // and runs the body in this thread's host.
             EvalRoute::Implementation(_) => {
                 self.enter_implementation();
-                evaluate_lifted(semantics, inputs, &mut self.budget(), MAX_CONSTSET_SIZE)
+                LiftedAnswer::Declined(DeclineReason::Unsupported)
             }
         }
     }
```

Observed mutant output:

```text
thread 'the_completion_test_needs_no_consumer_edit' (182176) panicked at rust/tcl-compiler/tests/value_transfer_witnesses.rs:2111:5:
assertion `left == right` failed: tcl8.4 tenant::label
  left: ["declined: unsupported", "declined: unsupported"]
 right: ["declined: unsupported", "declined: not-exact"]
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test the_completion_test_needs_no_consumer_edit ... FAILED

failures:

failures:
    the_completion_test_needs_no_consumer_edit

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 169 filtered out; finished in 0.19s

error: test failed, to rerun pass `-p tcl-compiler --test value_transfer_witnesses`
```

#### writes-first

```diff
diff --git a/rust/tcl-registry/src/value_transfer/inputs.rs b/rust/tcl-registry/src/value_transfer/inputs.rs
index 335658d35..12fbcd136 100644
--- a/rust/tcl-registry/src/value_transfer/inputs.rs
+++ b/rust/tcl-registry/src/value_transfer/inputs.rs
@@ -427,7 +427,7 @@ pub enum WrittenPlace {
 pub fn written_in(writes: &[(PlaceRef, StoreOutcome)], name: &str) -> WrittenPlace {
     let read = PlaceRef::scalar(name);
     let mut fact = WrittenPlace::Untouched;
-    for (place, store) in writes {
+    for (place, store) in writes.iter().rev() {
         let left = match store {
             StoreOutcome::Preserve { .. } => continue,
             StoreOutcome::Write { value, .. } | StoreOutcome::WriteElement { value, .. } => {
```

Observed mutant output:

```text
thread 'the_ordered_state_reads_its_own_writes_first' (167228) panicked at rust/tcl-registry/tests/value_transfers.rs:5228:5:
assertion `left == right` failed: the last write decides
  left: Exact(ExactValue { bytes: [49], numeric: None, representation: Unknown })
 right: Exact(ExactValue { bytes: [50], numeric: None, representation: Unknown })
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace
test the_ordered_state_reads_its_own_writes_first ... FAILED

failures:

failures:
    the_ordered_state_reads_its_own_writes_first

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 79 filtered out; finished in 0.00s

error: test failed, to rerun pass `-p tcl-registry --test value_transfers`
```


#### Final restoration and binary hashes

```text
git diff --exit-code: 0
git status --porcelain=v1: empty
All 7 temporarily changed source blobs match HEAD.
```

All baseline/restored test steps shown above executed nonzero tests. The ENOSPC builds executed no tests and are excluded from mutation classification. The final source tree is byte identical to HEAD for every temporarily changed file.

| Pristine final test binary | SHA-256 |
|---|---|
| `/workspace/tcl-lsp-registry-review/target/debug/deps/differential_fold-58d4571d17c2903d` | `db586622903aa139b2a1ab3c2c381a6c3ef5305655d0dded0705dc4db3070eef` |
| `/workspace/tcl-lsp-registry-review/target/debug/deps/families_e2e-fd1a086c666b766d` | `cb2a632e6bea1a1334173f589613f7291d2adf5f1cabdd07d30de7ed53e4038f` |
| `/workspace/tcl-lsp-registry-review/target/debug/deps/tcl_registry-fb0a92ae77ec54aa` | `5180b92ab347cdca185f1a6698d76ee98eed08f35c1344e91701254a67e474cd` |
| `/workspace/tcl-lsp-registry-review/target/debug/deps/value_transfer_witnesses-11eba072462a2de3` | `14c940f31e6b65ed9122b632c5410a8b048e92ae65128f0aeede8d1730b75022` |
| `/workspace/tcl-lsp-registry-review/target/debug/deps/value_transfers-5ccfb18dd6f03f28` | `696fca4b5ec6b3883cbc12b5a0948a2e85a9ee2b09559259af812d6139d1930d` |

After recording the final hashes and clean source state, removed only `/workspace/tcl-lsp-registry-review/target` (2.5G generated build artefacts) to release space for the continuing main CI queue. The source worktree was retained and `git diff --exit-code` remained 0.



## Runtime reproduction appendix

Tip standalone manifest (path dependencies are the reviewed main tree):

```toml
[package]
name = "runtime-review-repro"
version = "0.1.0"
edition = "2024"
[workspace]
[dependencies]
tcl-engine-api = { path = "/workspace/tcl-lsp/rust/tcl-engine-api" }
tcl-engine-tclvm = { path = "/workspace/tcl-lsp/rust/tcl-engine-tclvm" }
tcl-runtime = { path = "/workspace/tcl-lsp/runtime/rust", features = ["engine"] }
```

Tip fixed-input source:

```rust
use std::rc::Rc;
use tcl_engine_api::{Budget, BudgetKind, CompileUnit, Engine, EngineError, HostCommand, HostOutcome, Value};
use tcl_engine_tclvm::TclVmEngine;
use tcl_runtime::engine::RuntimeEngine;
struct Exhausted;
impl HostCommand for Exhausted {
    fn invoke(&self, _: &[Value]) -> Result<HostOutcome, EngineError> {
        Err(EngineError::BudgetExceeded(BudgetKind::Commands))
    }
}
fn examine<E: Engine>(mut engine: E) {
    engine.define_command("exhausted", Rc::new(Exhausted)).unwrap();
    for body in [
        "catch {exhausted}; return exact",
        "catch {string repeat abcdefgh 100}; return exact",
        "return [binary format H2 ff]",
        "return [binary format H2 00]",
        "return [lrepeat 100 abcdefgh]",
        "return [string cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh]",
    ] {
        engine.set_budget(Budget::of_commands(10000).with_max_value_bytes(64)).unwrap();
        let h = engine.compile(CompileUnit {name: "repro", parameters: &[], body}).unwrap();
        println!("{} | {} | {:?}", engine.name(), body, engine.invoke(&h, &[]));
    }
}
fn package_result() {
    let mut i = tcl_runtime::interp::Interp::new();
    i.set_result_bytes(b"preserved");
    let code = unsafe { tcl_runtime::capi::Tcl_PkgProvideEx(&mut i, c"review".as_ptr(), c"1.0".as_ptr(), std::ptr::null()) };
    println!("runtime pkg-result | {code} | {:?}", i.result_bytes());
    i.set_result_bytes(b"again");
    let code = unsafe { tcl_runtime::capi::Tcl_PkgProvideEx(&mut i, c"review".as_ptr(), c"1.0".as_ptr(), std::ptr::null()) };
    println!("runtime pkg-result repeated | {code} | {:?}", i.result_bytes());
}
fn confinement<E: Engine>(mut engine: E) {
    let h = engine.compile(CompileUnit {name: "seed", parameters: &[], body: "set ::seed old"}).unwrap();
    engine.invoke(&h, &[]).unwrap();
    engine.restrict_commands(&["set", "array", "unset", "return", "info"]).unwrap();
    engine.confine_stores().unwrap();
    for body in ["array set ::fresh {}; return [info exists ::fresh]", "return [info exists ::fresh]", "unset ::seed; return [info exists ::seed]", "return [info exists ::seed]"] {
        let h = engine.compile(CompileUnit {name: "repro", parameters: &[], body}).unwrap();
        println!("{} confined | {} | {:?}", engine.name(), body, engine.invoke(&h, &[]));
    }
}
fn main() {
    package_result();
    examine(TclVmEngine::new());
    examine(RuntimeEngine::new());
    confinement(TclVmEngine::new());
    confinement(RuntimeEngine::new());
}
```

Tip command, run from `/workspace/tcl-lsp`:

```bash
source /tmp/spectcl-review/build-env.sh
export CARGO_TARGET_DIR=/workspace/runtime-review-target
export CARGO_BUILD_JOBS=1
cargo run --manifest-path /tmp/spectcl-review/runtime-repro/Cargo.toml --offline > /tmp/spectcl-review/runtime-repro.log 2>&1
```

Tip complete final-run output:

```text
   Compiling runtime-review-repro v0.1.0 (/tmp/spectcl-review/runtime-repro)
    Finished `dev` profile [unoptimized] target(s) in 1.39s
     Running `/workspace/runtime-review-target/debug/runtime-review-repro`
runtime pkg-result | 0 | []
runtime pkg-result repeated | 0 | []
tclvm | catch {exhausted}; return exact | Ok(Str("exact"))
tclvm | catch {string repeat abcdefgh 100}; return exact | Ok(Str("exact"))
tclvm | return [binary format H2 ff] | Ok(Str("ÿ"))
tclvm | return [binary format H2 00] | Ok(Str("\0"))
tclvm | return [lrepeat 100 abcdefgh] | Ok(Str("abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh"))
tclvm | return [string cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh] | Ok(Str("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh"))
runtime | catch {exhausted}; return exact | Err(BudgetExceeded(Commands))
runtime | catch {string repeat abcdefgh 100}; return exact | Err(BudgetExceeded(ValueSize))
runtime | return [binary format H2 ff] | Ok(Str("ÿ"))
runtime | return [binary format H2 00] | Ok(Str("\0"))
runtime | return [lrepeat 100 abcdefgh] | Ok(Str("abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh"))
runtime | return [string cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh] | Ok(Str("abcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefghabcdefgh"))
tclvm confined | array set ::fresh {}; return [info exists ::fresh] | Ok(Str("1"))
tclvm confined | return [info exists ::fresh] | Ok(Str("1"))
tclvm confined | unset ::seed; return [info exists ::seed] | Ok(Str("0"))
tclvm confined | return [info exists ::seed] | Ok(Str("0"))
runtime confined | array set ::fresh {}; return [info exists ::fresh] | Ok(Str("1"))
runtime confined | return [info exists ::fresh] | Ok(Str("1"))
runtime confined | unset ::seed; return [info exists ::seed] | Ok(Str("0"))
runtime confined | return [info exists ::seed] | Ok(Str("0"))
```

Reference C API source:

```c
#include <stdio.h>
#include <tcl.h>
int main(void) {
    Tcl_FindExecutable("pkg-result");
    Tcl_Interp *i = Tcl_CreateInterp();
    Tcl_SetObjResult(i, Tcl_NewStringObj("preserved", -1));
    int code = Tcl_PkgProvide(i, "review", "1.0");
    printf("%d | %s\n", code, Tcl_GetStringResult(i));
    Tcl_SetObjResult(i, Tcl_NewStringObj("again", -1));
    code = Tcl_PkgProvide(i, "review", "1.0");
    printf("%d | %s\n", code, Tcl_GetStringResult(i));
    Tcl_DeleteInterp(i);
    Tcl_Finalize();
}
```

Reference command:

```bash
cc -I /workspace/tcl-lsp/tmp/tcl9.0.4/generic /tmp/spectcl-review/runtime-repro/pkg-result.c /workspace/tcl-lsp/tmp/tcl9.0.4/unix/libtcl9.0.a -lz -lm -ldl -lpthread -o /tmp/spectcl-review/runtime-repro/pkg-result-c-tcl
/tmp/spectcl-review/runtime-repro/pkg-result-c-tcl
```

Reference output:

```text
0 | preserved
0 | again
```

Base source (the old HostCommand interface returned `Value` rather than `HostOutcome`; the same body, budget and failure inputs are used):

```rust
use std::rc::Rc;
use tcl_engine_api::{Budget, BudgetKind, CompileUnit, Engine, EngineError, HostCommand, Value};
use tcl_engine_tclvm::TclVmEngine;
struct Exhausted;
impl HostCommand for Exhausted {
    fn invoke(&self, _: &[Value]) -> Result<Value, EngineError> {
        Err(EngineError::BudgetExceeded(BudgetKind::Commands))
    }
}
fn main() {
    let mut engine = TclVmEngine::new();
    engine.define_command("exhausted", Rc::new(Exhausted)).unwrap();
    for body in [
        "catch {exhausted}; return exact",
        "catch {string repeat abcdefgh 100}; return exact",
        "return [lrepeat 100 abcdefgh]",
        "return [string cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh]",
    ] {
        engine.set_budget(Budget::of_commands(10000).with_max_value_bytes(64)).unwrap();
        let h = engine.compile(CompileUnit { name: "repro", parameters: &[], body }).unwrap();
        println!("base tclvm | {} | {:?}", body, engine.invoke(&h, &[]).map(|v| v.as_str().unwrap().len()));
    }
}
```

Base command, linking the already-built base debug rlibs, without writing to the base target:

```bash
source /tmp/spectcl-review/build-env.sh
rustc --edition=2024 /tmp/spectcl-review/runtime-repro/base.rs -L dependency=/workspace/tcl-lsp-review-base/target/debug/deps --extern tcl_engine_api=/workspace/tcl-lsp-review-base/target/debug/deps/libtcl_engine_api-f789fd18f722c7b7.rlib --extern tcl_engine_tclvm=/workspace/tcl-lsp-review-base/target/debug/deps/libtcl_engine_tclvm-905525ad790dd2de.rlib -o /tmp/spectcl-review/runtime-repro/base-probe
/tmp/spectcl-review/runtime-repro/base-probe > /tmp/spectcl-review/runtime-base-repro.log
```

Base output (successful result byte lengths):

```text
base tclvm | catch {exhausted}; return exact | Ok(5)
base tclvm | catch {string repeat abcdefgh 100}; return exact | Ok(5)
base tclvm | return [lrepeat 100 abcdefgh] | Ok(899)
base tclvm | return [string cat abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh abcdefgh] | Ok(72)
```

Binary SHA-256:

```text
8323607f8e27305f382b5ba4288195bc3204aba37e4f21be25b7a5e4767ee6be  tip runtime-review-repro
061e19a3bcfa335eaab4d59bf0e636820cc2020a2c3e4300348208aa95da7e22  base-probe
59206196bb4bba03e8fe84322072f95df3bcd12d0b0ddc94607fa392df61daf8  pkg-result-c-tcl
```

Harness source SHA-256:

```text
8ac14d0f2b815421b13aa1304e90972f5819b2f600995dddba0195cc78339f29  Cargo.toml
af69cdda7849d39bb1908cacfd9f45522adeab7e95a57bd9218475fb68316e19  tip main.rs
e1b395b9b6a81a3ce39ca27ad166d6e745742f5d159e079431327e48b06396c2  base.rs
41ed1818e205f8e31612c76d23f45451d45c6c95db3655c0e8a8e11520ce5c2e  pkg-result.c
```

Exact five-release C API build/run log (same C harness above):

```text
COMMAND: cc -I /workspace/tcl-lsp/tmp/tcl8.4.20/generic /tmp/spectcl-review/runtime-repro/pkg-result.c /workspace/tcl-lsp/tmp/tcl8.4.20/unix/libtcl8.4.a -lz -lm -ldl -lpthread -o /tmp/spectcl-review/runtime-repro/pkg-result-tcl8.4.20
compile exit 0
Tcl 8.4.20 run exit 0
0 | preserved
0 | again

sha256 7de02100a90208195f65d024c4fda317ca60c73dceaaca5c47f1a6b276f77a73  /tmp/spectcl-review/runtime-repro/pkg-result-tcl8.4.20
COMMAND: cc -I /workspace/tcl-lsp/tmp/tcl8.5.19/generic /tmp/spectcl-review/runtime-repro/pkg-result.c /workspace/tcl-lsp/tmp/tcl8.5.19/unix/libtcl8.5.a -lz -lm -ldl -lpthread -o /tmp/spectcl-review/runtime-repro/pkg-result-tcl8.5.19
compile exit 0
Tcl 8.5.19 run exit 0
0 | preserved
0 | again

sha256 823065e8415278b24a83c1b96a93a89a9dcab8429472bf4005d75d4fe44c899c  /tmp/spectcl-review/runtime-repro/pkg-result-tcl8.5.19
COMMAND: cc -I /workspace/tcl-lsp/tmp/tcl8.6.18/generic /tmp/spectcl-review/runtime-repro/pkg-result.c /workspace/tcl-lsp/tmp/tcl8.6.18/unix/libtcl8.6.a -lz -lm -ldl -lpthread -o /tmp/spectcl-review/runtime-repro/pkg-result-tcl8.6.18
compile exit 0
Tcl 8.6.18 run exit 0
0 | preserved
0 | again

sha256 7d7d8968cf3e205aa1e50bd7e17777c34109d8c6711e1b635d01f1afe574df41  /tmp/spectcl-review/runtime-repro/pkg-result-tcl8.6.18
COMMAND: cc -I /workspace/tcl-lsp/tmp/tcl9.0.4/generic /tmp/spectcl-review/runtime-repro/pkg-result.c /workspace/tcl-lsp/tmp/tcl9.0.4/unix/libtcl9.0.a -lz -lm -ldl -lpthread -o /tmp/spectcl-review/runtime-repro/pkg-result-tcl9.0.4
compile exit 0
Tcl 9.0.4 run exit 0
0 | preserved
0 | again

sha256 59206196bb4bba03e8fe84322072f95df3bcd12d0b0ddc94607fa392df61daf8  /tmp/spectcl-review/runtime-repro/pkg-result-tcl9.0.4
COMMAND: cc -I /workspace/tcl-lsp/tmp/tcl9.1.0/generic /tmp/spectcl-review/runtime-repro/pkg-result.c /workspace/tcl-lsp/tmp/tcl9.1.0/unix/libtcl9.1.a -lz -lm -ldl -lpthread -o /tmp/spectcl-review/runtime-repro/pkg-result-tcl9.1.0
compile exit 0
Tcl 9.1.0 run exit 0
0 | preserved
0 | again

sha256 5ea25c9189201ba4eea180c9449aa2d63388bd9a2df5cc8359fff9c8e1814304  /tmp/spectcl-review/runtime-repro/pkg-result-tcl9.1.0
```


## Run log and binary identities

The tables group repeated read-only inspection commands and parameterised corpus loops; exact commands for every reported behavioural failure are reproduced in the findings or runtime appendix. Setup failures and corrected invocation mistakes are included as review-machinery results, rather than branch failures. No green gate is inferred from a source inspection. Commands use separate worktree targets with `CARGO_INCREMENTAL=0`; shared Cargo-home download caches do not share target directories.


### Root run log

All source inspections were read-only. Scratch files were written under `/tmp/spectcl-review`; build prerequisites and detached worktrees were written under `/workspace`.

| Command or operation | Result |
|---|---|
| Download the uploaded review brief | Initial attempt before workspace readiness failed; retry produced the 7,314-byte brief |
| Read cloud runtime skill and networking reference; inspect environment status and `/etc/codex/network-policy.json` | Managed environment ready; unrestricted HTTP through inherited proxy, no VPN |
| `pwd; rg --files -g AGENTS.md ...; git status --short --branch; git branch -a` in `/workspace` | Found `tcl-lsp/AGENTS.md`; Git commands correctly rejected the parent directory (128) |
| Read brief and `AGENTS.md`; status/branches/logs in `/workspace/tcl-lsp` | Clean `work` at supplied base; target branch initially absent |
| Inspect `CONTRIBUTING.md`, manifests, setup scripts, build-isolation helper, toolchain availability and disk | Toolchain recovery required; per-worktree targets required |
| `git fetch origin claude/spectcl-optimization-discussion-5qhf42` in default restricted command sandbox | Failed to reach proxy; no repository mutation |
| Same fetch using supported additional network permission, preserving proxy | Pass; fetched target `f82eaefbdc5bce8b817f62bf67b900300fb096c8` |
| GitHub read-only issue search and branch/ref/compare GETs | Read 100 recent issue records; verified target ref and 398 commits from supplied base; one percent-encoded branch URL was rejected by URL validation and replaced with the supported Git-ref endpoint |
| `git cat-file -t b06543f3f; git fsck --no-reflogs --unreachable` | Earlier review hash absent; fsck clean (no unreachable target found) |
| `git switch -c claude/spectcl-optimization-discussion-5qhf42 f82eaef...` | Pass |
| `git log --stat 6d214350b..HEAD`; `git diff --numstat`; `git diff --name-status`; `rg --files -g AGENTS.md` | Change map saved and reviewed; root guide only |
| `git worktree add --detach /workspace/tcl-lsp-review-base 6d214350b...` | Pass, separate base target |
| `git fetch origin rust` (supported network permission); `git merge-base origin/rust HEAD` | Origin refreshed; merge base exactly `6d214350b9f899a1399fa841be7f6fc5edbf225a` |
| `git merge-tree --write-tree HEAD origin/rust`; `git rev-list --count 6d214350b..HEAD` | Pass, merge tree `2b5b79936b8c766089373e24449aa6d7a90325dd`; 398 commits |
| `source /tmp/spectcl-review/build-env.sh; cargo build -p tcl-cli` in tip and base worktrees | Both pass; dev builds 1m43s/1m35s; incremental disabled, targets separate |
| `sha256sum` tip/base dev CLIs; copy binaries to stable scratch paths | Pass; hashes recorded in binary table |
| Inspect registry regex/destructuring/cell/list-update declarations and declared implementation driver; read design outlines | Exact functions and target/release rules reviewed |
| Run fixed root differential corpus using dev CLIs | Initial invalid EDA profile aliases corrected to canonical names before final matrix; partial slow dev run stopped, retained as diagnostic evidence only |
| Run fixed compiler differential corpus using dev CLIs | Partial run retained; timing limits under concurrent debug analysis were not treated as defects; rerun with fast binaries planned |
| Known resolved-issue fixtures against both dev CLIs and all five Tcl wrappers | #2388, #2389 argument effects, #2392, #2393 and #2418 corrected at tip; detailed outputs retained |
| `git worktree add --detach /workspace/tcl-lsp-review-fast f82eaef...`; `cargo build -p tcl-cli --profile ci` in fast tip and base | Fast builds for the final wide differential matrix; final outcomes recorded below |
| `rg -n -e scan_defined_and_unset -e existence_constant_branches rust/tcl-compiler/src` in tip/base | Tip exit 1 with no matches; base exit 0 with both definitions (`sccp.rs:1024,1188`), confirming B5 independently without table-escaped alternation |
| `rg -n -e intrinsic -e codegen_abi -e codegen-abi -e Intrinsic rust/xtask/src` | Exit 1 with no matches, independently confirming the B5 generator search with a table-safe command |
| `git diff --no-index --check /dev/null /tmp/spectcl-review/report-draft.md` | Initial check exited 3 and found two whitespace-only context lines in embedded mutation diffs; assembly trims those blank lines. The corrected comparison exits 1 because the report differs from `/dev/null`, with no whitespace diagnostics; staged Git whitespace validation follows before the report commit |

Exploratory `cat`, `head`, `tail`, `sed`, `rg`, `nl`, `git show`, `git status`, `git branch`, `git rev-parse`, `git worktree list`, `git count-objects`, `df`, `du`, tool-version and process-status reads located and verified the files cited in findings. Some guessed scratch/source paths did not yet exist; these were corrected using `rg --files` and were not attributed to the branch. Paused/resumed scratch-build processes and terminated partial corpus drivers affected only review machinery; no source findings depend on interrupted runs.

### Final differential corpus results

- `cargo build -p tcl-cli --profile ci` succeeded in separate exact-tip and base worktrees (12m51s and 12m20s). The initially requested dev builds also succeeded; the faster CI-profile binaries were used for the final wide corpus.
- `SPECTCL_REVIEW_TIP_BINARY=/workspace/tcl-lsp-review-fast/target/ci/tcl SPECTCL_REVIEW_BASE_BINARY=/workspace/tcl-lsp-review-base/target/ci/tcl python3 /tmp/spectcl-review/root-corpus.py`: exit 0; 44 fixed inputs × 14 profiles = 616 rows, each with tip and base optimisation plus original/rewritten execution under all five Tcl releases. `--profile full` is explicit. The root runner compares exit status and stdout, retaining stderr for inspection. No new mismatch on a named profile's intended release, and no mismatch under any of the five releases for release-less `tcl`. An 8.5/8.6-targeted rewrite using `lassign` fails under the unrelated 8.4 interpreter because `lassign` does not exist there; this is target availability, not a regression under the selected release.
- `SPECTCL_REVIEW_TIP_BINARY=/workspace/tcl-lsp-review-fast/target/ci/tcl SPECTCL_REVIEW_BASE_BINARY=/workspace/tcl-lsp-review-base/target/ci/tcl python3 /tmp/spectcl-review/run-compiler-corpus.py`: exit 0; 48 fixed inputs × five named Tcl profiles = 240 rows. Tip matches 220/240 and base 210/240. This corpus runs each named target against its corresponding interpreter, using exit status, stdout, presence of stderr and timeout as its comparison; it does not assert full stderr equality or a five-shell cross-product for every row. Every retained tip failure is also a base failure or the known class described above. The two corrected nested-read inputs explain ten improved rows. This runner omitted `--profile`; the CLI defaults to Full in `rust/tcl-cli/src/commands/transform.rs:117,187` and no scratch project/global config supplied a different profile.
- The root corpus also runs `diag --json --dialect tcl8.6` on tip/base for its 44 inputs; the compiler corpus does so for its 48 inputs. Claimed facts were compared with the reference executions. No additional branch-new false diagnostic was confirmed from these two sets.
- `python3 /tmp/spectcl-review/verify-registry.py`: exit 0; ten tip/base optimisation commands and 75 interpreter runs, plus route/diagnostic inspection. The five pack cases and their exact outputs are reproduced above. These findings used the separately saved dev binaries and the Full default. `python3 /tmp/spectcl-review/verify-registry-full.py` independently repeats all five cases using the saved CI-profile binaries with explicit `--profile full`; all ten CLI outputs and 75 interpreter status/stream pairs exactly match the dev-run evidence (exit 0).
- `python3 /tmp/spectcl-review/run-contract-mutations.py`: exit 0; baseline, two mutations, restoration and final clean-diff results described in S4. Further guard probes use the separate contract-review ledger.
- Full-case and per-command records were retained under `/tmp/spectcl-review`; only this report is handed off through Git. Corpus limits and input families are stated here rather than implying exhaustive soundness.

Profile names in the root matrix: `tcl8.4`, `tcl8.5`, `tcl8.6`, `tcl9.0`, `tcl9.1`, `tcl`, `f5-irules`, `f5-iapps`, `synopsys-eda-tcl`, `cadence-eda-tcl`, `xilinx-eda-tcl`, `intel-quartus-eda-tcl`, `mentor-eda-tcl`, `microchip-libero-eda-tcl`. Inputs cover binding/order, expansion/nested words, caller/namespace state, numerical grammar and tower, double spelling, indices, byte/binary data, regex writes/preservation, caught completion/prefix writes, try/finally and bounded loops. Pack fixtures separately exercise authored semantics, outcome publications, direct declarations, explicit and derived implementations.

| CLI | SHA-256 |
|---|---|
| Tip dev, `/tmp/spectcl-review/binaries/tip-dev` | `a62a23792c4220dfe5434bd98965fd61b5e3f65da625417af3377bc31796c656` |
| Base dev, `/tmp/spectcl-review/binaries/base-dev` | `802527e78e79378e42c517b9f9fd331605241a5f2926e9e7aed45019b851d557` |
| Tip CI profile, executed at `/workspace/tcl-lsp-review-fast/target/ci/tcl`, retained at `/tmp/spectcl-review/binaries/tip-ci` | `68b4bd78512d252df6f054b4c702a546344ce9eef0fa0bd51be5d6162da65096` |
| Base CI profile, executed at `/workspace/tcl-lsp-review-base/target/ci/tcl`, retained at `/tmp/spectcl-review/binaries/base-ci` | `9ac70499354a00fc0ddf3bb4508f16f3de0b895c6888d901c34874bb4837093f` |

Completed fast build targets were removed after the binaries were copied and the above SHA-256 values rechecked. Historical commands retain their actual executed paths; use the retained paths to rerun.

Each wrapper uses its own `TCL_LIBRARY`: Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0. The machine's system Tcl 8.6.16 was used only to identify the initial prerequisite gap, not as a review oracle.

### CI and hygiene results

#### Final CI, mergeability and hygiene evidence

Reviewed tip: `f82eaefbdc5bce8b817f62bf67b900300fb096c8`.
Merge base / refreshed `origin/rust`: `6d214350b9f899a1399fa841be7f6fc5edbf225a`.

The scheduled core Linux CI queue is complete. Every executed core Rust, Python, dependency, runtime and WASM gate passed after the documented host recovery. The reproduced branch CI failure is the required Studio Prettier gate below. Native Electron and real macOS coverage were not executed; the exact omissions are stated below. No source fixes, formatting writes, codegen writes, commits, pushes, issue writes or PR writes were made by this subreview. Final main tracked status is empty and HEAD remains the reviewed tip.

#### B — Studio fails the required front-end CI gate

Added `optionEffectEditor` code is not formatted with pinned Prettier 3.9.9, so the required `web-frontends` job fails before its Studio production build and unit tests.

File/function: `rust/tcl-spec-studio/web/src/editors.ts`, `makeEditors` / nested `optionEffectEditor`, line 685 and lines 692–694 at the reviewed tip. The formatter emits two wrapping changes. This is branch-introduced: the real merge-base file passes the same pinned formatter under its committed configuration; Studio `.prettierrc.json` and `package-lock.json` are unchanged between base and tip. No source fix was made. Collected known issue titles contained no matching formatting/Studio finding.

Exact reproduction, using Node v24.19.0 and package-manager-pinned npm 12.2.0:

```sh
cd rust/tcl-spec-studio/web
npm ci
cd ../../..
mkdir -p build/stamps
touch build/stamps/spec-studio-npm-install
make typecheck-spec-studio-ts lint-spec-studio-ts spec-studio-assets spec-studio-test
```

The strict install, TypeScript and ESLint pass; Make exits 2 at Prettier:

```text
==> Type-checking spec studio front-end with tsc
npm notice run tsc --noEmit -p tsconfig.json && tsc --noEmit -p tsconfig.test.json
==> Linting spec studio front-end (ESLint + Prettier check)
npm notice run eslint src test --ext .ts && prettier --check "src/**/*.ts" "test/**/*.ts"
Checking formatting...
[warn] src/editors.ts
[warn] Code style issues found in the above file. Run Prettier with --write to fix.
make: *** [Makefile:776: lint-spec-studio-ts] Error 1
```

Actual base-file command:

```sh
cd /workspace/tcl-lsp-review-base
node /workspace/tcl-node-ci/rust/tcl-spec-studio/web/node_modules/prettier/bin/prettier.cjs --check rust/tcl-spec-studio/web/src/editors.ts
```

Base result: exit 0, `All matched files use Prettier code style!`. Separately, the base file extracted with `git show 6d214350b:rust/tcl-spec-studio/web/src/editors.ts` also passes the tip's identical explicit config. Formatting the tip into a temporary file produced only the two added wrapping differences. Interpreter/CLI comparisons do not apply to a formatter failure. Full tip/base commands, excerpts and source attribution: `node-ci.md`, `node-logs/studio-gates.log`, `node-logs/base-real-editors-prettier.log`, and `node-logs/tip-editors-format-diff.log` under `/tmp/spectcl-review`.

#### Completed gate results

Every row marked PASS below completed with exit 0. Counts are per required gate, not a count of unique tests across differing feature/toolchain lanes.

| Exact gate / operation | Completed result | Evidence log |
| --- | --- | --- |
| `make rust-check` | PASS: workspace fmt/pedantic Clippy, standalone runtime fmt/Clippy, x509 Clippy + 4 tests, test-support tests, generated/doc/KCS/owner/backing/axis/callback checks, shell workflow contracts, C-ABI/header gates | `rust-check.log` |
| `bash scripts/dev/rust-test-binary-shard.sh run 1/5 --no-fail-fast` | 6,757 passed, 2 ignored/skipped; 157.164s execution | `nextest-shard-1.log` |
| Same exact shard command, `2/5` | 5,152 passed, 4 ignored/skipped; 1,027.349s execution; both real VM-on-WASM coroutine builds/runs passed | `nextest-shard-2.log` |
| Same exact shard command, `3/5`, clean retry | 3,172 passed, 1 ignored/skipped; 211.181s execution | `nextest-shard-3.log` |
| Same exact shard command, `4/5` | 2,662 passed, 12 ignored/skipped; 368.347s execution | `nextest-shard-4.log` |
| Same exact shard command, `5/5` | 3,977 passed, 1 ignored/skipped; 419.233s execution | `nextest-shard-5.log` |
| Five exact shard JSON listings and `verify-nextest-binary-shards.py --partition-count 5` | PASS: all 21,720 non-ignored cases selected exactly once; shard counts 6,757 / 5,152 / 3,172 / 2,662 / 3,977; 20 standard manual/ignored cases | `shard-coverage.log`, `rust-tests-*-5.json` |
| `make test-spectcl-compat` | 42 passed against exact Tcl 9.0.4, fail-closed `TCL_REQUIRE_SPECTCL_COMPAT=1`; 95.56s total | `spectcl-compat.log` |
| `make lint-py` | PASS: Ruff format check and lint on all 37 tracked Python files | `lint-py.log` |
| `make typecheck-py` | PASS: native binding/venv build, ty checks, Pyright 0 errors / 0 warnings / 0 informations; 177.25s total | `typecheck-py.log` |
| `make test-py-engine` | 13 pytest tests passed; native version/hash/describe provenance assertions passed | `test-py-engine.log` |
| `cargo test --workspace --all-features --doc --no-fail-fast` | 53 passed, 2 ignored, 47 doc-test crates; 81.26s total | `doctests.log` |
| `cargo nextest archive -p tcl-lsp-server --all-features --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst` | PASS: 7 binaries including actual non-test server; 44 archive files | `lsp-e2e-archive.log` |
| `cargo nextest run --archive-file … --workspace-remap /workspace/tcl-lsp --no-fail-fast --partition hash:1/3` | 800 passed; 175.021s execution | `lsp-e2e-1.log` |
| Same archived execution, `hash:2/3` | 753 passed; 154.281s execution | `lsp-e2e-2.log` |
| Same archived execution, `hash:3/3` | 741 passed; 128.911s execution | `lsp-e2e-3.log` |
| Archive all/partition JSON listings and `verify-nextest-partitions.py --partition-count 3` | PASS: all 2,294 non-ignored LSP tests selected exactly once; 5 manual/ignored cases | `lsp-partition-coverage.log`, `lsp-all.json`, `lsp-*-3.json` |
| `cargo nextest run --no-fail-fast -p tcl-irule-test -p f5-cli -p tcl-fuzz --all-features` | 312 passed, no skips; 51.321s execution / 107.38s total | `nextest-heavy.log` |
| `make runtime-rust-test-no-tommath` | 789 passed, no failures/skips; 36.07s total | `runtime-no-tommath.log` |
| `make runtime-rust-test` | 942 passed with pinned libtommath and `engine` feature, no failures/skips; 24.20s total | `runtime-tommath.log` |
| `TCL_REQUIRE_WASM_LINK=1 make check-c-extension-wasm` | PASS: 9 checker self-tests, 33 WASM / 36 native exported header functions, real wasm32 C compiles | `c-extension-wasm.log` |
| `TCL_REQUIRE_WASM_LINK=1 cargo test -p tcl-compiler --test wasm_real_link --test wasm_tiers -- --test-threads=1` | 22 passed (15 real-link + 7 tier tests); 103.18s total | `wasm-real-link.log` |
| `TCL_REQUIRE_WASM_LINK=1 cargo test -p tcl-engine-wasm` | 33 passed (2 unit + 31 real-WASM integration tests), no failures/skips; 183.03s total | `engine-wasm.log` |
| `cargo check -p tcl-lsp-server --lib --target wasm32-wasip1` | PASS; 39.00s total | `lsp-wasip1-check.log` |
| `make lsp-server-wasm-test` | PASS: actual release build, wasm-bindgen, growable externref validation, all 31 scripted Node LSP checks; 985.10s total | `lsp-browser-wasm.log` |
| `CARGO_PROFILE_RELEASE_LTO=thin make lsp-server-wasi-test`, exact recovered retry | PASS: real release/wasm-opt module, 22 Rust unit tests + all 26 scripted transport/cache/idle checks | `lsp-wasi-retry.log` |
| `bash scripts/dev/cargo-deny-all.sh`, pinned cargo-deny 0.20.2 | PASS: advisories/bans/licences/sources across all 12 committed lockfile roots, 0 error diagnostics; 27.09s | `cargo-deny-ci-exact.log`, `cargo-deny-ci.md` |
| `make typecheck-report-ts lint-report-ts check-report-assets` | PASS: report frontend typecheck, ESLint/Prettier and committed asset drift | `node-logs/report-gates.log` |
| `make spec-studio-assets spec-studio-test`, independently after failed Studio lint | Production assets built; 121 tests / 36 suites passed, no failures/skips; this does not make the required combined Studio gate pass | `node-logs/studio-build-units.log` |
| `npm run lint` in `editors/vscode` | PASS: extension ESLint and Prettier | `node-logs/extension-lint.log` |
| Required browser assets staged/validated, then exact `npm run test:web` | PASS: all 11 real VS Code web-host assertions, including diagnostics E001, semantic tokens, server restart and a new client | `node-logs/extension-web-smoke-proxy.log`, `node-ci.md` |

Focused registry evidence: `tcl-registry::differential_fold` passed 27 tests and `tcl-registry::value_transfers` passed 80 in shard 4. The complete SpecTcl crate passed 477 tests across the five native shards, with one additional manual/ignored case. The explicit fail-closed compatibility lane is separate from that standard suite and from the hermetic reference-shell harness.

LSP archive SHA-256: `c407b88482fa9cf22556c6e7bc0c8feec36d6169e2e104982e8d5686df74eed7`, 270,500,317 bytes; exact tip and nextest version are recorded in `lsp-archive-sha256.json`. The dependency audit used public RustSec database revision `7eebec69c352c7191b1f13eb95dd510eeca5d1de`; `cargo-deny-ci.md` includes official install digests and all 12 per-root success results.

#### Mergeability and hygiene

- Parent refreshed `origin/rust`, verified the merge base above, and `git merge-tree` succeeded (`merge-tree.txt`). The final main `git merge-base origin/rust HEAD` still reports that base.
- All 12 committed lockfile roots passed full `cargo metadata --manifest-path <manifest> --locked --format-version 1`; `lockfile-check.log` and `metadata-<lockfile-path>.json` record results. The independent exact cargo-deny script also reached all 12 roots.
- `git diff --check origin/rust...HEAD` passed (`diff-check.log`). Final `git status --porcelain --untracked-files=no` is empty; HEAD remains the reviewed tip.
- Added owned source/header scan checked 117 files, including tests, excluding generated/vendor/fixture material. No missing `SPDX-License-Identifier: AGPL-3.0-or-later` headers were found.
- No new Rust `#[allow]`, crate-level `#![allow]`, or `#[expect]` attributes were found (`new-lint-suppressions.txt`, empty).
- No added model identifiers were found. Four broader tool-brand candidates are existing Claude skill paths/integration labels, not worker/model provenance (`model-id-candidates.txt`, `tool-name-candidates.txt`).
- The 21 added Markdown and 25 owned-source-comment US-spelling candidates name actual APIs (`analyze`, `optimize`, `initialize`, `normalize`, `uri::canonicalize`), rather than prose errors (`uk-spelling-candidates.txt`, `source-uk-spelling-candidates.txt`).
- History: 398 commits above base, 333 `wip(...)` subjects, accepted as instructed; no history change made.
- Known standing state still observed: `cargo xtask dialect-drift` exited 1 with exactly 8 sites (`known-dialect-drift.log`). This is excluded from new findings. Oracle suites used `LANG=C.UTF-8`; POSIX-locale #2271 was not re-reported.

#### CI settings, host recovery and preserved provenance

Commands used the owned per-worktree target, `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, and `LANG=C.UTF-8`. Exact Tcl overrides resolve 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 with each interpreter's own library path. Required WASM lanes explicitly set `TCL_REQUIRE_WASM_LINK=1`, so missing tools could not silently make those gates green.

Tool versions: Rust/Cargo 1.99.0; Python 3.12.14; Node v24.19.0; uv 0.12.23; cargo-nextest 0.9.146; wasm-bindgen 0.2.129; Wasmtime 49.0.2; Binaryen 133; wasi-sdk 34.0 / clang 23.1.0-wasi-sdk; npm 12.2.0 via Corepack 0.34.6; Ruff 0.16.10; Prettier 3.9.9; cargo-deny 0.20.2. `ci-tool-versions.txt`, `environment.md`, tool install logs and `cargo-deny-ci.md` record versions, official digests and recovery commands. Tools/caches were placed in writable owned `/workspace` directories, with supported additional network permission and the inherited mandatory proxy/CA; no proxy bypass or credential values were used/logged.

- WASI functional PR tests used `CARGO_PROFILE_RELEASE_LTO=thin`, matching the workflow's functional PR setting at `.github/workflows/ci.yml:2077`. The separate fat-LTO tag/shipping build was not executed. Browser standalone release used its committed profile.
- First WASI attempt: exit 2 after successful compilation, before any unit-test execution, because Wasmtime tried to create `/home/agent/.cache/wasmtime` on the readonly home filesystem. Exact excerpt: `Error: failed to create cache directory: /home/agent/.cache/wasmtime`. An owned wrapper set `XDG_CACHE_HOME=/workspace/.review-cache` and delegated to the unchanged official binary; the exact Make gate then passed all 22 unit and 26 scripted checks. This is a recovered host condition, not a branch defect (`lsp-wasi.log`, `lsp-wasi-retry.log`).
- First VS Code web download failed before test execution with `AggregateError [ECONNREFUSED]`: the pinned downloader reads `npm_config_proxy`/`npm_config_https_proxy`, not the inherited uppercase HTTP proxy variables. Mapping the inherited mandatory values into those recognised settings, without printing them, recovered the exact command. Installed Chromium 153.0.8010.12 / revision 1243 and VS Code insider `ed380adfe6e65ec56c354a49622b844af3938878` ran all 11 assertions. The CI browser was installed without `--with-deps` because host libraries were already provisioned and manual system package writes were out of scope; actual browser launch/smoke passed. An initial Chromium launch permission failure also passed with the supported additional network permission. Details: `node-ci.md`.
- First shard 3 build: exit 101 in 26.03s when native links exhausted the workspace. No test executed in that attempt. Excerpts include `couldn't create a temp dir: No space left on device (os error 28)` and `collect2: fatal error: ld terminated with signal 7 [Bus error]` (`nextest-shard-3-enospc.log`). The runner stopped at its free-space guard. Parent-authorised cleanup removed only completed integration executables proven absent from every later global test-name selection and SpecTcl/WASM/native follow-up; pre-delete paths/SHA/inode/mtime/size are in `completed-test-cleanup.jsonl`, with separate proof JSONs. The independent selection/ledger audit passed (`storage-plan.md`). A narrow supported auto-reviewed escalation was needed only because bwrap could not create its mount stub on the full filesystem; it succeeded. No source was touched.
- Following AGENTS/KCS's ENOSPC recovery rule, full `cargo clean` of the owned main target removed 25.4 GiB logical artefacts before the exact failed shard was rebuilt from clean dependencies. Clean shard 3 passed all 3,172 tests. Green shards 1/2 were not repeated. Subsequent future-disjoint cleanup ran synchronously after each successful listing and before the next build; there was no further ENOSPC. Original failed and recovered logs/results remain in the ledger (`enospc-clean-rebuild-recovery.log`, `resumed-main-ci-runner.log`).
- Completed fast CLI binaries were copied byte-for-byte before parent-authorised cleanup of completed fast/base target directories; SHA-256 matches are in `moved-binary-sha256.json`. Tip copy `/tmp/spectcl-review/binaries/tip-ci`: `68b4bd78512d252df6f054b4c702a546344ce9eef0fa0bd51be5d6162da65096`; base copy `/tmp/spectcl-review/binaries/base-ci`: `9ac70499354a00fc0ddf3bb4508f16f3de0b895c6888d901c34874bb4837093f`. Completed standalone browser/WASI and base/debug targets were also removed only after their consumers/probes finished; browser dist, source trees, saved CLIs, archive and logs remain. `storage-cleanup.log` and the mapping ledger record moves/cleanup. Parent holds the original dev-binary provenance separately.

#### Unexecuted coverage

- Real `macos-wasm-check`: this is a Linux host, so Apple clang/Homebrew compiler-selection behaviour was not run. Hermetic compiler-selection/workflow shell contracts passed through `make rust-check`; actual Linux WASM builds/links passed as listed above.
- Native Electron VS Code `test-ext`: neither Xvfb nor `xvfb-run` is installed and DISPLAY is unset. Three single-root `make test-ext-partition` producers, `make test-ext-multi-folder`, their exact-once result-metadata aggregation, and the separate CI-profile native server build were not executed. xauth plus 32 principal Electron/X11 libraries are present, but a downloaded Electron executable's complete linkage was not verified. The supported `TCL_LSP_SERVER_BIN` override could use an exact-tip debug server, but that would deviate from the workflow's `cargo build -p tcl-lsp-server --profile ci` artefact. Producer prerequisites can also invoke Cargo xtask for missing/stale prompts. No native launch or hidden build was started. Read-only proof: `native-extension-feasibility.md`, `native-extension-host-evidence.json`. Passing browser extension-host coverage is reported separately.
- Other operating-system native host/release packaging lanes and fat-LTO tag shipping builds were not run in this Linux PR review.

#### Exact run log

All evidence paths above are under `/tmp/spectcl-review`. `ci-commands.tsv` contains exact core commands, including full archive/remap/partition arguments and retry/cleanup entries; `ci-status.tsv` preserves every completed exit and elapsed time, including original host failures and clean retry. `main-ci-runner.log` preserves the first runner and ENOSPC stop; `resumed-main-ci-runner.log` ends with `MAIN CI COMPLETE`. `run-main-ci.py`, `resume-main-ci.py` and `run-standalone-ci.py` record execution order/environment-sensitive flags. `node-ci.md` / `node-status.tsv`, `cargo-deny-ci.md`, `environment-command-log.md` and install logs contain the independent/frontend/prerequisite command results. Final tracked-tree and tip/base checks are in `ci-final-state.log`. There is no outstanding queued core command or unresolved host failure in this subreview.

### Environment setup command ledger


### Environment investigation command log

Working directory: `/workspace/tcl-lsp`. Read-only source inspection included `AGENTS.md`, `rust-toolchain.toml`, the development-environment contract, the session-start hook, Tcl reference adapter/test/fetch scripts, the dependency installer, and relevant Makefile targets.

| Command | Result |
|---|---|
| `command -v cargo rustc rustup tclsh cc make curl python3` | No Rust executables; Tcl, compiler, Make, curl and Python found. |
| `ls -d /tmp/*tcl* /root/.cargo/bin /usr/local/cargo/bin /opt/rust*` | No reference trees/toolchain paths found. |
| `rg --files /opt /usr/local /tmp /workspace` filtered for Cargo/Rustup/Rustc and Tcl build-tree paths | No candidate toolchain or exact Tcl build paths. |
| `getent hosts proxy` | `172.31.0.77 proxy`. |
| `curl --head --max-time 10 https://static.rust-lang.org/rustup/release-stable.toml` in default network sandbox | Exit 7; connection to proxy port 8080 refused. |
| Same official Rust HEAD with 15s timeout and supported additional network permission | Exit 0; HTTP 200 via the configured proxy. |
| `printf 'puts [info patchlevel]\\n' \| env -u TCL_LIBRARY /usr/bin/tclsh` | Tcl 8.6.16. |
| Scoped `ensure-test-deps.sh --check` below | Exit 1; seven prerequisite groups missing; log saved. |
| `bash scripts/dev/test-reference-tcl-toolchains.sh` | Exit 0; `reference Tcl toolchain shell regression: ok`; log saved. |
| Focused Rust installer below, with supported additional network permission | Exit 0; Rust/Cargo stable 1.99.0, rustfmt/Clippy, two WASM targets installed. |
| Focused Tcl installer below, with supported additional network permission | Exit 0; all five pinned reference interpreters installed. |
| Scoped dependency check after sourcing `build-env.sh` | Exit 0; all scoped dependencies satisfied; `/tmp/spectcl-review/deps-check-after.log`. |
| `rustc --version`, `cargo --version`, component/target probes, and Tcl resolver/patchlevel probes | Installed versions/targets confirmed; all five Tcl reference patchlevels resolved exactly. |

The scoped dependency check was:

```sh
env SKIP_PYTHON_TK=1 SKIP_NODE=1 SKIP_KOTLINC=1 \
  SKIP_WASMTIME=1 SKIP_BINARYEN=1 SKIP_WASI_SDK=1 \
  SKIP_EMACS=1 SKIP_XVFB=1 SKIP_TSHARK=1 SKIP_OPENSSL=1 \
  SKIP_PING=1 SKIP_RGXG=1 SKIP_TCLLIB=1 SKIP_UV=1 \
  TCL_LSP_TCL_SOURCE_PARENT=/workspace/tcl-lsp/tmp \
  TCL_LSP_TCL_BIN_DIR=/tmp/spectcl-review/tcl-reference-bin \
  bash scripts/dev/ensure-test-deps.sh --check \
  > /tmp/spectcl-review/deps-check.log 2>&1
```

The hermetic regression was:

```sh
bash scripts/dev/test-reference-tcl-toolchains.sh \
  > /tmp/spectcl-review/reference-shell-regression.log 2>&1
```

The focused installers were:

```sh
env CARGO_HOME=/workspace/.review-cargo \
  RUSTUP_HOME=/workspace/.review-rustup \
  PATH=/workspace/.review-cargo/bin:$PATH \
  make ensure-rust-deps > /tmp/spectcl-review/rust-setup.log 2>&1

env TCL_LSP_TCL_SOURCE_PARENT=/workspace/tcl-lsp/tmp \
  TCL_LSP_TCL_BIN_DIR=/tmp/spectcl-review/tcl-reference-bin \
  make ensure-tcl-deps > /tmp/spectcl-review/tcl-setup.log 2>&1
```

Both used `sandbox_permissions: "with_additional_permissions"` and `additional_permissions: { network: { enabled: true } }`, preserving the configured HTTP proxy and TLS trust.

No tracked-file modification, Cargo build, commit, push, proxy bypass, or credential inspection/output was performed.

### Node job command ledger

All source-affecting builds/checks were performed in the isolated detached worktree `/workspace/tcl-node-ci` at the exact reviewed tip. No Cargo build was started by this subreview. No commit, push, issue or PR write was made. The main review worktree was not edited. `git status --short` in this Node worktree remains empty after report rebuild and Studio build/unit checks.

Environment variables used for Node commands:

```sh
PATH=/workspace/.review-tools/node-bin:$PATH
COREPACK_HOME=/workspace/.review-tools/corepack
COREPACK_ENABLE_DOWNLOAD_PROMPT=0
npm_config_cache=/workspace/.review-tools/npm-cache
PLAYWRIGHT_BROWSERS_PATH=/workspace/.review-tools/playwright
```

The owned shim directory was provisioned by `corepack enable npm --install-directory /workspace/.review-tools/node-bin`; no system package manager or HOME change was used. Executable `/workspace/.review-tools/npm` and `npx` wrappers were then added for the parent runner; each exports the owned Corepack home and download-prompt setting before executing the corresponding shim. Both were verified to report `12.2.0` using the parent's `.review-tools`-first PATH in the main report frontend. All networked commands used the supported additional network permission and inherited proxy/CA settings. Managed environment status reported connected, current observations and enforced unrestricted HTTP policy; `/etc/codex/network-policy.json` reports no VPN or TCP grant.

| Command | Working directory | Result / log |
| --- | --- | --- |
| `git worktree add --detach /workspace/tcl-node-ci f82eaefbdc5bce8b817f62bf67b900300fb096c8` | `/workspace/tcl-lsp` | Pass; detached exact-tip checkout |
| `corepack enable npm --install-directory /workspace/.review-tools/node-bin` | repository | Pass |
| `node --version; /workspace/.review-tools/node-bin/npm --version; corepack --version` | report frontend | `v24.19.0`, `12.2.0`, `0.34.6`; `node-logs/versions.log` |
| `npm ci` | `rust/bigip-report-gen/frontend` | Pass, 102 packages; `node-logs/report-npm-ci.log` |
| `npm ci` | `rust/tcl-spec-studio/web` | Pass, 113 packages; `node-logs/studio-npm-ci.log` |
| `npm ci` | `editors/vscode` | Pass; `node-logs/extension-npm-ci.log` |
| `mkdir -p build/stamps` and `touch build/stamps/report-npm-install build/stamps/spec-studio-npm-install` | tip worktree | Pass; same stamps as CI, avoiding `npm install` replacement of strict CI installs |
| `make typecheck-report-ts lint-report-ts check-report-assets` | tip worktree | Pass; TS type-check, ESLint/Prettier and committed asset drift; `node-logs/report-gates.log` |
| `make typecheck-spec-studio-ts lint-spec-studio-ts spec-studio-assets spec-studio-test` | tip worktree | Exit 2 at Prettier, finding above; `node-logs/studio-gates.log` |
| `make spec-studio-assets spec-studio-test` | tip worktree | Pass independently after failure; production controller, native editor and Monaco assets built; 121 tests / 36 suites, 0 failures, 0 skips; `node-logs/studio-build-units.log` |
| `npm run lint` | `editors/vscode` | Pass; ESLint and Prettier; `node-logs/extension-lint.log` |
| `npx playwright install chromium` | `editors/vscode` | Pass; exact lockfile Chromium revision 1243, Chrome for Testing and headless shell `153.0.8010.12`, FFmpeg 1011; `node-logs/playwright-install.log` |
| `node scripts/copy-web-assets.cjs --require` | `editors/vscode` | Pass, 3 worker assets and 8 spec packs staged; `node-logs/extension-stage-browser.log` |
| `npm run test:web` | `editors/vscode` | Initial host download failed before test execution (`ECONNREFUSED`, downloader did not read inherited HTTP proxy variables); compile/type-check and all bundles passed; `node-logs/extension-web-smoke.log` |
| `npm run test:web` with npm downloader proxy settings using inherited proxy values | `editors/vscode` | Pass, all 11 browser assertions, 34.0 seconds; `node-logs/extension-web-smoke-proxy.log` |
| `node <tip>/rust/tcl-spec-studio/web/node_modules/prettier/bin/prettier.cjs --check rust/tcl-spec-studio/web/src/editors.ts` | base worktree | Pass under base configuration; `node-logs/base-real-editors-prettier.log` |
| `git show 6d214350b:rust/tcl-spec-studio/web/src/editors.ts` then pinned `prettier --config rust/tcl-spec-studio/web/.prettierrc.json --check /tmp/spectcl-review/node-base-editors.ts` | tip worktree | Pass; `node-logs/base-editors-prettier.log` |
| pinned `prettier rust/tcl-spec-studio/web/src/editors.ts` to a temporary file and `diff -u` | tip worktree | Expected diff exit 1, exactly two added wrapping sites; `node-logs/tip-editors-format-diff.log` |
| `git diff 6d214350b..f82eaefbd -- <Studio config, package-lock>` | tip worktree | Empty / pass, configuration and dependency versions unchanged |
| `make -n copy-canonical` | tip worktree | No Cargo prerequisite; only copy/touch generated prompt assets |
| `git status --short`, `git rev-parse HEAD`, `git worktree list` | tip worktree | Clean; exact hash confirmed |

Inspection commands (`cat`/`sed`/`rg`/`nl`) read `AGENTS.md`, the user brief, exact workflow jobs, all three package manifests, Makefile target definitions, Node build and copy scripts, known titles, logs and tracked diff. They introduced no edits. Raw timings and command strings are in `/tmp/spectcl-review/node-status.tsv`; runner sources/logs are `/tmp/spectcl-review/run-node-ci.py`, `run-node-gates.py`, `run-node-remaining.py` and their corresponding logs.

The browser extension smoke completed successfully. The exact CI install uses `npx playwright install --with-deps chromium`; this subreview installed the same browser without `--with-deps` because system Chromium and libraries are already provisioned and the assigned work excludes ad hoc system package writes. An actual launch of the installed headless shell and the completed smoke confirmed the current host libraries suffice.

The launch probe (`require("playwright").chromium.launch({headless:true})`, then a page with a data URL and title) first exited 1 in the default sandbox: Chromium reported `FATAL:content/browser/sandbox_host_linux.cc:41 ... shutdown: Operation not permitted (1)`. Re-running with supported additional network permission exited 0, outputting `Chromium version: 153.0.8010.12` and `Page title: probe`. Logs: `node-logs/playwright-launch-probe.log` and `node-logs/playwright-launch-probe-network.log`. This was a recovered host permission failure, not a branch defect.

The exact CI VS Code web version endpoint was read with `curl -fsSL --max-time 20 -o /tmp/spectcl-review/vscode-web.json https://update.code.visualstudio.com/api/update/web-standalone/insider/latest`; it succeeded and resolved to `ed380adfe6e65ec56c354a49622b844af3938878`, recorded in `node-logs/vscode-web-version.log`. The completed browser consumer and log are `/tmp/spectcl-review/run-extension-web.py` and `/tmp/spectcl-review/extension-web-runner.log`. It waits for the parent browser build script's successful compiler, wasm-bindgen and module verification completion (`==> done:` followed by its final wasm byte summary), independently re-runs `scripts/verify-wasm-externref.mjs` on that module, copies the ignored artefacts, records their SHA256 values, stages the assets, then runs the exact `npm run test:web` command with the CI ten-minute limit and supported network permission. The parent's standalone Node LSP smoke is a separate result and cannot suppress this independently verified browser extension test.

The parent-owned full browser build and standalone LSP scripted smoke passed (`31/31 checks`, overall exit 0 in 985.1 seconds). This subreview's independent module validation passed: `OK: __wbindgen_externrefs is growable (1024 -> 1028)`. Copied browser artefact SHA256 values are in `node-logs/browser-assets-sha256.log`:

```text
c2945ee7442600531337bc8f2e84d309ccd10655685f79ba23fc899347a7e836  worker.js
d22b56dadad0a330ddb7a2db9a91322820e6b7a2b0b0935653e65d9587632d7b  tcl_lsp_server_wasm.js
d1c33eaa2e468bc7359078532037543cef7429cf52cd09ae6d406111343121df  tcl_lsp_server_wasm_bg.wasm
```

The initial exact `npm run test:web` invocation passed TypeScript compilation, canonical-asset copying, worker staging, both product bundles and test bundling, then failed before browser tests downloading VS Code with `AggregateError [ECONNREFUSED]`. Inspection of the locked `@vscode/test-web/out/server/download.js` showed that its downloader reads `npm_config_proxy` and `npm_config_https_proxy` rather than the inherited mandatory `HTTP_PROXY` / `HTTPS_PROXY`. The retry script `/tmp/spectcl-review/retry-extension-web.py` copies the inherited values to those recognised variables without printing them, preserving the proxy path and TLS trust. The same exact test command then downloaded the resolved VS Code insider commit and exited 0 in 34.0 seconds. This recovered network setup issue is not a branch defect.

Successful smoke excerpt:

```text
tcl-lsp web smoke:
  ok   the extension is installed in the web host
  ok   the browser entry point activates
  ok   the fixture folder is open
  ok   the fixture opens as a Tcl document
  ok   the wasm server answers a request put to it directly
  ok   the wasm language server produces semantic tokens
  ok   the wasm language server publishes diagnostics
  ok   the diagnostics include the fixture's E001
  ok   Tcl: Restart Server completes
  ok   the restart built a new client rather than reusing the exited one
  ok   the server answers again after a restart
```

Final `git status --short` in the detached Node worktree is empty. No outstanding command is blocked in this subreview; the single branch-introduced failure is the Studio formatting gate described above.

### Supplemental explicit grammar, precision and global witnesses

`python3 /tmp/spectcl-review/supplement-corpus.py` exits 0: eleven fixed inputs × fourteen profiles = 154 rows, each original/tip/base text executed under all five exact shells. Every optimiser invocation explicitly uses `--profile full` and the saved CI-profile binaries. The initial 140-row pilot was retained, then membership/string operands were quoted and an explicit `in` witness added before this final run. No branch-new mismatch was found. The retained double mismatches are the known #2395 `tcl_precision` class (including the already-described script override), not additional findings. The explicit global witness is correct at tip and fails at base because O109 deletes its initial global store.

The fixed source inputs are:

**global-increment**

```tcl
set x 1; proc p {} {global x; incr x; return $x}; puts [p]; puts $x
```

**in**

```tcl
puts [catch {expr {"a" in {a b}}} x]; puts $x
```

**int32-wrap**

```tcl
puts [catch {expr {int(4294967295)}} x]; puts $x
```

**ni**

```tcl
puts [catch {expr {"a" ni {a b}}} x]; puts $x
```

**octal-binary-prefix**

```tcl
puts [catch {expr {0o10+0b11}} x]; puts $x
```

**precision-default**

```tcl
puts [expr {1.0/3.0}]
```

**precision-six**

```tcl
set tcl_precision 6; puts [expr {1.0/3.0}]
```

**string-ge**

```tcl
puts [catch {expr {"a" ge "a"}} x]; puts $x
```

**string-gt**

```tcl
puts [catch {expr {"b" gt "a"}} x]; puts $x
```

**string-le**

```tcl
puts [catch {expr {"a" le "a"}} x]; puts $x
```

**string-lt**

```tcl
puts [catch {expr {"a" lt "b"}} x]; puts $x
```

The per-release target/oracle comparisons below use exact JSON-escaped stdout; exit statuses are appended after `/`. The original and each selected-release rewrite were cross-run under all five shells in the full matrix. Off-target grammar availability is distinguished from selected-target parity.

| Input | Target and exact oracle | Original | Tip rewrite | Base rewrite |
|---|---|---|---|---|
| global-increment | tcl8.4 / 8.4.20 | `"2\n2\n"` / 0 | `"2\n2\n"` / 0 | `""` / 1 |
| global-increment | tcl8.5 / 8.5.19 | `"2\n2\n"` / 0 | `"2\n2\n"` / 0 | `"1\n1\n"` / 0 |
| global-increment | tcl8.6 / 8.6.18 | `"2\n2\n"` / 0 | `"2\n2\n"` / 0 | `"1\n1\n"` / 0 |
| global-increment | tcl9.0 / 9.0.4 | `"2\n2\n"` / 0 | `"2\n2\n"` / 0 | `"1\n1\n"` / 0 |
| global-increment | tcl9.1 / 9.1.0 | `"2\n2\n"` / 0 | `"2\n2\n"` / 0 | `"1\n1\n"` / 0 |
| in | tcl8.4 / 8.4.20 | `"1\nsyntax error in expression \"\"a\" in {a b}\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" in {a b}\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" in {a b}\": extra tokens at end of expression\n"` / 0 |
| in | tcl8.5 / 8.5.19 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| in | tcl8.6 / 8.6.18 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| in | tcl9.0 / 9.0.4 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| in | tcl9.1 / 9.1.0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| int32-wrap | tcl8.4 / 8.4.20 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 |
| int32-wrap | tcl8.5 / 8.5.19 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 |
| int32-wrap | tcl8.6 / 8.6.18 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 |
| int32-wrap | tcl9.0 / 9.0.4 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 |
| int32-wrap | tcl9.1 / 9.1.0 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 | `"0\n4294967295\n"` / 0 |
| ni | tcl8.4 / 8.4.20 | `"1\nsyntax error in expression \"\"a\" ni {a b}\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" ni {a b}\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" ni {a b}\": extra tokens at end of expression\n"` / 0 |
| ni | tcl8.5 / 8.5.19 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 |
| ni | tcl8.6 / 8.6.18 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 |
| ni | tcl9.0 / 9.0.4 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 |
| ni | tcl9.1 / 9.1.0 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 | `"0\n0\n"` / 0 |
| octal-binary-prefix | tcl8.4 / 8.4.20 | `"1\nsyntax error in expression \"0o10+0b11\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"0o10+0b11\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"0o10+0b11\": extra tokens at end of expression\n"` / 0 |
| octal-binary-prefix | tcl8.5 / 8.5.19 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 |
| octal-binary-prefix | tcl8.6 / 8.6.18 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 |
| octal-binary-prefix | tcl9.0 / 9.0.4 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 |
| octal-binary-prefix | tcl9.1 / 9.1.0 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 | `"0\n11\n"` / 0 |
| precision-default | tcl8.4 / 8.4.20 | `"0.333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-default | tcl8.5 / 8.5.19 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-default | tcl8.6 / 8.6.18 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-default | tcl9.0 / 9.0.4 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-default | tcl9.1 / 9.1.0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-six | tcl8.4 / 8.4.20 | `"0.333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-six | tcl8.5 / 8.5.19 | `"0.333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-six | tcl8.6 / 8.6.18 | `"0.333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-six | tcl9.0 / 9.0.4 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| precision-six | tcl9.1 / 9.1.0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 | `"0.3333333333333333\n"` / 0 |
| string-ge | tcl8.4 / 8.4.20 | `"1\nsyntax error in expression \"\"a\" ge \"a\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" ge \"a\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" ge \"a\"\": extra tokens at end of expression\n"` / 0 |
| string-ge | tcl8.5 / 8.5.19 | `"1\ninvalid bareword \"ge\"\nin expression \"\"a\" ge \"a\"\";\nshould be \"$ge\" or \"{ge}\" or \"ge(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"ge\"\nin expression \"\"a\" ge \"a\"\";\nshould be \"$ge\" or \"{ge}\" or \"ge(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"ge\"\nin expression \"\"a\" ge \"a\"\";\nshould be \"$ge\" or \"{ge}\" or \"ge(...)\" or ...\n"` / 0 |
| string-ge | tcl8.6 / 8.6.18 | `"1\ninvalid bareword \"ge\"\nin expression \"\"a\" ge \"a\"\";\nshould be \"$ge\" or \"{ge}\" or \"ge(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"ge\"\nin expression \"\"a\" ge \"a\"\";\nshould be \"$ge\" or \"{ge}\" or \"ge(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"ge\"\nin expression \"\"a\" ge \"a\"\";\nshould be \"$ge\" or \"{ge}\" or \"ge(...)\" or ...\n"` / 0 |
| string-ge | tcl9.0 / 9.0.4 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| string-ge | tcl9.1 / 9.1.0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| string-gt | tcl8.4 / 8.4.20 | `"1\nsyntax error in expression \"\"b\" gt \"a\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"b\" gt \"a\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"b\" gt \"a\"\": extra tokens at end of expression\n"` / 0 |
| string-gt | tcl8.5 / 8.5.19 | `"1\ninvalid bareword \"gt\"\nin expression \"\"b\" gt \"a\"\";\nshould be \"$gt\" or \"{gt}\" or \"gt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"gt\"\nin expression \"\"b\" gt \"a\"\";\nshould be \"$gt\" or \"{gt}\" or \"gt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"gt\"\nin expression \"\"b\" gt \"a\"\";\nshould be \"$gt\" or \"{gt}\" or \"gt(...)\" or ...\n"` / 0 |
| string-gt | tcl8.6 / 8.6.18 | `"1\ninvalid bareword \"gt\"\nin expression \"\"b\" gt \"a\"\";\nshould be \"$gt\" or \"{gt}\" or \"gt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"gt\"\nin expression \"\"b\" gt \"a\"\";\nshould be \"$gt\" or \"{gt}\" or \"gt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"gt\"\nin expression \"\"b\" gt \"a\"\";\nshould be \"$gt\" or \"{gt}\" or \"gt(...)\" or ...\n"` / 0 |
| string-gt | tcl9.0 / 9.0.4 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| string-gt | tcl9.1 / 9.1.0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| string-le | tcl8.4 / 8.4.20 | `"1\nsyntax error in expression \"\"a\" le \"a\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" le \"a\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" le \"a\"\": extra tokens at end of expression\n"` / 0 |
| string-le | tcl8.5 / 8.5.19 | `"1\ninvalid bareword \"le\"\nin expression \"\"a\" le \"a\"\";\nshould be \"$le\" or \"{le}\" or \"le(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"le\"\nin expression \"\"a\" le \"a\"\";\nshould be \"$le\" or \"{le}\" or \"le(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"le\"\nin expression \"\"a\" le \"a\"\";\nshould be \"$le\" or \"{le}\" or \"le(...)\" or ...\n"` / 0 |
| string-le | tcl8.6 / 8.6.18 | `"1\ninvalid bareword \"le\"\nin expression \"\"a\" le \"a\"\";\nshould be \"$le\" or \"{le}\" or \"le(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"le\"\nin expression \"\"a\" le \"a\"\";\nshould be \"$le\" or \"{le}\" or \"le(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"le\"\nin expression \"\"a\" le \"a\"\";\nshould be \"$le\" or \"{le}\" or \"le(...)\" or ...\n"` / 0 |
| string-le | tcl9.0 / 9.0.4 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| string-le | tcl9.1 / 9.1.0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| string-lt | tcl8.4 / 8.4.20 | `"1\nsyntax error in expression \"\"a\" lt \"b\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" lt \"b\"\": extra tokens at end of expression\n"` / 0 | `"1\nsyntax error in expression \"\"a\" lt \"b\"\": extra tokens at end of expression\n"` / 0 |
| string-lt | tcl8.5 / 8.5.19 | `"1\ninvalid bareword \"lt\"\nin expression \"\"a\" lt \"b\"\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"lt\"\nin expression \"\"a\" lt \"b\"\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"lt\"\nin expression \"\"a\" lt \"b\"\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...\n"` / 0 |
| string-lt | tcl8.6 / 8.6.18 | `"1\ninvalid bareword \"lt\"\nin expression \"\"a\" lt \"b\"\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"lt\"\nin expression \"\"a\" lt \"b\"\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...\n"` / 0 | `"1\ninvalid bareword \"lt\"\nin expression \"\"a\" lt \"b\"\";\nshould be \"$lt\" or \"{lt}\" or \"lt(...)\" or ...\n"` / 0 |
| string-lt | tcl9.0 / 9.0.4 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |
| string-lt | tcl9.1 / 9.1.0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 | `"0\n1\n"` / 0 |

Exact parameterised command forms (the JSON ledger records each expanded argv and captured stdout/stderr):

```sh
/tmp/spectcl-review/binaries/tip-ci opt /tmp/spectcl-review/supplement-corpus-final/CASE.tcl --profile full --dialect DIALECT
/tmp/spectcl-review/binaries/base-ci opt /tmp/spectcl-review/supplement-corpus-final/CASE.tcl --profile full --dialect DIALECT
/tmp/spectcl-review/tcl-reference-bin/tclshRELEASE /tmp/spectcl-review/supplement-corpus-final/CASE.tcl
/tmp/spectcl-review/tcl-reference-bin/tclshRELEASE /tmp/spectcl-review/supplement-corpus-final/CASE.DIALECT.tip.tcl
/tmp/spectcl-review/tcl-reference-bin/tclshRELEASE /tmp/spectcl-review/supplement-corpus-final/CASE.DIALECT.base.tcl
```

The same runner also invokes tip/base `diag FILE --json --dialect tcl8.6` for all eleven inputs. The mismatch metric is `(exit status, stdout)`, with stderr retained for inspection. All named-target green cases have empty stderr; the base global witness errors on Tcl 8.4 and gives `1
1
` on 8.5+, whereas the original and tip both give `2
2
` on every release.


### Documentary inspection and reproduction command ledgers

Read AGENTS, branch first-parent commit mapping and full `value-transfers.md` (3123lines), `value-evaluation.md` (2224lines), all changed KCS value-transfer/pack/evaluator/compiler-registry/feature pages except policy/ABI ones assigned to `doc_policy`; feature Spec Studio644lines read in two chunks, pack authoring177lines, feature CLI129lines, Docker120/pkg124/MCP97/Explorer97, containerise161. Delegated full examples2105/migration755 plus changed KCS codes to `doc_examples`; registry-consumer2607/diagnostic-policy1132/ABI539 and corresponding policy/registry/extension KCS to `doc_policy`. Their detailed evidence is in `docs-examples.md`/`docs-policy.md`. The shared authored-path existence scan checked all7 named design pages and every changed KCS page; missing config_ini path was a historic moved-from path and is narrative, not a false current path; missing generated studio dist directory was not reported.

Read-only commands in this subreview (shell combinations in exploratory calls occasionally returned exit2 for a guessed missing filename; no mutation of tracked files):

- `pwd && rg --files -g AGENTS.md -g '*spectcl*' /workspace /tmp/spectcl-review`0; `cat AGENTS.md`0.
- `cat /tmp/spectcl-review/git-log-stat.txt`0 (huge output truncated); `git log --oneline --first-parent 6d214350b..HEAD`0 provides readable mapping.
- `wc -l docs/design/compiler/{value-transfers,value-evaluation,value-transfers-examples,value-transfers-migration,registry-consumer-contracts,diagnostic-policy}.md docs/design/runtime/c-extension-abi.md`0; `git diff --name-only 6d214350b...HEAD -- docs/kcs/**`0; changed non-code KCS `xargs wc -l`0.
- Full `sed -n` reads of `value-transfers.md`:1–300,301–680,681–1080,1081–1470,1471–1880,1881–2280,2281–2700,2701–3123. Full `value-evaluation.md`:1–360,360–720,720–1100,1101–1480,1481–1850,1851–2224. All0.
- `cat` exact uploaded brief; all KCS pages described in scope (Spec Studio `sed1–330,331–644`); `cat /tmp/spectcl-review/build-env.sh`; all0.
- `cat /tmp/spectcl-review/known-issues.json`0 (truncated); Python JSON title filter for regexp/regex/documentation/doc/count/surface0; recognised no excluded issue covering these documentary claims.
- `rg` source checks for existence scans/rung, template consumers, Foreach/vendor declaration, direct folders/ConstOps, regex about/precision, Engine/set_release, HostKind/extension parser, dialect loader versions, common policy and various narrative terms. Inspected each matching definition as cited above. Three searches guessed nonexistent filenames (`regexp.rs`, `commands/opt.rs`, `commands/optimise.rs`, `tcl-cli/src/spec.rs`, `tcl-mcp/src/analysis.rs` or `lib.rs`, `tcl-diagnostics/src/policy.rs`); remaining output was inspected and canonical paths found with `rg --files`.
- `git show 6d214350b:<source>` for sccp/string folder/engine API/regex/loader/pack quickstart: all valid paths0. `git show` / `git cat-file -e` for added value-evaluation page, WASM engine, regex/value-transfer KCS pages exited128 proving absence at base. `git log --oneline base..HEAD -- <KCS paths>`0.
- Python backtick authored-code-path existence audit0, only two missing paths as explained above. `nl -ba` targeted quote validation0.
- Python wrote3 probe inputs under `/tmp`; timing-loop oracles0 with outputs above. Tip regex opt0; tip old-index opt0; tip spec-upgrade0 modifies only tmp probe; original/base/tip old-index matrix0. No compiler build, mutations, or generic test suite run by this subagent; root owns binary build and hashes.

Binary SHA-256 (queried after root build): tip `a62a23792c4220dfe5434bd98965fd61b5e3f65da625417af3377bc31796c656`; base `802527e78e79378e42c517b9f9fd331605241a5f2926e9e7aed45019b851d557`. `sha256sum` exited0.

Final extra command: `tcl explore /tmp/spectcl-review/docs-regexp-program.tcl --show sccp --text | tail -25` exited 0, printing the documented approximate decline.

Binary hashes obtained with `sha256sum`:

```text
a62a23792c4220dfe5434bd98965fd61b5e3f65da625417af3377bc31796c656  /workspace/tcl-lsp/target/debug/tcl
802527e78e79378e42c517b9f9fd331605241a5f2926e9e7aed45019b851d557  /workspace/tcl-lsp-review-base/target/debug/tcl
```

Exact CLI/reference commands and captured stdout/stderr/status are recorded in `/tmp/spectcl-review/docs-policy-behavior.log` (JSON). That log includes two intentionally rejected base `--show-suppressed` calls, then the valid base calls without the new flag. Fixtures are `/tmp/spectcl-review/{stub-arity,bidi-noqa,bidi-unsuppressed}.tcl`; imported pack `/tmp/spectcl-review/doc-pkga.tclspec`.

Other verification commands (all read-only, except scratch fixture/report writes):

1. `cat AGENTS.md`; `cat /workspace/attachments/673c7cb6-d479-4393-8592-32d140363047/Pasted\ text.txt`; `git diff --name-only BASE TIP -- docs/kcs ...`; `git log --oneline BASE..TIP -- <three scoped design pages>` — scope/requirements mapped successfully.
2. `nl -ba`, `sed -n` range reads of the three entire design pages and all scoped KCS pages; `rg -n '^#{1,5} |...'` for headings/process prose — confirmed source locations above. Large initial reads were output-truncated, then repeated in smaller ranges. No finding relies on an unseen truncated section.
3. `python` backtick-path existence scan of the three design docs — one missing path, old `rust/tcl-lsp-server/src/config_ini.rs:625`, expressly a historical reference and included only in the history finding.
4. `python` import of `scripts/check_c_extension_wasm.py` — `runtime_exports=35`, `shim_exports=36`, WASM leg declarations33/native36; `find_capi_exports=41`. `rg` of the authored headers for `TclOOStubs`, `TclStubs`, `tclOOStubsPtr`, `pkgooa` — no matching authored types/check. `ls runtime/rust/include` — two headers listed above.
5. Both exact `cc` commands in DP-B2 — intentionally exit1, missing` tclOO.h`. One preliminary base command pointed at the unprovisioned base `tmp/tcl9.0.4/...` and failed for a missing source; rerun used the real canonical tip source with the base include path. No WASM compile attempted: `command -v clang` yielded none, `/opt/wasi-sdk/bin/clang` absent; host`cc`/`gcc` exist.
6. `python` regex enum count —31 current analyser variants; base via `git show BASE:rust/tcl-registry/src/hooks.rs` gives43, `Set` present. Checked the shipped clause-grammar descriptor registrations (10 distinct grammars over11 command/subcommand positions) and intrinsic28-member table; those other stated counts agree.
7. `rg -n -e intrinsic -e codegen_abi -e codegen-abi -e Intrinsic rust/xtask/src` — no hits; `rg` for `CodegenAbiImportId`, `descriptor`, `ALL` in runtime-api source and backing report generation in xtask — locations above.
8. `git show BASE:<paths>` and `git blame -L ... -- <paths>` for each finding — attribution above. `git diff --numstat BASE..TIP -- <three design pages>` yields diagnostic-policy1132 added/0removed; registry-consumer2607/0; ABI182added/40removed.
9. `rg` across diagnostic source, config parser/merge, server watchers, clause/member/option/stamp source and tests — confirmed behaviours and ownership above. Read exact producer suppression and arity-skipping functions. Read the complete config parser before concluding the ignore-list facility is absent.
10. Issue-title/body filtering — #2439 recognised and excluded; no supplied issue describes the new documentary contradictions above. An exploratory Python read of `older-known-issues.json` assumed a list but encountered its object format and raised`AttributeError`; the parent had already supplied the explicit older exclusion numbers, which this review honours.
11. `ps`/scratch-file checks followed the initial buffered Python CLI batch; it completed successfully. One malformed exploratory `sed -n '241?'` was rejected and then replaced by valid exact base-range reads. These command errors are review-log artefacts, not CI failures.

No other confirmed contradiction was found in the scoped dormant-hook, stamp-refusal, option-effect DSL, registry-axes waiver rules, member/clause APIs, configuration policy-layer order, or extension-description source/probe instructions after comparison with the relevant code.

- `/tmp/spectcl-review/docs-repros/`: 11 small programs drawn from claims in the pages, with calls/output appended only to make their behaviour deterministic.
- `/tmp/spectcl-review/docs-repros/results.json`: every exact CLI/oracle command and unabridged stdout/stderr for tip and base (complete: 11 programs × 2 binaries × diag/opt, plus 55 original-program oracle executions).
- Tip CLI SHA-256: `a62a23792c4220dfe5434bd98965fd61b5e3f65da625417af3377bc31796c656`.
- Base CLI SHA-256: `802527e78e79378e42c517b9f9fd331605241a5f2926e9e7aed45019b851d557`.

Read-only command families run: `cat AGENTS.md` and the uploaded brief; `git diff --name-only BASE...HEAD -- docs/kcs/codes`; full numbered reads of both design pages and all changed code notes; full `git diff BASE...HEAD -- docs/kcs/codes`; `rg` for every contradicting identifier against current Rust source; path existence/count script; `git log --oneline BASE..HEAD --` the three affected pages; `git cat-file -e BASE:PATH`; base `git show` of W210's historical paragraph; CLI `diag/opt/spec export --help`; `sha256sum` of both binaries; corpus script running diag and full-profile opt on both binaries and all five reference Tcl wrappers. Code-inspection commands succeeded except expected no-match/missing-at-base checks; an incidental attempted read of `rust/tcl-compiler/src/unit.rs` failed because it does not exist, and was corrected by reading `compilation_unit.rs`.


### Additional review readings and bookkeeping

Read-only `rg`, `rg --files`, `cat`, `sed`, `nl`, `git show`, `git blame`, diff, manifest, target/version, status, disk and process probes located and verified all cited source. Their findings and scope are recorded above; no failing guessed filename is treated as a branch failure. Report assembly reads the scratch evidence, writes this Markdown only after CI has completed, verifies its embedded deletion instruction and finding groups, runs `git diff --check`, then commits only this file locally. No push, merge, source repair, issue or PR write is performed.

### Unexecuted native extension workflow commands

These command templates were read from the workflow; they were not executed. The completed browser-host test does not cover this native Electron lane.

`.github/workflows/ci.yml:521` defines `build-tcl-lsp-server`, which executes at line 575:

```sh
cargo build -p tcl-lsp-server --profile ci
```

It uploads `target/ci/tcl-lsp-server` as `tcl-lsp-server-release`. Each consumer downloads this artefact to `target/release`, marks it executable, and sets `TCL_LSP_SERVER_BIN=$GITHUB_WORKSPACE/target/release/tcl-lsp-server`.

The native producers are `test-ext-partition` (line 600; three explicit whole-file partitions) and `test-ext-multi-folder` (line 787). Their workflow commands are:

```sh
make check-vscode-test-partitions
# Producers also run npm ci, npm run lint, make copy-canonical, and npx tsc -p ./.
TCL_LSP_TEST_PARTITION=1/3 TCL_LSP_SERVER_BIN=/absolute/server/path make test-ext-partition
TCL_LSP_TEST_PARTITION=2/3 TCL_LSP_SERVER_BIN=/absolute/server/path make test-ext-partition
TCL_LSP_TEST_PARTITION=3/3 TCL_LSP_SERVER_BIN=/absolute/server/path make test-ext-partition
TCL_LSP_SERVER_BIN=/absolute/server/path make test-ext-multi-folder
```

For sequential local execution, preserve each partition's `.vscode-test/mocha-result.json` before the next runner replaces it. The aggregate `test-ext` job (workflow line 967) verifies all four producer metadata files using:

```sh
node scripts/dev/verify-vscode-test-results.mjs "$GITHUB_WORKSPACE" "$RESULT_ROOT"
```

The result root must contain `partition-1/mocha-result.json`, `partition-2/mocha-result.json`, `partition-3/mocha-result.json`, and `multi-folder/mocha-result-multifolder.json`.

### Completed CI command/result ledger

Initial host failures and subsequent successful retries are both preserved below. Missing elapsed values were not measured.

| Step | Exact command or recorded operation | Exit | Elapsed seconds |
|---|---|---:|---:|
| lock-metadata:Cargo.toml | `cargo metadata --manifest-path Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:editors/zed/Cargo.toml | `cargo metadata --manifest-path editors/zed/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:runtime/rust/Cargo.toml | `cargo metadata --manifest-path runtime/rust/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/bigip-query-wasm/Cargo.toml | `cargo metadata --manifest-path rust/bigip-query-wasm/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/bigip-report-gen/python/Cargo.toml | `cargo metadata --manifest-path rust/bigip-report-gen/python/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/bigip-report-gen/wasm/Cargo.toml | `cargo metadata --manifest-path rust/bigip-report-gen/wasm/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/tcl-explorer-wasm/Cargo.toml | `cargo metadata --manifest-path rust/tcl-explorer-wasm/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/tcl-lsp-server-wasi/Cargo.toml | `cargo metadata --manifest-path rust/tcl-lsp-server-wasi/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/tcl-lsp-server-wasm/Cargo.toml | `cargo metadata --manifest-path rust/tcl-lsp-server-wasm/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/tcl-spec-studio-wasm/Cargo.toml | `cargo metadata --manifest-path rust/tcl-spec-studio-wasm/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/tcl-vm-wasm/Cargo.toml | `cargo metadata --manifest-path rust/tcl-vm-wasm/Cargo.toml --locked --format-version 1` | 0 | — |
| lock-metadata:rust/zed-query-check/Cargo.toml | `cargo metadata --manifest-path rust/zed-query-check/Cargo.toml --locked --format-version 1` | 0 | — |
| standalone-wasip1-target | `rustup target add wasm32-wasip1` | 0 | 13.56 |
| diff-check | `git diff --check origin/rust...HEAD` | 0 | — |
| lint-py | `make lint-py` | 0 | — |
| known-dialect-drift | `cargo xtask dialect-drift` | 1 | — |
| lsp-browser-wasm | `make lsp-server-wasm-test` | 0 | 985.1 |
| rust-check | `make rust-check` | 0 | — |
| lsp-wasi | `make lsp-server-wasi-test` | 2 | 426.06 |
| nextest-shard-1 | `bash scripts/dev/rust-test-binary-shard.sh run 1/5 --no-fail-fast` | 0 | 425.32 |
| nextest-list-1 | `bash scripts/dev/rust-test-binary-shard.sh list 1/5 --message-format json` | 0 | 8.85 |
| cleanup-fast-targets | `See the CI recovery/setup ledger above` | 0 | — |
| lsp-wasi-retry | `See the CI recovery/setup ledger above` | 0 | — |
| nextest-shard-2 | `bash scripts/dev/rust-test-binary-shard.sh run 2/5 --no-fail-fast` | 0 | 1080.3 |
| nextest-list-2 | `bash scripts/dev/rust-test-binary-shard.sh list 2/5 --message-format json` | 0 | 9.94 |
| nextest-shard-3 | `bash scripts/dev/rust-test-binary-shard.sh run 3/5 --no-fail-fast` | 101 | 26.03 |
| enospc-clean-recovery | `cargo clean (owned main per-worktree target; failed shard 3 ENOSPC recovery)` | 0 | — |
| nextest-shard-3 | `bash scripts/dev/rust-test-binary-shard.sh run 3/5 --no-fail-fast` | 0 | 436.95 |
| nextest-list-3 | `bash scripts/dev/rust-test-binary-shard.sh list 3/5 --message-format json` | 0 | 11.29 |
| cleanup-completed-shard-3 | `python3 /tmp/spectcl-review/cleanup-completed-shard.py 3 --apply` | 0 | 5.8 |
| nextest-shard-4 | `bash scripts/dev/rust-test-binary-shard.sh run 4/5 --no-fail-fast` | 0 | 399.45 |
| nextest-list-4 | `bash scripts/dev/rust-test-binary-shard.sh list 4/5 --message-format json` | 0 | 8.88 |
| cleanup-completed-shard-4 | `python3 /tmp/spectcl-review/cleanup-completed-shard.py 4 --apply` | 0 | 4.72 |
| nextest-shard-5 | `bash scripts/dev/rust-test-binary-shard.sh run 5/5 --no-fail-fast` | 0 | 444.64 |
| nextest-list-5 | `bash scripts/dev/rust-test-binary-shard.sh list 5/5 --message-format json` | 0 | 10.64 |
| cleanup-completed-shard-5 | `python3 /tmp/spectcl-review/cleanup-completed-shard.py 5 --apply` | 0 | 6.85 |
| shard-metadata | `cargo metadata --no-deps --locked --format-version 1` | 0 | 0.04 |
| shard-coverage | `python3 scripts/dev/verify-nextest-binary-shards.py --partition-count 5 /tmp/spectcl-review/rust-tests-metadata.json scripts/dev/rust-test-binary-shards.tsv /tmp/spectcl-review/rust-tests-1-5.json /tmp/spectcl-review/rust-tests-2-5.json /tmp/spectcl-review/rust-tests-3-5.json /tmp/spectcl-review/rust-tests-4-5.json /tmp/spectcl-review/rust-tests-5-5.json` | 0 | 0.08 |
| spectcl-compat | `make test-spectcl-compat` | 0 | 95.56 |
| cargo-deny-ci-version | `cargo deny --version` | 0 | 0.03 |
| cargo-deny-ci-exact | `bash scripts/dev/cargo-deny-all.sh` | 0 | 27.09 |
| typecheck-py | `make typecheck-py` | 0 | 177.25 |
| test-py-engine | `make test-py-engine` | 0 | 1.09 |
| cargo-deny-ci-release-api | `curl -fsSL --max-time 60 -o /tmp/spectcl-review/cargo-deny-release.json https://api.github.com/repos/EmbarkStudios/cargo-deny/releases/tags/0.20.2` | 0 | — |
| cargo-deny-ci-install | `python3 /tmp/spectcl-review/install-cargo-deny.py` | 0 | — |
| doctests | `cargo test --workspace --all-features --doc --no-fail-fast` | 0 | 81.26 |
| lsp-e2e-archive | `cargo nextest archive -p tcl-lsp-server --all-features --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst` | 0 | 74.64 |
| lsp-e2e-all-list | `cargo nextest list --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst --workspace-remap /workspace/tcl-lsp --message-format json` | 0 | 1.74 |
| lsp-e2e-list-1 | `cargo nextest list --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst --workspace-remap /workspace/tcl-lsp --partition hash:1/3 --message-format json` | 0 | 1.72 |
| lsp-e2e-1 | `cargo nextest run --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst --workspace-remap /workspace/tcl-lsp --no-fail-fast --partition hash:1/3` | 0 | 176.78 |
| lsp-e2e-list-2 | `cargo nextest list --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst --workspace-remap /workspace/tcl-lsp --partition hash:2/3 --message-format json` | 0 | 1.79 |
| lsp-e2e-2 | `cargo nextest run --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst --workspace-remap /workspace/tcl-lsp --no-fail-fast --partition hash:2/3` | 0 | 156.1 |
| lsp-e2e-list-3 | `cargo nextest list --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst --workspace-remap /workspace/tcl-lsp --partition hash:3/3 --message-format json` | 0 | 1.79 |
| lsp-e2e-3 | `cargo nextest run --archive-file /tmp/spectcl-review/lsp-e2e.tar.zst --workspace-remap /workspace/tcl-lsp --no-fail-fast --partition hash:3/3` | 0 | 130.72 |
| lsp-partition-coverage | `python3 scripts/dev/verify-nextest-partitions.py --partition-count 3 /tmp/spectcl-review/lsp-all.json /tmp/spectcl-review/lsp-1-3.json /tmp/spectcl-review/lsp-2-3.json /tmp/spectcl-review/lsp-3-3.json` | 0 | 0.04 |
| nextest-heavy | `cargo nextest run --no-fail-fast -p tcl-irule-test -p f5-cli -p tcl-fuzz --all-features` | 0 | 107.38 |
| runtime-no-tommath | `make runtime-rust-test-no-tommath` | 0 | 36.07 |
| runtime-tommath | `make runtime-rust-test` | 0 | 24.2 |
| wasip1-target | `rustup target add wasm32-wasip1` | 0 | 0.05 |
| c-extension-wasm | `make check-c-extension-wasm` | 0 | 0.4 |
| wasm-real-link | `cargo test -p tcl-compiler --test wasm_real_link --test wasm_tiers -- --test-threads=1` | 0 | 103.18 |
| engine-wasm | `cargo test -p tcl-engine-wasm` | 0 | 183.03 |
| lsp-wasip1-check | `cargo check -p tcl-lsp-server --lib --target wasm32-wasip1` | 0 | 39.0 |
