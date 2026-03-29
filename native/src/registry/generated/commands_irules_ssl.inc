// ── irules_ssl commands ──

// SSL::allow_dynamic_record_sizing
inline constexpr FormDesc irules_ssl_SSL__allow_dynamic_record_sizing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__allow_dynamic_record_sizing_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__allow_dynamic_record_sizing_cmd{
    .name = "SSL::allow_dynamic_record_sizing",
    .forms = irules_ssl_SSL__allow_dynamic_record_sizing_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__allow_dynamic_record_sizing_effects,
    .hover_index = 771,
};

// SSL::allow_nonssl
inline constexpr FormDesc irules_ssl_SSL__allow_nonssl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__allow_nonssl_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__allow_nonssl_cmd{
    .name = "SSL::allow_nonssl",
    .forms = irules_ssl_SSL__allow_nonssl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__allow_nonssl_effects,
    .hover_index = 772,
};

// SSL::alpn
inline constexpr FormDesc irules_ssl_SSL__alpn_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__alpn_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__alpn_cmd{
    .name = "SSL::alpn",
    .forms = irules_ssl_SSL__alpn_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__alpn_effects,
    .hover_index = 773,
};

// SSL::authenticate
inline constexpr FormDesc irules_ssl_SSL__authenticate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__authenticate_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_ssl_SSL__authenticate_subcmds[] = {
    {.name = "once", .arity = {0, 0}, .mutator = true, .hover_index = 774},
    {.name = "always", .arity = {0, 0}, .mutator = true, .hover_index = 775},
    {.name = "depth", .arity = {1, 1}, .mutator = true, .hover_index = 776},
};

inline constexpr CommandDesc irules_ssl_SSL__authenticate_cmd{
    .name = "SSL::authenticate",
    .subcommands = irules_ssl_SSL__authenticate_subcmds,
    .forms = irules_ssl_SSL__authenticate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__authenticate_effects,
    .hover_index = 777,
};

// SSL::c3d
inline constexpr FormDesc irules_ssl_SSL__c3d_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__c3d_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_ssl_SSL__c3d_subcmds[] = {
    {.name = "extension", .arity = {2, 2}, .mutator = true, .hover_index = 778},
    {.name = "cert", .arity = {1, 1}, .mutator = true, .hover_index = 779},
    {.name = "subject", .arity = {2, 2}, .mutator = true, .hover_index = 780},
};

inline constexpr CommandDesc irules_ssl_SSL__c3d_cmd{
    .name = "SSL::c3d",
    .subcommands = irules_ssl_SSL__c3d_subcmds,
    .forms = irules_ssl_SSL__c3d_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__c3d_effects,
    .hover_index = 781,
};

// SSL::cert
inline constexpr FormDesc irules_ssl_SSL__cert_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__cert_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr SubCmdDesc irules_ssl_SSL__cert_subcmds[] = {
    {.name = "count", .arity = {0, 0}, .pure = true, .hover_index = 782},
    {.name = "issuer", .arity = {1, 1}, .pure = true, .hover_index = 783},
    {.name = "mode", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 784},
};

inline constexpr CommandDesc irules_ssl_SSL__cert_cmd{
    .name = "SSL::cert",
    .subcommands = irules_ssl_SSL__cert_subcmds,
    .forms = irules_ssl_SSL__cert_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_ssl_SSL__cert_effects,
    .hover_index = 785,
};

// SSL::cert_constraint
inline constexpr FormDesc irules_ssl_SSL__cert_constraint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__cert_constraint_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__cert_constraint_cmd{
    .name = "SSL::cert_constraint",
    .forms = irules_ssl_SSL__cert_constraint_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__cert_constraint_effects,
    .hover_index = 786,
};

// SSL::cipher
inline constexpr FormDesc irules_ssl_SSL__cipher_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__cipher_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr SubCmdDesc irules_ssl_SSL__cipher_subcmds[] = {
    {.name = "bits", .arity = {0, 0}, .pure = true, .hover_index = 787},
    {.name = "name", .arity = {0, 0}, .pure = true, .hover_index = 788},
    {.name = "version", .arity = {0, 0}, .pure = true, .hover_index = 789},
    {.name = "clientlist", .arity = {0, 0}, .pure = true, .hover_index = 790},
};

inline constexpr CommandDesc irules_ssl_SSL__cipher_cmd{
    .name = "SSL::cipher",
    .subcommands = irules_ssl_SSL__cipher_subcmds,
    .forms = irules_ssl_SSL__cipher_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_ssl_SSL__cipher_effects,
    .hover_index = 791,
};

// SSL::clientrandom
inline constexpr FormDesc irules_ssl_SSL__clientrandom_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__clientrandom_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__clientrandom_cmd{
    .name = "SSL::clientrandom",
    .forms = irules_ssl_SSL__clientrandom_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__clientrandom_effects,
    .hover_index = 792,
};

