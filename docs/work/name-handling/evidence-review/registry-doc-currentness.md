# Registry document currentness review

2026-10-12, read-only review of exactly the redesign and centralisation Markdown
documents. No durable docs, indexes, sources, manifests or issues were changed.
Semantic inspection used only three owner sources: `tcl-registry/src/model/assembly.rs`,
`tcl-spectcl/src/loader/available.rs`, `tcl-dialect/src/model/environment.rs`.
Paths below are repository-relative. No execution or gate result is claimed.

1. **Keep both documents; condense the redesign rather than delete it.**
   Redesign §§0–6 separate dialect, provider, environment and positioned binding;
   centralisation §§1–5 map those concepts to current consumers and authority.
   `assembly.rs:111–158` retains context and `Arc<CommandRegistry>`, assembles
   provider-filtered entries and answers from selected store/context facts.
   `environment.rs:433–470` retains environment identity, core selector, targets,
   expected packages, policy, detection and provenance. These are useful contracts.
   Inbound references make file deletion inappropriate: `docs/design/README.md:308,312`,
   `docs/GLOSSARY.md:136`, `docs/design/contracts/environment-selection.md:6`,
   `docs/design/f5/sslictcl-vocabulary.md:21` (classification anchor), plus
   `rust/tcl-dialect/src/model/version_set.rs:20` and model-module API docs.

2. **Correct/condense redesign generation debt at :879–881, :938, :974.**
   “Registry generations” as a wholly absent feature conflates two owners.
   `assembly.rs:22–43,111–114,476–512,515–530` already provides Arc-owned
   ContextRegistry generations, identity-keyed caching and bounded overlay
   pruning. However `available.rs:343–345` still leaks availability-window
   slices. Replace blanket absence with the implemented generation contract and
   separately scoped dynamic-allocation reclamation debt. Do not mark I7 passed
   or remove loader-lifetime debt: no allocator/reload acceptance was executed.

3. **Remove superseded process references, retaining substantive limitations.**
   Redesign :8–9 advertises the companion's “retirement ledger's open rows”;
   :423 and :661–662 refer to its C12/C10; :978 refers to companion §4 gap rulings;
   :998 points to §§7.2–7.5. Current companion §4 (:181–194) is relations/security,
   §7 has only 7.1–7.4, and no retirement ledger remains. Replace these with
   exact current contract/evidence destinations after root reviews each subject.
   Condense :949–953 by removing only the ruled-out implementation-plan list;
   retain current family/range rules and the conformance corpus's F5-divergence
   provenance reference in their contract/evidence sections.

4. **Condense duplicated centralisation resolution tables at :48–57 and :65–70.**
   Keep the first, fuller table; merge the second's unique `knowledge_at`,
   `command_existence_oracle` and `resolve_spec` owner notes into matching rows.
   Keep :59–62's positioned authority/refusal statement. The current owner
   distinction is supported by `assembly.rs::with_command_store` (:130–136):
   retaining context/store does not reconstruct or establish native identity.
   This is contract consolidation, not removal of uncertainty guidance.

5. **Keep verified limitations and evidence/procedure sections.**
   Redesign :977's package-window limitation is current: `available.rs::package_row`
   (:349–377) validates range, reports it cannot carry the window, and retains
   package name/surface only. Keep centralisation §6's source upgrade contract,
   real TODO loss markers and verification procedure, and §7's interpreter/
   fixture/native-library/provenance boundaries. Their terms describe actual
   user operations and evidence semantics, not completed development phases.
   Preserve redesign reference/F5 observations and legal/source provenance.

Other redesign §11 claims (D5/D7/D11/D18, O/V/E rows) remain **unverified** in this
bounded review. Do not delete their factual limitations or infer completion from
renamed APIs. Recast programme/decision narration only after owner-specific review;
keep genuine unsupported capability explicit. Root owns eventual cleanup and links.

Applied: four bounded correction groups in the same two docs; no headings changed.
Generation debt now separates Arc context caches from leaked loader windows;
obsolete pointers replaced, duplicate tables merged, ruled-out plan list condensed.
Five new local link targets/anchors verified; unverified debt preserved except pointers.
