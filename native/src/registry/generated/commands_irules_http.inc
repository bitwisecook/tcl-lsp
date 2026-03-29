// ── irules_http commands ──

// HTTP2::active
inline constexpr FormDesc irules_http_HTTP2__active_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__active_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__active_cmd{
    .name = "HTTP2::active",
    .forms = irules_http_HTTP2__active_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP2__active_effects,
    .hover_index = 637,
};

// HTTP2::concurrency
inline constexpr FormDesc irules_http_HTTP2__concurrency_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__concurrency_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__concurrency_cmd{
    .name = "HTTP2::concurrency",
    .forms = irules_http_HTTP2__concurrency_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP2__concurrency_effects,
    .hover_index = 638,
};

// HTTP2::disable
inline constexpr FormDesc irules_http_HTTP2__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__disable_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__disable_cmd{
    .name = "HTTP2::disable",
    .forms = irules_http_HTTP2__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_HTTP2__disable_effects,
    .hover_index = 639,
};

// HTTP2::disconnect
inline constexpr FormDesc irules_http_HTTP2__disconnect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__disconnect_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__disconnect_cmd{
    .name = "HTTP2::disconnect",
    .forms = irules_http_HTTP2__disconnect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP2__disconnect_effects,
    .hover_index = 640,
};

// HTTP2::enable
inline constexpr FormDesc irules_http_HTTP2__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__enable_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__enable_cmd{
    .name = "HTTP2::enable",
    .forms = irules_http_HTTP2__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_HTTP2__enable_effects,
    .hover_index = 641,
};

// HTTP2::header
inline constexpr FormDesc irules_http_HTTP2__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__header_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__header_cmd{
    .name = "HTTP2::header",
    .forms = irules_http_HTTP2__header_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP2__header_effects,
    .hover_index = 642,
};

// HTTP2::push
inline constexpr OptionDesc irules_http_HTTP2__push_opts[] = {
    {.name = "-priority", .takes_value = true, .value_hint = "PRIORITY", .detail = "Push priority number."},
    {.name = "-content", .takes_value = true, .value_hint = "CONTENT", .detail = "Pushed response content."},
    {.name = "-ifile", .takes_value = true, .value_hint = "IFILE_OBJ", .detail = "Serve content from iFile."},
    {.name = "-noserver", .detail = "Suppress \"Server: BigIP\" header."},
    {.name = "-nohost", .detail = "Disable Host header requirement."},
};

inline constexpr FormDesc irules_http_HTTP2__push_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__push_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_http_HTTP2__push_cmd{
    .name = "HTTP2::push",
    .options = irules_http_HTTP2__push_opts,
    .forms = irules_http_HTTP2__push_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_http_HTTP2__push_effects,
    .hover_index = 643,
};

// HTTP2::requests
inline constexpr FormDesc irules_http_HTTP2__requests_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__requests_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__requests_cmd{
    .name = "HTTP2::requests",
    .forms = irules_http_HTTP2__requests_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP2__requests_effects,
    .hover_index = 644,
};

// HTTP2::stream
inline constexpr FormDesc irules_http_HTTP2__stream_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__stream_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__stream_cmd{
    .name = "HTTP2::stream",
    .forms = irules_http_HTTP2__stream_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP2__stream_effects,
    .hover_index = 645,
};