// SSL::collect
inline constexpr FormDesc irules_ssl_SSL__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__collect_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__collect_cmd{
    .name = "SSL::collect",
    .forms = irules_ssl_SSL__collect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__collect_effects,
    .hover_index = 793,
};

// SSL::disable
inline constexpr FormDesc irules_ssl_SSL__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__disable_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__disable_cmd{
    .name = "SSL::disable",
    .forms = irules_ssl_SSL__disable_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_ssl_SSL__disable_effects,
    .hover_index = 794,
};

// SSL::enable
inline constexpr FormDesc irules_ssl_SSL__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__enable_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__enable_cmd{
    .name = "SSL::enable",
    .forms = irules_ssl_SSL__enable_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_ssl_SSL__enable_effects,
    .hover_index = 795,
};

// SSL::extensions
inline constexpr OptionDesc irules_ssl_SSL__extensions_opts[] = {
    {.name = "-index", .takes_value = true, .value_hint = "EXT_NUMBER", .detail = "Return extension at specified index."},
    {.name = "-type", .takes_value = true, .value_hint = "EXT_TYPE_VALUE", .detail = "Return extension matching specified type value."},
};

inline constexpr FormDesc irules_ssl_SSL__extensions_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__extensions_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_ssl_SSL__extensions_cmd{
    .name = "SSL::extensions",
    .options = irules_ssl_SSL__extensions_opts,
    .forms = irules_ssl_SSL__extensions_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_ssl_SSL__extensions_effects,
    .hover_index = 796,
};

// SSL::forward_proxy
inline constexpr FormDesc irules_ssl_SSL__forward_proxy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__forward_proxy_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_ssl_SSL__forward_proxy_subcmds[] = {
    {.name = "policy", .arity = {0, 1}, .mutator = true, .hover_index = 797},
    {.name = "cert", .arity = {0, 2}, .pure = true, .hover_index = 798},
    {.name = "verified_handshake", .arity = {0, 1}, .mutator = true, .hover_index = 799},
    {.name = "extension", .arity = {2, 2}, .mutator = true, .hover_index = 800},
};

inline constexpr CommandDesc irules_ssl_SSL__forward_proxy_cmd{
    .name = "SSL::forward_proxy",
    .subcommands = irules_ssl_SSL__forward_proxy_subcmds,
    .forms = irules_ssl_SSL__forward_proxy_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__forward_proxy_effects,
    .hover_index = 801,
};

// SSL::handshake
inline constexpr FormDesc irules_ssl_SSL__handshake_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__handshake_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__handshake_cmd{
    .name = "SSL::handshake",
    .forms = irules_ssl_SSL__handshake_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__handshake_effects,
    .hover_index = 802,
};

// SSL::is_renegotiation_secure
inline constexpr FormDesc irules_ssl_SSL__is_renegotiation_secure_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__is_renegotiation_secure_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__is_renegotiation_secure_cmd{
    .name = "SSL::is_renegotiation_secure",
    .forms = irules_ssl_SSL__is_renegotiation_secure_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__is_renegotiation_secure_effects,
    .hover_index = 803,
};

// SSL::maximum_record_size
inline constexpr FormDesc irules_ssl_SSL__maximum_record_size_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__maximum_record_size_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__maximum_record_size_cmd{
    .name = "SSL::maximum_record_size",
    .forms = irules_ssl_SSL__maximum_record_size_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__maximum_record_size_effects,
    .hover_index = 804,
};

// SSL::mode
inline constexpr FormDesc irules_ssl_SSL__mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__mode_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__mode_cmd{
    .name = "SSL::mode",
    .forms = irules_ssl_SSL__mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__mode_effects,
    .hover_index = 805,
};

// SSL::modssl_sessionid_headers
inline constexpr FormDesc irules_ssl_SSL__modssl_sessionid_headers_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__modssl_sessionid_headers_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__modssl_sessionid_headers_cmd{
    .name = "SSL::modssl_sessionid_headers",
    .forms = irules_ssl_SSL__modssl_sessionid_headers_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__modssl_sessionid_headers_effects,
    .hover_index = 806,
};

// SSL::nextproto
inline constexpr FormDesc irules_ssl_SSL__nextproto_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__nextproto_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__nextproto_cmd{
    .name = "SSL::nextproto",
    .forms = irules_ssl_SSL__nextproto_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__nextproto_effects,
    .hover_index = 807,
    .deprecated_replacement = "SSL::alpn",
};

// SSL::payload
inline constexpr FormDesc irules_ssl_SSL__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__payload_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__payload_cmd{
    .name = "SSL::payload",
    .forms = irules_ssl_SSL__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__payload_effects,
    .hover_index = 808,
};

