# Luna-10 review: Sol-10 `inline-objdefine-producer-v1.patch`

I reviewed the patch statically against HEAD `bb7277d82` on branch `work`, which contains integration37 `60051c3c2`. I did not run cargo, tests or gdb, and I did not modify `/home/user/tcl-lsp`.

The only tools I ran were:
- `nice tclsh8.6` and `nice tclsh9.0` probes
- `nice rustfmt --emit stdout`
- read-only Python checks: rendering, a line count, and selection lookups
- read-only `gh api` reads of upstream `bitwisecook/tcl-lsp@rust`

`a/` is byte-identical to HEAD for all four files, and `git apply --check` passes.

## Verdict

**HOLD (v2 needed). The blocker is mechanical; the design is sound.**

| Hunk | Verdict |
| --- | --- |
| `own_object_configuration.rs` producer (walker layout selection + `inline_member_definition`) | **HOLD → fix in v2.** The semantics are sound. It fails `clippy::too_many_lines` (pedantic, `-D warnings`): see C1. |
| `own_object_configuration.rs` new test `original_own_object_inline_member_installs_the_script_layout_entry` | ADMIT, together with the producer |
| `object_instance.rs`: inline row `false → true`, plus the new script-form row `true` | **ADMIT, coupled to the producer.** It is not an original assertion (§2). Admit it only together with the producer, and only on the run condition in §2. |
| `manifest.json` + `tcloo-original-own-object-method-transfer.md` (scope sentence + one test binding) | ADMIT |

## 1. Soundness

- **Layout gate.** HEAD admits only `argc==2 && (1,Body)`. The patch adds `facts.arg_roles_complete` for the inline layout.
  - **What "complete" means.** `oo_define_layout_roles` (`oo_define.rs:427`) returns `Some` whenever argv is exact and argument 1 is a literal exact grammar member whose `source_member_script_layout_arguments` count matches.
  - **NOTES is slightly wrong here.** "Complete" does *not* mean all argv words are literal. `oo::objdefine $object method valid …` and `oo::objdefine d method [list p] {} {}` are both complete.
  - **No hole results.** `inline_member_definition` separately requires every member word (`words[2..]`) to be `Provenance::Source` and Literal/ByteLiteral, so `[list p]` and `"p$m"` are refused there. The `$m` head is refused earlier: the resolver gives `None`, so the roles are incomplete.
- **Refusals I verified by reading:**
  - `$m` head (roles incomplete)
  - command or variable substitution in any member word (Dynamic)
  - `{*}`, by the word-count check `words.len() != count+1` and by `expand_word`
  - an extra operand: `required != count` gives `None`, so the roles are incomplete
  - `forward`, `export`, `class`, `self`, `private`, and so on: `native_deferred_method_setter` gives `None` in the shared transfer
  - the 9.0 `-export` form: arity mismatch in the transfer
  - a replaced worker: the shared `original_definition_member_target`
  - a prefix abbreviation such as `meth`: `DefinitionBodyGrammar::member` is an exact `keyword ==` match (`definer.rs:3191`)
  - class targets (`oo::objdefine C …`): `original_named_instance_for_operand` requires `BindingKind::Command`, and a class gets no `manufacture_object_proof`
  - a non-UTF-8 origin, a slice mismatch (`ExecutedScriptSource::contiguous` checks byte identity), and a partial, multi-command or unequal re-parse
- **A refusal mutates nothing.** Every refusal returns `None` before `install_own_object_method_transfers`, and everything before that point is read-only.
- **Equivalence with tclsh.** In 8.6 and 9.0, Tcl's inline path (`MagicDefinitionInvoke`) invokes the namespace-resolved worker with the already-substituted argv. The script path evaluates the same text in `::oo::objdefine`.
  - **Why they agree.** Every member word is a source literal, and the re-parse of the exact source slice must reproduce `WordExpr`-identical words (same text, spans and provenance; `Span` is offset-only, so the `with_base(start,0,0)` line/column base is irrelevant). Evaluating that slice as a script therefore yields exactly the inline argv. Escapes such as `p\uD800` decode once in both forms.
  - **Probes.** My probe `probes/row.tcl` agrees with Sol's `probe2.tcl`.
- **Worker resolution.** Inline resolution is namespace-only (prefix match inside `::oo::objdefine`). That is narrower than script resolution, so the shared worker-currency check, which proves the original `::oo::objdefine::method`, covers both forms.
- **Declaration join** (`receiver_identity.rs:158-185`):
  - **Compiler side.** `entry.declaration().offset` is the inner segment start, which equals the slice start, which equals `members[0].original_written_span().start()`.
  - **Analyser inline anchor** (`original_members.rs:948-951`): `head.original_word().group().span.start()` of the same word. For a bare `method` the two are equal.
  - **Name spans.** `name_source().span` is the inner `WordExpr` span, which the equality check makes equal to the outer one. The analyser side is `arg_tokens[name].span`. That is the same token/`WordExpr` relationship the script form already relies on.
  - **Coverage of the new test.** It asserts `name.start()==member+7`, that `name.end()-member` is equal across the two forms, and that the declaration offset equals the `method` word.
  - **Not covered.** Braced or quoted keyword words (`{method}`, `"method"`) are not tested. Any mismatch there only makes the LSP join fail closed, exactly as it would for the script form.
