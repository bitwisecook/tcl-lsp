// ── stdlib commands ──

// gettimes
inline constexpr CommandDesc stdlib_gettimes_cmd{
    .name = "gettimes",
    .warn_missing_import = false,
    .hover_index = 289,
};

// history
inline constexpr FormDesc stdlib_history_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc stdlib_history_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true, .writes = true},
};

inline constexpr SubCmdDesc stdlib_history_subcmds[] = {
    {.name = "add", .arity = {1, 2}, .hover_index = 290},
    {.name = "change", .arity = {1, 2}, .hover_index = 291},
    {.name = "clear", .arity = {0, 0}, .hover_index = 292},
    {.name = "event", .arity = {0, 1}, .hover_index = 293},
    {.name = "info", .arity = {0, 1}, .hover_index = 294},
    {.name = "keep", .arity = {0, 1}, .hover_index = 295},
    {.name = "nextid", .arity = {0, 0}, .hover_index = 296},
    {.name = "redo", .arity = {0, 1}, .hover_index = 297},
};

inline constexpr CommandDesc stdlib_history_cmd{
    .name = "history",
    .subcommands = stdlib_history_subcmds,
    .forms = stdlib_history_forms,
    .unsafe = true,
    .side_effect_hints = stdlib_history_effects,
    .hover_index = 298,
};

// http::cleanup
inline constexpr CommandDesc stdlib_http__cleanup_cmd{
    .name = "http::cleanup",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 299,
};

// http::code
inline constexpr CommandDesc stdlib_http__code_cmd{
    .name = "http::code",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 300,
};

// http::config
inline constexpr CommandDesc stdlib_http__config_cmd{
    .name = "http::config",
    .required_package = "http",
    .hover_index = 301,
};

// http::cookiejar
inline constexpr SideEffectDesc stdlib_http__cookiejar_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true},
};

inline constexpr CommandDesc stdlib_http__cookiejar_cmd{
    .name = "http::cookiejar",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .side_effect_hints = stdlib_http__cookiejar_effects,
    .required_package = "cookiejar",
    .hover_index = 302,
};

// http::cookiejar::IDNAdecode
inline constexpr CommandDesc stdlib_http__cookiejar__IDNAdecode_cmd{
    .name = "http::cookiejar::IDNAdecode",
    .arity = {1, 1},
    .pure = true,
    .required_package = "cookiejar",
    .hover_index = 303,
};

// http::cookiejar::IDNAencode
inline constexpr CommandDesc stdlib_http__cookiejar__IDNAencode_cmd{
    .name = "http::cookiejar::IDNAencode",
    .arity = {1, 1},
    .pure = true,
    .required_package = "cookiejar",
    .hover_index = 304,
};

// http::data
inline constexpr CommandDesc stdlib_http__data_cmd{
    .name = "http::data",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 305,
};

// http::error
inline constexpr CommandDesc stdlib_http__error_cmd{
    .name = "http::error",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 306,
};

// http::formatQuery
inline constexpr CommandDesc stdlib_http__formatQuery_cmd{
    .name = "http::formatQuery",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .pure = true,
    .required_package = "http",
    .hover_index = 307,
};

// http::geturl
inline constexpr SideEffectDesc stdlib_http__geturl_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true},
};

inline constexpr std::string_view stdlib_http__geturl_cred_opts[] = {
    "-headers",
};

inline constexpr CommandDesc stdlib_http__geturl_cmd{
    .name = "http::geturl",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .side_effect_hints = stdlib_http__geturl_effects,
    .required_package = "http",
    .hover_index = 308,
    .credential_options = stdlib_http__geturl_cred_opts,
};

// http::meta
inline constexpr CommandDesc stdlib_http__meta_cmd{
    .name = "http::meta",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 309,
};

// http::ncode
inline constexpr CommandDesc stdlib_http__ncode_cmd{
    .name = "http::ncode",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 310,
};

// http::quoteString
inline constexpr CommandDesc stdlib_http__quoteString_cmd{
    .name = "http::quoteString",
    .arity = {1, 1},
    .pure = true,
    .required_package = "http",
    .hover_index = 311,
};

// http::register
inline constexpr CommandDesc stdlib_http__register_cmd{
    .name = "http::register",
    .arity = {3, 3},
    .required_package = "http",
    .hover_index = 312,
};

// http::reset
inline constexpr CommandDesc stdlib_http__reset_cmd{
    .name = "http::reset",
    .arity = {1, 2},
    .required_package = "http",
    .hover_index = 313,
};

// http::size
inline constexpr CommandDesc stdlib_http__size_cmd{
    .name = "http::size",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 314,
};

// http::status
inline constexpr CommandDesc stdlib_http__status_cmd{
    .name = "http::status",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 315,
};

// http::unregister
inline constexpr CommandDesc stdlib_http__unregister_cmd{
    .name = "http::unregister",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 316,
};

// http::wait
inline constexpr CommandDesc stdlib_http__wait_cmd{
    .name = "http::wait",
    .arity = {1, 1},
    .required_package = "http",
    .hover_index = 317,
};

// lgen
inline constexpr CommandDesc stdlib_lgen_cmd{
    .name = "lgen",
    .warn_missing_import = false,
    .hover_index = 318,
};

// lstring
inline constexpr CommandDesc stdlib_lstring_cmd{
    .name = "lstring",
    .warn_missing_import = false,
    .hover_index = 319,
};

// msgcat::mc
inline constexpr SideEffectDesc stdlib_msgcat__mc_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true, .writes = true},
};

inline constexpr CommandDesc stdlib_msgcat__mc_cmd{
    .name = "msgcat::mc",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .side_effect_hints = stdlib_msgcat__mc_effects,
    .required_package = "msgcat",
    .hover_index = 320,
};

