
static const struct ClockCommand clockCommands[] = {
    { "getenv",			ClockGetenvObjCmd },
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
