 *	See the user documentation for details on what it does.
 *
 * Results:
 *	A standard Tcl result.
 *
 * Side effects:
 *	See the user documentation.
 *
 *-------------------------------------------------------------------------
 */

int
Tcl_ClockObjCmd (client, interp, objc, objv)
    ClientData client;			/* Not used. */
    Tcl_Interp *interp;			/* Current interpreter. */
    int objc;				/* Number of arguments. */
    Tcl_Obj *CONST objv[];		/* Argument values. */
{
    Tcl_Obj *resultPtr;
    int index;
    Tcl_Obj *CONST *objPtr;
    int useGMT = 0;
    char *format = "%a %b %d %X %Z %Y";
    int dummy;
    Tcl_WideInt baseClock, clockVal;
    long zone;
    Tcl_Obj *baseObjPtr = NULL;
    char *scanStr;
    int n;
    
    static CONST char *switches[] =
	{"clicks", "format", "scan", "seconds", (char *) NULL};
    enum command { COMMAND_CLICKS, COMMAND_FORMAT, COMMAND_SCAN,
		       COMMAND_SECONDS
    };
    static CONST char *formatSwitches[] = {"-format", "-gmt", (char *) NULL};
    static CONST char *scanSwitches[] = {"-base", "-gmt", (char *) NULL};

    resultPtr = Tcl_GetObjResult(interp);
    if (objc < 2) {
	Tcl_WrongNumArgs(interp, 1, objv, "option ?arg ...?");
	return TCL_ERROR;
    }

    if (Tcl_GetIndexFromObj(interp, objv[1], switches, "option", 0, &index)
	    != TCL_OK) {
	return TCL_ERROR;
    }
    switch ((enum command) index) {
	case COMMAND_CLICKS:	{		/* clicks */
	    int forceMilli = 0;

