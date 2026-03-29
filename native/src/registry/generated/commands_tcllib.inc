// ── tcllib commands ──

// base64::decode
inline constexpr FormDesc tcllib_base64__decode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_base64__decode_cmd{
    .name = "base64::decode",
    .arity = {1, 1},
    .forms = tcllib_base64__decode_forms,
    .required_package = "base64",
    .tcllib_package = "base64",
    .hover_index = 508,
};

// base64::encode
inline constexpr FormDesc tcllib_base64__encode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_base64__encode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_base64__encode_cmd{
    .name = "base64::encode",
    .arity = {1, 5},
    .forms = tcllib_base64__encode_forms,
    .side_effect_hints = tcllib_base64__encode_effects,
    .required_package = "base64",
    .tcllib_package = "base64",
    .hover_index = 509,
};

// cmdline::getopt
inline constexpr FormDesc tcllib_cmdline__getopt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_cmdline__getopt_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_cmdline__getopt_cmd{
    .name = "cmdline::getopt",
    .arity = {4, 4},
    .forms = tcllib_cmdline__getopt_forms,
    .side_effect_hints = tcllib_cmdline__getopt_effects,
    .required_package = "cmdline",
    .tcllib_package = "cmdline",
    .hover_index = 510,
};

// cmdline::getoptions
inline constexpr FormDesc tcllib_cmdline__getoptions_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_cmdline__getoptions_cmd{
    .name = "cmdline::getoptions",
    .arity = {2, 3},
    .forms = tcllib_cmdline__getoptions_forms,
    .required_package = "cmdline",
    .tcllib_package = "cmdline",
    .hover_index = 511,
};

// cmdline::usage
inline constexpr FormDesc tcllib_cmdline__usage_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_cmdline__usage_cmd{
    .name = "cmdline::usage",
    .arity = {1, 2},
    .forms = tcllib_cmdline__usage_forms,
    .pure = true,
    .required_package = "cmdline",
    .tcllib_package = "cmdline",
    .hover_index = 512,
};

// csv::join
inline constexpr FormDesc tcllib_csv__join_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_csv__join_cmd{
    .name = "csv::join",
    .arity = {1, 3},
    .forms = tcllib_csv__join_forms,
    .required_package = "csv",
    .tcllib_package = "csv",
    .hover_index = 513,
};

// csv::read
inline constexpr FormDesc tcllib_csv__read_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_csv__read_cmd{
    .name = "csv::read",
    .arity = {2, 4},
    .forms = tcllib_csv__read_forms,
    .required_package = "csv",
    .tcllib_package = "csv",
    .hover_index = 514,
};

// csv::report
inline constexpr FormDesc tcllib_csv__report_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_csv__report_cmd{
    .name = "csv::report",
    .arity = {2, 3},
    .forms = tcllib_csv__report_forms,
    .required_package = "csv",
    .tcllib_package = "csv",
    .hover_index = 515,
};

// csv::split
inline constexpr FormDesc tcllib_csv__split_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_csv__split_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_csv__split_cmd{
    .name = "csv::split",
    .arity = {1, 4},
    .forms = tcllib_csv__split_forms,
    .side_effect_hints = tcllib_csv__split_effects,
    .required_package = "csv",
    .tcllib_package = "csv",
    .hover_index = 516,
};

// dns::address
inline constexpr FormDesc tcllib_dns__address_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_dns__address_cmd{
    .name = "dns::address",
    .arity = {1, 1},
    .forms = tcllib_dns__address_forms,
    .required_package = "dns",
    .tcllib_package = "dns",
    .hover_index = 517,
};

// dns::cleanup
inline constexpr FormDesc tcllib_dns__cleanup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_dns__cleanup_cmd{
    .name = "dns::cleanup",
    .arity = {1, 1},
    .forms = tcllib_dns__cleanup_forms,
    .required_package = "dns",
    .tcllib_package = "dns",
    .hover_index = 518,
};

// dns::name
inline constexpr FormDesc tcllib_dns__name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_dns__name_cmd{
    .name = "dns::name",
    .arity = {1, 1},
    .forms = tcllib_dns__name_forms,
    .required_package = "dns",
    .tcllib_package = "dns",
    .hover_index = 519,
};

// dns::resolve
inline constexpr FormDesc tcllib_dns__resolve_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_dns__resolve_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcllib_dns__resolve_cmd{
    .name = "dns::resolve",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_dns__resolve_forms,
    .side_effect_hints = tcllib_dns__resolve_effects,
    .required_package = "dns",
    .tcllib_package = "dns",
    .hover_index = 520,
};

