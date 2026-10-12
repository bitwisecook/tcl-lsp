# GetDateFields: actual first missing clock worker

The unchanged original all.tcl wrapper now stops before any suite with invalid command name "GetDateFields"; prior split Unicode refusal is passed.
Actual C86 caller clock.tcl:745–747 is generated format-procedure preFormatCode: GetDateFields $clockval $TZData($timezone) locale Gregorian changeover.
Worker owner is generic/tclClock.c:428; table conversion/transition lookup and calendar helpers are in the same C file. This is not the existing proleptic-Gregorian convenience formatter.

capture-01 retains 32 fixed arity/value/calendar/table questions plus one read-effect question per complete setup.
C86 has 17 success/16 error rows; C85 same success counts with nine diagnostic/code differences. C84 and current Native C86 have no worker and 33 guest-missing rows.
Pinned Jim unadapted setup has no worker (32 missing rows), then rejects absent trace; no adaptation or worker emulation was added.
C90 original input retains32 rows then rejects removed classic trace syntax. C91 first metadata compile failure objClientData/objClientData2 is retained.
capture-02 corrects only that source observer ABI and asks three C9 version questions after genuine clock format warmup: each 1 success/2 errors, process0.
C9 uses a timezone name, not C86 tzdata; it reads TZData and has different overflow ordering. C90 constructor hook is present; C91 constructor hook absent. Neither grants C86 backing.
C85/C86 constructors have NULL hooks and actual shared clientdata with parser/getter. Hook observations are independent of effects/purity.

C86 evaluates Wide seconds then C Int changeover, then rejects an actual bignum seconds primary, then parses timezone data.
Thus unsigned9223372036854775808 is rejected here (unlike ParseFormatArgs); bad changeover wins before that rejection. C Int4294967295 is accepted;4294967296 errors.
Transition lookup selects last <=seconds; before first selects row0. It examines reached transitions only, ignores DST/extra cells, and preserves original tzName object.
Malformed outer lists/reached numbers have exact error codes; short rows that can leave C-owned cell extent were not executed or inferred as safe guest errors.
Calendar rows distinguish negative-day floor, leap day, ISO year/week, Julian/Gregorian changeover and supplied timezone offsets.
Empty timezone data genuinely calls C localtime; one synthetic TZ=UTC0 row returns tzName +0000. No host timezone/environment values were logged.
C86 original-object control sees fresh Dict primary, scalar/list coercion, original timezone-name identity (raw4e00ff), shared literal key identity and surviving name after retirement.
Read effects remain ordered seconds/zone/change before reached worker conversions.

Exact sources, commands, library/worker hashes, binary hashes and raw streams are in captures; control executables are ignored, not committed.
No Rust/Cargo/runtime worker implementation or framework replacement occurred. Existing seven-file split readiness/results remain sealed unchanged.
F5 TMM/MCPD native worker contexts remain unanswered; no Tcltest equivalence. Minimal selected C86 proposal is in proposal.md.
