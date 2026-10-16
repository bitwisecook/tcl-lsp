# Test-only Server wrapper compilation scope

The actual browser compile log `/workspace/naming-validation/lsp-browser-current-successor/build.log` passes but warns that six private methods are unused. A complete textual reference census of `rust/tcl-lsp-server/src` shows these are not Native-only functions: no production target uses the old wrapper declarations. Native library tests use them, which is why a `cargo test --lib --no-run` build includes their callers.

| Existing private method | Complete actual caller category | Production counterpart retained unchanged |
| --- | --- | --- |
| publish_disk_results | Root test module, test-only path admission module, and merge_workspace_scan_results wrapper | publish_scanned_disk_results |
| publish_disk_results_while_rehoming_locked | publish_disk_results wrapper only | publish_admitted_disk_results_while_rehoming_locked |
| original_declaration_reference_locations | Root test module and readonly_reference_reports::tests | Actual readonly_original_declaration_reference_report and independent original call hierarchy/source APIs |
| batch_reindex_from_disk | Root test modules and cfg(test) document metadata/path admission modules | batch_reindex_from_disk_using with actual retained watched-file event admissions |
| reindex_index_from_disk | Root test modules and cfg(test) document metadata/path admission modules | reindex_index_from_disk_using with actual original close/rename admission |
| merge_workspace_scan_results | Root test module only | Production workspace scan uses current_scanned_rows plus publish_scanned_disk_results |

`lib.rs` declares the external document metadata/path admission modules under `#[cfg(test)]`; the root tests begin under the same predicate. `readonly_reference_reports.rs` encloses its only legacy-strong-helper call under its `#[cfg(test)] mod tests`. All six existing declaration bodies and all callers/assertions remain byte-identical. Adding `#[cfg(test)]` keeps them present for tests on every target, while excluding them from production Native, browser and WASI builds equally. There is no target-family or operating-system assumption, no allow attribute and no fallback or semantic change.

Three production rustdoc links previously named a now test-only wrapper; they now name the actual unchanged admitted producer or retained-admission single-file entry used by that production path. Links inside test-only wrapper documentation remain valid in the same test scope.

Validation is source-only: complete source reference/cfg ancestry and real live counterpart calls inspected; manifest binds actual source files and original warning log. Inverting exactly the six cfg attributes and three link changes restores the entire original lib.rs byte-for-byte. Separate TMP `patch --fuzz=0` apply matches the declared afterimage hash. No main/Git/Cargo/tests or target builds executed. Root owns fresh Native/browser/WASI compiler validation; this proposal makes no fresh warning-free claim and no requirement-count advance.
