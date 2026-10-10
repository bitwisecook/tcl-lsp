# naming.callback.absolute-trace-source-lookup

Kind: `native-observation`

## Problem statement

Trace prefixes installed in A and triggered in B can be misbound by an installer namespace or an unavailable future frame. Absolute and relative heads require separate observed lookup answers for variable, command and execution callbacks.

## Question

Do absolute trace callback heads select Dest while relative heads select B for the exact variable write, command rename and execution enter/leave traces installed in A and triggered in B?

## Conclusion

All five C processes resolve relative cb in B and absolute ::Dest::cb in Dest, while the public triggering-caller namespace remains ::B for both. Variable and command callbacks each expose three public suffix arguments; execution enter and leave expose two and four. All six commands exit zero with empty stderr and eight exact stdout rows. Jim reports no trace command and catches invalid command name trace for all six installations; it reaches no execution-trace branch. These fixed ASCII public-source controls do not establish native object/header/argv/cache ownership, arbitrary source equivalence, current registrations, future callable occupancy or an entered future source frame. Independent current release-source inspection retains CString Tcl_Eval for C8.4/C8.5 execution callbacks, counted Tcl_EvalEx(flags0) for C8.6+ execution callbacks, and counted Tcl_EvalEx(flags0) for all five variable/command callbacks. These callsites supply no original object-body or compiler-header grant.

## Scope

Original fixed ASCII LF source-file CLI on C8.4.20, 8.5.19, 8.6.18, 9.0.4, 9.1.0 and Jim0.84-9-g5bac7c9. Twenty original request/probe/receipt/stream files are retained exactly; independent current release-source excerpts explain only the selected evaluation entry. BIG-IP is not tested.

## Provider answers

### tcl8.4

Status: `observed`. Version: 8.4.20. Build: Selected native executable SHA-256 f0e742ccc08a150afd9a0a236ce24c8e12098dae38c0efd46e59f9b1a28bf58a; the exact original receipt retains independently checked library/header/Makefile/source artifact pins and environment. No new build is claimed.. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.. Dialect: Tcl.

Exact stdout: version 8.4.20 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Selected native executable SHA-256 e7eb17821aeb7b1f84619f431e89ecff667bc5461fa8e00d67498b6b7d5dd61a; the exact original receipt retains independently checked library/header/Makefile/source artifact pins and environment. No new build is claimed.. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.. Dialect: Tcl.

Exact stdout: version 8.5.19 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Selected native executable SHA-256 9b156375d068a58fcfb82a2a0afd170afd2b03912cbb6e588676c7688118c8ff; the exact original receipt retains independently checked library/header/Makefile/source artifact pins and environment. No new build is claimed.. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.. Dialect: Tcl.

Exact stdout: version 8.6.18 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Selected native executable SHA-256 f4748357cdbf2762f1c5c05916fa53e54718d087120ae9de9c403bbdfaf764b1; the exact original receipt retains independently checked library/header/Makefile/source artifact pins and environment. No new build is claimed.. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.. Dialect: Tcl.

Exact stdout: version 9.0.4 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Selected native executable SHA-256 2a82f721b68f2ed2347ca4e3daa9a56c49c9e3efce77d00f22fc3b2012d8814d; the exact original receipt retains independently checked library/header/Makefile/source artifact pins and environment. No new build is claimed.. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.. Dialect: Tcl.

Exact stdout: version 9.1.0 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### jim

Status: `unsupported`. Version: 0.84-9-g5bac7c9. Build: Selected native executable SHA-256 e4265a14bb6ce0e652d9804f519dfc745fad3acbaf545ba3b120a8a532ab7806; the exact original receipt retains independently checked library/header/Makefile/source artifact pins and environment. No new build is claimed.. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt.. Dialect: Jim Tcl.

Exact stdout: version 0.84-9-g5bac7c9 | trace-command {} | add variable cb 1 {invalid command name "trace"} | add command cb 1 {invalid command name "trace"} | add execution cb 1 {invalid command name "trace"} | add variable ::Dest::cb 1 {invalid command name "trace"} | add command ::Dest::cb 1 {invalid command name "trace"} | add execution ::Dest::cb 1 {invalid command name "trace"} . Trace is absent; each caught installation fails and no callback fires.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: F5 iRules.

