# naming.numeric.io-debug-public-conversion-and-side-effect-order

Kind: `native-observation`

## Problem statement

A failed numeric conversion must not silently select a default read count, seek offset, wrapping length or debug Boolean. Public workers may use different release-specific converters and option grammars, and an unavailable worker cannot establish conversion behaviour. Cursor changes, debug state queries and errorCode updates distinguish actual command effects from conversion errors.

## Question

For the exact invalid, negative, zero, positive and oversized operands, what public read/seek/base64 maxlen/debug results, errorCode values and cursor or debug-state effects occur on each measured C Tcl release and Jim build?

## Conclusion

Every measured provider rejects read bad, 1.5, -1 and 18446744073709551616 without moving the cursor from 3; read 0 leaves it at 3 and read 2 returns de and moves it to 5. Invalid seek offsets leave the cursor at 3; relative -1 succeeds at 2. The C providers reject bad offset before invalid origin, while Jim returns its handle seek argument error for that combined input. Jim accepts oversized relative seek and reaches 2, so the C overflow results cannot be donated to Jim. Base64 maxlen is available only on the measured C 8.6/9.0/9.1 workers: negative length returns line length out of range, 0 yields unwrapped YWJjZGVm and 2 yields YW\nJj\nZG\nVm. Debug frame accepts 1.5 as true on C 8.5–9.1 and rejects bad while the separate state query stays false. C 8.4 debug and C 8.4/8.5/Jim base64 workers are unavailable; Jim child creation is unavailable for this probe. Exact release-specific messages and errorCode bytes remain in the raw captures. BIG-IP is not tested.

## Scope

One unchanged ASCII probe with physical LF creates a distinct owned per-provider disposable file whose runtime bytes are abcdef followed by LF. Public operations are read(channel,count), seek(channel,offset[,origin]), binary encode base64 -maxlen and interp debug child -frame. Each case seeds errorCode as R2286 SEEDED, records catch result/errorCode as hexadecimal, and reads tell(channel) where applicable. Debug BEFORE/AFTER 0 are supplied constants, not measurements; the separate debug-frame-state query measures debug state. Five C processes supply 43 rows each and Jim supplies 32 rows, totalling 247 rows, with process exit0 and empty stderr. The retained cases.json is a request-only declaration; actual receipts and streams supply results. Version comes from the independently captured same-ELF 627 VERSION stream. Original implementation C/Jim source, internal conversion/cleanup chronology, callbacks, Native operands, full build/load closure, current Rust outcomes and F5/TMM are unmeasured.

## Provider answers

### tcl8.4

Status: `observed`. Channel: CLI unchanged probe.tcl, distinct R2286_IO_PROBE_FILE, exact written abcdef\n escape, public worker calls and hexadecimal results/errorCode; tell and separate debug state queries. Version: tcl8.4 8.4.20.

Read bad/1.5/-1/oversized errors preserve cursor 3; read 0 stays at 3 and read 2 returns de at 5. Relative seek -1 reaches 2. Bad/oversized seek errors preserve cursor 3 and bad offset wins over invalid origin. Base64 encode and interp debug are unavailable. Read bad/-1 follow this release's nonewline option error and overflow reports ARITH IOVERFLOW; other exact errorCode and messages remain in the raw stream. Successful cases retain the seeded errorCode. Version is attributed by the independent same-ELF 627 VERSION capture; this process prints no version.

### tcl8.5

Status: `observed`. Channel: CLI unchanged probe.tcl, distinct R2286_IO_PROBE_FILE, exact written abcdef\n escape, public worker calls and hexadecimal results/errorCode; tell and separate debug state queries. Version: tcl8.5 8.5.19.

Read bad/1.5/-1/oversized errors preserve cursor 3; read 0 stays at 3 and read 2 returns de at 5. Relative seek -1 reaches 2. Bad/oversized seek errors preserve cursor 3 and bad offset wins over invalid origin. Base64 encode is unavailable. Debug bad errors and the separate state query remains 0; debug 1.5 succeeds and the state query becomes 1. Successful cases retain seeded errorCode. Exact release-specific messages/errorCode bytes remain in the raw stream. Version is attributed by the independent same-ELF 627 VERSION capture; this process prints no version.

