Tcl_EvalEx(
    Tcl_Interp *interp,		/* Interpreter in which to evaluate the
				 * script. Also used for error reporting. */
    const char *script,		/* First character of script to evaluate. */
    Tcl_Size numBytes,		/* Number of bytes in script. If -1, the
				 * script consists of all bytes up to the
				 * first null character. */
    int flags)			/* Collection of OR-ed bits that control the
				 * evaluation of the script. Only
				 * TCL_EVAL_GLOBAL is currently supported. */
{
    return TclEvalEx(interp, script, numBytes, flags, 1, NULL, script);
}
