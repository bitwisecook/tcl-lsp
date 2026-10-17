# Sol-6: member reference census V4

**Patch:** `member-census-v4.patch`
- It touches the same 9 files as V3.
- `git -C /home/user/tcl-lsp apply --check` passes at HEAD `89cfb925d`. The `a4/` copies are byte-identical to that HEAD.
- Every changed `.rs` file is `rustfmt --edition 2024` clean (checked on stdin). The patch adds no `#[allow]`.

**Static only.** Nothing was compiled or run.

Background: V2 is described in `NOTES.md` and the V3 additions in `NOTES-v3.md`. This file lists only the V4 deltas, which answer Luna-8 `REVIEW-v3.md` F1–F3. Everything else is unchanged from V3.

## F3: a code-lens inventory error degrades the tag, not the request
- **Where:** `rust/tcl-lsp-server/src/lib.rs`, `code_lens` (around line 33027).
- **Before:** the census-document count used `original_declaration_documents(&uri).await?`. A read or analysis error would fail the whole `textDocument/codeLens` request.
- **Now:** the count is `.await.map_or(0, |documents| documents.len())`.
- An error gives a count of 0. `single_document_coverage` downgrades any count other than 1, so the tag becomes `"partial"`.
- This matches the resolved path, which returns `None` (no census) when the inventory is unavailable.
- The call still happens only when some lens claims `CompleteSourceOccurrenceCensus`.

## F1: wording for the `implementation_namespace` rule
The bare `dict` descriptor sets `implementation_namespace: Some("::tcl::dict")` (`dict.rs:891`). So the interim rule keeps the census Partial for every `dict` call in the document, not only for qualified `tcl::dict::*` calls. NOTES-v3 understated this.

The wording is corrected in three places:
- **Code comment** (`MemberCensus::classify`): the rule now names the qualified `::oo::Helpers::next` and `::oo::Helpers::link` spellings, and notes that every `dict` call is excluded because the bare descriptor declares `::tcl::dict`.
- **Census doc comment** (`complete_member_source_census`): "no descriptor declaring an implementation namespace (qualified `oo::Helpers` spellings and every `dict` call)".
- **Contract** (`original-method-candidates.md:18`): "does not select a descriptor that declares an implementation namespace (the qualified `oo::Helpers` spellings and every `dict` call)".

The rule is a loss of usefulness, not a soundness problem. The target tests (056–060, test-096, the server wire test) use no `dict`.

It stays until the planned registry change lands:
- keep `TCLOO_NEXT_CHAIN` and `TCLOO_BINDS_METHOD_ALIAS` on the qualified `oo::Helpers` twins, after auditing the consumers of those traits;
- then drop the `implementation_namespace` rule.

## F2: class-family control (decision: keep snit and assert; drop itcl)
The test `readonly_member_census_stays_partial_for_other_class_families` now:
- checks only the snit source `snit::type C {method ping {} {}; method body {} {$self ping}}`, under tcl8.5 and tcl8.6;
- builds identities per method with `OriginalDeclarationIdentity::for_method`, iterating `original_class_declarations()` and each class's `original_members.declarations()`. It no longer uses the all-or-nothing `declarations()`;
- asserts that no checked identity reports `CompleteSourceOccurrenceCensus`;
- asserts that **at least one identity was checked** per dialect.

**Why snit stays.** There is in-repo evidence that `snit::type` publishes original class declarations with instance methods, without `package require`:
- `tcl-compiler/src/analyser/original_members.rs` `original_generic_definer_members_use_source_value_purpose`, at tcl8.5;
- `original_static_generic_class_names_retain_source_bytes_without_entered_handler`, at tcl8.5.

**Why itcl is dropped.** No test or code path read here shows that `itcl::class` publishes `original_class_declarations` or method metadata. A control that may check zero identities proves nothing. The test comment says so.

**Remaining risk.** If `for_method` declines snit's source-value method names, the new `checked > 0` assertion fails. That would be a visible signal that the family is not covered, not a soundness gap.