// fileutil::cat
inline constexpr FormDesc tcllib_fileutil__cat_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_fileutil__cat_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcllib_fileutil__cat_cmd{
    .name = "fileutil::cat",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_fileutil__cat_forms,
    .side_effect_hints = tcllib_fileutil__cat_effects,
    .required_package = "fileutil",
    .tcllib_package = "fileutil",
    .hover_index = 521,
};

// fileutil::tempdir
inline constexpr FormDesc tcllib_fileutil__tempdir_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_fileutil__tempdir_cmd{
    .name = "fileutil::tempdir",
    .arity = {0, 0},
    .forms = tcllib_fileutil__tempdir_forms,
    .required_package = "fileutil",
    .tcllib_package = "fileutil",
    .hover_index = 522,
};

// fileutil::tempfile
inline constexpr FormDesc tcllib_fileutil__tempfile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_fileutil__tempfile_cmd{
    .name = "fileutil::tempfile",
    .arity = {0, 1},
    .forms = tcllib_fileutil__tempfile_forms,
    .required_package = "fileutil",
    .tcllib_package = "fileutil",
    .hover_index = 523,
};

// fileutil::writeFile
inline constexpr FormDesc tcllib_fileutil__writeFile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_fileutil__writeFile_cmd{
    .name = "fileutil::writeFile",
    .arity = {2, 4},
    .forms = tcllib_fileutil__writeFile_forms,
    .required_package = "fileutil",
    .tcllib_package = "fileutil",
    .hover_index = 524,
};

// html::html_entities
inline constexpr FormDesc tcllib_html__html_entities_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_html__html_entities_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_html__html_entities_cmd{
    .name = "html::html_entities",
    .arity = {1, 1},
    .forms = tcllib_html__html_entities_forms,
    .pure = true,
    .side_effect_hints = tcllib_html__html_entities_effects,
    .required_package = "html",
    .tcllib_package = "html",
    .hover_index = 525,
};

// html::tagstrip
inline constexpr FormDesc tcllib_html__tagstrip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_html__tagstrip_cmd{
    .name = "html::tagstrip",
    .arity = {1, 1},
    .forms = tcllib_html__tagstrip_forms,
    .pure = true,
    .required_package = "html",
    .tcllib_package = "html",
    .hover_index = 526,
};

// ip::contract
inline constexpr FormDesc tcllib_ip__contract_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_ip__contract_cmd{
    .name = "ip::contract",
    .arity = {1, 1},
    .forms = tcllib_ip__contract_forms,
    .pure = true,
    .required_package = "ip",
    .tcllib_package = "ip",
    .hover_index = 527,
};

// ip::equal
inline constexpr FormDesc tcllib_ip__equal_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_ip__equal_cmd{
    .name = "ip::equal",
    .arity = {2, 2},
    .forms = tcllib_ip__equal_forms,
    .pure = true,
    .required_package = "ip",
    .tcllib_package = "ip",
    .hover_index = 528,
};

// ip::normalize
inline constexpr FormDesc tcllib_ip__normalize_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_ip__normalize_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_ip__normalize_cmd{
    .name = "ip::normalize",
    .arity = {1, 1},
    .forms = tcllib_ip__normalize_forms,
    .pure = true,
    .side_effect_hints = tcllib_ip__normalize_effects,
    .required_package = "ip",
    .tcllib_package = "ip",
    .hover_index = 529,
};

// ip::prefix
inline constexpr FormDesc tcllib_ip__prefix_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_ip__prefix_cmd{
    .name = "ip::prefix",
    .arity = {1, 1},
    .forms = tcllib_ip__prefix_forms,
    .pure = true,
    .required_package = "ip",
    .tcllib_package = "ip",
    .hover_index = 530,
};

// ip::version
inline constexpr FormDesc tcllib_ip__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_ip__version_cmd{
    .name = "ip::version",
    .arity = {1, 1},
    .forms = tcllib_ip__version_forms,
    .pure = true,
    .required_package = "ip",
    .tcllib_package = "ip",
    .hover_index = 531,
};

// json::dict2json
inline constexpr FormDesc tcllib_json__dict2json_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_json__dict2json_cmd{
    .name = "json::dict2json",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_json__dict2json_forms,
    .required_package = "json",
    .tcllib_package = "json",
    .hover_index = 532,
};

