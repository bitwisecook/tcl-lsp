// ── irules_access commands ──

// ACCESS2::access2_proc
inline constexpr FormDesc irules_access_ACCESS2__access2_proc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS2__access2_proc_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS2__access2_proc_cmd{
    .name = "ACCESS2::access2_proc",
    .forms = irules_access_ACCESS2__access2_proc_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS2__access2_proc_effects,
    .hover_index = 733,
};

// ACCESS::acl
inline constexpr FormDesc irules_access_ACCESS__acl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__acl_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_access_ACCESS__acl_subcmds[] = {
    {.name = "result", .arity = {0, 0}, .pure = true, .hover_index = 734},
    {.name = "matched", .arity = {0, 0}, .pure = true, .hover_index = 735},
    {.name = "lookup", .arity = {0, 0}, .pure = true, .hover_index = 736},
    {.name = "eval", .arity = {1, 1}, .hover_index = 737},
};

inline constexpr CommandDesc irules_access_ACCESS__acl_cmd{
    .name = "ACCESS::acl",
    .subcommands = irules_access_ACCESS__acl_subcmds,
    .forms = irules_access_ACCESS__acl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__acl_effects,
    .hover_index = 738,
};

// ACCESS::disable
inline constexpr FormDesc irules_access_ACCESS__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__disable_cmd{
    .name = "ACCESS::disable",
    .forms = irules_access_ACCESS__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_access_ACCESS__disable_effects,
    .hover_index = 739,
};

// ACCESS::enable
inline constexpr FormDesc irules_access_ACCESS__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__enable_cmd{
    .name = "ACCESS::enable",
    .forms = irules_access_ACCESS__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_access_ACCESS__enable_effects,
    .hover_index = 740,
};

// ACCESS::ephemeral-auth
inline constexpr OptionDesc irules_access_ACCESS__ephemeral_auth_opts[] = {
    {.name = "-user", .takes_value = true, .detail = "Option -user."},
    {.name = "-auth_cfg", .takes_value = true, .detail = "Option -auth_cfg."},
    {.name = "-sid", .takes_value = true, .detail = "Option -sid."},
    {.name = "-password", .takes_value = true, .detail = "Option -password."},
    {.name = "-protocol", .takes_value = true, .detail = "Option -protocol."},
};

inline constexpr FormDesc irules_access_ACCESS__ephemeral_auth_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__ephemeral_auth_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__ephemeral_auth_cmd{
    .name = "ACCESS::ephemeral-auth",
    .options = irules_access_ACCESS__ephemeral_auth_opts,
    .forms = irules_access_ACCESS__ephemeral_auth_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__ephemeral_auth_effects,
    .hover_index = 741,
};

// ACCESS::flowid
inline constexpr FormDesc irules_access_ACCESS__flowid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__flowid_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__flowid_cmd{
    .name = "ACCESS::flowid",
    .forms = irules_access_ACCESS__flowid_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__flowid_effects,
    .hover_index = 742,
};

// ACCESS::log
inline constexpr FormDesc irules_access_ACCESS__log_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__log_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__log_cmd{
    .name = "ACCESS::log",
    .forms = irules_access_ACCESS__log_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__log_effects,
    .hover_index = 743,
};

// ACCESS::oauth
inline constexpr OptionDesc irules_access_ACCESS__oauth_opts[] = {
    {.name = "-header", .takes_value = true, .value_hint = "RAW_DATA", .detail = "Raw data for JOSE header section."},
    {.name = "-payload", .takes_value = true, .value_hint = "RAW_DATA", .detail = "Raw data for JWS payload."},
    {.name = "-key", .takes_value = true, .value_hint = "JWK_OBJECT", .detail = "JWK object for signing."},
    {.name = "-alg", .takes_value = true, .value_hint = "ALG", .detail = "Signing algorithm."},
    {.name = "-ignore-cert-expiry", .detail = "Allow expired certificate."},
};

inline constexpr FormDesc irules_access_ACCESS__oauth_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__oauth_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__oauth_cmd{
    .name = "ACCESS::oauth",
    .options = irules_access_ACCESS__oauth_opts,
    .forms = irules_access_ACCESS__oauth_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__oauth_effects,
    .hover_index = 744,
};

