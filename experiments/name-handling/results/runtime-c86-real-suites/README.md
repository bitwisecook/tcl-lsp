# Actual C8.6 naming suite measurements

The selected native C8.6 runtime implements `ParseFormatArgs`, `getenv` and
`GetDateFields` with genuine shared three-worker literal ownership, shipped
backing and absent native compiler hooks. `GetDateFields` preserves original
Wide/Int getter order, reached transition access, timezone-name identity and
native Dict/key/numeric roles. Empty timezone data explicitly refuses unavailable
native C localtime conversion. `Oldscan`, `ConvertLocalToUTC`,
`GetJulianDayFromEraYearMonthDay` and `GetJulianDayFromEraYearWeekDay` remain
absent; full seven-worker lifetime and complete clock support are unproved.

| Evidence | Measured scope |
| --- | --- |
| [Original GetDateFields question](get-date-fields-question/README.md) | C8.6 scalar/list/calendar ordering and object roles, with separate C8.4–9.1 and Jim outcomes. C9's timezone-name signature supplies no C8.6 admission. |
| [Runtime exact controls](get-date-fields-runtime/validation-02/validation.json) | Nine controls passed: one compares 32 retained original C8.6 rows, excluding unavailable empty-table localtime; eight cover refusal, original object/key/name ownership, invocation spelling, context/storage effects, actual three-worker deletion and selected-profile backing/hooks. |
| [Core exact controls](get-date-fields-implementation/shared-validation-03/README.md) | Two controls passed: 16 successful original C8.6 calendar rows through the pure numeric helper, plus two unmeasured extent refusal inputs. This supplies no physical Runtime acceptance. |
| [Registry exact control](get-date-fields-implementation/shared-validation-02/README.md) | One guard passed for C8.6-only registration/surface, Dict type, non-pure trait and native null hook. Actual shipped backing is checked by the separate Runtime controls. |
| [Original all.tcl orchestration](get-date-fields-runtime/orchestration-01/record.json) | Init, real Tcltest and original orchestration enter/return were reached. It sourced zero test files and emitted zero tests; exit zero supplies no suite pass. File selection reported no matches; source inspection identifies Runtime `glob -types {b c f p s}` filtering as the current gap. |
| [Upstream suite measurements](20261012T193343Z/README.md) | The identified C capture completes eight unchanged suites with 1182 passes, 85 skips and no failures. Its Native capture completes proc/rename with 40 passes, 17 skips and no failures; five other files refuse and ooNext2 reports a script error. These are the recorded binaries' results, not totals for the current three-worker implementation. |

[Frame-enabled](20261012T192755Z/README.md) and
[default-frame](20261012T193042Z/README.md) diagnostics retain their distinct
wrapper configurations. Skipped, refused, unreached and incomplete files supply
no pass. Exact controls and orchestration observations do not establish an
all-suite result. The captures retain source, dependency, oracle, binary and raw
stream hashes; their scopes remain separate.

Replay the bounded suite question with
`python experiments/name-handling/results/runtime-c86-real-suites/capture.py`;
the exact pinned runner must already exist. No build is performed.