// json::json2dict
inline constexpr FormDesc tcllib_json__json2dict_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_json__json2dict_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_json__json2dict_cmd{
    .name = "json::json2dict",
    .arity = {1, 1},
    .forms = tcllib_json__json2dict_forms,
    .side_effect_hints = tcllib_json__json2dict_effects,
    .required_package = "json",
    .tcllib_package = "json",
    .hover_index = 533,
};

// logger::init
inline constexpr FormDesc tcllib_logger__init_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_logger__init_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr CommandDesc tcllib_logger__init_cmd{
    .name = "logger::init",
    .arity = {1, 1},
    .forms = tcllib_logger__init_forms,
    .side_effect_hints = tcllib_logger__init_effects,
    .required_package = "logger",
    .tcllib_package = "logger",
    .hover_index = 534,
};

// logger::levels
inline constexpr FormDesc tcllib_logger__levels_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_logger__levels_cmd{
    .name = "logger::levels",
    .arity = {0, 0},
    .forms = tcllib_logger__levels_forms,
    .pure = true,
    .required_package = "logger",
    .tcllib_package = "logger",
    .hover_index = 535,
};

// logger::servicecmd
inline constexpr FormDesc tcllib_logger__servicecmd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_logger__servicecmd_cmd{
    .name = "logger::servicecmd",
    .arity = {1, 1},
    .forms = tcllib_logger__servicecmd_forms,
    .required_package = "logger",
    .tcllib_package = "logger",
    .hover_index = 536,
};

// logger::services
inline constexpr FormDesc tcllib_logger__services_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_logger__services_cmd{
    .name = "logger::services",
    .arity = {0, 0},
    .forms = tcllib_logger__services_forms,
    .required_package = "logger",
    .tcllib_package = "logger",
    .hover_index = 537,
};

// math::statistics::basic-stats
inline constexpr FormDesc tcllib_math__statistics__basic_stats_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_math__statistics__basic_stats_cmd{
    .name = "math::statistics::basic-stats",
    .arity = {1, 1},
    .forms = tcllib_math__statistics__basic_stats_forms,
    .pure = true,
    .required_package = "math::statistics",
    .tcllib_package = "math::statistics",
    .hover_index = 538,
};

// math::statistics::mean
inline constexpr FormDesc tcllib_math__statistics__mean_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_math__statistics__mean_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_math__statistics__mean_cmd{
    .name = "math::statistics::mean",
    .arity = {1, 1},
    .forms = tcllib_math__statistics__mean_forms,
    .pure = true,
    .side_effect_hints = tcllib_math__statistics__mean_effects,
    .required_package = "math::statistics",
    .tcllib_package = "math::statistics",
    .hover_index = 539,
};

// math::statistics::median
inline constexpr FormDesc tcllib_math__statistics__median_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_math__statistics__median_cmd{
    .name = "math::statistics::median",
    .arity = {1, 1},
    .forms = tcllib_math__statistics__median_forms,
    .pure = true,
    .required_package = "math::statistics",
    .tcllib_package = "math::statistics",
    .hover_index = 540,
};

// math::statistics::quantiles
inline constexpr FormDesc tcllib_math__statistics__quantiles_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_math__statistics__quantiles_cmd{
    .name = "math::statistics::quantiles",
    .arity = {2, 2},
    .forms = tcllib_math__statistics__quantiles_forms,
    .pure = true,
    .required_package = "math::statistics",
    .tcllib_package = "math::statistics",
    .hover_index = 541,
};

// math::statistics::stdev
inline constexpr FormDesc tcllib_math__statistics__stdev_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_math__statistics__stdev_cmd{
    .name = "math::statistics::stdev",
    .arity = {1, 1},
    .forms = tcllib_math__statistics__stdev_forms,
    .pure = true,
    .required_package = "math::statistics",
    .tcllib_package = "math::statistics",
    .hover_index = 542,
};

// math::statistics::var
inline constexpr FormDesc tcllib_math__statistics__var_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_math__statistics__var_cmd{
    .name = "math::statistics::var",
    .arity = {1, 1},
    .forms = tcllib_math__statistics__var_forms,
    .pure = true,
    .required_package = "math::statistics",
    .tcllib_package = "math::statistics",
    .hover_index = 543,
};

