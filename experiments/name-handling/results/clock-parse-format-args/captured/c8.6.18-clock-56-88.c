 */

typedef enum ClockLiteral {
    LIT__NIL,
    LIT__DEFAULT_FORMAT,
    LIT_BCE,		LIT_C,
    LIT_CANNOT_USE_GMT_AND_TIMEZONE,
    LIT_CE,
    LIT_DAYOFMONTH,	LIT_DAYOFWEEK,		LIT_DAYOFYEAR,
    LIT_ERA,		LIT_GMT,		LIT_GREGORIAN,
    LIT_INTEGER_VALUE_TOO_LARGE,
    LIT_ISO8601WEEK,	LIT_ISO8601YEAR,
    LIT_JULIANDAY,	LIT_LOCALSECONDS,
    LIT_MONTH,
    LIT_SECONDS,	LIT_TZNAME,		LIT_TZOFFSET,
    LIT_YEAR,
    LIT__END
} ClockLiteral;
static const char *const literals[] = {
    "",
    "%a %b %d %H:%M:%S %Z %Y",
    "BCE",		"C",
    "cannot use -gmt and -timezone in same call",
    "CE",
    "dayOfMonth",	"dayOfWeek",		"dayOfYear",
    "era",		":GMT",			"gregorian",
    "integer value too large to represent",
    "iso8601Week",	"iso8601Year",
    "julianDay",	"localSeconds",
    "month",
    "seconds",		"tzName",		"tzOffset",
    "year"
};