// HTTP2::version
inline constexpr FormDesc irules_http_HTTP2__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP2__version_effects[] = {
    {.target = SideEffectTarget::HTTP2_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP2__version_cmd{
    .name = "HTTP2::version",
    .forms = irules_http_HTTP2__version_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP2__version_effects,
    .hover_index = 646,
};

// HTTP::class
inline constexpr SideEffectDesc irules_http_HTTP__class_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__class_cmd{
    .name = "HTTP::class",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_HTTP__class_effects,
    .hover_index = 647,
    .deprecated_replacement = "CLASSIFY::application",
};

// HTTP::close
inline constexpr FormDesc irules_http_HTTP__close_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__close_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__close_cmd{
    .name = "HTTP::close",
    .forms = irules_http_HTTP__close_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__close_effects,
    .hover_index = 648,
};

// HTTP::collect
inline constexpr FormDesc irules_http_HTTP__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__collect_effects[] = {
    {.target = SideEffectTarget::HTTP_BODY, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__collect_cmd{
    .name = "HTTP::collect",
    .forms = irules_http_HTTP__collect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__collect_effects,
    .hover_index = 649,
};

// HTTP::cookie
inline constexpr FormDesc irules_http_HTTP__cookie_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__cookie_effects[] = {
    {.target = SideEffectTarget::HTTP_COOKIE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_version_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_path_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_domain_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_ports_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_maxage_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_expires_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_comment_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_secure_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_commenturl_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr FormDesc irules_http_HTTP__cookie_sub_httponly_forms[] = {
    {.kind = FormKind::GETTER, .arity = {1, 1}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {2, 2}, .mutator = true},
};

inline constexpr SubCmdDesc irules_http_HTTP__cookie_subcmds[] = {
    {.name = "names", .arity = {0, 0}, .hover_index = 650},
    {.name = "count", .arity = {0, 0}, .hover_index = 651},
    {.name = "value", .arity = {1, 1}, .hover_index = 652},
    {.name = "version", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_version_forms, .hover_index = 653},
    {.name = "path", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_path_forms, .hover_index = 654},
    {.name = "domain", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_domain_forms, .hover_index = 655},
    {.name = "ports", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_ports_forms, .hover_index = 656},
    {.name = "insert", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .mutator = true, .credential_arg = 2, .hover_index = 657},
    {.name = "remove", .arity = {1, 1}, .mutator = true, .hover_index = 658},
    {.name = "sanitize", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .hover_index = 659},
    {.name = "exists", .arity = {1, 1}, .hover_index = 660},
    {.name = "maxage", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_maxage_forms, .hover_index = 661},
    {.name = "expires", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_expires_forms, .hover_index = 662},
    {.name = "comment", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_comment_forms, .hover_index = 663},
    {.name = "secure", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_secure_forms, .hover_index = 664},
    {.name = "commenturl", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_commenturl_forms, .hover_index = 665},
    {.name = "encrypt", .arity = {2, 2}, .hover_index = 666},
    {.name = "decrypt", .arity = {2, 2}, .hover_index = 667},
    {.name = "httponly", .arity = {1, 2}, .forms = irules_http_HTTP__cookie_sub_httponly_forms, .hover_index = 668},
    {.name = "attribute", .arity = {1, 3}, .hover_index = 669},
    {.name = "replace", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .mutator = true, .credential_arg = 2, .hover_index = 670},
};

inline constexpr CommandDesc irules_http_HTTP__cookie_cmd{
    .name = "HTTP::cookie",
    .subcommands = irules_http_HTTP__cookie_subcmds,
    .forms = irules_http_HTTP__cookie_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__cookie_effects,
    .taint_hints = {.output_sink_code = "IRULE3002"},
    .hover_index = 671,
};

// HTTP::disable
inline constexpr FormDesc irules_http_HTTP__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__disable_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__disable_cmd{
    .name = "HTTP::disable",
    .forms = irules_http_HTTP__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__disable_effects,
    .hover_index = 672,
};

// HTTP::enable
inline constexpr FormDesc irules_http_HTTP__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__enable_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__enable_cmd{
    .name = "HTTP::enable",
    .forms = irules_http_HTTP__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_HTTP__enable_effects,
    .hover_index = 673,
};

// HTTP::fallback
inline constexpr FormDesc irules_http_HTTP__fallback_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__fallback_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__fallback_cmd{
    .name = "HTTP::fallback",
    .forms = irules_http_HTTP__fallback_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__fallback_effects,
    .hover_index = 674,
};

// HTTP::has_responded
inline constexpr FormDesc irules_http_HTTP__has_responded_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__has_responded_effects[] = {
    {.target = SideEffectTarget::RESPONSE_COMMIT, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__has_responded_cmd{
    .name = "HTTP::has_responded",
    .forms = irules_http_HTTP__has_responded_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__has_responded_effects,
    .hover_index = 675,
};

// HTTP::header
inline constexpr FormDesc irules_http_HTTP__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__header_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr FormDesc irules_http_HTTP__header_sub_lws_forms[] = {
    {.kind = FormKind::GETTER, .arity = {0, 0}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {1, 1}, .mutator = true},
};

inline constexpr SubCmdDesc irules_http_HTTP__header_subcmds[] = {
    {.name = "at", .arity = {1, 1}, .hover_index = 676},
    {.name = "count", .arity = {0, 1}, .hover_index = 677},
    {.name = "exists", .arity = {1, 1}, .hover_index = 678},
    {.name = "insert", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .mutator = true, .credential_arg = 2, .hover_index = 679},
    {.name = "insert_modssl_fields", .arity = {0, 0}, .mutator = true, .hover_index = 680},
    {.name = "is_keepalive", .arity = {0, 0}, .hover_index = 681},
    {.name = "is_redirect", .arity = {0, 0}, .hover_index = 682},
    {.name = "lws", .arity = {0, 1}, .forms = irules_http_HTTP__header_sub_lws_forms, .hover_index = 683},
    {.name = "names", .arity = {0, 0}, .hover_index = 684},
    {.name = "remove", .arity = {1, 1}, .mutator = true, .hover_index = 685},
    {.name = "replace", .arity = {2, std::numeric_limits<std::int32_t>::max()}, .mutator = true, .credential_arg = 2, .hover_index = 686},
    {.name = "sanitize", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .mutator = true, .hover_index = 687},
    {.name = "value", .arity = {1, 1}, .hover_index = 688},
    {.name = "values", .arity = {1, 1}, .hover_index = 689},
};

inline constexpr CommandDesc irules_http_HTTP__header_cmd{
    .name = "HTTP::header",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .subcommands = irules_http_HTTP__header_subcmds,
    .forms = irules_http_HTTP__header_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__header_effects,
    .taint_hints = {.output_sink_code = "IRULE3002"},
    .hover_index = 690,
};

// HTTP::host
inline constexpr FormDesc irules_http_HTTP__host_forms[] = {
    {.kind = FormKind::GETTER, .arity = {0, 0}, .pure = true},
};

inline constexpr SideEffectDesc irules_http_HTTP__host_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_http_HTTP__host_cmd{
    .name = "HTTP::host",
    .forms = irules_http_HTTP__host_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__host_effects,
    .hover_index = 691,
};

// HTTP::hsts
inline constexpr FormDesc irules_http_HTTP__hsts_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__hsts_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_http_HTTP__hsts_cmd{
    .name = "HTTP::hsts",
    .forms = irules_http_HTTP__hsts_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__hsts_effects,
    .hover_index = 692,
};

// HTTP::is_keepalive
inline constexpr FormDesc irules_http_HTTP__is_keepalive_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__is_keepalive_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__is_keepalive_cmd{
    .name = "HTTP::is_keepalive",
    .forms = irules_http_HTTP__is_keepalive_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__is_keepalive_effects,
    .hover_index = 693,
};

// HTTP::is_redirect
inline constexpr FormDesc irules_http_HTTP__is_redirect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__is_redirect_effects[] = {
    {.target = SideEffectTarget::HTTP_STATUS, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__is_redirect_cmd{
    .name = "HTTP::is_redirect",
    .forms = irules_http_HTTP__is_redirect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__is_redirect_effects,
    .hover_index = 694,
};

// HTTP::method
inline constexpr FormDesc irules_http_HTTP__method_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__method_effects[] = {
    {.target = SideEffectTarget::HTTP_METHOD, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_http_HTTP__method_cmd{
    .name = "HTTP::method",
    .forms = irules_http_HTTP__method_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_http_HTTP__method_effects,
    .hover_index = 695,
};

// HTTP::passthrough_reason
inline constexpr FormDesc irules_http_HTTP__passthrough_reason_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__passthrough_reason_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__passthrough_reason_cmd{
    .name = "HTTP::passthrough_reason",
    .forms = irules_http_HTTP__passthrough_reason_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__passthrough_reason_effects,
    .hover_index = 696,
};

// HTTP::password
inline constexpr FormDesc irules_http_HTTP__password_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__password_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__password_cmd{
    .name = "HTTP::password",
    .forms = irules_http_HTTP__password_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__password_effects,
    .hover_index = 697,
};

// HTTP::path
inline constexpr OptionDesc irules_http_HTTP__path_opts[] = {
    {.name = "-normalized", .detail = "Return path normalized for consistent comparisons."},
};

inline constexpr FormDesc irules_http_HTTP__path_forms[] = {
    {.kind = FormKind::GETTER, .arity = {0, 0}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {1, 1}, .mutator = true},
};

inline constexpr CommandDesc irules_http_HTTP__path_cmd{
    .name = "HTTP::path",
    .options = irules_http_HTTP__path_opts,
    .forms = irules_http_HTTP__path_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .diagram_action = true,
    .hover_index = 698,
};

// HTTP::payload
inline constexpr FormDesc irules_http_HTTP__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__payload_effects[] = {
    {.target = SideEffectTarget::HTTP_BODY, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_http_HTTP__payload_cmd{
    .name = "HTTP::payload",
    .forms = irules_http_HTTP__payload_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_http_HTTP__payload_effects,
    .hover_index = 699,
};

// HTTP::proxy
inline constexpr FormDesc irules_http_HTTP__proxy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__proxy_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_http_HTTP__proxy_subcmds[] = {
    {.name = "enable", .arity = {0, 0}, .mutator = true, .hover_index = 700},
    {.name = "disable", .arity = {0, 0}, .mutator = true, .hover_index = 701},
    {.name = "uri-rewrite", .arity = {1, 1}, .mutator = true, .hover_index = 702},
    {.name = "addr", .arity = {0, 0}, .pure = true, .hover_index = 703},
    {.name = "port", .arity = {0, 0}, .pure = true, .hover_index = 704},
    {.name = "rtdom", .arity = {0, 0}, .pure = true, .hover_index = 705},
    {.name = "exists", .arity = {0, 0}, .pure = true, .hover_index = 706},
    {.name = "iptuple", .arity = {0, 0}, .pure = true, .hover_index = 707},
    {.name = "chain", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .mutator = true, .hover_index = 708},
};

inline constexpr CommandDesc irules_http_HTTP__proxy_cmd{
    .name = "HTTP::proxy",
    .subcommands = irules_http_HTTP__proxy_subcmds,
    .forms = irules_http_HTTP__proxy_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__proxy_effects,
    .hover_index = 709,
};

// HTTP::query
inline constexpr OptionDesc irules_http_HTTP__query_opts[] = {
    {.name = "-normalized", .detail = "Return query normalized for consistent comparisons."},
};

inline constexpr FormDesc irules_http_HTTP__query_forms[] = {
    {.kind = FormKind::GETTER, .arity = {0, 0}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {1, 1}, .mutator = true},
};

inline constexpr CommandDesc irules_http_HTTP__query_cmd{
    .name = "HTTP::query",
    .options = irules_http_HTTP__query_opts,
    .forms = irules_http_HTTP__query_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .hover_index = 710,
};

// HTTP::redirect
inline constexpr FormDesc irules_http_HTTP__redirect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__redirect_effects[] = {
    {.target = SideEffectTarget::RESPONSE_COMMIT, .reads = true, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_http_HTTP__redirect_cmd{
    .name = "HTTP::redirect",
    .forms = irules_http_HTTP__redirect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__redirect_effects,
    .taint_hints = {.output_sink_code = "IRULE3004"},
    .hover_index = 711,
};

// HTTP::reject_reason
inline constexpr FormDesc irules_http_HTTP__reject_reason_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__reject_reason_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__reject_reason_cmd{
    .name = "HTTP::reject_reason",
    .forms = irules_http_HTTP__reject_reason_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__reject_reason_effects,
    .hover_index = 712,
};

// HTTP::release
inline constexpr FormDesc irules_http_HTTP__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__release_effects[] = {
    {.target = SideEffectTarget::HTTP_BODY, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__release_cmd{
    .name = "HTTP::release",
    .forms = irules_http_HTTP__release_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__release_effects,
    .hover_index = 713,
};

// HTTP::request
inline constexpr FormDesc irules_http_HTTP__request_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__request_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_http_HTTP__request_cmd{
    .name = "HTTP::request",
    .forms = irules_http_HTTP__request_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_http_HTTP__request_effects,
    .hover_index = 714,
};

// HTTP::request_num
inline constexpr FormDesc irules_http_HTTP__request_num_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__request_num_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_http_HTTP__request_num_cmd{
    .name = "HTTP::request_num",
    .arity = {0, 0},
    .forms = irules_http_HTTP__request_num_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_http_HTTP__request_num_effects,
    .hover_index = 715,
};

// HTTP::respond
inline constexpr OptionDesc irules_http_HTTP__respond_opts[] = {
    {.name = "-version", .takes_value = true, .value_hint = "VERSION", .detail = "Set HTTP version (1.0, 1.1, auto)."},
    {.name = "-content", .takes_value = true, .value_hint = "CONTENT", .detail = "Response body content."},
    {.name = "-ifile", .takes_value = true, .value_hint = "IFILE_OBJ", .detail = "Serve content from iFile."},
    {.name = "-noserver", .detail = "Suppress \"Server: BigIP\" header."},
    {.name = "-reset", .detail = "Reset server-side connection."},
};

inline constexpr FormDesc irules_http_HTTP__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__respond_effects[] = {
    {.target = SideEffectTarget::RESPONSE_COMMIT, .reads = true, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_http_HTTP__respond_cmd{
    .name = "HTTP::respond",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = irules_http_HTTP__respond_opts,
    .forms = irules_http_HTTP__respond_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__respond_effects,
    .taint_hints = {.output_sink_code = "IRULE3001"},
    .hover_index = 716,
};

// HTTP::response
inline constexpr FormDesc irules_http_HTTP__response_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__response_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__response_cmd{
    .name = "HTTP::response",
    .forms = irules_http_HTTP__response_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__response_effects,
    .hover_index = 717,
};

// HTTP::retry
inline constexpr OptionDesc irules_http_HTTP__retry_opts[] = {
    {.name = "-reset", .takes_value = true, .detail = "Option -reset."},
};

inline constexpr FormDesc irules_http_HTTP__retry_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__retry_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_http_HTTP__retry_cmd{
    .name = "HTTP::retry",
    .options = irules_http_HTTP__retry_opts,
    .forms = irules_http_HTTP__retry_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .diagram_action = true,
    .side_effect_hints = irules_http_HTTP__retry_effects,
    .hover_index = 718,
};

// HTTP::status
inline constexpr FormDesc irules_http_HTTP__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__status_effects[] = {
    {.target = SideEffectTarget::HTTP_STATUS, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_http_HTTP__status_cmd{
    .name = "HTTP::status",
    .forms = irules_http_HTTP__status_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_http_HTTP__status_effects,
    .hover_index = 719,
};

// HTTP::uri
inline constexpr OptionDesc irules_http_HTTP__uri_opts[] = {
    {.name = "-normalized", .detail = "Return URI normalized for consistent comparisons."},
};

inline constexpr FormDesc irules_http_HTTP__uri_forms[] = {
    {.kind = FormKind::GETTER, .arity = {0, 0}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {1, 1}, .mutator = true},
};

inline constexpr CommandDesc irules_http_HTTP__uri_cmd{
    .name = "HTTP::uri",
    .options = irules_http_HTTP__uri_opts,
    .forms = irules_http_HTTP__uri_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .diagram_action = true,
    .hover_index = 720,
};

// HTTP::username
inline constexpr FormDesc irules_http_HTTP__username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__username_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTP__username_cmd{
    .name = "HTTP::username",
    .forms = irules_http_HTTP__username_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_http_HTTP__username_effects,
    .hover_index = 721,
};

// HTTP::version
inline constexpr OptionDesc irules_http_HTTP__version_opts[] = {
    {.name = "-string", .takes_value = true, .detail = "Option -string."},
};

inline constexpr FormDesc irules_http_HTTP__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTP__version_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_http_HTTP__version_cmd{
    .name = "HTTP::version",
    .options = irules_http_HTTP__version_opts,
    .forms = irules_http_HTTP__version_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_http_HTTP__version_effects,
    .hover_index = 722,
};

// HTTPLOG::disable
inline constexpr FormDesc irules_http_HTTPLOG__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTPLOG__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTPLOG__disable_cmd{
    .name = "HTTPLOG::disable",
    .forms = irules_http_HTTPLOG__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_HTTPLOG__disable_effects,
    .hover_index = 723,
};

// HTTPLOG::enable
inline constexpr FormDesc irules_http_HTTPLOG__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_HTTPLOG__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_HTTPLOG__enable_cmd{
    .name = "HTTPLOG::enable",
    .forms = irules_http_HTTPLOG__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_HTTPLOG__enable_effects,
    .hover_index = 724,
};

// http_client_ip
inline constexpr FormDesc irules_http_http_client_ip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_client_ip_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_http_http_client_ip_cmd{
    .name = "http_client_ip",
    .arity = {0, 1},
    .forms = irules_http_http_client_ip_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_http_client_ip_effects,
    .hover_index = 725,
};

// http_content_len_max
inline constexpr FormDesc irules_http_http_content_len_max_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_content_len_max_effects[] = {
    {.target = SideEffectTarget::HTTP_BODY, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_http_content_len_max_cmd{
    .name = "http_content_len_max",
    .arity = {0, 1},
    .forms = irules_http_http_content_len_max_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_http_http_content_len_max_effects,
    .hover_index = 726,
};

// http_cookie
inline constexpr FormDesc irules_http_http_cookie_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_cookie_effects[] = {
    {.target = SideEffectTarget::HTTP_COOKIE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_http_cookie_cmd{
    .name = "http_cookie",
    .forms = irules_http_http_cookie_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_http_cookie_effects,
    .hover_index = 727,
    .deprecated_replacement = "HTTP::cookie",
};

// http_header
inline constexpr FormDesc irules_http_http_header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_header_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_http_header_cmd{
    .name = "http_header",
    .forms = irules_http_http_header_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_http_header_effects,
    .hover_index = 728,
    .deprecated_replacement = "HTTP::header",
};

// http_host
inline constexpr FormDesc irules_http_http_host_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_host_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_http_http_host_cmd{
    .name = "http_host",
    .forms = irules_http_http_host_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_http_host_effects,
    .hover_index = 729,
    .deprecated_replacement = "HTTP::host",
};

// http_method
inline constexpr FormDesc irules_http_http_method_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_method_effects[] = {
    {.target = SideEffectTarget::HTTP_METHOD, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_http_method_cmd{
    .name = "http_method",
    .forms = irules_http_http_method_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_http_method_effects,
    .hover_index = 730,
    .deprecated_replacement = "HTTP::method",
};

// http_uri
inline constexpr FormDesc irules_http_http_uri_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_uri_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_http_uri_cmd{
    .name = "http_uri",
    .forms = irules_http_http_uri_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_http_uri_effects,
    .hover_index = 731,
    .deprecated_replacement = "HTTP::uri",
};

// http_version
inline constexpr FormDesc irules_http_http_version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_http_http_version_effects[] = {
    {.target = SideEffectTarget::HTTP_STATUS, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_http_http_version_cmd{
    .name = "http_version",
    .forms = irules_http_http_version_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_http_http_version_effects,
    .hover_index = 732,
    .deprecated_replacement = "HTTP::version",
};
