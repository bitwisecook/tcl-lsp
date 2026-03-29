// ── irules_misc commands ──

// AAA::acct_result
inline constexpr FormDesc irules_misc_AAA__acct_result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AAA__acct_result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AAA__acct_result_cmd{
    .name = "AAA::acct_result",
    .forms = irules_misc_AAA__acct_result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AAA__acct_result_effects,
    .hover_index = 826,
};

// AAA::acct_send
inline constexpr FormDesc irules_misc_AAA__acct_send_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AAA__acct_send_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AAA__acct_send_cmd{
    .name = "AAA::acct_send",
    .forms = irules_misc_AAA__acct_send_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AAA__acct_send_effects,
    .hover_index = 827,
};

// AAA::auth_result
inline constexpr FormDesc irules_misc_AAA__auth_result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AAA__auth_result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AAA__auth_result_cmd{
    .name = "AAA::auth_result",
    .forms = irules_misc_AAA__auth_result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AAA__auth_result_effects,
    .hover_index = 828,
};

// AAA::auth_send
inline constexpr FormDesc irules_misc_AAA__auth_send_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AAA__auth_send_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AAA__auth_send_cmd{
    .name = "AAA::auth_send",
    .forms = irules_misc_AAA__auth_send_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AAA__auth_send_effects,
    .hover_index = 829,
};

// ACL::action
inline constexpr FormDesc irules_misc_ACL__action_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ACL__action_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ACL__action_cmd{
    .name = "ACL::action",
    .forms = irules_misc_ACL__action_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ACL__action_effects,
    .hover_index = 830,
};

// ACL::eval
inline constexpr OptionDesc irules_misc_ACL__eval_opts[] = {
    {.name = "-l7", .takes_value = true, .detail = "Option -l7."},
};

inline constexpr FormDesc irules_misc_ACL__eval_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ACL__eval_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ACL__eval_cmd{
    .name = "ACL::eval",
    .options = irules_misc_ACL__eval_opts,
    .forms = irules_misc_ACL__eval_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ACL__eval_effects,
    .hover_index = 831,
};

// ADAPT::allow
inline constexpr FormDesc irules_misc_ADAPT__allow_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__allow_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__allow_cmd{
    .name = "ADAPT::allow",
    .forms = irules_misc_ADAPT__allow_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__allow_effects,
    .hover_index = 832,
};

// ADAPT::context_create
inline constexpr FormDesc irules_misc_ADAPT__context_create_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__context_create_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__context_create_cmd{
    .name = "ADAPT::context_create",
    .forms = irules_misc_ADAPT__context_create_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__context_create_effects,
    .hover_index = 833,
};

// ADAPT::context_current
inline constexpr FormDesc irules_misc_ADAPT__context_current_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__context_current_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__context_current_cmd{
    .name = "ADAPT::context_current",
    .forms = irules_misc_ADAPT__context_current_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__context_current_effects,
    .hover_index = 834,
};

// ADAPT::context_delete_all
inline constexpr FormDesc irules_misc_ADAPT__context_delete_all_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__context_delete_all_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__context_delete_all_cmd{
    .name = "ADAPT::context_delete_all",
    .forms = irules_misc_ADAPT__context_delete_all_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__context_delete_all_effects,
    .hover_index = 835,
};

// ADAPT::context_name
inline constexpr FormDesc irules_misc_ADAPT__context_name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__context_name_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__context_name_cmd{
    .name = "ADAPT::context_name",
    .forms = irules_misc_ADAPT__context_name_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__context_name_effects,
    .hover_index = 836,
};

// ADAPT::context_static
inline constexpr FormDesc irules_misc_ADAPT__context_static_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__context_static_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__context_static_cmd{
    .name = "ADAPT::context_static",
    .forms = irules_misc_ADAPT__context_static_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__context_static_effects,
    .hover_index = 837,
};

// ADAPT::enable
inline constexpr FormDesc irules_misc_ADAPT__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__enable_cmd{
    .name = "ADAPT::enable",
    .forms = irules_misc_ADAPT__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__enable_effects,
    .hover_index = 838,
};

// ADAPT::preview_size
inline constexpr FormDesc irules_misc_ADAPT__preview_size_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__preview_size_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__preview_size_cmd{
    .name = "ADAPT::preview_size",
    .forms = irules_misc_ADAPT__preview_size_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__preview_size_effects,
    .hover_index = 839,
};

// ADAPT::result
inline constexpr FormDesc irules_misc_ADAPT__result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__result_cmd{
    .name = "ADAPT::result",
    .forms = irules_misc_ADAPT__result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ADAPT__result_effects,
    .hover_index = 840,
};

// ADAPT::select
inline constexpr FormDesc irules_misc_ADAPT__select_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__select_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__select_cmd{
    .name = "ADAPT::select",
    .forms = irules_misc_ADAPT__select_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__select_effects,
    .hover_index = 841,
};

// ADAPT::service_down_action
inline constexpr FormDesc irules_misc_ADAPT__service_down_action_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__service_down_action_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__service_down_action_cmd{
    .name = "ADAPT::service_down_action",
    .forms = irules_misc_ADAPT__service_down_action_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__service_down_action_effects,
    .hover_index = 842,
};

// ADAPT::timeout
inline constexpr FormDesc irules_misc_ADAPT__timeout_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ADAPT__timeout_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ADAPT__timeout_cmd{
    .name = "ADAPT::timeout",
    .forms = irules_misc_ADAPT__timeout_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_ADAPT__timeout_effects,
    .hover_index = 843,
};

// AES::decrypt
inline constexpr FormDesc irules_misc_AES__decrypt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AES__decrypt_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_AES__decrypt_cmd{
    .name = "AES::decrypt",
    .forms = irules_misc_AES__decrypt_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AES__decrypt_effects,
    .hover_index = 844,
};

// AES::encrypt
inline constexpr FormDesc irules_misc_AES__encrypt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AES__encrypt_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_AES__encrypt_cmd{
    .name = "AES::encrypt",
    .forms = irules_misc_AES__encrypt_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AES__encrypt_effects,
    .hover_index = 845,
};

// AES::key
inline constexpr FormDesc irules_misc_AES__key_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AES__key_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_AES__key_cmd{
    .name = "AES::key",
    .forms = irules_misc_AES__key_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AES__key_effects,
    .hover_index = 846,
};

// AM::age
inline constexpr FormDesc irules_misc_AM__age_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AM__age_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AM__age_cmd{
    .name = "AM::age",
    .forms = irules_misc_AM__age_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AM__age_effects,
    .hover_index = 847,
};

// AM::application
inline constexpr FormDesc irules_misc_AM__application_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AM__application_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AM__application_cmd{
    .name = "AM::application",
    .forms = irules_misc_AM__application_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AM__application_effects,
    .hover_index = 848,
};

// AM::cache
inline constexpr FormDesc irules_misc_AM__cache_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AM__cache_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AM__cache_cmd{
    .name = "AM::cache",
    .forms = irules_misc_AM__cache_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AM__cache_effects,
    .hover_index = 849,
};

// AM::disable
inline constexpr FormDesc irules_misc_AM__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AM__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AM__disable_cmd{
    .name = "AM::disable",
    .forms = irules_misc_AM__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AM__disable_effects,
    .hover_index = 850,
};

// AM::expires
inline constexpr FormDesc irules_misc_AM__expires_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AM__expires_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AM__expires_cmd{
    .name = "AM::expires",
    .forms = irules_misc_AM__expires_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AM__expires_effects,
    .hover_index = 851,
};

// AM::media_playlist
inline constexpr FormDesc irules_misc_AM__media_playlist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AM__media_playlist_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AM__media_playlist_cmd{
    .name = "AM::media_playlist",
    .forms = irules_misc_AM__media_playlist_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AM__media_playlist_effects,
    .hover_index = 852,
};

// AM::policy_node
inline constexpr FormDesc irules_misc_AM__policy_node_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AM__policy_node_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AM__policy_node_cmd{
    .name = "AM::policy_node",
    .forms = irules_misc_AM__policy_node_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AM__policy_node_effects,
    .hover_index = 853,
};

// ANTIFRAUD::alert_additional_info
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_additional_info_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_additional_info_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_additional_info_cmd{
    .name = "ANTIFRAUD::alert_additional_info",
    .forms = irules_misc_ANTIFRAUD__alert_additional_info_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_additional_info_effects,
    .hover_index = 854,
};

// ANTIFRAUD::alert_bait_signatures
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_bait_signatures_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_bait_signatures_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_bait_signatures_cmd{
    .name = "ANTIFRAUD::alert_bait_signatures",
    .forms = irules_misc_ANTIFRAUD__alert_bait_signatures_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_bait_signatures_effects,
    .hover_index = 855,
    .deprecated_replacement = "ANTIFRAUD::alert_details",
};

// ANTIFRAUD::alert_component
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_component_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_component_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_component_cmd{
    .name = "ANTIFRAUD::alert_component",
    .forms = irules_misc_ANTIFRAUD__alert_component_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_component_effects,
    .hover_index = 856,
};

// ANTIFRAUD::alert_defined_value
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_defined_value_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_defined_value_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_defined_value_cmd{
    .name = "ANTIFRAUD::alert_defined_value",
    .forms = irules_misc_ANTIFRAUD__alert_defined_value_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_defined_value_effects,
    .hover_index = 857,
};

// ANTIFRAUD::alert_details
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_details_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_details_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_details_cmd{
    .name = "ANTIFRAUD::alert_details",
    .forms = irules_misc_ANTIFRAUD__alert_details_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_details_effects,
    .hover_index = 858,
};

// ANTIFRAUD::alert_device_id
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_device_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_device_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_device_id_cmd{
    .name = "ANTIFRAUD::alert_device_id",
    .forms = irules_misc_ANTIFRAUD__alert_device_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_device_id_effects,
    .hover_index = 859,
    .deprecated_replacement = "ANTIFRAUD::device_id",
};

// ANTIFRAUD::alert_expected_value
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_expected_value_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_expected_value_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_expected_value_cmd{
    .name = "ANTIFRAUD::alert_expected_value",
    .forms = irules_misc_ANTIFRAUD__alert_expected_value_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_expected_value_effects,
    .hover_index = 860,
};

// ANTIFRAUD::alert_fingerprint
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_fingerprint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_fingerprint_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_fingerprint_cmd{
    .name = "ANTIFRAUD::alert_fingerprint",
    .forms = irules_misc_ANTIFRAUD__alert_fingerprint_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_fingerprint_effects,
    .hover_index = 861,
};

// ANTIFRAUD::alert_forbidden_added_element
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_forbidden_added_element_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_forbidden_added_element_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_forbidden_added_element_cmd{
    .name = "ANTIFRAUD::alert_forbidden_added_element",
    .forms = irules_misc_ANTIFRAUD__alert_forbidden_added_element_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_forbidden_added_element_effects,
    .hover_index = 862,
    .deprecated_replacement = "ANTIFRAUD::alert_details",
};

// ANTIFRAUD::alert_guid
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_guid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_guid_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_guid_cmd{
    .name = "ANTIFRAUD::alert_guid",
    .forms = irules_misc_ANTIFRAUD__alert_guid_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_guid_effects,
    .hover_index = 863,
};

// ANTIFRAUD::alert_html
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_html_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_html_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_html_cmd{
    .name = "ANTIFRAUD::alert_html",
    .forms = irules_misc_ANTIFRAUD__alert_html_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_html_effects,
    .hover_index = 864,
};

// ANTIFRAUD::alert_http_referrer
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_http_referrer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_http_referrer_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_http_referrer_cmd{
    .name = "ANTIFRAUD::alert_http_referrer",
    .forms = irules_misc_ANTIFRAUD__alert_http_referrer_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_http_referrer_effects,
    .hover_index = 865,
};

// ANTIFRAUD::alert_id
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_id_cmd{
    .name = "ANTIFRAUD::alert_id",
    .forms = irules_misc_ANTIFRAUD__alert_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_id_effects,
    .hover_index = 866,
};

// ANTIFRAUD::alert_license_id
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_license_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_license_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_license_id_cmd{
    .name = "ANTIFRAUD::alert_license_id",
    .forms = irules_misc_ANTIFRAUD__alert_license_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_license_id_effects,
    .hover_index = 867,
};

// ANTIFRAUD::alert_min
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_min_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_min_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_min_cmd{
    .name = "ANTIFRAUD::alert_min",
    .forms = irules_misc_ANTIFRAUD__alert_min_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_min_effects,
    .hover_index = 868,
};

// ANTIFRAUD::alert_origin
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_origin_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_origin_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_origin_cmd{
    .name = "ANTIFRAUD::alert_origin",
    .forms = irules_misc_ANTIFRAUD__alert_origin_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_origin_effects,
    .hover_index = 869,
};

// ANTIFRAUD::alert_resolved_value
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_resolved_value_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_resolved_value_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_resolved_value_cmd{
    .name = "ANTIFRAUD::alert_resolved_value",
    .forms = irules_misc_ANTIFRAUD__alert_resolved_value_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_resolved_value_effects,
    .hover_index = 870,
};

// ANTIFRAUD::alert_score
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_score_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_score_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_score_cmd{
    .name = "ANTIFRAUD::alert_score",
    .forms = irules_misc_ANTIFRAUD__alert_score_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_score_effects,
    .hover_index = 871,
};

// ANTIFRAUD::alert_transaction_data
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_transaction_data_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_transaction_data_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_transaction_data_cmd{
    .name = "ANTIFRAUD::alert_transaction_data",
    .forms = irules_misc_ANTIFRAUD__alert_transaction_data_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_transaction_data_effects,
    .hover_index = 872,
};

// ANTIFRAUD::alert_transaction_id
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_transaction_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_transaction_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_transaction_id_cmd{
    .name = "ANTIFRAUD::alert_transaction_id",
    .forms = irules_misc_ANTIFRAUD__alert_transaction_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_transaction_id_effects,
    .hover_index = 873,
};

// ANTIFRAUD::alert_type
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_type_cmd{
    .name = "ANTIFRAUD::alert_type",
    .forms = irules_misc_ANTIFRAUD__alert_type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_type_effects,
    .hover_index = 874,
};

// ANTIFRAUD::alert_username
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_username_cmd{
    .name = "ANTIFRAUD::alert_username",
    .forms = irules_misc_ANTIFRAUD__alert_username_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_username_effects,
    .hover_index = 875,
};

// ANTIFRAUD::alert_view_id
inline constexpr FormDesc irules_misc_ANTIFRAUD__alert_view_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__alert_view_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__alert_view_id_cmd{
    .name = "ANTIFRAUD::alert_view_id",
    .forms = irules_misc_ANTIFRAUD__alert_view_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__alert_view_id_effects,
    .hover_index = 876,
};

// ANTIFRAUD::client_id
inline constexpr FormDesc irules_misc_ANTIFRAUD__client_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__client_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__client_id_cmd{
    .name = "ANTIFRAUD::client_id",
    .forms = irules_misc_ANTIFRAUD__client_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__client_id_effects,
    .hover_index = 877,
};

// ANTIFRAUD::device_id
inline constexpr FormDesc irules_misc_ANTIFRAUD__device_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__device_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__device_id_cmd{
    .name = "ANTIFRAUD::device_id",
    .forms = irules_misc_ANTIFRAUD__device_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__device_id_effects,
    .hover_index = 878,
};

// ANTIFRAUD::disable
inline constexpr FormDesc irules_misc_ANTIFRAUD__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__disable_cmd{
    .name = "ANTIFRAUD::disable",
    .forms = irules_misc_ANTIFRAUD__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__disable_effects,
    .hover_index = 879,
};

// ANTIFRAUD::disable_alert
inline constexpr FormDesc irules_misc_ANTIFRAUD__disable_alert_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__disable_alert_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__disable_alert_cmd{
    .name = "ANTIFRAUD::disable_alert",
    .forms = irules_misc_ANTIFRAUD__disable_alert_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__disable_alert_effects,
    .hover_index = 880,
};

// ANTIFRAUD::disable_app_layer_encryption
inline constexpr FormDesc irules_misc_ANTIFRAUD__disable_app_layer_encryption_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__disable_app_layer_encryption_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__disable_app_layer_encryption_cmd{
    .name = "ANTIFRAUD::disable_app_layer_encryption",
    .forms = irules_misc_ANTIFRAUD__disable_app_layer_encryption_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_ANTIFRAUD__disable_app_layer_encryption_effects,
    .hover_index = 881,
};

// ANTIFRAUD::disable_auto_transactions
inline constexpr FormDesc irules_misc_ANTIFRAUD__disable_auto_transactions_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__disable_auto_transactions_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__disable_auto_transactions_cmd{
    .name = "ANTIFRAUD::disable_auto_transactions",
    .forms = irules_misc_ANTIFRAUD__disable_auto_transactions_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_ANTIFRAUD__disable_auto_transactions_effects,
    .hover_index = 882,
};

// ANTIFRAUD::disable_injection
inline constexpr FormDesc irules_misc_ANTIFRAUD__disable_injection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__disable_injection_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__disable_injection_cmd{
    .name = "ANTIFRAUD::disable_injection",
    .forms = irules_misc_ANTIFRAUD__disable_injection_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_ANTIFRAUD__disable_injection_effects,
    .hover_index = 883,
};

// ANTIFRAUD::disable_malware
inline constexpr FormDesc irules_misc_ANTIFRAUD__disable_malware_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__disable_malware_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__disable_malware_cmd{
    .name = "ANTIFRAUD::disable_malware",
    .forms = irules_misc_ANTIFRAUD__disable_malware_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_ANTIFRAUD__disable_malware_effects,
    .hover_index = 884,
};

// ANTIFRAUD::disable_phishing
inline constexpr FormDesc irules_misc_ANTIFRAUD__disable_phishing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__disable_phishing_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__disable_phishing_cmd{
    .name = "ANTIFRAUD::disable_phishing",
    .forms = irules_misc_ANTIFRAUD__disable_phishing_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_ANTIFRAUD__disable_phishing_effects,
    .hover_index = 885,
};

// ANTIFRAUD::enable
inline constexpr FormDesc irules_misc_ANTIFRAUD__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__enable_cmd{
    .name = "ANTIFRAUD::enable",
    .forms = irules_misc_ANTIFRAUD__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__enable_effects,
    .hover_index = 886,
};

// ANTIFRAUD::enable_log
inline constexpr FormDesc irules_misc_ANTIFRAUD__enable_log_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__enable_log_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__enable_log_cmd{
    .name = "ANTIFRAUD::enable_log",
    .forms = irules_misc_ANTIFRAUD__enable_log_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__enable_log_effects,
    .hover_index = 887,
};

// ANTIFRAUD::fingerprint
inline constexpr FormDesc irules_misc_ANTIFRAUD__fingerprint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__fingerprint_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__fingerprint_cmd{
    .name = "ANTIFRAUD::fingerprint",
    .forms = irules_misc_ANTIFRAUD__fingerprint_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__fingerprint_effects,
    .hover_index = 888,
};

// ANTIFRAUD::geo
inline constexpr FormDesc irules_misc_ANTIFRAUD__geo_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__geo_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__geo_cmd{
    .name = "ANTIFRAUD::geo",
    .forms = irules_misc_ANTIFRAUD__geo_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__geo_effects,
    .hover_index = 889,
};

// ANTIFRAUD::guid
inline constexpr FormDesc irules_misc_ANTIFRAUD__guid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__guid_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__guid_cmd{
    .name = "ANTIFRAUD::guid",
    .forms = irules_misc_ANTIFRAUD__guid_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__guid_effects,
    .hover_index = 890,
};

// ANTIFRAUD::result
inline constexpr FormDesc irules_misc_ANTIFRAUD__result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__result_cmd{
    .name = "ANTIFRAUD::result",
    .forms = irules_misc_ANTIFRAUD__result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__result_effects,
    .hover_index = 891,
};

// ANTIFRAUD::username
inline constexpr FormDesc irules_misc_ANTIFRAUD__username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ANTIFRAUD__username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ANTIFRAUD__username_cmd{
    .name = "ANTIFRAUD::username",
    .forms = irules_misc_ANTIFRAUD__username_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ANTIFRAUD__username_effects,
    .hover_index = 892,
};

// ASM::captcha
inline constexpr FormDesc irules_misc_ASM__captcha_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__captcha_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__captcha_cmd{
    .name = "ASM::captcha",
    .forms = irules_misc_ASM__captcha_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__captcha_effects,
    .hover_index = 893,
};

// ASM::captcha_age
inline constexpr FormDesc irules_misc_ASM__captcha_age_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__captcha_age_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__captcha_age_cmd{
    .name = "ASM::captcha_age",
    .forms = irules_misc_ASM__captcha_age_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__captcha_age_effects,
    .hover_index = 894,
};

// ASM::captcha_status
inline constexpr FormDesc irules_misc_ASM__captcha_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__captcha_status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__captcha_status_cmd{
    .name = "ASM::captcha_status",
    .forms = irules_misc_ASM__captcha_status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__captcha_status_effects,
    .hover_index = 895,
};

// ASM::client_ip
inline constexpr FormDesc irules_misc_ASM__client_ip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__client_ip_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__client_ip_cmd{
    .name = "ASM::client_ip",
    .forms = irules_misc_ASM__client_ip_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__client_ip_effects,
    .hover_index = 896,
};

// ASM::conviction
inline constexpr FormDesc irules_misc_ASM__conviction_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__conviction_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__conviction_cmd{
    .name = "ASM::conviction",
    .forms = irules_misc_ASM__conviction_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__conviction_effects,
    .hover_index = 897,
};

// ASM::deception
inline constexpr FormDesc irules_misc_ASM__deception_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__deception_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__deception_cmd{
    .name = "ASM::deception",
    .forms = irules_misc_ASM__deception_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__deception_effects,
    .hover_index = 898,
};

// ASM::disable
inline constexpr FormDesc irules_misc_ASM__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__disable_cmd{
    .name = "ASM::disable",
    .forms = irules_misc_ASM__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .xc_translatable = 1,
    .side_effect_hints = irules_misc_ASM__disable_effects,
    .hover_index = 899,
};

// ASM::enable
inline constexpr FormDesc irules_misc_ASM__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__enable_cmd{
    .name = "ASM::enable",
    .forms = irules_misc_ASM__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .xc_translatable = 1,
    .side_effect_hints = irules_misc_ASM__enable_effects,
    .hover_index = 900,
};

// ASM::fingerprint
inline constexpr FormDesc irules_misc_ASM__fingerprint_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__fingerprint_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__fingerprint_cmd{
    .name = "ASM::fingerprint",
    .forms = irules_misc_ASM__fingerprint_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__fingerprint_effects,
    .hover_index = 901,
};

// ASM::is_authenticated
inline constexpr FormDesc irules_misc_ASM__is_authenticated_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__is_authenticated_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__is_authenticated_cmd{
    .name = "ASM::is_authenticated",
    .forms = irules_misc_ASM__is_authenticated_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__is_authenticated_effects,
    .hover_index = 902,
};

// ASM::login_status
inline constexpr FormDesc irules_misc_ASM__login_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__login_status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__login_status_cmd{
    .name = "ASM::login_status",
    .forms = irules_misc_ASM__login_status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__login_status_effects,
    .hover_index = 903,
};

// ASM::microservice
inline constexpr FormDesc irules_misc_ASM__microservice_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__microservice_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__microservice_cmd{
    .name = "ASM::microservice",
    .forms = irules_misc_ASM__microservice_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__microservice_effects,
    .hover_index = 904,
};

// ASM::payload
inline constexpr FormDesc irules_misc_ASM__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__payload_cmd{
    .name = "ASM::payload",
    .forms = irules_misc_ASM__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__payload_effects,
    .hover_index = 905,
};

// ASM::policy
inline constexpr FormDesc irules_misc_ASM__policy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__policy_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__policy_cmd{
    .name = "ASM::policy",
    .forms = irules_misc_ASM__policy_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__policy_effects,
    .hover_index = 906,
};

// ASM::raise
inline constexpr FormDesc irules_misc_ASM__raise_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__raise_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__raise_cmd{
    .name = "ASM::raise",
    .forms = irules_misc_ASM__raise_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__raise_effects,
    .hover_index = 907,
};

// ASM::severity
inline constexpr FormDesc irules_misc_ASM__severity_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__severity_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__severity_cmd{
    .name = "ASM::severity",
    .forms = irules_misc_ASM__severity_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__severity_effects,
    .hover_index = 908,
};

// ASM::signature
inline constexpr FormDesc irules_misc_ASM__signature_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__signature_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__signature_cmd{
    .name = "ASM::signature",
    .forms = irules_misc_ASM__signature_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__signature_effects,
    .hover_index = 909,
};

// ASM::status
inline constexpr FormDesc irules_misc_ASM__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__status_cmd{
    .name = "ASM::status",
    .forms = irules_misc_ASM__status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__status_effects,
    .hover_index = 910,
};

// ASM::support_id
inline constexpr FormDesc irules_misc_ASM__support_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__support_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__support_id_cmd{
    .name = "ASM::support_id",
    .forms = irules_misc_ASM__support_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__support_id_effects,
    .hover_index = 911,
};

// ASM::threat_campaign
inline constexpr FormDesc irules_misc_ASM__threat_campaign_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__threat_campaign_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__threat_campaign_cmd{
    .name = "ASM::threat_campaign",
    .forms = irules_misc_ASM__threat_campaign_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__threat_campaign_effects,
    .hover_index = 912,
};

// ASM::unblock
inline constexpr FormDesc irules_misc_ASM__unblock_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__unblock_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__unblock_cmd{
    .name = "ASM::unblock",
    .forms = irules_misc_ASM__unblock_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__unblock_effects,
    .hover_index = 913,
};

// ASM::uncaptcha
inline constexpr FormDesc irules_misc_ASM__uncaptcha_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__uncaptcha_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__uncaptcha_cmd{
    .name = "ASM::uncaptcha",
    .forms = irules_misc_ASM__uncaptcha_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__uncaptcha_effects,
    .hover_index = 914,
};

// ASM::username
inline constexpr FormDesc irules_misc_ASM__username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__username_cmd{
    .name = "ASM::username",
    .forms = irules_misc_ASM__username_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__username_effects,
    .hover_index = 915,
};

// ASM::violation
inline constexpr FormDesc irules_misc_ASM__violation_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__violation_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__violation_cmd{
    .name = "ASM::violation",
    .forms = irules_misc_ASM__violation_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__violation_effects,
    .hover_index = 916,
};

// ASM::violation_data
inline constexpr FormDesc irules_misc_ASM__violation_data_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASM__violation_data_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_ASM__violation_data_cmd{
    .name = "ASM::violation_data",
    .forms = irules_misc_ASM__violation_data_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASM__violation_data_effects,
    .hover_index = 917,
    .deprecated_replacement = "ASM::violation",
};

// ASN1::decode
inline constexpr FormDesc irules_misc_ASN1__decode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASN1__decode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_ASN1__decode_cmd{
    .name = "ASN1::decode",
    .forms = irules_misc_ASN1__decode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASN1__decode_effects,
    .hover_index = 918,
};

// ASN1::element
inline constexpr FormDesc irules_misc_ASN1__element_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASN1__element_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_ASN1__element_cmd{
    .name = "ASN1::element",
    .forms = irules_misc_ASN1__element_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASN1__element_effects,
    .hover_index = 919,
};

// ASN1::encode
inline constexpr FormDesc irules_misc_ASN1__encode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ASN1__encode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_ASN1__encode_cmd{
    .name = "ASN1::encode",
    .forms = irules_misc_ASN1__encode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ASN1__encode_effects,
    .hover_index = 920,
};

// AUTH::abort
inline constexpr FormDesc irules_misc_AUTH__abort_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__abort_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__abort_cmd{
    .name = "AUTH::abort",
    .forms = irules_misc_AUTH__abort_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_AUTH__abort_effects,
    .hover_index = 921,
};

// AUTH::authenticate
inline constexpr FormDesc irules_misc_AUTH__authenticate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__authenticate_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__authenticate_cmd{
    .name = "AUTH::authenticate",
    .forms = irules_misc_AUTH__authenticate_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_AUTH__authenticate_effects,
    .hover_index = 922,
};

// AUTH::authenticate_continue
inline constexpr FormDesc irules_misc_AUTH__authenticate_continue_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__authenticate_continue_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__authenticate_continue_cmd{
    .name = "AUTH::authenticate_continue",
    .forms = irules_misc_AUTH__authenticate_continue_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__authenticate_continue_effects,
    .hover_index = 923,
};

// AUTH::cert_credential
inline constexpr FormDesc irules_misc_AUTH__cert_credential_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__cert_credential_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__cert_credential_cmd{
    .name = "AUTH::cert_credential",
    .forms = irules_misc_AUTH__cert_credential_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__cert_credential_effects,
    .hover_index = 924,
};

// AUTH::cert_issuer_credential
inline constexpr FormDesc irules_misc_AUTH__cert_issuer_credential_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__cert_issuer_credential_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__cert_issuer_credential_cmd{
    .name = "AUTH::cert_issuer_credential",
    .forms = irules_misc_AUTH__cert_issuer_credential_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__cert_issuer_credential_effects,
    .hover_index = 925,
};

// AUTH::last_event_session_id
inline constexpr FormDesc irules_misc_AUTH__last_event_session_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__last_event_session_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__last_event_session_id_cmd{
    .name = "AUTH::last_event_session_id",
    .forms = irules_misc_AUTH__last_event_session_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__last_event_session_id_effects,
    .hover_index = 926,
};

// AUTH::password_credential
inline constexpr FormDesc irules_misc_AUTH__password_credential_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__password_credential_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__password_credential_cmd{
    .name = "AUTH::password_credential",
    .forms = irules_misc_AUTH__password_credential_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_AUTH__password_credential_effects,
    .hover_index = 927,
};

// AUTH::response_data
inline constexpr FormDesc irules_misc_AUTH__response_data_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__response_data_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__response_data_cmd{
    .name = "AUTH::response_data",
    .forms = irules_misc_AUTH__response_data_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__response_data_effects,
    .hover_index = 928,
};

// AUTH::ssl_cc_ldap_status
inline constexpr FormDesc irules_misc_AUTH__ssl_cc_ldap_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__ssl_cc_ldap_status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__ssl_cc_ldap_status_cmd{
    .name = "AUTH::ssl_cc_ldap_status",
    .forms = irules_misc_AUTH__ssl_cc_ldap_status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__ssl_cc_ldap_status_effects,
    .hover_index = 929,
};

// AUTH::ssl_cc_ldap_username
inline constexpr FormDesc irules_misc_AUTH__ssl_cc_ldap_username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__ssl_cc_ldap_username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__ssl_cc_ldap_username_cmd{
    .name = "AUTH::ssl_cc_ldap_username",
    .forms = irules_misc_AUTH__ssl_cc_ldap_username_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__ssl_cc_ldap_username_effects,
    .hover_index = 930,
};

// AUTH::start
inline constexpr FormDesc irules_misc_AUTH__start_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__start_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__start_cmd{
    .name = "AUTH::start",
    .forms = irules_misc_AUTH__start_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__start_effects,
    .hover_index = 931,
};

// AUTH::status
inline constexpr FormDesc irules_misc_AUTH__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__status_cmd{
    .name = "AUTH::status",
    .forms = irules_misc_AUTH__status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__status_effects,
    .hover_index = 932,
};

// AUTH::subscribe
inline constexpr FormDesc irules_misc_AUTH__subscribe_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__subscribe_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__subscribe_cmd{
    .name = "AUTH::subscribe",
    .forms = irules_misc_AUTH__subscribe_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__subscribe_effects,
    .hover_index = 933,
};

// AUTH::unsubscribe
inline constexpr FormDesc irules_misc_AUTH__unsubscribe_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__unsubscribe_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__unsubscribe_cmd{
    .name = "AUTH::unsubscribe",
    .forms = irules_misc_AUTH__unsubscribe_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__unsubscribe_effects,
    .hover_index = 934,
};

// AUTH::username_credential
inline constexpr FormDesc irules_misc_AUTH__username_credential_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__username_credential_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__username_credential_cmd{
    .name = "AUTH::username_credential",
    .forms = irules_misc_AUTH__username_credential_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__username_credential_effects,
    .hover_index = 935,
};

// AUTH::wantcredential_prompt
inline constexpr FormDesc irules_misc_AUTH__wantcredential_prompt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__wantcredential_prompt_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__wantcredential_prompt_cmd{
    .name = "AUTH::wantcredential_prompt",
    .forms = irules_misc_AUTH__wantcredential_prompt_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_AUTH__wantcredential_prompt_effects,
    .hover_index = 936,
};

// AUTH::wantcredential_prompt_style
inline constexpr FormDesc irules_misc_AUTH__wantcredential_prompt_style_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__wantcredential_prompt_style_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__wantcredential_prompt_style_cmd{
    .name = "AUTH::wantcredential_prompt_style",
    .forms = irules_misc_AUTH__wantcredential_prompt_style_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__wantcredential_prompt_style_effects,
    .hover_index = 937,
};

// AUTH::wantcredential_type
inline constexpr FormDesc irules_misc_AUTH__wantcredential_type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AUTH__wantcredential_type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AUTH__wantcredential_type_cmd{
    .name = "AUTH::wantcredential_type",
    .forms = irules_misc_AUTH__wantcredential_type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AUTH__wantcredential_type_effects,
    .hover_index = 938,
};

// AVR::disable
inline constexpr FormDesc irules_misc_AVR__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AVR__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AVR__disable_cmd{
    .name = "AVR::disable",
    .forms = irules_misc_AVR__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AVR__disable_effects,
    .hover_index = 939,
};

// AVR::disable_cspm_injection
inline constexpr FormDesc irules_misc_AVR__disable_cspm_injection_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AVR__disable_cspm_injection_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AVR__disable_cspm_injection_cmd{
    .name = "AVR::disable_cspm_injection",
    .forms = irules_misc_AVR__disable_cspm_injection_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_AVR__disable_cspm_injection_effects,
    .hover_index = 940,
};

// AVR::enable
inline constexpr FormDesc irules_misc_AVR__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AVR__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AVR__enable_cmd{
    .name = "AVR::enable",
    .forms = irules_misc_AVR__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AVR__enable_effects,
    .hover_index = 941,
};

// AVR::log
inline constexpr FormDesc irules_misc_AVR__log_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_AVR__log_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_AVR__log_cmd{
    .name = "AVR::log",
    .forms = irules_misc_AVR__log_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_AVR__log_effects,
    .hover_index = 942,
};

// BIGPROTO::enable_fix_reset
inline constexpr FormDesc irules_misc_BIGPROTO__enable_fix_reset_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BIGPROTO__enable_fix_reset_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BIGPROTO__enable_fix_reset_cmd{
    .name = "BIGPROTO::enable_fix_reset",
    .forms = irules_misc_BIGPROTO__enable_fix_reset_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BIGPROTO__enable_fix_reset_effects,
    .hover_index = 943,
};

// BIGTCP::release_flow
inline constexpr FormDesc irules_misc_BIGTCP__release_flow_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BIGTCP__release_flow_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BIGTCP__release_flow_cmd{
    .name = "BIGTCP::release_flow",
    .forms = irules_misc_BIGTCP__release_flow_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BIGTCP__release_flow_effects,
    .hover_index = 944,
};

// BOTDEFENSE::action
inline constexpr FormDesc irules_misc_BOTDEFENSE__action_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__action_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__action_cmd{
    .name = "BOTDEFENSE::action",
    .forms = irules_misc_BOTDEFENSE__action_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__action_effects,
    .hover_index = 945,
};

// BOTDEFENSE::bot_anomalies
inline constexpr FormDesc irules_misc_BOTDEFENSE__bot_anomalies_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__bot_anomalies_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__bot_anomalies_cmd{
    .name = "BOTDEFENSE::bot_anomalies",
    .forms = irules_misc_BOTDEFENSE__bot_anomalies_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__bot_anomalies_effects,
    .hover_index = 946,
};

// BOTDEFENSE::bot_categories
inline constexpr FormDesc irules_misc_BOTDEFENSE__bot_categories_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__bot_categories_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__bot_categories_cmd{
    .name = "BOTDEFENSE::bot_categories",
    .forms = irules_misc_BOTDEFENSE__bot_categories_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__bot_categories_effects,
    .hover_index = 947,
};

// BOTDEFENSE::bot_name
inline constexpr FormDesc irules_misc_BOTDEFENSE__bot_name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__bot_name_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__bot_name_cmd{
    .name = "BOTDEFENSE::bot_name",
    .forms = irules_misc_BOTDEFENSE__bot_name_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__bot_name_effects,
    .hover_index = 948,
};

// BOTDEFENSE::bot_signature
inline constexpr FormDesc irules_misc_BOTDEFENSE__bot_signature_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__bot_signature_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__bot_signature_cmd{
    .name = "BOTDEFENSE::bot_signature",
    .forms = irules_misc_BOTDEFENSE__bot_signature_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__bot_signature_effects,
    .hover_index = 949,
};

