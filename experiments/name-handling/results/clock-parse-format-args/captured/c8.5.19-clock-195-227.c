static void		ClockDeleteCmdProc(ClientData);

/*
 * Structure containing description of "native" clock commands to create.
 */

struct ClockCommand {
    const char *name;		/* The tail of the command name. The full name
				 * is "::tcl::clock::<name>". When NULL marks
				 * the end of the table. */
    Tcl_ObjCmdProc *objCmdProc;	/* Function that implements the command. This
				 * will always have the ClockClientData sent
				 * to it, but may well ignore this data. */
};

static const struct ClockCommand clockCommands[] = {
    { "clicks",			ClockClicksObjCmd },
    { "getenv",			ClockGetenvObjCmd },
    { "microseconds",		ClockMicrosecondsObjCmd },
    { "milliseconds",		ClockMillisecondsObjCmd },
    { "seconds",		ClockSecondsObjCmd },
    { "Oldscan",		TclClockOldscanObjCmd },
    { "ConvertLocalToUTC",	ClockConvertlocaltoutcObjCmd },
    { "GetDateFields",		ClockGetdatefieldsObjCmd },
    { "GetJulianDayFromEraYearMonthDay",
		ClockGetjuliandayfromerayearmonthdayObjCmd },
    { "GetJulianDayFromEraYearWeekDay",
    		ClockGetjuliandayfromerayearweekdayObjCmd },
    { "ParseFormatArgs",	ClockParseformatargsObjCmd },
    { NULL, NULL }
};

/*