// msgcat::mcexists
inline constexpr CommandDesc stdlib_msgcat__mcexists_cmd{
    .name = "msgcat::mcexists",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .required_package = "msgcat",
    .hover_index = 321,
};

// msgcat::mcflmset
inline constexpr CommandDesc stdlib_msgcat__mcflmset_cmd{
    .name = "msgcat::mcflmset",
    .arity = {1, 1},
    .required_package = "msgcat",
    .hover_index = 322,
};

// msgcat::mcflset
inline constexpr CommandDesc stdlib_msgcat__mcflset_cmd{
    .name = "msgcat::mcflset",
    .arity = {1, 2},
    .required_package = "msgcat",
    .hover_index = 323,
};

// msgcat::mcforgetpackage
inline constexpr CommandDesc stdlib_msgcat__mcforgetpackage_cmd{
    .name = "msgcat::mcforgetpackage",
    .arity = {0, 0},
    .required_package = "msgcat",
    .hover_index = 324,
};

// msgcat::mcload
inline constexpr CommandDesc stdlib_msgcat__mcload_cmd{
    .name = "msgcat::mcload",
    .arity = {1, 1},
    .required_package = "msgcat",
    .hover_index = 325,
};

// msgcat::mcloadedlocales
inline constexpr CommandDesc stdlib_msgcat__mcloadedlocales_cmd{
    .name = "msgcat::mcloadedlocales",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .required_package = "msgcat",
    .hover_index = 326,
};

// msgcat::mclocale
inline constexpr CommandDesc stdlib_msgcat__mclocale_cmd{
    .name = "msgcat::mclocale",
    .arity = {0, 1},
    .required_package = "msgcat",
    .hover_index = 327,
};

// msgcat::mcmax
inline constexpr CommandDesc stdlib_msgcat__mcmax_cmd{
    .name = "msgcat::mcmax",
    .required_package = "msgcat",
    .hover_index = 328,
};

// msgcat::mcmset
inline constexpr CommandDesc stdlib_msgcat__mcmset_cmd{
    .name = "msgcat::mcmset",
    .arity = {2, 2},
    .required_package = "msgcat",
    .hover_index = 329,
};

// msgcat::mcn
inline constexpr CommandDesc stdlib_msgcat__mcn_cmd{
    .name = "msgcat::mcn",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .required_package = "msgcat",
    .hover_index = 330,
};

// msgcat::mcpackageconfig
inline constexpr CommandDesc stdlib_msgcat__mcpackageconfig_cmd{
    .name = "msgcat::mcpackageconfig",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .required_package = "msgcat",
    .hover_index = 331,
};

// msgcat::mcpackagelocale
inline constexpr CommandDesc stdlib_msgcat__mcpackagelocale_cmd{
    .name = "msgcat::mcpackagelocale",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .required_package = "msgcat",
    .hover_index = 332,
};

// msgcat::mcpreferences
inline constexpr CommandDesc stdlib_msgcat__mcpreferences_cmd{
    .name = "msgcat::mcpreferences",
    .arity = {0, 0},
    .required_package = "msgcat",
    .hover_index = 333,
};

// msgcat::mcset
inline constexpr CommandDesc stdlib_msgcat__mcset_cmd{
    .name = "msgcat::mcset",
    .arity = {2, 3},
    .required_package = "msgcat",
    .hover_index = 334,
};

// msgcat::mcunknown
inline constexpr CommandDesc stdlib_msgcat__mcunknown_cmd{
    .name = "msgcat::mcunknown",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .required_package = "msgcat",
    .hover_index = 335,
};

// noop
inline constexpr CommandDesc stdlib_noop_cmd{
    .name = "noop",
    .warn_missing_import = false,
    .hover_index = 336,
};

// pkg::create
inline constexpr CommandDesc stdlib_pkg__create_cmd{
    .name = "pkg::create",
    .arity = {4, std::numeric_limits<std::int32_t>::max()},
    .hover_index = 337,
};

// pkg_mkIndex
inline constexpr SideEffectDesc stdlib_pkg_mkIndex_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true, .writes = true},
};

inline constexpr CommandDesc stdlib_pkg_mkIndex_cmd{
    .name = "pkg_mkIndex",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .side_effect_hints = stdlib_pkg_mkIndex_effects,
    .hover_index = 338,
};

// platform::generic
inline constexpr CommandDesc stdlib_platform__generic_cmd{
    .name = "platform::generic",
    .arity = {0, 0},
    .pure = true,
    .required_package = "platform",
    .hover_index = 339,
};

// platform::identify
inline constexpr SideEffectDesc stdlib_platform__identify_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc stdlib_platform__identify_cmd{
    .name = "platform::identify",
    .arity = {0, 0},
    .pure = true,
    .side_effect_hints = stdlib_platform__identify_effects,
    .required_package = "platform",
    .hover_index = 340,
};

// platform::patterns
inline constexpr CommandDesc stdlib_platform__patterns_cmd{
    .name = "platform::patterns",
    .arity = {1, 1},
    .pure = true,
    .required_package = "platform",
    .hover_index = 341,
};

// platform::shell::generic
inline constexpr CommandDesc stdlib_platform__shell__generic_cmd{
    .name = "platform::shell::generic",
    .arity = {1, 1},
    .required_package = "platform::shell",
    .hover_index = 342,
};

// platform::shell::identify
inline constexpr CommandDesc stdlib_platform__shell__identify_cmd{
    .name = "platform::shell::identify",
    .arity = {1, 1},
    .required_package = "platform::shell",
    .hover_index = 343,
};

// safe::interpAddToAccessPath
inline constexpr CommandDesc stdlib_safe__interpAddToAccessPath_cmd{
    .name = "safe::interpAddToAccessPath",
    .arity = {2, 2},
    .required_package = "safe",
    .hover_index = 344,
};

