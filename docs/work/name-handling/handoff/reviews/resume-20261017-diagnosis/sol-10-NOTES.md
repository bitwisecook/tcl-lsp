# Sol-10: inline `oo::objdefine obj member …` own-object producer (Sol-8 issue 2)

I worked statically only. I did not run cargo, any test binary or gdb, and I did not modify `/home/user/tcl-lsp`. The only tools I ran were the following, all on scratch copies:
- `nice tclsh8.6` / `nice tclsh9.0` probes (`probe.tcl`, `probe2.tcl`, `probe2-86.tcl`)
- `nice -n 19 rustfmt --edition 2024 --emit stdout` on stdin
- `python3 -I` for text edits and a JSON validity check

Line numbers refer to HEAD `bb7277d82`, which contains integration37 candidate `60051c3c2`.

## Deliverable
- **Patch:** `inline-objdefine-producer-v1.patch` (`diff -ruN a b`). `git apply --check` passes against `/home/user/tcl-lsp`.
- **rustfmt:** both touched `.rs` files are unchanged by rustfmt (edition 2024).
- **No `#[allow]`** anywhere in the patch.
- **Not run:** clippy.
- **Files:**
  - `rust/tcl-compiler/src/command_binding/own_object_configuration.rs`: the producer change and a new fixed-input test.
  - `rust/tcl-compiler/src/command_binding/object_instance.rs`: one fixture row flips, and a script-form row is added beside it.
  - `docs/design/analysis/name-resolution-proofs/manifest.json` and `tcloo-original-own-object-method-transfer.md`: the contract scope sentence and the new test binding. These are needed because `scripts/dev/check-name-resolution-proofs.py` requires every test carrying a `naming.` comment to have a catalogue binding.

## Design
`walk_closed_own_object_method_configuration` now selects one of two layouts. Everything after body selection is unchanged and shared: `own_object_method_transfers`, `install_own_object_method_transfers` and `register_original_method_bodies`.