No appliance process or event callback observation is retained for this question.

## Exact evidence

- `8-4-20-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/receipt.json). SHA-256 `aa81ce6d7ae75cab51fd6624d5da14ab998b3a0f1dccf43702e01fc74c9eeeb2`. Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `8-4-20-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original stderr; guest errors are caught in stdout.
- `8-4-20-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stdout). SHA-256 `047f6bb21e3542d33bf51789813b1d3517f6db7a242467d81ddd3ef7c49760cb`. Exact entire original stdout including caught guest outcomes.
- `8-5-19-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/receipt.json). SHA-256 `a93f80cf3c51d230f1e3a2b50b9d055d6883b88d990d0f9e42a4e1d09aa075dd`. Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `8-5-19-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original stderr; guest errors are caught in stdout.
- `8-5-19-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stdout). SHA-256 `a72684743a10312b0600432ceeb3b878613cc13c3a04ccc1cac8229a1d54cc86`. Exact entire original stdout including caught guest outcomes.
- `8-6-18-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/receipt.json). SHA-256 `ae9eb0d6901d2c773b8d5739073695dc5594e642beacc1fff8fc4f65cc59a3da`. Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `8-6-18-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original stderr; guest errors are caught in stdout.
- `8-6-18-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stdout). SHA-256 `6b334a0a2c99347178175fa71a7a75d1363dd6a77adb981e575a24020a096cec`. Exact entire original stdout including caught guest outcomes.
- `9-0-4-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/receipt.json). SHA-256 `06d021d6fe9cb09c30013ef6191023fbbb3316497c8e456aec3b9aa63a35bb69`. Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `9-0-4-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original stderr; guest errors are caught in stdout.
- `9-0-4-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stdout). SHA-256 `bc7ac2829769017c4ec9fc26d05dbfc753e73afa714b4fdfa9370f917c70f38d`. Exact entire original stdout including caught guest outcomes.
- `9-1-0-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/receipt.json). SHA-256 `1506f2aca36b58654d146be26d9b7ba5929d920a296c9a53795cbc8ed2cf8059`. Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `9-1-0-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original stderr; guest errors are caught in stdout.
- `9-1-0-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stdout). SHA-256 `2b59a96f6148c0cba4c49c519553ce00dc7f2e10af956de448fe891d80b07905`. Exact entire original stdout including caught guest outcomes.
- `jim-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/receipt.json). SHA-256 `f5ece5b731d3e26b0b47164efb834439d248ee79c8141e4cf175cab0b08e962e`. Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stderr). SHA-256 `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`. Exact empty original stderr; guest errors are caught in stdout.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stdout). SHA-256 `c0868e6029f77eccaad348ec5755159a47fe8ab1489f96607377a8a02cbcab95`. Exact entire original stdout including caught guest outcomes.
- `probe-tcl` (input): [rust/tcl-registry/tests/data/native_absolute_trace_source253/probe.tcl](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/probe.tcl). SHA-256 `fedbb2fc9b26c70cfaf67624bfa331a48bc218f1674e3e8d484a16235d183855`. Exact original finite ASCII source/provider request; no process outcome.
- `request-json` (input): [rust/tcl-registry/tests/data/native_absolute_trace_source253/request.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/request.json). SHA-256 `32fceb5fc719ff00129d8846e7184706056ab74ae7ff45cc58b62d430b9c6f4b`. Exact original finite ASCII source/provider request; no process outcome.
- `source-tcl8.4-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceCommandProc.txt). SHA-256 `6003fe877b2989d81e0d64e48cabc4fb895fd839b6dd48c5d2a437a66f28c6f1`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.4-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceExecutionProc.txt). SHA-256 `4d8673249a3b9b29ecf41221387264fc83443c3703d2dea90562b550cd6e1579`. Independent exact current pinned release-source excerpt: CString Tcl_Eval. Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.4-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceVarProc.txt). SHA-256 `73ebdb1a3586cdd34307be4626e83b8746df1eedb55316cf9e750f0fcd3b3e6e`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.5-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceCommandProc.txt). SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.5-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceExecutionProc.txt). SHA-256 `6e895721f77c3ebbce6be2fbaf4bed1de66663790dcf392f346663d9e6c130f1`. Independent exact current pinned release-source excerpt: CString Tcl_Eval. Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.5-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceVarProc.txt). SHA-256 `cc4de57d24b69f416d76492c85edad38964e86cdf9ec9465c3a5a7e3f5a74dd2`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.6-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceCommandProc.txt). SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.6-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceExecutionProc.txt). SHA-256 `4d453cf08adcce08d14076b8ff04f319ce3c58bca532ad3a6432e8087824ecf7`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.6-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceVarProc.txt). SHA-256 `0eda9899f1387ac36922043f914e1df52c90697a4c859fe48fadb5ffdcb17b6b`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.0-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceCommandProc.txt). SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.0-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceExecutionProc.txt). SHA-256 `4d453cf08adcce08d14076b8ff04f319ce3c58bca532ad3a6432e8087824ecf7`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.0-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceVarProc.txt). SHA-256 `0eda9899f1387ac36922043f914e1df52c90697a4c859fe48fadb5ffdcb17b6b`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.1-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceCommandProc.txt). SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.1-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceExecutionProc.txt). SHA-256 `4d453cf08adcce08d14076b8ff04f319ce3c58bca532ad3a6432e8087824ecf7`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.1-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceVarProc.txt). SHA-256 `0eda9899f1387ac36922043f914e1df52c90697a4c859fe48fadb5ffdcb17b6b`. Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.

## Source inspection

tcl8.4 8.4.20, revision `pinned release source 8.4.20`, `generic/tclCmdMZ.c`, function `TraceCommandProc`, lines 4217–4223. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `6003fe877b2989d81e0d64e48cabc4fb895fd839b6dd48c5d2a437a66f28c6f1`; retained evidence `source-tcl8.4-TraceCommandProc`.

```text
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}

	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {	     
	    /* We ignore errors in these traced commands */

```

tcl8.4 8.4.20, revision `pinned release source 8.4.20`, `generic/tclCmdMZ.c`, function `TraceExecutionProc`, lines 4745–4751. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `4d8673249a3b9b29ecf41221387264fc83443c3703d2dea90562b550cd6e1579`; retained evidence `source-tcl8.4-TraceExecutionProc`.

```text
	     * including deleting the trace, the command being
	     * traced, or even the interpreter.
	     */
	    traceCode = Tcl_Eval(interp, Tcl_DStringValue(&cmd));
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;

	    /*

```

tcl8.4 8.4.20, revision `pinned release source 8.4.20`, `generic/tclCmdMZ.c`, function `TraceVarProc`, lines 4907–4913. Full-source SHA-256 `b374cf4353b86235b189d432239a1eedf8e854eab266b701e9d9625b969d27dc`; snippet SHA-256 `73ebdb1a3586cdd34307be4626e83b8746df1eedb55316cf9e750f0fcd3b3e6e`; retained evidence `source-tcl8.4-TraceVarProc`.

```text
		tvarPtr->flags |= TCL_TRACE_DESTROYED;
	    }

	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (code != TCL_OK) {	     /* copy error msg to result */
		register Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);