// md5::md5
inline constexpr FormDesc tcllib_md5__md5_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_md5__md5_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_md5__md5_cmd{
    .name = "md5::md5",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_md5__md5_forms,
    .pure = true,
    .side_effect_hints = tcllib_md5__md5_effects,
    .required_package = "md5",
    .tcllib_package = "md5",
    .hover_index = 544,
};

// mime::finalize
inline constexpr FormDesc tcllib_mime__finalize_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_mime__finalize_cmd{
    .name = "mime::finalize",
    .arity = {1, 3},
    .forms = tcllib_mime__finalize_forms,
    .required_package = "mime",
    .tcllib_package = "mime",
    .hover_index = 545,
};

// mime::getbody
inline constexpr FormDesc tcllib_mime__getbody_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_mime__getbody_cmd{
    .name = "mime::getbody",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_mime__getbody_forms,
    .required_package = "mime",
    .tcllib_package = "mime",
    .hover_index = 546,
};

// mime::getproperty
inline constexpr FormDesc tcllib_mime__getproperty_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_mime__getproperty_cmd{
    .name = "mime::getproperty",
    .arity = {1, 2},
    .forms = tcllib_mime__getproperty_forms,
    .required_package = "mime",
    .tcllib_package = "mime",
    .hover_index = 547,
};

// mime::initialize
inline constexpr FormDesc tcllib_mime__initialize_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_mime__initialize_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_mime__initialize_cmd{
    .name = "mime::initialize",
    .forms = tcllib_mime__initialize_forms,
    .side_effect_hints = tcllib_mime__initialize_effects,
    .required_package = "mime",
    .tcllib_package = "mime",
    .hover_index = 548,
};

// sha1::sha1
inline constexpr FormDesc tcllib_sha1__sha1_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_sha1__sha1_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_sha1__sha1_cmd{
    .name = "sha1::sha1",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_sha1__sha1_forms,
    .pure = true,
    .side_effect_hints = tcllib_sha1__sha1_effects,
    .required_package = "sha1",
    .tcllib_package = "sha1",
    .hover_index = 549,
};

// sha2::sha256
inline constexpr FormDesc tcllib_sha2__sha256_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_sha2__sha256_cmd{
    .name = "sha2::sha256",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_sha2__sha256_forms,
    .pure = true,
    .required_package = "sha2",
    .tcllib_package = "sha2",
    .hover_index = 550,
};

// smtp::sendmessage
inline constexpr FormDesc tcllib_smtp__sendmessage_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_smtp__sendmessage_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr CommandDesc tcllib_smtp__sendmessage_cmd{
    .name = "smtp::sendmessage",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_smtp__sendmessage_forms,
    .side_effect_hints = tcllib_smtp__sendmessage_effects,
    .required_package = "smtp",
    .tcllib_package = "smtp",
    .hover_index = 551,
};

// snit::method
inline constexpr FormDesc tcllib_snit__method_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_snit__method_cmd{
    .name = "snit::method",
    .arity = {4, 4},
    .forms = tcllib_snit__method_forms,
    .required_package = "snit",
    .tcllib_package = "snit",
    .hover_index = 552,
};

// snit::type
inline constexpr FormDesc tcllib_snit__type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_snit__type_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true},
};

inline constexpr CommandDesc tcllib_snit__type_cmd{
    .name = "snit::type",
    .arity = {2, 2},
    .forms = tcllib_snit__type_forms,
    .never_inline_body = true,
    .creates_dynamic_barrier = true,
    .side_effect_hints = tcllib_snit__type_effects,
    .required_package = "snit",
    .tcllib_package = "snit",
    .hover_index = 553,
};

// snit::typemethod
inline constexpr FormDesc tcllib_snit__typemethod_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_snit__typemethod_cmd{
    .name = "snit::typemethod",
    .arity = {4, 4},
    .forms = tcllib_snit__typemethod_forms,
    .required_package = "snit",
    .tcllib_package = "snit",
    .hover_index = 554,
};

// snit::widget
inline constexpr FormDesc tcllib_snit__widget_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_snit__widget_cmd{
    .name = "snit::widget",
    .arity = {2, 2},
    .forms = tcllib_snit__widget_forms,
    .never_inline_body = true,
    .creates_dynamic_barrier = true,
    .required_package = "snit",
    .tcllib_package = "snit",
    .hover_index = 555,
};