// BOTDEFENSE::bot_signature_category
inline constexpr FormDesc irules_misc_BOTDEFENSE__bot_signature_category_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__bot_signature_category_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__bot_signature_category_cmd{
    .name = "BOTDEFENSE::bot_signature_category",
    .forms = irules_misc_BOTDEFENSE__bot_signature_category_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__bot_signature_category_effects,
    .hover_index = 950,
};

// BOTDEFENSE::captcha_age
inline constexpr FormDesc irules_misc_BOTDEFENSE__captcha_age_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__captcha_age_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__captcha_age_cmd{
    .name = "BOTDEFENSE::captcha_age",
    .forms = irules_misc_BOTDEFENSE__captcha_age_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__captcha_age_effects,
    .hover_index = 951,
};

// BOTDEFENSE::captcha_status
inline constexpr FormDesc irules_misc_BOTDEFENSE__captcha_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__captcha_status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__captcha_status_cmd{
    .name = "BOTDEFENSE::captcha_status",
    .forms = irules_misc_BOTDEFENSE__captcha_status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__captcha_status_effects,
    .hover_index = 952,
};

// BOTDEFENSE::client_class
inline constexpr FormDesc irules_misc_BOTDEFENSE__client_class_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__client_class_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__client_class_cmd{
    .name = "BOTDEFENSE::client_class",
    .forms = irules_misc_BOTDEFENSE__client_class_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__client_class_effects,
    .hover_index = 953,
};

// BOTDEFENSE::client_type
inline constexpr FormDesc irules_misc_BOTDEFENSE__client_type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__client_type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__client_type_cmd{
    .name = "BOTDEFENSE::client_type",
    .forms = irules_misc_BOTDEFENSE__client_type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__client_type_effects,
    .hover_index = 954,
};

// BOTDEFENSE::cookie_age
inline constexpr FormDesc irules_misc_BOTDEFENSE__cookie_age_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__cookie_age_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__cookie_age_cmd{
    .name = "BOTDEFENSE::cookie_age",
    .forms = irules_misc_BOTDEFENSE__cookie_age_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__cookie_age_effects,
    .hover_index = 955,
};

// BOTDEFENSE::cookie_status
inline constexpr FormDesc irules_misc_BOTDEFENSE__cookie_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__cookie_status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__cookie_status_cmd{
    .name = "BOTDEFENSE::cookie_status",
    .forms = irules_misc_BOTDEFENSE__cookie_status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__cookie_status_effects,
    .hover_index = 956,
};

// BOTDEFENSE::cs_allowed
inline constexpr FormDesc irules_misc_BOTDEFENSE__cs_allowed_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__cs_allowed_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__cs_allowed_cmd{
    .name = "BOTDEFENSE::cs_allowed",
    .forms = irules_misc_BOTDEFENSE__cs_allowed_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__cs_allowed_effects,
    .hover_index = 957,
};

// BOTDEFENSE::cs_attribute
inline constexpr FormDesc irules_misc_BOTDEFENSE__cs_attribute_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__cs_attribute_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__cs_attribute_cmd{
    .name = "BOTDEFENSE::cs_attribute",
    .forms = irules_misc_BOTDEFENSE__cs_attribute_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__cs_attribute_effects,
    .hover_index = 958,
};

// BOTDEFENSE::cs_possible
inline constexpr FormDesc irules_misc_BOTDEFENSE__cs_possible_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__cs_possible_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__cs_possible_cmd{
    .name = "BOTDEFENSE::cs_possible",
    .forms = irules_misc_BOTDEFENSE__cs_possible_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__cs_possible_effects,
    .hover_index = 959,
};

// BOTDEFENSE::device_id
inline constexpr FormDesc irules_misc_BOTDEFENSE__device_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__device_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__device_id_cmd{
    .name = "BOTDEFENSE::device_id",
    .forms = irules_misc_BOTDEFENSE__device_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__device_id_effects,
    .hover_index = 960,
};

// BOTDEFENSE::disable
inline constexpr FormDesc irules_misc_BOTDEFENSE__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__disable_cmd{
    .name = "BOTDEFENSE::disable",
    .forms = irules_misc_BOTDEFENSE__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__disable_effects,
    .hover_index = 961,
};

// BOTDEFENSE::enable
inline constexpr FormDesc irules_misc_BOTDEFENSE__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__enable_cmd{
    .name = "BOTDEFENSE::enable",
    .forms = irules_misc_BOTDEFENSE__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__enable_effects,
    .hover_index = 962,
};

// BOTDEFENSE::intent
inline constexpr FormDesc irules_misc_BOTDEFENSE__intent_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__intent_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__intent_cmd{
    .name = "BOTDEFENSE::intent",
    .forms = irules_misc_BOTDEFENSE__intent_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__intent_effects,
    .hover_index = 963,
};

// BOTDEFENSE::micro_service
inline constexpr FormDesc irules_misc_BOTDEFENSE__micro_service_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__micro_service_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__micro_service_cmd{
    .name = "BOTDEFENSE::micro_service",
    .forms = irules_misc_BOTDEFENSE__micro_service_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__micro_service_effects,
    .hover_index = 964,
};

// BOTDEFENSE::previous_action
inline constexpr FormDesc irules_misc_BOTDEFENSE__previous_action_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__previous_action_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__previous_action_cmd{
    .name = "BOTDEFENSE::previous_action",
    .forms = irules_misc_BOTDEFENSE__previous_action_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__previous_action_effects,
    .hover_index = 965,
};

// BOTDEFENSE::previous_request_age
inline constexpr FormDesc irules_misc_BOTDEFENSE__previous_request_age_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__previous_request_age_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__previous_request_age_cmd{
    .name = "BOTDEFENSE::previous_request_age",
    .forms = irules_misc_BOTDEFENSE__previous_request_age_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__previous_request_age_effects,
    .hover_index = 966,
};

// BOTDEFENSE::previous_support_id
inline constexpr FormDesc irules_misc_BOTDEFENSE__previous_support_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__previous_support_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__previous_support_id_cmd{
    .name = "BOTDEFENSE::previous_support_id",
    .forms = irules_misc_BOTDEFENSE__previous_support_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__previous_support_id_effects,
    .hover_index = 967,
};

// BOTDEFENSE::reason
inline constexpr FormDesc irules_misc_BOTDEFENSE__reason_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__reason_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__reason_cmd{
    .name = "BOTDEFENSE::reason",
    .forms = irules_misc_BOTDEFENSE__reason_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__reason_effects,
    .hover_index = 968,
};

// BOTDEFENSE::support_id
inline constexpr FormDesc irules_misc_BOTDEFENSE__support_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BOTDEFENSE__support_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_BOTDEFENSE__support_id_cmd{
    .name = "BOTDEFENSE::support_id",
    .forms = irules_misc_BOTDEFENSE__support_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BOTDEFENSE__support_id_effects,
    .hover_index = 969,
};

// BWC::color
inline constexpr FormDesc irules_misc_BWC__color_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__color_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__color_cmd{
    .name = "BWC::color",
    .forms = irules_misc_BWC__color_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__color_effects,
    .hover_index = 970,
};

// BWC::debug
inline constexpr FormDesc irules_misc_BWC__debug_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__debug_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__debug_cmd{
    .name = "BWC::debug",
    .forms = irules_misc_BWC__debug_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__debug_effects,
    .hover_index = 971,
};

// BWC::mark
inline constexpr FormDesc irules_misc_BWC__mark_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__mark_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__mark_cmd{
    .name = "BWC::mark",
    .forms = irules_misc_BWC__mark_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__mark_effects,
    .hover_index = 972,
};

// BWC::measure
inline constexpr FormDesc irules_misc_BWC__measure_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__measure_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__measure_cmd{
    .name = "BWC::measure",
    .forms = irules_misc_BWC__measure_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__measure_effects,
    .hover_index = 973,
};

// BWC::policy
inline constexpr FormDesc irules_misc_BWC__policy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__policy_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__policy_cmd{
    .name = "BWC::policy",
    .forms = irules_misc_BWC__policy_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__policy_effects,
    .hover_index = 974,
};

// BWC::pps
inline constexpr FormDesc irules_misc_BWC__pps_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__pps_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__pps_cmd{
    .name = "BWC::pps",
    .forms = irules_misc_BWC__pps_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__pps_effects,
    .hover_index = 975,
};

// BWC::priority
inline constexpr FormDesc irules_misc_BWC__priority_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__priority_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__priority_cmd{
    .name = "BWC::priority",
    .forms = irules_misc_BWC__priority_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__priority_effects,
    .hover_index = 976,
};

// BWC::rate
inline constexpr FormDesc irules_misc_BWC__rate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_BWC__rate_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_BWC__rate_cmd{
    .name = "BWC::rate",
    .forms = irules_misc_BWC__rate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_BWC__rate_effects,
    .hover_index = 977,
};

// CACHE::accept_encoding
inline constexpr FormDesc irules_misc_CACHE__accept_encoding_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__accept_encoding_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__accept_encoding_cmd{
    .name = "CACHE::accept_encoding",
    .forms = irules_misc_CACHE__accept_encoding_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__accept_encoding_effects,
    .hover_index = 978,
};

// CACHE::age
inline constexpr FormDesc irules_misc_CACHE__age_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__age_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__age_cmd{
    .name = "CACHE::age",
    .forms = irules_misc_CACHE__age_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__age_effects,
    .hover_index = 979,
};

// CACHE::disable
inline constexpr FormDesc irules_misc_CACHE__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__disable_cmd{
    .name = "CACHE::disable",
    .forms = irules_misc_CACHE__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CACHE__disable_effects,
    .hover_index = 980,
};

// CACHE::disabled
inline constexpr FormDesc irules_misc_CACHE__disabled_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__disabled_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__disabled_cmd{
    .name = "CACHE::disabled",
    .forms = irules_misc_CACHE__disabled_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CACHE__disabled_effects,
    .hover_index = 981,
};

// CACHE::enable
inline constexpr FormDesc irules_misc_CACHE__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__enable_cmd{
    .name = "CACHE::enable",
    .forms = irules_misc_CACHE__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CACHE__enable_effects,
    .hover_index = 982,
};

// CACHE::expire
inline constexpr FormDesc irules_misc_CACHE__expire_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__expire_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__expire_cmd{
    .name = "CACHE::expire",
    .forms = irules_misc_CACHE__expire_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__expire_effects,
    .hover_index = 983,
};

// CACHE::fresh
inline constexpr FormDesc irules_misc_CACHE__fresh_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__fresh_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__fresh_cmd{
    .name = "CACHE::fresh",
    .forms = irules_misc_CACHE__fresh_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__fresh_effects,
    .hover_index = 984,
};

// CACHE::header
inline constexpr FormDesc irules_misc_CACHE__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__header_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__header_cmd{
    .name = "CACHE::header",
    .forms = irules_misc_CACHE__header_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__header_effects,
    .hover_index = 985,
};

// CACHE::headers
inline constexpr FormDesc irules_misc_CACHE__headers_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__headers_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__headers_cmd{
    .name = "CACHE::headers",
    .forms = irules_misc_CACHE__headers_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__headers_effects,
    .hover_index = 986,
};

// CACHE::hits
inline constexpr FormDesc irules_misc_CACHE__hits_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__hits_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__hits_cmd{
    .name = "CACHE::hits",
    .forms = irules_misc_CACHE__hits_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__hits_effects,
    .hover_index = 987,
};

// CACHE::payload
inline constexpr FormDesc irules_misc_CACHE__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__payload_cmd{
    .name = "CACHE::payload",
    .forms = irules_misc_CACHE__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__payload_effects,
    .hover_index = 988,
};

// CACHE::priority
inline constexpr FormDesc irules_misc_CACHE__priority_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__priority_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__priority_cmd{
    .name = "CACHE::priority",
    .forms = irules_misc_CACHE__priority_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_CACHE__priority_effects,
    .hover_index = 989,
};

// CACHE::statskey
inline constexpr FormDesc irules_misc_CACHE__statskey_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__statskey_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__statskey_cmd{
    .name = "CACHE::statskey",
    .forms = irules_misc_CACHE__statskey_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__statskey_effects,
    .hover_index = 990,
};

// CACHE::trace
inline constexpr FormDesc irules_misc_CACHE__trace_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__trace_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__trace_cmd{
    .name = "CACHE::trace",
    .forms = irules_misc_CACHE__trace_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CACHE__trace_effects,
    .hover_index = 991,
};

// CACHE::uri
inline constexpr FormDesc irules_misc_CACHE__uri_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__uri_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__uri_cmd{
    .name = "CACHE::uri",
    .forms = irules_misc_CACHE__uri_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_CACHE__uri_effects,
    .hover_index = 992,
};

// CACHE::useragent
inline constexpr FormDesc irules_misc_CACHE__useragent_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__useragent_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__useragent_cmd{
    .name = "CACHE::useragent",
    .forms = irules_misc_CACHE__useragent_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_CACHE__useragent_effects,
    .hover_index = 993,
};

// CACHE::userkey
inline constexpr FormDesc irules_misc_CACHE__userkey_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CACHE__userkey_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CACHE__userkey_cmd{
    .name = "CACHE::userkey",
    .forms = irules_misc_CACHE__userkey_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_CACHE__userkey_effects,
    .hover_index = 994,
};

// CATEGORY::analytics
inline constexpr FormDesc irules_misc_CATEGORY__analytics_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CATEGORY__analytics_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CATEGORY__analytics_cmd{
    .name = "CATEGORY::analytics",
    .forms = irules_misc_CATEGORY__analytics_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CATEGORY__analytics_effects,
    .hover_index = 995,
};

// CATEGORY::filetype
inline constexpr OptionDesc irules_misc_CATEGORY__filetype_opts[] = {
    {.name = "-mimetype", .takes_value = true, .value_hint = "TYPE", .detail = "Variable name to store MIME type."},
    {.name = "-mimesubtype", .takes_value = true, .value_hint = "SUBTYPE", .detail = "Variable name to store MIME subtype."},
};

inline constexpr FormDesc irules_misc_CATEGORY__filetype_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CATEGORY__filetype_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CATEGORY__filetype_cmd{
    .name = "CATEGORY::filetype",
    .options = irules_misc_CATEGORY__filetype_opts,
    .forms = irules_misc_CATEGORY__filetype_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_CATEGORY__filetype_effects,
    .hover_index = 996,
};

// CATEGORY::lookup
inline constexpr OptionDesc irules_misc_CATEGORY__lookup_opts[] = {
    {.name = "-display", .detail = "Return display name of categories."},
    {.name = "-id", .detail = "Return category IDs."},
    {.name = "-ip", .takes_value = true, .value_hint = "IP", .detail = "IP address to categorize."},
    {.name = "-custom_cat_match", .takes_value = true, .value_hint = "CUSTOM_CAT", .detail = "Match against a specified custom category."},
};

inline constexpr FormDesc irules_misc_CATEGORY__lookup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CATEGORY__lookup_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CATEGORY__lookup_cmd{
    .name = "CATEGORY::lookup",
    .options = irules_misc_CATEGORY__lookup_opts,
    .forms = irules_misc_CATEGORY__lookup_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CATEGORY__lookup_effects,
    .hover_index = 997,
};

// CATEGORY::matchtype
inline constexpr FormDesc irules_misc_CATEGORY__matchtype_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CATEGORY__matchtype_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CATEGORY__matchtype_cmd{
    .name = "CATEGORY::matchtype",
    .forms = irules_misc_CATEGORY__matchtype_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CATEGORY__matchtype_effects,
    .hover_index = 998,
};

// CATEGORY::result
inline constexpr OptionDesc irules_misc_CATEGORY__result_opts[] = {
    {.name = "-display", .takes_value = true, .detail = "Option -display."},
    {.name = "-id", .takes_value = true, .detail = "Option -id."},
};

inline constexpr FormDesc irules_misc_CATEGORY__result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CATEGORY__result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CATEGORY__result_cmd{
    .name = "CATEGORY::result",
    .options = irules_misc_CATEGORY__result_opts,
    .forms = irules_misc_CATEGORY__result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CATEGORY__result_effects,
    .hover_index = 999,
};

// CATEGORY::safesearch
inline constexpr OptionDesc irules_misc_CATEGORY__safesearch_opts[] = {
    {.name = "-ip", .takes_value = true, .detail = "Option -ip."},
};

inline constexpr FormDesc irules_misc_CATEGORY__safesearch_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CATEGORY__safesearch_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CATEGORY__safesearch_cmd{
    .name = "CATEGORY::safesearch",
    .options = irules_misc_CATEGORY__safesearch_opts,
    .forms = irules_misc_CATEGORY__safesearch_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CATEGORY__safesearch_effects,
    .hover_index = 1000,
};

// CLASSIFICATION::app
inline constexpr FormDesc irules_misc_CLASSIFICATION__app_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__app_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__app_cmd{
    .name = "CLASSIFICATION::app",
    .forms = irules_misc_CLASSIFICATION__app_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__app_effects,
    .hover_index = 1001,
};

// CLASSIFICATION::category
inline constexpr FormDesc irules_misc_CLASSIFICATION__category_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__category_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__category_cmd{
    .name = "CLASSIFICATION::category",
    .forms = irules_misc_CLASSIFICATION__category_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__category_effects,
    .hover_index = 1002,
};

// CLASSIFICATION::disable
inline constexpr FormDesc irules_misc_CLASSIFICATION__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__disable_cmd{
    .name = "CLASSIFICATION::disable",
    .forms = irules_misc_CLASSIFICATION__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__disable_effects,
    .hover_index = 1003,
};

// CLASSIFICATION::enable
inline constexpr FormDesc irules_misc_CLASSIFICATION__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__enable_cmd{
    .name = "CLASSIFICATION::enable",
    .forms = irules_misc_CLASSIFICATION__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__enable_effects,
    .hover_index = 1004,
};

// CLASSIFICATION::protocol
inline constexpr FormDesc irules_misc_CLASSIFICATION__protocol_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__protocol_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__protocol_cmd{
    .name = "CLASSIFICATION::protocol",
    .forms = irules_misc_CLASSIFICATION__protocol_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__protocol_effects,
    .hover_index = 1005,
};

// CLASSIFICATION::result
inline constexpr FormDesc irules_misc_CLASSIFICATION__result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__result_cmd{
    .name = "CLASSIFICATION::result",
    .forms = irules_misc_CLASSIFICATION__result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__result_effects,
    .hover_index = 1006,
};

// CLASSIFICATION::urlcat
inline constexpr FormDesc irules_misc_CLASSIFICATION__urlcat_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__urlcat_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__urlcat_cmd{
    .name = "CLASSIFICATION::urlcat",
    .forms = irules_misc_CLASSIFICATION__urlcat_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__urlcat_effects,
    .hover_index = 1007,
};

// CLASSIFICATION::username
inline constexpr FormDesc irules_misc_CLASSIFICATION__username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFICATION__username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFICATION__username_cmd{
    .name = "CLASSIFICATION::username",
    .forms = irules_misc_CLASSIFICATION__username_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFICATION__username_effects,
    .hover_index = 1008,
};

// CLASSIFY::application
inline constexpr FormDesc irules_misc_CLASSIFY__application_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFY__application_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFY__application_cmd{
    .name = "CLASSIFY::application",
    .forms = irules_misc_CLASSIFY__application_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CLASSIFY__application_effects,
    .hover_index = 1009,
};

// CLASSIFY::category
inline constexpr FormDesc irules_misc_CLASSIFY__category_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFY__category_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFY__category_cmd{
    .name = "CLASSIFY::category",
    .forms = irules_misc_CLASSIFY__category_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CLASSIFY__category_effects,
    .hover_index = 1010,
};

// CLASSIFY::defer
inline constexpr FormDesc irules_misc_CLASSIFY__defer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFY__defer_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFY__defer_cmd{
    .name = "CLASSIFY::defer",
    .forms = irules_misc_CLASSIFY__defer_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CLASSIFY__defer_effects,
    .hover_index = 1011,
};

// CLASSIFY::disable
inline constexpr FormDesc irules_misc_CLASSIFY__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFY__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFY__disable_cmd{
    .name = "CLASSIFY::disable",
    .forms = irules_misc_CLASSIFY__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFY__disable_effects,
    .hover_index = 1012,
};

// CLASSIFY::urlcat
inline constexpr FormDesc irules_misc_CLASSIFY__urlcat_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFY__urlcat_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFY__urlcat_cmd{
    .name = "CLASSIFY::urlcat",
    .forms = irules_misc_CLASSIFY__urlcat_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_CLASSIFY__urlcat_effects,
    .hover_index = 1013,
};

// CLASSIFY::username
inline constexpr FormDesc irules_misc_CLASSIFY__username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CLASSIFY__username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CLASSIFY__username_cmd{
    .name = "CLASSIFY::username",
    .forms = irules_misc_CLASSIFY__username_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CLASSIFY__username_effects,
    .hover_index = 1014,
};

// COMPRESS::buffer_size
inline constexpr FormDesc irules_misc_COMPRESS__buffer_size_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_COMPRESS__buffer_size_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_COMPRESS__buffer_size_cmd{
    .name = "COMPRESS::buffer_size",
    .forms = irules_misc_COMPRESS__buffer_size_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_COMPRESS__buffer_size_effects,
    .hover_index = 1015,
};

// COMPRESS::disable
inline constexpr FormDesc irules_misc_COMPRESS__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_COMPRESS__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_COMPRESS__disable_cmd{
    .name = "COMPRESS::disable",
    .forms = irules_misc_COMPRESS__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_COMPRESS__disable_effects,
    .hover_index = 1016,
};

// COMPRESS::enable
inline constexpr FormDesc irules_misc_COMPRESS__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_COMPRESS__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_COMPRESS__enable_cmd{
    .name = "COMPRESS::enable",
    .forms = irules_misc_COMPRESS__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_COMPRESS__enable_effects,
    .hover_index = 1017,
};

// COMPRESS::gzip
inline constexpr FormDesc irules_misc_COMPRESS__gzip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_COMPRESS__gzip_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_COMPRESS__gzip_cmd{
    .name = "COMPRESS::gzip",
    .forms = irules_misc_COMPRESS__gzip_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_COMPRESS__gzip_effects,
    .hover_index = 1018,
};

// COMPRESS::method
inline constexpr FormDesc irules_misc_COMPRESS__method_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_COMPRESS__method_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_COMPRESS__method_cmd{
    .name = "COMPRESS::method",
    .forms = irules_misc_COMPRESS__method_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_COMPRESS__method_effects,
    .hover_index = 1019,
};

// COMPRESS::nodelay
inline constexpr FormDesc irules_misc_COMPRESS__nodelay_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_COMPRESS__nodelay_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_COMPRESS__nodelay_cmd{
    .name = "COMPRESS::nodelay",
    .forms = irules_misc_COMPRESS__nodelay_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_COMPRESS__nodelay_effects,
    .hover_index = 1020,
};

// CONNECTOR::disable
inline constexpr FormDesc irules_misc_CONNECTOR__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CONNECTOR__disable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CONNECTOR__disable_cmd{
    .name = "CONNECTOR::disable",
    .forms = irules_misc_CONNECTOR__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CONNECTOR__disable_effects,
    .hover_index = 1021,
};

// CONNECTOR::enable
inline constexpr FormDesc irules_misc_CONNECTOR__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CONNECTOR__enable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CONNECTOR__enable_cmd{
    .name = "CONNECTOR::enable",
    .forms = irules_misc_CONNECTOR__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CONNECTOR__enable_effects,
    .hover_index = 1022,
};

// CONNECTOR::profile
inline constexpr FormDesc irules_misc_CONNECTOR__profile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CONNECTOR__profile_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CONNECTOR__profile_cmd{
    .name = "CONNECTOR::profile",
    .forms = irules_misc_CONNECTOR__profile_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CONNECTOR__profile_effects,
    .hover_index = 1023,
};

// CONNECTOR::remap
inline constexpr FormDesc irules_misc_CONNECTOR__remap_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CONNECTOR__remap_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_CONNECTOR__remap_cmd{
    .name = "CONNECTOR::remap",
    .forms = irules_misc_CONNECTOR__remap_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CONNECTOR__remap_effects,
    .hover_index = 1024,
};

// CRYPTO::decrypt
inline constexpr OptionDesc irules_misc_CRYPTO__decrypt_opts[] = {
    {.name = "-alg", .takes_value = true, .value_hint = "ALG", .detail = "Decryption algorithm."},
    {.name = "-ctx", .takes_value = true, .value_hint = "CTX_VAR", .detail = "Context variable for multi-step operations."},
    {.name = "-final", .detail = "Finalize context-based operation."},
    {.name = "-key", .takes_value = true, .value_hint = "KEY", .detail = "Binary key."},
    {.name = "-keyhex", .takes_value = true, .value_hint = "KEY_HEX", .detail = "Hex-encoded key."},
    {.name = "-iv", .takes_value = true, .value_hint = "IV", .detail = "Initialization vector (binary)."},
    {.name = "-ivhex", .takes_value = true, .value_hint = "IV_HEX", .detail = "Initialization vector (hex)."},
    {.name = "-padding", .takes_value = true, .value_hint = "PADDING", .detail = "Padding mode (pkcs, oaep, none)."},
};

inline constexpr FormDesc irules_misc_CRYPTO__decrypt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CRYPTO__decrypt_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_CRYPTO__decrypt_cmd{
    .name = "CRYPTO::decrypt",
    .options = irules_misc_CRYPTO__decrypt_opts,
    .forms = irules_misc_CRYPTO__decrypt_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CRYPTO__decrypt_effects,
    .hover_index = 1025,
};

// CRYPTO::encrypt
inline constexpr OptionDesc irules_misc_CRYPTO__encrypt_opts[] = {
    {.name = "-alg", .takes_value = true, .value_hint = "ALG", .detail = "Encryption algorithm."},
    {.name = "-ctx", .takes_value = true, .value_hint = "CTX_VAR", .detail = "Context variable for multi-step operations."},
    {.name = "-final", .detail = "Finalize context-based operation."},
    {.name = "-key", .takes_value = true, .value_hint = "KEY", .detail = "Binary key."},
    {.name = "-keyhex", .takes_value = true, .value_hint = "KEY_HEX", .detail = "Hex-encoded key."},
    {.name = "-iv", .takes_value = true, .value_hint = "IV", .detail = "Initialization vector (binary)."},
    {.name = "-ivhex", .takes_value = true, .value_hint = "IV_HEX", .detail = "Initialization vector (hex)."},
    {.name = "-padding", .takes_value = true, .value_hint = "PADDING", .detail = "Padding mode (pkcs, oaep, none)."},
};

inline constexpr FormDesc irules_misc_CRYPTO__encrypt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CRYPTO__encrypt_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_CRYPTO__encrypt_cmd{
    .name = "CRYPTO::encrypt",
    .options = irules_misc_CRYPTO__encrypt_opts,
    .forms = irules_misc_CRYPTO__encrypt_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CRYPTO__encrypt_effects,
    .hover_index = 1026,
};

// CRYPTO::hash
inline constexpr OptionDesc irules_misc_CRYPTO__hash_opts[] = {
    {.name = "-alg", .takes_value = true, .value_hint = "ALG", .detail = "Hash algorithm."},
    {.name = "-ctx", .takes_value = true, .value_hint = "CTX_VAR", .detail = "Context variable for multi-step operations."},
    {.name = "-final", .detail = "Finalize context-based operation."},
};

inline constexpr FormDesc irules_misc_CRYPTO__hash_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CRYPTO__hash_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_CRYPTO__hash_cmd{
    .name = "CRYPTO::hash",
    .options = irules_misc_CRYPTO__hash_opts,
    .forms = irules_misc_CRYPTO__hash_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CRYPTO__hash_effects,
    .hover_index = 1027,
};

// CRYPTO::keygen
inline constexpr OptionDesc irules_misc_CRYPTO__keygen_opts[] = {
    {.name = "-alg", .takes_value = true, .value_hint = "ALG", .detail = "Key generation algorithm."},
    {.name = "-len", .takes_value = true, .value_hint = "LENGTH", .detail = "Key length (must be multiple of 8)."},
    {.name = "-exp", .takes_value = true, .value_hint = "EXPONENT", .detail = "Exponent (for RSA)."},
    {.name = "-passphrase", .takes_value = true, .value_hint = "PASSPHRASE", .detail = "Passphrase for key derivation."},
    {.name = "-salt", .takes_value = true, .value_hint = "SALT", .detail = "Binary salt."},
    {.name = "-salthex", .takes_value = true, .value_hint = "SALT_HEX", .detail = "Hex-encoded salt."},
    {.name = "-rounds", .takes_value = true, .value_hint = "ROUNDS", .detail = "Rounds for PBKDF2."},
};

inline constexpr FormDesc irules_misc_CRYPTO__keygen_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CRYPTO__keygen_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_CRYPTO__keygen_cmd{
    .name = "CRYPTO::keygen",
    .options = irules_misc_CRYPTO__keygen_opts,
    .forms = irules_misc_CRYPTO__keygen_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CRYPTO__keygen_effects,
    .hover_index = 1028,
};

// CRYPTO::sign
inline constexpr OptionDesc irules_misc_CRYPTO__sign_opts[] = {
    {.name = "-alg", .takes_value = true, .value_hint = "ALG", .detail = "Signing algorithm."},
    {.name = "-ctx", .takes_value = true, .value_hint = "CTX_VAR", .detail = "Context variable for multi-step operations."},
    {.name = "-final", .detail = "Finalize context-based operation."},
    {.name = "-key", .takes_value = true, .value_hint = "KEY", .detail = "Binary key."},
    {.name = "-keyhex", .takes_value = true, .value_hint = "KEY_HEX", .detail = "Hex-encoded key."},
};

inline constexpr FormDesc irules_misc_CRYPTO__sign_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CRYPTO__sign_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_CRYPTO__sign_cmd{
    .name = "CRYPTO::sign",
    .options = irules_misc_CRYPTO__sign_opts,
    .forms = irules_misc_CRYPTO__sign_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CRYPTO__sign_effects,
    .hover_index = 1029,
};

// CRYPTO::verify
inline constexpr OptionDesc irules_misc_CRYPTO__verify_opts[] = {
    {.name = "-alg", .takes_value = true, .value_hint = "ALG", .detail = "Verification algorithm."},
    {.name = "-ctx", .takes_value = true, .value_hint = "CTX_VAR", .detail = "Context variable for multi-step operations."},
    {.name = "-final", .detail = "Finalize context-based operation."},
    {.name = "-key", .takes_value = true, .value_hint = "KEY", .detail = "Binary key."},
    {.name = "-keyhex", .takes_value = true, .value_hint = "KEY_HEX", .detail = "Hex-encoded key."},
    {.name = "-signature", .takes_value = true, .value_hint = "SIGNATURE", .detail = "Signature to verify against."},
};

inline constexpr FormDesc irules_misc_CRYPTO__verify_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_CRYPTO__verify_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_CRYPTO__verify_cmd{
    .name = "CRYPTO::verify",
    .options = irules_misc_CRYPTO__verify_opts,
    .forms = irules_misc_CRYPTO__verify_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_CRYPTO__verify_effects,
    .hover_index = 1030,
};

// DATAGRAM::dns
inline constexpr FormDesc irules_misc_DATAGRAM__dns_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DATAGRAM__dns_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DATAGRAM__dns_cmd{
    .name = "DATAGRAM::dns",
    .forms = irules_misc_DATAGRAM__dns_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DATAGRAM__dns_effects,
    .hover_index = 1031,
};

// DATAGRAM::ip
inline constexpr FormDesc irules_misc_DATAGRAM__ip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DATAGRAM__ip_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DATAGRAM__ip_cmd{
    .name = "DATAGRAM::ip",
    .forms = irules_misc_DATAGRAM__ip_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DATAGRAM__ip_effects,
    .hover_index = 1032,
};

// DATAGRAM::ip6
inline constexpr FormDesc irules_misc_DATAGRAM__ip6_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DATAGRAM__ip6_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DATAGRAM__ip6_cmd{
    .name = "DATAGRAM::ip6",
    .forms = irules_misc_DATAGRAM__ip6_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DATAGRAM__ip6_effects,
    .hover_index = 1033,
};

// DATAGRAM::l2
inline constexpr FormDesc irules_misc_DATAGRAM__l2_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DATAGRAM__l2_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DATAGRAM__l2_cmd{
    .name = "DATAGRAM::l2",
    .forms = irules_misc_DATAGRAM__l2_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DATAGRAM__l2_effects,
    .hover_index = 1034,
};

// DATAGRAM::tcp
inline constexpr FormDesc irules_misc_DATAGRAM__tcp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DATAGRAM__tcp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DATAGRAM__tcp_cmd{
    .name = "DATAGRAM::tcp",
    .forms = irules_misc_DATAGRAM__tcp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DATAGRAM__tcp_effects,
    .hover_index = 1035,
};

// DATAGRAM::udp
inline constexpr FormDesc irules_misc_DATAGRAM__udp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DATAGRAM__udp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DATAGRAM__udp_cmd{
    .name = "DATAGRAM::udp",
    .forms = irules_misc_DATAGRAM__udp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DATAGRAM__udp_effects,
    .hover_index = 1036,
};

// DECOMPRESS::disable
inline constexpr FormDesc irules_misc_DECOMPRESS__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DECOMPRESS__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DECOMPRESS__disable_cmd{
    .name = "DECOMPRESS::disable",
    .forms = irules_misc_DECOMPRESS__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DECOMPRESS__disable_effects,
    .hover_index = 1037,
};

// DECOMPRESS::enable
inline constexpr FormDesc irules_misc_DECOMPRESS__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DECOMPRESS__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DECOMPRESS__enable_cmd{
    .name = "DECOMPRESS::enable",
    .forms = irules_misc_DECOMPRESS__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DECOMPRESS__enable_effects,
    .hover_index = 1038,
};

// DEMANGLE::disable
inline constexpr FormDesc irules_misc_DEMANGLE__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DEMANGLE__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DEMANGLE__disable_cmd{
    .name = "DEMANGLE::disable",
    .forms = irules_misc_DEMANGLE__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DEMANGLE__disable_effects,
    .hover_index = 1039,
};

// DEMANGLE::enable
inline constexpr FormDesc irules_misc_DEMANGLE__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DEMANGLE__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DEMANGLE__enable_cmd{
    .name = "DEMANGLE::enable",
    .forms = irules_misc_DEMANGLE__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DEMANGLE__enable_effects,
    .hover_index = 1040,
};

// DHCP::version
inline constexpr FormDesc irules_misc_DHCP__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCP__version_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCP__version_cmd{
    .name = "DHCP::version",
    .forms = irules_misc_DHCP__version_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCP__version_effects,
    .hover_index = 1041,
};

// DHCPv4::chaddr
inline constexpr FormDesc irules_misc_DHCPv4__chaddr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__chaddr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__chaddr_cmd{
    .name = "DHCPv4::chaddr",
    .forms = irules_misc_DHCPv4__chaddr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__chaddr_effects,
    .hover_index = 1042,
};

// DHCPv4::ciaddr
inline constexpr FormDesc irules_misc_DHCPv4__ciaddr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__ciaddr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__ciaddr_cmd{
    .name = "DHCPv4::ciaddr",
    .forms = irules_misc_DHCPv4__ciaddr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__ciaddr_effects,
    .hover_index = 1043,
};

// DHCPv4::drop
inline constexpr FormDesc irules_misc_DHCPv4__drop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__drop_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__drop_cmd{
    .name = "DHCPv4::drop",
    .forms = irules_misc_DHCPv4__drop_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__drop_effects,
    .hover_index = 1044,
};

// DHCPv4::giaddr
inline constexpr FormDesc irules_misc_DHCPv4__giaddr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__giaddr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__giaddr_cmd{
    .name = "DHCPv4::giaddr",
    .forms = irules_misc_DHCPv4__giaddr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__giaddr_effects,
    .hover_index = 1045,
};

// DHCPv4::hlen
inline constexpr FormDesc irules_misc_DHCPv4__hlen_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__hlen_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__hlen_cmd{
    .name = "DHCPv4::hlen",
    .forms = irules_misc_DHCPv4__hlen_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__hlen_effects,
    .hover_index = 1046,
};

// DHCPv4::hops
inline constexpr FormDesc irules_misc_DHCPv4__hops_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__hops_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__hops_cmd{
    .name = "DHCPv4::hops",
    .forms = irules_misc_DHCPv4__hops_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__hops_effects,
    .hover_index = 1047,
};

// DHCPv4::htype
inline constexpr FormDesc irules_misc_DHCPv4__htype_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__htype_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__htype_cmd{
    .name = "DHCPv4::htype",
    .forms = irules_misc_DHCPv4__htype_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__htype_effects,
    .hover_index = 1048,
};

// DHCPv4::len
inline constexpr FormDesc irules_misc_DHCPv4__len_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__len_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__len_cmd{
    .name = "DHCPv4::len",
    .forms = irules_misc_DHCPv4__len_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__len_effects,
    .hover_index = 1049,
};

// DHCPv4::opcode
inline constexpr FormDesc irules_misc_DHCPv4__opcode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__opcode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__opcode_cmd{
    .name = "DHCPv4::opcode",
    .forms = irules_misc_DHCPv4__opcode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__opcode_effects,
    .hover_index = 1050,
};

