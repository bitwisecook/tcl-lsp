# Luna-8 review: Sol-6 member census V2 (`member-census-v2.patch`)

Verdict: **HOLD**. The patch applies cleanly to current HEAD, is formatted, and is
well structured. However, three dispatch forms make the census return Complete
while the same document really reaches the member. tclsh 8.6.18 and 9.0.4
confirm each one. All three have small fixes that use registry data only. Admit
once those fixes and their fail-closed tests are added and the patch is built.

Scope: static review only. Nothing was compiled or run. The only commands run
were `nice tclsh8.6/9.0` probes and `rustfmt --check` on stdin.

Paths:
- Scratch copies: `a/` is HEAD; `b/` is HEAD with the patch applied.
- Probes: `probes/p1.tcl`, `probes/p2.tcl`, `probes/p3.tcl`.
- Probe output: `probes/probe-results.txt`.
- Verdicts from the lsp-36 capture at f606aa20c: `cap/verdicts.json`.

## 0. Rebase onto 60051c3c2 / bb7277d82

**No rebase is needed.**

- `git apply --check` is clean against HEAD.
- None of the 8 patched files changed between 84cf7c884 and HEAD.
- My `a/` copies are byte-identical to Sol's `a/`. Applying the patch with
  `patch -p1` gives a `b/` that is byte-identical to Sol's `b/`.

Integration37 touches different files: `handlers.rs`, `state.rs`,
`logical_members.rs`, `definition.rs`, `references.rs` and
`original_call_hierarchy.rs`.

- The new `source_self_method_edges` reads the same method tape, read-only. It
  does not interact with the census.
- The `handlers.rs` change makes inline `oo::define C method …` member forms
  analysable. The census still fails closed on those forms through its
  "definer without a selected definition body" rule.

## 1. Soundness: holes found

### H1. `subst` templates are an unwalked command-evaluation context (HIGH)

`subst` has no Body role. Its template is evaluated through
`body_execution: SubstitutionTemplate` and `Traits::PERFORMS_SUBSTITUTION`
(`tcl-registry/src/commands/tcl/subst_.rs:308`).

- Neither the checked walker nor the CheckedSyntax coverage descends into the
  template.
- `classify` gives `subst` the result `Other`: its roles are known, and
  `authored_source_retains_callback()` answers `Some(false)` because there is no
  prefix descriptor (`original_declaration.rs` b:1166).
