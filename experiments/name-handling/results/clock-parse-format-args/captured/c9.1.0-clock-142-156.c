static const struct ClockCommand clockCommands[] = {
    {"add",		ClockAddObjCmd,		TclCompileBasicMin1ArgCmd, 1},
    {"format",		ClockFormatObjCmd,	TclCompileBasicMin1ArgCmd, 1},
    {"getenv",		ClockGetenvObjCmd,	NULL, 1},
    {"scan",		ClockScanObjCmd,	TclCompileBasicMin1ArgCmd, 1},
    {"ConvertLocalToUTC", ClockConvertlocaltoutcObjCmd, NULL, 1},
    {"GetDateFields",	ClockGetdatefieldsObjCmd, NULL, 1},
    {"GetJulianDayFromEraYearMonthDay",
			ClockGetjuliandayfromerayearmonthdayObjCmd, NULL, 1},
    {"GetJulianDayFromEraYearWeekDay",
			ClockGetjuliandayfromerayearweekdayObjCmd, NULL, 1},
    {"catch",		TclSafeCatchCmd,	NULL, 0},
    {NULL, NULL, NULL, 0}
};