// DHCPv4::option
inline constexpr FormDesc irules_misc_DHCPv4__option_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__option_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__option_cmd{
    .name = "DHCPv4::option",
    .forms = irules_misc_DHCPv4__option_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__option_effects,
    .hover_index = 1051,
};

// DHCPv4::reject
inline constexpr FormDesc irules_misc_DHCPv4__reject_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__reject_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__reject_cmd{
    .name = "DHCPv4::reject",
    .forms = irules_misc_DHCPv4__reject_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__reject_effects,
    .hover_index = 1052,
};

// DHCPv4::secs
inline constexpr FormDesc irules_misc_DHCPv4__secs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__secs_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__secs_cmd{
    .name = "DHCPv4::secs",
    .forms = irules_misc_DHCPv4__secs_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__secs_effects,
    .hover_index = 1053,
};

// DHCPv4::siaddr
inline constexpr FormDesc irules_misc_DHCPv4__siaddr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__siaddr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__siaddr_cmd{
    .name = "DHCPv4::siaddr",
    .forms = irules_misc_DHCPv4__siaddr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__siaddr_effects,
    .hover_index = 1054,
};

// DHCPv4::type
inline constexpr FormDesc irules_misc_DHCPv4__type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__type_cmd{
    .name = "DHCPv4::type",
    .forms = irules_misc_DHCPv4__type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__type_effects,
    .hover_index = 1055,
};

// DHCPv4::xid
inline constexpr FormDesc irules_misc_DHCPv4__xid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__xid_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__xid_cmd{
    .name = "DHCPv4::xid",
    .forms = irules_misc_DHCPv4__xid_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__xid_effects,
    .hover_index = 1056,
};

// DHCPv4::yiaddr
inline constexpr FormDesc irules_misc_DHCPv4__yiaddr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv4__yiaddr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv4__yiaddr_cmd{
    .name = "DHCPv4::yiaddr",
    .forms = irules_misc_DHCPv4__yiaddr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv4__yiaddr_effects,
    .hover_index = 1057,
};

// DHCPv6::drop
inline constexpr FormDesc irules_misc_DHCPv6__drop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__drop_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__drop_cmd{
    .name = "DHCPv6::drop",
    .forms = irules_misc_DHCPv6__drop_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__drop_effects,
    .hover_index = 1058,
};

// DHCPv6::hop_count
inline constexpr FormDesc irules_misc_DHCPv6__hop_count_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__hop_count_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__hop_count_cmd{
    .name = "DHCPv6::hop_count",
    .forms = irules_misc_DHCPv6__hop_count_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__hop_count_effects,
    .hover_index = 1059,
};

// DHCPv6::len
inline constexpr FormDesc irules_misc_DHCPv6__len_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__len_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__len_cmd{
    .name = "DHCPv6::len",
    .forms = irules_misc_DHCPv6__len_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__len_effects,
    .hover_index = 1060,
};

// DHCPv6::link_address
inline constexpr FormDesc irules_misc_DHCPv6__link_address_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__link_address_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__link_address_cmd{
    .name = "DHCPv6::link_address",
    .forms = irules_misc_DHCPv6__link_address_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__link_address_effects,
    .hover_index = 1061,
};

// DHCPv6::msg_type
inline constexpr FormDesc irules_misc_DHCPv6__msg_type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__msg_type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__msg_type_cmd{
    .name = "DHCPv6::msg_type",
    .forms = irules_misc_DHCPv6__msg_type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__msg_type_effects,
    .hover_index = 1062,
};

// DHCPv6::option
inline constexpr FormDesc irules_misc_DHCPv6__option_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__option_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__option_cmd{
    .name = "DHCPv6::option",
    .forms = irules_misc_DHCPv6__option_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__option_effects,
    .hover_index = 1063,
};

// DHCPv6::peer_address
inline constexpr FormDesc irules_misc_DHCPv6__peer_address_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__peer_address_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__peer_address_cmd{
    .name = "DHCPv6::peer_address",
    .forms = irules_misc_DHCPv6__peer_address_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__peer_address_effects,
    .hover_index = 1064,
};

// DHCPv6::reject
inline constexpr FormDesc irules_misc_DHCPv6__reject_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__reject_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__reject_cmd{
    .name = "DHCPv6::reject",
    .forms = irules_misc_DHCPv6__reject_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__reject_effects,
    .hover_index = 1065,
};

// DHCPv6::transaction_id
inline constexpr FormDesc irules_misc_DHCPv6__transaction_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DHCPv6__transaction_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DHCPv6__transaction_id_cmd{
    .name = "DHCPv6::transaction_id",
    .forms = irules_misc_DHCPv6__transaction_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DHCPv6__transaction_id_effects,
    .hover_index = 1066,
};

// DIAG::test
inline constexpr FormDesc irules_misc_DIAG__test_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAG__test_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_DIAG__test_cmd{
    .name = "DIAG::test",
    .forms = irules_misc_DIAG__test_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DIAG__test_effects,
    .hover_index = 1067,
};

// DIAMETER::avp
inline constexpr FormDesc irules_misc_DIAMETER__avp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__avp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_DIAMETER__avp_subcmds[] = {
    {.name = "code", .arity = {1, 3}, .pure = true, .hover_index = 1068},
    {.name = "count", .arity = {1, 2}, .pure = true, .hover_index = 1069},
    {.name = "length", .arity = {1, 3}, .pure = true, .hover_index = 1070},
    {.name = "create", .arity = {2, 3}, .mutator = true, .hover_index = 1071},
    {.name = "data", .arity = {1, 3}, .pure = true, .mutator = true, .hover_index = 1072},
    {.name = "delete", .arity = {1, 3}, .mutator = true, .hover_index = 1073},
    {.name = "replace", .arity = {2, 4}, .mutator = true, .hover_index = 1074},
    {.name = "insert", .arity = {3, 4}, .mutator = true, .hover_index = 1075},
    {.name = "append", .arity = {2, 3}, .mutator = true, .hover_index = 1076},
    {.name = "flags", .arity = {2, 5}, .pure = true, .mutator = true, .hover_index = 1077},
};

inline constexpr CommandDesc irules_misc_DIAMETER__avp_cmd{
    .name = "DIAMETER::avp",
    .subcommands = irules_misc_DIAMETER__avp_subcmds,
    .forms = irules_misc_DIAMETER__avp_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__avp_effects,
    .hover_index = 1078,
};

// DIAMETER::command
inline constexpr FormDesc irules_misc_DIAMETER__command_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__command_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__command_cmd{
    .name = "DIAMETER::command",
    .forms = irules_misc_DIAMETER__command_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__command_effects,
    .hover_index = 1079,
};

// DIAMETER::disconnect
inline constexpr FormDesc irules_misc_DIAMETER__disconnect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__disconnect_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__disconnect_cmd{
    .name = "DIAMETER::disconnect",
    .forms = irules_misc_DIAMETER__disconnect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__disconnect_effects,
    .hover_index = 1080,
};

// DIAMETER::drop
inline constexpr FormDesc irules_misc_DIAMETER__drop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__drop_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__drop_cmd{
    .name = "DIAMETER::drop",
    .forms = irules_misc_DIAMETER__drop_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__drop_effects,
    .hover_index = 1081,
};

// DIAMETER::dynamic_route_insertion
inline constexpr FormDesc irules_misc_DIAMETER__dynamic_route_insertion_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__dynamic_route_insertion_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__dynamic_route_insertion_cmd{
    .name = "DIAMETER::dynamic_route_insertion",
    .forms = irules_misc_DIAMETER__dynamic_route_insertion_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DIAMETER__dynamic_route_insertion_effects,
    .hover_index = 1082,
};

// DIAMETER::dynamic_route_lookup
inline constexpr FormDesc irules_misc_DIAMETER__dynamic_route_lookup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__dynamic_route_lookup_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__dynamic_route_lookup_cmd{
    .name = "DIAMETER::dynamic_route_lookup",
    .forms = irules_misc_DIAMETER__dynamic_route_lookup_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__dynamic_route_lookup_effects,
    .hover_index = 1083,
};

// DIAMETER::header
inline constexpr FormDesc irules_misc_DIAMETER__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__header_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_DIAMETER__header_subcmds[] = {
    {.name = "version", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1084},
    {.name = "length", .arity = {0, 0}, .pure = true, .hover_index = 1085},
    {.name = "rflag", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1086},
    {.name = "pflag", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1087},
    {.name = "eflag", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1088},
    {.name = "tflag", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1089},
    {.name = "command_code", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1090},
    {.name = "application_id", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1091},
    {.name = "hop_by_hop_id", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1092},
    {.name = "end_to_end_id", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1093},
};

inline constexpr CommandDesc irules_misc_DIAMETER__header_cmd{
    .name = "DIAMETER::header",
    .subcommands = irules_misc_DIAMETER__header_subcmds,
    .forms = irules_misc_DIAMETER__header_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__header_effects,
    .hover_index = 1094,
};

// DIAMETER::host
inline constexpr FormDesc irules_misc_DIAMETER__host_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__host_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__host_cmd{
    .name = "DIAMETER::host",
    .forms = irules_misc_DIAMETER__host_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__host_effects,
    .hover_index = 1095,
};

// DIAMETER::is_request
inline constexpr FormDesc irules_misc_DIAMETER__is_request_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__is_request_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__is_request_cmd{
    .name = "DIAMETER::is_request",
    .forms = irules_misc_DIAMETER__is_request_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__is_request_effects,
    .hover_index = 1096,
};

// DIAMETER::is_response
inline constexpr FormDesc irules_misc_DIAMETER__is_response_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__is_response_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__is_response_cmd{
    .name = "DIAMETER::is_response",
    .forms = irules_misc_DIAMETER__is_response_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__is_response_effects,
    .hover_index = 1097,
};

// DIAMETER::is_retransmission
inline constexpr FormDesc irules_misc_DIAMETER__is_retransmission_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__is_retransmission_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__is_retransmission_cmd{
    .name = "DIAMETER::is_retransmission",
    .forms = irules_misc_DIAMETER__is_retransmission_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__is_retransmission_effects,
    .hover_index = 1098,
};

// DIAMETER::length
inline constexpr FormDesc irules_misc_DIAMETER__length_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__length_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__length_cmd{
    .name = "DIAMETER::length",
    .forms = irules_misc_DIAMETER__length_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__length_effects,
    .hover_index = 1099,
};

// DIAMETER::message
inline constexpr FormDesc irules_misc_DIAMETER__message_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__message_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__message_cmd{
    .name = "DIAMETER::message",
    .forms = irules_misc_DIAMETER__message_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__message_effects,
    .hover_index = 1100,
};

// DIAMETER::payload
inline constexpr FormDesc irules_misc_DIAMETER__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__payload_cmd{
    .name = "DIAMETER::payload",
    .forms = irules_misc_DIAMETER__payload_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__payload_effects,
    .hover_index = 1101,
};

// DIAMETER::persist
inline constexpr FormDesc irules_misc_DIAMETER__persist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__persist_effects[] = {
    {.target = SideEffectTarget::PERSISTENCE_TABLE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__persist_cmd{
    .name = "DIAMETER::persist",
    .forms = irules_misc_DIAMETER__persist_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__persist_effects,
    .hover_index = 1102,
};

// DIAMETER::realm
inline constexpr FormDesc irules_misc_DIAMETER__realm_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__realm_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__realm_cmd{
    .name = "DIAMETER::realm",
    .forms = irules_misc_DIAMETER__realm_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__realm_effects,
    .hover_index = 1103,
};

// DIAMETER::respond
inline constexpr FormDesc irules_misc_DIAMETER__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__respond_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__respond_cmd{
    .name = "DIAMETER::respond",
    .forms = irules_misc_DIAMETER__respond_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__respond_effects,
    .hover_index = 1104,
};

// DIAMETER::result
inline constexpr FormDesc irules_misc_DIAMETER__result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__result_cmd{
    .name = "DIAMETER::result",
    .forms = irules_misc_DIAMETER__result_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__result_effects,
    .hover_index = 1105,
};

// DIAMETER::retransmission
inline constexpr FormDesc irules_misc_DIAMETER__retransmission_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__retransmission_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__retransmission_cmd{
    .name = "DIAMETER::retransmission",
    .forms = irules_misc_DIAMETER__retransmission_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__retransmission_effects,
    .hover_index = 1106,
};

// DIAMETER::retransmission_default
inline constexpr FormDesc irules_misc_DIAMETER__retransmission_default_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__retransmission_default_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__retransmission_default_cmd{
    .name = "DIAMETER::retransmission_default",
    .forms = irules_misc_DIAMETER__retransmission_default_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DIAMETER__retransmission_default_effects,
    .hover_index = 1107,
};

// DIAMETER::retransmission_reason
inline constexpr FormDesc irules_misc_DIAMETER__retransmission_reason_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__retransmission_reason_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__retransmission_reason_cmd{
    .name = "DIAMETER::retransmission_reason",
    .forms = irules_misc_DIAMETER__retransmission_reason_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__retransmission_reason_effects,
    .hover_index = 1108,
};

// DIAMETER::retransmit
inline constexpr FormDesc irules_misc_DIAMETER__retransmit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__retransmit_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__retransmit_cmd{
    .name = "DIAMETER::retransmit",
    .forms = irules_misc_DIAMETER__retransmit_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__retransmit_effects,
    .hover_index = 1109,
};

// DIAMETER::retry
inline constexpr FormDesc irules_misc_DIAMETER__retry_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__retry_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__retry_cmd{
    .name = "DIAMETER::retry",
    .forms = irules_misc_DIAMETER__retry_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__retry_effects,
    .hover_index = 1110,
};

// DIAMETER::route_status
inline constexpr FormDesc irules_misc_DIAMETER__route_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__route_status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__route_status_cmd{
    .name = "DIAMETER::route_status",
    .forms = irules_misc_DIAMETER__route_status_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__route_status_effects,
    .hover_index = 1111,
};

// DIAMETER::session
inline constexpr FormDesc irules_misc_DIAMETER__session_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__session_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__session_cmd{
    .name = "DIAMETER::session",
    .forms = irules_misc_DIAMETER__session_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__session_effects,
    .hover_index = 1112,
};

// DIAMETER::skip_capabilities_exchange
inline constexpr FormDesc irules_misc_DIAMETER__skip_capabilities_exchange_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__skip_capabilities_exchange_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__skip_capabilities_exchange_cmd{
    .name = "DIAMETER::skip_capabilities_exchange",
    .forms = irules_misc_DIAMETER__skip_capabilities_exchange_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DIAMETER__skip_capabilities_exchange_effects,
    .hover_index = 1113,
};

// DIAMETER::state
inline constexpr FormDesc irules_misc_DIAMETER__state_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DIAMETER__state_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DIAMETER__state_cmd{
    .name = "DIAMETER::state",
    .forms = irules_misc_DIAMETER__state_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DIAMETER},
    .side_effect_hints = irules_misc_DIAMETER__state_effects,
    .hover_index = 1114,
};

// DNS::additional
inline constexpr FormDesc irules_misc_DNS__additional_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__additional_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_DNS__additional_subcmds[] = {
    {.name = "clear", .arity = {0, 0}, .mutator = true, .hover_index = 1115},
    {.name = "insert", .arity = {1, 1}, .mutator = true, .hover_index = 1116},
    {.name = "remove", .arity = {1, 1}, .mutator = true, .hover_index = 1117},
};

inline constexpr CommandDesc irules_misc_DNS__additional_cmd{
    .name = "DNS::additional",
    .subcommands = irules_misc_DNS__additional_subcmds,
    .forms = irules_misc_DNS__additional_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__additional_effects,
    .hover_index = 1118,
};

// DNS::answer
inline constexpr FormDesc irules_misc_DNS__answer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__answer_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_DNS__answer_subcmds[] = {
    {.name = "clear", .arity = {0, 0}, .mutator = true, .hover_index = 1119},
    {.name = "insert", .arity = {1, 1}, .mutator = true, .hover_index = 1120},
    {.name = "remove", .arity = {1, 1}, .mutator = true, .hover_index = 1121},
};

inline constexpr CommandDesc irules_misc_DNS__answer_cmd{
    .name = "DNS::answer",
    .subcommands = irules_misc_DNS__answer_subcmds,
    .forms = irules_misc_DNS__answer_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .diagram_action = true,
    .side_effect_hints = irules_misc_DNS__answer_effects,
    .hover_index = 1122,
};

// DNS::authority
inline constexpr FormDesc irules_misc_DNS__authority_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__authority_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_DNS__authority_subcmds[] = {
    {.name = "clear", .arity = {0, 0}, .mutator = true, .hover_index = 1123},
    {.name = "insert", .arity = {1, 1}, .mutator = true, .hover_index = 1124},
    {.name = "remove", .arity = {1, 1}, .mutator = true, .hover_index = 1125},
};

inline constexpr CommandDesc irules_misc_DNS__authority_cmd{
    .name = "DNS::authority",
    .subcommands = irules_misc_DNS__authority_subcmds,
    .forms = irules_misc_DNS__authority_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__authority_effects,
    .hover_index = 1126,
};

// DNS::class
inline constexpr FormDesc irules_misc_DNS__class_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__class_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__class_cmd{
    .name = "DNS::class",
    .forms = irules_misc_DNS__class_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__class_effects,
    .hover_index = 1127,
};

// DNS::disable
inline constexpr FormDesc irules_misc_DNS__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__disable_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__disable_cmd{
    .name = "DNS::disable",
    .forms = irules_misc_DNS__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__disable_effects,
    .hover_index = 1128,
};

// DNS::drop
inline constexpr FormDesc irules_misc_DNS__drop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__drop_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__drop_cmd{
    .name = "DNS::drop",
    .forms = irules_misc_DNS__drop_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__drop_effects,
    .hover_index = 1129,
};

// DNS::edns0
inline constexpr FormDesc irules_misc_DNS__edns0_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__edns0_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__edns0_cmd{
    .name = "DNS::edns0",
    .forms = irules_misc_DNS__edns0_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__edns0_effects,
    .hover_index = 1130,
};

// DNS::enable
inline constexpr FormDesc irules_misc_DNS__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__enable_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__enable_cmd{
    .name = "DNS::enable",
    .forms = irules_misc_DNS__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__enable_effects,
    .hover_index = 1131,
};

// DNS::header
inline constexpr FormDesc irules_misc_DNS__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__header_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_DNS__header_subcmds[] = {
    {.name = "ra", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1132},
    {.name = "rcode", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1133},
    {.name = "id", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1134},
    {.name = "qr", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1135},
    {.name = "tc", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1136},
    {.name = "rd", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1137},
    {.name = "ad", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1138},
    {.name = "ancount", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1139},
    {.name = "nscount", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1140},
    {.name = "aa", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1141},
    {.name = "arcount", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1142},
    {.name = "opcode", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1143},
    {.name = "qdcount", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1144},
    {.name = "cd", .arity = {0, 1}, .pure = true, .mutator = true, .hover_index = 1145},
};

inline constexpr CommandDesc irules_misc_DNS__header_cmd{
    .name = "DNS::header",
    .subcommands = irules_misc_DNS__header_subcmds,
    .forms = irules_misc_DNS__header_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .diagram_action = true,
    .side_effect_hints = irules_misc_DNS__header_effects,
    .hover_index = 1146,
};

// DNS::is_wideip
inline constexpr FormDesc irules_misc_DNS__is_wideip_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__is_wideip_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__is_wideip_cmd{
    .name = "DNS::is_wideip",
    .forms = irules_misc_DNS__is_wideip_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__is_wideip_effects,
    .hover_index = 1147,
};

// DNS::last_act
inline constexpr FormDesc irules_misc_DNS__last_act_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__last_act_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__last_act_cmd{
    .name = "DNS::last_act",
    .forms = irules_misc_DNS__last_act_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__last_act_effects,
    .hover_index = 1148,
};

// DNS::len
inline constexpr FormDesc irules_misc_DNS__len_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__len_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__len_cmd{
    .name = "DNS::len",
    .forms = irules_misc_DNS__len_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__len_effects,
    .hover_index = 1149,
};

// DNS::log
inline constexpr FormDesc irules_misc_DNS__log_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__log_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__log_cmd{
    .name = "DNS::log",
    .forms = irules_misc_DNS__log_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__log_effects,
    .hover_index = 1150,
};

// DNS::name
inline constexpr FormDesc irules_misc_DNS__name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__name_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__name_cmd{
    .name = "DNS::name",
    .forms = irules_misc_DNS__name_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__name_effects,
    .hover_index = 1151,
};

// DNS::origin
inline constexpr FormDesc irules_misc_DNS__origin_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__origin_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__origin_cmd{
    .name = "DNS::origin",
    .forms = irules_misc_DNS__origin_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__origin_effects,
    .hover_index = 1152,
};

// DNS::ptype
inline constexpr FormDesc irules_misc_DNS__ptype_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__ptype_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__ptype_cmd{
    .name = "DNS::ptype",
    .forms = irules_misc_DNS__ptype_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__ptype_effects,
    .hover_index = 1153,
};

// DNS::query
inline constexpr FormDesc irules_misc_DNS__query_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__query_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__query_cmd{
    .name = "DNS::query",
    .forms = irules_misc_DNS__query_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__query_effects,
    .hover_index = 1154,
};

// DNS::question
inline constexpr FormDesc irules_misc_DNS__question_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__question_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__question_cmd{
    .name = "DNS::question",
    .forms = irules_misc_DNS__question_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__question_effects,
    .hover_index = 1155,
};

// DNS::rdata
inline constexpr FormDesc irules_misc_DNS__rdata_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__rdata_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__rdata_cmd{
    .name = "DNS::rdata",
    .forms = irules_misc_DNS__rdata_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__rdata_effects,
    .hover_index = 1156,
};

// DNS::return
inline constexpr FormDesc irules_misc_DNS__return_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__return_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__return_cmd{
    .name = "DNS::return",
    .forms = irules_misc_DNS__return_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .diagram_action = true,
    .side_effect_hints = irules_misc_DNS__return_effects,
    .hover_index = 1157,
};

// DNS::rpz_policy
inline constexpr FormDesc irules_misc_DNS__rpz_policy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__rpz_policy_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__rpz_policy_cmd{
    .name = "DNS::rpz_policy",
    .forms = irules_misc_DNS__rpz_policy_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__rpz_policy_effects,
    .hover_index = 1158,
};

// DNS::rr
inline constexpr FormDesc irules_misc_DNS__rr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__rr_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__rr_cmd{
    .name = "DNS::rr",
    .forms = irules_misc_DNS__rr_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__rr_effects,
    .hover_index = 1159,
};

// DNS::scrape
inline constexpr FormDesc irules_misc_DNS__scrape_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__scrape_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__scrape_cmd{
    .name = "DNS::scrape",
    .forms = irules_misc_DNS__scrape_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__scrape_effects,
    .hover_index = 1160,
};

// DNS::tsig
inline constexpr FormDesc irules_misc_DNS__tsig_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__tsig_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__tsig_cmd{
    .name = "DNS::tsig",
    .forms = irules_misc_DNS__tsig_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DNS__tsig_effects,
    .hover_index = 1161,
};

// DNS::ttl
inline constexpr FormDesc irules_misc_DNS__ttl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__ttl_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__ttl_cmd{
    .name = "DNS::ttl",
    .forms = irules_misc_DNS__ttl_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__ttl_effects,
    .hover_index = 1162,
};

// DNS::type
inline constexpr FormDesc irules_misc_DNS__type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNS__type_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNS__type_cmd{
    .name = "DNS::type",
    .forms = irules_misc_DNS__type_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_DNS__type_effects,
    .hover_index = 1163,
};

// DNSMSG::header
inline constexpr FormDesc irules_misc_DNSMSG__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNSMSG__header_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNSMSG__header_cmd{
    .name = "DNSMSG::header",
    .forms = irules_misc_DNSMSG__header_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DNSMSG__header_effects,
    .hover_index = 1164,
};

// DNSMSG::record
inline constexpr FormDesc irules_misc_DNSMSG__record_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNSMSG__record_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNSMSG__record_cmd{
    .name = "DNSMSG::record",
    .forms = irules_misc_DNSMSG__record_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DNSMSG__record_effects,
    .hover_index = 1165,
};

// DNSMSG::section
inline constexpr FormDesc irules_misc_DNSMSG__section_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DNSMSG__section_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DNSMSG__section_cmd{
    .name = "DNSMSG::section",
    .forms = irules_misc_DNSMSG__section_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DNSMSG__section_effects,
    .hover_index = 1166,
};

// DOSL7::disable
inline constexpr FormDesc irules_misc_DOSL7__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DOSL7__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DOSL7__disable_cmd{
    .name = "DOSL7::disable",
    .forms = irules_misc_DOSL7__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DOSL7__disable_effects,
    .hover_index = 1167,
};

// DOSL7::enable
inline constexpr FormDesc irules_misc_DOSL7__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DOSL7__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DOSL7__enable_cmd{
    .name = "DOSL7::enable",
    .forms = irules_misc_DOSL7__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DOSL7__enable_effects,
    .hover_index = 1168,
};

// DOSL7::health
inline constexpr FormDesc irules_misc_DOSL7__health_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DOSL7__health_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_DOSL7__health_cmd{
    .name = "DOSL7::health",
    .forms = irules_misc_DOSL7__health_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DOSL7__health_effects,
    .hover_index = 1169,
};

// DOSL7::is_ip_slowdown
inline constexpr FormDesc irules_misc_DOSL7__is_ip_slowdown_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DOSL7__is_ip_slowdown_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DOSL7__is_ip_slowdown_cmd{
    .name = "DOSL7::is_ip_slowdown",
    .forms = irules_misc_DOSL7__is_ip_slowdown_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DOSL7__is_ip_slowdown_effects,
    .hover_index = 1170,
};

// DOSL7::is_mitigated
inline constexpr FormDesc irules_misc_DOSL7__is_mitigated_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DOSL7__is_mitigated_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DOSL7__is_mitigated_cmd{
    .name = "DOSL7::is_mitigated",
    .forms = irules_misc_DOSL7__is_mitigated_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DOSL7__is_mitigated_effects,
    .hover_index = 1171,
};

// DOSL7::profile
inline constexpr FormDesc irules_misc_DOSL7__profile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DOSL7__profile_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_DOSL7__profile_cmd{
    .name = "DOSL7::profile",
    .forms = irules_misc_DOSL7__profile_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_DOSL7__profile_effects,
    .hover_index = 1172,
};

// DOSL7::slowdown
inline constexpr FormDesc irules_misc_DOSL7__slowdown_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DOSL7__slowdown_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DOSL7__slowdown_cmd{
    .name = "DOSL7::slowdown",
    .forms = irules_misc_DOSL7__slowdown_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DOSL7__slowdown_effects,
    .hover_index = 1173,
};

// DSLITE::remote_addr
inline constexpr FormDesc irules_misc_DSLITE__remote_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_DSLITE__remote_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_DSLITE__remote_addr_cmd{
    .name = "DSLITE::remote_addr",
    .forms = irules_misc_DSLITE__remote_addr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_DSLITE__remote_addr_effects,
    .hover_index = 1174,
};

// ECA::client_machine_name
inline constexpr FormDesc irules_misc_ECA__client_machine_name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ECA__client_machine_name_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ECA__client_machine_name_cmd{
    .name = "ECA::client_machine_name",
    .forms = irules_misc_ECA__client_machine_name_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ECA__client_machine_name_effects,
    .hover_index = 1175,
};

// ECA::disable
inline constexpr FormDesc irules_misc_ECA__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ECA__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ECA__disable_cmd{
    .name = "ECA::disable",
    .forms = irules_misc_ECA__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ECA__disable_effects,
    .hover_index = 1176,
};

// ECA::domainname
inline constexpr FormDesc irules_misc_ECA__domainname_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ECA__domainname_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ECA__domainname_cmd{
    .name = "ECA::domainname",
    .forms = irules_misc_ECA__domainname_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ECA__domainname_effects,
    .hover_index = 1177,
};

// ECA::enable
inline constexpr FormDesc irules_misc_ECA__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ECA__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ECA__enable_cmd{
    .name = "ECA::enable",
    .forms = irules_misc_ECA__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ECA__enable_effects,
    .hover_index = 1178,
};

// ECA::select
inline constexpr FormDesc irules_misc_ECA__select_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ECA__select_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ECA__select_cmd{
    .name = "ECA::select",
    .forms = irules_misc_ECA__select_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ECA__select_effects,
    .hover_index = 1179,
};

// ECA::status
inline constexpr FormDesc irules_misc_ECA__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ECA__status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ECA__status_cmd{
    .name = "ECA::status",
    .forms = irules_misc_ECA__status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ECA__status_effects,
    .hover_index = 1180,
};

// ECA::username
inline constexpr FormDesc irules_misc_ECA__username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ECA__username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ECA__username_cmd{
    .name = "ECA::username",
    .forms = irules_misc_ECA__username_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ECA__username_effects,
    .hover_index = 1181,
};

// FIX::tag
inline constexpr FormDesc irules_misc_FIX__tag_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FIX__tag_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FIX__tag_cmd{
    .name = "FIX::tag",
    .forms = irules_misc_FIX__tag_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FIX__tag_effects,
    .hover_index = 1182,
};

// FLOW::create_related
inline constexpr OptionDesc irules_misc_FLOW__create_related_opts[] = {
    {.name = "-translation-loose", .detail = "Option -translation-loose."},
    {.name = "-hairpin", .detail = "Option -hairpin."},
};

inline constexpr FormDesc irules_misc_FLOW__create_related_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOW__create_related_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOW__create_related_cmd{
    .name = "FLOW::create_related",
    .options = irules_misc_FLOW__create_related_opts,
    .forms = irules_misc_FLOW__create_related_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOW__create_related_effects,
    .hover_index = 1183,
};

// FLOW::idle_duration
inline constexpr FormDesc irules_misc_FLOW__idle_duration_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOW__idle_duration_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOW__idle_duration_cmd{
    .name = "FLOW::idle_duration",
    .forms = irules_misc_FLOW__idle_duration_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOW__idle_duration_effects,
    .hover_index = 1184,
};

// FLOW::idle_timeout
inline constexpr FormDesc irules_misc_FLOW__idle_timeout_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOW__idle_timeout_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOW__idle_timeout_cmd{
    .name = "FLOW::idle_timeout",
    .forms = irules_misc_FLOW__idle_timeout_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOW__idle_timeout_effects,
    .hover_index = 1185,
};

// FLOW::peer
inline constexpr FormDesc irules_misc_FLOW__peer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOW__peer_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOW__peer_cmd{
    .name = "FLOW::peer",
    .forms = irules_misc_FLOW__peer_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOW__peer_effects,
    .hover_index = 1186,
};

// FLOW::priority
inline constexpr FormDesc irules_misc_FLOW__priority_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOW__priority_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOW__priority_cmd{
    .name = "FLOW::priority",
    .forms = irules_misc_FLOW__priority_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOW__priority_effects,
    .hover_index = 1187,
};

// FLOW::refresh
inline constexpr FormDesc irules_misc_FLOW__refresh_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOW__refresh_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOW__refresh_cmd{
    .name = "FLOW::refresh",
    .forms = irules_misc_FLOW__refresh_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOW__refresh_effects,
    .hover_index = 1188,
};

// FLOW::this
inline constexpr FormDesc irules_misc_FLOW__this_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOW__this_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOW__this_cmd{
    .name = "FLOW::this",
    .forms = irules_misc_FLOW__this_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOW__this_effects,
    .hover_index = 1189,
};

// FLOWTABLE::count
inline constexpr FormDesc irules_misc_FLOWTABLE__count_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOWTABLE__count_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOWTABLE__count_cmd{
    .name = "FLOWTABLE::count",
    .forms = irules_misc_FLOWTABLE__count_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOWTABLE__count_effects,
    .hover_index = 1190,
};

// FLOWTABLE::limit
inline constexpr FormDesc irules_misc_FLOWTABLE__limit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FLOWTABLE__limit_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FLOWTABLE__limit_cmd{
    .name = "FLOWTABLE::limit",
    .forms = irules_misc_FLOWTABLE__limit_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FLOWTABLE__limit_effects,
    .hover_index = 1191,
};

// FTP::allow_active_mode
inline constexpr FormDesc irules_misc_FTP__allow_active_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FTP__allow_active_mode_effects[] = {
    {.target = SideEffectTarget::FTP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FTP__allow_active_mode_cmd{
    .name = "FTP::allow_active_mode",
    .forms = irules_misc_FTP__allow_active_mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FTP__allow_active_mode_effects,
    .hover_index = 1192,
};

// FTP::disable
inline constexpr FormDesc irules_misc_FTP__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FTP__disable_effects[] = {
    {.target = SideEffectTarget::FTP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FTP__disable_cmd{
    .name = "FTP::disable",
    .forms = irules_misc_FTP__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FTP__disable_effects,
    .hover_index = 1193,
};

// FTP::enable
inline constexpr FormDesc irules_misc_FTP__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FTP__enable_effects[] = {
    {.target = SideEffectTarget::FTP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FTP__enable_cmd{
    .name = "FTP::enable",
    .forms = irules_misc_FTP__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FTP__enable_effects,
    .hover_index = 1194,
};

// FTP::enforce_tls_session_reuse
inline constexpr FormDesc irules_misc_FTP__enforce_tls_session_reuse_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FTP__enforce_tls_session_reuse_effects[] = {
    {.target = SideEffectTarget::FTP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FTP__enforce_tls_session_reuse_cmd{
    .name = "FTP::enforce_tls_session_reuse",
    .forms = irules_misc_FTP__enforce_tls_session_reuse_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FTP__enforce_tls_session_reuse_effects,
    .hover_index = 1195,
};

// FTP::ftps_mode
inline constexpr FormDesc irules_misc_FTP__ftps_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FTP__ftps_mode_effects[] = {
    {.target = SideEffectTarget::FTP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FTP__ftps_mode_cmd{
    .name = "FTP::ftps_mode",
    .forms = irules_misc_FTP__ftps_mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FTP__ftps_mode_effects,
    .hover_index = 1196,
};

// FTP::port
inline constexpr FormDesc irules_misc_FTP__port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_FTP__port_effects[] = {
    {.target = SideEffectTarget::FTP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_FTP__port_cmd{
    .name = "FTP::port",
    .forms = irules_misc_FTP__port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_FTP__port_effects,
    .hover_index = 1197,
};

// GENERICMESSAGE::message
inline constexpr FormDesc irules_misc_GENERICMESSAGE__message_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GENERICMESSAGE__message_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GENERICMESSAGE__message_cmd{
    .name = "GENERICMESSAGE::message",
    .forms = irules_misc_GENERICMESSAGE__message_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GENERICMESSAGE__message_effects,
    .hover_index = 1198,
};

// GENERICMESSAGE::peer
inline constexpr FormDesc irules_misc_GENERICMESSAGE__peer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GENERICMESSAGE__peer_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GENERICMESSAGE__peer_cmd{
    .name = "GENERICMESSAGE::peer",
    .forms = irules_misc_GENERICMESSAGE__peer_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GENERICMESSAGE__peer_effects,
    .hover_index = 1199,
};

// GENERICMESSAGE::route
inline constexpr FormDesc irules_misc_GENERICMESSAGE__route_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GENERICMESSAGE__route_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GENERICMESSAGE__route_cmd{
    .name = "GENERICMESSAGE::route",
    .forms = irules_misc_GENERICMESSAGE__route_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GENERICMESSAGE__route_effects,
    .hover_index = 1200,
};

// GTP::clone
inline constexpr FormDesc irules_misc_GTP__clone_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__clone_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__clone_cmd{
    .name = "GTP::clone",
    .forms = irules_misc_GTP__clone_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__clone_effects,
    .hover_index = 1201,
};

// GTP::discard
inline constexpr FormDesc irules_misc_GTP__discard_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__discard_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__discard_cmd{
    .name = "GTP::discard",
    .forms = irules_misc_GTP__discard_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__discard_effects,
    .hover_index = 1202,
};

// GTP::forward
inline constexpr FormDesc irules_misc_GTP__forward_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__forward_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__forward_cmd{
    .name = "GTP::forward",
    .forms = irules_misc_GTP__forward_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__forward_effects,
    .hover_index = 1203,
};

// GTP::header
inline constexpr OptionDesc irules_misc_GTP__header_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on specific message."},
};

inline constexpr FormDesc irules_misc_GTP__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__header_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr OptionDesc irules_misc_GTP__header_sub_version_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on specific message."},
};

inline constexpr OptionDesc irules_misc_GTP__header_sub_type_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on specific message."},
};

inline constexpr OptionDesc irules_misc_GTP__header_sub_sequence_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on specific message."},
};

inline constexpr OptionDesc irules_misc_GTP__header_sub_teid_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on specific message."},
};

inline constexpr OptionDesc irules_misc_GTP__header_sub_npdu_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on specific message."},
};

inline constexpr OptionDesc irules_misc_GTP__header_sub_extension_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on specific message."},
};

inline constexpr SubCmdDesc irules_misc_GTP__header_subcmds[] = {
    {.name = "version", .arity = {0, 0}, .options = irules_misc_GTP__header_sub_version_opts, .pure = true, .hover_index = 1204},
    {.name = "type", .arity = {0, 0}, .options = irules_misc_GTP__header_sub_type_opts, .pure = true, .hover_index = 1205},
    {.name = "sequence", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_GTP__header_sub_sequence_opts, .pure = true, .mutator = true, .hover_index = 1206},
    {.name = "teid", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_GTP__header_sub_teid_opts, .pure = true, .mutator = true, .hover_index = 1207},
    {.name = "npdu", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_GTP__header_sub_npdu_opts, .pure = true, .mutator = true, .hover_index = 1208},
    {.name = "extension", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_GTP__header_sub_extension_opts, .pure = true, .mutator = true, .hover_index = 1209},
};

inline constexpr CommandDesc irules_misc_GTP__header_cmd{
    .name = "GTP::header",
    .subcommands = irules_misc_GTP__header_subcmds,
    .options = irules_misc_GTP__header_opts,
    .forms = irules_misc_GTP__header_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__header_effects,
    .hover_index = 1210,
};

// GTP::ie
inline constexpr OptionDesc irules_misc_GTP__ie_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on a specific GTP message object."},
    {.name = "-type", .takes_value = true, .value_hint = "TYPE", .detail = "Filter by IE type value."},
    {.name = "-instance", .takes_value = true, .value_hint = "INSTANCE", .detail = "Filter by IE instance."},
};

inline constexpr FormDesc irules_misc_GTP__ie_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__ie_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__ie_cmd{
    .name = "GTP::ie",
    .options = irules_misc_GTP__ie_opts,
    .forms = irules_misc_GTP__ie_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__ie_effects,
    .hover_index = 1211,
};

// GTP::length
inline constexpr OptionDesc irules_misc_GTP__length_opts[] = {
    {.name = "-message", .takes_value = true, .detail = "Option -message."},
};

inline constexpr FormDesc irules_misc_GTP__length_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__length_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__length_cmd{
    .name = "GTP::length",
    .options = irules_misc_GTP__length_opts,
    .forms = irules_misc_GTP__length_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__length_effects,
    .hover_index = 1212,
};

// GTP::message
inline constexpr OptionDesc irules_misc_GTP__message_opts[] = {
    {.name = "-message", .takes_value = true, .detail = "Option -message."},
};

inline constexpr FormDesc irules_misc_GTP__message_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__message_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__message_cmd{
    .name = "GTP::message",
    .options = irules_misc_GTP__message_opts,
    .forms = irules_misc_GTP__message_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__message_effects,
    .hover_index = 1213,
};

// GTP::new
inline constexpr FormDesc irules_misc_GTP__new_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__new_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__new_cmd{
    .name = "GTP::new",
    .forms = irules_misc_GTP__new_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__new_effects,
    .hover_index = 1214,
};

// GTP::parse
inline constexpr FormDesc irules_misc_GTP__parse_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__parse_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__parse_cmd{
    .name = "GTP::parse",
    .forms = irules_misc_GTP__parse_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__parse_effects,
    .hover_index = 1215,
};

// GTP::payload
inline constexpr OptionDesc irules_misc_GTP__payload_opts[] = {
    {.name = "-message", .takes_value = true, .detail = "Option -message."},
};

inline constexpr FormDesc irules_misc_GTP__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__payload_cmd{
    .name = "GTP::payload",
    .options = irules_misc_GTP__payload_opts,
    .forms = irules_misc_GTP__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__payload_effects,
    .hover_index = 1216,
};

// GTP::respond
inline constexpr FormDesc irules_misc_GTP__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__respond_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__respond_cmd{
    .name = "GTP::respond",
    .forms = irules_misc_GTP__respond_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__respond_effects,
    .hover_index = 1217,
};

// GTP::tunnel
inline constexpr OptionDesc irules_misc_GTP__tunnel_opts[] = {
    {.name = "-message", .takes_value = true, .value_hint = "MESSAGE", .detail = "Operate on a specific GTP message object."},
};

inline constexpr FormDesc irules_misc_GTP__tunnel_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_GTP__tunnel_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_GTP__tunnel_cmd{
    .name = "GTP::tunnel",
    .options = irules_misc_GTP__tunnel_opts,
    .forms = irules_misc_GTP__tunnel_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_GTP__tunnel_effects,
    .hover_index = 1218,
};

// HA::status
inline constexpr FormDesc irules_misc_HA__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_HA__status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_HA__status_cmd{
    .name = "HA::status",
    .forms = irules_misc_HA__status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_HA__status_effects,
    .hover_index = 1219,
};

// HSL::open
inline constexpr OptionDesc irules_misc_HSL__open_opts[] = {
    {.name = "-publisher", .takes_value = true, .detail = "Option -publisher."},
    {.name = "-pub", .takes_value = true, .detail = "Option -pub."},
    {.name = "-proto", .takes_value = true, .detail = "Option -proto."},
    {.name = "-pool", .takes_value = true, .detail = "Option -pool."},
};

inline constexpr FormDesc irules_misc_HSL__open_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_HSL__open_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_HSL__open_cmd{
    .name = "HSL::open",
    .options = irules_misc_HSL__open_opts,
    .forms = irules_misc_HSL__open_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_HSL__open_effects,
    .hover_index = 1220,
};

// HSL::send
inline constexpr FormDesc irules_misc_HSL__send_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_HSL__send_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_HSL__send_cmd{
    .name = "HSL::send",
    .forms = irules_misc_HSL__send_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_HSL__send_effects,
    .hover_index = 1221,
};

// HTML::comment
inline constexpr FormDesc irules_misc_HTML__comment_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_HTML__comment_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_HTML__comment_cmd{
    .name = "HTML::comment",
    .forms = irules_misc_HTML__comment_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_HTML__comment_effects,
    .hover_index = 1222,
};

// HTML::disable
inline constexpr FormDesc irules_misc_HTML__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_HTML__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_HTML__disable_cmd{
    .name = "HTML::disable",
    .forms = irules_misc_HTML__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_HTML__disable_effects,
    .hover_index = 1223,
};

// HTML::enable
inline constexpr FormDesc irules_misc_HTML__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_HTML__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_HTML__enable_cmd{
    .name = "HTML::enable",
    .forms = irules_misc_HTML__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_HTML__enable_effects,
    .hover_index = 1224,
};

// HTML::encode
inline constexpr FormDesc irules_misc_HTML__encode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc irules_misc_HTML__encode_cmd{
    .name = "HTML::encode",
    .forms = irules_misc_HTML__encode_forms,
    .dialects = DialectFlags::IRULES,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 1225,
};

// HTML::tag
inline constexpr FormDesc irules_misc_HTML__tag_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_HTML__tag_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_HTML__tag_cmd{
    .name = "HTML::tag",
    .forms = irules_misc_HTML__tag_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_HTML__tag_effects,
    .hover_index = 1226,
};

// ICAP::header
inline constexpr FormDesc irules_misc_ICAP__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ICAP__header_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ICAP__header_cmd{
    .name = "ICAP::header",
    .forms = irules_misc_ICAP__header_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ICAP__header_effects,
    .hover_index = 1227,
};

// ICAP::method
inline constexpr FormDesc irules_misc_ICAP__method_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ICAP__method_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ICAP__method_cmd{
    .name = "ICAP::method",
    .forms = irules_misc_ICAP__method_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ICAP__method_effects,
    .hover_index = 1228,
};

// ICAP::status
inline constexpr FormDesc irules_misc_ICAP__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ICAP__status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ICAP__status_cmd{
    .name = "ICAP::status",
    .forms = irules_misc_ICAP__status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ICAP__status_effects,
    .hover_index = 1229,
};

// ICAP::uri
inline constexpr FormDesc irules_misc_ICAP__uri_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ICAP__uri_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ICAP__uri_cmd{
    .name = "ICAP::uri",
    .forms = irules_misc_ICAP__uri_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ICAP__uri_effects,
    .hover_index = 1230,
};

// IKE::auth_success
inline constexpr FormDesc irules_misc_IKE__auth_success_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__auth_success_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__auth_success_cmd{
    .name = "IKE::auth_success",
    .forms = irules_misc_IKE__auth_success_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__auth_success_effects,
    .hover_index = 1231,
};

// IKE::cert
inline constexpr FormDesc irules_misc_IKE__cert_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__cert_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__cert_cmd{
    .name = "IKE::cert",
    .forms = irules_misc_IKE__cert_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__cert_effects,
    .hover_index = 1232,
};

// IKE::san_dirname
inline constexpr FormDesc irules_misc_IKE__san_dirname_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_dirname_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_dirname_cmd{
    .name = "IKE::san_dirname",
    .forms = irules_misc_IKE__san_dirname_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_dirname_effects,
    .hover_index = 1233,
};

// IKE::san_dns
inline constexpr FormDesc irules_misc_IKE__san_dns_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_dns_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_dns_cmd{
    .name = "IKE::san_dns",
    .forms = irules_misc_IKE__san_dns_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_dns_effects,
    .hover_index = 1234,
};

// IKE::san_ediparty
inline constexpr FormDesc irules_misc_IKE__san_ediparty_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_ediparty_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_ediparty_cmd{
    .name = "IKE::san_ediparty",
    .forms = irules_misc_IKE__san_ediparty_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_ediparty_effects,
    .hover_index = 1235,
};

// IKE::san_email
inline constexpr FormDesc irules_misc_IKE__san_email_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_email_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_email_cmd{
    .name = "IKE::san_email",
    .forms = irules_misc_IKE__san_email_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_email_effects,
    .hover_index = 1236,
};

// IKE::san_ipadd
inline constexpr FormDesc irules_misc_IKE__san_ipadd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_ipadd_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_ipadd_cmd{
    .name = "IKE::san_ipadd",
    .forms = irules_misc_IKE__san_ipadd_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_ipadd_effects,
    .hover_index = 1237,
};

// IKE::san_othername
inline constexpr FormDesc irules_misc_IKE__san_othername_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_othername_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_othername_cmd{
    .name = "IKE::san_othername",
    .forms = irules_misc_IKE__san_othername_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_othername_effects,
    .hover_index = 1238,
};

// IKE::san_rid
inline constexpr FormDesc irules_misc_IKE__san_rid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_rid_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_rid_cmd{
    .name = "IKE::san_rid",
    .forms = irules_misc_IKE__san_rid_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_rid_effects,
    .hover_index = 1239,
};

// IKE::san_uri
inline constexpr FormDesc irules_misc_IKE__san_uri_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_uri_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_uri_cmd{
    .name = "IKE::san_uri",
    .forms = irules_misc_IKE__san_uri_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_uri_effects,
    .hover_index = 1240,
};

// IKE::san_x400
inline constexpr FormDesc irules_misc_IKE__san_x400_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__san_x400_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__san_x400_cmd{
    .name = "IKE::san_x400",
    .forms = irules_misc_IKE__san_x400_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__san_x400_effects,
    .hover_index = 1241,
};

// IKE::subjectAltName
inline constexpr FormDesc irules_misc_IKE__subjectAltName_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IKE__subjectAltName_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IKE__subjectAltName_cmd{
    .name = "IKE::subjectAltName",
    .forms = irules_misc_IKE__subjectAltName_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IKE__subjectAltName_effects,
    .hover_index = 1242,
};

// ILX::call
inline constexpr FormDesc irules_misc_ILX__call_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ILX__call_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ILX__call_cmd{
    .name = "ILX::call",
    .forms = irules_misc_ILX__call_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ILX__call_effects,
    .hover_index = 1243,
};

// ILX::init
inline constexpr FormDesc irules_misc_ILX__init_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ILX__init_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ILX__init_cmd{
    .name = "ILX::init",
    .forms = irules_misc_ILX__init_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ILX__init_effects,
    .hover_index = 1244,
};

// ILX::notify
inline constexpr FormDesc irules_misc_ILX__notify_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ILX__notify_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ILX__notify_cmd{
    .name = "ILX::notify",
    .forms = irules_misc_ILX__notify_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ILX__notify_effects,
    .hover_index = 1245,
};

// IMAP::activation_mode
inline constexpr FormDesc irules_misc_IMAP__activation_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IMAP__activation_mode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IMAP__activation_mode_cmd{
    .name = "IMAP::activation_mode",
    .forms = irules_misc_IMAP__activation_mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IMAP__activation_mode_effects,
    .hover_index = 1246,
};

// IMAP::disable
inline constexpr FormDesc irules_misc_IMAP__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IMAP__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IMAP__disable_cmd{
    .name = "IMAP::disable",
    .forms = irules_misc_IMAP__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IMAP__disable_effects,
    .hover_index = 1247,
};

// IMAP::enable
inline constexpr FormDesc irules_misc_IMAP__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IMAP__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IMAP__enable_cmd{
    .name = "IMAP::enable",
    .forms = irules_misc_IMAP__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IMAP__enable_effects,
    .hover_index = 1248,
};

// IP::addr
inline constexpr OptionDesc irules_misc_IP__addr_opts[] = {
    {.name = "-swap", .detail = "Swap byte order."},
    {.name = "-ipv4", .detail = "Parse as IPv4 address."},
    {.name = "-ipv6", .detail = "Parse as IPv6 address."},
};

inline constexpr FormDesc irules_misc_IP__addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__addr_cmd{
    .name = "IP::addr",
    .options = irules_misc_IP__addr_opts,
    .forms = irules_misc_IP__addr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__addr_effects,
    .hover_index = 1249,
};

// IP::client_addr
inline constexpr FormDesc irules_misc_IP__client_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__client_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::CONNECTION},
};

inline constexpr CommandDesc irules_misc_IP__client_addr_cmd{
    .name = "IP::client_addr",
    .forms = irules_misc_IP__client_addr_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .xc_translatable = 1,
    .side_effect_hints = irules_misc_IP__client_addr_effects,
    .hover_index = 1250,
};

// IP::hops
inline constexpr FormDesc irules_misc_IP__hops_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__hops_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__hops_cmd{
    .name = "IP::hops",
    .forms = irules_misc_IP__hops_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__hops_effects,
    .hover_index = 1251,
};

// IP::idle_timeout
inline constexpr FormDesc irules_misc_IP__idle_timeout_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__idle_timeout_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__idle_timeout_cmd{
    .name = "IP::idle_timeout",
    .forms = irules_misc_IP__idle_timeout_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__idle_timeout_effects,
    .hover_index = 1252,
};

// IP::ingress_drop_rate
inline constexpr FormDesc irules_misc_IP__ingress_drop_rate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__ingress_drop_rate_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__ingress_drop_rate_cmd{
    .name = "IP::ingress_drop_rate",
    .forms = irules_misc_IP__ingress_drop_rate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__ingress_drop_rate_effects,
    .hover_index = 1253,
};

// IP::ingress_rate_limit
inline constexpr FormDesc irules_misc_IP__ingress_rate_limit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__ingress_rate_limit_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__ingress_rate_limit_cmd{
    .name = "IP::ingress_rate_limit",
    .forms = irules_misc_IP__ingress_rate_limit_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__ingress_rate_limit_effects,
    .hover_index = 1254,
};

// IP::intelligence
inline constexpr FormDesc irules_misc_IP__intelligence_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__intelligence_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__intelligence_cmd{
    .name = "IP::intelligence",
    .forms = irules_misc_IP__intelligence_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__intelligence_effects,
    .hover_index = 1255,
};

// IP::local_addr
inline constexpr FormDesc irules_misc_IP__local_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__local_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::CONNECTION},
};

inline constexpr CommandDesc irules_misc_IP__local_addr_cmd{
    .name = "IP::local_addr",
    .forms = irules_misc_IP__local_addr_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_IP__local_addr_effects,
    .hover_index = 1256,
};

// IP::protocol
inline constexpr FormDesc irules_misc_IP__protocol_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__protocol_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::CONNECTION},
};