### tcl8.6, tcl9.0, tcl9.1

Status: `observed`. Channel: CLI unchanged probe.tcl, distinct R2286_IO_PROBE_FILE, exact written abcdef\n escape, public worker calls and hexadecimal results/errorCode; tell and separate debug state queries. Version: tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Read bad/1.5/-1/oversized errors preserve cursor 3; read 0 stays at 3 and read 2 returns de at 5. Relative seek -1 reaches 2. Bad/oversized seek errors preserve cursor 3 and bad offset wins over invalid origin. Base64 maxlen bad/1.5/oversized error; -1 returns line length out of range, 0 returns unwrapped YWJjZGVm and 2 returns YW\nJj\nZG\nVm. Debug bad errors and the separate state query remains 0; debug 1.5 succeeds and the state query becomes 1. Successful cases retain seeded errorCode. Exact release-specific messages/errorCode bytes remain in the raw stream. Version is attributed by the independent same-ELF 627 VERSION capture; this process prints no version.

### jim

Status: `observed`. Channel: CLI unchanged probe.tcl, distinct R2286_IO_PROBE_FILE, exact written abcdef\n escape, public worker calls and hexadecimal results/errorCode; tell and separate debug state queries. Version: jim 0.84-9-g5bac7c9.

Read bad/1.5/-1/oversized errors preserve cursor 3; read 0 stays at 3 and read 2 returns de at 5. Relative seek -1 reaches 2. Bad offset with invalid origin reports the handle seek argument error; oversized relative seek succeeds at 2. Base64 encode and child-interpreter creation are unavailable for the intended workers. Observed errorCode is NONE, including successful calls. Version is attributed by the independent same-ELF 627 VERSION capture; this process prints no version.

### bigip

Status: `not-tested`. Channel: No appliance capture for this question.

No actual BIG-IP response to these exact inputs is retained. Local C Tcl/Jim outcomes establish no BIG-IP or TMM behaviour.

## Consumer bindings