// safe::interpConfigure
inline constexpr CommandDesc stdlib_safe__interpConfigure_cmd{
    .name = "safe::interpConfigure",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .required_package = "safe",
    .hover_index = 345,
};

// safe::interpCreate
inline constexpr SideEffectDesc stdlib_safe__interpCreate_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc stdlib_safe__interpCreate_cmd{
    .name = "safe::interpCreate",
    .side_effect_hints = stdlib_safe__interpCreate_effects,
    .required_package = "safe",
    .hover_index = 346,
};

// safe::interpDelete
inline constexpr CommandDesc stdlib_safe__interpDelete_cmd{
    .name = "safe::interpDelete",
    .arity = {1, 1},
    .required_package = "safe",
    .hover_index = 347,
};

// safe::interpFindInAccessPath
inline constexpr CommandDesc stdlib_safe__interpFindInAccessPath_cmd{
    .name = "safe::interpFindInAccessPath",
    .arity = {2, 2},
    .required_package = "safe",
    .hover_index = 348,
};

// safe::interpInit
inline constexpr CommandDesc stdlib_safe__interpInit_cmd{
    .name = "safe::interpInit",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .required_package = "safe",
    .hover_index = 349,
};

// safe::setLogCmd
inline constexpr CommandDesc stdlib_safe__setLogCmd_cmd{
    .name = "safe::setLogCmd",
    .required_package = "safe",
    .hover_index = 350,
};

// safe::setSyncMode
inline constexpr CommandDesc stdlib_safe__setSyncMode_cmd{
    .name = "safe::setSyncMode",
    .arity = {0, 2},
    .required_package = "safe",
    .hover_index = 351,
};

// tcl::OptKeyDelete
inline constexpr CommandDesc stdlib_tcl__OptKeyDelete_cmd{
    .name = "tcl::OptKeyDelete",
    .arity = {1, 1},
    .required_package = "opt",
    .hover_index = 352,
};

// tcl::OptKeyError
inline constexpr CommandDesc stdlib_tcl__OptKeyError_cmd{
    .name = "tcl::OptKeyError",
    .arity = {1, 2},
    .required_package = "opt",
    .hover_index = 353,
};

// tcl::OptKeyParse
inline constexpr CommandDesc stdlib_tcl__OptKeyParse_cmd{
    .name = "tcl::OptKeyParse",
    .arity = {2, 2},
    .required_package = "opt",
    .hover_index = 354,
};

// tcl::OptKeyRegister
inline constexpr CommandDesc stdlib_tcl__OptKeyRegister_cmd{
    .name = "tcl::OptKeyRegister",
    .arity = {1, 2},
    .required_package = "opt",
    .hover_index = 355,
};

// tcl::OptParse
inline constexpr CommandDesc stdlib_tcl__OptParse_cmd{
    .name = "tcl::OptParse",
    .arity = {2, 2},
    .required_package = "opt",
    .hover_index = 356,
};

// tcl::OptProc
inline constexpr SideEffectDesc stdlib_tcl__OptProc_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc stdlib_tcl__OptProc_cmd{
    .name = "tcl::OptProc",
    .arity = {3, 3},
    .side_effect_hints = stdlib_tcl__OptProc_effects,
    .required_package = "opt",
    .hover_index = 357,
};

// tcl::OptProcArgGiven
inline constexpr CommandDesc stdlib_tcl__OptProcArgGiven_cmd{
    .name = "tcl::OptProcArgGiven",
    .arity = {1, 1},
    .required_package = "opt",
    .hover_index = 358,
};

// tcl::tm::path
inline constexpr SideEffectDesc stdlib_tcl__tm__path_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true, .writes = true},
};