inline constexpr CommandDesc irules_misc_IP__protocol_cmd{
    .name = "IP::protocol",
    .forms = irules_misc_IP__protocol_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_IP__protocol_effects,
    .hover_index = 1257,
};

// IP::remote_addr
inline constexpr FormDesc irules_misc_IP__remote_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__remote_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::CONNECTION},
};

inline constexpr CommandDesc irules_misc_IP__remote_addr_cmd{
    .name = "IP::remote_addr",
    .forms = irules_misc_IP__remote_addr_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_IP__remote_addr_effects,
    .hover_index = 1258,
};

// IP::reputation
inline constexpr FormDesc irules_misc_IP__reputation_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__reputation_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__reputation_cmd{
    .name = "IP::reputation",
    .forms = irules_misc_IP__reputation_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__reputation_effects,
    .hover_index = 1259,
};

// IP::server_addr
inline constexpr FormDesc irules_misc_IP__server_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__server_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_misc_IP__server_addr_cmd{
    .name = "IP::server_addr",
    .forms = irules_misc_IP__server_addr_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_IP__server_addr_effects,
    .hover_index = 1260,
};

// IP::stats
inline constexpr FormDesc irules_misc_IP__stats_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__stats_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_IP__stats_subcmds[] = {
    {.name = "pkts", .arity = {0, 1}, .pure = true, .hover_index = 1261},
    {.name = "bytes", .arity = {0, 1}, .pure = true, .hover_index = 1262},
    {.name = "in", .arity = {0, 0}, .pure = true, .hover_index = 1263},
    {.name = "out", .arity = {0, 0}, .pure = true, .hover_index = 1264},
    {.name = "age", .arity = {0, 0}, .pure = true, .hover_index = 1265},
};

inline constexpr CommandDesc irules_misc_IP__stats_cmd{
    .name = "IP::stats",
    .subcommands = irules_misc_IP__stats_subcmds,
    .forms = irules_misc_IP__stats_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__stats_effects,
    .hover_index = 1266,
};

// IP::tos
inline constexpr FormDesc irules_misc_IP__tos_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__tos_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_misc_IP__tos_cmd{
    .name = "IP::tos",
    .forms = irules_misc_IP__tos_forms,
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_IP__tos_effects,
    .hover_index = 1267,
};

// IP::ttl
inline constexpr FormDesc irules_misc_IP__ttl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__ttl_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__ttl_cmd{
    .name = "IP::ttl",
    .forms = irules_misc_IP__ttl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__ttl_effects,
    .hover_index = 1268,
};

// IP::version
inline constexpr FormDesc irules_misc_IP__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IP__version_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IP__version_cmd{
    .name = "IP::version",
    .forms = irules_misc_IP__version_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IP__version_effects,
    .hover_index = 1269,
};

// IPFIX::destination
inline constexpr OptionDesc irules_misc_IPFIX__destination_opts[] = {
    {.name = "-publisher", .takes_value = true, .detail = "Option -publisher."},
};

inline constexpr FormDesc irules_misc_IPFIX__destination_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IPFIX__destination_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IPFIX__destination_cmd{
    .name = "IPFIX::destination",
    .options = irules_misc_IPFIX__destination_opts,
    .forms = irules_misc_IPFIX__destination_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IPFIX__destination_effects,
    .hover_index = 1270,
};

// IPFIX::msg
inline constexpr OptionDesc irules_misc_IPFIX__msg_opts[] = {
    {.name = "-pos", .takes_value = true, .value_hint = "IPFIX_POS", .detail = "Position index for duplicate field types."},
};

inline constexpr FormDesc irules_misc_IPFIX__msg_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IPFIX__msg_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IPFIX__msg_cmd{
    .name = "IPFIX::msg",
    .options = irules_misc_IPFIX__msg_opts,
    .forms = irules_misc_IPFIX__msg_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IPFIX__msg_effects,
    .hover_index = 1271,
};

// IPFIX::template
inline constexpr FormDesc irules_misc_IPFIX__template_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IPFIX__template_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IPFIX__template_cmd{
    .name = "IPFIX::template",
    .forms = irules_misc_IPFIX__template_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IPFIX__template_effects,
    .hover_index = 1272,
};

// ISESSION::deduplication
inline constexpr FormDesc irules_misc_ISESSION__deduplication_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ISESSION__deduplication_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ISESSION__deduplication_cmd{
    .name = "ISESSION::deduplication",
    .forms = irules_misc_ISESSION__deduplication_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ISESSION__deduplication_effects,
    .hover_index = 1273,
};

// ISTATS::get
inline constexpr FormDesc irules_misc_ISTATS__get_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ISTATS__get_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_ISTATS__get_cmd{
    .name = "ISTATS::get",
    .forms = irules_misc_ISTATS__get_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ISTATS__get_effects,
    .hover_index = 1274,
};

// ISTATS::incr
inline constexpr FormDesc irules_misc_ISTATS__incr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ISTATS__incr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_ISTATS__incr_cmd{
    .name = "ISTATS::incr",
    .forms = irules_misc_ISTATS__incr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ISTATS__incr_effects,
    .hover_index = 1275,
};

// ISTATS::remove
inline constexpr FormDesc irules_misc_ISTATS__remove_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ISTATS__remove_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_ISTATS__remove_cmd{
    .name = "ISTATS::remove",
    .forms = irules_misc_ISTATS__remove_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ISTATS__remove_effects,
    .hover_index = 1276,
};

// ISTATS::set
inline constexpr FormDesc irules_misc_ISTATS__set_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ISTATS__set_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_ISTATS__set_cmd{
    .name = "ISTATS::set",
    .forms = irules_misc_ISTATS__set_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ISTATS__set_effects,
    .hover_index = 1277,
};

// IVS_ENTRY::result
inline constexpr FormDesc irules_misc_IVS_ENTRY__result_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_IVS_ENTRY__result_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_IVS_ENTRY__result_cmd{
    .name = "IVS_ENTRY::result",
    .forms = irules_misc_IVS_ENTRY__result_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_IVS_ENTRY__result_effects,
    .hover_index = 1278,
};

// JSON::array
inline constexpr FormDesc irules_misc_JSON__array_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__array_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_JSON__array_cmd{
    .name = "JSON::array",
    .forms = irules_misc_JSON__array_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__array_effects,
    .hover_index = 1279,
};

// JSON::create
inline constexpr FormDesc irules_misc_JSON__create_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__create_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr CommandDesc irules_misc_JSON__create_cmd{
    .name = "JSON::create",
    .forms = irules_misc_JSON__create_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__create_effects,
    .hover_index = 1280,
};

// JSON::get
inline constexpr FormDesc irules_misc_JSON__get_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__get_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_JSON__get_cmd{
    .name = "JSON::get",
    .forms = irules_misc_JSON__get_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__get_effects,
    .hover_index = 1281,
};

// JSON::object
inline constexpr FormDesc irules_misc_JSON__object_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__object_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_JSON__object_cmd{
    .name = "JSON::object",
    .forms = irules_misc_JSON__object_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__object_effects,
    .hover_index = 1282,
};

// JSON::parse
inline constexpr FormDesc irules_misc_JSON__parse_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__parse_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_JSON__parse_cmd{
    .name = "JSON::parse",
    .forms = irules_misc_JSON__parse_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__parse_effects,
    .hover_index = 1283,
};

// JSON::render
inline constexpr FormDesc irules_misc_JSON__render_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__render_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_JSON__render_cmd{
    .name = "JSON::render",
    .forms = irules_misc_JSON__render_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__render_effects,
    .hover_index = 1284,
};

// JSON::root
inline constexpr FormDesc irules_misc_JSON__root_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__root_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_JSON__root_cmd{
    .name = "JSON::root",
    .forms = irules_misc_JSON__root_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__root_effects,
    .hover_index = 1285,
};

// JSON::set
inline constexpr FormDesc irules_misc_JSON__set_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__set_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true},
};

inline constexpr CommandDesc irules_misc_JSON__set_cmd{
    .name = "JSON::set",
    .forms = irules_misc_JSON__set_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__set_effects,
    .hover_index = 1286,
};

// JSON::type
inline constexpr FormDesc irules_misc_JSON__type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_JSON__type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true},
};

inline constexpr CommandDesc irules_misc_JSON__type_cmd{
    .name = "JSON::type",
    .forms = irules_misc_JSON__type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_JSON__type_effects,
    .hover_index = 1287,
};

// L7CHECK::protocol
inline constexpr FormDesc irules_misc_L7CHECK__protocol_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_L7CHECK__protocol_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_L7CHECK__protocol_cmd{
    .name = "L7CHECK::protocol",
    .forms = irules_misc_L7CHECK__protocol_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_L7CHECK__protocol_effects,
    .hover_index = 1288,
};

// LB::bias
inline constexpr FormDesc irules_misc_LB__bias_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__bias_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__bias_cmd{
    .name = "LB::bias",
    .forms = irules_misc_LB__bias_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__bias_effects,
    .hover_index = 1289,
};

// LB::class
inline constexpr FormDesc irules_misc_LB__class_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__class_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__class_cmd{
    .name = "LB::class",
    .forms = irules_misc_LB__class_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__class_effects,
    .hover_index = 1290,
};

// LB::command
inline constexpr FormDesc irules_misc_LB__command_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__command_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__command_cmd{
    .name = "LB::command",
    .forms = irules_misc_LB__command_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__command_effects,
    .hover_index = 1291,
};

// LB::connect
inline constexpr FormDesc irules_misc_LB__connect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__connect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__connect_cmd{
    .name = "LB::connect",
    .forms = irules_misc_LB__connect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__connect_effects,
    .hover_index = 1292,
};

// LB::connlimit
inline constexpr FormDesc irules_misc_LB__connlimit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__connlimit_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr SubCmdDesc irules_misc_LB__connlimit_subcmds[] = {
    {.name = "virtual", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true, .mutator = true, .hover_index = 1293},
    {.name = "node", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true, .mutator = true, .hover_index = 1294},
    {.name = "poolmember", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true, .mutator = true, .hover_index = 1295},
};

inline constexpr CommandDesc irules_misc_LB__connlimit_cmd{
    .name = "LB::connlimit",
    .subcommands = irules_misc_LB__connlimit_subcmds,
    .forms = irules_misc_LB__connlimit_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__connlimit_effects,
    .hover_index = 1296,
};

// LB::context_id
inline constexpr FormDesc irules_misc_LB__context_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__context_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__context_id_cmd{
    .name = "LB::context_id",
    .forms = irules_misc_LB__context_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__context_id_effects,
    .hover_index = 1297,
};

// LB::detach
inline constexpr FormDesc irules_misc_LB__detach_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__detach_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__detach_cmd{
    .name = "LB::detach",
    .forms = irules_misc_LB__detach_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__detach_effects,
    .hover_index = 1298,
};

// LB::down
inline constexpr FormDesc irules_misc_LB__down_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__down_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr SubCmdDesc irules_misc_LB__down_subcmds[] = {
    {.name = "node", .arity = {1, 1}, .mutator = true, .hover_index = 1299},
    {.name = "pool", .arity = {3, 3}, .mutator = true, .hover_index = 1300},
};

inline constexpr CommandDesc irules_misc_LB__down_cmd{
    .name = "LB::down",
    .subcommands = irules_misc_LB__down_subcmds,
    .forms = irules_misc_LB__down_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__down_effects,
    .hover_index = 1301,
};

// LB::dst_tag
inline constexpr FormDesc irules_misc_LB__dst_tag_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__dst_tag_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__dst_tag_cmd{
    .name = "LB::dst_tag",
    .forms = irules_misc_LB__dst_tag_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__dst_tag_effects,
    .hover_index = 1302,
};

// LB::enable_decisionlog
inline constexpr FormDesc irules_misc_LB__enable_decisionlog_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__enable_decisionlog_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LB__enable_decisionlog_cmd{
    .name = "LB::enable_decisionlog",
    .forms = irules_misc_LB__enable_decisionlog_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_LB__enable_decisionlog_effects,
    .hover_index = 1303,
};

// LB::mode
inline constexpr FormDesc irules_misc_LB__mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__mode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__mode_cmd{
    .name = "LB::mode",
    .forms = irules_misc_LB__mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__mode_effects,
    .hover_index = 1304,
};