- [rust/tcl-registry/src/native_numeric_error/command.rs](../../../../rust/tcl-registry/src/native_numeric_error/command.rs), `InvocationDialect::native_numeric_command_protocol`: Select actual command-purpose primitive widths, read grammar/error recipe, C/Jim seek order and unsupported worker gate; descriptive metadata supplies no primitive execution authority.
- [rust/tcl-vm/src/expr/original_numeric_context.rs](../../../../rust/tcl-vm/src/expr/original_numeric_context.rs), `with_original_numeric_command`: Retain actual same command scope before Name/numeric stages through backend/result settlement, refusing full Host before any Guest fallback/default or mutation.
- [rust/tcl-vm/src/cmd_chan.rs](../../../../rust/tcl-vm/src/cmd_chan.rs), `Channel::seek`: Refuse negative actual target before cursor mutation and retain original opened byte path for selected Jim diagnostics.
- [runtime/rust/src/cmd_binary.rs](../../../../runtime/rust/src/cmd_binary.rs), `binary_encode_wrapped`: Actual original maxlen operands use selected primitive purpose and exact full-cause scalar owner, without container Index-expression fallback; unsupported public workers remain unavailable.
- [rust/tcl-registry/src/native_numeric_error/command.rs](../../../../rust/tcl-registry/src/native_numeric_error/command.rs), `native_numeric_error::command::tests::original_numeric_command_recipes_keep_actual_width_order_and_unavailable_workers` (linked): Selected Registry recipes preserve C release-specific read-count rules, seek width/order, line-length getter widths and unavailable base64/debug purposes; Jim retains its own aio options, origin order and path-aware negative-seek recipe. Descriptor assertions do not observe internal Native worker timing or side effects.
- [rust/tcl-vm/src/cmd_chan.rs](../../../../rust/tcl-vm/src/cmd_chan.rs), `cmd_chan::original_channel_position_tests::original_channel_negative_target_refuses_before_position_and_retains_opened_path` (linked): The backend control asserts that an invalid negative target leaves the actual cursor unchanged and preserves the owned opened byte path used by Jim error presentation. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.
- [rust/tcl-vm/src/interp/native_string_stage_tests.rs](../../../../rust/tcl-vm/src/interp/native_string_stage_tests.rs), `interp::native_numeric_ops::original_string_stage_tests::original_public_read_counts_preserve_selected_errors_and_untouched_cursor` (linked): The control asserts release-selected ASCII read errors/errorCode and unchanged cursor for invalid/negative counts, plus successful count2. Native628 independently measures its own public channel/script inputs. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.
- [rust/tcl-vm/src/interp/native_string_stage_tests.rs](../../../../rust/tcl-vm/src/interp/native_string_stage_tests.rs), `interp::native_numeric_ops::original_string_stage_tests::original_public_seek_errors_preserve_conversion_order_and_negative_target_position` (linked): The control asserts C numeric-before-origin versus Jim origin-first diagnostics, owned-path negative-target errors and unchanged cursor, and legitimate relative negative seek. Native628 supplies only its separate public observations. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.
- [rust/tcl-vm/src/interp/native_string_stage_tests.rs](../../../../rust/tcl-vm/src/interp/native_string_stage_tests.rs), `interp::native_numeric_ops::original_string_stage_tests::original_public_binary_line_length_uses_actual_primitive_and_negative_code` (linked): The control asserts C86/C9 selected primitive diagnostics/errorCode and negative line-length code on the VM adapter, independently of the exact628 public process rows. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.
- [rust/tcl-vm/src/interp/native_string_stage_tests.rs](../../../../rust/tcl-vm/src/interp/native_string_stage_tests.rs), `interp::native_numeric_ops::original_string_stage_tests::original_public_debug_bad_boolean_refuses_before_flag_and_numeric_boolean_succeeds` (linked): The control asserts selected C85+ bad-Boolean diagnostics/errorCode without flag mutation and successful numeric Boolean input; Native628 records its separate public worker outcomes. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.
- [rust/tcl-vm/src/interp/native_string_stage_tests.rs](../../../../rust/tcl-vm/src/interp/native_string_stage_tests.rs), `interp::native_numeric_ops::original_string_stage_tests::original_open_channel_retains_the_actual_checked_path_for_jim_seek_error` (linked): The control opens a real disposable software-owned file and asserts that the actual retained byte path drives Jim negative-seek presentation and cursor preservation before close. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.
- [runtime/rust/src/cmd_binary/original_line_length_tests.rs](../../../../runtime/rust/src/cmd_binary/original_line_length_tests.rs), `cmd_binary::original_line_length_tests::original_binary_maxlen_uses_selected_primitive_errors_and_never_index_grammar` (linked): The Runtime control asserts primitive integer errors for BAD/end/1+2 and negative line-length errorCode, with no Index-expression acceptance or typed Host refusal; Native628 separately measures its public cases. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.
- [runtime/rust/src/cmd_binary/original_line_length_tests.rs](../../../../runtime/rust/src/cmd_binary/original_line_length_tests.rs), `cmd_binary::original_line_length_tests::original_binary_base64_wide_and_uuencode_int_remain_distinct` (linked): This is an additional source/API width control for original2147483648: selected C9 base64 Wide succeeds while uuencode Int fails. The628 public script did not measure this uuencode/width case and supplies no such observation. This is a linked source-defined software/API control; no matching current owned assertion outcome is attached here. It grants no external-provider observation, Native callback chronology, physical frame identity or execution permission.

## Evidence

- [runtime/rust/tests/data/native_io_debug_public_conversion_side_effect_order_original](../../../../runtime/rust/tests/data/native_io_debug_public_conversion_side_effect_order_original)
- [runtime/rust/tests/data/native_io_debug_public_conversion_side_effect_order_original/628](../../../../runtime/rust/tests/data/native_io_debug_public_conversion_side_effect_order_original/628)
- [rust/tcl-registry/src/native_numeric_error](../../../../rust/tcl-registry/src/native_numeric_error)
- [runtime/rust/src](../../../../runtime/rust/src)
- [runtime/rust/src/cmd_binary](../../../../runtime/rust/src/cmd_binary)
- [rust/tcl-vm/src](../../../../rust/tcl-vm/src)
- [rust/tcl-vm/src/expr](../../../../rust/tcl-vm/src/expr)
- [rust/tcl-vm/src/interp](../../../../rust/tcl-vm/src/interp)