```

tcl8.5 8.5.19, revision `pinned release source 8.5.19`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1313–1319. Full-source SHA-256 `d974ce7d57afd060edbc9a858975e17c9577c7905f5dfaf6df477fa409ef89e6`; snippet SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`; retained evidence `source-tcl8.5-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl8.5 8.5.19, revision `pinned release source 8.5.19`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1887–1893. Full-source SHA-256 `d974ce7d57afd060edbc9a858975e17c9577c7905f5dfaf6df477fa409ef89e6`; snippet SHA-256 `6e895721f77c3ebbce6be2fbaf4bed1de66663790dcf392f346663d9e6c130f1`; retained evidence `source-tcl8.5-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_Eval(interp, Tcl_DStringValue(&cmd));
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;

	    /*

```

tcl8.5 8.5.19, revision `pinned release source 8.5.19`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2036–2042. Full-source SHA-256 `d974ce7d57afd060edbc9a858975e17c9577c7905f5dfaf6df477fa409ef89e6`; snippet SHA-256 `cc4de57d24b69f416d76492c85edad38964e86cdf9ec9465c3a5a7e3f5a74dd2`; retained evidence `source-tcl8.5-TraceVarProc`.

```text
		destroy = 1;
		tvarPtr->flags |= TCL_TRACE_DESTROYED;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (code != TCL_OK) {		/* copy error msg to result */
		Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);