// LB::persist
inline constexpr FormDesc irules_misc_LB__persist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__persist_effects[] = {
    {.target = SideEffectTarget::PERSISTENCE_TABLE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_LB__persist_subcmds[] = {
    {.name = "key", .arity = {0, 0}, .pure = true, .hover_index = 1305},
    {.name = "cookie", .arity = {0, 0}, .pure = true, .hover_index = 1306},
};

inline constexpr CommandDesc irules_misc_LB__persist_cmd{
    .name = "LB::persist",
    .subcommands = irules_misc_LB__persist_subcmds,
    .forms = irules_misc_LB__persist_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__persist_effects,
    .hover_index = 1307,
};

// LB::prime
inline constexpr FormDesc irules_misc_LB__prime_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__prime_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__prime_cmd{
    .name = "LB::prime",
    .forms = irules_misc_LB__prime_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__prime_effects,
    .hover_index = 1308,
};

// LB::queue
inline constexpr FormDesc irules_misc_LB__queue_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__queue_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__queue_cmd{
    .name = "LB::queue",
    .forms = irules_misc_LB__queue_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__queue_effects,
    .hover_index = 1309,
};

// LB::reselect
inline constexpr FormDesc irules_misc_LB__reselect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__reselect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__reselect_cmd{
    .name = "LB::reselect",
    .forms = irules_misc_LB__reselect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__reselect_effects,
    .hover_index = 1310,
};

// LB::select
inline constexpr FormDesc irules_misc_LB__select_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__select_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__select_cmd{
    .name = "LB::select",
    .forms = irules_misc_LB__select_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__select_effects,
    .hover_index = 1311,
};

// LB::server
inline constexpr FormDesc irules_misc_LB__server_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__server_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr SubCmdDesc irules_misc_LB__server_subcmds[] = {
    {.name = "name", .arity = {0, 0}, .pure = true, .hover_index = 1312},
    {.name = "pool", .arity = {0, 0}, .pure = true, .hover_index = 1313},
    {.name = "route_domain", .arity = {0, 0}, .pure = true, .hover_index = 1314},
    {.name = "addr", .arity = {0, 0}, .pure = true, .hover_index = 1315},
    {.name = "port", .arity = {0, 0}, .pure = true, .hover_index = 1316},
    {.name = "priority", .arity = {0, 0}, .pure = true, .hover_index = 1317},
    {.name = "ratio", .arity = {0, 0}, .pure = true, .hover_index = 1318},
    {.name = "weight", .arity = {0, 0}, .pure = true, .hover_index = 1319},
    {.name = "ripeness", .arity = {0, 0}, .pure = true, .hover_index = 1320},
};

inline constexpr CommandDesc irules_misc_LB__server_cmd{
    .name = "LB::server",
    .subcommands = irules_misc_LB__server_subcmds,
    .forms = irules_misc_LB__server_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__server_effects,
    .hover_index = 1321,
};

// LB::snat
inline constexpr FormDesc irules_misc_LB__snat_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__snat_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__snat_cmd{
    .name = "LB::snat",
    .forms = irules_misc_LB__snat_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__snat_effects,
    .hover_index = 1322,
};

// LB::src_tag
inline constexpr FormDesc irules_misc_LB__src_tag_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__src_tag_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_LB__src_tag_cmd{
    .name = "LB::src_tag",
    .forms = irules_misc_LB__src_tag_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__src_tag_effects,
    .hover_index = 1323,
};

// LB::status
inline constexpr FormDesc irules_misc_LB__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr SubCmdDesc irules_misc_LB__status_subcmds[] = {
    {.name = "node", .arity = {1, std::numeric_limits<std::int32_t>::max()}, .pure = true, .hover_index = 1324},
    {.name = "pool", .arity = {3, std::numeric_limits<std::int32_t>::max()}, .pure = true, .hover_index = 1325},
};

inline constexpr CommandDesc irules_misc_LB__status_cmd{
    .name = "LB::status",
    .subcommands = irules_misc_LB__status_subcmds,
    .forms = irules_misc_LB__status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__status_effects,
    .hover_index = 1326,
};

// LB::up
inline constexpr FormDesc irules_misc_LB__up_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LB__up_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr SubCmdDesc irules_misc_LB__up_subcmds[] = {
    {.name = "node", .arity = {1, 1}, .mutator = true, .hover_index = 1327},
    {.name = "pool", .arity = {3, 3}, .mutator = true, .hover_index = 1328},
};

inline constexpr CommandDesc irules_misc_LB__up_cmd{
    .name = "LB::up",
    .subcommands = irules_misc_LB__up_subcmds,
    .forms = irules_misc_LB__up_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LB__up_effects,
    .hover_index = 1329,
};

// LDAP::activation_mode
inline constexpr FormDesc irules_misc_LDAP__activation_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LDAP__activation_mode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LDAP__activation_mode_cmd{
    .name = "LDAP::activation_mode",
    .forms = irules_misc_LDAP__activation_mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LDAP__activation_mode_effects,
    .hover_index = 1330,
};

// LDAP::disable
inline constexpr FormDesc irules_misc_LDAP__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LDAP__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LDAP__disable_cmd{
    .name = "LDAP::disable",
    .forms = irules_misc_LDAP__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LDAP__disable_effects,
    .hover_index = 1331,
};

// LDAP::enable
inline constexpr FormDesc irules_misc_LDAP__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LDAP__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LDAP__enable_cmd{
    .name = "LDAP::enable",
    .forms = irules_misc_LDAP__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LDAP__enable_effects,
    .hover_index = 1332,
};

// LINE::get
inline constexpr FormDesc irules_misc_LINE__get_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LINE__get_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LINE__get_cmd{
    .name = "LINE::get",
    .forms = irules_misc_LINE__get_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LINE__get_effects,
    .hover_index = 1333,
};

// LINE::set
inline constexpr FormDesc irules_misc_LINE__set_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LINE__set_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LINE__set_cmd{
    .name = "LINE::set",
    .forms = irules_misc_LINE__set_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LINE__set_effects,
    .hover_index = 1334,
};

// LINK::lasthop
inline constexpr FormDesc irules_misc_LINK__lasthop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LINK__lasthop_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LINK__lasthop_cmd{
    .name = "LINK::lasthop",
    .forms = irules_misc_LINK__lasthop_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LINK__lasthop_effects,
    .hover_index = 1335,
};

// LINK::nexthop
inline constexpr FormDesc irules_misc_LINK__nexthop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LINK__nexthop_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LINK__nexthop_cmd{
    .name = "LINK::nexthop",
    .forms = irules_misc_LINK__nexthop_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LINK__nexthop_effects,
    .hover_index = 1336,
};

// LINK::qos
inline constexpr FormDesc irules_misc_LINK__qos_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LINK__qos_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LINK__qos_cmd{
    .name = "LINK::qos",
    .forms = irules_misc_LINK__qos_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LINK__qos_effects,
    .hover_index = 1337,
};

// LINK::vlan_id
inline constexpr FormDesc irules_misc_LINK__vlan_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LINK__vlan_id_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LINK__vlan_id_cmd{
    .name = "LINK::vlan_id",
    .forms = irules_misc_LINK__vlan_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LINK__vlan_id_effects,
    .hover_index = 1338,
};

// LSN::address
inline constexpr FormDesc irules_misc_LSN__address_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__address_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__address_cmd{
    .name = "LSN::address",
    .forms = irules_misc_LSN__address_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LSN__address_effects,
    .hover_index = 1339,
};

// LSN::disable
inline constexpr FormDesc irules_misc_LSN__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__disable_cmd{
    .name = "LSN::disable",
    .forms = irules_misc_LSN__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LSN__disable_effects,
    .hover_index = 1340,
};

// LSN::inbound
inline constexpr FormDesc irules_misc_LSN__inbound_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__inbound_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__inbound_cmd{
    .name = "LSN::inbound",
    .forms = irules_misc_LSN__inbound_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LSN__inbound_effects,
    .hover_index = 1341,
};

// LSN::inbound-entry
inline constexpr OptionDesc irules_misc_LSN__inbound_entry_opts[] = {
    {.name = "-mirror", .detail = "Option -mirror."},
};

inline constexpr FormDesc irules_misc_LSN__inbound_entry_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__inbound_entry_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__inbound_entry_cmd{
    .name = "LSN::inbound-entry",
    .options = irules_misc_LSN__inbound_entry_opts,
    .forms = irules_misc_LSN__inbound_entry_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LSN__inbound_entry_effects,
    .hover_index = 1342,
};

// LSN::persistence
inline constexpr FormDesc irules_misc_LSN__persistence_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__persistence_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__persistence_cmd{
    .name = "LSN::persistence",
    .forms = irules_misc_LSN__persistence_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LSN__persistence_effects,
    .hover_index = 1343,
};

// LSN::persistence-entry
inline constexpr OptionDesc irules_misc_LSN__persistence_entry_opts[] = {
    {.name = "-override", .detail = "Option -override."},
};

inline constexpr FormDesc irules_misc_LSN__persistence_entry_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__persistence_entry_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__persistence_entry_cmd{
    .name = "LSN::persistence-entry",
    .options = irules_misc_LSN__persistence_entry_opts,
    .forms = irules_misc_LSN__persistence_entry_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LSN__persistence_entry_effects,
    .hover_index = 1344,
};

// LSN::pool
inline constexpr FormDesc irules_misc_LSN__pool_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__pool_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__pool_cmd{
    .name = "LSN::pool",
    .forms = irules_misc_LSN__pool_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP | ProfileFlags::SIP},
    .side_effect_hints = irules_misc_LSN__pool_effects,
    .hover_index = 1345,
};

// LSN::port
inline constexpr FormDesc irules_misc_LSN__port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_LSN__port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_LSN__port_cmd{
    .name = "LSN::port",
    .forms = irules_misc_LSN__port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_LSN__port_effects,
    .hover_index = 1346,
};

// MESSAGE::field
inline constexpr FormDesc irules_misc_MESSAGE__field_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MESSAGE__field_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MESSAGE__field_cmd{
    .name = "MESSAGE::field",
    .forms = irules_misc_MESSAGE__field_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MESSAGE__field_effects,
    .hover_index = 1347,
};

// MESSAGE::proto
inline constexpr FormDesc irules_misc_MESSAGE__proto_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MESSAGE__proto_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MESSAGE__proto_cmd{
    .name = "MESSAGE::proto",
    .forms = irules_misc_MESSAGE__proto_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MESSAGE__proto_effects,
    .hover_index = 1348,
};

// MESSAGE::type
inline constexpr FormDesc irules_misc_MESSAGE__type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MESSAGE__type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MESSAGE__type_cmd{
    .name = "MESSAGE::type",
    .forms = irules_misc_MESSAGE__type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MESSAGE__type_effects,
    .hover_index = 1349,
};

// MQTT::clean_session
inline constexpr FormDesc irules_misc_MQTT__clean_session_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__clean_session_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__clean_session_cmd{
    .name = "MQTT::clean_session",
    .forms = irules_misc_MQTT__clean_session_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__clean_session_effects,
    .hover_index = 1350,
};

// MQTT::client_id
inline constexpr FormDesc irules_misc_MQTT__client_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__client_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__client_id_cmd{
    .name = "MQTT::client_id",
    .forms = irules_misc_MQTT__client_id_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__client_id_effects,
    .hover_index = 1351,
};

// MQTT::collect
inline constexpr FormDesc irules_misc_MQTT__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__collect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__collect_cmd{
    .name = "MQTT::collect",
    .forms = irules_misc_MQTT__collect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__collect_effects,
    .hover_index = 1352,
};

// MQTT::disable
inline constexpr FormDesc irules_misc_MQTT__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__disable_cmd{
    .name = "MQTT::disable",
    .forms = irules_misc_MQTT__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__disable_effects,
    .hover_index = 1353,
};

// MQTT::disconnect
inline constexpr FormDesc irules_misc_MQTT__disconnect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__disconnect_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__disconnect_cmd{
    .name = "MQTT::disconnect",
    .forms = irules_misc_MQTT__disconnect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__disconnect_effects,
    .hover_index = 1354,
};

// MQTT::drop
inline constexpr FormDesc irules_misc_MQTT__drop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__drop_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__drop_cmd{
    .name = "MQTT::drop",
    .forms = irules_misc_MQTT__drop_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__drop_effects,
    .hover_index = 1355,
};

// MQTT::dup
inline constexpr FormDesc irules_misc_MQTT__dup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__dup_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__dup_cmd{
    .name = "MQTT::dup",
    .forms = irules_misc_MQTT__dup_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__dup_effects,
    .hover_index = 1356,
};

// MQTT::enable
inline constexpr FormDesc irules_misc_MQTT__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__enable_cmd{
    .name = "MQTT::enable",
    .forms = irules_misc_MQTT__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MQTT__enable_effects,
    .hover_index = 1357,
};

// MQTT::insert
inline constexpr FormDesc irules_misc_MQTT__insert_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__insert_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__insert_cmd{
    .name = "MQTT::insert",
    .forms = irules_misc_MQTT__insert_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__insert_effects,
    .hover_index = 1358,
};

// MQTT::keep_alive
inline constexpr FormDesc irules_misc_MQTT__keep_alive_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__keep_alive_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__keep_alive_cmd{
    .name = "MQTT::keep_alive",
    .forms = irules_misc_MQTT__keep_alive_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__keep_alive_effects,
    .hover_index = 1359,
};

// MQTT::length
inline constexpr FormDesc irules_misc_MQTT__length_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__length_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__length_cmd{
    .name = "MQTT::length",
    .forms = irules_misc_MQTT__length_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__length_effects,
    .hover_index = 1360,
};

// MQTT::message
inline constexpr FormDesc irules_misc_MQTT__message_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__message_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__message_cmd{
    .name = "MQTT::message",
    .forms = irules_misc_MQTT__message_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__message_effects,
    .hover_index = 1361,
};

// MQTT::packet_id
inline constexpr FormDesc irules_misc_MQTT__packet_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__packet_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__packet_id_cmd{
    .name = "MQTT::packet_id",
    .forms = irules_misc_MQTT__packet_id_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__packet_id_effects,
    .hover_index = 1362,
};

// MQTT::password
inline constexpr FormDesc irules_misc_MQTT__password_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__password_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__password_cmd{
    .name = "MQTT::password",
    .forms = irules_misc_MQTT__password_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__password_effects,
    .hover_index = 1363,
};

// MQTT::payload
inline constexpr FormDesc irules_misc_MQTT__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_MQTT__payload_subcmds[] = {
    {.name = "length", .arity = {0, 0}, .pure = true, .hover_index = 1364},
    {.name = "replace", .arity = {1, 3}, .mutator = true, .hover_index = 1365},
    {.name = "prepend", .arity = {1, 1}, .mutator = true, .hover_index = 1366},
    {.name = "append", .arity = {1, 1}, .mutator = true, .hover_index = 1367},
};

inline constexpr CommandDesc irules_misc_MQTT__payload_cmd{
    .name = "MQTT::payload",
    .subcommands = irules_misc_MQTT__payload_subcmds,
    .forms = irules_misc_MQTT__payload_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__payload_effects,
    .hover_index = 1368,
};

// MQTT::protocol_name
inline constexpr FormDesc irules_misc_MQTT__protocol_name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__protocol_name_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__protocol_name_cmd{
    .name = "MQTT::protocol_name",
    .forms = irules_misc_MQTT__protocol_name_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__protocol_name_effects,
    .hover_index = 1369,
};

// MQTT::protocol_version
inline constexpr FormDesc irules_misc_MQTT__protocol_version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__protocol_version_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__protocol_version_cmd{
    .name = "MQTT::protocol_version",
    .forms = irules_misc_MQTT__protocol_version_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__protocol_version_effects,
    .hover_index = 1370,
};

// MQTT::qos
inline constexpr FormDesc irules_misc_MQTT__qos_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__qos_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__qos_cmd{
    .name = "MQTT::qos",
    .forms = irules_misc_MQTT__qos_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__qos_effects,
    .hover_index = 1371,
};

// MQTT::release
inline constexpr FormDesc irules_misc_MQTT__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__release_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__release_cmd{
    .name = "MQTT::release",
    .forms = irules_misc_MQTT__release_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MQTT__release_effects,
    .hover_index = 1372,
};

// MQTT::replace
inline constexpr FormDesc irules_misc_MQTT__replace_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__replace_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__replace_cmd{
    .name = "MQTT::replace",
    .forms = irules_misc_MQTT__replace_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__replace_effects,
    .hover_index = 1373,
};

// MQTT::respond
inline constexpr FormDesc irules_misc_MQTT__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__respond_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__respond_cmd{
    .name = "MQTT::respond",
    .forms = irules_misc_MQTT__respond_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__respond_effects,
    .hover_index = 1374,
};

// MQTT::retain
inline constexpr FormDesc irules_misc_MQTT__retain_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__retain_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__retain_cmd{
    .name = "MQTT::retain",
    .forms = irules_misc_MQTT__retain_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__retain_effects,
    .hover_index = 1375,
};

// MQTT::return_code
inline constexpr FormDesc irules_misc_MQTT__return_code_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__return_code_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__return_code_cmd{
    .name = "MQTT::return_code",
    .forms = irules_misc_MQTT__return_code_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__return_code_effects,
    .hover_index = 1376,
};

// MQTT::return_code_list
inline constexpr FormDesc irules_misc_MQTT__return_code_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__return_code_list_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__return_code_list_cmd{
    .name = "MQTT::return_code_list",
    .forms = irules_misc_MQTT__return_code_list_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__return_code_list_effects,
    .hover_index = 1377,
};

// MQTT::session_present
inline constexpr FormDesc irules_misc_MQTT__session_present_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__session_present_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__session_present_cmd{
    .name = "MQTT::session_present",
    .forms = irules_misc_MQTT__session_present_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__session_present_effects,
    .hover_index = 1378,
};

// MQTT::topic
inline constexpr FormDesc irules_misc_MQTT__topic_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__topic_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_MQTT__topic_subcmds[] = {
    {.name = "replace", .arity = {1, 2}, .mutator = true, .hover_index = 1379},
    {.name = "count", .arity = {0, 0}, .pure = true, .hover_index = 1380},
    {.name = "list", .arity = {0, 0}, .pure = true, .hover_index = 1381},
    {.name = "index", .arity = {1, 1}, .pure = true, .hover_index = 1382},
    {.name = "qos", .arity = {0, 1}, .pure = true, .hover_index = 1383},
    {.name = "add", .arity = {1, 2}, .mutator = true, .hover_index = 1384},
    {.name = "delete", .arity = {1, 1}, .mutator = true, .hover_index = 1385},
};

inline constexpr CommandDesc irules_misc_MQTT__topic_cmd{
    .name = "MQTT::topic",
    .subcommands = irules_misc_MQTT__topic_subcmds,
    .forms = irules_misc_MQTT__topic_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__topic_effects,
    .hover_index = 1386,
};

// MQTT::type
inline constexpr FormDesc irules_misc_MQTT__type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__type_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__type_cmd{
    .name = "MQTT::type",
    .forms = irules_misc_MQTT__type_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__type_effects,
    .hover_index = 1387,
};

// MQTT::username
inline constexpr FormDesc irules_misc_MQTT__username_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__username_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__username_cmd{
    .name = "MQTT::username",
    .forms = irules_misc_MQTT__username_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__username_effects,
    .hover_index = 1388,
};

// MQTT::will
inline constexpr FormDesc irules_misc_MQTT__will_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MQTT__will_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MQTT__will_cmd{
    .name = "MQTT::will",
    .forms = irules_misc_MQTT__will_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::MQTT},
    .side_effect_hints = irules_misc_MQTT__will_effects,
    .hover_index = 1389,
};

// MR::always_match_port
inline constexpr FormDesc irules_misc_MR__always_match_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__always_match_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__always_match_port_cmd{
    .name = "MR::always_match_port",
    .forms = irules_misc_MR__always_match_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__always_match_port_effects,
    .hover_index = 1390,
};

// MR::available_for_routing
inline constexpr FormDesc irules_misc_MR__available_for_routing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__available_for_routing_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_MR__available_for_routing_cmd{
    .name = "MR::available_for_routing",
    .forms = irules_misc_MR__available_for_routing_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__available_for_routing_effects,
    .hover_index = 1391,
};

// MR::collect
inline constexpr FormDesc irules_misc_MR__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__collect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__collect_cmd{
    .name = "MR::collect",
    .forms = irules_misc_MR__collect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__collect_effects,
    .hover_index = 1392,
};

// MR::connect_back_port
inline constexpr FormDesc irules_misc_MR__connect_back_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__connect_back_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__connect_back_port_cmd{
    .name = "MR::connect_back_port",
    .forms = irules_misc_MR__connect_back_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__connect_back_port_effects,
    .hover_index = 1393,
};

// MR::connection_instance
inline constexpr FormDesc irules_misc_MR__connection_instance_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__connection_instance_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__connection_instance_cmd{
    .name = "MR::connection_instance",
    .forms = irules_misc_MR__connection_instance_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__connection_instance_effects,
    .hover_index = 1394,
};

// MR::connection_mode
inline constexpr FormDesc irules_misc_MR__connection_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__connection_mode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__connection_mode_cmd{
    .name = "MR::connection_mode",
    .forms = irules_misc_MR__connection_mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__connection_mode_effects,
    .hover_index = 1395,
};

// MR::equivalent_transport
inline constexpr FormDesc irules_misc_MR__equivalent_transport_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__equivalent_transport_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__equivalent_transport_cmd{
    .name = "MR::equivalent_transport",
    .forms = irules_misc_MR__equivalent_transport_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__equivalent_transport_effects,
    .hover_index = 1396,
};

// MR::flow_id
inline constexpr FormDesc irules_misc_MR__flow_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__flow_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__flow_id_cmd{
    .name = "MR::flow_id",
    .forms = irules_misc_MR__flow_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__flow_id_effects,
    .hover_index = 1397,
};

// MR::ignore_peer_port
inline constexpr FormDesc irules_misc_MR__ignore_peer_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__ignore_peer_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__ignore_peer_port_cmd{
    .name = "MR::ignore_peer_port",
    .forms = irules_misc_MR__ignore_peer_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__ignore_peer_port_effects,
    .hover_index = 1398,
};

// MR::instance
inline constexpr FormDesc irules_misc_MR__instance_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__instance_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__instance_cmd{
    .name = "MR::instance",
    .forms = irules_misc_MR__instance_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__instance_effects,
    .hover_index = 1399,
};

// MR::max_retries
inline constexpr FormDesc irules_misc_MR__max_retries_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__max_retries_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__max_retries_cmd{
    .name = "MR::max_retries",
    .forms = irules_misc_MR__max_retries_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__max_retries_effects,
    .hover_index = 1400,
};

// MR::message
inline constexpr OptionDesc irules_misc_MR__message_opts[] = {
    {.name = "-count", .takes_value = true, .detail = "Option -count."},
};

inline constexpr FormDesc irules_misc_MR__message_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__message_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__message_cmd{
    .name = "MR::message",
    .options = irules_misc_MR__message_opts,
    .forms = irules_misc_MR__message_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__message_effects,
    .hover_index = 1401,
};

// MR::payload
inline constexpr FormDesc irules_misc_MR__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__payload_cmd{
    .name = "MR::payload",
    .forms = irules_misc_MR__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__payload_effects,
    .hover_index = 1402,
};

// MR::peer
inline constexpr FormDesc irules_misc_MR__peer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__peer_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__peer_cmd{
    .name = "MR::peer",
    .forms = irules_misc_MR__peer_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__peer_effects,
    .hover_index = 1403,
};

// MR::prime
inline constexpr FormDesc irules_misc_MR__prime_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__prime_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__prime_cmd{
    .name = "MR::prime",
    .forms = irules_misc_MR__prime_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__prime_effects,
    .hover_index = 1404,
};

// MR::protocol
inline constexpr FormDesc irules_misc_MR__protocol_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__protocol_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__protocol_cmd{
    .name = "MR::protocol",
    .forms = irules_misc_MR__protocol_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__protocol_effects,
    .hover_index = 1405,
};

// MR::release
inline constexpr FormDesc irules_misc_MR__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__release_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__release_cmd{
    .name = "MR::release",
    .forms = irules_misc_MR__release_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__release_effects,
    .hover_index = 1406,
};

// MR::restore
inline constexpr FormDesc irules_misc_MR__restore_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__restore_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__restore_cmd{
    .name = "MR::restore",
    .forms = irules_misc_MR__restore_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__restore_effects,
    .hover_index = 1407,
};

// MR::retry
inline constexpr FormDesc irules_misc_MR__retry_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__retry_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__retry_cmd{
    .name = "MR::retry",
    .forms = irules_misc_MR__retry_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__retry_effects,
    .hover_index = 1408,
};

// MR::return
inline constexpr FormDesc irules_misc_MR__return_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__return_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__return_cmd{
    .name = "MR::return",
    .forms = irules_misc_MR__return_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__return_effects,
    .hover_index = 1409,
};

// MR::store
inline constexpr FormDesc irules_misc_MR__store_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__store_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__store_cmd{
    .name = "MR::store",
    .forms = irules_misc_MR__store_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__store_effects,
    .hover_index = 1410,
};

// MR::stream
inline constexpr FormDesc irules_misc_MR__stream_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__stream_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__stream_cmd{
    .name = "MR::stream",
    .forms = irules_misc_MR__stream_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__stream_effects,
    .hover_index = 1411,
};

// MR::transport
inline constexpr FormDesc irules_misc_MR__transport_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_MR__transport_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_MR__transport_cmd{
    .name = "MR::transport",
    .forms = irules_misc_MR__transport_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_MR__transport_effects,
    .hover_index = 1412,
};

// NAME::lookup
inline constexpr FormDesc irules_misc_NAME__lookup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NAME__lookup_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NAME__lookup_cmd{
    .name = "NAME::lookup",
    .forms = irules_misc_NAME__lookup_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NAME__lookup_effects,
    .hover_index = 1413,
    .deprecated_replacement = "RESOLV::lookup",
};

// NAME::response
inline constexpr FormDesc irules_misc_NAME__response_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NAME__response_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NAME__response_cmd{
    .name = "NAME::response",
    .forms = irules_misc_NAME__response_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NAME__response_effects,
    .hover_index = 1414,
    .deprecated_replacement = "RESOLV::lookup",
};

// NSH::chain
inline constexpr FormDesc irules_misc_NSH__chain_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NSH__chain_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NSH__chain_cmd{
    .name = "NSH::chain",
    .forms = irules_misc_NSH__chain_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NSH__chain_effects,
    .hover_index = 1415,
};

// NSH::context
inline constexpr FormDesc irules_misc_NSH__context_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NSH__context_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NSH__context_cmd{
    .name = "NSH::context",
    .forms = irules_misc_NSH__context_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NSH__context_effects,
    .hover_index = 1416,
};

// NSH::md1
inline constexpr FormDesc irules_misc_NSH__md1_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NSH__md1_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NSH__md1_cmd{
    .name = "NSH::md1",
    .forms = irules_misc_NSH__md1_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NSH__md1_effects,
    .hover_index = 1417,
};

// NSH::mocksf
inline constexpr FormDesc irules_misc_NSH__mocksf_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NSH__mocksf_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NSH__mocksf_cmd{
    .name = "NSH::mocksf",
    .forms = irules_misc_NSH__mocksf_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NSH__mocksf_effects,
    .hover_index = 1418,
};

// NSH::path_id
inline constexpr FormDesc irules_misc_NSH__path_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NSH__path_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NSH__path_id_cmd{
    .name = "NSH::path_id",
    .forms = irules_misc_NSH__path_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NSH__path_id_effects,
    .hover_index = 1419,
};

// NSH::service_index
inline constexpr FormDesc irules_misc_NSH__service_index_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NSH__service_index_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NSH__service_index_cmd{
    .name = "NSH::service_index",
    .forms = irules_misc_NSH__service_index_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NSH__service_index_effects,
    .hover_index = 1420,
};

// NTLM::disable
inline constexpr FormDesc irules_misc_NTLM__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NTLM__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NTLM__disable_cmd{
    .name = "NTLM::disable",
    .forms = irules_misc_NTLM__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NTLM__disable_effects,
    .hover_index = 1421,
};

// NTLM::enable
inline constexpr FormDesc irules_misc_NTLM__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_NTLM__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_NTLM__enable_cmd{
    .name = "NTLM::enable",
    .forms = irules_misc_NTLM__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_NTLM__enable_effects,
    .hover_index = 1422,
};

// OFFBOX::request
inline constexpr FormDesc irules_misc_OFFBOX__request_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_OFFBOX__request_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_OFFBOX__request_cmd{
    .name = "OFFBOX::request",
    .forms = irules_misc_OFFBOX__request_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_OFFBOX__request_effects,
    .hover_index = 1423,
};

// ONECONNECT::detach
inline constexpr FormDesc irules_misc_ONECONNECT__detach_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ONECONNECT__detach_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_ONECONNECT__detach_cmd{
    .name = "ONECONNECT::detach",
    .forms = irules_misc_ONECONNECT__detach_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ONECONNECT__detach_effects,
    .hover_index = 1424,
};

// ONECONNECT::label
inline constexpr FormDesc irules_misc_ONECONNECT__label_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ONECONNECT__label_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ONECONNECT__label_cmd{
    .name = "ONECONNECT::label",
    .forms = irules_misc_ONECONNECT__label_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ONECONNECT__label_effects,
    .hover_index = 1425,
};

// ONECONNECT::reuse
inline constexpr FormDesc irules_misc_ONECONNECT__reuse_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ONECONNECT__reuse_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_ONECONNECT__reuse_cmd{
    .name = "ONECONNECT::reuse",
    .forms = irules_misc_ONECONNECT__reuse_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ONECONNECT__reuse_effects,
    .hover_index = 1426,
};

// ONECONNECT::select
inline constexpr FormDesc irules_misc_ONECONNECT__select_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ONECONNECT__select_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_ONECONNECT__select_cmd{
    .name = "ONECONNECT::select",
    .forms = irules_misc_ONECONNECT__select_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ONECONNECT__select_effects,
    .hover_index = 1427,
};

// PCP::reject
inline constexpr FormDesc irules_misc_PCP__reject_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PCP__reject_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PCP__reject_cmd{
    .name = "PCP::reject",
    .forms = irules_misc_PCP__reject_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PCP__reject_effects,
    .hover_index = 1428,
};

// PCP::request
inline constexpr FormDesc irules_misc_PCP__request_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PCP__request_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PCP__request_cmd{
    .name = "PCP::request",
    .forms = irules_misc_PCP__request_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PCP__request_effects,
    .hover_index = 1429,
};

// PCP::response
inline constexpr FormDesc irules_misc_PCP__response_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PCP__response_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PCP__response_cmd{
    .name = "PCP::response",
    .forms = irules_misc_PCP__response_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PCP__response_effects,
    .hover_index = 1430,
};

// PEM::disable
inline constexpr FormDesc irules_misc_PEM__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PEM__disable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PEM__disable_cmd{
    .name = "PEM::disable",
    .forms = irules_misc_PEM__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PEM__disable_effects,
    .hover_index = 1431,
};

// PEM::enable
inline constexpr FormDesc irules_misc_PEM__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PEM__enable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PEM__enable_cmd{
    .name = "PEM::enable",
    .forms = irules_misc_PEM__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PEM__enable_effects,
    .hover_index = 1432,
};

// PEM::flow
inline constexpr FormDesc irules_misc_PEM__flow_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PEM__flow_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PEM__flow_cmd{
    .name = "PEM::flow",
    .forms = irules_misc_PEM__flow_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PEM__flow_effects,
    .hover_index = 1433,
};

// PEM::session
inline constexpr FormDesc irules_misc_PEM__session_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PEM__session_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PEM__session_cmd{
    .name = "PEM::session",
    .forms = irules_misc_PEM__session_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PEM__session_effects,
    .hover_index = 1434,
};

// PEM::subscriber
inline constexpr FormDesc irules_misc_PEM__subscriber_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PEM__subscriber_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PEM__subscriber_cmd{
    .name = "PEM::subscriber",
    .forms = irules_misc_PEM__subscriber_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PEM__subscriber_effects,
    .hover_index = 1435,
};

// PLUGIN::disable
inline constexpr SideEffectDesc irules_misc_PLUGIN__disable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PLUGIN__disable_cmd{
    .name = "PLUGIN::disable",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PLUGIN__disable_effects,
    .hover_index = 1436,
    .deprecated_replacement = "(removed)",
};

// PLUGIN::enable
inline constexpr SideEffectDesc irules_misc_PLUGIN__enable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PLUGIN__enable_cmd{
    .name = "PLUGIN::enable",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PLUGIN__enable_effects,
    .hover_index = 1437,
    .deprecated_replacement = "(removed)",
};

// POLICY::controls
inline constexpr FormDesc irules_misc_POLICY__controls_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_POLICY__controls_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_POLICY__controls_cmd{
    .name = "POLICY::controls",
    .forms = irules_misc_POLICY__controls_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_POLICY__controls_effects,
    .hover_index = 1438,
};

// POLICY::names
inline constexpr FormDesc irules_misc_POLICY__names_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_POLICY__names_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_POLICY__names_cmd{
    .name = "POLICY::names",
    .forms = irules_misc_POLICY__names_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_POLICY__names_effects,
    .hover_index = 1439,
};

// POLICY::rules
inline constexpr FormDesc irules_misc_POLICY__rules_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_POLICY__rules_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_POLICY__rules_cmd{
    .name = "POLICY::rules",
    .forms = irules_misc_POLICY__rules_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_POLICY__rules_effects,
    .hover_index = 1440,
};

// POLICY::targets
inline constexpr FormDesc irules_misc_POLICY__targets_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_POLICY__targets_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_POLICY__targets_cmd{
    .name = "POLICY::targets",
    .forms = irules_misc_POLICY__targets_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_POLICY__targets_effects,
    .hover_index = 1441,
};

// POP3::activation_mode
inline constexpr FormDesc irules_misc_POP3__activation_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_POP3__activation_mode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_POP3__activation_mode_cmd{
    .name = "POP3::activation_mode",
    .forms = irules_misc_POP3__activation_mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_POP3__activation_mode_effects,
    .hover_index = 1442,
};

// POP3::disable
inline constexpr FormDesc irules_misc_POP3__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_POP3__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_POP3__disable_cmd{
    .name = "POP3::disable",
    .forms = irules_misc_POP3__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_POP3__disable_effects,
    .hover_index = 1443,
};

// POP3::enable
inline constexpr FormDesc irules_misc_POP3__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_POP3__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_POP3__enable_cmd{
    .name = "POP3::enable",
    .forms = irules_misc_POP3__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_POP3__enable_effects,
    .hover_index = 1444,
};

// PROFILE::access
inline constexpr FormDesc irules_misc_PROFILE__access_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__access_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__access_cmd{
    .name = "PROFILE::access",
    .forms = irules_misc_PROFILE__access_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__access_effects,
    .hover_index = 1445,
};

// PROFILE::antifraud
inline constexpr FormDesc irules_misc_PROFILE__antifraud_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__antifraud_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__antifraud_cmd{
    .name = "PROFILE::antifraud",
    .forms = irules_misc_PROFILE__antifraud_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__antifraud_effects,
    .hover_index = 1446,
};

// PROFILE::auth
inline constexpr FormDesc irules_misc_PROFILE__auth_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__auth_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__auth_cmd{
    .name = "PROFILE::auth",
    .forms = irules_misc_PROFILE__auth_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__auth_effects,
    .hover_index = 1447,
};

// PROFILE::avr
inline constexpr FormDesc irules_misc_PROFILE__avr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__avr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__avr_cmd{
    .name = "PROFILE::avr",
    .forms = irules_misc_PROFILE__avr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__avr_effects,
    .hover_index = 1448,
};

// PROFILE::clientssl
inline constexpr FormDesc irules_misc_PROFILE__clientssl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__clientssl_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__clientssl_cmd{
    .name = "PROFILE::clientssl",
    .forms = irules_misc_PROFILE__clientssl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__clientssl_effects,
    .hover_index = 1449,
};

// PROFILE::diameter
inline constexpr FormDesc irules_misc_PROFILE__diameter_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__diameter_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__diameter_cmd{
    .name = "PROFILE::diameter",
    .forms = irules_misc_PROFILE__diameter_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__diameter_effects,
    .hover_index = 1450,
};

// PROFILE::exchange
inline constexpr FormDesc irules_misc_PROFILE__exchange_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__exchange_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__exchange_cmd{
    .name = "PROFILE::exchange",
    .forms = irules_misc_PROFILE__exchange_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__exchange_effects,
    .hover_index = 1451,
};

// PROFILE::exists
inline constexpr FormDesc irules_misc_PROFILE__exists_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__exists_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__exists_cmd{
    .name = "PROFILE::exists",
    .forms = irules_misc_PROFILE__exists_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__exists_effects,
    .hover_index = 1452,
};

// PROFILE::fastL4
inline constexpr FormDesc irules_misc_PROFILE__fastL4_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__fastL4_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__fastL4_cmd{
    .name = "PROFILE::fastL4",
    .forms = irules_misc_PROFILE__fastL4_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__fastL4_effects,
    .hover_index = 1453,
};

// PROFILE::fasthttp
inline constexpr FormDesc irules_misc_PROFILE__fasthttp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__fasthttp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__fasthttp_cmd{
    .name = "PROFILE::fasthttp",
    .forms = irules_misc_PROFILE__fasthttp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__fasthttp_effects,
    .hover_index = 1454,
};

// PROFILE::ftp
inline constexpr FormDesc irules_misc_PROFILE__ftp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__ftp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__ftp_cmd{
    .name = "PROFILE::ftp",
    .forms = irules_misc_PROFILE__ftp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__ftp_effects,
    .hover_index = 1455,
};

// PROFILE::http
inline constexpr FormDesc irules_misc_PROFILE__http_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__http_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__http_cmd{
    .name = "PROFILE::http",
    .forms = irules_misc_PROFILE__http_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__http_effects,
    .hover_index = 1456,
};

// PROFILE::httpclass
inline constexpr SideEffectDesc irules_misc_PROFILE__httpclass_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__httpclass_cmd{
    .name = "PROFILE::httpclass",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__httpclass_effects,
    .hover_index = 1457,
    .deprecated_replacement = "PROFILE::http",
};

// PROFILE::httpcompression
inline constexpr FormDesc irules_misc_PROFILE__httpcompression_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__httpcompression_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__httpcompression_cmd{
    .name = "PROFILE::httpcompression",
    .forms = irules_misc_PROFILE__httpcompression_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__httpcompression_effects,
    .hover_index = 1458,
};

// PROFILE::list
inline constexpr FormDesc irules_misc_PROFILE__list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__list_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__list_cmd{
    .name = "PROFILE::list",
    .forms = irules_misc_PROFILE__list_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__list_effects,
    .hover_index = 1459,
};

// PROFILE::oneconnect
inline constexpr FormDesc irules_misc_PROFILE__oneconnect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__oneconnect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__oneconnect_cmd{
    .name = "PROFILE::oneconnect",
    .forms = irules_misc_PROFILE__oneconnect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__oneconnect_effects,
    .hover_index = 1460,
};

// PROFILE::persist
inline constexpr FormDesc irules_misc_PROFILE__persist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__persist_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__persist_cmd{
    .name = "PROFILE::persist",
    .forms = irules_misc_PROFILE__persist_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__persist_effects,
    .hover_index = 1461,
};

// PROFILE::serverssl
inline constexpr FormDesc irules_misc_PROFILE__serverssl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__serverssl_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__serverssl_cmd{
    .name = "PROFILE::serverssl",
    .forms = irules_misc_PROFILE__serverssl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__serverssl_effects,
    .hover_index = 1462,
};

// PROFILE::stream
inline constexpr FormDesc irules_misc_PROFILE__stream_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__stream_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__stream_cmd{
    .name = "PROFILE::stream",
    .forms = irules_misc_PROFILE__stream_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__stream_effects,
    .hover_index = 1463,
};

// PROFILE::tcp
inline constexpr FormDesc irules_misc_PROFILE__tcp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__tcp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__tcp_cmd{
    .name = "PROFILE::tcp",
    .forms = irules_misc_PROFILE__tcp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__tcp_effects,
    .hover_index = 1464,
};

// PROFILE::tftp
inline constexpr FormDesc irules_misc_PROFILE__tftp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__tftp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__tftp_cmd{
    .name = "PROFILE::tftp",
    .forms = irules_misc_PROFILE__tftp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__tftp_effects,
    .hover_index = 1465,
};

// PROFILE::udp
inline constexpr FormDesc irules_misc_PROFILE__udp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__udp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__udp_cmd{
    .name = "PROFILE::udp",
    .forms = irules_misc_PROFILE__udp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__udp_effects,
    .hover_index = 1466,
};

// PROFILE::vdi
inline constexpr FormDesc irules_misc_PROFILE__vdi_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__vdi_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__vdi_cmd{
    .name = "PROFILE::vdi",
    .forms = irules_misc_PROFILE__vdi_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__vdi_effects,
    .hover_index = 1467,
};

// PROFILE::webacceleration
inline constexpr FormDesc irules_misc_PROFILE__webacceleration_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__webacceleration_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__webacceleration_cmd{
    .name = "PROFILE::webacceleration",
    .forms = irules_misc_PROFILE__webacceleration_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__webacceleration_effects,
    .hover_index = 1468,
};

// PROFILE::xml
inline constexpr FormDesc irules_misc_PROFILE__xml_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROFILE__xml_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_PROFILE__xml_cmd{
    .name = "PROFILE::xml",
    .forms = irules_misc_PROFILE__xml_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROFILE__xml_effects,
    .hover_index = 1469,
};

// PROTOCOL_INSPECTION::disable
inline constexpr FormDesc irules_misc_PROTOCOL_INSPECTION__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROTOCOL_INSPECTION__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PROTOCOL_INSPECTION__disable_cmd{
    .name = "PROTOCOL_INSPECTION::disable",
    .forms = irules_misc_PROTOCOL_INSPECTION__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROTOCOL_INSPECTION__disable_effects,
    .hover_index = 1470,
};

// PROTOCOL_INSPECTION::id
inline constexpr FormDesc irules_misc_PROTOCOL_INSPECTION__id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PROTOCOL_INSPECTION__id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PROTOCOL_INSPECTION__id_cmd{
    .name = "PROTOCOL_INSPECTION::id",
    .forms = irules_misc_PROTOCOL_INSPECTION__id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PROTOCOL_INSPECTION__id_effects,
    .hover_index = 1471,
};

// PSC::aaa_reporting_interval
inline constexpr FormDesc irules_misc_PSC__aaa_reporting_interval_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__aaa_reporting_interval_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__aaa_reporting_interval_cmd{
    .name = "PSC::aaa_reporting_interval",
    .forms = irules_misc_PSC__aaa_reporting_interval_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__aaa_reporting_interval_effects,
    .hover_index = 1472,
};

// PSC::attr
inline constexpr FormDesc irules_misc_PSC__attr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__attr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__attr_cmd{
    .name = "PSC::attr",
    .forms = irules_misc_PSC__attr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__attr_effects,
    .hover_index = 1473,
};

// PSC::calling_id
inline constexpr FormDesc irules_misc_PSC__calling_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__calling_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__calling_id_cmd{
    .name = "PSC::calling_id",
    .forms = irules_misc_PSC__calling_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__calling_id_effects,
    .hover_index = 1474,
};

// PSC::imeisv
inline constexpr FormDesc irules_misc_PSC__imeisv_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__imeisv_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__imeisv_cmd{
    .name = "PSC::imeisv",
    .forms = irules_misc_PSC__imeisv_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__imeisv_effects,
    .hover_index = 1475,
};

// PSC::imsi
inline constexpr FormDesc irules_misc_PSC__imsi_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__imsi_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__imsi_cmd{
    .name = "PSC::imsi",
    .forms = irules_misc_PSC__imsi_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__imsi_effects,
    .hover_index = 1476,
};

// PSC::ip_address
inline constexpr FormDesc irules_misc_PSC__ip_address_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__ip_address_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__ip_address_cmd{
    .name = "PSC::ip_address",
    .forms = irules_misc_PSC__ip_address_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__ip_address_effects,
    .hover_index = 1477,
};

// PSC::lease_time
inline constexpr FormDesc irules_misc_PSC__lease_time_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__lease_time_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__lease_time_cmd{
    .name = "PSC::lease_time",
    .forms = irules_misc_PSC__lease_time_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__lease_time_effects,
    .hover_index = 1478,
};

// PSC::policy
inline constexpr FormDesc irules_misc_PSC__policy_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__policy_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__policy_cmd{
    .name = "PSC::policy",
    .forms = irules_misc_PSC__policy_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__policy_effects,
    .hover_index = 1479,
};

// PSC::subscriber_id
inline constexpr FormDesc irules_misc_PSC__subscriber_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__subscriber_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__subscriber_id_cmd{
    .name = "PSC::subscriber_id",
    .forms = irules_misc_PSC__subscriber_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__subscriber_id_effects,
    .hover_index = 1480,
};

// PSC::tower_id
inline constexpr FormDesc irules_misc_PSC__tower_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__tower_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__tower_id_cmd{
    .name = "PSC::tower_id",
    .forms = irules_misc_PSC__tower_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__tower_id_effects,
    .hover_index = 1481,
};

// PSC::user_name
inline constexpr FormDesc irules_misc_PSC__user_name_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSC__user_name_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSC__user_name_cmd{
    .name = "PSC::user_name",
    .forms = irules_misc_PSC__user_name_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSC__user_name_effects,
    .hover_index = 1482,
};

// PSM::FTP::disable
inline constexpr FormDesc irules_misc_PSM__FTP__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSM__FTP__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSM__FTP__disable_cmd{
    .name = "PSM::FTP::disable",
    .forms = irules_misc_PSM__FTP__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSM__FTP__disable_effects,
    .hover_index = 1483,
};

// PSM::FTP::enable
inline constexpr FormDesc irules_misc_PSM__FTP__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSM__FTP__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSM__FTP__enable_cmd{
    .name = "PSM::FTP::enable",
    .forms = irules_misc_PSM__FTP__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSM__FTP__enable_effects,
    .hover_index = 1484,
};

// PSM::HTTP::disable
inline constexpr FormDesc irules_misc_PSM__HTTP__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSM__HTTP__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSM__HTTP__disable_cmd{
    .name = "PSM::HTTP::disable",
    .forms = irules_misc_PSM__HTTP__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_PSM__HTTP__disable_effects,
    .hover_index = 1485,
};