// ACCESS::perflow
inline constexpr FormDesc irules_access_ACCESS__perflow_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__perflow_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_access_ACCESS__perflow_subcmds[] = {
    {.name = "get", .arity = {1, 1}, .pure = true, .hover_index = 745},
    {.name = "set", .arity = {2, 2}, .mutator = true, .hover_index = 746},
};

inline constexpr CommandDesc irules_access_ACCESS__perflow_cmd{
    .name = "ACCESS::perflow",
    .subcommands = irules_access_ACCESS__perflow_subcmds,
    .forms = irules_access_ACCESS__perflow_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__perflow_effects,
    .hover_index = 747,
};

// ACCESS::policy
inline constexpr FormDesc irules_access_ACCESS__policy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__policy_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr OptionDesc irules_access_ACCESS__policy_sub_evaluate_opts[] = {
    {.name = "-sid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Session ID."},
};

inline constexpr OptionDesc irules_access_ACCESS__policy_sub_result_opts[] = {
    {.name = "-sid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Session ID."},
};

inline constexpr SubCmdDesc irules_access_ACCESS__policy_subcmds[] = {
    {.name = "agent_id", .arity = {0, 0}, .pure = true, .hover_index = 748},
    {.name = "evaluate", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_access_ACCESS__policy_sub_evaluate_opts, .hover_index = 749},
    {.name = "result", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_access_ACCESS__policy_sub_result_opts, .pure = true, .hover_index = 750},
    {.name = "uri", .arity = {0, 0}, .pure = true, .hover_index = 751},
};

inline constexpr CommandDesc irules_access_ACCESS__policy_cmd{
    .name = "ACCESS::policy",
    .subcommands = irules_access_ACCESS__policy_subcmds,
    .forms = irules_access_ACCESS__policy_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_access_ACCESS__policy_effects,
    .hover_index = 752,
};

// ACCESS::respond
inline constexpr OptionDesc irules_access_ACCESS__respond_opts[] = {
    {.name = "-ifile", .detail = "Option -ifile."},
    {.name = "-content", .takes_value = true, .detail = "Option -content."},
};

inline constexpr FormDesc irules_access_ACCESS__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__respond_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__respond_cmd{
    .name = "ACCESS::respond",
    .options = irules_access_ACCESS__respond_opts,
    .forms = irules_access_ACCESS__respond_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__respond_effects,
    .hover_index = 753,
};

// ACCESS::restrict_irule_events
inline constexpr FormDesc irules_access_ACCESS__restrict_irule_events_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__restrict_irule_events_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__restrict_irule_events_cmd{
    .name = "ACCESS::restrict_irule_events",
    .forms = irules_access_ACCESS__restrict_irule_events_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__restrict_irule_events_effects,
    .hover_index = 754,
};

// ACCESS::saml
inline constexpr FormDesc irules_access_ACCESS__saml_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__saml_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_access_ACCESS__saml_subcmds[] = {
    {.name = "authn", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 755},
    {.name = "assertion", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 756},
    {.name = "slo_req", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 757},
    {.name = "slo_resp", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 758},
};

inline constexpr CommandDesc irules_access_ACCESS__saml_cmd{
    .name = "ACCESS::saml",
    .subcommands = irules_access_ACCESS__saml_subcmds,
    .forms = irules_access_ACCESS__saml_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__saml_effects,
    .hover_index = 759,
};

// ACCESS::session
inline constexpr OptionDesc irules_access_ACCESS__session_opts[] = {
    {.name = "-flow", .detail = "Create a flow-scoped session."},
    {.name = "-timeout", .takes_value = true, .value_hint = "SECONDS", .detail = "Session timeout in seconds."},
    {.name = "-lifetime", .takes_value = true, .value_hint = "SECONDS", .detail = "Session lifetime in seconds."},
    {.name = "-sid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Session ID."},
    {.name = "-remaining", .takes_value = true, .detail = "Remaining time."},
    {.name = "-secure", .detail = "Access secure session data."},
    {.name = "-config", .detail = "Access config session data."},
    {.name = "-ssid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Sub-session ID."},
};

