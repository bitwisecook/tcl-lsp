# Test scrub: Tk widgets and TclOO navigation

Ledger: [`ledger/tk-oo.tsv`](ledger/tk-oo.tsv), measured on `work` at `7e1306a5` (17 November 2026).

The area has no crate family of its own; it takes the failing tests its issues name (#832, #1655, #1956,
#2324 and the TclOO navigation behind them). The ledger has 63 rows in two parts:

- **35 rows triaged here.** The 29 failing `tcl-lsp-server` e2e `tcloo_navigation` tests (routed to
  lsp-features by server-e2e-db, not in any other ledger), and six `tcl-compiler` library tests no other
  ledger holds (three `analyser::commands` widget tests, three `analyser::tk_checks` declared-provides
  tests).
- **28 rows cross-listed.** The 27 failing `tcl-lsp-core` callback tests and the tcllib clay corpus test,
  whose verdicts lsp-features and class-facts already made. I read each and agree; the rows carry their
  verdict and evidence with the source named, and they are not counted twice in the totals below.

Every TclOO fixture behind a KEEP-ORACLE row was run on tclsh 8.6.18, 9.0.4 and 9.1.0; on 8.4.20 and
8.5.19 `oo::class` is an invalid command (no TclOO), and `classmethod` exists from 9.0. The Tk `bind`
shape was run on Tk 9.0.3 (python tkinter under xvfb), the only Tk interpreter here. Itcl is not
available under `tmp/` (`package require itcl` fails on every release).

## Counts by verdict

| Verdict | Triaged here | Cross-listed |
| --- | ---: | ---: |
| KEEP-ORACLE | 20 | 5 |
| FIX-FIXTURE | 12 | 15 |
| KEEP-CONTRACT | 3 | 4 |
| REWRITE | 0 | 3 |
| MERGE | 0 | 1 |

No RETIRE, FIX-EXPECTATION or UNDECIDED rows.

## Causes ranked by tests blocked (triaged here)

| Cause | Tests | Verdicts |
| --- | ---: | --- |
| `uncalled-procedure-facts` | 7 | FIX-FIXTURE 7 |
| `classmethod-rename-refused-incomplete-coverage` | 3 | KEEP-ORACLE 3 |
| `variable-receiver-definition-empty` | 3 | KEEP-ORACLE 3 |
| `renamemethod-destination-rename-no-edits` | 3 | KEEP-ORACLE 3 |
| `renamemethod-collision-refusal-reason` | 3 | KEEP-ORACLE 3 |
| `tk-fixture-without-tk` | 3 | FIX-FIXTURE 3 |
| `declared-provides-not-applied` | 3 | KEEP-CONTRACT 3 |
| `workspace-bare-receiver-namespace-resolution` | 2 | KEEP-ORACLE 2 |
| `cross-file-override-family-unavailable` | 2 | KEEP-ORACLE 2 |
| `rename-refused-unclosed-member-family` | 1 | KEEP-ORACLE 1 |
| `per-object-visibility-not-applied` | 1 | KEEP-ORACLE 1 |
| `renamemethod-destination-not-a-member` | 1 | KEEP-ORACLE 1 |
| `bareword-object-call-hover-empty` | 1 | KEEP-ORACLE 1 |
| `after-callback-my-outside-object` | 1 | FIX-FIXTURE 1 |
| `itcl-fixture-without-package` | 1 | FIX-FIXTURE 1 |

The seven `uncalled-procedure-facts` rows follow the lsp-features verdict on their core twins: each
callback is captured in a method nobody calls, and which provider `[self] tick` or `my read` reaches
depends on the receiver's class, so the rename-grade assertions need a call (`[Child new] wire`). The
oracle agrees with every expectation; only the fixture is short of a call.

## Distinct failing behaviours

No wrong answer is shown: in every row the product abstains or answers nothing where tclsh shows an
answer. One is not yet ruled out: two collision rows (`renamemethod old old`, onto a live sibling) fail
because the rename is not refused, and the test does not show whether it answered with edits; their
fresh-name twins answer with no edits. That is the first thing to check when the cause is taken up.

1. A bare class receiver in a consumer document does not resolve through the current namespace or a
   `namespace import` (`namespace eval ::a {C cm}` with both `::a::C` and `::C`; `W cm` after
   `namespace import ::lib::*`).
2. Renaming a 9.0 `classmethod` is refused (`incomplete-original-member-coverage`), and renaming a
   class-side `self` method from a pure consumer is refused (`unclosed-original-member-family`), although
   definition and references reach both.
3. `$o cm` gives no definition when the receiver is a variable holding `[C new]` and the class also
   has a class-side member of the same name, or a sibling object has a per-object `unexport`.
4. A per-object `oo::objdefine $o {export M}` does not revive the member for `$o M`.
5. `renamemethod old new` does not make `new` a navigable member, renaming its destination word answers
   with no edits, and the three shapes Tcl rejects (`renamemethod old old`, onto a live sibling, a sibling
   onto a later destination) are not refused with the interpreter's error.
6. The cross-file override family is unavailable: references from `Animal::speak` return null and the
   lens reads "References unavailable" where the sibling's `Dog` override and `$d speak` belong.
7. Hover on a call through a bareword object (`Widget create w; w reopened hi`) is empty while the
   declaration hovers.
8. A declared `package X provides Tk` (directive or `[packages.provides]`) activates the Tk checks but not
   availability: W120 or H301 still fires (`docs/design/contracts/package-loading.md` rules 16 to 19).
9. Callback method words captured in uncalled methods (`[list [self] tick]`, `[namespace code [list my
   read]]`, `bind ... [list [self] animTick %x %y]`) are not method references; to be re-read once the
   fixtures call the method.

## Overlap groups touching these tests

- No exact-copy group.
- Overlap group `92645d20273d`: `tp_renaming_a_moved_member_onto_a_live_sibling_is_refused` and
  `tn_renaming_a_moved_member_onto_an_earlier_deleted_name_is_allowed`. Not a merge: the second deletes
  `sib` before the rename and expects the opposite answer (tclsh: `method called sib already exists`
  against a class created with `[C new] sib -> 1`); the grouping matches on helpers and spellings only.
- Cross-layer twins, not merged: seven e2e callback tests repeat a core `callback_targets` test on the same
  source (for example `tp_overriding_receiver_keeps_its_callback_in_its_own_family` and
  `an_overriding_receiver_keeps_its_callback_in_its_own_family`). The e2e test holds the server's
  agreement across definition, references, lens, rename and call hierarchy, which the core test does not;
  the second pass can decide whether one per shape is enough.

## For #1655

The clay corpus row (class-facts) names the blocker: `package require md5 2` at load time leaves the
rest of `clay.tcl` unproved, so no class is recorded. That is where the #1655 bisection starts.