- **No new Native/source identity grant.** The configuration and transfers use the same struct and the same `install_own_method_configuration`. `configuration.words` holds the full inline argv, and only `Hash`/`Eq` consume it. `body` is a contiguous `ExecutedScriptSource` over authored bytes.
  - **Residual note.** The inline "definition script" is a synthetic parse carrier that Tcl never forms as a value. Its only consumers are the shared transfer parse and `register_original_method_bodies`; the method body inside it is a real stored value. I found no consumer that treats the configuration body as executed-script evidence.
- **Registry rule.** Keyword membership comes from registry data only (`grammar.member`), and the layout comes from the registry resolver. No command name appears in non-test code.

## 2. Assertion preservation (critical)

**Changed row** (`object_instance.rs:994-997`, test `command_binding::object_instance::tests::object_read_requires_its_allocation_and_current_dispatch`):

- **The fixture.** Under `tcl9.0` with `NativeCompilationMode::Direct`, the source is `oo::class create C {method valid {} {}}; oo::class create B {}; set object [C new]; oo::objdefine $object method valid {} {return CUSTOM}; $object valid`.
- **What the row asserts.** It asserts whether the variable read `$object` at the final `$object valid` carries `proved_object_instance()`.
  - **HEAD: `false`.** This is the opaque fallback.
  - **Patch: `true`.** The test then also requires class `::C`, a closed inventory that contains `valid` and not `bogus`, and allocation `First` at `C new`.
- **What the patch adds.** A sibling script-form row `oo::objdefine $object {method valid {} {return CUSTOM}}` → `true`.

**Origin. It is not an original assertion:**
- **History.** The repo is shallow; the graft is at `4c717119b` (2026-10-16 15:54Z, a squash). `git log -S` and `git log -L` for the row and the test both stop at the graft, so git cannot date the row.
- **Absent upstream.** The upstream default branch `rust@fcb7927f0` (2026-10-16T15:50Z) has no `rust/tcl-compiler/src/command_binding/` directory. Its 7,288-path tree has no `object_instance.rs`, and its `command_binding.rs` has neither the test, `proved_object_instance`, `AllocationIncarnation`, nor any `objdefine`/`CUSTOM` fixture. The test belongs to this naming migration's `SourceObjectInstanceProof` work.
- **Absent from the selections.** The selector is in neither `original1024.json` (162 compiler selectors) nor `additive314.json` (27). Its name appears only in full binary listings: `experiments/.../test-list.txt` (2026-10-13/14) and `rust-validation/frozen144…347` listings.
- **No proof binding.** The test has no `naming.` comment, and no `manifest.json` record binds it.
- **Conclusion.** No user decision is required under the preserve-original-assertions rule. Because of the shallow history I cannot say on which migration day the exact row was written; I can only say it is migration-introduced.

**Semantics.** The new `true` is correct TclOO. My probe `probes/row-8.6.out` and `probes/row-9.0.out` shows the same results on 8.6 and 9.0, for both the inline and the script form:
- same object
- `info object class` = `::C`
- `$object valid` → `CUSTOM`
- `valid` in `info object methods -all -private`
- `bogus` errors

The `class B` and `set object TEXT` rows stay `false`, which is also correct. The mechanism is the same as the script form: `install_own_method_configuration` restamps store proofs (`object_instance.rs:381-442`), with no dispatch invalidation, so `object_read_proof` → `receiver_allocation_is_current` holds. The existing script-form test `original_own_object_readonly_target_retains_no_generated_name` exercises the same `$object` and `[C new]` route.

**Can the producer be admitted without this hunk? No.**
- **Producer without the hunk.** The producer makes exactly that inline read proved, so the old `false` row would fail. The row is not in any selection, so a selection capture would not catch it; it would be a silent compiler-libtest regression.
- **Hunk without the producer.** The new `true` inline row fails.
- **Therefore** admit them together.
- **Run condition** (agreeing with Sol):
  - **Script row `true`, inline row `false`:** the producer did not fire, most likely because the `WordExpr` equality check failed. Hold both.
  - **Script row `false`:** revert both rows to `false`.

## 3. Compile, format, lint and docs

- **Signatures.** All signatures match HEAD exactly:
  - `effective_invocation_word(&WordExpr, EscapeSyntax, WordValueRules)`
  - `WordValueRules::from_config(&LexerConfig)` (Copy)
  - `EffectiveInvocationWord::{literal_bytes, as_registry_word}`
  - `InvocationWord::literal(self)`
  - `WordExpr::{source, original_written_span(impl AsRef<[u8]>)}`
  - `SegmentedCommand::{execution_span(&str), expand_word, is_partial}`
  - `SourceOriginId::try_text`
  - `ExecutedScriptSource::contiguous(Arc<SourceOriginId>, &str, u32)` and `pub text`
  - `SourceMap::from_image(...).with_base(u32,u32,u32)`
  - `segment_commands_image_with_offset_and_config(&SourceImage, u32, LexerConfig)`
  - `CommandTokens::from_segmented`, `words()`
  - `LexerConfig` is Copy, `WordExpr` is `PartialEq`, and `SourceNativeInvocation` is Copy
  - `&mut ModuleCommandBindings` coerces to `&`