```

tcl8.6 8.6.18, revision `pinned release source 8.6.18`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1324–1330. Full-source SHA-256 `8034d0a6da9f529bebeedfa4ab8af88f944bf858ad8c498719a972ac81d503a9`; snippet SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`; retained evidence `source-tcl8.6-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl8.6 8.6.18, revision `pinned release source 8.6.18`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1891–1897. Full-source SHA-256 `8034d0a6da9f529bebeedfa4ab8af88f944bf858ad8c498719a972ac81d503a9`; snippet SHA-256 `4d453cf08adcce08d14076b8ff04f319ce3c58bca532ad3a6432e8087824ecf7`; retained evidence `source-tcl8.6-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;


```

tcl8.6 8.6.18, revision `pinned release source 8.6.18`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2047–2053. Full-source SHA-256 `8034d0a6da9f529bebeedfa4ab8af88f944bf858ad8c498719a972ac81d503a9`; snippet SHA-256 `0eda9899f1387ac36922043f914e1df52c90697a4c859fe48fadb5ffdcb17b6b`; retained evidence `source-tcl8.6-TraceVarProc`.

```text
	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;

```

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1205–1211. Full-source SHA-256 `b091fe603d801002a17d6d4c18de08f43e7aa7851abc64849ab2b1f7aac52904`; snippet SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`; retained evidence `source-tcl9.0-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1772–1778. Full-source SHA-256 `b091fe603d801002a17d6d4c18de08f43e7aa7851abc64849ab2b1f7aac52904`; snippet SHA-256 `4d453cf08adcce08d14076b8ff04f319ce3c58bca532ad3a6432e8087824ecf7`; retained evidence `source-tcl9.0-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;


```

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1912–1918. Full-source SHA-256 `b091fe603d801002a17d6d4c18de08f43e7aa7851abc64849ab2b1f7aac52904`; snippet SHA-256 `0eda9899f1387ac36922043f914e1df52c90697a4c859fe48fadb5ffdcb17b6b`; retained evidence `source-tcl9.0-TraceVarProc`.

```text
	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;

```

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1210–1216. Full-source SHA-256 `b39de1869c493f1405355c9f92537bdd5d9839a42a10da78b18a725309d71847`; snippet SHA-256 `cb29d0ee573fff02a881aaab4c737137857c06911e59140b6824fa1e26ade4ba`; retained evidence `source-tcl9.1-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1777–1783. Full-source SHA-256 `b39de1869c493f1405355c9f92537bdd5d9839a42a10da78b18a725309d71847`; snippet SHA-256 `4d453cf08adcce08d14076b8ff04f319ce3c58bca532ad3a6432e8087824ecf7`; retained evidence `source-tcl9.1-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;


```

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1917–1923. Full-source SHA-256 `b39de1869c493f1405355c9f92537bdd5d9839a42a10da78b18a725309d71847`; snippet SHA-256 `0eda9899f1387ac36922043f914e1df52c90697a4c859fe48fadb5ffdcb17b6b`; retained evidence `source-tcl9.1-TraceVarProc`.

```text
	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;

```


## Consumer bindings

No implementation binding is claimed by this observation record.

A named test is a coverage binding, not a claim that it executed.

## Replay

Recorded argument vector:

```json
[
  "/absolute/path/to/independently-selected/native-tclsh",
  "rust/tcl-registry/tests/data/native_absolute_trace_source253/probe.tcl"
]
```

Repeat independently for each required provider. The original receipt command, environment, exact executable and required artifact SHA, source SHA, whole stdout/stderr and process exit remain the authority. Preserve caught guest errors and all eight rows, including release-specific error presentation. No internal object, native argv/header/cache, future callable/installation, frame or BIG-IP reconfirmation follows. Related source-contract selectors are independent Rust coverage, not additional native results.
