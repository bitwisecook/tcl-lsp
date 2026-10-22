Actual heavy comparison on 2026-10-22T21:18:22.535645+00:00: current349 tests (298PASS/51FAIL) versus actual baseline312 namedPASS. Exact binary+name join confirms27PASS→FAIL;24 failures absent baseline;0unknown. Current also has284matchedPASS→PASS and14absent-baselinePASS. Baseline-only one test is tcl-irule-test::embedded::tests::itest_core_when_loader_contract; its omission supplies no current result. All ordinal ranges/denominators are complete;51 repeated final failure rows were excluded. This is a separate HEAVY ledger, not part of main24312.

| Primary route | Current failures | Confirmed P2F | Absent baseline |
| --- | ---: | ---: | ---: |
| compiler | 2 | 2 | 0 |
| compiler-codegen | 8 | 8 | 0 |
| external-BIGIP-via-root | 10 | 6 | 4 |
| vm-runtime | 31 | 11 | 20 |

Actual diagnostic groups: BPF2 refuse missing retained/typed selection; WASM8 cannot instantiate missing host_refusal_pending import; VM31 include30 F5 profile-manifest mismatches and1 genuine-native-preflight refusal. F5 CLI10 stay with external owner via root (including2 simulation profile mismatches shared with VM, not double-counted). No fixes/assertion changes/builds were performed.
