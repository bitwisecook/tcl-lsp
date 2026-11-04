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

## Historical references

- `8-4-20-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/receipt.json). Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `8-4-20-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stderr). Exact empty original stderr; guest errors are caught in stdout.
- `8-4-20-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20/stdout). Exact entire original stdout including caught guest outcomes.
- `8-5-19-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/receipt.json). Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `8-5-19-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stderr). Exact empty original stderr; guest errors are caught in stdout.
- `8-5-19-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19/stdout). Exact entire original stdout including caught guest outcomes.
- `8-6-18-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/receipt.json). Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `8-6-18-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stderr). Exact empty original stderr; guest errors are caught in stdout.
- `8-6-18-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18/stdout). Exact entire original stdout including caught guest outcomes.
- `9-0-4-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/receipt.json). Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `9-0-4-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stderr). Exact empty original stderr; guest errors are caught in stdout.
- `9-0-4-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4/stdout). Exact entire original stdout including caught guest outcomes.
- `9-1-0-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/receipt.json). Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `9-1-0-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stderr). Exact empty original stderr; guest errors are caught in stdout.
- `9-1-0-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0/stdout). Exact entire original stdout including caught guest outcomes.
- `jim-receipt-json` (provider): [rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/receipt.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/receipt.json). Exact original process receipt including selected executable/artifact pins, environment and whole raw stream joins.
- `jim-stderr` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stderr](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stderr). Exact empty original stderr; guest errors are caught in stdout.
- `jim-stdout` (observation): [rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stdout](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/jim/stdout). Exact entire original stdout including caught guest outcomes.
- `probe-tcl` (input): [rust/tcl-registry/tests/data/native_absolute_trace_source253/probe.tcl](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/probe.tcl). Exact original finite ASCII source/provider request; no process outcome.
- `request-json` (input): [rust/tcl-registry/tests/data/native_absolute_trace_source253/request.json](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/request.json). Exact original finite ASCII source/provider request; no process outcome.
- `source-tcl8.4-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceCommandProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.4-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceExecutionProc.txt). Independent exact current pinned release-source excerpt: CString Tcl_Eval. Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.4-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.4.20-TraceVarProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.5-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceCommandProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.5-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceExecutionProc.txt). Independent exact current pinned release-source excerpt: CString Tcl_Eval. Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.5-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.5.19-TraceVarProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.6-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceCommandProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.6-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceExecutionProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl8.6-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/8.6.18-TraceVarProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.0-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceCommandProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.0-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceExecutionProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.0-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.0.4-TraceVarProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.1-TraceCommandProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceCommandProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceCommandProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.1-TraceExecutionProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceExecutionProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceExecutionProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.
- `source-tcl9.1-TraceVarProc` (source-anchor): [rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceVarProc.txt](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/source-anchors/9.1.0-TraceVarProc.txt). Independent exact current pinned release-source excerpt: counted Tcl_EvalEx(flags0). Full source hash and original LF line coordinates are retained; no new build or internal guest observation.

## Source inspection

tcl8.4 8.4.20, revision `pinned release source 8.4.20`, `generic/tclCmdMZ.c`, function `TraceCommandProc`, lines 4217–4223. Retained reference `source-tcl8.4-TraceCommandProc`.

```text
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}

	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {	     
	    /* We ignore errors in these traced commands */

```

tcl8.4 8.4.20, revision `pinned release source 8.4.20`, `generic/tclCmdMZ.c`, function `TraceExecutionProc`, lines 4745–4751. Retained reference `source-tcl8.4-TraceExecutionProc`.

```text
	     * including deleting the trace, the command being
	     * traced, or even the interpreter.
	     */
	    traceCode = Tcl_Eval(interp, Tcl_DStringValue(&cmd));
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;

	    /*

```

tcl8.4 8.4.20, revision `pinned release source 8.4.20`, `generic/tclCmdMZ.c`, function `TraceVarProc`, lines 4907–4913. Retained reference `source-tcl8.4-TraceVarProc`.

```text
		tvarPtr->flags |= TCL_TRACE_DESTROYED;
	    }

	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (code != TCL_OK) {	     /* copy error msg to result */
		register Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);

```

tcl8.5 8.5.19, revision `pinned release source 8.5.19`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1313–1319. Retained reference `source-tcl8.5-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl8.5 8.5.19, revision `pinned release source 8.5.19`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1887–1893. Retained reference `source-tcl8.5-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_Eval(interp, Tcl_DStringValue(&cmd));
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;

	    /*

```

tcl8.5 8.5.19, revision `pinned release source 8.5.19`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2036–2042. Retained reference `source-tcl8.5-TraceVarProc`.

```text
		destroy = 1;
		tvarPtr->flags |= TCL_TRACE_DESTROYED;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (code != TCL_OK) {		/* copy error msg to result */
		Tcl_Obj *errMsgObj = Tcl_GetObjResult(interp);

```

tcl8.6 8.6.18, revision `pinned release source 8.6.18`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1324–1330. Retained reference `source-tcl8.6-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl8.6 8.6.18, revision `pinned release source 8.6.18`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1891–1897. Retained reference `source-tcl8.6-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;


```

tcl8.6 8.6.18, revision `pinned release source 8.6.18`, `generic/tclTrace.c`, function `TraceVarProc`, lines 2047–2053. Retained reference `source-tcl8.6-TraceVarProc`.

```text
	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;

```

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1205–1211. Retained reference `source-tcl9.0-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1772–1778. Retained reference `source-tcl9.0-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;


```

tcl9.0 9.0.4, revision `pinned release source 9.0.4`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1912–1918. Retained reference `source-tcl9.0-TraceVarProc`.

```text
	    if (rewind && (flags & TCL_TRACE_UNSETS)) {
		((Interp *)interp)->execEnvPtr->rewind = 0;
	    }
	    code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    if (rewind) {
		((Interp *)interp)->execEnvPtr->rewind = rewind;

```

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclTrace.c`, function `TraceCommandProc`, lines 1210–1216. Retained reference `source-tcl9.1-TraceCommandProc`.

```text
	if (flags & TCL_TRACE_DESTROYED) {
	    tcmdPtr->flags |= TCL_TRACE_DESTROYED;
	}
	code = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		Tcl_DStringLength(&cmd), 0);
	if (code != TCL_OK) {
	    /* We ignore errors in these traced commands */

```

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclTrace.c`, function `TraceExecutionProc`, lines 1777–1783. Retained reference `source-tcl9.1-TraceExecutionProc`.

```text
	     * interpreter.
	     */

	    traceCode = Tcl_EvalEx(interp, Tcl_DStringValue(&cmd),
		    Tcl_DStringLength(&cmd), 0);
	    tcmdPtr->flags &= ~TCL_TRACE_EXEC_IN_PROGRESS;


```

tcl9.1 9.1.0, revision `pinned release source 9.1.0`, `generic/tclTrace.c`, function `TraceVarProc`, lines 1917–1923. Retained reference `source-tcl9.1-TraceVarProc`.

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