- **Borrow.** `definition` moves after the last use of `map`. `SourceMap` has no `Drop` impl.
- **Test accessors.** `formals`, `body().text.bytes()`, `receiver` (`SourceMethodReceiver: Debug+PartialEq`), `is_exported`, `declaration().offset` and `name_source().span` all exist. `Vec<u8> == &[u8;N]` comparisons compile.
- **rustfmt.** Both touched `.rs` files are fmt-clean (edition 2024).
- **No `#[allow]`.**
- **C1 (blocking lint).** `clippy::too_many_lines`: pedantic is `warn` workspace-wide, and `make rust-check` passes `-D warnings`. The default threshold is 100.
  - I simulated clippy's counter, which counts non-blank, non-comment-only lines of the body block: `walk_closed_own_object_method_configuration` goes from **96 at HEAD to 102 with the patch**. That is an error, and an `#[allow]` is forbidden.
  - **Fix.** Move layout and body selection into a helper, for example `own_object_definition(native, facts, grammar, state, config) -> Option<ExecutedScriptSource>` holding the `script_layout` test, the `retained_script_operand` call and the `arg_roles_complete` → `inline_member_definition` branch. Keep the cheap early guard. This saves about 9 counted lines with no semantic change.
  - The new helper (43 lines) and the new test (65) are under the limit. Clippy was otherwise not run.
- **Proof docs:**
  - `manifest.json` stays in canonical `json.dumps(indent=2, ensure_ascii=False)` form.
  - The new binding's selector is unique within the record.
  - The test body carries both the ID and the document path, as `check-name-resolution-proofs.py:test_annotation_errors` and the per-binding comment check require.
  - Neither touched `.rs` file is pinned as evidence or as a source anchor in any record.
  - The scope sentence occurs only in these two files.
  - **Render drift.** `update-name-resolution-proof-index.py:render()` differs from the `.md` by the same pre-existing 28-line drift at HEAD and with the patch (an extra evidence line and the appended "Exact original-source Compiler software outcomes" section); only the line offset moves. The patch introduces no new drift.
  - The README question index is unaffected, because the question is unchanged.

## 4. Expected flips and pass→fail risk (not executed)

**Expected fail → pass:**
- **In `original1024`, all FAIL in the retained capture list:**
  - `original_declaration::own_object_identity_tests::original_own_object_identity_keeps_uri_currency_and_wire_reselection`
  - `original_call_hierarchy::own_object_call_tests::original_own_object_calls_and_references_keep_the_canonical_object_worker`
  - `code_lens::native_lens_presentation_tests::native_own_object_method_lens_keeps_identity_when_class_display_key_is_unavailable`. NOTES says this one "should stay green", but it is currently in the fail list.
- **Probably:** `method_symbol::…::original_own_object_generation_and_private_entries_block_class_advice`, which is not in a selection.
- **New compiler test:** `original_own_object_inline_member_installs_the_script_layout_entry`.

**Conditional:** `object_instance::…::object_read_requires_its_allocation_and_current_dispatch`; see §2.

**Unaffected by reading:**
- `object_types::physical_type_receipts_do_not_close_object_method_dispatch`: `object_dispatch_unknown()` is a constant `true`.
- `checks.rs::method_unknown_and_objdefine_suppress`: `custom` is in the closed inventory, so there is still no W308.
- `checks.rs::structural_bodies_do_not_leak_into_outer_scope`: class target.
- `semantic_tokens` (`$o` unset; inline `self`).
- All class-target fixtures: `deferred_method.rs` 2578/2637/2842/2846, `source_class.rs` 1250–1879, `fp/obj.rs`, `var_command.rs`, `references`/`receiver_identity` `C ping [oo::objdefine C …]`, `project_class_cards`, `type_infer`.
- `class B` rows: the `class` member has no setter, so the path is unchanged.
- Registry `.tcl` fixtures, which are not walker inputs.

**Residual pass→fail risk:**
- **(a)** If `WordExpr` equality fails at runtime, nothing is installed. The three LSP targets then stay red and the inline `object_instance` row fails (see the run condition in §2).
- **(b)** For an inline form that now installs, the early return skips the generic `transfer_native_invocation` and `walk_selected_native_body` for the method-body operand, exactly as the script form already does. Any untested consumer that relied on the generic inline path (opaque dispatch) changes to the precise own table. I found no test that asserts the old opaque behaviour on an instance target other than the row this patch changes.

## Required for v2

1. Fix C1: extract a body-selection helper so `walk_closed_own_object_method_configuration` is at most 100 clippy lines. Add no `#[allow]`.
2. Optional: correct the NOTES wording about `arg_roles_complete` and the code_lens test status.
3. Keep the `object_instance.rs` hunk paired with the producer. Admit both only if the script and inline rows both run `true`.
