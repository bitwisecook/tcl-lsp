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

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt. Version: tcl8.4 8.4.20.

Exact stdout: version 8.4.20 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl8.5

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt. Version: tcl8.5 8.5.19.

Exact stdout: version 8.5.19 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl8.6

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt. Version: tcl8.6 8.6.18.

Exact stdout: version 8.6.18 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl9.0

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt. Version: tcl9.0 9.0.4.

Exact stdout: version 9.0.4 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### tcl9.1

Status: `observed`. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt. Version: tcl9.1 9.1.0.

Exact stdout: version 9.1.0 | trace-command trace | fire variable cb 0 VALUE {{variable B 3 ::B}} | fire command cb 0 {} {{command B 3 ::B}} | fire execution cb 0 VALUE {{execution B 2 ::B} {execution B 4 ::B}} | fire variable ::Dest::cb 0 VALUE {{variable Dest 3 ::B}} | fire command ::Dest::cb 0 {} {{command Dest 3 ::B}} | fire execution ::Dest::cb 0 VALUE {{execution Dest 2 ::B} {execution Dest 4 ::B}} . Relative head selects B, absolute head selects Dest, and both report triggering caller ::B. Public variable/command suffix counts are three; execution enter/leave counts are two/four.

### jim

Status: `unsupported`. Channel: Original fixed ASCII LF source-file CLI; caught trace installation/firing, public callback suffix length and public namespace labels; no internal object/header/argv/cache or future registration-currentness receipt. Version: jim 0.84-9-g5bac7c9.

Exact stdout: version 0.84-9-g5bac7c9 | trace-command {} | add variable cb 1 {invalid command name "trace"} | add command cb 1 {invalid command name "trace"} | add execution cb 1 {invalid command name "trace"} | add variable ::Dest::cb 1 {invalid command name "trace"} | add command ::Dest::cb 1 {invalid command name "trace"} | add execution ::Dest::cb 1 {invalid command name "trace"} . Trace is absent; each caught installation fails and no callback fires.

### bigip

Status: `not-tested`.

No appliance process or event callback observation is retained for this question.

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

## Evidence

- [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.4.20)
- [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.5.19)
- [rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/8.6.18)
- [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.0.4)
- [rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/9.1.0)
- [rust/tcl-registry/tests/data/native_absolute_trace_source253/jim](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253/jim)
- [rust/tcl-registry/tests/data/native_absolute_trace_source253](../../../../rust/tcl-registry/tests/data/native_absolute_trace_source253)
