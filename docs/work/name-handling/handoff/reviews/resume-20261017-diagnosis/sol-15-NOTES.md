# Sol-15: closing the open obligations of worklist rows #2 and #14

Base: `/home/user/tcl-lsp` HEAD `db5a1ba9a` on branch `work`. The repository was not modified. This was static work only: no cargo, no test runs, and no binary was executed. The only programs run were two small tclsh probes.
Failure evidence: capture41, `w/lsp-41/original-tcl_lsp_core/test-{513,608,626,628,629}.log`. The same five selectors also fail in lsp-36, lsp-37 and lsp-38. None of them has passed since its controls were added; the quoted-body selector last passed in frozen091.
`git log` shows that both consumer files (`signature_help.rs`, `linked_editing_range.rs`/`linked_authoring.rs`) are unchanged since before capture41.

Files in this directory:
- `row14-lsp-linked-logical-name-geometry.patch`: `diff -ruN a b` from `row14/`. `git apply --check` against HEAD: OK.
- `row2-PROPOSAL-513-fixture-REQUIRES-USER-EXCEPTION.patch`: a fixture-correction proposal that is not part of any closure. `git apply --check`: OK. It changes a fixture, so it needs an explicit user exception.
- `probe/`: the tclsh probe for 513.

---

## Row #14 `lsp-linked-logical-name-geometry`

Catalogue (`repository-consumer-audit-final.json`, `/units/621`):
- required_change: "Legacy String source text + trailing :: substring derives editable command component extent rather than actual shared name protocol/wholeword/original effective operand. Generic lexical declaration advice branch cannot supply current source edit permission; migrate to genuine source declaration/selector/component owner with explicit live authoring purpose, preserving narrow self-call policy."
- implementation.scope (the recorded open obligation): "Intentional Logical linked source authoring uses genuine retained source declaration and exact name geometry, preserving Native route. Partial: no whole-observer or future-collision equivalence claim."

### Root cause (628 lone-CR lift, 629 surrogate cursor; also the mechanism behind 512's first assertion)

`OriginalLinkedSourceSpelling::capture` (Logical branch) needs `source_logical_procedure_calls` to contain the direct self-call inside the declaration's own body. Calls at top level work: 511 and the definition tests pass. Calls inside any body never survive. The cause is in the tape merge:

1. `retain_source_inventory` (`source_transition_advice.rs:2511-2519`) runs `retain_original_body_catalogue` whenever `procedure.is_none()`. `source_procedure_declaration` returns `None` for every non-Native policy (`source_procedure.rs:256`). So under Logical input, every `proc` body is walked by the syntax-only catalogue `BodyWalk` (`inventory: None`, `logical_scope: None`).
2. That walk's `command_inner` (`body_effects.rs:1225`) calls `retain_logical_procedure_call` on `authored_prefixes`. The catalogue walk never retains `logical_variable_words`; only inventory walks with a logical scope do, at `:1182`. So `capture_logical_procedure_call` has no Named recipe. For a bare head (`recur`, `p`) it has no Absolute recipe either, and the catalogue graph predates the definition anyway. The capture returns `None`, and `merge_receipt` records `None` at the call offset.
3. `LogicalBodyWalk::procedure` (`logical_body.rs`) is the recipe-owning walk. It applies the definition, retains the Named recipe and captures `Some(call)` at the same offset. `merge_receipt` treats `None` versus `Some` as a conflict and stores `None`. The order of the two walks does not matter (`source_class.rs:637`).
4. `logical_procedure_call_at` therefore yields nothing. `linked_editing_ranges` then has one span and returns `None`, which produces the panics at 474 and 511.

The same withdrawal comes from the immediate Logical `namespace eval` walk (`inventory: None` for Structural bodies, `body_effects.rs:614`). That explains why `signature_keeps_real_namespace_frame…` (512) fails at its first in-body call. The Native route is untouched because `procedure` is `Some`, so no catalogue walk runs. Logical navigation (`merge_logical_navigation`) is only retained by recipe-owning walks, which is why call hierarchy inside bodies already passes.

### Patch (`row14-…patch`, 2 compiler files, +34/−5)

- `logical_procedure_invocation.rs`: new `retain_positive_logical_procedure_call`. It merges only a captured `Some`, so a missing recipe is absence and never a conflicting receipt.
- `body_effects.rs` `BodyWalk::command_inner`: a walk can withdraw (keep the existing `retain_logical_procedure_call`) only when it owns the Logical source-word recipe, meaning `inventory.is_some() && logical_scope.is_some()`. This is the same condition as `retain_logical_navigation`. Every other walk is positive-only.

Genuine conflicts are still withdrawn: two different positive receipts, or a recipe-owning walk's own `None`. Positives from catalogue walks (Absolute recipes) are kept exactly as today.

What the patch does not touch: tests, fixtures, budgets, registry, names, and the Native route. It adds no `#[allow]`, no name match, and no Native grant.

Not checked: rustfmt and clippy have not run. The code was hand-formatted to rustfmt style; the borrow order was checked by reading (`owns_logical_recipe` is computed before the `tape` borrow).

### Expected flips (unexecuted)