// snit::widgetadaptor
inline constexpr FormDesc tcllib_snit__widgetadaptor_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_snit__widgetadaptor_cmd{
    .name = "snit::widgetadaptor",
    .arity = {2, 2},
    .forms = tcllib_snit__widgetadaptor_forms,
    .never_inline_body = true,
    .creates_dynamic_barrier = true,
    .required_package = "snit",
    .tcllib_package = "snit",
    .hover_index = 556,
};

// struct::list
inline constexpr FormDesc tcllib_struct__list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_struct__list_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr SubCmdDesc tcllib_struct__list_subcmds[] = {
    {.name = "assign", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 557},
    {.name = "dbJoin", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 558},
    {.name = "dbJoinKeyed", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 559},
    {.name = "equal", .arity = {2, 2}, .hover_index = 560},
    {.name = "filter", .arity = {2, 2}, .hover_index = 561},
    {.name = "filterfor", .arity = {3, 3}, .hover_index = 562},
    {.name = "flatten", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 563},
    {.name = "fold", .arity = {3, 3}, .hover_index = 564},
    {.name = "foreachperm", .arity = {3, 3}, .hover_index = 565},
    {.name = "iota", .arity = {1, 1}, .hover_index = 566},
    {.name = "lcsInvert", .arity = {3, 3}, .hover_index = 567},
    {.name = "longestCommonSubsequence", .arity = {2, 3}, .hover_index = 568},
    {.name = "map", .arity = {2, 2}, .hover_index = 569},
    {.name = "mapfor", .arity = {3, 3}, .hover_index = 570},
    {.name = "repeat", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 571},
    {.name = "reverse", .arity = {1, 1}, .hover_index = 572},
    {.name = "shift", .arity = {1, 1}, .hover_index = 573},
    {.name = "shuffle", .arity = {1, 1}, .hover_index = 574},
    {.name = "swap", .arity = {3, 3}, .hover_index = 575},
};

inline constexpr CommandDesc tcllib_struct__list_cmd{
    .name = "struct::list",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcllib_struct__list_subcmds,
    .forms = tcllib_struct__list_forms,
    .pure = true,
    .side_effect_hints = tcllib_struct__list_effects,
    .required_package = "struct::list",
    .tcllib_package = "struct::list",
    .hover_index = 576,
};

// struct::queue
inline constexpr FormDesc tcllib_struct__queue_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_struct__queue_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcllib_struct__queue_cmd{
    .name = "struct::queue",
    .forms = tcllib_struct__queue_forms,
    .side_effect_hints = tcllib_struct__queue_effects,
    .required_package = "struct::queue",
    .tcllib_package = "struct::queue",
    .hover_index = 577,
};

// struct::set
inline constexpr FormDesc tcllib_struct__set_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_struct__set_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr SubCmdDesc tcllib_struct__set_subcmds[] = {
    {.name = "contains", .arity = {2, 2}, .hover_index = 578},
    {.name = "difference", .arity = {2, 2}, .hover_index = 579},
    {.name = "empty", .arity = {1, 1}, .hover_index = 580},
    {.name = "equal", .arity = {2, 2}, .hover_index = 581},
    {.name = "exclude", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 582},
    {.name = "include", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .hover_index = 583},
    {.name = "intersect", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .hover_index = 584},
    {.name = "subsetof", .arity = {2, 2}, .hover_index = 585},
    {.name = "symdiff", .arity = {2, 2}, .hover_index = 586},
    {.name = "union", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .hover_index = 587},
};

inline constexpr CommandDesc tcllib_struct__set_cmd{
    .name = "struct::set",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcllib_struct__set_subcmds,
    .forms = tcllib_struct__set_forms,
    .pure = true,
    .side_effect_hints = tcllib_struct__set_effects,
    .required_package = "struct::set",
    .tcllib_package = "struct::set",
    .hover_index = 588,
};

// struct::stack
inline constexpr FormDesc tcllib_struct__stack_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_struct__stack_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true},
};

inline constexpr CommandDesc tcllib_struct__stack_cmd{
    .name = "struct::stack",
    .forms = tcllib_struct__stack_forms,
    .side_effect_hints = tcllib_struct__stack_effects,
    .required_package = "struct::stack",
    .tcllib_package = "struct::stack",
    .hover_index = 589,
};

// textutil::adjust
inline constexpr FormDesc tcllib_textutil__adjust_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_textutil__adjust_cmd{
    .name = "textutil::adjust",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .forms = tcllib_textutil__adjust_forms,
    .required_package = "textutil",
    .tcllib_package = "textutil",
    .hover_index = 590,
};