- The mention net only catches a literal `ping` token. A selector that is
  escaped or computed escapes all three checks:
  - `method body {} {subst {[my p\x69ng]}}` (`census_tokens` splits the raw
    braced text at `\`).
  - `method body3 {} {set s [string cat pi ng]; subst {[my $s]}}`.
- tclsh 8.6.18 and 9.0.4 both dispatch `C::ping` (`probes/p1.tcl`).

The census then returns Complete with 0 spans. The plain equivalent, `my $s`, is
already treated as Unproven, so this is also internally inconsistent.

**Fix:** in `classify`, return `None` when
`schema.semantics.traits.contains(Traits::PERFORMS_SUBSTITUTION)`, or when the
spec's `body_execution` is a substitution template. Add both probe shapes to
`readonly_member_census_fails_closed_for_deferred_and_unproven_dispatch`.

### H2. Qualified `oo::Helpers::next` / `nextto` dispatch with no trait (HIGH)

`oo_helpers.rs:153` `QUALIFIED_SPELLING_EXCLUDED` strips these traits from the
qualified spellings:
- `TCLOO_NEXT_CHAIN`
- `TCLOO_BINDS_METHOD_ALIAS`
- `TCLOO_SELF_DISPATCH`
- `TCLOO_METHOD_CONTEXT`
- `TCLOO_REQUIRES_METHOD_FRAME`
- `TCLOO_INTROSPECTION`

This lets the following source pass the census:

```
oo::class create Child {superclass Base; method ping {} {::oo::Helpers::next}}
```

- `::oo::Helpers::next` classifies as `Other`.
- It has no operands, so it records no mention.
- The census for `Base::ping` is Complete.

tclsh 8.6/9.0 print `Base::ping reached` (`probes/p1.tcl`). The project already
counts next-chain sites as references (e2e
`test_constructor_lens_counts_and_resolves_subclass_next_chain`). NOTES §Risks
5 says this form "names no member, so the census is unaffected". That reasoning
is wrong: bare `next` names no member either, and the census correctly treats
bare `next` as incomplete.

### H3. Qualified `oo::Helpers::link` with a computed alias shadows a builtin (MEDIUM/HIGH)

```
constructor {} {::oo::Helpers::link [list set [string cat pi ng]]}
method body {} {set a 1}
```

- tclsh 9.0.4 prints `C::ping via linked set (a 1)` (`probes/p2.tcl`).
- In the census, qualified `link` has no `TCLOO_BINDS_METHOD_ALIAS` trait and
  no literal mention.
- `set a 1` then classifies as the global `set`, so the census is Complete.
- Bare `link` is correctly incomplete.

**Fix for H2 and H3:** a registry decision.
- **Preferred:** keep the "what the call does" traits (`TCLOO_NEXT_CHAIN`,
  `TCLOO_BINDS_METHOD_ALIAS`) on the qualified twins. Strip only the
  "where it may be written" traits (`TCLOO_METHOD_CONTEXT`,
  `TCLOO_REQUIRES_METHOD_FRAME`). This needs an audit of the consumers of those
  traits: `references.rs`, `call_hierarchy.rs`, `definition.rs`, `analyser/oo.rs`
  and `original_linked_member.rs`.
- **Interim:** fail closed in the census for any spec with
  `implementation_namespace: Some(_)`. That is registry data only: it covers
  `oo::Helpers::*` and `tcl::dict::*`, which are rare and conservative.

Add these fail-closed tests:
- `::oo::Helpers::next` inside an overriding subclass method (tcl8.6 and tcl9.0);
- `oo::Helpers::nextto Base`;
- `oo::Helpers::link [list set [string cat pi ng]]` (tcl9.0).

### Forms checked and found fail-closed (by static reading)

| Form | Why it fails closed |
|---|---|
| `my $x`, `my {*}`, `my p\x69ng` (non-literal) | Unproven |
| Untaped `my X` (`my variable`, `my eval {…}`, `my configure -p`, `my destroy`, builtins) | walked ≠ tape |
| `[self] m`, `$obj m`, `[C new] m`, `{*}$cb` | dynamic head |
| Object command `c m` | unknown head, or a `ping` mention |
| `next`, `nextto` (bare) | `NEXT_CHAIN` |
| `callback`, `mymethod`, `link`, `classvariable` (bare) | `METHOD_CONTEXT` without `INTROSPECTION` |
| `after`, `trace`, `fileevent`, `package ifneeded` | `DEFERS_BODY` |
| `namespace code` | `WRAPS_COMMAND_PREFIX` |
| `lsort -command`, `interp alias`, `tailcall`, `yieldto`, `coroutine`, `coroinject` | command-prefix descriptors |
| `eval $s`, `uplevel $s`, `namespace eval ns $s`, `interp eval {} $s`, `if 1 $b`, `expr $e` | dynamic Body or Expr geometry, so coverage is Incomplete |
| `apply` | `LambdaLiteral`, so coverage is Incomplete |
| `eval my $m`, `namespace inscope ns my $m` | concatenated script: the Body walks `my` alone and gets Unproven |
| Tk `-command [list …]` (deferred Body option) | dynamic Body, so coverage is Incomplete |
| Literal Tk `-command {…}` | walked, then fails tape reconciliation |
| Members `forward`, `filter`, `mixin`, `export`, `unexport`, `deletemethod`, `renamemethod` | rejected by effect, routed through the member row (`self forward` resolves to the wrapped Forward) |
| Inline `oo::define C …` and `oo::objdefine o …` without a body | definer without a selected body |
| `unknown` handler `my $m` | Unproven |
| `configure -prop`, `cget -prop` (inside or outside) | untaped `configure`, or a `-p…` mention |
| Literal in another class (`[list my m]`, `{my m}`, `return [list my m]`, `oo::Helpers::callback m`) | mention net |
| Superclass and subclass `my m` | taped through the inherited relation, or untaped, which makes it incomplete |
| `mixin` or `filter` in another class | member effect rejected |

### Residual trust and assumptions (document them; not blockers)

- **Tape identity.** The census checks span-set equality between walked
  self-dispatches and the typed tape. It inherits the tape's per-identity
  attribution and MRO, and does not recheck them.
- **Class family.** There is no family gate. `original_class_declarations` is
  family-generic (`analyser/oo.rs:735`). For itcl, a bare-name method call in a
  method body is only safe if `source_registry_words` cannot resolve that head.
  Add an itcl/snit fail-closed control, or gate on the identity family's
  self-dispatch descriptor.
- **Runtime command absence.** A registry-known command that is absent at
  runtime can fall into a `namespace unknown` handler. This is out of model.
  `source` of another file is outside the per-document contract.
- **Property tests do not match tclsh 9.0.4** (`probes/p3.tcl`):
  - `my size` raises `unknown method "size"`;
  - `property` inside a plain `oo::class create` body raises
    `invalid command name "property"`, because it needs `oo::configurable`.
  The property census counts `my size` as a reference. That modelling predates
  this patch and is sound only as over-count: real accesses (`configure`/`cget`,
  `<ReadProp-…>`) are caught. It is a candidate for the "Correct to tclsh"
  policy, not for this patch.

## 2. The new ResolvedInvocation queries

`authored_source_self_dispatch` (resolved_invocation.rs b:3905) and
`authored_source_retains_callback` (b:3928) are pure registry data. They read:
- traits;
- selected descriptors' `command_prefixes` / `command_prefix_resolver`;
- authored and static roles, `arg_role_resolver_roles`, and option
  `value_role`.

Neither has a hardcoded name.

Unresolved subcommands are handled correctly:
- `retains_callback` returns `None` for `Indeterminate`, `Unknown` and
  `Ambiguous`.
- The early `Some(true)` for `WRAPS`/`DEFERS` is conservative.
- `self_dispatch` deliberately ignores subcommand resolution: the trait fixes
  the layout, and a missing or non-literal selector gives `Unproven`.

Two notes:
- `retains_callback` does not consider `deferred_script()` Body-valued options
  (Tk `-command`). That is safe today only because the CheckedSyntax coverage
  walks Body roles, including option values, and refuses dynamic ones. A
  sentence in the doc comment saying so would prevent misuse by other
  consumers.
- The registry test expects `my ping $x` to give a callback of `None`. That
  relies on how `allow_unknown_subcommands` resolves an unlisted selector,
  either `Unknown` or `NotApplicable`. It does not affect soundness, because
  the census never asks about callbacks for a self-dispatch, but it is a
  possible red registry test.

## 3. The server guard (readonly_reference_reports.rs b:1712)

**Keep it.** It is correct and needed.

- The core census covers only the owner's document.
- The server's `targets` add only typed cross-document references.
- `original_declaration_documents` (`lib.rs:14659`) retains the workspace-index
  documents plus the open documents. Without the guard, a `$o ping` in another
  retained file would be undercounted while the server claims "N references".

Currently-passing tests that flip because of the guard: **none found**.
- In the lsp-36 capture, no server test currently observes
  `complete-source-census`.
- The e2e servers use a nonexistent root (`file:///e2e/root-PID`), so they hold
  one document.
- Server unit tests register one document, except Sol's new test.

Follow-ups (not blocking):
- **Inconsistent lens tag.** The unresolved lens's data tag comes from core
  `lens.reference_coverage` without the guard (`lib.rs:33040`). With 2 or more
  documents, the unresolved lens says `complete-source-census` and the resolved
  lens says `partial`. Align them.
- **Narrowing.** In real workspaces there is always more than one retained
  document, so editor lenses never show Complete. A later narrowing could run
  the mention and head checks over the other retained documents instead.
- **Unindexed files** and `source`d files remain outside the guard. The contract
  text already says the census is per document.
- **Clickability.** Unchanged by this patch: `lens_command` makes a Complete
  census with 0 locations non-clickable (`readonly_reference_reports.rs:167`).
  These tests will therefore still fail on the command id even when the title
  becomes "0 references":
  - e2e `test_method_lens_zero_when_uncalled` and
    `test_property_lens_zero_when_unused`;
  - server `code_lens_resolve_for_unused_method_is_clickable_with_zero_locations`.

## 4. Test expectation changes

The changes are limited as required.

- **test-096** (`original_declaration.rs` b:1976): only `Partial` becomes
  `CompleteSourceOccurrenceCensus`. The source is
  `oo::class create C {method ping {} {}; method body {} {my ping; my ping}}`.
  Every command is either a member or a taped `my ping`. The only mention is
  inside the walked class-body region of the `oo::class create` word. The spans,
  the kernel assertion and the strong-inventory assertions are unchanged.
- **Server wire test:** only coverage, the `"partial"` tags (×2) and the title
  `"at least 2 references"` → `"2 references"` change, for the same source with
  one retained document. Sol kept the test name; renaming it is optional.

Risk: both tests **pass today** (lsp-36 capture). After the patch they pass only
if the unbuilt census actually reaches Complete, which relies on
`source_registry_words` resolving `my ping` with roles inside method bodies.
Build and run these two tests first.

## 5. Blast radius of the `census_command_bodies` change (source_structure.rs b:1615)

Before the patch, a member head inside a class body fell to a global lookup,
which marked the CheckedSyntax census unavailable. So no document with a
member-bearing class body could reach Complete, and method bodies were never
queued as regions.

After the patch:
- such documents can reach Complete;
- `coverage.commands()` now **also contains the commands inside method
  bodies**.

**Likely fail→pass** (all currently FAIL in lsp-36 unless noted):
- core `code_lens` 056 `lens_counts_method_calls_within_class_body`;
- core `code_lens` 058 `lens_reports_zero_for_uncalled_method`;
- core `code_lens` 059 `lens_counts_property_my_dispatch_within_class_body`;
- core `code_lens` 060 `lens_reports_zero_for_unused_property`;
- `readonly_property_reports_promote_only_a_complete_typed_source_census`;
- `rename_safety` FN guards on class documents, if the whole document has known
  heads (not in the capture): `fn_guard_allows_a_literal_my_dispatch`,
  `tn_allows_an_ordinary_member_rename_with_no_renamemethod_in_the_body`.

**At risk, either direction:**
- `call_hierarchy::logical_source::genuine_own_member_frame_and_selectors_keep_unrepresented_inherited_alternatives`
  (FAIL now). Its `incomplete` flag comes from applicability, and its outgoing
  edges now include method-body commands.
- Server `logical_call_wire_tests` / `logical_hierarchy_tests` (FAIL now) use the
  same inventory.
- `code_lens::logical_source` class lenses: targets now include factory calls
  inside method bodies. The current tests have none, so they should stay PASS.
- `references::DispatchRegionCensus` (`namespace_rename.rs:487`) and
  `receiver_identity::uneditable_method_selector`: class documents can now be
  censused. Namespace and method rename refusals that depended on "potential
  source coverage unavailable" in class documents can turn into acceptances.
  The current tests that assert that refusal use non-class sources, so they are
  unaffected.
- Minify: when the census is complete, `CommandCensus` no longer forces
  `all_variable_scopes`/`procs` barriers. Member heads (`method …`) still have no
  global schema, so `find_rename_barriers` keeps all barriers. Risk is low, but
  add a TclOO compaction control (`variable x` member plus `$x` in a method).
- `extract_variable` / `inline_variable`: checked-walk refusals in method bodies
  can now become Complete. No class-based tests were found.
- `completion.rs:2034`: `scoped_environment_regions` now gains nested bodies
  inside method bodies (member bodies themselves are not pushed). Low risk.
- `proposal_diagnostics.rs:188`: project class-factory recompute now admits
  class documents. This is behavioural; there is no direct test.
- `code_actions` package census: `package require` inside method bodies is now
  visible. A member disposition of `Unavailable` now pushes `unavailable`
  instead of `syntax_unavailable`, so a negative may be refused where it used to
  be allowed.

**Likely pass→fail:** only the two §4 tests, and only if the census does not
build or reach Complete.

**Stays PASS:**
`property_source_census_unclassified_receiver_tests::unclassified_object_receivers_and_unknown_heads_preserve_partial_counts`
(dynamic or unknown heads stay Partial).

## 6. Compile, format and style

**Compile.** All signatures match:
- `visit_checked_analysis_executable_commands_with_regions`;
- `with_source_schema` (`Arc` deref);
- `definition_member_at` (`region` borrows `&self.structure`);
- `script_body_disposition`, `original_command_words_at`, `content_span`,
  `definition_parent`;
- `checked_references` / `kind()` (`MemberRefKind: PartialEq`);
- `argument_words_for_role`;
- `ExecutableText::Decoded(Vec<u8>)` (deref to `&[u8]`);
- `TokenType::Sep`/`Eol`;
- `SignatureSourceNameInput::bytes`;
- `OptionSpec::value_role`, `InvocationOptions::available` (Copy);
- `AuthoredSourceDescriptors` (exactly 2 fields);
- the command-prefix field names;
- the tape re-exports and the `command_span` return types (method
  `Option<Span>`, property `Span`).

Compile risk is low.

**rustfmt.** All 7 `.rs` files pass `rustfmt --edition 2024 --check`. The repo
has no `rustfmt.toml`.

**Style.**
- No `#[allow]` and no `expect(clippy…)`.
- No UK-spelling violations, and no process names (Sol, Luna, fix-N) in code
  comments.
- Comment density is acceptable.

**Clippy watch items:**
- `too_many_lines`: `complete_member_source_census` is about 75 lines and
  `classify` about 80.
- `needless_pass_by_value` on `skip: impl Fn`.

**Performance (unchanged from NOTES).** Each lens runs one full checked walk,
plus a structure capture and a coverage capture. A class with many members does
quadratic work.

## Required before ADMIT

1. Close H1 (`PERFORMS_SUBSTITUTION`) and H2/H3 (keep the dispatch traits on the
   qualified `oo::Helpers` twins, or fail closed on `implementation_namespace`).
   Add the five fail-closed probe shapes as tests.
2. Build and run the registry test, test-096, the server wire test, Sol's two
   new core tests and the new server test. Then run `code_lens` 056–060 and
   admission26.
3. Run `rename_safety`, `namespace_rename`, `call_hierarchy::logical_source`,
   `minify` and the server logical-hierarchy groups to record the §5 flips.
4. Optional:
   - align the unresolved-lens data tag with the guard;
   - add an itcl/snit fail-closed control;
   - document the Body-option assumption in `retains_callback`.