- **Script layout (unchanged):** `exact_argv_len == Some(2)` and `facts.arg_roles` contains `(1, Body)`. The body comes from `retained_script_operand(1, …)`.
- **Inline member layout (new):** only when `facts.arg_roles_complete` holds, meaning the registry resolver ran on all-literal argv. The new `inline_member_definition(native, selected_grammar, state, config)`:
  1. **Word shape.** It requires an exact argv, `native.words.len() == argc + 1`, no `{*}` word (`segment.expand_word`), and an empty `prepended`. The caller already checks `prepended`.
  2. **Static literals.** Every member word (`native.words[2..]`) must have `Provenance::Source`, and its `effective_invocation_word` must be `Literal` or `ByteLiteral`. Escapes such as `p\uD800` are allowed, exactly as inside a braced script body. Variable, command or template substitution, `Opaque` and `Expanded` words are refused.
  3. **Member keyword.** The first member word must be a member of the selected descriptor's `definition_body` grammar (`grammar.member`). This is registry data, not a command name.
  4. **Contiguous slice.** The slice runs from the first member word's `original_written_span` start to `segment.execution_span(source).end()`. Using `execution_span` means the closing delimiter, including a quoted last word, follows the lexer's own convention (#527), not `end + 1`. The slice is retained as `ExecutedScriptSource::contiguous(current_source_origin, slice, start)`, which checks byte identity against the origin.
  5. **Re-parse proof.** The slice is re-segmented exactly as `own_object_method_transfers` will segment it (`segment_commands_image_with_offset_and_config(text, start, config.nested())` plus `CommandTokens::from_segmented(map.with_base(start), config, …)`). It must give one non-partial command whose `WordExpr` vector equals `native.words[2..]`: same text, same spans, same provenance. The transfer therefore reads the same argv the native handler received, and every downstream span is an authored-document span.
- **Why this matches the analyser join** (`receiver_identity.rs:158-185`):
  - **Compiler side:**
    - `entry.declaration()` is `{definition.origin, inner segment.span.start()}`. That is the first member word's written start, because the slice begins exactly there. This comes from `deferred_method_body`, `deferred_method.rs:2201`.
    - `name_source().span` is the inner `WordExpr` span, which the equality check makes equal to the outer word's span.
  - **Analyser side, inline** (`original_configuration_delta`, `original_members.rs:368-380`):
    - The declaration site is `head.original_word().group().span.start()` of the same `method` word (`original_members.rs:948-951`).
    - `name_span` is that word's argv token span (`:1366`).
    - The side is `ClassObject`.
  - **Same relationship as the script form.** Both correspondences, argv token versus `WordExpr` and group start versus first-word start, are the same ones the script form already relies on. Only the nesting level differs.
- **Downstream checks are unchanged.** `original_member_input` and `formal_topology::capture` re-project with `original_native_compiler_words(origin, words, segment.span.start())`. That function requires `first written span start == offset` (`native_compilation_source.rs:214`), which holds by construction. It then parses the region up to the last written span; for the inline form, that region ends at the outer command's terminator.
- **Registry rule.** Nothing is hard-coded:
  - The layout comes from `facts.arg_roles` / `arg_roles_complete`.
  - The member comes from the `definition_body` grammar.
  - The target comes from the `ObjectDispatch::Configure{Object}` transition.
  - What gets installed comes from `native_deferred_method_setter` (exact three-operand `method` only).

## Refusal cases (install nothing; fall through to the existing generic or opaque path)
- **Roles incomplete:** any non-literal argument makes `arg_roles_complete` false.
- **Inline-specific refusals:**
  - an expansion word, or an argv/word count mismatch
  - any member word that is substituted, opaque, derived or expanded
  - a first member word that is not an exact member of the selected grammar. This includes Tcl's prefix abbreviation `meth`, which tclsh accepts for both forms; refusing it is conservative.
  - origin text that is not UTF-8, a slice that is not byte-identical, or a re-parse that differs or is partial
- **Inherited from the shared transfer, identical to the script form:**
  - any member that is not a deferred method setter: `forward`, `export`, `unexport`, `deletemethod`, `renamemethod`, `filter`, `mixin`, `variable`, `class`, `self`, `private`
  - the wrong operand count: `method x {} {} extra`, or 9.0's `method x -export {} {}`
  - an invalid name or formals
  - a replaced, renamed or tainted worker (`oo::objdefine::method`)
  - a non-current receiver
  - a target that is not an instance; for example `oo::objdefine C …` on a class has no Command-kind instance proof, so the `deferred_method.rs:2578/2637/2846` and `source_class.rs:1635/1640/1878` fixtures are untouched
  - opaque domains or observed execution

## tclsh evidence (8.6 and 9.0, `probe2.tcl`)
- **Same effect in both forms.** Inline and script produce the same effect for `method` (including `p\uD800`, braced `{method}` and quoted `"method"`), `forward`, `unexport`, `export`, `renamemethod`, `deletemethod`, `filter` and `mixin`. The table name and the call result agree in each case.
- **Wrong arity.** Inline with extra or missing arguments errors on both versions and installs nothing.
- **Accepted in Tcl, refused by us:** `{*}{method …}` and `$m` (with `m` = `method`) are accepted by Tcl, and the producer refuses both (conservative). `meth` (an abbreviation) is accepted by Tcl in both forms, and the grammar refuses it in both forms, consistently.
- **Only `method` is installed.** The producer installs only the three-operand `method` setter, which is what the script form installs.

## Expected test flips (not executed)
- **Should go green:**
  - `tcl-lsp-core original_declaration::own_object_identity_tests::original_own_object_identity_keeps_uri_currency_and_wire_reselection` (`:2045` reference count 0 → 1)
  - `tcl-lsp-core original_call_hierarchy::own_object_call_tests::original_own_object_calls_and_references_keep_the_canonical_object_worker` (Sol's G9)
  - probably `tcl-lsp-core method_symbol::own_object_tests::original_own_object_generation_and_private_entries_block_class_advice`: "replacement own worker" now has the inline generation-2 entry.
- **Should stay green, probably helped:** `code_lens::native_lens_presentation_tests::native_own_object_method_lens_keeps_identity_when_class_display_key_is_unavailable`.
- **New test:** `command_binding::own_object_configuration::tests::original_own_object_inline_member_installs_the_script_layout_entry`.
  - **Equivalence half:** for 8.6, 9.0 and 9.1, the script and inline forms give equal entry projections: name bytes, receiver, exported flag, formals, body bytes and name-span length. The declaration offset equals the `method` word in each source.
  - **Refusal half:** `$m`, `[list p]`, `"p$m"`, `{*}`, extra operand, `forward`, and a replaced worker each yield no own entry.
- **Deliberately changed:** `command_binding::object_instance::tests::object_read_requires_its_allocation_and_current_dispatch`.
  - **Inline row:** `oo::objdefine $object method valid {…}` goes from `false` to `true`.
  - **New script-form row:** `oo::objdefine $object {method valid {…}}` added with `true`.
  - **Reasoning:** a represented own-method install restamps the store proof (`object_instance.rs:409-442`) and does not invalidate dispatch. The existing script-form test `original_own_object_readonly_target_retains_no_generated_name` relies on exactly this for `$object own…`. The old `false` was the opaque fallback.
  - **If the run disagrees:** if the script row turns out `false`, the store-restamp reasoning is wrong. Revert both rows to `false`; the equivalence is the point.
- **Unaffected (checked by reading):**
  - `object_types::physical_type_receipts_do_not_close_object_method_dispatch`: `object_dispatch_unknown()` is a constant `true`.
  - `semantic_tokens` qualified-head test: `$o` is unset, so there is no receiver.
  - `checks.rs` `method_unknown_and_objdefine_suppress`: W308 is still suppressed, now also by a real own entry.
  - the class-target fixtures listed above.

## Risks / unverified
1. **Equality of `native.words` and re-parsed words.** This is the main runtime uncertainty. The outer words come from the walker's map over the origin image; the inner words come from a `NativeValue` image with `with_base(start)`. Both use the same lexer on the same bytes and are shifted to absolute offsets, so `WordExpr` equality should hold. If some walker path builds outer words differently (other map or channel), the check refuses and the target tests stay red. That is safe, but the fix would not take effect. A quick probe if the target tests stay red: `TCL_LSP_TRACE_ORIGINAL_CHILD_GUARDS=1` and a `dbg!` of the two vectors.
2. **`name_span` join for `p\uD800`.** It relies on the argv-token-span versus `WordExpr`-span relationship being identical at both nesting levels, as it already is for the script form. Not executed.
3. **Producer cost.** Inline objdefine forms with complete roles now go past the early guard into the receiver and grammar lookups (all read-only) before refusing. For non-objdefine definitions, `own_object_configuration_target` still returns early.
4. **Clippy pedantic not run.** I avoided `!(a || b)` (`layout_selected`) and used no `#[allow]`.
5. **Docs edited by hand.** `manifest.json` (still valid JSON) and the rendered `.md` were edited consistently: scope text, plus one test binding inserted after `…replacement_retires_prior_captured_prefix`. Run `scripts/dev/check-name-resolution-proofs.py`; if a render-drift check exists, re-render through `scripts/dev/update-name-resolution-proof-index.py`. The `## Conclusion` text is unchanged and still accurate.
6. **Analyser and compiler refusal mismatch.** Where the compiler refuses but the analyser accepts, for example the `$m` head handled by `handlers.rs:6352`, the analyser still records declarations, and LSP finds no realm entry. This is the same as today's behaviour for every inline form.

## Files
- `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-10/inline-objdefine-producer-v1.patch`
- `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-10/a/`, `.../b/`: pre- and post-patch copies
- `/tmp/claude-0/-home-user-tcl-lsp/8d747420-e446-5e83-b7e6-dff30ae9952e/scratchpad/agents/sol-10/probe2.tcl`, `probe2-86.tcl`: tclsh semantics probes