// textutil::indent
inline constexpr FormDesc tcllib_textutil__indent_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_textutil__indent_cmd{
    .name = "textutil::indent",
    .arity = {2, 3},
    .forms = tcllib_textutil__indent_forms,
    .required_package = "textutil",
    .tcllib_package = "textutil",
    .hover_index = 591,
};

// textutil::splitx
inline constexpr FormDesc tcllib_textutil__splitx_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_textutil__splitx_cmd{
    .name = "textutil::splitx",
    .arity = {1, 2},
    .forms = tcllib_textutil__splitx_forms,
    .required_package = "textutil",
    .tcllib_package = "textutil",
    .hover_index = 592,
};

// textutil::trim
inline constexpr FormDesc tcllib_textutil__trim_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_textutil__trim_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_textutil__trim_cmd{
    .name = "textutil::trim",
    .arity = {1, 2},
    .forms = tcllib_textutil__trim_forms,
    .side_effect_hints = tcllib_textutil__trim_effects,
    .required_package = "textutil",
    .tcllib_package = "textutil",
    .hover_index = 593,
};

// textutil::undent
inline constexpr FormDesc tcllib_textutil__undent_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_textutil__undent_cmd{
    .name = "textutil::undent",
    .arity = {1, 1},
    .forms = tcllib_textutil__undent_forms,
    .required_package = "textutil",
    .tcllib_package = "textutil",
    .hover_index = 594,
};

// uri::join
inline constexpr FormDesc tcllib_uri__join_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_uri__join_cmd{
    .name = "uri::join",
    .forms = tcllib_uri__join_forms,
    .required_package = "uri",
    .tcllib_package = "uri",
    .hover_index = 595,
};

// uri::resolve
inline constexpr FormDesc tcllib_uri__resolve_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_uri__resolve_cmd{
    .name = "uri::resolve",
    .arity = {2, 2},
    .forms = tcllib_uri__resolve_forms,
    .required_package = "uri",
    .tcllib_package = "uri",
    .hover_index = 596,
};

// uri::split
inline constexpr FormDesc tcllib_uri__split_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_uri__split_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_uri__split_cmd{
    .name = "uri::split",
    .arity = {1, 2},
    .forms = tcllib_uri__split_forms,
    .side_effect_hints = tcllib_uri__split_effects,
    .required_package = "uri",
    .tcllib_package = "uri",
    .hover_index = 597,
};

// uuid::uuid
inline constexpr FormDesc tcllib_uuid__uuid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_uuid__uuid_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr SubCmdDesc tcllib_uuid__uuid_subcmds[] = {
    {.name = "equal", .arity = {2, 2}},
    {.name = "generate", .arity = {0, 0}},
};

inline constexpr CommandDesc tcllib_uuid__uuid_cmd{
    .name = "uuid::uuid",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = tcllib_uuid__uuid_subcmds,
    .forms = tcllib_uuid__uuid_forms,
    .side_effect_hints = tcllib_uuid__uuid_effects,
    .required_package = "uuid",
    .tcllib_package = "uuid",
    .hover_index = 598,
};

// yaml::dict2yaml
inline constexpr FormDesc tcllib_yaml__dict2yaml_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_yaml__dict2yaml_cmd{
    .name = "yaml::dict2yaml",
    .arity = {1, 3},
    .forms = tcllib_yaml__dict2yaml_forms,
    .required_package = "yaml",
    .tcllib_package = "yaml",
    .hover_index = 599,
};

// yaml::huddle2yaml
inline constexpr FormDesc tcllib_yaml__huddle2yaml_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc tcllib_yaml__huddle2yaml_cmd{
    .name = "yaml::huddle2yaml",
    .arity = {1, 3},
    .forms = tcllib_yaml__huddle2yaml_forms,
    .required_package = "yaml",
    .tcllib_package = "yaml",
    .hover_index = 600,
};

// yaml::yaml2dict
inline constexpr FormDesc tcllib_yaml__yaml2dict_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc tcllib_yaml__yaml2dict_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc tcllib_yaml__yaml2dict_cmd{
    .name = "yaml::yaml2dict",
    .arity = {1, 2},
    .forms = tcllib_yaml__yaml2dict_forms,
    .side_effect_hints = tcllib_yaml__yaml2dict_effects,
    .required_package = "yaml",
    .tcllib_package = "yaml",
    .hover_index = 601,
};