// SSL::profile
inline constexpr FormDesc irules_ssl_SSL__profile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__profile_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__profile_cmd{
    .name = "SSL::profile",
    .forms = irules_ssl_SSL__profile_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__profile_effects,
    .hover_index = 809,
};

// SSL::release
inline constexpr FormDesc irules_ssl_SSL__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__release_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__release_cmd{
    .name = "SSL::release",
    .forms = irules_ssl_SSL__release_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__release_effects,
    .hover_index = 810,
};

// SSL::renegotiate
inline constexpr FormDesc irules_ssl_SSL__renegotiate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__renegotiate_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__renegotiate_cmd{
    .name = "SSL::renegotiate",
    .forms = irules_ssl_SSL__renegotiate_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__renegotiate_effects,
    .hover_index = 811,
};

// SSL::respond
inline constexpr FormDesc irules_ssl_SSL__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__respond_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__respond_cmd{
    .name = "SSL::respond",
    .forms = irules_ssl_SSL__respond_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .diagram_action = true,
    .side_effect_hints = irules_ssl_SSL__respond_effects,
    .hover_index = 812,
};

// SSL::secure_renegotiation
inline constexpr FormDesc irules_ssl_SSL__secure_renegotiation_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__secure_renegotiation_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__secure_renegotiation_cmd{
    .name = "SSL::secure_renegotiation",
    .forms = irules_ssl_SSL__secure_renegotiation_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__secure_renegotiation_effects,
    .hover_index = 813,
};

// SSL::session
inline constexpr FormDesc irules_ssl_SSL__session_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__session_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__session_cmd{
    .name = "SSL::session",
    .forms = irules_ssl_SSL__session_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__session_effects,
    .hover_index = 814,
};

// SSL::sessionid
inline constexpr FormDesc irules_ssl_SSL__sessionid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__sessionid_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__sessionid_cmd{
    .name = "SSL::sessionid",
    .forms = irules_ssl_SSL__sessionid_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__sessionid_effects,
    .hover_index = 815,
};

// SSL::sessionsecret
inline constexpr FormDesc irules_ssl_SSL__sessionsecret_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__sessionsecret_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__sessionsecret_cmd{
    .name = "SSL::sessionsecret",
    .forms = irules_ssl_SSL__sessionsecret_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__sessionsecret_effects,
    .hover_index = 816,
};

// SSL::sessionticket
inline constexpr FormDesc irules_ssl_SSL__sessionticket_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__sessionticket_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__sessionticket_cmd{
    .name = "SSL::sessionticket",
    .forms = irules_ssl_SSL__sessionticket_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__sessionticket_effects,
    .hover_index = 817,
};

// SSL::sni
inline constexpr FormDesc irules_ssl_SSL__sni_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__sni_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_ssl_SSL__sni_subcmds[] = {
    {.name = "name", .arity = {0, 0}, .pure = true, .hover_index = 818},
    {.name = "required", .arity = {0, 0}, .pure = true, .hover_index = 819},
};

inline constexpr CommandDesc irules_ssl_SSL__sni_cmd{
    .name = "SSL::sni",
    .subcommands = irules_ssl_SSL__sni_subcmds,
    .forms = irules_ssl_SSL__sni_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__sni_effects,
    .hover_index = 820,
};

// SSL::tls13_secret
inline constexpr FormDesc irules_ssl_SSL__tls13_secret_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__tls13_secret_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_ssl_SSL__tls13_secret_subcmds[] = {
    {.name = "client", .arity = {1, 1}, .pure = true, .hover_index = 821},
    {.name = "server", .arity = {1, 1}, .pure = true, .hover_index = 822},
};

inline constexpr CommandDesc irules_ssl_SSL__tls13_secret_cmd{
    .name = "SSL::tls13_secret",
    .subcommands = irules_ssl_SSL__tls13_secret_subcmds,
    .forms = irules_ssl_SSL__tls13_secret_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_ssl_SSL__tls13_secret_effects,
    .hover_index = 823,
};

// SSL::unclean_shutdown
inline constexpr FormDesc irules_ssl_SSL__unclean_shutdown_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__unclean_shutdown_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__unclean_shutdown_cmd{
    .name = "SSL::unclean_shutdown",
    .forms = irules_ssl_SSL__unclean_shutdown_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__unclean_shutdown_effects,
    .hover_index = 824,
};

// SSL::verify_result
inline constexpr FormDesc irules_ssl_SSL__verify_result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_ssl_SSL__verify_result_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_ssl_SSL__verify_result_cmd{
    .name = "SSL::verify_result",
    .forms = irules_ssl_SSL__verify_result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_ssl_SSL__verify_result_effects,
    .hover_index = 825,
};
