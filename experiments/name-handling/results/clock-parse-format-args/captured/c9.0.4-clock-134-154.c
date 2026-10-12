static const struct ClockCommand clockCommands[] = {
    {"add",		ClockAddObjCmd,		TclCompileBasicMin1ArgCmd, NULL},
    {"clicks",		ClockClicksObjCmd,	TclCompileClockClicksCmd,  NULL},
    {"format",		ClockFormatObjCmd,	TclCompileBasicMin1ArgCmd, NULL},
    {"getenv",		ClockGetenvObjCmd,	TclCompileBasicMin1ArgCmd, NULL},
    {"microseconds",	ClockMicrosecondsObjCmd,TclCompileClockReadingCmd, INT2PTR(1)},
    {"milliseconds",	ClockMillisecondsObjCmd,TclCompileClockReadingCmd, INT2PTR(2)},
    {"scan",		ClockScanObjCmd,	TclCompileBasicMin1ArgCmd, NULL},
    {"seconds",		ClockSecondsObjCmd,	TclCompileClockReadingCmd, INT2PTR(3)},
    {"ConvertLocalToUTC", ClockConvertlocaltoutcObjCmd,		NULL, NULL},
    {"GetDateFields",	  ClockGetdatefieldsObjCmd,		NULL, NULL},
    {"GetJulianDayFromEraYearMonthDay",
		ClockGetjuliandayfromerayearmonthdayObjCmd,	NULL, NULL},
    {"GetJulianDayFromEraYearWeekDay",
		ClockGetjuliandayfromerayearweekdayObjCmd,	NULL, NULL},
    {"catch",		ClockSafeCatchCmd,	TclCompileBasicMin1ArgCmd, NULL},
    {NULL, NULL, NULL, NULL}
};

/*
 *----------------------------------------------------------------------
