 *-----------------------------------------------------------------------------
 *
 * ClockParseformatargsObjCmd --
 *
 *	Parses the arguments for [clock format].
 *
 * Results:
 *	Returns a standard Tcl result, whose value is a four-element list
 *	comprising the time format, the locale, and the timezone.
 *
 * This function exists because the loop that parses the [clock format]
 * options is a known performance "hot spot", and is implemented in an effort
 * to speed that particular code up.
 *
 *-----------------------------------------------------------------------------
 */

static int
ClockParseformatargsObjCmd(
    ClientData clientData,	/* Client data containing literal pool */
    Tcl_Interp *interp,		/* Tcl interpreter */
    int objc,			/* Parameter count */
    Tcl_Obj *const objv[])	/* Parameter vector */
{
    ClockClientData *dataPtr = (ClockClientData *)clientData;
    Tcl_Obj **litPtr = dataPtr->literals;
    Tcl_Obj *results[3];	/* Format, locale and timezone */
#define formatObj results[0]
#define localeObj results[1]
#define timezoneObj results[2]
    int gmtFlag = 0;
    static const char *const options[] = { /* Command line options expected */
	"-format",	"-gmt",		"-locale",
	"-timezone",	NULL };
    enum optionInd {
	CLOCK_FORMAT_FORMAT,	CLOCK_FORMAT_GMT,	CLOCK_FORMAT_LOCALE,
	CLOCK_FORMAT_TIMEZONE
    };
    int optionIndex;		/* Index of an option. */
    int saw = 0;		/* Flag == 1 if option was seen already. */
    Tcl_WideInt clockVal;	/* Clock value - just used to parse. */
    int i;

    /*
     * Args consist of a time followed by keyword-value pairs.
     */

    if (objc < 2 || (objc % 2) != 0) {
	Tcl_WrongNumArgs(interp, 0, objv,
		"clock format clockval ?-format string? "
		"?-gmt boolean? ?-locale LOCALE? ?-timezone ZONE?");
	Tcl_SetErrorCode(interp, "CLOCK", "wrongNumArgs", (char *)NULL);
	return TCL_ERROR;
    }

    /*
     * Extract values for the keywords.
     */

    formatObj = litPtr[LIT__DEFAULT_FORMAT];
    localeObj = litPtr[LIT_C];
    timezoneObj = litPtr[LIT__NIL];
    for (i = 2; i < objc; i+=2) {
	if (Tcl_GetIndexFromObj(interp, objv[i], options, "option", 0,
		&optionIndex) != TCL_OK) {
	    Tcl_SetErrorCode(interp, "CLOCK", "badOption",
		    TclGetString(objv[i]), (char *)NULL);
	    return TCL_ERROR;
	}
	switch (optionIndex) {
	case CLOCK_FORMAT_FORMAT:
	    formatObj = objv[i+1];
	    break;
	case CLOCK_FORMAT_GMT:
	    if (Tcl_GetBooleanFromObj(interp, objv[i+1], &gmtFlag) != TCL_OK){
		return TCL_ERROR;
	    }
	    break;
	case CLOCK_FORMAT_LOCALE:
	    localeObj = objv[i+1];
	    break;
	case CLOCK_FORMAT_TIMEZONE:
	    timezoneObj = objv[i+1];
	    break;
	}
	saw |= 1 << optionIndex;
    }

    /*
     * Check options.
     */

    if (TclGetWideIntFromObj(interp, objv[1], &clockVal) != TCL_OK) {
	return TCL_ERROR;
    }
    if ((saw & (1 << CLOCK_FORMAT_GMT))
	    && (saw & (1 << CLOCK_FORMAT_TIMEZONE))) {
	Tcl_SetObjResult(interp, litPtr[LIT_CANNOT_USE_GMT_AND_TIMEZONE]);
	Tcl_SetErrorCode(interp, "CLOCK", "gmtWithTimezone", (char *)NULL);
	return TCL_ERROR;
    }
    if (gmtFlag) {
	timezoneObj = litPtr[LIT_GMT];
    }

    /*
     * Return options as a list.
     */

    Tcl_SetObjResult(interp, Tcl_NewListObj(3, results));
    return TCL_OK;

#undef timezoneObj
#undef localeObj
#undef formatObj
}

/*----------------------------------------------------------------------