// PSM::HTTP::enable
inline constexpr FormDesc irules_misc_PSM__HTTP__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSM__HTTP__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSM__HTTP__enable_cmd{
    .name = "PSM::HTTP::enable",
    .forms = irules_misc_PSM__HTTP__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_PSM__HTTP__enable_effects,
    .hover_index = 1486,
};

// PSM::SMTP::disable
inline constexpr FormDesc irules_misc_PSM__SMTP__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSM__SMTP__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSM__SMTP__disable_cmd{
    .name = "PSM::SMTP::disable",
    .forms = irules_misc_PSM__SMTP__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSM__SMTP__disable_effects,
    .hover_index = 1487,
};

// PSM::SMTP::enable
inline constexpr FormDesc irules_misc_PSM__SMTP__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_PSM__SMTP__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_PSM__SMTP__enable_cmd{
    .name = "PSM::SMTP::enable",
    .forms = irules_misc_PSM__SMTP__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_PSM__SMTP__enable_effects,
    .hover_index = 1488,
};

// QOE::disable
inline constexpr FormDesc irules_misc_QOE__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_QOE__disable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_QOE__disable_cmd{
    .name = "QOE::disable",
    .forms = irules_misc_QOE__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_QOE__disable_effects,
    .hover_index = 1489,
    .deprecated_replacement = "(removed)",
};

// QOE::enable
inline constexpr FormDesc irules_misc_QOE__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_QOE__enable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_QOE__enable_cmd{
    .name = "QOE::enable",
    .forms = irules_misc_QOE__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_QOE__enable_effects,
    .hover_index = 1490,
    .deprecated_replacement = "(removed)",
};

// QOE::video
inline constexpr FormDesc irules_misc_QOE__video_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_QOE__video_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_QOE__video_cmd{
    .name = "QOE::video",
    .forms = irules_misc_QOE__video_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_QOE__video_effects,
    .hover_index = 1491,
    .deprecated_replacement = "(removed)",
};

// RADIUS::avp
inline constexpr FormDesc irules_misc_RADIUS__avp_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RADIUS__avp_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RADIUS__avp_cmd{
    .name = "RADIUS::avp",
    .forms = irules_misc_RADIUS__avp_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RADIUS__avp_effects,
    .hover_index = 1492,
};

// RADIUS::code
inline constexpr FormDesc irules_misc_RADIUS__code_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RADIUS__code_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RADIUS__code_cmd{
    .name = "RADIUS::code",
    .forms = irules_misc_RADIUS__code_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RADIUS__code_effects,
    .hover_index = 1493,
};

// RADIUS::id
inline constexpr FormDesc irules_misc_RADIUS__id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RADIUS__id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RADIUS__id_cmd{
    .name = "RADIUS::id",
    .forms = irules_misc_RADIUS__id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RADIUS__id_effects,
    .hover_index = 1494,
};

// RADIUS::rtdom
inline constexpr FormDesc irules_misc_RADIUS__rtdom_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RADIUS__rtdom_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RADIUS__rtdom_cmd{
    .name = "RADIUS::rtdom",
    .forms = irules_misc_RADIUS__rtdom_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RADIUS__rtdom_effects,
    .hover_index = 1495,
};

// RADIUS::subscriber
inline constexpr FormDesc irules_misc_RADIUS__subscriber_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RADIUS__subscriber_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RADIUS__subscriber_cmd{
    .name = "RADIUS::subscriber",
    .forms = irules_misc_RADIUS__subscriber_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RADIUS__subscriber_effects,
    .hover_index = 1496,
};

// RESOLV::lookup
inline constexpr OptionDesc irules_misc_RESOLV__lookup_opts[] = {
    {.name = "-a", .detail = "Query for type A (IPv4) records."},
    {.name = "-aaaa", .detail = "Query for type AAAA (IPv6) records."},
    {.name = "-ptr", .detail = "Query for PTR records."},
    {.name = "-txt", .detail = "Query for TXT records."},
    {.name = "-mx", .detail = "Query for MX records."},
};

inline constexpr FormDesc irules_misc_RESOLV__lookup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RESOLV__lookup_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_RESOLV__lookup_cmd{
    .name = "RESOLV::lookup",
    .options = irules_misc_RESOLV__lookup_opts,
    .forms = irules_misc_RESOLV__lookup_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RESOLV__lookup_effects,
    .hover_index = 1497,
};

// RESOLVER::name_lookup
inline constexpr FormDesc irules_misc_RESOLVER__name_lookup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RESOLVER__name_lookup_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_RESOLVER__name_lookup_cmd{
    .name = "RESOLVER::name_lookup",
    .forms = irules_misc_RESOLVER__name_lookup_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RESOLVER__name_lookup_effects,
    .hover_index = 1498,
};

// RESOLVER::summarize
inline constexpr FormDesc irules_misc_RESOLVER__summarize_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RESOLVER__summarize_effects[] = {
    {.target = SideEffectTarget::DNS_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_RESOLVER__summarize_cmd{
    .name = "RESOLVER::summarize",
    .forms = irules_misc_RESOLVER__summarize_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RESOLVER__summarize_effects,
    .hover_index = 1499,
};

// REST::send
inline constexpr OptionDesc irules_misc_REST__send_opts[] = {
    {.name = "-method", .takes_value = true, .value_hint = "METHOD", .detail = "HTTP method (GET, POST, PUT, DELETE, etc.)."},
};

inline constexpr FormDesc irules_misc_REST__send_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_REST__send_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_REST__send_cmd{
    .name = "REST::send",
    .options = irules_misc_REST__send_opts,
    .forms = irules_misc_REST__send_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_REST__send_effects,
    .hover_index = 1500,
};

// REWRITE::disable
inline constexpr FormDesc irules_misc_REWRITE__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_REWRITE__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_REWRITE__disable_cmd{
    .name = "REWRITE::disable",
    .forms = irules_misc_REWRITE__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_REWRITE__disable_effects,
    .hover_index = 1501,
};

// REWRITE::enable
inline constexpr FormDesc irules_misc_REWRITE__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_REWRITE__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_REWRITE__enable_cmd{
    .name = "REWRITE::enable",
    .forms = irules_misc_REWRITE__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_REWRITE__enable_effects,
    .hover_index = 1502,
};

// REWRITE::payload
inline constexpr FormDesc irules_misc_REWRITE__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_REWRITE__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_REWRITE__payload_cmd{
    .name = "REWRITE::payload",
    .forms = irules_misc_REWRITE__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_REWRITE__payload_effects,
    .hover_index = 1503,
};

// REWRITE::post_process
inline constexpr FormDesc irules_misc_REWRITE__post_process_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_REWRITE__post_process_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_REWRITE__post_process_cmd{
    .name = "REWRITE::post_process",
    .forms = irules_misc_REWRITE__post_process_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_REWRITE__post_process_effects,
    .hover_index = 1504,
};

// ROUTE::age
inline constexpr FormDesc irules_misc_ROUTE__age_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__age_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__age_cmd{
    .name = "ROUTE::age",
    .forms = irules_misc_ROUTE__age_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__age_effects,
    .hover_index = 1505,
    .deprecated_replacement = "(removed)",
};

// ROUTE::bandwidth
inline constexpr FormDesc irules_misc_ROUTE__bandwidth_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__bandwidth_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__bandwidth_cmd{
    .name = "ROUTE::bandwidth",
    .forms = irules_misc_ROUTE__bandwidth_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__bandwidth_effects,
    .hover_index = 1506,
};

// ROUTE::clear
inline constexpr FormDesc irules_misc_ROUTE__clear_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__clear_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__clear_cmd{
    .name = "ROUTE::clear",
    .forms = irules_misc_ROUTE__clear_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__clear_effects,
    .hover_index = 1507,
};

// ROUTE::cwnd
inline constexpr FormDesc irules_misc_ROUTE__cwnd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__cwnd_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__cwnd_cmd{
    .name = "ROUTE::cwnd",
    .forms = irules_misc_ROUTE__cwnd_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__cwnd_effects,
    .hover_index = 1508,
};

// ROUTE::domain
inline constexpr FormDesc irules_misc_ROUTE__domain_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__domain_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__domain_cmd{
    .name = "ROUTE::domain",
    .forms = irules_misc_ROUTE__domain_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__domain_effects,
    .hover_index = 1509,
};

// ROUTE::expiration
inline constexpr FormDesc irules_misc_ROUTE__expiration_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__expiration_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__expiration_cmd{
    .name = "ROUTE::expiration",
    .forms = irules_misc_ROUTE__expiration_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__expiration_effects,
    .hover_index = 1510,
};

// ROUTE::mtu
inline constexpr FormDesc irules_misc_ROUTE__mtu_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__mtu_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__mtu_cmd{
    .name = "ROUTE::mtu",
    .forms = irules_misc_ROUTE__mtu_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__mtu_effects,
    .hover_index = 1511,
};

// ROUTE::rtt
inline constexpr FormDesc irules_misc_ROUTE__rtt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__rtt_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__rtt_cmd{
    .name = "ROUTE::rtt",
    .forms = irules_misc_ROUTE__rtt_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__rtt_effects,
    .hover_index = 1512,
};

// ROUTE::rttvar
inline constexpr FormDesc irules_misc_ROUTE__rttvar_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ROUTE__rttvar_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ROUTE__rttvar_cmd{
    .name = "ROUTE::rttvar",
    .forms = irules_misc_ROUTE__rttvar_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ROUTE__rttvar_effects,
    .hover_index = 1513,
};

// RTSP::collect
inline constexpr FormDesc irules_misc_RTSP__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__collect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__collect_cmd{
    .name = "RTSP::collect",
    .forms = irules_misc_RTSP__collect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__collect_effects,
    .hover_index = 1514,
};

// RTSP::header
inline constexpr FormDesc irules_misc_RTSP__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__header_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__header_cmd{
    .name = "RTSP::header",
    .forms = irules_misc_RTSP__header_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__header_effects,
    .hover_index = 1515,
};

// RTSP::method
inline constexpr FormDesc irules_misc_RTSP__method_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__method_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__method_cmd{
    .name = "RTSP::method",
    .forms = irules_misc_RTSP__method_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__method_effects,
    .hover_index = 1516,
};

// RTSP::msg_source
inline constexpr FormDesc irules_misc_RTSP__msg_source_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__msg_source_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__msg_source_cmd{
    .name = "RTSP::msg_source",
    .forms = irules_misc_RTSP__msg_source_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__msg_source_effects,
    .hover_index = 1517,
};

// RTSP::payload
inline constexpr FormDesc irules_misc_RTSP__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__payload_cmd{
    .name = "RTSP::payload",
    .forms = irules_misc_RTSP__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__payload_effects,
    .hover_index = 1518,
};

// RTSP::release
inline constexpr FormDesc irules_misc_RTSP__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__release_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__release_cmd{
    .name = "RTSP::release",
    .forms = irules_misc_RTSP__release_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__release_effects,
    .hover_index = 1519,
};

// RTSP::respond
inline constexpr FormDesc irules_misc_RTSP__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__respond_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__respond_cmd{
    .name = "RTSP::respond",
    .forms = irules_misc_RTSP__respond_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__respond_effects,
    .hover_index = 1520,
};

// RTSP::status
inline constexpr FormDesc irules_misc_RTSP__status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__status_cmd{
    .name = "RTSP::status",
    .forms = irules_misc_RTSP__status_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__status_effects,
    .hover_index = 1521,
};

// RTSP::uri
inline constexpr FormDesc irules_misc_RTSP__uri_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__uri_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__uri_cmd{
    .name = "RTSP::uri",
    .forms = irules_misc_RTSP__uri_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__uri_effects,
    .hover_index = 1522,
};

// RTSP::version
inline constexpr FormDesc irules_misc_RTSP__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_RTSP__version_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_RTSP__version_cmd{
    .name = "RTSP::version",
    .forms = irules_misc_RTSP__version_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_RTSP__version_effects,
    .hover_index = 1523,
};

// SCTP::client_port
inline constexpr FormDesc irules_misc_SCTP__client_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__client_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__client_port_cmd{
    .name = "SCTP::client_port",
    .forms = irules_misc_SCTP__client_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__client_port_effects,
    .hover_index = 1524,
};

// SCTP::collect
inline constexpr FormDesc irules_misc_SCTP__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__collect_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__collect_cmd{
    .name = "SCTP::collect",
    .forms = irules_misc_SCTP__collect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__collect_effects,
    .hover_index = 1525,
};

// SCTP::local_port
inline constexpr FormDesc irules_misc_SCTP__local_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__local_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__local_port_cmd{
    .name = "SCTP::local_port",
    .forms = irules_misc_SCTP__local_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__local_port_effects,
    .hover_index = 1526,
};

// SCTP::mss
inline constexpr FormDesc irules_misc_SCTP__mss_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__mss_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__mss_cmd{
    .name = "SCTP::mss",
    .forms = irules_misc_SCTP__mss_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__mss_effects,
    .hover_index = 1527,
};

// SCTP::payload
inline constexpr FormDesc irules_misc_SCTP__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__payload_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__payload_cmd{
    .name = "SCTP::payload",
    .forms = irules_misc_SCTP__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__payload_effects,
    .hover_index = 1528,
};

// SCTP::ppi
inline constexpr FormDesc irules_misc_SCTP__ppi_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__ppi_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__ppi_cmd{
    .name = "SCTP::ppi",
    .forms = irules_misc_SCTP__ppi_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__ppi_effects,
    .hover_index = 1529,
};

// SCTP::release
inline constexpr FormDesc irules_misc_SCTP__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__release_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__release_cmd{
    .name = "SCTP::release",
    .forms = irules_misc_SCTP__release_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__release_effects,
    .hover_index = 1530,
};

// SCTP::remote_port
inline constexpr FormDesc irules_misc_SCTP__remote_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__remote_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__remote_port_cmd{
    .name = "SCTP::remote_port",
    .forms = irules_misc_SCTP__remote_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__remote_port_effects,
    .hover_index = 1531,
};

// SCTP::respond
inline constexpr FormDesc irules_misc_SCTP__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__respond_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__respond_cmd{
    .name = "SCTP::respond",
    .forms = irules_misc_SCTP__respond_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__respond_effects,
    .hover_index = 1532,
};

// SCTP::rto_initial
inline constexpr FormDesc irules_misc_SCTP__rto_initial_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__rto_initial_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__rto_initial_cmd{
    .name = "SCTP::rto_initial",
    .forms = irules_misc_SCTP__rto_initial_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__rto_initial_effects,
    .hover_index = 1533,
};

// SCTP::rto_max
inline constexpr FormDesc irules_misc_SCTP__rto_max_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__rto_max_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__rto_max_cmd{
    .name = "SCTP::rto_max",
    .forms = irules_misc_SCTP__rto_max_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__rto_max_effects,
    .hover_index = 1534,
};

// SCTP::rto_min
inline constexpr FormDesc irules_misc_SCTP__rto_min_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__rto_min_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__rto_min_cmd{
    .name = "SCTP::rto_min",
    .forms = irules_misc_SCTP__rto_min_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__rto_min_effects,
    .hover_index = 1535,
};

// SCTP::sack_timeout
inline constexpr FormDesc irules_misc_SCTP__sack_timeout_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__sack_timeout_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__sack_timeout_cmd{
    .name = "SCTP::sack_timeout",
    .forms = irules_misc_SCTP__sack_timeout_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__sack_timeout_effects,
    .hover_index = 1536,
};

// SCTP::server_port
inline constexpr FormDesc irules_misc_SCTP__server_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SCTP__server_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SCTP__server_port_cmd{
    .name = "SCTP::server_port",
    .forms = irules_misc_SCTP__server_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SCTP__server_port_effects,
    .hover_index = 1537,
};

// SDP::field
inline constexpr FormDesc irules_misc_SDP__field_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SDP__field_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SDP__field_cmd{
    .name = "SDP::field",
    .forms = irules_misc_SDP__field_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SDP__field_effects,
    .hover_index = 1538,
};

// SDP::media
inline constexpr FormDesc irules_misc_SDP__media_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SDP__media_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SDP__media_cmd{
    .name = "SDP::media",
    .forms = irules_misc_SDP__media_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SDP__media_effects,
    .hover_index = 1539,
};

// SDP::session_id
inline constexpr FormDesc irules_misc_SDP__session_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SDP__session_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SDP__session_id_cmd{
    .name = "SDP::session_id",
    .forms = irules_misc_SDP__session_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SDP__session_id_effects,
    .hover_index = 1540,
};

// SIP::call_id
inline constexpr FormDesc irules_misc_SIP__call_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__call_id_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__call_id_cmd{
    .name = "SIP::call_id",
    .forms = irules_misc_SIP__call_id_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__call_id_effects,
    .hover_index = 1541,
};

// SIP::discard
inline constexpr FormDesc irules_misc_SIP__discard_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__discard_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__discard_cmd{
    .name = "SIP::discard",
    .forms = irules_misc_SIP__discard_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__discard_effects,
    .hover_index = 1542,
};

// SIP::from
inline constexpr FormDesc irules_misc_SIP__from_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__from_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__from_cmd{
    .name = "SIP::from",
    .forms = irules_misc_SIP__from_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__from_effects,
    .hover_index = 1543,
};

// SIP::header
inline constexpr FormDesc irules_misc_SIP__header_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__header_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_SIP__header_subcmds[] = {
    {.name = "value", .arity = {1, 2}, .pure = true, .hover_index = 1544},
    {.name = "remove", .arity = {1, 2}, .mutator = true, .hover_index = 1545},
    {.name = "insert", .arity = {2, 3}, .mutator = true, .hover_index = 1546},
    {.name = "names", .arity = {0, 0}, .pure = true, .hover_index = 1547},
    {.name = "at", .arity = {1, 1}, .pure = true, .hover_index = 1548},
    {.name = "exists", .arity = {1, 1}, .pure = true, .hover_index = 1549},
    {.name = "replace", .arity = {2, 3}, .mutator = true, .hover_index = 1550},
    {.name = "count", .arity = {1, 1}, .pure = true, .hover_index = 1551},
    {.name = "values", .arity = {1, 1}, .pure = true, .hover_index = 1552},
};

inline constexpr CommandDesc irules_misc_SIP__header_cmd{
    .name = "SIP::header",
    .subcommands = irules_misc_SIP__header_subcmds,
    .forms = irules_misc_SIP__header_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__header_effects,
    .hover_index = 1553,
};

// SIP::message
inline constexpr FormDesc irules_misc_SIP__message_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__message_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__message_cmd{
    .name = "SIP::message",
    .forms = irules_misc_SIP__message_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__message_effects,
    .hover_index = 1554,
};

// SIP::method
inline constexpr FormDesc irules_misc_SIP__method_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__method_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__method_cmd{
    .name = "SIP::method",
    .forms = irules_misc_SIP__method_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__method_effects,
    .hover_index = 1555,
};

// SIP::payload
inline constexpr FormDesc irules_misc_SIP__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__payload_cmd{
    .name = "SIP::payload",
    .forms = irules_misc_SIP__payload_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__payload_effects,
    .hover_index = 1556,
};

// SIP::persist
inline constexpr FormDesc irules_misc_SIP__persist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__persist_effects[] = {
    {.target = SideEffectTarget::PERSISTENCE_TABLE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__persist_cmd{
    .name = "SIP::persist",
    .forms = irules_misc_SIP__persist_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__persist_effects,
    .hover_index = 1557,
};

// SIP::record-route
inline constexpr FormDesc irules_misc_SIP__record_route_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__record_route_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__record_route_cmd{
    .name = "SIP::record-route",
    .forms = irules_misc_SIP__record_route_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__record_route_effects,
    .hover_index = 1558,
};

// SIP::respond
inline constexpr FormDesc irules_misc_SIP__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__respond_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__respond_cmd{
    .name = "SIP::respond",
    .forms = irules_misc_SIP__respond_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__respond_effects,
    .hover_index = 1559,
};

// SIP::response
inline constexpr FormDesc irules_misc_SIP__response_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__response_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_SIP__response_subcmds[] = {
    {.name = "code", .arity = {0, 0}, .pure = true, .hover_index = 1560},
    {.name = "phrase", .arity = {0, 0}, .pure = true, .hover_index = 1561},
    {.name = "rewrite", .arity = {1, 2}, .mutator = true, .hover_index = 1562},
};

inline constexpr CommandDesc irules_misc_SIP__response_cmd{
    .name = "SIP::response",
    .subcommands = irules_misc_SIP__response_subcmds,
    .forms = irules_misc_SIP__response_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__response_effects,
    .hover_index = 1563,
};

// SIP::route
inline constexpr FormDesc irules_misc_SIP__route_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__route_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__route_cmd{
    .name = "SIP::route",
    .forms = irules_misc_SIP__route_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__route_effects,
    .hover_index = 1564,
};

// SIP::route_status
inline constexpr FormDesc irules_misc_SIP__route_status_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__route_status_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__route_status_cmd{
    .name = "SIP::route_status",
    .forms = irules_misc_SIP__route_status_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__route_status_effects,
    .hover_index = 1565,
};

// SIP::to
inline constexpr FormDesc irules_misc_SIP__to_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__to_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__to_cmd{
    .name = "SIP::to",
    .forms = irules_misc_SIP__to_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__to_effects,
    .hover_index = 1566,
};

// SIP::uri
inline constexpr FormDesc irules_misc_SIP__uri_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__uri_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIP__uri_cmd{
    .name = "SIP::uri",
    .forms = irules_misc_SIP__uri_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__uri_effects,
    .hover_index = 1567,
};

// SIP::via
inline constexpr FormDesc irules_misc_SIP__via_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIP__via_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_SIP__via_subcmds[] = {
    {.name = "proto", .arity = {0, 1}, .pure = true, .hover_index = 1568},
    {.name = "sent_by", .arity = {0, 1}, .pure = true, .hover_index = 1569},
    {.name = "received", .arity = {0, 1}, .pure = true, .hover_index = 1570},
    {.name = "branch", .arity = {0, 1}, .pure = true, .hover_index = 1571},
    {.name = "maddr", .arity = {0, 1}, .pure = true, .hover_index = 1572},
    {.name = "ttl", .arity = {0, 1}, .pure = true, .hover_index = 1573},
};

inline constexpr CommandDesc irules_misc_SIP__via_cmd{
    .name = "SIP::via",
    .subcommands = irules_misc_SIP__via_subcmds,
    .forms = irules_misc_SIP__via_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIP__via_effects,
    .hover_index = 1574,
};

// SIPALG::hairpin
inline constexpr FormDesc irules_misc_SIPALG__hairpin_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIPALG__hairpin_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIPALG__hairpin_cmd{
    .name = "SIPALG::hairpin",
    .forms = irules_misc_SIPALG__hairpin_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIPALG__hairpin_effects,
    .hover_index = 1575,
};

// SIPALG::hairpin_default
inline constexpr FormDesc irules_misc_SIPALG__hairpin_default_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIPALG__hairpin_default_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIPALG__hairpin_default_cmd{
    .name = "SIPALG::hairpin_default",
    .forms = irules_misc_SIPALG__hairpin_default_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIPALG__hairpin_default_effects,
    .hover_index = 1576,
};

// SIPALG::nonregister_subscriber_listener
inline constexpr FormDesc irules_misc_SIPALG__nonregister_subscriber_listener_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SIPALG__nonregister_subscriber_listener_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SIPALG__nonregister_subscriber_listener_cmd{
    .name = "SIPALG::nonregister_subscriber_listener",
    .forms = irules_misc_SIPALG__nonregister_subscriber_listener_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::SIP},
    .side_effect_hints = irules_misc_SIPALG__nonregister_subscriber_listener_effects,
    .hover_index = 1577,
};

// SMTPS::activation_mode
inline constexpr FormDesc irules_misc_SMTPS__activation_mode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SMTPS__activation_mode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SMTPS__activation_mode_cmd{
    .name = "SMTPS::activation_mode",
    .forms = irules_misc_SMTPS__activation_mode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SMTPS__activation_mode_effects,
    .hover_index = 1578,
};

// SMTPS::disable
inline constexpr FormDesc irules_misc_SMTPS__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SMTPS__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SMTPS__disable_cmd{
    .name = "SMTPS::disable",
    .forms = irules_misc_SMTPS__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SMTPS__disable_effects,
    .hover_index = 1579,
};

// SMTPS::enable
inline constexpr FormDesc irules_misc_SMTPS__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SMTPS__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SMTPS__enable_cmd{
    .name = "SMTPS::enable",
    .forms = irules_misc_SMTPS__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SMTPS__enable_effects,
    .hover_index = 1580,
};

// SOCKS::allowed
inline constexpr FormDesc irules_misc_SOCKS__allowed_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SOCKS__allowed_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SOCKS__allowed_cmd{
    .name = "SOCKS::allowed",
    .forms = irules_misc_SOCKS__allowed_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SOCKS__allowed_effects,
    .hover_index = 1581,
};

// SOCKS::destination
inline constexpr FormDesc irules_misc_SOCKS__destination_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SOCKS__destination_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SOCKS__destination_cmd{
    .name = "SOCKS::destination",
    .forms = irules_misc_SOCKS__destination_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SOCKS__destination_effects,
    .hover_index = 1582,
};

// SOCKS::version
inline constexpr FormDesc irules_misc_SOCKS__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SOCKS__version_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SOCKS__version_cmd{
    .name = "SOCKS::version",
    .forms = irules_misc_SOCKS__version_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SOCKS__version_effects,
    .hover_index = 1583,
};

// SSE::field
inline constexpr FormDesc irules_misc_SSE__field_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_SSE__field_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_SSE__field_cmd{
    .name = "SSE::field",
    .forms = irules_misc_SSE__field_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_SSE__field_effects,
    .hover_index = 1584,
};

// STATS::get
inline constexpr FormDesc irules_misc_STATS__get_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STATS__get_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_STATS__get_cmd{
    .name = "STATS::get",
    .forms = irules_misc_STATS__get_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STATS__get_effects,
    .hover_index = 1585,
};

// STATS::incr
inline constexpr FormDesc irules_misc_STATS__incr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STATS__incr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_STATS__incr_cmd{
    .name = "STATS::incr",
    .forms = irules_misc_STATS__incr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STATS__incr_effects,
    .hover_index = 1586,
};

// STATS::set
inline constexpr FormDesc irules_misc_STATS__set_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STATS__set_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_STATS__set_cmd{
    .name = "STATS::set",
    .forms = irules_misc_STATS__set_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STATS__set_effects,
    .hover_index = 1587,
};

// STATS::setmax
inline constexpr FormDesc irules_misc_STATS__setmax_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STATS__setmax_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_STATS__setmax_cmd{
    .name = "STATS::setmax",
    .forms = irules_misc_STATS__setmax_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STATS__setmax_effects,
    .hover_index = 1588,
};

// STATS::setmin
inline constexpr FormDesc irules_misc_STATS__setmin_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STATS__setmin_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_STATS__setmin_cmd{
    .name = "STATS::setmin",
    .forms = irules_misc_STATS__setmin_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STATS__setmin_effects,
    .hover_index = 1589,
};

// STREAM::disable
inline constexpr FormDesc irules_misc_STREAM__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STREAM__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_STREAM__disable_cmd{
    .name = "STREAM::disable",
    .forms = irules_misc_STREAM__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STREAM__disable_effects,
    .hover_index = 1590,
};

// STREAM::enable
inline constexpr FormDesc irules_misc_STREAM__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STREAM__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_STREAM__enable_cmd{
    .name = "STREAM::enable",
    .forms = irules_misc_STREAM__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STREAM__enable_effects,
    .hover_index = 1591,
};

// STREAM::encoding
inline constexpr FormDesc irules_misc_STREAM__encoding_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STREAM__encoding_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_STREAM__encoding_cmd{
    .name = "STREAM::encoding",
    .forms = irules_misc_STREAM__encoding_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STREAM__encoding_effects,
    .hover_index = 1592,
};

// STREAM::expression
inline constexpr FormDesc irules_misc_STREAM__expression_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STREAM__expression_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_STREAM__expression_cmd{
    .name = "STREAM::expression",
    .forms = irules_misc_STREAM__expression_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STREAM__expression_effects,
    .hover_index = 1593,
};

// STREAM::match
inline constexpr FormDesc irules_misc_STREAM__match_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STREAM__match_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_STREAM__match_cmd{
    .name = "STREAM::match",
    .forms = irules_misc_STREAM__match_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STREAM__match_effects,
    .hover_index = 1594,
};

// STREAM::max_matchsize
inline constexpr FormDesc irules_misc_STREAM__max_matchsize_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STREAM__max_matchsize_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_STREAM__max_matchsize_cmd{
    .name = "STREAM::max_matchsize",
    .forms = irules_misc_STREAM__max_matchsize_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_STREAM__max_matchsize_effects,
    .hover_index = 1595,
};

// STREAM::replace
inline constexpr FormDesc irules_misc_STREAM__replace_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_STREAM__replace_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_STREAM__replace_cmd{
    .name = "STREAM::replace",
    .forms = irules_misc_STREAM__replace_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_STREAM__replace_effects,
    .hover_index = 1596,
};

// TAP::action
inline constexpr FormDesc irules_misc_TAP__action_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TAP__action_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TAP__action_cmd{
    .name = "TAP::action",
    .forms = irules_misc_TAP__action_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TAP__action_effects,
    .hover_index = 1597,
};

// TAP::config
inline constexpr FormDesc irules_misc_TAP__config_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TAP__config_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TAP__config_cmd{
    .name = "TAP::config",
    .forms = irules_misc_TAP__config_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TAP__config_effects,
    .hover_index = 1598,
};

// TAP::insight
inline constexpr FormDesc irules_misc_TAP__insight_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TAP__insight_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TAP__insight_cmd{
    .name = "TAP::insight",
    .forms = irules_misc_TAP__insight_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TAP__insight_effects,
    .hover_index = 1599,
};

// TAP::insight_requested
inline constexpr FormDesc irules_misc_TAP__insight_requested_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TAP__insight_requested_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TAP__insight_requested_cmd{
    .name = "TAP::insight_requested",
    .forms = irules_misc_TAP__insight_requested_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TAP__insight_requested_effects,
    .hover_index = 1600,
};

// TAP::score
inline constexpr FormDesc irules_misc_TAP__score_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TAP__score_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TAP__score_cmd{
    .name = "TAP::score",
    .forms = irules_misc_TAP__score_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TAP__score_effects,
    .hover_index = 1601,
};

// TCP::abc
inline constexpr FormDesc irules_misc_TCP__abc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__abc_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__abc_cmd{
    .name = "TCP::abc",
    .forms = irules_misc_TCP__abc_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__abc_effects,
    .hover_index = 1602,
};

// TCP::analytics
inline constexpr FormDesc irules_misc_TCP__analytics_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__analytics_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__analytics_cmd{
    .name = "TCP::analytics",
    .forms = irules_misc_TCP__analytics_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__analytics_effects,
    .hover_index = 1603,
};

// TCP::autowin
inline constexpr FormDesc irules_misc_TCP__autowin_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__autowin_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__autowin_cmd{
    .name = "TCP::autowin",
    .forms = irules_misc_TCP__autowin_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__autowin_effects,
    .hover_index = 1604,
};

// TCP::bandwidth
inline constexpr FormDesc irules_misc_TCP__bandwidth_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__bandwidth_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_misc_TCP__bandwidth_cmd{
    .name = "TCP::bandwidth",
    .forms = irules_misc_TCP__bandwidth_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_TCP__bandwidth_effects,
    .hover_index = 1605,
};

// TCP::client_port
inline constexpr FormDesc irules_misc_TCP__client_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__client_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::CLIENT, .scope = StorageScope::CONNECTION},
};

inline constexpr CommandDesc irules_misc_TCP__client_port_cmd{
    .name = "TCP::client_port",
    .forms = irules_misc_TCP__client_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_TCP__client_port_effects,
    .hover_index = 1606,
};

// TCP::close
inline constexpr FormDesc irules_misc_TCP__close_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__close_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__close_cmd{
    .name = "TCP::close",
    .forms = irules_misc_TCP__close_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .diagram_action = true,
    .side_effect_hints = irules_misc_TCP__close_effects,
    .hover_index = 1607,
};

// TCP::collect
inline constexpr FormDesc irules_misc_TCP__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__collect_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__collect_cmd{
    .name = "TCP::collect",
    .forms = irules_misc_TCP__collect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .diagram_action = true,
    .side_effect_hints = irules_misc_TCP__collect_effects,
    .hover_index = 1608,
};

// TCP::congestion
inline constexpr FormDesc irules_misc_TCP__congestion_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__congestion_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__congestion_cmd{
    .name = "TCP::congestion",
    .forms = irules_misc_TCP__congestion_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__congestion_effects,
    .hover_index = 1609,
};

// TCP::delayed_ack
inline constexpr FormDesc irules_misc_TCP__delayed_ack_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__delayed_ack_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__delayed_ack_cmd{
    .name = "TCP::delayed_ack",
    .forms = irules_misc_TCP__delayed_ack_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__delayed_ack_effects,
    .hover_index = 1610,
};

// TCP::dsack
inline constexpr FormDesc irules_misc_TCP__dsack_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__dsack_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__dsack_cmd{
    .name = "TCP::dsack",
    .forms = irules_misc_TCP__dsack_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__dsack_effects,
    .hover_index = 1611,
};

// TCP::earlyrxmit
inline constexpr FormDesc irules_misc_TCP__earlyrxmit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__earlyrxmit_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__earlyrxmit_cmd{
    .name = "TCP::earlyrxmit",
    .forms = irules_misc_TCP__earlyrxmit_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__earlyrxmit_effects,
    .hover_index = 1612,
};

// TCP::ecn
inline constexpr FormDesc irules_misc_TCP__ecn_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__ecn_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__ecn_cmd{
    .name = "TCP::ecn",
    .forms = irules_misc_TCP__ecn_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__ecn_effects,
    .hover_index = 1613,
};

// TCP::enhanced_loss_recovery
inline constexpr FormDesc irules_misc_TCP__enhanced_loss_recovery_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__enhanced_loss_recovery_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__enhanced_loss_recovery_cmd{
    .name = "TCP::enhanced_loss_recovery",
    .forms = irules_misc_TCP__enhanced_loss_recovery_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__enhanced_loss_recovery_effects,
    .hover_index = 1614,
};

// TCP::idletime
inline constexpr FormDesc irules_misc_TCP__idletime_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__idletime_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__idletime_cmd{
    .name = "TCP::idletime",
    .forms = irules_misc_TCP__idletime_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__idletime_effects,
    .hover_index = 1615,
};

// TCP::keepalive
inline constexpr FormDesc irules_misc_TCP__keepalive_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__keepalive_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__keepalive_cmd{
    .name = "TCP::keepalive",
    .forms = irules_misc_TCP__keepalive_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__keepalive_effects,
    .hover_index = 1616,
};

// TCP::limxmit
inline constexpr FormDesc irules_misc_TCP__limxmit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__limxmit_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__limxmit_cmd{
    .name = "TCP::limxmit",
    .forms = irules_misc_TCP__limxmit_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__limxmit_effects,
    .hover_index = 1617,
};

// TCP::local_port
inline constexpr FormDesc irules_misc_TCP__local_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__local_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::CONNECTION},
};

inline constexpr CommandDesc irules_misc_TCP__local_port_cmd{
    .name = "TCP::local_port",
    .forms = irules_misc_TCP__local_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_TCP__local_port_effects,
    .hover_index = 1618,
};

// TCP::lossfilter
inline constexpr FormDesc irules_misc_TCP__lossfilter_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__lossfilter_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__lossfilter_cmd{
    .name = "TCP::lossfilter",
    .forms = irules_misc_TCP__lossfilter_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__lossfilter_effects,
    .hover_index = 1619,
};

// TCP::lossfilterburst
inline constexpr FormDesc irules_misc_TCP__lossfilterburst_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__lossfilterburst_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__lossfilterburst_cmd{
    .name = "TCP::lossfilterburst",
    .forms = irules_misc_TCP__lossfilterburst_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__lossfilterburst_effects,
    .hover_index = 1620,
};

// TCP::lossfilterrate
inline constexpr FormDesc irules_misc_TCP__lossfilterrate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__lossfilterrate_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__lossfilterrate_cmd{
    .name = "TCP::lossfilterrate",
    .forms = irules_misc_TCP__lossfilterrate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__lossfilterrate_effects,
    .hover_index = 1621,
};

// TCP::mss
inline constexpr FormDesc irules_misc_TCP__mss_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__mss_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__mss_cmd{
    .name = "TCP::mss",
    .forms = irules_misc_TCP__mss_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__mss_effects,
    .hover_index = 1622,
};

// TCP::nagle
inline constexpr FormDesc irules_misc_TCP__nagle_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__nagle_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__nagle_cmd{
    .name = "TCP::nagle",
    .forms = irules_misc_TCP__nagle_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__nagle_effects,
    .hover_index = 1623,
};

