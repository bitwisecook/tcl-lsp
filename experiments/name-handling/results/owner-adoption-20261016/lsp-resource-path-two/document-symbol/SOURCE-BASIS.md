# Document-symbol resource configuration caller

Exact patch SHA256: d1dc75a66097bc77abba750b9551fed170f11227fc9e8aeecc66e144765ba1d1
Existing baseline unit: lsp-server-document-symbol-resource-config.
One actual caller migration; no new semantic/owner/helper count.

The production change is exactly db_document_symbols' AnalyserConfig selection: self.resolved_db_config(uri).await replaces the process-global db_config handle. The existing owner chooses the genuine longest matching folder handle and otherwise the actual global handle. URI-selected SourceFile, Salsa snapshot creation, same document_symbols query, worker, Cancelled::catch, join handling and projection outside the read remain unchanged. No source recapture, dialect-label config construction, Native admission, arbitrary pack fallback or new transaction is introduced. This follows the already existing db_folding_ranges/file-analysis consumer pattern.

The pack set remains a workspace fact; this patch does not claim to install independent per-folder packs. Each selected folder configuration retains its actual complete analysis input and pack key. A missing generation stays unavailable in the existing DB/Core providers, rather than borrowing an available sibling/global input.

New fixed control original_document_metadata_tests::document_symbols_keep_the_resource_input_and_withdraw_unavailable_overlays uses actual Backend state, real installed workspace pack, apply_folder_configs with a genuine NonAsciiMode override, two distinct URI-selected SourceFiles, and the real db_document_symbols worker. It checks the retained input's Registry semantic key; makes global generation unavailable while the folder's remains valid; requires a real function outline only for the folder; edits that same SourceFile and requires only the new header; inverts availability and requires the folder to decline while the sibling global outline survives; restores the folder input; then uses real db_remove_source and checks missing-file None. All snapshots end before writes/worker awaits. No raw labels or maps supply input identity.

No existing assertions, original tests or budgets were changed. All Native/source input checks remain in the unchanged producers. The public document_symbol fallback/path-admission task and its analysis_for .await? hunk are exclusively owned by sol_runtime_next; this patch touches neither and is composed from isolated db_document_symbols and test insertion hunks.

Validation: isolated TMP rustfmt --check PASS; immutable patch applied to a separate TMP beforeimage copy PASS; all resulting hashes equal manifest afterimages; both full main files matched retained beforeimages at packaging. No Cargo/compiler/test binary/main/Git/network operation. Root must independently review and execute the new exact selector and existing document-symbol smoke/outline regressions before acceptance.