- 628 `linked_spelling_lifts_lone_cr_source_with_the_actual_lsp_line_geometry`: FAIL→PASS, likely. The `;\r` keeps the lexer split, and the LSP geometry comes from `new_lsp`.
- 629 `linked_spelling_rejects_surrogate_interior_cursor_and_same_named_data`: FAIL→PASS, likely. The data `p` and the surrogate cursor `(1,3)` still return `None`.
- 626 `logical_linked_spelling_uses_the_actual_header_and_direct_call_after_reports_are_removed`: **no flip claimed.** No recipe-less walk visits `::N::café`, because the proc body is nested in `namespace eval` and only `LogicalBodyWalk` enters it. Its blocker was not found statically. The most likely candidate is the declaration-side `original_plain_source_command_tail` for a declaration word inside an immediate `namespace eval` body: the `original_written_word_at_span_in_source` written/catalogue join. Unverified.
- Possible side effects: 512's first assertion may pass. Calls inside Logical bodies (`tcl`/unversioned) now reach `references.rs:173`, W143 suppression (`validity.rs:2357`) and `SourceUserCallAritySubject` (`diagnostic_subject.rs:366`). Expect more Logical in-body references, fewer false W143 and new in-body arity diagnostics. These are improvements, but they can change pass/fail in Logical diagnostic suites, so run them.

### Closure claim

Once 628 and 629 pass, the patch closes the part of "uses genuine retained source declaration and exact name geometry" that covers a top-level Logical declaration and its direct self-calls in its own body: lone-CR lift and surrogate refusal.

**Still open:** the namespace-nested declaration case (626). "No whole-observer or future-collision equivalence claim" stays out of scope as recorded. The row stays PARTIAL until 626 is root-caused and all three selectors pass on a fresh build.

---

## Row #2 `lsp-signature-cursor-source-transport`

Catalogue (`/units/637`):
- required_change: "Cursor uses offset_at_utf16 clamp without exact UTF16 roundtrip; manually lexes truncated region and rebuilds grouped NativeWords instead of purpose-specific current partial cursor receipt. Migrate original partial/source extent transport and retain unavailable/incomplete distinction; no invented closers, preserve source schema and effective arg mappings."
- implementation.scope (the recorded open obligation): "Genuine full configured partial cursor/strict UTF16 adopted; Logical expansion mapping retains explicit unavailability."

**No admissible patch.** The row cannot be counted full yet.

### 513 `signature_cursor_unicode_cr_and_utf16_positions_do_not_clamp`: root cause is the fixture versus the lexer contract

The fixture `"proc 茶 {一 二} {}\r茶 😀 value\r"` has no `;` before the lone CR. The shared lexer treats raw `\r` as a word separator (`WordSeparators::is_separator`, `lexer.rs:2165-2172`; `LineIndex::new` documents this). Lone-CR→LF translation is a server-boundary step (`tcl_lexer::normalise_lone_cr`, applied before analysis in `tcl-lsp-server`). The Core test passes raw text straight to `Analyser`, which sees one 7-word `proc` command. No `茶` declaration exists, the cursor's command is `proc`, `registry` is `None`, and `unwrap()` panics at 1577.

The cursor transport is correct: line 1 col 10 maps to byte 36 and round-trips, and `(1,3)` and `(1,1000)` are refused.

tclsh probe (`probe/`, 8.6.18 and 9.0.4): `source` of these bytes runs two commands and the call is reached. `eval` of the same string gives `wrong # args: should be "proc name args body"`. So the expectation matches tclsh `source`, but not the Core API on raw text. Passing peers (`implementation`/`type_hierarchy` `…lifts_lone_cr…`, linked 628) all write `;\r`. The failing call-hierarchy 501 (`"# 😀\r…"`) is the same class.

Code fix options, neither of them minimal:
- Make the lexer treat lone CR as a terminator for Document images. This is a shared-owner change that contradicts the `LineIndex::new`/CST contract and affects every Core consumer.
- Normalise inside `Analyser`. This breaks the image==source currency check.

Proposal (needs a user exception, not admitted): add `;` before `\r` in the fixture and in its "other" control. That gives the patch in this directory. All assertion values and the line-1 UTF-16 columns stay the same, and it should then pass on the 511 top-level Logical path.

### 608 `substitution_free_quoted_proc_body_is_a_nested_script_region` (unterminated sub-case): not root-caused statically

The panic is at `expect("unterminated quoted body command signature")`. The terminated sub-case passes. With an analysis present, the walker skips `literal_body_region`, so `innermost_analysis_executable_region_at` relies only on `original_incomplete_body_region_at` (Native policy, tcl8.6). The failing link is one of:

- (a) the incomplete region: the `header()` cut and segment, the row-words equality, or `procedure_namespace`;
- (b) the child lookup through `incomplete_body_operand_layout_advice`/`incomplete_child_layout_advice`, which returns `source_words = None`.

The cursor (`argument_cursor_in`) and `source_signature_help` were checked by reading and match the passing sub-case.

The existing `TCL_LSP_TRACE_INCOMPLETE_HEADER=1` traces (compiler plus signature `stage=command/selected`) locate it in one run: no `stage=command` line means (a); `stage=selected available=false` means (b). Also run `tcl_compiler … incomplete_body_metadata::tests::incomplete_original_body_prefix_has_grammar_without_a_complete_body`. It is not in capture41, and it uses `from_grammar` while the LSP uses `for_file_grammar`.

### Closure claim

Nothing is closed. The recorded obligation needs 513 (user decision) and 608 (diagnosis by execution). "Logical expansion stays explicitly unavailable" is unchanged and not at risk.

---

## Validation owed (after the guarded capture finishes)

1. Apply the row-14 patch and do an ordinary build of tcl-compiler and tcl-lsp-core. Run selectors 626/628/629/512 and the `linked_editing_range::*` and `logical_procedure_invocation::tests::*` modules, plus the Logical diagnostic, references and code-lens suites.
2. Run 608 with `TCL_LSP_TRACE_INCOMPLETE_HEADER=1`.
3. `make rust-check`.
4. Use the unchanged 180-second test and 30-second listing budgets, with final guards.
