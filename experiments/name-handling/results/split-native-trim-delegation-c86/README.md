# C86 trim delegates to the central decoder

Only rust/tcl-syntax/src/native_string_trim.rs changed: its existing C86 unit branch now calls NativeTclUtf::decode_c86_ucs4; every other release still calls decode_unit(bytes,None).
The before/after bodies and exact whole-file hashes are retained in preparation.json/source-ready.json. No original seven split/source-producer owners or fixtures changed during this window.
The existing central method retains the same measured C86 surrogate-combination purpose; no global UTF8 relaxation or second native decoder was added.

Existing exact runtime interp::native_body_artifact::native_string_trim::tests::original_string_trim_matches_all_216_native_header_windows PASSED: one test, all216 direct/compiled header/result/identity windows,74.411s.
Existing exact VM exec::native_string_trim_tests::original_string_trim_matches_all_216_native_header_windows PASSED: one test, all216 original windows,144.810s.
Each retained fixture spans C84/C85/C86/C90/C91/Jim, six subjects and three trim directions; actual constructor/header/refcounts and no-cut identity are checked.
These fixtures have no surrogate-pair subjects. Central pair-purpose evidence remains the separately accepted original18 C86 split rows; no new trim-pair oracle or target parity is claimed.
No profile, input, fixture or assertion was substituted; no wider tests or all.tcl rerun was performed.

validation.json retains exact offline-j2 argv, target caches, actual test binary hashes, Cargo artifact package IDs, metadata scope and raw output hashes.
The runtime used tmp/name-target-runtime; the VM used tmp/name-target-issues sequentially. Both caches/source freeze were released after controls completed.
Initial broad root cargo metadata --offline failed because addr2line0.25.1 is uncached; that setup failure remains vm-metadata-failed01.*. No HTTP request was made.
Local --no-deps metadata then succeeded; actual VM Cargo target built and passed offline. Full workspace metadata acceptance is not claimed.
Compiled local-package inventories and the conservative18,051 coordinator-source/fixture inventory changed0 during the controls. Cached Cargo artifacts are included; external Cargo source currency is not asserted.
manifest-currency.json compares actual current root/runtime manifests/locks with the retained earlier receipt, allunchanged; it is explicitly not an invented immediate-before metadata snapshot.
Source receipts refer to the exact pre-deferred-formatting inputs. Parent-owned later runtime2021 formatting receives separate source-currency labels and does not alter these saved observations.
The separate GetDateFields question is sealed: selected C86 proposal only, other four workers/environment synchronisation/localtime capability remain open; no runtime worker implementation added.