// TCP::naglemode
inline constexpr FormDesc irules_misc_TCP__naglemode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__naglemode_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__naglemode_cmd{
    .name = "TCP::naglemode",
    .forms = irules_misc_TCP__naglemode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__naglemode_effects,
    .hover_index = 1624,
};

// TCP::naglestate
inline constexpr FormDesc irules_misc_TCP__naglestate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__naglestate_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__naglestate_cmd{
    .name = "TCP::naglestate",
    .forms = irules_misc_TCP__naglestate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__naglestate_effects,
    .hover_index = 1625,
};

// TCP::notify
inline constexpr FormDesc irules_misc_TCP__notify_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__notify_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__notify_cmd{
    .name = "TCP::notify",
    .forms = irules_misc_TCP__notify_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__notify_effects,
    .hover_index = 1626,
};

// TCP::offset
inline constexpr FormDesc irules_misc_TCP__offset_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__offset_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__offset_cmd{
    .name = "TCP::offset",
    .forms = irules_misc_TCP__offset_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__offset_effects,
    .hover_index = 1627,
};

// TCP::option
inline constexpr FormDesc irules_misc_TCP__option_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__option_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_TCP__option_subcmds[] = {
    {.name = "get", .arity = {1, 1}, .pure = true, .hover_index = 1628},
    {.name = "set", .arity = {2, 3}, .mutator = true, .hover_index = 1629},
    {.name = "noset", .arity = {1, 1}, .mutator = true, .hover_index = 1630},
};

inline constexpr CommandDesc irules_misc_TCP__option_cmd{
    .name = "TCP::option",
    .subcommands = irules_misc_TCP__option_subcmds,
    .forms = irules_misc_TCP__option_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__option_effects,
    .hover_index = 1631,
};

// TCP::pacing
inline constexpr FormDesc irules_misc_TCP__pacing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__pacing_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__pacing_cmd{
    .name = "TCP::pacing",
    .forms = irules_misc_TCP__pacing_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__pacing_effects,
    .hover_index = 1632,
};

// TCP::payload
inline constexpr FormDesc irules_misc_TCP__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__payload_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__payload_cmd{
    .name = "TCP::payload",
    .forms = irules_misc_TCP__payload_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__payload_effects,
    .hover_index = 1633,
};

// TCP::proxybuffer
inline constexpr FormDesc irules_misc_TCP__proxybuffer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__proxybuffer_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__proxybuffer_cmd{
    .name = "TCP::proxybuffer",
    .forms = irules_misc_TCP__proxybuffer_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__proxybuffer_effects,
    .hover_index = 1634,
};

// TCP::proxybufferhigh
inline constexpr FormDesc irules_misc_TCP__proxybufferhigh_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__proxybufferhigh_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__proxybufferhigh_cmd{
    .name = "TCP::proxybufferhigh",
    .forms = irules_misc_TCP__proxybufferhigh_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__proxybufferhigh_effects,
    .hover_index = 1635,
};

// TCP::proxybufferlow
inline constexpr FormDesc irules_misc_TCP__proxybufferlow_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__proxybufferlow_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__proxybufferlow_cmd{
    .name = "TCP::proxybufferlow",
    .forms = irules_misc_TCP__proxybufferlow_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__proxybufferlow_effects,
    .hover_index = 1636,
};

// TCP::push_flag
inline constexpr FormDesc irules_misc_TCP__push_flag_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__push_flag_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__push_flag_cmd{
    .name = "TCP::push_flag",
    .forms = irules_misc_TCP__push_flag_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__push_flag_effects,
    .hover_index = 1637,
};

// TCP::rcv_scale
inline constexpr FormDesc irules_misc_TCP__rcv_scale_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__rcv_scale_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__rcv_scale_cmd{
    .name = "TCP::rcv_scale",
    .forms = irules_misc_TCP__rcv_scale_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__rcv_scale_effects,
    .hover_index = 1638,
};

// TCP::rcv_size
inline constexpr FormDesc irules_misc_TCP__rcv_size_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__rcv_size_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__rcv_size_cmd{
    .name = "TCP::rcv_size",
    .forms = irules_misc_TCP__rcv_size_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__rcv_size_effects,
    .hover_index = 1639,
};

// TCP::recvwnd
inline constexpr FormDesc irules_misc_TCP__recvwnd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__recvwnd_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__recvwnd_cmd{
    .name = "TCP::recvwnd",
    .forms = irules_misc_TCP__recvwnd_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__recvwnd_effects,
    .hover_index = 1640,
};

// TCP::release
inline constexpr FormDesc irules_misc_TCP__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__release_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__release_cmd{
    .name = "TCP::release",
    .forms = irules_misc_TCP__release_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .diagram_action = true,
    .side_effect_hints = irules_misc_TCP__release_effects,
    .hover_index = 1641,
};

// TCP::remote_port
inline constexpr FormDesc irules_misc_TCP__remote_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__remote_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::CONNECTION},
};

inline constexpr CommandDesc irules_misc_TCP__remote_port_cmd{
    .name = "TCP::remote_port",
    .forms = irules_misc_TCP__remote_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_TCP__remote_port_effects,
    .hover_index = 1642,
};

// TCP::respond
inline constexpr FormDesc irules_misc_TCP__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__respond_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__respond_cmd{
    .name = "TCP::respond",
    .forms = irules_misc_TCP__respond_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .diagram_action = true,
    .side_effect_hints = irules_misc_TCP__respond_effects,
    .hover_index = 1643,
};

// TCP::rexmt_thresh
inline constexpr FormDesc irules_misc_TCP__rexmt_thresh_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__rexmt_thresh_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__rexmt_thresh_cmd{
    .name = "TCP::rexmt_thresh",
    .forms = irules_misc_TCP__rexmt_thresh_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__rexmt_thresh_effects,
    .hover_index = 1644,
};

// TCP::rt_metrics_timeout
inline constexpr FormDesc irules_misc_TCP__rt_metrics_timeout_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__rt_metrics_timeout_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__rt_metrics_timeout_cmd{
    .name = "TCP::rt_metrics_timeout",
    .forms = irules_misc_TCP__rt_metrics_timeout_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__rt_metrics_timeout_effects,
    .hover_index = 1645,
};

// TCP::rto
inline constexpr FormDesc irules_misc_TCP__rto_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__rto_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__rto_cmd{
    .name = "TCP::rto",
    .forms = irules_misc_TCP__rto_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__rto_effects,
    .hover_index = 1646,
};

// TCP::rtt
inline constexpr FormDesc irules_misc_TCP__rtt_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__rtt_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_misc_TCP__rtt_cmd{
    .name = "TCP::rtt",
    .forms = irules_misc_TCP__rtt_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_TCP__rtt_effects,
    .hover_index = 1647,
};

// TCP::rttvar
inline constexpr FormDesc irules_misc_TCP__rttvar_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__rttvar_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__rttvar_cmd{
    .name = "TCP::rttvar",
    .forms = irules_misc_TCP__rttvar_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__rttvar_effects,
    .hover_index = 1648,
};

// TCP::sendbuf
inline constexpr FormDesc irules_misc_TCP__sendbuf_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__sendbuf_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__sendbuf_cmd{
    .name = "TCP::sendbuf",
    .forms = irules_misc_TCP__sendbuf_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__sendbuf_effects,
    .hover_index = 1649,
};

// TCP::server_port
inline constexpr FormDesc irules_misc_TCP__server_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__server_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::SERVER, .scope = StorageScope::EVENT},
};

inline constexpr CommandDesc irules_misc_TCP__server_port_cmd{
    .name = "TCP::server_port",
    .forms = irules_misc_TCP__server_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .pure = true,
    .cse_candidate = true,
    .side_effect_hints = irules_misc_TCP__server_port_effects,
    .hover_index = 1650,
};

// TCP::setmss
inline constexpr FormDesc irules_misc_TCP__setmss_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__setmss_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__setmss_cmd{
    .name = "TCP::setmss",
    .forms = irules_misc_TCP__setmss_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__setmss_effects,
    .hover_index = 1651,
};

// TCP::snd_cwnd
inline constexpr FormDesc irules_misc_TCP__snd_cwnd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__snd_cwnd_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__snd_cwnd_cmd{
    .name = "TCP::snd_cwnd",
    .forms = irules_misc_TCP__snd_cwnd_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__snd_cwnd_effects,
    .hover_index = 1652,
};

// TCP::snd_scale
inline constexpr FormDesc irules_misc_TCP__snd_scale_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__snd_scale_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__snd_scale_cmd{
    .name = "TCP::snd_scale",
    .forms = irules_misc_TCP__snd_scale_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__snd_scale_effects,
    .hover_index = 1653,
};

// TCP::snd_ssthresh
inline constexpr FormDesc irules_misc_TCP__snd_ssthresh_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__snd_ssthresh_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__snd_ssthresh_cmd{
    .name = "TCP::snd_ssthresh",
    .forms = irules_misc_TCP__snd_ssthresh_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__snd_ssthresh_effects,
    .hover_index = 1654,
};

// TCP::snd_wnd
inline constexpr FormDesc irules_misc_TCP__snd_wnd_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__snd_wnd_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__snd_wnd_cmd{
    .name = "TCP::snd_wnd",
    .forms = irules_misc_TCP__snd_wnd_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TCP__snd_wnd_effects,
    .hover_index = 1655,
};

// TCP::unused_port
inline constexpr FormDesc irules_misc_TCP__unused_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TCP__unused_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TCP__unused_port_cmd{
    .name = "TCP::unused_port",
    .forms = irules_misc_TCP__unused_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP},
    .side_effect_hints = irules_misc_TCP__unused_port_effects,
    .hover_index = 1656,
};

// TDS::msg
inline constexpr FormDesc irules_misc_TDS__msg_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TDS__msg_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TDS__msg_cmd{
    .name = "TDS::msg",
    .forms = irules_misc_TDS__msg_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TDS__msg_effects,
    .hover_index = 1657,
};

// TDS::session
inline constexpr FormDesc irules_misc_TDS__session_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TDS__session_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_TDS__session_cmd{
    .name = "TDS::session",
    .forms = irules_misc_TDS__session_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TDS__session_effects,
    .hover_index = 1658,
};

// TMM::cmp_count
inline constexpr FormDesc irules_misc_TMM__cmp_count_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TMM__cmp_count_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_TMM__cmp_count_cmd{
    .name = "TMM::cmp_count",
    .forms = irules_misc_TMM__cmp_count_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TMM__cmp_count_effects,
    .hover_index = 1659,
};

// TMM::cmp_group
inline constexpr FormDesc irules_misc_TMM__cmp_group_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TMM__cmp_group_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_TMM__cmp_group_cmd{
    .name = "TMM::cmp_group",
    .forms = irules_misc_TMM__cmp_group_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TMM__cmp_group_effects,
    .hover_index = 1660,
};

// TMM::cmp_groups
inline constexpr FormDesc irules_misc_TMM__cmp_groups_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TMM__cmp_groups_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_TMM__cmp_groups_cmd{
    .name = "TMM::cmp_groups",
    .forms = irules_misc_TMM__cmp_groups_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TMM__cmp_groups_effects,
    .hover_index = 1661,
};

// TMM::cmp_primary_group
inline constexpr FormDesc irules_misc_TMM__cmp_primary_group_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TMM__cmp_primary_group_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_TMM__cmp_primary_group_cmd{
    .name = "TMM::cmp_primary_group",
    .forms = irules_misc_TMM__cmp_primary_group_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TMM__cmp_primary_group_effects,
    .hover_index = 1662,
};

// TMM::cmp_unit
inline constexpr FormDesc irules_misc_TMM__cmp_unit_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_TMM__cmp_unit_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_TMM__cmp_unit_cmd{
    .name = "TMM::cmp_unit",
    .forms = irules_misc_TMM__cmp_unit_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_TMM__cmp_unit_effects,
    .hover_index = 1663,
};

// UDP::client_port
inline constexpr FormDesc irules_misc_UDP__client_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__client_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_UDP__client_port_cmd{
    .name = "UDP::client_port",
    .forms = irules_misc_UDP__client_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__client_port_effects,
    .hover_index = 1664,
};

// UDP::debug_queue
inline constexpr FormDesc irules_misc_UDP__debug_queue_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__debug_queue_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__debug_queue_cmd{
    .name = "UDP::debug_queue",
    .forms = irules_misc_UDP__debug_queue_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_UDP__debug_queue_effects,
    .hover_index = 1665,
};

// UDP::drop
inline constexpr FormDesc irules_misc_UDP__drop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__drop_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__drop_cmd{
    .name = "UDP::drop",
    .forms = irules_misc_UDP__drop_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__drop_effects,
    .hover_index = 1666,
};

// UDP::hold
inline constexpr FormDesc irules_misc_UDP__hold_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__hold_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__hold_cmd{
    .name = "UDP::hold",
    .forms = irules_misc_UDP__hold_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__hold_effects,
    .hover_index = 1667,
};

// UDP::local_port
inline constexpr FormDesc irules_misc_UDP__local_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__local_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__local_port_cmd{
    .name = "UDP::local_port",
    .forms = irules_misc_UDP__local_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__local_port_effects,
    .hover_index = 1668,
};

// UDP::max_buf_pkts
inline constexpr FormDesc irules_misc_UDP__max_buf_pkts_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__max_buf_pkts_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__max_buf_pkts_cmd{
    .name = "UDP::max_buf_pkts",
    .forms = irules_misc_UDP__max_buf_pkts_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_UDP__max_buf_pkts_effects,
    .hover_index = 1669,
};

// UDP::max_rate
inline constexpr FormDesc irules_misc_UDP__max_rate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__max_rate_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__max_rate_cmd{
    .name = "UDP::max_rate",
    .forms = irules_misc_UDP__max_rate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_UDP__max_rate_effects,
    .hover_index = 1670,
};

// UDP::mss
inline constexpr FormDesc irules_misc_UDP__mss_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__mss_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__mss_cmd{
    .name = "UDP::mss",
    .forms = irules_misc_UDP__mss_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__mss_effects,
    .hover_index = 1671,
};

// UDP::payload
inline constexpr FormDesc irules_misc_UDP__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__payload_cmd{
    .name = "UDP::payload",
    .forms = irules_misc_UDP__payload_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__payload_effects,
    .hover_index = 1672,
};

// UDP::release
inline constexpr FormDesc irules_misc_UDP__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__release_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__release_cmd{
    .name = "UDP::release",
    .forms = irules_misc_UDP__release_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__release_effects,
    .hover_index = 1673,
};

// UDP::remote_port
inline constexpr FormDesc irules_misc_UDP__remote_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__remote_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__remote_port_cmd{
    .name = "UDP::remote_port",
    .forms = irules_misc_UDP__remote_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__remote_port_effects,
    .hover_index = 1674,
};

// UDP::respond
inline constexpr FormDesc irules_misc_UDP__respond_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__respond_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__respond_cmd{
    .name = "UDP::respond",
    .forms = irules_misc_UDP__respond_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__respond_effects,
    .hover_index = 1675,
};

// UDP::sendbuffer
inline constexpr FormDesc irules_misc_UDP__sendbuffer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__sendbuffer_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__sendbuffer_cmd{
    .name = "UDP::sendbuffer",
    .forms = irules_misc_UDP__sendbuffer_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_UDP__sendbuffer_effects,
    .hover_index = 1676,
};

// UDP::server_port
inline constexpr FormDesc irules_misc_UDP__server_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__server_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_UDP__server_port_cmd{
    .name = "UDP::server_port",
    .forms = irules_misc_UDP__server_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__server_port_effects,
    .hover_index = 1677,
};

// UDP::unused_port
inline constexpr FormDesc irules_misc_UDP__unused_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_UDP__unused_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_UDP__unused_port_cmd{
    .name = "UDP::unused_port",
    .forms = irules_misc_UDP__unused_port_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::UDP},
    .side_effect_hints = irules_misc_UDP__unused_port_effects,
    .hover_index = 1678,
};

// URI::basename
inline constexpr FormDesc irules_misc_URI__basename_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__basename_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__basename_cmd{
    .name = "URI::basename",
    .forms = irules_misc_URI__basename_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__basename_effects,
    .hover_index = 1679,
};

// URI::compare
inline constexpr FormDesc irules_misc_URI__compare_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__compare_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__compare_cmd{
    .name = "URI::compare",
    .forms = irules_misc_URI__compare_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__compare_effects,
    .hover_index = 1680,
};

// URI::decode
inline constexpr FormDesc irules_misc_URI__decode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__decode_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__decode_cmd{
    .name = "URI::decode",
    .forms = irules_misc_URI__decode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__decode_effects,
    .hover_index = 1681,
};

// URI::encode
inline constexpr FormDesc irules_misc_URI__encode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__encode_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__encode_cmd{
    .name = "URI::encode",
    .forms = irules_misc_URI__encode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__encode_effects,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 1682,
};

// URI::encode_component
inline constexpr FormDesc irules_misc_URI__encode_component_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc irules_misc_URI__encode_component_cmd{
    .name = "URI::encode_component",
    .forms = irules_misc_URI__encode_component_forms,
    .dialects = DialectFlags::IRULES,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 1683,
};

// URI::escape
inline constexpr FormDesc irules_misc_URI__escape_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc irules_misc_URI__escape_cmd{
    .name = "URI::escape",
    .forms = irules_misc_URI__escape_forms,
    .dialects = DialectFlags::IRULES,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 1684,
};

// URI::host
inline constexpr FormDesc irules_misc_URI__host_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__host_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__host_cmd{
    .name = "URI::host",
    .forms = irules_misc_URI__host_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__host_effects,
    .hover_index = 1685,
};

// URI::path
inline constexpr FormDesc irules_misc_URI__path_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__path_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__path_cmd{
    .name = "URI::path",
    .forms = irules_misc_URI__path_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__path_effects,
    .hover_index = 1686,
};

// URI::port
inline constexpr FormDesc irules_misc_URI__port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__port_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__port_cmd{
    .name = "URI::port",
    .forms = irules_misc_URI__port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__port_effects,
    .hover_index = 1687,
};

// URI::protocol
inline constexpr FormDesc irules_misc_URI__protocol_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__protocol_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__protocol_cmd{
    .name = "URI::protocol",
    .forms = irules_misc_URI__protocol_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_URI__protocol_effects,
    .hover_index = 1688,
};