inline constexpr SubCmdDesc stdlib_tcl__tm__path_subcmds[] = {
    {.name = "add", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "list", .arity = {0, 0}},
    {.name = "remove", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc stdlib_tcl__tm__path_cmd{
    .name = "tcl::tm::path",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = stdlib_tcl__tm__path_subcmds,
    .side_effect_hints = stdlib_tcl__tm__path_effects,
    .hover_index = 359,
};

// tcl::tm::roots
inline constexpr CommandDesc stdlib_tcl__tm__roots_cmd{
    .name = "tcl::tm::roots",
    .arity = {1, 1},
    .hover_index = 360,
};

// tcl_endOfWord
inline constexpr CommandDesc stdlib_tcl_endOfWord_cmd{
    .name = "tcl_endOfWord",
    .arity = {2, 2},
    .pure = true,
    .hover_index = 361,
};

// tcl_startOfNextWord
inline constexpr CommandDesc stdlib_tcl_startOfNextWord_cmd{
    .name = "tcl_startOfNextWord",
    .arity = {2, 2},
    .pure = true,
    .hover_index = 362,
};

// tcl_startOfPreviousWord
inline constexpr CommandDesc stdlib_tcl_startOfPreviousWord_cmd{
    .name = "tcl_startOfPreviousWord",
    .arity = {2, 2},
    .pure = true,
    .hover_index = 363,
};

// tcl_wordBreakAfter
inline constexpr SideEffectDesc stdlib_tcl_wordBreakAfter_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc stdlib_tcl_wordBreakAfter_cmd{
    .name = "tcl_wordBreakAfter",
    .arity = {2, 2},
    .pure = true,
    .side_effect_hints = stdlib_tcl_wordBreakAfter_effects,
    .hover_index = 364,
};

// tcl_wordBreakBefore
inline constexpr CommandDesc stdlib_tcl_wordBreakBefore_cmd{
    .name = "tcl_wordBreakBefore",
    .arity = {2, 2},
    .pure = true,
    .hover_index = 365,
};

// tcltest::bytestring
inline constexpr CommandDesc stdlib_tcltest__bytestring_cmd{
    .name = "tcltest::bytestring",
    .arity = {1, 1},
    .required_package = "tcltest",
    .hover_index = 366,
    .deprecated_replacement = "encoding convertfrom identity",
};

// tcltest::cleanupTests
inline constexpr CommandDesc stdlib_tcltest__cleanupTests_cmd{
    .name = "tcltest::cleanupTests",
    .arity = {0, 0},
    .required_package = "tcltest",
    .hover_index = 367,
};

// tcltest::configure
inline constexpr CommandDesc stdlib_tcltest__configure_cmd{
    .name = "tcltest::configure",
    .required_package = "tcltest",
    .hover_index = 368,
};

// tcltest::customMatch
inline constexpr CommandDesc stdlib_tcltest__customMatch_cmd{
    .name = "tcltest::customMatch",
    .arity = {2, 2},
    .required_package = "tcltest",
    .hover_index = 369,
};

// tcltest::debug
inline constexpr CommandDesc stdlib_tcltest__debug_cmd{
    .name = "tcltest::debug",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 370,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::errorChannel
inline constexpr CommandDesc stdlib_tcltest__errorChannel_cmd{
    .name = "tcltest::errorChannel",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 371,
};

// tcltest::errorFile
inline constexpr CommandDesc stdlib_tcltest__errorFile_cmd{
    .name = "tcltest::errorFile",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 372,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::getMatchingFiles
inline constexpr CommandDesc stdlib_tcltest__getMatchingFiles_cmd{
    .name = "tcltest::getMatchingFiles",
    .required_package = "tcltest",
    .hover_index = 373,
};

// tcltest::interpreter
inline constexpr CommandDesc stdlib_tcltest__interpreter_cmd{
    .name = "tcltest::interpreter",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 374,
};

// tcltest::limitConstraints
inline constexpr CommandDesc stdlib_tcltest__limitConstraints_cmd{
    .name = "tcltest::limitConstraints",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 375,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::loadFile
inline constexpr CommandDesc stdlib_tcltest__loadFile_cmd{
    .name = "tcltest::loadFile",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 376,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::loadScript
inline constexpr CommandDesc stdlib_tcltest__loadScript_cmd{
    .name = "tcltest::loadScript",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 377,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::loadTestedCommands
inline constexpr CommandDesc stdlib_tcltest__loadTestedCommands_cmd{
    .name = "tcltest::loadTestedCommands",
    .arity = {0, 0},
    .required_package = "tcltest",
    .hover_index = 378,
};

// tcltest::mainThread
inline constexpr CommandDesc stdlib_tcltest__mainThread_cmd{
    .name = "tcltest::mainThread",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 379,
};

// tcltest::makeDirectory
inline constexpr CommandDesc stdlib_tcltest__makeDirectory_cmd{
    .name = "tcltest::makeDirectory",
    .arity = {1, 2},
    .required_package = "tcltest",
    .hover_index = 380,
};

// tcltest::makeFile
inline constexpr CommandDesc stdlib_tcltest__makeFile_cmd{
    .name = "tcltest::makeFile",
    .arity = {2, 3},
    .required_package = "tcltest",
    .hover_index = 381,
};

// tcltest::match
inline constexpr CommandDesc stdlib_tcltest__match_cmd{
    .name = "tcltest::match",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 382,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::matchDirectories
inline constexpr CommandDesc stdlib_tcltest__matchDirectories_cmd{
    .name = "tcltest::matchDirectories",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 383,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::matchFiles
inline constexpr CommandDesc stdlib_tcltest__matchFiles_cmd{
    .name = "tcltest::matchFiles",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 384,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::normalizeMsg
inline constexpr CommandDesc stdlib_tcltest__normalizeMsg_cmd{
    .name = "tcltest::normalizeMsg",
    .arity = {1, 1},
    .required_package = "tcltest",
    .hover_index = 385,
    .deprecated_replacement = "tcltest::customMatch",
};

// tcltest::normalizePath
inline constexpr ArgPattern stdlib_tcltest__normalizePath_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr CommandDesc stdlib_tcltest__normalizePath_cmd{
    .name = "tcltest::normalizePath",
    .arity = {1, 1},
    .arg_patterns = stdlib_tcltest__normalizePath_args,
    .required_package = "tcltest",
    .hover_index = 386,
    .deprecated_replacement = "file normalize",
};

// tcltest::outputChannel
inline constexpr CommandDesc stdlib_tcltest__outputChannel_cmd{
    .name = "tcltest::outputChannel",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 387,
};

// tcltest::outputFile
inline constexpr CommandDesc stdlib_tcltest__outputFile_cmd{
    .name = "tcltest::outputFile",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 388,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::preserveCore
inline constexpr CommandDesc stdlib_tcltest__preserveCore_cmd{
    .name = "tcltest::preserveCore",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 389,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::removeDirectory
inline constexpr CommandDesc stdlib_tcltest__removeDirectory_cmd{
    .name = "tcltest::removeDirectory",
    .arity = {1, 2},
    .required_package = "tcltest",
    .hover_index = 390,
};

// tcltest::removeFile
inline constexpr CommandDesc stdlib_tcltest__removeFile_cmd{
    .name = "tcltest::removeFile",
    .arity = {1, 2},
    .required_package = "tcltest",
    .hover_index = 391,
};

// tcltest::restoreState
inline constexpr CommandDesc stdlib_tcltest__restoreState_cmd{
    .name = "tcltest::restoreState",
    .arity = {0, 0},
    .required_package = "tcltest",
    .hover_index = 392,
};

// tcltest::runAllTests
inline constexpr CommandDesc stdlib_tcltest__runAllTests_cmd{
    .name = "tcltest::runAllTests",
    .arity = {0, 0},
    .required_package = "tcltest",
    .hover_index = 393,
};

// tcltest::saveState
inline constexpr CommandDesc stdlib_tcltest__saveState_cmd{
    .name = "tcltest::saveState",
    .arity = {0, 0},
    .required_package = "tcltest",
    .hover_index = 394,
};

// tcltest::singleProcess
inline constexpr CommandDesc stdlib_tcltest__singleProcess_cmd{
    .name = "tcltest::singleProcess",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 395,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::skip
inline constexpr CommandDesc stdlib_tcltest__skip_cmd{
    .name = "tcltest::skip",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 396,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::skipDirectories
inline constexpr CommandDesc stdlib_tcltest__skipDirectories_cmd{
    .name = "tcltest::skipDirectories",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 397,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::skipFiles
inline constexpr CommandDesc stdlib_tcltest__skipFiles_cmd{
    .name = "tcltest::skipFiles",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 398,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::temporaryDirectory
inline constexpr CommandDesc stdlib_tcltest__temporaryDirectory_cmd{
    .name = "tcltest::temporaryDirectory",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 399,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::test
inline constexpr OptionDesc stdlib_tcltest__test_opts[] = {
    {.name = "-body", .takes_value = true, .value_hint = "script"},
    {.name = "-result", .takes_value = true},
    {.name = "-output", .takes_value = true},
    {.name = "-errorOutput", .takes_value = true},
    {.name = "-returnCodes", .takes_value = true},
    {.name = "-errorCode", .takes_value = true},
    {.name = "-match", .takes_value = true},
    {.name = "-setup", .takes_value = true, .value_hint = "script"},
    {.name = "-cleanup", .takes_value = true, .value_hint = "script"},
    {.name = "-constraints", .takes_value = true},
};

inline constexpr FormDesc stdlib_tcltest__test_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc stdlib_tcltest__test_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true, .writes = true},
};

inline constexpr CommandDesc stdlib_tcltest__test_cmd{
    .name = "tcltest::test",
    .arity = {2, std::numeric_limits<std::int32_t>::max()},
    .layout_resolver = LayoutResolver::TCLTEST,
    .options = stdlib_tcltest__test_opts,
    .forms = stdlib_tcltest__test_forms,
    .side_effect_hints = stdlib_tcltest__test_effects,
    .required_package = "tcltest",
    .hover_index = 400,
};

// tcltest::testConstraint
inline constexpr CommandDesc stdlib_tcltest__testConstraint_cmd{
    .name = "tcltest::testConstraint",
    .arity = {1, 2},
    .required_package = "tcltest",
    .hover_index = 401,
};

// tcltest::testsDirectory
inline constexpr CommandDesc stdlib_tcltest__testsDirectory_cmd{
    .name = "tcltest::testsDirectory",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 402,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::threadReap
inline constexpr CommandDesc stdlib_tcltest__threadReap_cmd{
    .name = "tcltest::threadReap",
    .arity = {0, 0},
    .required_package = "tcltest",
    .hover_index = 403,
};

// tcltest::verbose
inline constexpr CommandDesc stdlib_tcltest__verbose_cmd{
    .name = "tcltest::verbose",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 404,
    .deprecated_replacement = "tcltest::configure",
};

// tcltest::viewFile
inline constexpr CommandDesc stdlib_tcltest__viewFile_cmd{
    .name = "tcltest::viewFile",
    .arity = {1, 2},
    .required_package = "tcltest",
    .hover_index = 405,
};

// tcltest::workingDirectory
inline constexpr CommandDesc stdlib_tcltest__workingDirectory_cmd{
    .name = "tcltest::workingDirectory",
    .arity = {0, 1},
    .required_package = "tcltest",
    .hover_index = 406,
    .deprecated_replacement = "tcltest::configure",
};

// testapplylambda
inline constexpr CommandDesc stdlib_testapplylambda_cmd{
    .name = "testapplylambda",
    .warn_missing_import = false,
    .hover_index = 407,
};

// testappverifierpresent
inline constexpr CommandDesc stdlib_testappverifierpresent_cmd{
    .name = "testappverifierpresent",
    .warn_missing_import = false,
    .hover_index = 408,
};

// testasync
inline constexpr CommandDesc stdlib_testasync_cmd{
    .name = "testasync",
    .warn_missing_import = false,
    .hover_index = 409,
};

// testbigdata
inline constexpr CommandDesc stdlib_testbigdata_cmd{
    .name = "testbigdata",
    .warn_missing_import = false,
    .hover_index = 410,
};

// testbignumobj
inline constexpr CommandDesc stdlib_testbignumobj_cmd{
    .name = "testbignumobj",
    .warn_missing_import = false,
    .hover_index = 411,
};

// testbooleanobj
inline constexpr CommandDesc stdlib_testbooleanobj_cmd{
    .name = "testbooleanobj",
    .warn_missing_import = false,
    .hover_index = 412,
};

// testbumpinterpepoch
inline constexpr CommandDesc stdlib_testbumpinterpepoch_cmd{
    .name = "testbumpinterpepoch",
    .warn_missing_import = false,
    .hover_index = 413,
};

// testbytestring
inline constexpr CommandDesc stdlib_testbytestring_cmd{
    .name = "testbytestring",
    .warn_missing_import = false,
    .hover_index = 414,
};

// testchannel
inline constexpr CommandDesc stdlib_testchannel_cmd{
    .name = "testchannel",
    .warn_missing_import = false,
    .hover_index = 415,
};

// testchannelevent
inline constexpr CommandDesc stdlib_testchannelevent_cmd{
    .name = "testchannelevent",
    .warn_missing_import = false,
    .hover_index = 416,
};

// testcmdinfo
inline constexpr CommandDesc stdlib_testcmdinfo_cmd{
    .name = "testcmdinfo",
    .warn_missing_import = false,
    .hover_index = 417,
};

// testcmdtoken
inline constexpr CommandDesc stdlib_testcmdtoken_cmd{
    .name = "testcmdtoken",
    .warn_missing_import = false,
    .hover_index = 418,
};

// testcmdtrace
inline constexpr CommandDesc stdlib_testcmdtrace_cmd{
    .name = "testcmdtrace",
    .warn_missing_import = false,
    .hover_index = 419,
};

// testconcatobj
inline constexpr CommandDesc stdlib_testconcatobj_cmd{
    .name = "testconcatobj",
    .warn_missing_import = false,
    .hover_index = 420,
};

// testcpuid
inline constexpr CommandDesc stdlib_testcpuid_cmd{
    .name = "testcpuid",
    .warn_missing_import = false,
    .hover_index = 421,
};

// testcreatecommand
inline constexpr CommandDesc stdlib_testcreatecommand_cmd{
    .name = "testcreatecommand",
    .warn_missing_import = false,
    .hover_index = 422,
};

// testdcall
inline constexpr CommandDesc stdlib_testdcall_cmd{
    .name = "testdcall",
    .warn_missing_import = false,
    .hover_index = 423,
};

// testdel
inline constexpr CommandDesc stdlib_testdel_cmd{
    .name = "testdel",
    .warn_missing_import = false,
    .hover_index = 424,
};

// testdelassocdata
inline constexpr CommandDesc stdlib_testdelassocdata_cmd{
    .name = "testdelassocdata",
    .warn_missing_import = false,
    .hover_index = 425,
};

// testdoubledigits
inline constexpr CommandDesc stdlib_testdoubledigits_cmd{
    .name = "testdoubledigits",
    .warn_missing_import = false,
    .hover_index = 426,
};

// testdoubleobj
inline constexpr CommandDesc stdlib_testdoubleobj_cmd{
    .name = "testdoubleobj",
    .warn_missing_import = false,
    .hover_index = 427,
};

// testdstring
inline constexpr CommandDesc stdlib_testdstring_cmd{
    .name = "testdstring",
    .warn_missing_import = false,
    .hover_index = 428,
};

// testencoding
inline constexpr CommandDesc stdlib_testencoding_cmd{
    .name = "testencoding",
    .warn_missing_import = false,
    .hover_index = 429,
};

// testevalex
inline constexpr CommandDesc stdlib_testevalex_cmd{
    .name = "testevalex",
    .warn_missing_import = false,
    .hover_index = 430,
};

// testevalobjv
inline constexpr CommandDesc stdlib_testevalobjv_cmd{
    .name = "testevalobjv",
    .warn_missing_import = false,
    .hover_index = 431,
};

// testevent
inline constexpr CommandDesc stdlib_testevent_cmd{
    .name = "testevent",
    .warn_missing_import = false,
    .hover_index = 432,
};

// testexithandler
inline constexpr CommandDesc stdlib_testexithandler_cmd{
    .name = "testexithandler",
    .warn_missing_import = false,
    .hover_index = 433,
};

// testexitmainloop
inline constexpr CommandDesc stdlib_testexitmainloop_cmd{
    .name = "testexitmainloop",
    .warn_missing_import = false,
    .hover_index = 434,
};

// testexprdouble
inline constexpr CommandDesc stdlib_testexprdouble_cmd{
    .name = "testexprdouble",
    .warn_missing_import = false,
    .hover_index = 435,
};

// testexprdoubleobj
inline constexpr CommandDesc stdlib_testexprdoubleobj_cmd{
    .name = "testexprdoubleobj",
    .warn_missing_import = false,
    .hover_index = 436,
};

// testexprlong
inline constexpr CommandDesc stdlib_testexprlong_cmd{
    .name = "testexprlong",
    .warn_missing_import = false,
    .hover_index = 437,
};

// testexprlongobj
inline constexpr CommandDesc stdlib_testexprlongobj_cmd{
    .name = "testexprlongobj",
    .warn_missing_import = false,
    .hover_index = 438,
};

// testexprparser
inline constexpr CommandDesc stdlib_testexprparser_cmd{
    .name = "testexprparser",
    .warn_missing_import = false,
    .hover_index = 439,
};

// testexprstring
inline constexpr CommandDesc stdlib_testexprstring_cmd{
    .name = "testexprstring",
    .warn_missing_import = false,
    .hover_index = 440,
};

// testfevent
inline constexpr CommandDesc stdlib_testfevent_cmd{
    .name = "testfevent",
    .warn_missing_import = false,
    .hover_index = 441,
};

// testfile
inline constexpr CommandDesc stdlib_testfile_cmd{
    .name = "testfile",
    .warn_missing_import = false,
    .hover_index = 442,
};

// testfilelink
inline constexpr CommandDesc stdlib_testfilelink_cmd{
    .name = "testfilelink",
    .warn_missing_import = false,
    .hover_index = 443,
};

// testfilesystem
inline constexpr CommandDesc stdlib_testfilesystem_cmd{
    .name = "testfilesystem",
    .warn_missing_import = false,
    .hover_index = 444,
};

// testfindfirst
inline constexpr CommandDesc stdlib_testfindfirst_cmd{
    .name = "testfindfirst",
    .warn_missing_import = false,
    .hover_index = 445,
};

// testfindlast
inline constexpr CommandDesc stdlib_testfindlast_cmd{
    .name = "testfindlast",
    .warn_missing_import = false,
    .hover_index = 446,
};

// testfstildeexpand
inline constexpr CommandDesc stdlib_testfstildeexpand_cmd{
    .name = "testfstildeexpand",
    .warn_missing_import = false,
    .hover_index = 447,
};

// testgetassocdata
inline constexpr CommandDesc stdlib_testgetassocdata_cmd{
    .name = "testgetassocdata",
    .warn_missing_import = false,
    .hover_index = 448,
};

// testgetindexfromobjstruct
inline constexpr CommandDesc stdlib_testgetindexfromobjstruct_cmd{
    .name = "testgetindexfromobjstruct",
    .warn_missing_import = false,
    .hover_index = 449,
};

// testgetint
inline constexpr CommandDesc stdlib_testgetint_cmd{
    .name = "testgetint",
    .warn_missing_import = false,
    .hover_index = 450,
};

// testgetintforindex
inline constexpr CommandDesc stdlib_testgetintforindex_cmd{
    .name = "testgetintforindex",
    .warn_missing_import = false,
    .hover_index = 451,
};

// testgetplatform
inline constexpr CommandDesc stdlib_testgetplatform_cmd{
    .name = "testgetplatform",
    .warn_missing_import = false,
    .hover_index = 452,
};

// testgetunichar
inline constexpr CommandDesc stdlib_testgetunichar_cmd{
    .name = "testgetunichar",
    .warn_missing_import = false,
    .hover_index = 453,
};

// testgetvarfullname
inline constexpr CommandDesc stdlib_testgetvarfullname_cmd{
    .name = "testgetvarfullname",
    .warn_missing_import = false,
    .hover_index = 454,
};

// testhandlecount
inline constexpr CommandDesc stdlib_testhandlecount_cmd{
    .name = "testhandlecount",
    .warn_missing_import = false,
    .hover_index = 455,
};

// testhashsystemhash
inline constexpr CommandDesc stdlib_testhashsystemhash_cmd{
    .name = "testhashsystemhash",
    .warn_missing_import = false,
    .hover_index = 456,
};

// testindexobj
inline constexpr CommandDesc stdlib_testindexobj_cmd{
    .name = "testindexobj",
    .warn_missing_import = false,
    .hover_index = 457,
};

// testinterpdelete
inline constexpr CommandDesc stdlib_testinterpdelete_cmd{
    .name = "testinterpdelete",
    .warn_missing_import = false,
    .hover_index = 458,
};

// testinterpresolver
inline constexpr CommandDesc stdlib_testinterpresolver_cmd{
    .name = "testinterpresolver",
    .warn_missing_import = false,
    .hover_index = 459,
};

// testintobj
inline constexpr CommandDesc stdlib_testintobj_cmd{
    .name = "testintobj",
    .warn_missing_import = false,
    .hover_index = 460,
};

// testlink
inline constexpr CommandDesc stdlib_testlink_cmd{
    .name = "testlink",
    .warn_missing_import = false,
    .hover_index = 461,
};

// testlinkarray
inline constexpr CommandDesc stdlib_testlinkarray_cmd{
    .name = "testlinkarray",
    .warn_missing_import = false,
    .hover_index = 462,
};

// testlistobj
inline constexpr CommandDesc stdlib_testlistobj_cmd{
    .name = "testlistobj",
    .warn_missing_import = false,
    .hover_index = 463,
};

// testlistrep
inline constexpr CommandDesc stdlib_testlistrep_cmd{
    .name = "testlistrep",
    .warn_missing_import = false,
    .hover_index = 464,
};

// testlocale
inline constexpr CommandDesc stdlib_testlocale_cmd{
    .name = "testlocale",
    .warn_missing_import = false,
    .hover_index = 465,
};

// testlongsize
inline constexpr CommandDesc stdlib_testlongsize_cmd{
    .name = "testlongsize",
    .warn_missing_import = false,
    .hover_index = 466,
};

// testlutil
inline constexpr CommandDesc stdlib_testlutil_cmd{
    .name = "testlutil",
    .warn_missing_import = false,
    .hover_index = 467,
};

// testmainthread
inline constexpr CommandDesc stdlib_testmainthread_cmd{
    .name = "testmainthread",
    .warn_missing_import = false,
    .hover_index = 468,
};

// testmsb
inline constexpr CommandDesc stdlib_testmsb_cmd{
    .name = "testmsb",
    .warn_missing_import = false,
    .hover_index = 469,
};

// testnrelevels
inline constexpr CommandDesc stdlib_testnrelevels_cmd{
    .name = "testnrelevels",
    .warn_missing_import = false,
    .hover_index = 470,
};

// testnreunwind
inline constexpr CommandDesc stdlib_testnreunwind_cmd{
    .name = "testnreunwind",
    .warn_missing_import = false,
    .hover_index = 471,
};

// testnumutfchars
inline constexpr CommandDesc stdlib_testnumutfchars_cmd{
    .name = "testnumutfchars",
    .warn_missing_import = false,
    .hover_index = 472,
};

// testobj
inline constexpr CommandDesc stdlib_testobj_cmd{
    .name = "testobj",
    .warn_missing_import = false,
    .hover_index = 473,
};

// testpanic
inline constexpr CommandDesc stdlib_testpanic_cmd{
    .name = "testpanic",
    .warn_missing_import = false,
    .hover_index = 474,
};

// testparseargs
inline constexpr CommandDesc stdlib_testparseargs_cmd{
    .name = "testparseargs",
    .warn_missing_import = false,
    .hover_index = 475,
};

// testparser
inline constexpr CommandDesc stdlib_testparser_cmd{
    .name = "testparser",
    .warn_missing_import = false,
    .hover_index = 476,
};

// testparsevar
inline constexpr CommandDesc stdlib_testparsevar_cmd{
    .name = "testparsevar",
    .warn_missing_import = false,
    .hover_index = 477,
};

// testparsevarname
inline constexpr CommandDesc stdlib_testparsevarname_cmd{
    .name = "testparsevarname",
    .warn_missing_import = false,
    .hover_index = 478,
};

// testpreferstable
inline constexpr CommandDesc stdlib_testpreferstable_cmd{
    .name = "testpreferstable",
    .warn_missing_import = false,
    .hover_index = 479,
};

// testprint
inline constexpr CommandDesc stdlib_testprint_cmd{
    .name = "testprint",
    .warn_missing_import = false,
    .hover_index = 480,
};

// testpurebytesobj
inline constexpr CommandDesc stdlib_testpurebytesobj_cmd{
    .name = "testpurebytesobj",
    .warn_missing_import = false,
    .hover_index = 481,
};

// testregexp
inline constexpr CommandDesc stdlib_testregexp_cmd{
    .name = "testregexp",
    .warn_missing_import = false,
    .hover_index = 482,
};

// testreturn
inline constexpr CommandDesc stdlib_testreturn_cmd{
    .name = "testreturn",
    .warn_missing_import = false,
    .hover_index = 483,
};

// testsaveresult
inline constexpr CommandDesc stdlib_testsaveresult_cmd{
    .name = "testsaveresult",
    .warn_missing_import = false,
    .hover_index = 484,
};

// testservicemode
inline constexpr CommandDesc stdlib_testservicemode_cmd{
    .name = "testservicemode",
    .warn_missing_import = false,
    .hover_index = 485,
};

// testset2
inline constexpr CommandDesc stdlib_testset2_cmd{
    .name = "testset2",
    .warn_missing_import = false,
    .hover_index = 486,
};

// testsetassocdata
inline constexpr CommandDesc stdlib_testsetassocdata_cmd{
    .name = "testsetassocdata",
    .warn_missing_import = false,
    .hover_index = 487,
};

// testsetbytearraylength
inline constexpr CommandDesc stdlib_testsetbytearraylength_cmd{
    .name = "testsetbytearraylength",
    .warn_missing_import = false,
    .hover_index = 488,
};

// testseterr
inline constexpr CommandDesc stdlib_testseterr_cmd{
    .name = "testseterr",
    .warn_missing_import = false,
    .hover_index = 489,
};

// testseterrorcode
inline constexpr CommandDesc stdlib_testseterrorcode_cmd{
    .name = "testseterrorcode",
    .warn_missing_import = false,
    .hover_index = 490,
};

// testsetmainloop
inline constexpr CommandDesc stdlib_testsetmainloop_cmd{
    .name = "testsetmainloop",
    .warn_missing_import = false,
    .hover_index = 491,
};

// testsetnoerr
inline constexpr CommandDesc stdlib_testsetnoerr_cmd{
    .name = "testsetnoerr",
    .warn_missing_import = false,
    .hover_index = 492,
};

// testsetobjerrorcode
inline constexpr CommandDesc stdlib_testsetobjerrorcode_cmd{
    .name = "testsetobjerrorcode",
    .warn_missing_import = false,
    .hover_index = 493,
};

// testsetplatform
inline constexpr CommandDesc stdlib_testsetplatform_cmd{
    .name = "testsetplatform",
    .warn_missing_import = false,
    .hover_index = 494,
};

// testsimplefilesystem
inline constexpr CommandDesc stdlib_testsimplefilesystem_cmd{
    .name = "testsimplefilesystem",
    .warn_missing_import = false,
    .hover_index = 495,
};

// testsize
inline constexpr CommandDesc stdlib_testsize_cmd{
    .name = "testsize",
    .warn_missing_import = false,
    .hover_index = 496,
};

// testsocket
inline constexpr CommandDesc stdlib_testsocket_cmd{
    .name = "testsocket",
    .warn_missing_import = false,
    .hover_index = 497,
};

// teststaticlibrary
inline constexpr CommandDesc stdlib_teststaticlibrary_cmd{
    .name = "teststaticlibrary",
    .warn_missing_import = false,
    .hover_index = 498,
};

// teststaticpkg
inline constexpr CommandDesc stdlib_teststaticpkg_cmd{
    .name = "teststaticpkg",
    .warn_missing_import = false,
    .hover_index = 499,
};

// teststringbytes
inline constexpr CommandDesc stdlib_teststringbytes_cmd{
    .name = "teststringbytes",
    .warn_missing_import = false,
    .hover_index = 500,
};

// teststringobj
inline constexpr CommandDesc stdlib_teststringobj_cmd{
    .name = "teststringobj",
    .warn_missing_import = false,
    .hover_index = 501,
};

// testtranslatefilename
inline constexpr CommandDesc stdlib_testtranslatefilename_cmd{
    .name = "testtranslatefilename",
    .warn_missing_import = false,
    .hover_index = 502,
};

// testuniclass
inline constexpr CommandDesc stdlib_testuniclass_cmd{
    .name = "testuniclass",
    .warn_missing_import = false,
    .hover_index = 503,
};

// testupvar
inline constexpr CommandDesc stdlib_testupvar_cmd{
    .name = "testupvar",
    .warn_missing_import = false,
    .hover_index = 504,
};

// testutfnext
inline constexpr CommandDesc stdlib_testutfnext_cmd{
    .name = "testutfnext",
    .warn_missing_import = false,
    .hover_index = 505,
};

// testutfprev
inline constexpr CommandDesc stdlib_testutfprev_cmd{
    .name = "testutfprev",
    .warn_missing_import = false,
    .hover_index = 506,
};

// testwrongnumargs
inline constexpr CommandDesc stdlib_testwrongnumargs_cmd{
    .name = "testwrongnumargs",
    .warn_missing_import = false,
    .hover_index = 507,
};