inline constexpr FormDesc irules_access_ACCESS__session_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__session_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr OptionDesc irules_access_ACCESS__session_sub_create_opts[] = {
    {.name = "-flow", .detail = "Create a flow-scoped session."},
    {.name = "-timeout", .takes_value = true, .value_hint = "SECONDS", .detail = "Session timeout in seconds."},
    {.name = "-lifetime", .takes_value = true, .value_hint = "SECONDS", .detail = "Session lifetime in seconds."},
};

inline constexpr OptionDesc irules_access_ACCESS__session_sub_modify_opts[] = {
    {.name = "-sid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Session ID."},
    {.name = "-timeout", .takes_value = true, .value_hint = "SECONDS", .detail = "Session timeout in seconds."},
    {.name = "-lifetime", .takes_value = true, .value_hint = "SECONDS", .detail = "Session lifetime in seconds."},
    {.name = "-remaining", .takes_value = true, .detail = "Remaining time."},
};

inline constexpr OptionDesc irules_access_ACCESS__session_sub_exists_opts[] = {
    {.name = "-sid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Session ID."},
    {.name = "-state_allow", .detail = "Check for allow state."},
    {.name = "-state_deny", .detail = "Check for deny state."},
    {.name = "-state_redirect", .detail = "Check for redirect state."},
    {.name = "-state_inprogress", .detail = "Check for in-progress state."},
};

inline constexpr OptionDesc irules_access_ACCESS__session_sub_data_opts[] = {
    {.name = "-sid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Session ID."},
    {.name = "-secure", .detail = "Access secure session data."},
    {.name = "-config", .detail = "Access config session data."},
    {.name = "-ssid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Sub-session ID."},
};

inline constexpr OptionDesc irules_access_ACCESS__session_sub_remove_opts[] = {
    {.name = "-sid", .takes_value = true, .value_hint = "SESSION_ID", .detail = "Session ID."},
};

inline constexpr SubCmdDesc irules_access_ACCESS__session_subcmds[] = {
    {.name = "create", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_access_ACCESS__session_sub_create_opts, .mutator = true, .hover_index = 760},
    {.name = "modify", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_access_ACCESS__session_sub_modify_opts, .mutator = true, .hover_index = 761},
    {.name = "exists", .arity = {0, 1}, .options = irules_access_ACCESS__session_sub_exists_opts, .pure = true, .hover_index = 762},
    {.name = "data", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .options = irules_access_ACCESS__session_sub_data_opts, .hover_index = 763},
    {.name = "remove", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_access_ACCESS__session_sub_remove_opts, .mutator = true, .hover_index = 764},
    {.name = "sid", .arity = {0, 0}, .pure = true, .hover_index = 765},
};

inline constexpr CommandDesc irules_access_ACCESS__session_cmd{
    .name = "ACCESS::session",
    .subcommands = irules_access_ACCESS__session_subcmds,
    .options = irules_access_ACCESS__session_opts,
    .forms = irules_access_ACCESS__session_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__session_effects,
    .hover_index = 766,
};

// ACCESS::user
inline constexpr FormDesc irules_access_ACCESS__user_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__user_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_access_ACCESS__user_subcmds[] = {
    {.name = "getkey", .arity = {1, 1}, .pure = true, .hover_index = 767},
    {.name = "getsid", .arity = {1, 1}, .pure = true, .hover_index = 768},
};

inline constexpr CommandDesc irules_access_ACCESS__user_cmd{
    .name = "ACCESS::user",
    .subcommands = irules_access_ACCESS__user_subcmds,
    .forms = irules_access_ACCESS__user_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__user_effects,
    .hover_index = 769,
};

// ACCESS::uuid
inline constexpr FormDesc irules_access_ACCESS__uuid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_access_ACCESS__uuid_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_access_ACCESS__uuid_cmd{
    .name = "ACCESS::uuid",
    .forms = irules_access_ACCESS__uuid_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_access_ACCESS__uuid_effects,
    .hover_index = 770,
};