// URI::query
inline constexpr FormDesc irules_misc_URI__query_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_URI__query_effects[] = {
    {.target = SideEffectTarget::HTTP_URI, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_URI__query_cmd{
    .name = "URI::query",
    .forms = irules_misc_URI__query_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_URI__query_effects,
    .hover_index = 1689,
};

// VALIDATE::protocol
inline constexpr FormDesc irules_misc_VALIDATE__protocol_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_VALIDATE__protocol_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_VALIDATE__protocol_cmd{
    .name = "VALIDATE::protocol",
    .forms = irules_misc_VALIDATE__protocol_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_VALIDATE__protocol_effects,
    .hover_index = 1690,
};

// VDI::disable
inline constexpr FormDesc irules_misc_VDI__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_VDI__disable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_VDI__disable_cmd{
    .name = "VDI::disable",
    .forms = irules_misc_VDI__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_VDI__disable_effects,
    .hover_index = 1691,
};

// VDI::enable
inline constexpr FormDesc irules_misc_VDI__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_VDI__enable_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_VDI__enable_cmd{
    .name = "VDI::enable",
    .forms = irules_misc_VDI__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_VDI__enable_effects,
    .hover_index = 1692,
};

// WAM::disable
inline constexpr FormDesc irules_misc_WAM__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WAM__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WAM__disable_cmd{
    .name = "WAM::disable",
    .forms = irules_misc_WAM__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_WAM__disable_effects,
    .hover_index = 1693,
    .deprecated_replacement = "(removed)",
};

// WAM::enable
inline constexpr FormDesc irules_misc_WAM__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WAM__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WAM__enable_cmd{
    .name = "WAM::enable",
    .forms = irules_misc_WAM__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WAM__enable_effects,
    .hover_index = 1694,
    .deprecated_replacement = "(removed)",
};

// WEBSSO::disable
inline constexpr FormDesc irules_misc_WEBSSO__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WEBSSO__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WEBSSO__disable_cmd{
    .name = "WEBSSO::disable",
    .forms = irules_misc_WEBSSO__disable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WEBSSO__disable_effects,
    .hover_index = 1695,
};

// WEBSSO::enable
inline constexpr FormDesc irules_misc_WEBSSO__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WEBSSO__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WEBSSO__enable_cmd{
    .name = "WEBSSO::enable",
    .forms = irules_misc_WEBSSO__enable_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WEBSSO__enable_effects,
    .hover_index = 1696,
};

// WEBSSO::select
inline constexpr FormDesc irules_misc_WEBSSO__select_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WEBSSO__select_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WEBSSO__select_cmd{
    .name = "WEBSSO::select",
    .forms = irules_misc_WEBSSO__select_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WEBSSO__select_effects,
    .hover_index = 1697,
};

// WS::collect
inline constexpr FormDesc irules_misc_WS__collect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__collect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__collect_cmd{
    .name = "WS::collect",
    .forms = irules_misc_WS__collect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__collect_effects,
    .hover_index = 1698,
};

// WS::disconnect
inline constexpr FormDesc irules_misc_WS__disconnect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__disconnect_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__disconnect_cmd{
    .name = "WS::disconnect",
    .forms = irules_misc_WS__disconnect_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__disconnect_effects,
    .hover_index = 1699,
};

// WS::enabled
inline constexpr FormDesc irules_misc_WS__enabled_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__enabled_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__enabled_cmd{
    .name = "WS::enabled",
    .forms = irules_misc_WS__enabled_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__enabled_effects,
    .hover_index = 1700,
};

// WS::frame
inline constexpr FormDesc irules_misc_WS__frame_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__frame_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_WS__frame_subcmds[] = {
    {.name = "eom", .arity = {0, 0}, .pure = true, .hover_index = 1701},
    {.name = "drop", .arity = {0, 0}, .mutator = true, .hover_index = 1702},
    {.name = "orig_masked", .arity = {0, 0}, .pure = true, .hover_index = 1703},
    {.name = "type", .arity = {0, 0}, .pure = true, .hover_index = 1704},
    {.name = "mask", .arity = {0, 0}, .pure = true, .hover_index = 1705},
    {.name = "insert", .arity = {2, 3}, .mutator = true, .hover_index = 1706},
    {.name = "prepend", .arity = {1, 1}, .mutator = true, .hover_index = 1707},
    {.name = "append", .arity = {1, 1}, .mutator = true, .hover_index = 1708},
    {.name = "replace", .arity = {1, 1}, .mutator = true, .hover_index = 1709},
};

inline constexpr CommandDesc irules_misc_WS__frame_cmd{
    .name = "WS::frame",
    .subcommands = irules_misc_WS__frame_subcmds,
    .forms = irules_misc_WS__frame_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__frame_effects,
    .hover_index = 1710,
};

// WS::masking
inline constexpr FormDesc irules_misc_WS__masking_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__masking_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__masking_cmd{
    .name = "WS::masking",
    .forms = irules_misc_WS__masking_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__masking_effects,
    .hover_index = 1711,
};

// WS::message
inline constexpr FormDesc irules_misc_WS__message_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__message_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__message_cmd{
    .name = "WS::message",
    .forms = irules_misc_WS__message_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__message_effects,
    .hover_index = 1712,
};

// WS::payload
inline constexpr FormDesc irules_misc_WS__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__payload_cmd{
    .name = "WS::payload",
    .forms = irules_misc_WS__payload_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__payload_effects,
    .hover_index = 1713,
};

// WS::payload_ivs
inline constexpr FormDesc irules_misc_WS__payload_ivs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__payload_ivs_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__payload_ivs_cmd{
    .name = "WS::payload_ivs",
    .forms = irules_misc_WS__payload_ivs_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__payload_ivs_effects,
    .hover_index = 1714,
};

// WS::payload_processing
inline constexpr FormDesc irules_misc_WS__payload_processing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__payload_processing_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__payload_processing_cmd{
    .name = "WS::payload_processing",
    .forms = irules_misc_WS__payload_processing_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__payload_processing_effects,
    .hover_index = 1715,
};

// WS::release
inline constexpr FormDesc irules_misc_WS__release_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__release_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_WS__release_cmd{
    .name = "WS::release",
    .forms = irules_misc_WS__release_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__release_effects,
    .hover_index = 1716,
};

// WS::request
inline constexpr FormDesc irules_misc_WS__request_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__request_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_WS__request_subcmds[] = {
    {.name = "protocol", .arity = {0, 0}, .pure = true, .hover_index = 1717},
    {.name = "extension", .arity = {0, 0}, .pure = true, .hover_index = 1718},
    {.name = "version", .arity = {0, 0}, .pure = true, .hover_index = 1719},
    {.name = "key", .arity = {0, 0}, .pure = true, .hover_index = 1720},
};

inline constexpr CommandDesc irules_misc_WS__request_cmd{
    .name = "WS::request",
    .subcommands = irules_misc_WS__request_subcmds,
    .forms = irules_misc_WS__request_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__request_effects,
    .hover_index = 1721,
};

// WS::response
inline constexpr FormDesc irules_misc_WS__response_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_WS__response_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr SubCmdDesc irules_misc_WS__response_subcmds[] = {
    {.name = "protocol", .arity = {0, 0}, .pure = true, .hover_index = 1722},
    {.name = "extension", .arity = {0, 0}, .pure = true, .hover_index = 1723},
    {.name = "version", .arity = {0, 0}, .pure = true, .hover_index = 1724},
    {.name = "key", .arity = {0, 0}, .pure = true, .hover_index = 1725},
    {.name = "valid", .arity = {0, 0}, .pure = true, .hover_index = 1726},
};

inline constexpr CommandDesc irules_misc_WS__response_cmd{
    .name = "WS::response",
    .subcommands = irules_misc_WS__response_subcmds,
    .forms = irules_misc_WS__response_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_WS__response_effects,
    .hover_index = 1727,
};

// X509::cert_fields
inline constexpr FormDesc irules_misc_X509__cert_fields_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__cert_fields_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__cert_fields_cmd{
    .name = "X509::cert_fields",
    .forms = irules_misc_X509__cert_fields_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__cert_fields_effects,
    .hover_index = 1728,
};

// X509::extensions
inline constexpr FormDesc irules_misc_X509__extensions_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__extensions_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__extensions_cmd{
    .name = "X509::extensions",
    .forms = irules_misc_X509__extensions_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__extensions_effects,
    .hover_index = 1729,
};

// X509::hash
inline constexpr FormDesc irules_misc_X509__hash_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__hash_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__hash_cmd{
    .name = "X509::hash",
    .forms = irules_misc_X509__hash_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__hash_effects,
    .hover_index = 1730,
};

// X509::issuer
inline constexpr FormDesc irules_misc_X509__issuer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__issuer_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__issuer_cmd{
    .name = "X509::issuer",
    .forms = irules_misc_X509__issuer_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__issuer_effects,
    .hover_index = 1731,
};

// X509::not_valid_after
inline constexpr FormDesc irules_misc_X509__not_valid_after_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__not_valid_after_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__not_valid_after_cmd{
    .name = "X509::not_valid_after",
    .forms = irules_misc_X509__not_valid_after_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__not_valid_after_effects,
    .hover_index = 1732,
};

// X509::not_valid_before
inline constexpr FormDesc irules_misc_X509__not_valid_before_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__not_valid_before_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__not_valid_before_cmd{
    .name = "X509::not_valid_before",
    .forms = irules_misc_X509__not_valid_before_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__not_valid_before_effects,
    .hover_index = 1733,
};

// X509::pem2der
inline constexpr FormDesc irules_misc_X509__pem2der_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__pem2der_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__pem2der_cmd{
    .name = "X509::pem2der",
    .forms = irules_misc_X509__pem2der_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__pem2der_effects,
    .hover_index = 1734,
};

// X509::serial_number
inline constexpr FormDesc irules_misc_X509__serial_number_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__serial_number_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__serial_number_cmd{
    .name = "X509::serial_number",
    .forms = irules_misc_X509__serial_number_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__serial_number_effects,
    .hover_index = 1735,
};

// X509::signature_algorithm
inline constexpr FormDesc irules_misc_X509__signature_algorithm_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__signature_algorithm_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__signature_algorithm_cmd{
    .name = "X509::signature_algorithm",
    .forms = irules_misc_X509__signature_algorithm_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__signature_algorithm_effects,
    .hover_index = 1736,
};

// X509::subject
inline constexpr FormDesc irules_misc_X509__subject_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__subject_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__subject_cmd{
    .name = "X509::subject",
    .forms = irules_misc_X509__subject_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__subject_effects,
    .hover_index = 1737,
};

// X509::subject_public_key
inline constexpr FormDesc irules_misc_X509__subject_public_key_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__subject_public_key_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__subject_public_key_cmd{
    .name = "X509::subject_public_key",
    .forms = irules_misc_X509__subject_public_key_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__subject_public_key_effects,
    .hover_index = 1738,
};

// X509::subject_public_key_RSA_bits
inline constexpr FormDesc irules_misc_X509__subject_public_key_RSA_bits_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__subject_public_key_RSA_bits_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__subject_public_key_RSA_bits_cmd{
    .name = "X509::subject_public_key_RSA_bits",
    .forms = irules_misc_X509__subject_public_key_RSA_bits_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__subject_public_key_RSA_bits_effects,
    .hover_index = 1739,
};

// X509::subject_public_key_type
inline constexpr FormDesc irules_misc_X509__subject_public_key_type_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__subject_public_key_type_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__subject_public_key_type_cmd{
    .name = "X509::subject_public_key_type",
    .forms = irules_misc_X509__subject_public_key_type_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__subject_public_key_type_effects,
    .hover_index = 1740,
};

// X509::verify_cert_error_string
inline constexpr FormDesc irules_misc_X509__verify_cert_error_string_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__verify_cert_error_string_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__verify_cert_error_string_cmd{
    .name = "X509::verify_cert_error_string",
    .forms = irules_misc_X509__verify_cert_error_string_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__verify_cert_error_string_effects,
    .hover_index = 1741,
};

// X509::version
inline constexpr FormDesc irules_misc_X509__version_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__version_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__version_cmd{
    .name = "X509::version",
    .forms = irules_misc_X509__version_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__version_effects,
    .hover_index = 1742,
};

// X509::whole
inline constexpr FormDesc irules_misc_X509__whole_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_X509__whole_effects[] = {
    {.target = SideEffectTarget::SSL_STATE, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_X509__whole_cmd{
    .name = "X509::whole",
    .forms = irules_misc_X509__whole_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_X509__whole_effects,
    .hover_index = 1743,
};

// XLAT::listen
inline constexpr OptionDesc irules_misc_XLAT__listen_opts[] = {
    {.name = "-hairpin", .detail = "Enable hairpin mode for the listener."},
    {.name = "-inherit-main-rules", .detail = "Execute main rules attached to parent virtual."},
    {.name = "-single-connection", .detail = "Listener expires after one connection."},
    {.name = "-translation-loose", .detail = "Use hint data as suggestion; don't fail if unusable."},
};

inline constexpr FormDesc irules_misc_XLAT__listen_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XLAT__listen_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XLAT__listen_cmd{
    .name = "XLAT::listen",
    .options = irules_misc_XLAT__listen_opts,
    .forms = irules_misc_XLAT__listen_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XLAT__listen_effects,
    .hover_index = 1744,
};

// XLAT::listen_lifetime
inline constexpr FormDesc irules_misc_XLAT__listen_lifetime_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XLAT__listen_lifetime_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XLAT__listen_lifetime_cmd{
    .name = "XLAT::listen_lifetime",
    .forms = irules_misc_XLAT__listen_lifetime_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XLAT__listen_lifetime_effects,
    .hover_index = 1745,
};

// XLAT::src_addr
inline constexpr FormDesc irules_misc_XLAT__src_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XLAT__src_addr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XLAT__src_addr_cmd{
    .name = "XLAT::src_addr",
    .forms = irules_misc_XLAT__src_addr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XLAT__src_addr_effects,
    .hover_index = 1746,
};

// XLAT::src_config
inline constexpr FormDesc irules_misc_XLAT__src_config_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XLAT__src_config_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XLAT__src_config_cmd{
    .name = "XLAT::src_config",
    .forms = irules_misc_XLAT__src_config_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XLAT__src_config_effects,
    .hover_index = 1747,
};

// XLAT::src_endpoint_reservation
inline constexpr OptionDesc irules_misc_XLAT__src_endpoint_reservation_opts[] = {
    {.name = "-no-persist", .detail = "Skip creation of persist entry."},
    {.name = "-dslite", .takes_value = true, .value_hint = "LOCAL_ADDR REMOTE_ADDR", .detail = "DS-Lite local and remote endpoint."},
    {.name = "-pool", .takes_value = true, .value_hint = "POOL_NAME", .detail = "Specify pool for endpoint reservation."},
    {.name = "-translation-loose", .takes_value = true, .value_hint = "IP PORT", .detail = "Hint data; command won't fail if hints can't be used."},
    {.name = "-translation-strict", .takes_value = true, .value_hint = "IP PORT", .detail = "Hint data; command fails if hints can't be used."},
};

inline constexpr FormDesc irules_misc_XLAT__src_endpoint_reservation_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XLAT__src_endpoint_reservation_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XLAT__src_endpoint_reservation_cmd{
    .name = "XLAT::src_endpoint_reservation",
    .options = irules_misc_XLAT__src_endpoint_reservation_opts,
    .forms = irules_misc_XLAT__src_endpoint_reservation_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XLAT__src_endpoint_reservation_effects,
    .hover_index = 1748,
};

// XLAT::src_nat_valid_range
inline constexpr FormDesc irules_misc_XLAT__src_nat_valid_range_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XLAT__src_nat_valid_range_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XLAT__src_nat_valid_range_cmd{
    .name = "XLAT::src_nat_valid_range",
    .forms = irules_misc_XLAT__src_nat_valid_range_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XLAT__src_nat_valid_range_effects,
    .hover_index = 1749,
};

// XLAT::src_port
inline constexpr FormDesc irules_misc_XLAT__src_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XLAT__src_port_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XLAT__src_port_cmd{
    .name = "XLAT::src_port",
    .forms = irules_misc_XLAT__src_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XLAT__src_port_effects,
    .hover_index = 1750,
};

// XML::address
inline constexpr SideEffectDesc irules_misc_XML__address_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__address_cmd{
    .name = "XML::address",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__address_effects,
    .hover_index = 1751,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::collect
inline constexpr SideEffectDesc irules_misc_XML__collect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__collect_cmd{
    .name = "XML::collect",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__collect_effects,
    .hover_index = 1752,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::disable
inline constexpr FormDesc irules_misc_XML__disable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XML__disable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__disable_cmd{
    .name = "XML::disable",
    .forms = irules_misc_XML__disable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__disable_effects,
    .hover_index = 1753,
};

// XML::element
inline constexpr SideEffectDesc irules_misc_XML__element_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__element_cmd{
    .name = "XML::element",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__element_effects,
    .hover_index = 1754,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::enable
inline constexpr FormDesc irules_misc_XML__enable_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XML__enable_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__enable_cmd{
    .name = "XML::enable",
    .forms = irules_misc_XML__enable_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__enable_effects,
    .hover_index = 1755,
};

// XML::event
inline constexpr SideEffectDesc irules_misc_XML__event_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__event_cmd{
    .name = "XML::event",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__event_effects,
    .hover_index = 1756,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::eventid
inline constexpr SideEffectDesc irules_misc_XML__eventid_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__eventid_cmd{
    .name = "XML::eventid",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__eventid_effects,
    .hover_index = 1757,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::parse
inline constexpr SideEffectDesc irules_misc_XML__parse_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__parse_cmd{
    .name = "XML::parse",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__parse_effects,
    .hover_index = 1758,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::payload
inline constexpr FormDesc irules_misc_XML__payload_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_XML__payload_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__payload_cmd{
    .name = "XML::payload",
    .forms = irules_misc_XML__payload_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__payload_effects,
    .hover_index = 1759,
};

// XML::release
inline constexpr SideEffectDesc irules_misc_XML__release_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__release_cmd{
    .name = "XML::release",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__release_effects,
    .hover_index = 1760,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::soap
inline constexpr SideEffectDesc irules_misc_XML__soap_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__soap_cmd{
    .name = "XML::soap",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__soap_effects,
    .hover_index = 1761,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// XML::subscribe
inline constexpr SideEffectDesc irules_misc_XML__subscribe_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_XML__subscribe_cmd{
    .name = "XML::subscribe",
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_XML__subscribe_effects,
    .hover_index = 1762,
    .deprecated_replacement = "(removed — XML profile deprecated)",
};

// accumulate
inline constexpr CommandDesc irules_misc_accumulate_cmd{
    .name = "accumulate",
    .dialects = DialectFlags::IRULES,
    .hover_index = 1763,
    .deprecated_replacement = "TCP::collect",
};

// active_members
inline constexpr OptionDesc irules_misc_active_members_opts[] = {
    {.name = "-list", .takes_value = true, .detail = "Option -list."},
};

inline constexpr FormDesc irules_misc_active_members_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_active_members_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_active_members_cmd{
    .name = "active_members",
    .options = irules_misc_active_members_opts,
    .forms = irules_misc_active_members_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::DNS},
    .side_effect_hints = irules_misc_active_members_effects,
    .hover_index = 1764,
};

// active_nodes
inline constexpr OptionDesc irules_misc_active_nodes_opts[] = {
    {.name = "-list", .takes_value = true, .detail = "Option -list."},
};

inline constexpr FormDesc irules_misc_active_nodes_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_active_nodes_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_active_nodes_cmd{
    .name = "active_nodes",
    .options = irules_misc_active_nodes_opts,
    .forms = irules_misc_active_nodes_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_active_nodes_effects,
    .hover_index = 1765,
    .deprecated_replacement = "active_members",
};

// after
inline constexpr OptionDesc irules_misc_after_opts[] = {
    {.name = "-periodic", .detail = "Option -periodic."},
    {.name = "-current", .detail = "Option -current."},
};

inline constexpr FormDesc irules_misc_after_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_after_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_after_cmd{
    .name = "after",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .options = irules_misc_after_opts,
    .forms = irules_misc_after_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .xc_translatable = -1,
    .side_effect_hints = irules_misc_after_effects,
    .hover_index = 1766,
};

// b64decode
inline constexpr FormDesc irules_misc_b64decode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_b64decode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_b64decode_cmd{
    .name = "b64decode",
    .forms = irules_misc_b64decode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_b64decode_effects,
    .hover_index = 1767,
};

// b64encode
inline constexpr FormDesc irules_misc_b64encode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_b64encode_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_b64encode_cmd{
    .name = "b64encode",
    .forms = irules_misc_b64encode_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_b64encode_effects,
    .hover_index = 1768,
};

// call
inline constexpr ArgPattern irules_misc_call_args[] = {
    {.kind = FIXED, .role = NAME, .index = 0},
};

inline constexpr FormDesc irules_misc_call_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_call_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_call_cmd{
    .name = "call",
    .arity = {1, std::numeric_limits<std::int32_t>::max()},
    .arg_patterns = irules_misc_call_args,
    .forms = irules_misc_call_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_call_effects,
    .hover_index = 1769,
};

// check
inline constexpr FormDesc irules_misc_check_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_check_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_check_cmd{
    .name = "check",
    .forms = irules_misc_check_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_check_effects,
    .hover_index = 1770,
};

// class
inline constexpr OptionDesc irules_misc_class_opts[] = {
    {.name = "-all", .detail = "Return all matches."},
    {.name = "-value", .detail = "Return value instead of name."},
    {.name = "-name", .detail = "Return name."},
    {.name = "-index", .detail = "Return index."},
    {.name = "-element", .detail = "Return full element."},
    {.name = "-nocase", .detail = "Case-insensitive comparison."},
    {.name = "-list", .detail = "Return value always as a list."},
};

inline constexpr FormDesc irules_misc_class_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_class_effects[] = {
    {.target = SideEffectTarget::DATA_GROUP, .reads = true, .connection_side = ConnectionSide::GLOBAL, .scope = StorageScope::DATA_GROUP},
};

inline constexpr OptionDesc irules_misc_class_sub_match_opts[] = {
    {.name = "-all", .detail = "Return all matches."},
    {.name = "-value", .detail = "Return value instead of name."},
    {.name = "-name", .detail = "Return name."},
    {.name = "-index", .detail = "Return index."},
    {.name = "-element", .detail = "Return full element."},
    {.name = "-nocase", .detail = "Case-insensitive comparison."},
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_class_sub_search_opts[] = {
    {.name = "-all", .detail = "Return all matches."},
    {.name = "-value", .detail = "Return value instead of name."},
    {.name = "-name", .detail = "Return name."},
    {.name = "-index", .detail = "Return index."},
    {.name = "-element", .detail = "Return full element."},
    {.name = "-nocase", .detail = "Case-insensitive comparison."},
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_class_sub_lookup_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_class_sub_element_opts[] = {
    {.name = "-value", .detail = "Return value instead of name."},
    {.name = "-name", .detail = "Return name."},
};

inline constexpr OptionDesc irules_misc_class_sub_names_opts[] = {
    {.name = "-nocase", .detail = "Case-insensitive comparison."},
};

inline constexpr OptionDesc irules_misc_class_sub_nextelement_opts[] = {
    {.name = "-value", .detail = "Return value instead of name."},
    {.name = "-name", .detail = "Return name."},
    {.name = "--", .is_terminator = true},
};

inline constexpr SubCmdDesc irules_misc_class_subcmds[] = {
    {.name = "match", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_class_sub_match_opts, .pure = true, .hover_index = 1771},
    {.name = "search", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_class_sub_search_opts, .pure = true, .hover_index = 1772},
    {.name = "lookup", .arity = {2, 2}, .options = irules_misc_class_sub_lookup_opts, .pure = true, .hover_index = 1773},
    {.name = "element", .arity = {2, 2}, .options = irules_misc_class_sub_element_opts, .pure = true, .hover_index = 1774},
    {.name = "type", .arity = {1, 1}, .pure = true, .hover_index = 1775},
    {.name = "exists", .arity = {1, 1}, .pure = true, .hover_index = 1776},
    {.name = "size", .arity = {1, 1}, .pure = true, .hover_index = 1777},
    {.name = "names", .arity = {0, 1}, .options = irules_misc_class_sub_names_opts, .pure = true, .hover_index = 1778},
    {.name = "get", .arity = {1, 1}, .pure = true, .hover_index = 1779},
    {.name = "startsearch", .arity = {1, 1}, .hover_index = 1780},
    {.name = "nextelement", .arity = {1, 1}, .options = irules_misc_class_sub_nextelement_opts, .hover_index = 1781},
    {.name = "anymore", .arity = {1, 1}, .pure = true, .hover_index = 1782},
    {.name = "donesearch", .arity = {1, 1}, .hover_index = 1783},
};

inline constexpr CommandDesc irules_misc_class_cmd{
    .name = "class",
    .subcommands = irules_misc_class_subcmds,
    .options = irules_misc_class_opts,
    .forms = irules_misc_class_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_class_effects,
    .hover_index = 1784,
};

// client_addr
inline constexpr FormDesc irules_misc_client_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_client_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_client_addr_cmd{
    .name = "client_addr",
    .forms = irules_misc_client_addr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_client_addr_effects,
    .hover_index = 1785,
    .deprecated_replacement = "IP::client_addr",
};

// client_port
inline constexpr FormDesc irules_misc_client_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_client_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_client_port_cmd{
    .name = "client_port",
    .forms = irules_misc_client_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_client_port_effects,
    .hover_index = 1786,
    .deprecated_replacement = "TCP::client_port",
};

// clientside
inline constexpr FormDesc irules_misc_clientside_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_clientside_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_clientside_cmd{
    .name = "clientside",
    .forms = irules_misc_clientside_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_clientside_effects,
    .hover_index = 1787,
};

// clone
inline constexpr FormDesc irules_misc_clone_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_clone_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_clone_cmd{
    .name = "clone",
    .forms = irules_misc_clone_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_clone_effects,
    .hover_index = 1788,
};

// close
inline constexpr ArgPattern irules_misc_close_args[] = {
    {.kind = FIXED, .role = CHANNEL, .index = 0},
};

inline constexpr FormDesc irules_misc_close_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_close_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_close_cmd{
    .name = "close",
    .arity = {1, 2},
    .arg_patterns = irules_misc_close_args,
    .forms = irules_misc_close_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_close_effects,
    .hover_index = 1789,
};

// connect
inline constexpr OptionDesc irules_misc_connect_opts[] = {
    {.name = "-protocol", .takes_value = true, .value_hint = "PROTO", .detail = "IP protocol (default TCP)."},
    {.name = "-myaddr", .takes_value = true, .value_hint = "IP_ADDR", .detail = "Source address for the connection."},
    {.name = "-myport", .takes_value = true, .value_hint = "PORT", .detail = "Source port for the connection."},
    {.name = "-timeout", .takes_value = true, .value_hint = "MSEC", .detail = "Time in ms to wait for connection."},
    {.name = "-idle", .takes_value = true, .value_hint = "SEC", .detail = "Idle timeout in seconds (default 300)."},
    {.name = "-tos", .takes_value = true, .value_hint = "TOS", .detail = "IP TOS value."},
    {.name = "-status", .takes_value = true, .value_hint = "VARIABLE", .detail = "Save connection status into variable."},
};

inline constexpr FormDesc irules_misc_connect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_connect_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_connect_cmd{
    .name = "connect",
    .options = irules_misc_connect_opts,
    .forms = irules_misc_connect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_connect_effects,
    .hover_index = 1790,
};

// cpu
inline constexpr FormDesc irules_misc_cpu_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_cpu_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_cpu_cmd{
    .name = "cpu",
    .forms = irules_misc_cpu_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_cpu_effects,
    .hover_index = 1791,
};

// crc32
inline constexpr FormDesc irules_misc_crc32_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_crc32_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_crc32_cmd{
    .name = "crc32",
    .forms = irules_misc_crc32_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_crc32_effects,
    .hover_index = 1792,
};

// decode_uri
inline constexpr FormDesc irules_misc_decode_uri_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_decode_uri_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_decode_uri_cmd{
    .name = "decode_uri",
    .forms = irules_misc_decode_uri_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_decode_uri_effects,
    .hover_index = 1793,
    .deprecated_replacement = "URI::decode",
};

// discard
inline constexpr FormDesc irules_misc_discard_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_discard_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_discard_cmd{
    .name = "discard",
    .forms = irules_misc_discard_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_discard_effects,
    .hover_index = 1794,
};

// domain
inline constexpr FormDesc irules_misc_domain_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_domain_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_domain_cmd{
    .name = "domain",
    .forms = irules_misc_domain_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_domain_effects,
    .hover_index = 1795,
};

// drop
inline constexpr FormDesc irules_misc_drop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_drop_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_drop_cmd{
    .name = "drop",
    .forms = irules_misc_drop_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_drop_effects,
    .hover_index = 1796,
};

// event
inline constexpr FormDesc irules_misc_event_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_event_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_event_cmd{
    .name = "event",
    .forms = irules_misc_event_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_event_effects,
    .hover_index = 1797,
};

// fasthash
inline constexpr FormDesc irules_misc_fasthash_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_fasthash_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_fasthash_cmd{
    .name = "fasthash",
    .forms = irules_misc_fasthash_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_fasthash_effects,
    .hover_index = 1798,
};

// findclass
inline constexpr FormDesc irules_misc_findclass_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_findclass_effects[] = {
    {.target = SideEffectTarget::DATA_GROUP, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_findclass_cmd{
    .name = "findclass",
    .forms = irules_misc_findclass_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_findclass_effects,
    .hover_index = 1799,
    .deprecated_replacement = "class match / class search",
};

// findstr
inline constexpr FormDesc irules_misc_findstr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_findstr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_findstr_cmd{
    .name = "findstr",
    .forms = irules_misc_findstr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_findstr_effects,
    .hover_index = 1800,
};

// forward
inline constexpr FormDesc irules_misc_forward_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_forward_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_forward_cmd{
    .name = "forward",
    .forms = irules_misc_forward_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_forward_effects,
    .hover_index = 1801,
};

// getfield
inline constexpr FormDesc irules_misc_getfield_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_getfield_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_getfield_cmd{
    .name = "getfield",
    .forms = irules_misc_getfield_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_getfield_effects,
    .hover_index = 1802,
};

// html_encode
inline constexpr FormDesc irules_misc_html_encode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc irules_misc_html_encode_cmd{
    .name = "html_encode",
    .forms = irules_misc_html_encode_forms,
    .dialects = DialectFlags::IRULES,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 1803,
};

// html_escape
inline constexpr FormDesc irules_misc_html_escape_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc irules_misc_html_escape_cmd{
    .name = "html_escape",
    .forms = irules_misc_html_escape_forms,
    .dialects = DialectFlags::IRULES,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 1804,
};

// htmlencode
inline constexpr FormDesc irules_misc_htmlencode_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr CommandDesc irules_misc_htmlencode_cmd{
    .name = "htmlencode",
    .forms = irules_misc_htmlencode_forms,
    .dialects = DialectFlags::IRULES,
    .taint_hints = {.transform_colour = TaintColour::TAINTED},
    .hover_index = 1805,
};

// htonl
inline constexpr FormDesc irules_misc_htonl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_htonl_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_htonl_cmd{
    .name = "htonl",
    .forms = irules_misc_htonl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_htonl_effects,
    .hover_index = 1806,
};

// htons
inline constexpr FormDesc irules_misc_htons_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_htons_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_htons_cmd{
    .name = "htons",
    .forms = irules_misc_htons_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_htons_effects,
    .hover_index = 1807,
};

// ifile
inline constexpr FormDesc irules_misc_ifile_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ifile_effects[] = {
    {.target = SideEffectTarget::FILE_IO, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_ifile_cmd{
    .name = "ifile",
    .forms = irules_misc_ifile_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ifile_effects,
    .hover_index = 1808,
};

// imid
inline constexpr FormDesc irules_misc_imid_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_imid_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_imid_cmd{
    .name = "imid",
    .forms = irules_misc_imid_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::FASTHTTP | ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_imid_effects,
    .hover_index = 1809,
    .deprecated_replacement = "IMID::id",
};

// ip_addr
inline constexpr CommandDesc irules_misc_ip_addr_cmd{
    .name = "ip_addr",
    .dialects = DialectFlags::IRULES,
    .hover_index = 1810,
    .deprecated_replacement = "IP::addr",
};

// ip_protocol
inline constexpr FormDesc irules_misc_ip_protocol_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ip_protocol_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ip_protocol_cmd{
    .name = "ip_protocol",
    .forms = irules_misc_ip_protocol_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ip_protocol_effects,
    .hover_index = 1811,
    .deprecated_replacement = "IP::protocol",
};

// ip_tos
inline constexpr FormDesc irules_misc_ip_tos_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ip_tos_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ip_tos_cmd{
    .name = "ip_tos",
    .forms = irules_misc_ip_tos_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ip_tos_effects,
    .hover_index = 1812,
    .deprecated_replacement = "IP::tos",
};

// ip_ttl
inline constexpr FormDesc irules_misc_ip_ttl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ip_ttl_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_ip_ttl_cmd{
    .name = "ip_ttl",
    .forms = irules_misc_ip_ttl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ip_ttl_effects,
    .hover_index = 1813,
    .deprecated_replacement = "IP::ttl",
};

// lasthop
inline constexpr FormDesc irules_misc_lasthop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_lasthop_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_lasthop_cmd{
    .name = "lasthop",
    .forms = irules_misc_lasthop_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_lasthop_effects,
    .hover_index = 1814,
};

// link_qos
inline constexpr FormDesc irules_misc_link_qos_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_link_qos_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_link_qos_cmd{
    .name = "link_qos",
    .forms = irules_misc_link_qos_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_link_qos_effects,
    .hover_index = 1815,
    .deprecated_replacement = "LINK::qos",
};

// listen
inline constexpr FormDesc irules_misc_listen_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_listen_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_listen_cmd{
    .name = "listen",
    .forms = irules_misc_listen_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_listen_effects,
    .hover_index = 1816,
};

// llookup
inline constexpr FormDesc irules_misc_llookup_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_llookup_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_llookup_cmd{
    .name = "llookup",
    .arity = {2, 2},
    .forms = irules_misc_llookup_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_llookup_effects,
    .hover_index = 1817,
};

// local_addr
inline constexpr FormDesc irules_misc_local_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_local_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_local_addr_cmd{
    .name = "local_addr",
    .forms = irules_misc_local_addr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_local_addr_effects,
    .hover_index = 1818,
    .deprecated_replacement = "IP::local_addr",
};

// local_port
inline constexpr CommandDesc irules_misc_local_port_cmd{
    .name = "local_port",
    .dialects = DialectFlags::IRULES,
    .hover_index = 1819,
    .deprecated_replacement = "TCP::local_port",
};

// log
inline constexpr OptionDesc irules_misc_log_opts[] = {
    {.name = "-noname", .detail = "Suppress iRule name prefix in log message."},
};

inline constexpr FormDesc irules_misc_log_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_log_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_log_cmd{
    .name = "log",
    .arity = {1, 2},
    .options = irules_misc_log_opts,
    .forms = irules_misc_log_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_log_effects,
    .hover_index = 1820,
};

// matchclass
inline constexpr FormDesc irules_misc_matchclass_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_matchclass_effects[] = {
    {.target = SideEffectTarget::DATA_GROUP, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_matchclass_cmd{
    .name = "matchclass",
    .forms = irules_misc_matchclass_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_matchclass_effects,
    .hover_index = 1821,
    .deprecated_replacement = "class",
};

// md4
inline constexpr FormDesc irules_misc_md4_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_md4_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_md4_cmd{
    .name = "md4",
    .forms = irules_misc_md4_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_md4_effects,
    .hover_index = 1822,
};

// md5
inline constexpr FormDesc irules_misc_md5_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_md5_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_md5_cmd{
    .name = "md5",
    .forms = irules_misc_md5_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_md5_effects,
    .hover_index = 1823,
};

// members
inline constexpr OptionDesc irules_misc_members_opts[] = {
    {.name = "-list", .takes_value = true, .detail = "Option -list."},
};

inline constexpr FormDesc irules_misc_members_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_members_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_members_cmd{
    .name = "members",
    .options = irules_misc_members_opts,
    .forms = irules_misc_members_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_members_effects,
    .hover_index = 1824,
};

// nexthop
inline constexpr FormDesc irules_misc_nexthop_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_nexthop_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_nexthop_cmd{
    .name = "nexthop",
    .forms = irules_misc_nexthop_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_nexthop_effects,
    .hover_index = 1825,
};

// node
inline constexpr FormDesc irules_misc_node_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_node_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_node_cmd{
    .name = "node",
    .arity = {1, 2},
    .forms = irules_misc_node_forms,
    .dialects = DialectFlags::IRULES,
    .cse_candidate = true,
    .diagram_action = true,
    .side_effect_hints = irules_misc_node_effects,
    .hover_index = 1826,
};

// nodes
inline constexpr OptionDesc irules_misc_nodes_opts[] = {
    {.name = "-list", .detail = "Option -list."},
};

inline constexpr FormDesc irules_misc_nodes_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_nodes_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_nodes_cmd{
    .name = "nodes",
    .options = irules_misc_nodes_opts,
    .forms = irules_misc_nodes_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_nodes_effects,
    .hover_index = 1827,
};

// ntohl
inline constexpr FormDesc irules_misc_ntohl_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ntohl_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_ntohl_cmd{
    .name = "ntohl",
    .forms = irules_misc_ntohl_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ntohl_effects,
    .hover_index = 1828,
};

// ntohs
inline constexpr FormDesc irules_misc_ntohs_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_ntohs_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_ntohs_cmd{
    .name = "ntohs",
    .forms = irules_misc_ntohs_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_ntohs_effects,
    .hover_index = 1829,
};

// peer
inline constexpr FormDesc irules_misc_peer_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_peer_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_peer_cmd{
    .name = "peer",
    .forms = irules_misc_peer_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_peer_effects,
    .hover_index = 1830,
};

// pem_dtos
inline constexpr FormDesc irules_misc_pem_dtos_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_pem_dtos_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_pem_dtos_cmd{
    .name = "pem_dtos",
    .forms = irules_misc_pem_dtos_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_pem_dtos_effects,
    .hover_index = 1831,
};

// persist
inline constexpr FormDesc irules_misc_persist_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_persist_effects[] = {
    {.target = SideEffectTarget::PERSISTENCE_TABLE, .writes = true, .connection_side = ConnectionSide::CLIENT, .scope = StorageScope::PERSISTENCE},
};

inline constexpr CommandDesc irules_misc_persist_cmd{
    .name = "persist",
    .forms = irules_misc_persist_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_persist_effects,
    .hover_index = 1832,
};

// pool
inline constexpr FormDesc irules_misc_pool_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_pool_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_pool_cmd{
    .name = "pool",
    .arity = {1, 3},
    .forms = irules_misc_pool_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_pool_effects,
    .hover_index = 1833,
};

// priority
inline constexpr FormDesc irules_misc_priority_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_priority_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_priority_cmd{
    .name = "priority",
    .forms = irules_misc_priority_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_priority_effects,
    .hover_index = 1834,
};

// proc
inline constexpr ArgPattern irules_misc_proc_args[] = {
    {.kind = FIXED, .role = NAME, .index = 0},
    {.kind = FIXED, .role = PARAM_LIST, .index = 1},
    {.kind = FIXED, .role = BODY, .index = 2},
};

inline constexpr FormDesc irules_misc_proc_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_proc_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_proc_cmd{
    .name = "proc",
    .arity = {3, 3},
    .arg_patterns = irules_misc_proc_args,
    .forms = irules_misc_proc_forms,
    .dialects = DialectFlags::IRULES,
    .defines_procedure = true,
    .is_scope_barrier = true,
    .side_effect_hints = irules_misc_proc_effects,
    .hover_index = 1835,
};

// radius_authenticate
inline constexpr FormDesc irules_misc_radius_authenticate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_radius_authenticate_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_radius_authenticate_cmd{
    .name = "radius_authenticate",
    .forms = irules_misc_radius_authenticate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_radius_authenticate_effects,
    .hover_index = 1836,
};

// rateclass
inline constexpr FormDesc irules_misc_rateclass_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_rateclass_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_rateclass_cmd{
    .name = "rateclass",
    .forms = irules_misc_rateclass_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_rateclass_effects,
    .hover_index = 1837,
};

// recv
inline constexpr OptionDesc irules_misc_recv_opts[] = {
    {.name = "-eol", .detail = "Suspend until end-of-line received."},
    {.name = "-peek", .detail = "Return data but leave it buffered."},
    {.name = "-timeout", .takes_value = true, .value_hint = "MSEC", .detail = "Time in ms to wait for data."},
    {.name = "-status", .takes_value = true, .value_hint = "VARIABLE", .detail = "Save recv status into variable."},
};

inline constexpr FormDesc irules_misc_recv_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_recv_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_recv_cmd{
    .name = "recv",
    .options = irules_misc_recv_opts,
    .forms = irules_misc_recv_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_recv_effects,
    .hover_index = 1838,
};

// redirect
inline constexpr FormDesc irules_misc_redirect_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_redirect_effects[] = {
    {.target = SideEffectTarget::RESPONSE_COMMIT, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_redirect_cmd{
    .name = "redirect",
    .forms = irules_misc_redirect_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_redirect_effects,
    .hover_index = 1839,
    .deprecated_replacement = "HTTP::redirect",
};

// reject
inline constexpr FormDesc irules_misc_reject_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_reject_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_reject_cmd{
    .name = "reject",
    .forms = irules_misc_reject_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_reject_effects,
    .hover_index = 1840,
};

// relate_client
inline constexpr FormDesc irules_misc_relate_client_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_relate_client_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_relate_client_cmd{
    .name = "relate_client",
    .forms = irules_misc_relate_client_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_relate_client_effects,
    .hover_index = 1841,
};

// relate_server
inline constexpr FormDesc irules_misc_relate_server_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_relate_server_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_relate_server_cmd{
    .name = "relate_server",
    .forms = irules_misc_relate_server_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_relate_server_effects,
    .hover_index = 1842,
};

// remote_addr
inline constexpr FormDesc irules_misc_remote_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_remote_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_remote_addr_cmd{
    .name = "remote_addr",
    .forms = irules_misc_remote_addr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_remote_addr_effects,
    .hover_index = 1843,
    .deprecated_replacement = "IP::remote_addr",
};

// remote_port
inline constexpr CommandDesc irules_misc_remote_port_cmd{
    .name = "remote_port",
    .dialects = DialectFlags::IRULES,
    .hover_index = 1844,
    .deprecated_replacement = "TCP::remote_port",
};

// rmd160
inline constexpr FormDesc irules_misc_rmd160_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_rmd160_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_rmd160_cmd{
    .name = "rmd160",
    .forms = irules_misc_rmd160_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_rmd160_effects,
    .hover_index = 1845,
};

// send
inline constexpr OptionDesc irules_misc_send_opts[] = {
    {.name = "-timeout", .takes_value = true, .value_hint = "MSEC", .detail = "Time in ms to wait for data to be sent."},
    {.name = "-status", .takes_value = true, .value_hint = "VARIABLE", .detail = "Save send status into variable."},
    {.name = "--", .is_terminator = true},
};

inline constexpr FormDesc irules_misc_send_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_send_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_send_cmd{
    .name = "send",
    .options = irules_misc_send_opts,
    .forms = irules_misc_send_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_send_effects,
    .hover_index = 1846,
};

// server_addr
inline constexpr FormDesc irules_misc_server_addr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_server_addr_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_server_addr_cmd{
    .name = "server_addr",
    .forms = irules_misc_server_addr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_server_addr_effects,
    .hover_index = 1847,
    .deprecated_replacement = "IP::server_addr",
};

// server_port
inline constexpr FormDesc irules_misc_server_port_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_server_port_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_server_port_cmd{
    .name = "server_port",
    .forms = irules_misc_server_port_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_server_port_effects,
    .hover_index = 1848,
    .deprecated_replacement = "TCP::server_port",
};

// serverside
inline constexpr FormDesc irules_misc_serverside_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_serverside_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .reads = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_serverside_cmd{
    .name = "serverside",
    .forms = irules_misc_serverside_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_serverside_effects,
    .hover_index = 1849,
};

// session
inline constexpr FormDesc irules_misc_session_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_session_effects[] = {
    {.target = SideEffectTarget::PERSISTENCE_TABLE, .reads = true, .writes = true, .connection_side = ConnectionSide::CLIENT, .scope = StorageScope::PERSISTENCE},
};

inline constexpr SubCmdDesc irules_misc_session_subcmds[] = {
    {.name = "add", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .mutator = true},
    {.name = "lookup", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true},
    {.name = "delete", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .mutator = true},
    {.name = "count", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .pure = true},
};

inline constexpr CommandDesc irules_misc_session_cmd{
    .name = "session",
    .subcommands = irules_misc_session_subcmds,
    .forms = irules_misc_session_forms,
    .dialects = DialectFlags::IRULES,
    .xc_translatable = -1,
    .side_effect_hints = irules_misc_session_effects,
    .hover_index = 1850,
};

// sha1
inline constexpr FormDesc irules_misc_sha1_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_sha1_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_sha1_cmd{
    .name = "sha1",
    .forms = irules_misc_sha1_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_sha1_effects,
    .hover_index = 1851,
};

// sha256
inline constexpr FormDesc irules_misc_sha256_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_sha256_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_sha256_cmd{
    .name = "sha256",
    .forms = irules_misc_sha256_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_sha256_effects,
    .hover_index = 1852,
};

// sha384
inline constexpr FormDesc irules_misc_sha384_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_sha384_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_sha384_cmd{
    .name = "sha384",
    .forms = irules_misc_sha384_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_sha384_effects,
    .hover_index = 1853,
};

// sha512
inline constexpr FormDesc irules_misc_sha512_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_sha512_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_sha512_cmd{
    .name = "sha512",
    .forms = irules_misc_sha512_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_sha512_effects,
    .hover_index = 1854,
};

// sharedvar
inline constexpr ArgPattern irules_misc_sharedvar_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
};

inline constexpr FormDesc irules_misc_sharedvar_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_sharedvar_effects[] = {
    {.target = SideEffectTarget::VARIABLE, .reads = true, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_sharedvar_cmd{
    .name = "sharedvar",
    .arg_patterns = irules_misc_sharedvar_args,
    .forms = irules_misc_sharedvar_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_sharedvar_effects,
    .hover_index = 1855,
};

// snat
inline constexpr FormDesc irules_misc_snat_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_snat_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_snat_cmd{
    .name = "snat",
    .forms = irules_misc_snat_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .diagram_action = true,
    .side_effect_hints = irules_misc_snat_effects,
    .hover_index = 1856,
};

// snatpool
inline constexpr FormDesc irules_misc_snatpool_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_snatpool_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .writes = true, .connection_side = ConnectionSide::SERVER},
};

inline constexpr CommandDesc irules_misc_snatpool_cmd{
    .name = "snatpool",
    .forms = irules_misc_snatpool_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .side_effect_hints = irules_misc_snatpool_effects,
    .hover_index = 1857,
};

// substr
inline constexpr FormDesc irules_misc_substr_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_substr_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_substr_cmd{
    .name = "substr",
    .forms = irules_misc_substr_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_substr_effects,
    .hover_index = 1858,
};

// table
inline constexpr OptionDesc irules_misc_table_opts[] = {
    {.name = "-mustexist", .detail = "Fail if key does not already exist."},
    {.name = "-excl", .detail = "Fail if key already exists."},
    {.name = "-notouch", .detail = "Do not reset lifetime/timeout on access."},
    {.name = "-subtable", .takes_value = true, .detail = "Operate on a named subtable."},
    {.name = "-georedundancy", .detail = "Enable geo-redundancy for this entry."},
    {.name = "-remaining", .detail = "Return remaining time."},
    {.name = "-count", .detail = "Return count of matching keys."},
    {.name = "-all", .detail = "Delete all keys in a subtable."},
};

inline constexpr FormDesc irules_misc_table_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_table_effects[] = {
    {.target = SideEffectTarget::SESSION_TABLE, .reads = true, .writes = true, .connection_side = ConnectionSide::BOTH, .scope = StorageScope::SESSION_TABLE},
};

inline constexpr OptionDesc irules_misc_table_sub_set_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_delete_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_incr_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_append_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_add_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_replace_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_timeout_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_lookup_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr OptionDesc irules_misc_table_sub_lifetime_opts[] = {
    {.name = "--", .is_terminator = true},
};

inline constexpr SubCmdDesc irules_misc_table_subcmds[] = {
    {.name = "set", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_set_opts, .mutator = true},
    {.name = "delete", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_delete_opts, .mutator = true},
    {.name = "incr", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_incr_opts, .mutator = true},
    {.name = "append", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_append_opts, .mutator = true},
    {.name = "add", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_add_opts, .mutator = true},
    {.name = "replace", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_replace_opts, .mutator = true},
    {.name = "keys", .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.name = "timeout", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_timeout_opts},
    {.name = "lookup", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_lookup_opts},
    {.name = "lifetime", .arity = {0, std::numeric_limits<std::int32_t>::max()}, .options = irules_misc_table_sub_lifetime_opts},
};

inline constexpr CommandDesc irules_misc_table_cmd{
    .name = "table",
    .subcommands = irules_misc_table_subcmds,
    .options = irules_misc_table_opts,
    .forms = irules_misc_table_forms,
    .dialects = DialectFlags::IRULES,
    .diagram_action = true,
    .xc_translatable = -1,
    .side_effect_hints = irules_misc_table_effects,
    .hover_index = 1859,
};

// tcpdump
inline constexpr FormDesc irules_misc_tcpdump_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_tcpdump_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_tcpdump_cmd{
    .name = "tcpdump",
    .forms = irules_misc_tcpdump_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_tcpdump_effects,
    .hover_index = 1860,
};

// timing
inline constexpr FormDesc irules_misc_timing_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_timing_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_timing_cmd{
    .name = "timing",
    .forms = irules_misc_timing_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_timing_effects,
    .hover_index = 1861,
};

// traffic_group
inline constexpr FormDesc irules_misc_traffic_group_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_traffic_group_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_traffic_group_cmd{
    .name = "traffic_group",
    .forms = irules_misc_traffic_group_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_traffic_group_effects,
    .hover_index = 1862,
};

// translate
inline constexpr FormDesc irules_misc_translate_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_translate_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_translate_cmd{
    .name = "translate",
    .forms = irules_misc_translate_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_translate_effects,
    .hover_index = 1863,
};

// uniq_ordered_ip_list
inline constexpr FormDesc irules_misc_uniq_ordered_ip_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_uniq_ordered_ip_list_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_uniq_ordered_ip_list_cmd{
    .name = "uniq_ordered_ip_list",
    .forms = irules_misc_uniq_ordered_ip_list_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_uniq_ordered_ip_list_effects,
    .hover_index = 1864,
};

// uniq_sorted_ip_list
inline constexpr FormDesc irules_misc_uniq_sorted_ip_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_uniq_sorted_ip_list_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_uniq_sorted_ip_list_cmd{
    .name = "uniq_sorted_ip_list",
    .forms = irules_misc_uniq_sorted_ip_list_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_uniq_sorted_ip_list_effects,
    .hover_index = 1865,
};

// urlcatblindquery
inline constexpr FormDesc irules_misc_urlcatblindquery_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_urlcatblindquery_effects[] = {
    {.target = SideEffectTarget::DATA_GROUP, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_urlcatblindquery_cmd{
    .name = "urlcatblindquery",
    .forms = irules_misc_urlcatblindquery_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_urlcatblindquery_effects,
    .hover_index = 1866,
    .deprecated_replacement = "CATEGORY::lookup",
};

// urlcatquery
inline constexpr FormDesc irules_misc_urlcatquery_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_urlcatquery_effects[] = {
    {.target = SideEffectTarget::DATA_GROUP, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_urlcatquery_cmd{
    .name = "urlcatquery",
    .forms = irules_misc_urlcatquery_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.profiles = ProfileFlags::FASTHTTP},
    .side_effect_hints = irules_misc_urlcatquery_effects,
    .hover_index = 1867,
    .deprecated_replacement = "CATEGORY::lookup",
};

// use
inline constexpr FormDesc irules_misc_use_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_use_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_use_cmd{
    .name = "use",
    .forms = irules_misc_use_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_use_effects,
    .hover_index = 1868,
    .deprecated_replacement = "virtual",
};

// virtual
inline constexpr FormDesc irules_misc_virtual_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_virtual_effects[] = {
    {.target = SideEffectTarget::CONNECTION_CONTROL, .writes = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_virtual_cmd{
    .name = "virtual",
    .forms = irules_misc_virtual_forms,
    .dialects = DialectFlags::IRULES,
    .cse_candidate = true,
    .diagram_action = true,
    .side_effect_hints = irules_misc_virtual_effects,
    .hover_index = 1869,
};

// vlan_id
inline constexpr FormDesc irules_misc_vlan_id_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_vlan_id_effects[] = {
    {.target = SideEffectTarget::TCP_STATE, .reads = true, .connection_side = ConnectionSide::BOTH},
};

inline constexpr CommandDesc irules_misc_vlan_id_cmd{
    .name = "vlan_id",
    .forms = irules_misc_vlan_id_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_vlan_id_effects,
    .hover_index = 1870,
    .deprecated_replacement = "VLAN::id",
};

// when
inline constexpr FormDesc irules_misc_when_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_when_effects[] = {
    {.target = SideEffectTarget::INTERP_STATE, .writes = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_when_cmd{
    .name = "when",
    .arity = {2, 6},
    .layout_resolver = LayoutResolver::WHEN,
    .forms = irules_misc_when_forms,
    .dialects = DialectFlags::IRULES,
    .top_level_only = true,
    .side_effect_hints = irules_misc_when_effects,
    .hover_index = 1871,
};

// whereis
inline constexpr FormDesc irules_misc_whereis_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_whereis_effects[] = {
    {.target = SideEffectTarget::UNKNOWN, .reads = true, .connection_side = ConnectionSide::GLOBAL},
};

inline constexpr CommandDesc irules_misc_whereis_cmd{
    .name = "whereis",
    .forms = irules_misc_whereis_forms,
    .dialects = DialectFlags::IRULES,
    .side_effect_hints = irules_misc_whereis_effects,
    .hover_index = 1872,
};

// xff_list
inline constexpr FormDesc irules_misc_xff_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_xff_list_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_xff_list_cmd{
    .name = "xff_list",
    .arity = {0, 1},
    .forms = irules_misc_xff_list_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_xff_list_effects,
    .hover_index = 1873,
};

// xff_uniq_ordered_ip_list
inline constexpr FormDesc irules_misc_xff_uniq_ordered_ip_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_xff_uniq_ordered_ip_list_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_xff_uniq_ordered_ip_list_cmd{
    .name = "xff_uniq_ordered_ip_list",
    .arity = {0, 1},
    .forms = irules_misc_xff_uniq_ordered_ip_list_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_xff_uniq_ordered_ip_list_effects,
    .hover_index = 1874,
};

// xff_uniq_sorted_ip_list
inline constexpr FormDesc irules_misc_xff_uniq_sorted_ip_list_forms[] = {
    {.kind = FormKind::DEFAULT, .arity = {0, std::numeric_limits<std::int32_t>::max()}},
};

inline constexpr SideEffectDesc irules_misc_xff_uniq_sorted_ip_list_effects[] = {
    {.target = SideEffectTarget::HTTP_HEADER, .reads = true, .connection_side = ConnectionSide::CLIENT},
};

inline constexpr CommandDesc irules_misc_xff_uniq_sorted_ip_list_cmd{
    .name = "xff_uniq_sorted_ip_list",
    .arity = {0, 1},
    .forms = irules_misc_xff_uniq_sorted_ip_list_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP, .profiles = ProfileFlags::HTTP},
    .side_effect_hints = irules_misc_xff_uniq_sorted_ip_list_effects,
    .hover_index = 1875,
};
