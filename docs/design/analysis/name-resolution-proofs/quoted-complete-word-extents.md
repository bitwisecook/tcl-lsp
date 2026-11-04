# naming.quoted-complete-word-extents

Kind: `native-observation`

## Problem statement

A quoted word ending in an escape or bracketed fragment may have a parser component ending before its closing quote. Borrowing that component as the written word extent can truncate compiler/source correspondence and editor selection.

## Question

Does Tcl_ParseCommand retain the final complete word extent for quoted escape tails, command-fragment tails and empty quotes in the five original sources?

## Conclusion

All five C releases report a final top-level word end equal to the complete source length in every original case. This is word geometry, not evaluated value or compiler admission evidence. No Jim ParseCommand or BIG-IP parser result is attached.

## Scope

C Tcl releases explicitly identified in the retained provider rows. Jim and BIG-IP are not tested unless a separate row identifies them. Public Tcl_ParseCommand over the exact five ASCII C strings; original token sizes and word counts, not Unicode/native value conversion.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Recorded executable. Channel: Exact producer API/source in the attached probe or retained row; see scoped input description.. Dialect: Tcl.

1 retained provider record(s) answer the exact scoped inputs. Final selected record fields: {"compile_exit": 0, "run_exit": 0}. Exact result, diagnostic and owner columns are retained in the attached evidence; guest errors are outcomes, not harness success claims.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: Jim Tcl.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: not recorded. Dialect: F5 iRules.

No native observation for this precise question and provider is attached; no result is inferred from another dialect, source channel or record.

## Historical references

- `receipt-0` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/manifest.json](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/manifest.json). Exact recovered input/provider/observation record; initial failed observer attempts, where present, are retained as harness limitations.
- `artifact-1` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.20.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.20.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-2` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.4.value.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-3` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.19.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.19.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-4` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.5.value.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-5` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.18.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.18.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-6` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/8.6.value.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-7` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.4.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.4.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-8` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.0.value.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-9` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.0.tsv](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.0.tsv). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-10` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/9.1.value.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-11` (observation): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/jim.value.txt](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/jim.value.txt). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-12` (input): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.c](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.c). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.
- `artifact-13` (input): [rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.tcl](../../../../rust/tcl-lexer/tests/data/native_quoted_word_geometry/probe.tcl). Retained exact input/observation bytes. Per-question scope selects the relevant producer/case ranges; adjacent streams are not automatically equivalent.

## Source inspection

No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.

## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Original producer code/inputs and captured invocation metadata are pinned above. Restore the exact named provider build and run the original probe with its matching public/private headers and library, preserving its observer order and the60-second/two-slot limits. Recorded absolute executable/output paths are metadata, not usable current paths. Original host probes absent from a corpus remain unreplayable; no source is reconstructed from a digest. Maintained source-only replay instructions are supplied separately where available.
