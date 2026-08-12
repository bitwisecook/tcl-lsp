// @generated — do not edit.
//! Generated scalar per-kind parsers.

// Generated file: uniform by-value parser signatures, `x = y.clone()` assigns,
// and glob re-exports are emitted by codegen and not hand-fixable without
// editing the generator.
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::assigning_clones)]
#![allow(clippy::wildcard_imports)]
#![allow(unused_variables)]

use super::*;

/// Scalar parser for `BigipApmEphemeralAuthSshSecurityConfig` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_apm_ephemeral_auth_ssh_security_config(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipApmEphemeralAuthSshSecurityConfig {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipApmEphemeralAuthSshSecurityConfig::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.ciphers = crate::parser::scalar::list_field(&props, "ciphers");
    obj.hmacs = crate::parser::scalar::list_field(&props, "hmacs");
    obj.kex_methods = crate::parser::scalar::list_field(&props, "kex-methods");
    obj.compressions = crate::parser::scalar::list_field(&props, "compressions");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipApmOauthDbInstance` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_apm_oauth_db_instance(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipApmOauthDbInstance {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipApmOauthDbInstance::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.db_name = crate::parser::scalar::get_str(&props, "db-name");
    obj.purge_frequency = crate::parser::scalar::get_str(&props, "purge-frequency");
    obj.purge_time = crate::parser::scalar::get_str(&props, "purge-time");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipApmPolicyAccessPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_apm_policy_access_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipApmPolicyAccessPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipApmPolicyAccessPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.start_item = crate::parser::scalar::get_str(&props, "start-item");
    obj.default_ending = crate::parser::scalar::get_str(&props, "default-ending");
    obj.items = crate::parser::scalar::list_field(&props, "items");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipApmPolicyAgent` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_apm_policy_agent(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipApmPolicyAgent {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipApmPolicyAgent::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.agent_type = crate::parser::scalar::get_str(&props, "agent-type");
    obj.customization_group = crate::parser::scalar::get_str(&props, "customization-group");
    obj.auth = crate::parser::scalar::get_str(&props, "auth");
    obj.max_logon_attempt = crate::parser::scalar::get_str(&props, "max-logon-attempt");
    obj.auth_max_logon_attempt = crate::parser::scalar::get_str(&props, "auth-max-logon-attempt");
    obj.fetch_nested_groups = crate::parser::scalar::get_str(&props, "fetch-nested-groups");
    obj.fetch_primary_groups = crate::parser::scalar::get_str(&props, "fetch-primary-groups");
    obj.password_source = crate::parser::scalar::get_str(&props, "password-source");
    obj.query = crate::parser::scalar::get_str(&props, "query");
    obj.query_attrname = crate::parser::scalar::get_str(&props, "query-attrname");
    obj.query_filter = crate::parser::scalar::get_str(&props, "query-filter");
    obj.server = crate::parser::scalar::get_str(&props, "server");
    obj.show_extended_error = crate::parser::scalar::get_str(&props, "show-extended-error");
    obj.upn = crate::parser::scalar::get_str(&props, "upn");
    obj.username_source = crate::parser::scalar::get_str(&props, "username-source");
    obj.attribute_consuming_service =
        crate::parser::scalar::get_str(&props, "attribute-consuming-service");
    obj.attr_consuming_service_session_var =
        crate::parser::scalar::get_str(&props, "attr-consuming-service-session-var");
    obj.hints = crate::parser::scalar::get_str(&props, "hints");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipApmPolicyCustomizationSource` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_apm_policy_customization_source(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipApmPolicyCustomizationSource {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipApmPolicyCustomizationSource::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipApmPolicyItem` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_apm_policy_item(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipApmPolicyItem {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipApmPolicyItem::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.caption = crate::parser::scalar::get_str(&props, "caption");
    obj.color = crate::parser::scalar::get_str(&props, "color");
    obj.item_type = crate::parser::scalar::get_str(&props, "item-type");
    obj.agents = crate::parser::scalar::list_field(&props, "agents");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipApmReportDefaultReport` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_apm_report_default_report(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipApmReportDefaultReport {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipApmReportDefaultReport::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.report_name = crate::parser::scalar::get_str(&props, "report-name");
    obj.user = crate::parser::scalar::get_str(&props, "user");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthApmAuth` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_apm_auth(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthApmAuth {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthApmAuth::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.profile = crate::parser::scalar::get_str(&props, "profile");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthCertLdap` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_cert_ldap(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthCertLdap {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthCertLdap::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.bind_dn = crate::parser::scalar::get_str(&props, "bind-dn");
    obj.bind_pw = crate::parser::scalar::get_str(&props, "bind-pw");
    obj.bind_timeout = crate::parser::scalar::get_str(&props, "bind-timeout");
    obj.idle_timeout = crate::parser::scalar::get_str(&props, "idle-timeout");
    obj.login_attribute = crate::parser::scalar::get_str(&props, "login-attribute");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.scope = crate::parser::scalar::get_str(&props, "scope");
    obj.search_base_dn = crate::parser::scalar::get_str(&props, "search-base-dn");
    obj.search_timeout = crate::parser::scalar::get_str(&props, "search-timeout");
    obj.servers = crate::parser::scalar::list_field(&props, "servers");
    obj.ssl = crate::parser::scalar::get_str(&props, "ssl");
    obj.user_template = crate::parser::scalar::get_str(&props, "user-template");
    obj.version = crate::parser::scalar::get_str(&props, "version");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthLdap` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_ldap(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthLdap {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthLdap::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.bind_dn = crate::parser::scalar::get_str(&props, "bind-dn");
    obj.bind_pw = crate::parser::scalar::get_str(&props, "bind-pw");
    obj.bind_timeout = crate::parser::scalar::get_str(&props, "bind-timeout");
    obj.check_host_attr = crate::parser::scalar::get_str(&props, "check-host-attr");
    obj.check_roles_group = crate::parser::scalar::get_str(&props, "check-roles-group");
    obj.filter_ = crate::parser::scalar::get_str(&props, "filter");
    obj.group_dn = crate::parser::scalar::get_str(&props, "group-dn");
    obj.group_member_attribute = crate::parser::scalar::get_str(&props, "group-member-attribute");
    obj.idle_timeout = crate::parser::scalar::get_str(&props, "idle-timeout");
    obj.ignore_auth_info_unavail =
        crate::parser::scalar::get_str(&props, "ignore-auth-info-unavail");
    obj.ignore_unknown_user = crate::parser::scalar::get_str(&props, "ignore-unknown-user");
    obj.login_attribute = crate::parser::scalar::get_str(&props, "login-attribute");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.scope = crate::parser::scalar::get_str(&props, "scope");
    obj.search_base_dn = crate::parser::scalar::get_str(&props, "search-base-dn");
    obj.search_timeout = crate::parser::scalar::get_str(&props, "search-timeout");
    obj.servers = crate::parser::scalar::list_field(&props, "servers");
    obj.ssl = crate::parser::scalar::get_str(&props, "ssl");
    obj.ssl_ca_cert = crate::parser::scalar::get_str(&props, "ssl-ca-cert");
    obj.ssl_check_peer = crate::parser::scalar::get_str(&props, "ssl-check-peer");
    obj.ssl_client_cert = crate::parser::scalar::get_str(&props, "ssl-client-cert");
    obj.ssl_client_key = crate::parser::scalar::get_str(&props, "ssl-client-key");
    obj.user_template = crate::parser::scalar::get_str(&props, "user-template");
    obj.version = crate::parser::scalar::get_str(&props, "version");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthLoginFailures` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_login_failures(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthLoginFailures {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthLoginFailures::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthPartition` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_partition(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthPartition {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthPartition::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.default_route_domain = crate::parser::scalar::get_str(&props, "default-route-domain");
    obj.inherited_traffic_group = crate::parser::scalar::get_str(&props, "inherited-traffic-group");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthPassword` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_password(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthPassword {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthPassword::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.expiration_warning = crate::parser::scalar::get_str(&props, "expiration-warning");
    obj.minimum_length = crate::parser::scalar::get_str(&props, "minimum-length");
    obj.policy = crate::parser::scalar::get_str(&props, "policy");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthPasswordPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_password_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthPasswordPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthPasswordPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.expiration_warning = crate::parser::scalar::get_str(&props, "expiration-warning");
    obj.max_duration = crate::parser::scalar::get_str(&props, "max-duration");
    obj.max_login_failures = crate::parser::scalar::get_str(&props, "max-login-failures");
    obj.min_duration = crate::parser::scalar::get_str(&props, "min-duration");
    obj.minimum_length = crate::parser::scalar::get_str(&props, "minimum-length");
    obj.minimum_regular_characters =
        crate::parser::scalar::get_str(&props, "minimum-regular-characters");
    obj.password_memory = crate::parser::scalar::get_str(&props, "password-memory");
    obj.policy_enforcement = crate::parser::scalar::get_str(&props, "policy-enforcement");
    obj.required_lowercase = crate::parser::scalar::get_str(&props, "required-lowercase");
    obj.required_numeric = crate::parser::scalar::get_str(&props, "required-numeric");
    obj.required_special = crate::parser::scalar::get_str(&props, "required-special");
    obj.required_uppercase = crate::parser::scalar::get_str(&props, "required-uppercase");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthRadius` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_radius(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthRadius {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthRadius::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.service_type = crate::parser::scalar::get_str(&props, "service-type");
    obj.servers = crate::parser::scalar::list_field(&props, "servers");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthRadiusServer` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_radius_server(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthRadiusServer {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthRadiusServer::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.server = crate::parser::scalar::get_str(&props, "server");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.secret = crate::parser::scalar::get_str(&props, "secret");
    obj.timeout = crate::parser::scalar::get_str(&props, "timeout");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthRemoteRole` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_remote_role(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthRemoteRole {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthRemoteRole::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.role_info = crate::parser::scalar::list_field(&props, "role-info");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthRemoteUser` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_remote_user(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthRemoteUser {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthRemoteUser::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.default_partition = crate::parser::scalar::get_str(&props, "default-partition");
    obj.default_role = crate::parser::scalar::get_str(&props, "default-role");
    obj.remote_console_access = crate::parser::scalar::get_str(&props, "remote-console-access");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthSource` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_source(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthSource {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthSource::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.fallback = crate::parser::scalar::get_str(&props, "fallback");
    obj.type_ = crate::parser::scalar::get_str(&props, "type");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthTacacs` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_tacacs(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthTacacs {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthTacacs::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.protocol = crate::parser::scalar::get_str(&props, "protocol");
    obj.secret = crate::parser::scalar::get_str(&props, "secret");
    obj.service = crate::parser::scalar::get_str(&props, "service");
    obj.servers = crate::parser::scalar::list_field(&props, "servers");
    obj.accounting = crate::parser::scalar::get_str(&props, "accounting");
    obj.authentication = crate::parser::scalar::get_str(&props, "authentication");
    obj.debug = crate::parser::scalar::get_str(&props, "debug");
    obj.encryption = crate::parser::scalar::get_str(&props, "encryption");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipAuthUser` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_auth_user(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipAuthUser {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipAuthUser::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.partition = crate::parser::scalar::get_str(&props, "partition");
    obj.shell = crate::parser::scalar::get_str(&props, "shell");
    obj.encrypted_password = crate::parser::scalar::get_str(&props, "encrypted-password");
    obj.partition_access = crate::parser::scalar::list_field(&props, "partition-access");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipCmCert` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_cm_cert(full_path: &str, body: &str, range: crate::range::Range) -> BigipCmCert {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipCmCert::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.cache_path = crate::parser::scalar::get_str(&props, "cache-path");
    obj.checksum = crate::parser::scalar::get_str(&props, "checksum");
    obj.revision = crate::parser::scalar::get_str(&props, "revision");
    obj.issuer = crate::parser::scalar::get_str(&props, "issuer");
    obj.subject = crate::parser::scalar::get_str(&props, "subject");
    obj.subject_alternative_name =
        crate::parser::scalar::get_str(&props, "subject-alternative-name");
    obj.expiration_date = crate::parser::scalar::get_str(&props, "expiration-date");
    obj.expiration_string = crate::parser::scalar::get_str(&props, "expiration-string");
    obj.fingerprint = crate::parser::scalar::get_str(&props, "fingerprint");
    obj.serial_number = crate::parser::scalar::get_str(&props, "serial-number");
    obj.version = crate::parser::scalar::get_str(&props, "version");
    obj.key_type = crate::parser::scalar::get_str(&props, "key-type");
    obj.certificate_key_size = crate::parser::scalar::get_str(&props, "certificate-key-size");
    obj.is_bundle = crate::parser::scalar::get_str(&props, "is-bundle");
    obj.email = crate::parser::scalar::get_str(&props, "email");
    obj.source_path = crate::parser::scalar::get_str(&props, "source-path");
    obj.system_path = crate::parser::scalar::get_str(&props, "system-path");
    obj.size = crate::parser::scalar::get_str(&props, "size");
    obj.mode = crate::parser::scalar::get_str(&props, "mode");
    obj.create_time = crate::parser::scalar::get_str(&props, "create-time");
    obj.created_by = crate::parser::scalar::get_str(&props, "created-by");
    obj.last_update_time = crate::parser::scalar::get_str(&props, "last-update-time");
    obj.updated_by = crate::parser::scalar::get_str(&props, "updated-by");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipCmDevice` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_cm_device(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipCmDevice {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipCmDevice::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.hostname = crate::parser::scalar::get_str(&props, "hostname");
    obj.management_ip = crate::parser::scalar::get_str(&props, "management-ip");
    obj.base_mac = crate::parser::scalar::get_str(&props, "base-mac");
    obj.build = crate::parser::scalar::get_str(&props, "build");
    obj.edition = crate::parser::scalar::get_str(&props, "edition");
    obj.version = crate::parser::scalar::get_str(&props, "version");
    obj.product = crate::parser::scalar::get_str(&props, "product");
    obj.platform_id = crate::parser::scalar::get_str(&props, "platform-id");
    obj.chassis_id = crate::parser::scalar::get_str(&props, "chassis-id");
    obj.marketing_name = crate::parser::scalar::get_str(&props, "marketing-name");
    obj.self_device = crate::parser::scalar::get_str(&props, "self-device");
    obj.time_zone = crate::parser::scalar::get_str(&props, "time-zone");
    obj.cert = crate::parser::scalar::get_str(&props, "cert");
    obj.key = crate::parser::scalar::get_str(&props, "key");
    obj.description = crate::parser::scalar::description(&props);
    obj.comment = crate::parser::scalar::get_str(&props, "comment");
    obj.contact = crate::parser::scalar::get_str(&props, "contact");
    obj.location = crate::parser::scalar::get_str(&props, "location");
    obj.mirror_ip = crate::parser::scalar::get_str(&props, "mirror-ip");
    obj.mirror_secondary_ip = crate::parser::scalar::get_str(&props, "mirror-secondary-ip");
    obj.multicast_interface = crate::parser::scalar::get_str(&props, "multicast-interface");
    obj.multicast_ip = crate::parser::scalar::get_str(&props, "multicast-ip");
    obj.multicast_port = crate::parser::scalar::get_str(&props, "multicast-port");
    obj.unicast_address = crate::parser::scalar::list_field(&props, "unicast-address");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipCmDeviceGroup` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_cm_device_group(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipCmDeviceGroup {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipCmDeviceGroup::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.auto_sync = crate::parser::scalar::get_str(&props, "auto-sync");
    obj.network_failover = crate::parser::scalar::get_str(&props, "network-failover");
    obj.hidden = crate::parser::scalar::get_str(&props, "hidden");
    obj.devices = crate::parser::scalar::list_field(&props, "devices");
    obj.description = crate::parser::scalar::description(&props);
    obj.type_ = crate::parser::scalar::get_str(&props, "type");
    obj.save_on_auto_sync = crate::parser::scalar::get_str(&props, "save-on-auto-sync");
    obj.full_load_on_sync = crate::parser::scalar::get_str(&props, "full-load-on-sync");
    obj.asm_sync = crate::parser::scalar::get_str(&props, "asm-sync");
    obj.incremental_config_sync_size_max =
        crate::parser::scalar::get_str(&props, "incremental-config-sync-size-max");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipCmHaGroup` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_cm_ha_group(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipCmHaGroup {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipCmHaGroup::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.enabled_state = crate::parser::scalar::get_str(&props, "enabled-state");
    obj.active_bonus = crate::parser::scalar::get_str(&props, "active-bonus");
    obj.pools = crate::parser::scalar::list_field(&props, "pools");
    obj.trunks = crate::parser::scalar::list_field(&props, "trunks");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipCmKey` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_cm_key(full_path: &str, body: &str, range: crate::range::Range) -> BigipCmKey {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipCmKey::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.cache_path = crate::parser::scalar::get_str(&props, "cache-path");
    obj.checksum = crate::parser::scalar::get_str(&props, "checksum");
    obj.revision = crate::parser::scalar::get_str(&props, "revision");
    obj.key_size = crate::parser::scalar::get_str(&props, "key-size");
    obj.key_type = crate::parser::scalar::get_str(&props, "key-type");
    obj.security_type = crate::parser::scalar::get_str(&props, "security-type");
    obj.source_path = crate::parser::scalar::get_str(&props, "source-path");
    obj.system_path = crate::parser::scalar::get_str(&props, "system-path");
    obj.size = crate::parser::scalar::get_str(&props, "size");
    obj.mode = crate::parser::scalar::get_str(&props, "mode");
    obj.create_time = crate::parser::scalar::get_str(&props, "create-time");
    obj.created_by = crate::parser::scalar::get_str(&props, "created-by");
    obj.last_update_time = crate::parser::scalar::get_str(&props, "last-update-time");
    obj.updated_by = crate::parser::scalar::get_str(&props, "updated-by");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipCmTrafficGroup` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_cm_traffic_group(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipCmTrafficGroup {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipCmTrafficGroup::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.unit_id = crate::parser::scalar::get_str(&props, "unit-id");
    obj.description = crate::parser::scalar::description(&props);
    obj.default_device = crate::parser::scalar::get_str(&props, "default-device");
    obj.ha_load_factor = crate::parser::scalar::get_str(&props, "ha-load-factor");
    obj.ha_order = crate::parser::scalar::list_field(&props, "ha-order");
    obj.ha_group = crate::parser::scalar::get_str(&props, "ha-group");
    obj.auto_failback_enabled = crate::parser::scalar::get_str(&props, "auto-failback-enabled");
    obj.auto_failback_time = crate::parser::scalar::get_str(&props, "auto-failback-time");
    obj.mac = crate::parser::scalar::get_str(&props, "mac");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipCmTrustDomain` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_cm_trust_domain(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipCmTrustDomain {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipCmTrustDomain::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.ca_cert = crate::parser::scalar::get_str(&props, "ca-cert");
    obj.ca_cert_bundle = crate::parser::scalar::get_str(&props, "ca-cert-bundle");
    obj.ca_key = crate::parser::scalar::get_str(&props, "ca-key");
    obj.ca_devices = crate::parser::scalar::list_field(&props, "ca-devices");
    obj.guid = crate::parser::scalar::get_str(&props, "guid");
    obj.status = crate::parser::scalar::get_str(&props, "status");
    obj.trust_group = crate::parser::scalar::get_str(&props, "trust-group");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmDatacenter` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_datacenter(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmDatacenter {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmDatacenter::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.contact = crate::parser::scalar::get_str(&props, "contact");
    obj.location = crate::parser::scalar::get_str(&props, "location");
    obj.description = crate::parser::scalar::description(&props);
    obj.prober_pool = crate::parser::scalar::get_str(&props, "prober-pool");
    obj.prober_preference = crate::parser::scalar::get_str(&props, "prober-preference");
    obj.prober_fallback = crate::parser::scalar::get_str(&props, "prober-fallback");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmDistributedApp` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_distributed_app(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmDistributedApp {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmDistributedApp::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.wide_ips = crate::parser::scalar::list_field(&props, "wide-ips");
    obj.persist_cidr = crate::parser::scalar::get_str(&props, "persist-cidr");
    obj.dependency_level = crate::parser::scalar::get_str(&props, "dependency-level");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmGlobalSettingsGeneral` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_global_settings_general(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmGlobalSettingsGeneral {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmGlobalSettingsGeneral::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.auto_discovery = crate::parser::scalar::get_str(&props, "auto-discovery");
    obj.synchronization = crate::parser::scalar::get_str(&props, "synchronization");
    obj.synchronization_group_name =
        crate::parser::scalar::get_str(&props, "synchronization-group-name");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmGlobalSettingsLoadBalancing` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_global_settings_load_balancing(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmGlobalSettingsLoadBalancing {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmGlobalSettingsLoadBalancing::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.topology_longest_match = crate::parser::scalar::get_str(&props, "topology-longest-match");
    obj.ignore_path_ttl = crate::parser::scalar::get_str(&props, "ignore-path-ttl");
    obj.respect_dependent_objects =
        crate::parser::scalar::get_str(&props, "respect-dependent-objects");
    obj.verify_vs_availability = crate::parser::scalar::get_str(&props, "verify-vs-availability");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmGlobalSettingsMetrics` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_global_settings_metrics(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmGlobalSettingsMetrics {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmGlobalSettingsMetrics::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.metrics_collection_protocols =
        crate::parser::scalar::list_field(&props, "metrics-collection-protocols");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmGlobalSettingsMetricsExclusions` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_global_settings_metrics_exclusions(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmGlobalSettingsMetricsExclusions {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmGlobalSettingsMetricsExclusions::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.addresses = crate::parser::scalar::get_str(&props, "addresses");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmLink` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_link(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmLink {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmLink::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.datacenter = crate::parser::scalar::get_str(&props, "datacenter");
    obj.monitor = crate::parser::scalar::get_str(&props, "monitor");
    obj.prober_pool = crate::parser::scalar::get_str(&props, "prober-pool");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.weight = crate::parser::scalar::get_str(&props, "weight");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmListener` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_listener(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmListener {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmListener::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.address = crate::parser::scalar::get_str(&props, "address");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.ip_protocol = crate::parser::scalar::get_str(&props, "ip-protocol");
    obj.mask = crate::parser::scalar::get_str(&props, "mask");
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.profiles = crate::parser::scalar::list_field(&props, "profiles");
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.source_address_translation =
        crate::parser::scalar::get_str(&props, "source-address-translation");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.vlans = crate::parser::scalar::list_field(&props, "vlans");
    obj.vlans_disabled = crate::parser::scalar::get_bool(&props, "vlans-disabled");
    obj.vlans_enabled = crate::parser::scalar::get_bool(&props, "vlans-enabled");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmListenerDohProxy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_listener_doh_proxy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmListenerDohProxy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmListenerDohProxy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.address = crate::parser::scalar::get_str(&props, "address");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmListenerDohServer` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_listener_doh_server(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmListenerDohServer {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmListenerDohServer::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.address = crate::parser::scalar::get_str(&props, "address");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmPool` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_pool(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmPool {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmPool::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.record_type = crate::parser::scalar::get_str(&props, "record-type");
    obj.monitor = crate::parser::scalar::get_str(&props, "monitor");
    obj.alternate_mode = crate::parser::scalar::get_str(&props, "alternate-mode");
    obj.fallback_mode = crate::parser::scalar::get_str(&props, "fallback-mode");
    obj.load_balancing_mode = crate::parser::scalar::get_str(&props, "load-balancing-mode");
    obj.ttl = crate::parser::scalar::get_str(&props, "ttl");
    obj.description = crate::parser::scalar::description(&props);
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.verify_member_availability =
        crate::parser::scalar::get_str(&props, "verify-member-availability");
    obj.fallback_ip = crate::parser::scalar::get_str(&props, "fallback-ip");
    obj.max_answers_returned = crate::parser::scalar::get_str(&props, "max-answers-returned");
    obj.qos_hit_ratio = crate::parser::scalar::get_str(&props, "qos-hit-ratio");
    obj.qos_hops = crate::parser::scalar::get_str(&props, "qos-hops");
    obj.qos_kbps = crate::parser::scalar::get_str(&props, "qos-kbps");
    obj.qos_lcs = crate::parser::scalar::get_str(&props, "qos-lcs");
    obj.qos_packet_rate = crate::parser::scalar::get_str(&props, "qos-packet-rate");
    obj.qos_rtt = crate::parser::scalar::get_str(&props, "qos-rtt");
    obj.qos_topology = crate::parser::scalar::get_str(&props, "qos-topology");
    obj.qos_vs_capacity = crate::parser::scalar::get_str(&props, "qos-vs-capacity");
    obj.qos_vs_score = crate::parser::scalar::get_str(&props, "qos-vs-score");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmPoolMember` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_pool_member(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmPoolMember {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmPoolMember::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.description = crate::parser::scalar::description(&props);
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.member_order = crate::parser::scalar::get_str(&props, "member-order");
    obj.order = crate::parser::scalar::get_str(&props, "order");
    obj.service_port = crate::parser::scalar::get_str(&props, "service-port");
    obj.ratio = crate::parser::scalar::get_str(&props, "ratio");
    obj.monitor = crate::parser::scalar::get_str(&props, "monitor");
    obj.depends_on = crate::parser::scalar::get_str(&props, "depends-on");
    obj.limit_max_bps = crate::parser::scalar::get_str(&props, "limit-max-bps");
    obj.limit_max_connections = crate::parser::scalar::get_str(&props, "limit-max-connections");
    obj.limit_max_pps = crate::parser::scalar::get_str(&props, "limit-max-pps");
    obj.static_target = crate::parser::scalar::get_str(&props, "static-target");
    obj
}

/// Scalar parser for `BigipGtmProberPool` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_prober_pool(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmProberPool {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmProberPool::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.load_balancing_mode = crate::parser::scalar::get_str(&props, "load-balancing-mode");
    obj.members = crate::parser::scalar::list_field(&props, "members");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmRegion` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_region(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmRegion {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmRegion::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.region_members = crate::parser::scalar::list_field(&props, "region-members");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmRule` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_rule(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmRule {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmRule::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.source = crate::parser::scalar::get_str(&props, "source");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmServer` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_server(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmServer {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmServer::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.datacenter = crate::parser::scalar::get_str(&props, "datacenter");
    obj.monitor = crate::parser::scalar::get_str(&props, "monitor");
    obj.product = crate::parser::scalar::get_str(&props, "product");
    obj.addresses = crate::parser::scalar::list_field(&props, "addresses");
    obj.virtual_servers = crate::parser::scalar::list_field(&props, "virtual-servers");
    obj.description = crate::parser::scalar::description(&props);
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.prober_pool = crate::parser::scalar::get_str(&props, "prober-pool");
    obj.prober_preference = crate::parser::scalar::get_str(&props, "prober-preference");
    obj.prober_fallback = crate::parser::scalar::get_str(&props, "prober-fallback");
    obj.virtual_server_discovery =
        crate::parser::scalar::get_str(&props, "virtual-server-discovery");
    obj.expose_route_domains = crate::parser::scalar::get_str(&props, "expose-route-domains");
    obj.iq_allow_path = crate::parser::scalar::get_str(&props, "iq-allow-path");
    obj.iq_allow_service_check = crate::parser::scalar::get_str(&props, "iq-allow-service-check");
    obj.iq_allow_snmp = crate::parser::scalar::get_str(&props, "iq-allow-snmp");
    obj.limit_max_bps = crate::parser::scalar::get_str(&props, "limit-max-bps");
    obj.limit_max_connections = crate::parser::scalar::get_str(&props, "limit-max-connections");
    obj.limit_max_pps = crate::parser::scalar::get_str(&props, "limit-max-pps");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmTopology` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_topology(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmTopology {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmTopology::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.order = crate::parser::scalar::get_str(&props, "order");
    obj.score = crate::parser::scalar::get_str(&props, "score");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipGtmWideip` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_gtm_wideip(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipGtmWideip {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipGtmWideip::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.record_type = crate::parser::scalar::get_str(&props, "record-type");
    obj.pools = crate::parser::scalar::list_field(&props, "pools");
    obj.aliases = crate::parser::scalar::list_field(&props, "aliases");
    obj.pool_lb_mode = crate::parser::scalar::get_str(&props, "pool-lb-mode");
    obj.last_resort_pool = crate::parser::scalar::get_str(&props, "last-resort-pool");
    obj.description = crate::parser::scalar::description(&props);
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.failure_rcode = crate::parser::scalar::get_str(&props, "failure-rcode");
    obj.failure_rcode_response = crate::parser::scalar::get_str(&props, "failure-rcode-response");
    obj.failure_rcode_ttl = crate::parser::scalar::get_str(&props, "failure-rcode-ttl");
    obj.minimal_response = crate::parser::scalar::get_str(&props, "minimal-response");
    obj.persistence = crate::parser::scalar::get_str(&props, "persistence");
    obj.persist_cidr_ipv4 = crate::parser::scalar::get_str(&props, "persist-cidr-ipv4");
    obj.persist_cidr_ipv6 = crate::parser::scalar::get_str(&props, "persist-cidr-ipv6");
    obj.topology_prefer_edns0_client_subnet =
        crate::parser::scalar::get_str(&props, "topology-prefer-edns0-client-subnet");
    obj.ttl_persistence = crate::parser::scalar::get_str(&props, "ttl-persistence");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipDataGroup` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_data_group(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipDataGroup {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipDataGroup::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.value_type = crate::parser::scalar::get_str(&props, "value-type");
    obj.records = crate::parser::scalar::list_field(&props, "records");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmAuthObject` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_auth_object(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmAuthObject {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmAuthObject::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.kind = crate::parser::scalar::get_str(&props, "kind");
    obj.description = crate::parser::scalar::description(&props);
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmCipherGroup` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_cipher_group(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmCipherGroup {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmCipherGroup::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.allow = crate::parser::scalar::list_field(&props, "allow");
    obj.require = crate::parser::scalar::list_field(&props, "require");
    obj.exclude = crate::parser::scalar::list_field(&props, "exclude");
    obj.ordering = crate::parser::scalar::get_str(&props, "ordering");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmCipherRule` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_cipher_rule(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmCipherRule {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmCipherRule::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.cipher = crate::parser::scalar::get_str(&props, "cipher");
    obj.dh_groups = crate::parser::scalar::get_str(&props, "dh-groups");
    obj.signature_algorithms = crate::parser::scalar::get_str(&props, "signature-algorithms");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsAnalyticsGlobalSettings` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_analytics_global_settings(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsAnalyticsGlobalSettings {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsAnalyticsGlobalSettings::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsCacheGlobalSettings` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_cache_global_settings(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsCacheGlobalSettings {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsCacheGlobalSettings::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.expiry_time = crate::parser::scalar::get_str(&props, "expiry-time");
    obj.nameserver_ttl = crate::parser::scalar::get_str(&props, "nameserver-ttl");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsCacheRecord` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_cache_record(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsCacheRecord {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsCacheRecord::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.record_kind = crate::parser::scalar::get_str(&props, "record-kind");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsCacheResolver` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_cache_resolver(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsCacheResolver {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsCacheResolver::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.message_cache_size = crate::parser::scalar::get_str(&props, "message-cache-size");
    obj.resolver_cache_size = crate::parser::scalar::get_str(&props, "resolver-cache-size");
    obj.answer_default_zones = crate::parser::scalar::get_str(&props, "answer-default-zones");
    obj.forward_zones = crate::parser::scalar::list_field(&props, "forward-zones");
    obj.route_domain = crate::parser::scalar::get_str(&props, "route-domain");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsCacheTransparent` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_cache_transparent(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsCacheTransparent {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsCacheTransparent::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.message_cache_size = crate::parser::scalar::get_str(&props, "message-cache-size");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsCacheValidatingResolver` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_cache_validating_resolver(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsCacheValidatingResolver {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsCacheValidatingResolver::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.message_cache_size = crate::parser::scalar::get_str(&props, "message-cache-size");
    obj.resolver_cache_size = crate::parser::scalar::get_str(&props, "resolver-cache-size");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsDnssecKey` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_dnssec_key(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsDnssecKey {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsDnssecKey::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.type_ = crate::parser::scalar::get_str(&props, "type");
    obj.algorithm = crate::parser::scalar::get_str(&props, "algorithm");
    obj.bit_width = crate::parser::scalar::get_str(&props, "bit-width");
    obj.rollover_period = crate::parser::scalar::get_str(&props, "rollover-period");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsDnssecZone` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_dnssec_zone(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsDnssecZone {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsDnssecZone::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.keys = crate::parser::scalar::list_field(&props, "keys");
    obj.enable = crate::parser::scalar::get_str(&props, "enable");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsHpkeKey` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_hpke_key(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsHpkeKey {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsHpkeKey::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.algorithm = crate::parser::scalar::get_str(&props, "algorithm");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsHpkeProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_hpke_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsHpkeProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsHpkeProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.keys = crate::parser::scalar::list_field(&props, "keys");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsNameserver` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_nameserver(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsNameserver {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsNameserver::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.address = crate::parser::scalar::get_str(&props, "address");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.tsig_key = crate::parser::scalar::get_str(&props, "tsig-key");
    obj.route_domain = crate::parser::scalar::get_str(&props, "route-domain");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsTsigKey` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_tsig_key(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsTsigKey {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsTsigKey::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.algorithm = crate::parser::scalar::get_str(&props, "algorithm");
    obj.secret = crate::parser::scalar::get_str(&props, "secret");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmDnsZone` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_dns_zone(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmDnsZone {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmDnsZone::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.dns_express_server = crate::parser::scalar::get_str(&props, "dns-express-server");
    obj.dns_express_allow_notify =
        crate::parser::scalar::list_field(&props, "dns-express-allow-notify");
    obj.dns_express_enabled = crate::parser::scalar::get_str(&props, "dns-express-enabled");
    obj.response_policy = crate::parser::scalar::get_str(&props, "response-policy");
    obj.transfer_clients = crate::parser::scalar::list_field(&props, "transfer-clients");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmEvictionPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_eviction_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmEvictionPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmEvictionPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.high_water_mark = crate::parser::scalar::get_str(&props, "high-water-mark");
    obj.low_water_mark = crate::parser::scalar::get_str(&props, "low-water-mark");
    obj.slow_flow_throttle = crate::parser::scalar::get_str(&props, "slow-flow-throttle");
    obj.slow_flow_monitoring = crate::parser::scalar::get_str(&props, "slow-flow-monitoring");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmIfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_ifile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmIfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmIfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.file_name = crate::parser::scalar::get_str(&props, "file-name");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmMessageRoutingObject` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_message_routing_object(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmMessageRoutingObject {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmMessageRoutingObject::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.kind = crate::parser::scalar::get_str(&props, "kind");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmNat` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_nat(full_path: &str, body: &str, range: crate::range::Range) -> BigipLtmNat {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmNat::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.translation_address = crate::parser::scalar::get_str(&props, "translation-address");
    obj.originating_address = crate::parser::scalar::get_str(&props, "originating-address");
    obj.traffic_group = crate::parser::scalar::get_str(&props, "traffic-group");
    obj.vlans = crate::parser::scalar::list_field(&props, "vlans");
    obj.vlans_disabled = crate::parser::scalar::get_bool(&props, "vlans-disabled");
    obj.vlans_enabled = crate::parser::scalar::get_bool(&props, "vlans-enabled");
    obj.mirror = crate::parser::scalar::get_str(&props, "mirror");
    obj.arp = crate::parser::scalar::get_str(&props, "arp");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmPolicyStrategy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_policy_strategy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmPolicyStrategy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmPolicyStrategy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.strategy = crate::parser::scalar::get_str(&props, "strategy");
    obj.operands = crate::parser::scalar::list_field(&props, "operands");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmRateClass` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_rate_class(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmRateClass {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmRateClass::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rate = crate::parser::scalar::get_str(&props, "rate");
    obj.ceiling = crate::parser::scalar::get_str(&props, "ceiling");
    obj.burst_size = crate::parser::scalar::get_str(&props, "burst-size");
    obj.direction = crate::parser::scalar::get_str(&props, "direction");
    obj.queue_management = crate::parser::scalar::get_str(&props, "queue-management");
    obj.parent = crate::parser::scalar::get_str(&props, "parent");
    obj.drop_policy = crate::parser::scalar::get_str(&props, "drop-policy");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmSnat` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_snat(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmSnat {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmSnat::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.origins = crate::parser::scalar::list_field(&props, "origins");
    obj.translation = crate::parser::scalar::get_str(&props, "translation");
    obj.snatpool = crate::parser::scalar::get_str(&props, "snatpool");
    obj.vlans = crate::parser::scalar::list_field(&props, "vlans");
    obj.vlans_disabled = crate::parser::scalar::get_bool(&props, "vlans-disabled");
    obj.vlans_enabled = crate::parser::scalar::get_bool(&props, "vlans-enabled");
    obj.automap = crate::parser::scalar::get_bool(&props, "automap");
    obj.mirror = crate::parser::scalar::get_str(&props, "mirror");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmSnatTranslation` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_snat_translation(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmSnatTranslation {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmSnatTranslation::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.address = crate::parser::scalar::get_str(&props, "address");
    obj.inherited_traffic_group = crate::parser::scalar::get_str(&props, "inherited-traffic-group");
    obj.traffic_group = crate::parser::scalar::get_str(&props, "traffic-group");
    obj.connection_limit = crate::parser::scalar::get_str(&props, "connection-limit");
    obj.ip_idle_timeout = crate::parser::scalar::get_str(&props, "ip-idle-timeout");
    obj.tcp_idle_timeout = crate::parser::scalar::get_str(&props, "tcp-idle-timeout");
    obj.udp_idle_timeout = crate::parser::scalar::get_str(&props, "udp-idle-timeout");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmTrafficClass` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_traffic_class(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmTrafficClass {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmTrafficClass::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.classification = crate::parser::scalar::get_str(&props, "classification");
    obj.match_method = crate::parser::scalar::get_str(&props, "match-method");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipLtmTrafficMatchingCriteria` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_ltm_traffic_matching_criteria(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipLtmTrafficMatchingCriteria {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipLtmTrafficMatchingCriteria::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.destination_address_list =
        crate::parser::scalar::get_str(&props, "destination-address-list");
    obj.destination_address_inline =
        crate::parser::scalar::get_str(&props, "destination-address-inline");
    obj.destination_port_list = crate::parser::scalar::get_str(&props, "destination-port-list");
    obj.destination_port_inline = crate::parser::scalar::get_str(&props, "destination-port-inline");
    obj.source_address_list = crate::parser::scalar::get_str(&props, "source-address-list");
    obj.source_address_inline = crate::parser::scalar::get_str(&props, "source-address-inline");
    obj.source_port_list = crate::parser::scalar::get_str(&props, "source-port-list");
    obj.source_port_inline = crate::parser::scalar::get_str(&props, "source-port-inline");
    obj.protocol = crate::parser::scalar::get_str(&props, "protocol");
    obj.route_domain = crate::parser::scalar::get_str(&props, "route-domain");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipMonitor` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_monitor(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipMonitor {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipMonitor::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.monitor_type = crate::parser::scalar::get_str(&props, "monitor-type");
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.description = crate::parser::scalar::description(&props);
    obj.interval = crate::parser::scalar::get_str(&props, "interval");
    obj.timeout = crate::parser::scalar::get_str(&props, "timeout");
    obj.destination = crate::parser::scalar::get_str(&props, "destination");
    obj.send = crate::parser::scalar::get_str(&props, "send");
    obj.recv = crate::parser::scalar::get_str(&props, "recv");
    obj.recv_disable = crate::parser::scalar::get_str(&props, "recv-disable");
    obj.username = crate::parser::scalar::get_str(&props, "username");
    obj.password = crate::parser::scalar::get_str(&props, "password");
    obj.base = crate::parser::scalar::get_str(&props, "base");
    obj.filter = crate::parser::scalar::get_str(&props, "filter");
    obj.count = crate::parser::scalar::get_str(&props, "count");
    obj.database = crate::parser::scalar::get_str(&props, "database");
    obj.args = crate::parser::scalar::get_str(&props, "args");
    obj.run = crate::parser::scalar::get_str(&props, "run");
    obj.adaptive = crate::parser::scalar::get_str(&props, "adaptive");
    obj.adaptive_divergence_type =
        crate::parser::scalar::get_str(&props, "adaptive-divergence-type");
    obj.adaptive_divergence_value =
        crate::parser::scalar::get_str(&props, "adaptive-divergence-value");
    obj.adaptive_limit = crate::parser::scalar::get_str(&props, "adaptive-limit");
    obj.adaptive_sampling_timespan =
        crate::parser::scalar::get_str(&props, "adaptive-sampling-timespan");
    obj.transparent = crate::parser::scalar::get_str(&props, "transparent");
    obj.reverse = crate::parser::scalar::get_str(&props, "reverse");
    obj.manual_resume = crate::parser::scalar::get_str(&props, "manual-resume");
    obj.ignore_down_response = crate::parser::scalar::get_str(&props, "ignore-down-response");
    obj.ip_dscp = crate::parser::scalar::get_str(&props, "ip-dscp");
    obj.up_interval = crate::parser::scalar::get_str(&props, "up-interval");
    obj.time_until_up = crate::parser::scalar::get_str(&props, "time-until-up");
    obj.cipherlist = crate::parser::scalar::get_str(&props, "cipherlist");
    obj.cert = crate::parser::scalar::get_str(&props, "cert");
    obj.key = crate::parser::scalar::get_str(&props, "key");
    obj.compatibility = crate::parser::scalar::get_str(&props, "compatibility");
    obj.community = crate::parser::scalar::get_str(&props, "community");
    obj.version = crate::parser::scalar::get_str(&props, "version");
    obj.agent_type = crate::parser::scalar::get_str(&props, "agent-type");
    obj.cpu_coefficient = crate::parser::scalar::get_str(&props, "cpu-coefficient");
    obj.cpu_threshold = crate::parser::scalar::get_str(&props, "cpu-threshold");
    obj.disk_coefficient = crate::parser::scalar::get_str(&props, "disk-coefficient");
    obj.disk_threshold = crate::parser::scalar::get_str(&props, "disk-threshold");
    obj.memory_coefficient = crate::parser::scalar::get_str(&props, "memory-coefficient");
    obj.memory_threshold = crate::parser::scalar::get_str(&props, "memory-threshold");
    obj.headers = crate::parser::scalar::get_str(&props, "headers");
    obj.request = crate::parser::scalar::get_str(&props, "request");
    obj.response = crate::parser::scalar::get_str(&props, "response");
    obj.mode = crate::parser::scalar::get_str(&props, "mode");
    obj.alias_address = crate::parser::scalar::get_str(&props, "alias-address");
    obj.alias_service_port = crate::parser::scalar::get_str(&props, "alias-service-port");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNode` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_node(full_path: &str, body: &str, range: crate::range::Range) -> BigipNode {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNode::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.monitor = crate::parser::scalar::get_str(&props, "monitor");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.connection_limit = crate::parser::scalar::get_str(&props, "connection-limit");
    obj.rate_limit = crate::parser::scalar::get_str(&props, "rate-limit");
    obj.ratio = crate::parser::scalar::get_str(&props, "ratio");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPersistence` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_persistence(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPersistence {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPersistence::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.persistence_type = crate::parser::scalar::get_str(&props, "persistence-type");
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.description = crate::parser::scalar::description(&props);
    obj.timeout = crate::parser::scalar::get_str(&props, "timeout");
    obj.match_across_pools = crate::parser::scalar::get_str(&props, "match-across-pools");
    obj.match_across_services = crate::parser::scalar::get_str(&props, "match-across-services");
    obj.match_across_virtuals = crate::parser::scalar::get_str(&props, "match-across-virtuals");
    obj.mirror = crate::parser::scalar::get_str(&props, "mirror");
    obj.override_connection_limit =
        crate::parser::scalar::get_str(&props, "override-connection-limit");
    obj.always_send = crate::parser::scalar::get_str(&props, "always-send");
    obj.cookie_name = crate::parser::scalar::get_str(&props, "cookie-name");
    obj.cookie_encryption = crate::parser::scalar::get_str(&props, "cookie-encryption");
    obj.cookie_encryption_passphrase =
        crate::parser::scalar::get_str(&props, "cookie-encryption-passphrase");
    obj.httponly = crate::parser::scalar::get_str(&props, "httponly");
    obj.secure = crate::parser::scalar::get_str(&props, "secure");
    obj.expiration = crate::parser::scalar::get_str(&props, "expiration");
    obj.method = crate::parser::scalar::get_str(&props, "method");
    obj.hash_length = crate::parser::scalar::get_str(&props, "hash-length");
    obj.hash_offset = crate::parser::scalar::get_str(&props, "hash-offset");
    obj.mcp_encryption_passphrase =
        crate::parser::scalar::get_str(&props, "mcp-encryption-passphrase");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_policy(full_path: &str, body: &str, range: crate::range::Range) -> BigipPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.strategy = crate::parser::scalar::get_str(&props, "strategy");
    obj.requires = crate::parser::scalar::list_field(&props, "requires");
    obj.controls = crate::parser::scalar::list_field(&props, "controls");
    obj.description = crate::parser::scalar::description(&props);
    obj.status = crate::parser::scalar::get_str(&props, "status");
    obj.last_modified = crate::parser::scalar::get_str(&props, "last-modified");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPolicyAction` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_policy_action(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPolicyAction {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPolicyAction::default();
    let _ = &props;
    obj.index = crate::parser::scalar::get_int(&props, "index");
    obj.target = crate::parser::scalar::get_str(&props, "target");
    obj.verb = crate::parser::scalar::get_str(&props, "verb");
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.location = crate::parser::scalar::get_str(&props, "location");
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.value = crate::parser::scalar::get_str(&props, "value");
    obj.path = crate::parser::scalar::get_str(&props, "path");
    obj.query = crate::parser::scalar::get_str(&props, "query");
    obj.host = crate::parser::scalar::get_str(&props, "host");
    obj.event = crate::parser::scalar::get_str(&props, "event");
    obj
}

/// Scalar parser for `BigipPolicyCondition` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_policy_condition(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPolicyCondition {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPolicyCondition::default();
    let _ = &props;
    obj.index = crate::parser::scalar::get_int(&props, "index");
    obj.operand = crate::parser::scalar::get_str(&props, "operand");
    obj.selector = crate::parser::scalar::get_str(&props, "selector");
    obj.operator = crate::parser::scalar::get_str(&props, "operator");
    obj.values = crate::parser::scalar::list_field(&props, "values");
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.negate = crate::parser::scalar::get_bool(&props, "negate");
    obj.case_insensitive = crate::parser::scalar::get_bool(&props, "case-insensitive");
    obj.event = crate::parser::scalar::get_str(&props, "event");
    obj
}

/// Scalar parser for `BigipPolicyRule` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_policy_rule(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPolicyRule {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPolicyRule::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.ordinal = crate::parser::scalar::get_int(&props, "ordinal");
    obj
}

/// Scalar parser for `BigipPool` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pool(full_path: &str, body: &str, range: crate::range::Range) -> BigipPool {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPool::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.monitor = crate::parser::scalar::get_str(&props, "monitor");
    obj.load_balancing_mode = crate::parser::scalar::get_str(&props, "load-balancing-mode");
    obj.description = crate::parser::scalar::description(&props);
    obj.min_active_members = crate::parser::scalar::get_str(&props, "min-active-members");
    obj.min_up_members = crate::parser::scalar::get_str(&props, "min-up-members");
    obj.service_down_action = crate::parser::scalar::get_str(&props, "service-down-action");
    obj.slow_ramp_time = crate::parser::scalar::get_str(&props, "slow-ramp-time");
    obj.allow_snat = crate::parser::scalar::get_str(&props, "allow-snat");
    obj.allow_nat = crate::parser::scalar::get_str(&props, "allow-nat");
    obj.reselect_tries = crate::parser::scalar::get_str(&props, "reselect-tries");
    obj.queue_depth_limit = crate::parser::scalar::get_str(&props, "queue-depth-limit");
    obj.queue_time_limit = crate::parser::scalar::get_str(&props, "queue-time-limit");
    obj.connection_limit = crate::parser::scalar::get_str(&props, "connection-limit");
    obj.rate_limit = crate::parser::scalar::get_str(&props, "rate-limit");
    obj.ratio = crate::parser::scalar::get_str(&props, "ratio");
    obj.down_interval = crate::parser::scalar::get_str(&props, "down-interval");
    obj.interval = crate::parser::scalar::get_str(&props, "interval");
    obj.min_up_members_action = crate::parser::scalar::get_str(&props, "min-up-members-action");
    obj.min_up_members_checking = crate::parser::scalar::get_str(&props, "min-up-members-checking");
    obj.ip_tos_to_client = crate::parser::scalar::get_str(&props, "ip-tos-to-client");
    obj.ip_tos_to_server = crate::parser::scalar::get_str(&props, "ip-tos-to-server");
    obj.link_qos_to_client = crate::parser::scalar::get_str(&props, "link-qos-to-client");
    obj.link_qos_to_server = crate::parser::scalar::get_str(&props, "link-qos-to-server");
    obj.gateway_failsafe_device = crate::parser::scalar::get_str(&props, "gateway-failsafe-device");
    obj.ignore_persisted_weight = crate::parser::scalar::get_str(&props, "ignore-persisted-weight");
    obj.inherit_profile = crate::parser::scalar::get_str(&props, "inherit-profile");
    obj.queue_on_connection_limit =
        crate::parser::scalar::get_str(&props, "queue-on-connection-limit");
    obj.address_family = crate::parser::scalar::get_str(&props, "address-family");
    obj.autopopulate = crate::parser::scalar::get_str(&props, "autopopulate");
    obj.profiles = crate::parser::scalar::list_field(&props, "profiles");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPoolMember` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pool_member(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPoolMember {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPoolMember::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.port = crate::parser::scalar::get_int(&props, "port");
    obj.monitor = crate::parser::scalar::get_str(&props, "monitor");
    obj.description = crate::parser::scalar::description(&props);
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.ratio = crate::parser::scalar::get_str(&props, "ratio");
    obj.priority_group = crate::parser::scalar::get_str(&props, "priority-group");
    obj.connection_limit = crate::parser::scalar::get_str(&props, "connection-limit");
    obj.rate_limit = crate::parser::scalar::get_str(&props, "rate-limit");
    obj
}

/// Scalar parser for `BigipProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.description = crate::parser::scalar::description(&props);
    obj.idle_timeout = crate::parser::scalar::get_str(&props, "idle-timeout");
    obj.insert_xforwarded_for = crate::parser::scalar::get_str(&props, "insert-xforwarded-for");
    obj.request_chunking = crate::parser::scalar::get_str(&props, "request-chunking");
    obj.response_chunking = crate::parser::scalar::get_str(&props, "response-chunking");
    obj.lws_max_columns = crate::parser::scalar::get_str(&props, "lws-max-columns");
    obj.lws_separator = crate::parser::scalar::get_str(&props, "lws-separator");
    obj.server_agent_name = crate::parser::scalar::get_str(&props, "server-agent-name");
    obj.via_request = crate::parser::scalar::get_str(&props, "via-request");
    obj.via_response = crate::parser::scalar::get_str(&props, "via-response");
    obj.ciphers = crate::parser::scalar::get_str(&props, "ciphers");
    obj.cipher_group = crate::parser::scalar::get_str(&props, "cipher-group");
    obj.cert = crate::parser::scalar::get_str(&props, "cert");
    obj.key = crate::parser::scalar::get_str(&props, "key");
    obj.chain = crate::parser::scalar::get_str(&props, "chain");
    obj.ca_file = crate::parser::scalar::get_str(&props, "ca-file");
    obj.crl_file = crate::parser::scalar::get_str(&props, "crl-file");
    obj.cert_extension_includes = crate::parser::scalar::get_str(&props, "cert-extension-includes");
    obj.options = crate::parser::scalar::get_str(&props, "options");
    obj.peer_cert_mode = crate::parser::scalar::get_str(&props, "peer-cert-mode");
    obj.sni_default = crate::parser::scalar::get_str(&props, "sni-default");
    obj.sni_require = crate::parser::scalar::get_str(&props, "sni-require");
    obj.server_name = crate::parser::scalar::get_str(&props, "server-name");
    obj.renegotiation = crate::parser::scalar::get_str(&props, "renegotiation");
    obj.secure_renegotiation = crate::parser::scalar::get_str(&props, "secure-renegotiation");
    obj.proxy_ca_cert = crate::parser::scalar::get_str(&props, "proxy-ca-cert");
    obj.proxy_ca_key = crate::parser::scalar::get_str(&props, "proxy-ca-key");
    obj.keep_alive_interval = crate::parser::scalar::get_str(&props, "keep-alive-interval");
    obj.ip_tos_to_client = crate::parser::scalar::get_str(&props, "ip-tos-to-client");
    obj.ip_tos_to_server = crate::parser::scalar::get_str(&props, "ip-tos-to-server");
    obj.link_qos_to_client = crate::parser::scalar::get_str(&props, "link-qos-to-client");
    obj.link_qos_to_server = crate::parser::scalar::get_str(&props, "link-qos-to-server");
    obj.nagle = crate::parser::scalar::get_str(&props, "nagle");
    obj.reset_on_timeout = crate::parser::scalar::get_str(&props, "reset-on-timeout");
    obj.send_buffer_size = crate::parser::scalar::get_str(&props, "send-buffer-size");
    obj.receive_window_size = crate::parser::scalar::get_str(&props, "receive-window-size");
    obj.proxy_buffer_low = crate::parser::scalar::get_str(&props, "proxy-buffer-low");
    obj.proxy_buffer_high = crate::parser::scalar::get_str(&props, "proxy-buffer-high");
    obj.pva_acceleration = crate::parser::scalar::get_str(&props, "pva-acceleration");
    obj.pva_dynamic_client_packets =
        crate::parser::scalar::get_str(&props, "pva-dynamic-client-packets");
    obj.pva_dynamic_server_packets =
        crate::parser::scalar::get_str(&props, "pva-dynamic-server-packets");
    obj.loose_close = crate::parser::scalar::get_str(&props, "loose-close");
    obj.loose_initialization = crate::parser::scalar::get_str(&props, "loose-initialization");
    obj.datagram_load_balancing = crate::parser::scalar::get_str(&props, "datagram-load-balancing");
    obj.allow_no_payload = crate::parser::scalar::get_str(&props, "allow-no-payload");
    obj.source_mask = crate::parser::scalar::get_str(&props, "source-mask");
    obj.idle_timeout_override = crate::parser::scalar::get_str(&props, "idle-timeout-override");
    obj.max_age = crate::parser::scalar::get_str(&props, "max-age");
    obj.max_reuse = crate::parser::scalar::get_str(&props, "max-reuse");
    obj.max_size = crate::parser::scalar::get_str(&props, "max-size");
    obj.source = crate::parser::scalar::get_str(&props, "source");
    obj.target = crate::parser::scalar::get_str(&props, "target");
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.collected_stats_internal_logging =
        crate::parser::scalar::get_str(&props, "collected-stats-internal-logging");
    obj.collected_stats_external_logging =
        crate::parser::scalar::get_str(&props, "collected-stats-external-logging");
    obj.publisher = crate::parser::scalar::get_str(&props, "publisher");
    obj.maximum_bytes = crate::parser::scalar::get_str(&props, "maximum-bytes");
    obj.maximum_entries = crate::parser::scalar::get_str(&props, "maximum-entries");
    obj.maximum_non_json_bytes = crate::parser::scalar::get_str(&props, "maximum-non-json-bytes");
    obj.max_buffered_msg_bytes = crate::parser::scalar::get_str(&props, "max-buffered-msg-bytes");
    obj.max_field_name_size = crate::parser::scalar::get_str(&props, "max-field-name-size");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipRule` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_rule(full_path: &str, body: &str, range: crate::range::Range) -> BigipRule {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipRule::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.source = crate::parser::scalar::get_str(&props, "source");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSnatPool` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_snat_pool(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSnatPool {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSnatPool::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipVirtualAddress` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_virtual_address(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipVirtualAddress {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipVirtualAddress::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.mask = crate::parser::scalar::get_str(&props, "mask");
    obj.arp = crate::parser::scalar::get_str(&props, "arp");
    obj.icmp_echo = crate::parser::scalar::get_str(&props, "icmp-echo");
    obj.auto_delete = crate::parser::scalar::get_str(&props, "auto-delete");
    obj.connection_limit = crate::parser::scalar::get_str(&props, "connection-limit");
    obj.traffic_group = crate::parser::scalar::get_str(&props, "traffic-group");
    obj.inherited_traffic_group = crate::parser::scalar::get_str(&props, "inherited-traffic-group");
    obj.route_advertisement = crate::parser::scalar::get_str(&props, "route-advertisement");
    obj.server_scope = crate::parser::scalar::get_str(&props, "server-scope");
    obj.spanning = crate::parser::scalar::get_str(&props, "spanning");
    obj.unit = crate::parser::scalar::get_str(&props, "unit");
    obj.description = crate::parser::scalar::description(&props);
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.floating = crate::parser::scalar::get_str(&props, "floating");
    obj.traffic_group_restored = crate::parser::scalar::get_str(&props, "traffic-group-restored");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipVirtualServer` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_virtual_server(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipVirtualServer {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipVirtualServer::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.snatpool = crate::parser::scalar::get_str(&props, "snatpool");
    obj.source_address_translation =
        crate::parser::scalar::get_str(&props, "source-address-translation");
    obj.description = crate::parser::scalar::description(&props);
    obj.mask = crate::parser::scalar::get_str(&props, "mask");
    obj.source = crate::parser::scalar::get_str(&props, "source");
    obj.ip_protocol = crate::parser::scalar::get_str(&props, "ip-protocol");
    obj.connection_limit = crate::parser::scalar::get_str(&props, "connection-limit");
    obj.rate_limit = crate::parser::scalar::get_str(&props, "rate-limit");
    obj.rate_limit_mode = crate::parser::scalar::get_str(&props, "rate-limit-mode");
    obj.rate_limit_dst_mask = crate::parser::scalar::get_str(&props, "rate-limit-dst-mask");
    obj.rate_limit_src_mask = crate::parser::scalar::get_str(&props, "rate-limit-src-mask");
    obj.auto_lasthop = crate::parser::scalar::get_str(&props, "auto-lasthop");
    obj.translate_address = crate::parser::scalar::get_str(&props, "translate-address");
    obj.translate_port = crate::parser::scalar::get_str(&props, "translate-port");
    obj.state = crate::parser::scalar::state_flag(&props);
    obj.address_status = crate::parser::scalar::get_str(&props, "address-status");
    obj.auto_discovery = crate::parser::scalar::get_str(&props, "auto-discovery");
    obj.cmp_enabled = crate::parser::scalar::get_str(&props, "cmp-enabled");
    obj.eviction_protected = crate::parser::scalar::get_str(&props, "eviction-protected");
    obj.dhcp_relay = crate::parser::scalar::get_bool(&props, "dhcp-relay");
    obj.internal = crate::parser::scalar::get_bool(&props, "internal");
    obj.ip_forward = crate::parser::scalar::get_bool(&props, "ip-forward");
    obj.l2_forward = crate::parser::scalar::get_bool(&props, "l2-forward");
    obj.reject = crate::parser::scalar::get_bool(&props, "reject");
    obj.nat64 = crate::parser::scalar::get_str(&props, "nat64");
    obj.gtm_score = crate::parser::scalar::get_str(&props, "gtm-score");
    obj.mirror = crate::parser::scalar::get_str(&props, "mirror");
    obj.service_down_immediate_action =
        crate::parser::scalar::get_str(&props, "service-down-immediate-action");
    obj.source_port = crate::parser::scalar::get_str(&props, "source-port");
    obj.serverssl_use_sni = crate::parser::scalar::get_str(&props, "serverssl-use-sni");
    obj.rate_class = crate::parser::scalar::get_str(&props, "rate-class");
    obj.per_flow_request_access_policy =
        crate::parser::scalar::get_str(&props, "per-flow-request-access-policy");
    obj.transparent_nexthop = crate::parser::scalar::get_str(&props, "transparent-nexthop");
    obj.vlans_disabled = crate::parser::scalar::get_bool(&props, "vlans-disabled");
    obj.vlans_enabled = crate::parser::scalar::get_bool(&props, "vlans-enabled");
    obj.fallback_persistence = crate::parser::scalar::get_str(&props, "fallback-persistence");
    obj.last_hop_pool = crate::parser::scalar::get_str(&props, "last-hop-pool");
    obj.fw_enforced_policy = crate::parser::scalar::get_str(&props, "fw-enforced-policy");
    obj.fw_staged_policy = crate::parser::scalar::get_str(&props, "fw-staged-policy");
    obj.flow_eviction_policy = crate::parser::scalar::get_str(&props, "flow-eviction-policy");
    obj.service_policy = crate::parser::scalar::get_str(&props, "service-policy");
    obj.auth_profiles = crate::parser::scalar::list_field(&props, "auth-profiles");
    obj.traffic_classes = crate::parser::scalar::list_field(&props, "traffic-classes");
    obj.clone_pools = crate::parser::scalar::list_field(&props, "clone-pools");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetDnsResolver` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_dns_resolver(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetDnsResolver {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetDnsResolver::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.route_domain = crate::parser::scalar::get_str(&props, "route-domain");
    obj.forward_zones = crate::parser::scalar::list_field(&props, "forward-zones");
    obj.description = crate::parser::scalar::description(&props);
    obj.cache_size = crate::parser::scalar::get_str(&props, "cache-size");
    obj.randomize_query_name_case =
        crate::parser::scalar::get_str(&props, "randomize-query-name-case");
    obj.use_ipv4 = crate::parser::scalar::get_str(&props, "use-ipv4");
    obj.use_ipv6 = crate::parser::scalar::get_str(&props, "use-ipv6");
    obj.use_tcp = crate::parser::scalar::get_str(&props, "use-tcp");
    obj.use_udp = crate::parser::scalar::get_str(&props, "use-udp");
    obj.nameservers = crate::parser::scalar::list_field(&props, "nameservers");
    obj.answer_default_zones = crate::parser::scalar::get_str(&props, "answer-default-zones");
    obj.prefetch = crate::parser::scalar::get_str(&props, "prefetch");
    obj.nameserver_min_rtt = crate::parser::scalar::get_str(&props, "nameserver-min-rtt");
    obj.nameserver_ttl = crate::parser::scalar::get_str(&props, "nameserver-ttl");
    obj.outbound_msg_retry = crate::parser::scalar::get_str(&props, "outbound-msg-retry");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetInterface` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_interface(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetInterface {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetInterface::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.media_fixed = crate::parser::scalar::get_str(&props, "media-fixed");
    obj.description = crate::parser::scalar::description(&props);
    obj.enabled = crate::parser::scalar::get_bool(&props, "enabled");
    obj.disabled = crate::parser::scalar::get_bool(&props, "disabled");
    obj.bundle = crate::parser::scalar::get_str(&props, "bundle");
    obj.bundle_speed = crate::parser::scalar::get_str(&props, "bundle-speed");
    obj.lldp_admin = crate::parser::scalar::get_str(&props, "lldp-admin");
    obj.mtu = crate::parser::scalar::get_str(&props, "mtu");
    obj.flow_control = crate::parser::scalar::get_str(&props, "flow-control");
    obj.mac_address = crate::parser::scalar::get_str(&props, "mac-address");
    obj.media_active = crate::parser::scalar::get_str(&props, "media-active");
    obj.media_max = crate::parser::scalar::get_str(&props, "media-max");
    obj.media_sfp = crate::parser::scalar::get_str(&props, "media-sfp");
    obj.port_fwd_mode = crate::parser::scalar::get_str(&props, "port-fwd-mode");
    obj.qinq_ethertype = crate::parser::scalar::get_str(&props, "qinq-ethertype");
    obj.stp = crate::parser::scalar::get_str(&props, "stp");
    obj.stp_edge_port = crate::parser::scalar::get_str(&props, "stp-edge-port");
    obj.stp_link_type = crate::parser::scalar::get_str(&props, "stp-link-type");
    obj.stp_auto_edge_port = crate::parser::scalar::get_str(&props, "stp-auto-edge-port");
    obj.stp_reset = crate::parser::scalar::get_str(&props, "stp-reset");
    obj.sflow_poll_interval = crate::parser::scalar::get_str(&props, "sflow-poll-interval");
    obj.sflow_poll_interval_global =
        crate::parser::scalar::get_str(&props, "sflow-poll-interval-global");
    obj.vendor = crate::parser::scalar::get_str(&props, "vendor");
    obj.vendor_oui = crate::parser::scalar::get_str(&props, "vendor-oui");
    obj.vendor_partnum = crate::parser::scalar::get_str(&props, "vendor-partnum");
    obj.vendor_revision = crate::parser::scalar::get_str(&props, "vendor-revision");
    obj.virtual_wire = crate::parser::scalar::get_str(&props, "virtual-wire");
    obj.transmitter_technology = crate::parser::scalar::get_str(&props, "transmitter-technology");
    obj.lacp_port_priority = crate::parser::scalar::get_str(&props, "lacp-port-priority");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetPortList` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_port_list(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetPortList {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetPortList::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.ports = crate::parser::scalar::list_field(&props, "ports");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetRoute` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_route(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetRoute {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetRoute::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.is_default_route = crate::parser::scalar::get_bool(&props, "is-default-route");
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.description = crate::parser::scalar::description(&props);
    obj.mtu = crate::parser::scalar::get_str(&props, "mtu");
    obj.blackhole = crate::parser::scalar::get_bool(&props, "blackhole");
    obj.interface = crate::parser::scalar::get_str(&props, "interface");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetRouteDomain` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_route_domain(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetRouteDomain {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetRouteDomain::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.id = crate::parser::scalar::get_int(&props, "id");
    obj.vlans = crate::parser::scalar::list_field(&props, "vlans");
    obj.description = crate::parser::scalar::description(&props);
    obj.parent = crate::parser::scalar::get_str(&props, "parent");
    obj.strict = crate::parser::scalar::get_str(&props, "strict");
    obj.fw_enforced_policy = crate::parser::scalar::get_str(&props, "fw-enforced-policy");
    obj.fw_staged_policy = crate::parser::scalar::get_str(&props, "fw-staged-policy");
    obj.bwc_policy = crate::parser::scalar::get_str(&props, "bwc-policy");
    obj.connection_limit = crate::parser::scalar::get_str(&props, "connection-limit");
    obj.flow_eviction_policy = crate::parser::scalar::get_str(&props, "flow-eviction-policy");
    obj.routing_protocol = crate::parser::scalar::list_field(&props, "routing-protocol");
    obj.security_nat_policy = crate::parser::scalar::get_str(&props, "security-nat-policy");
    obj.service_policy = crate::parser::scalar::get_str(&props, "service-policy");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetSelf` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_self(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetSelf {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetSelf::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.vlan = crate::parser::scalar::get_str(&props, "vlan");
    obj.traffic_group = crate::parser::scalar::get_str(&props, "traffic-group");
    obj.allow_service = crate::parser::scalar::list_field(&props, "allow-service");
    obj.description = crate::parser::scalar::description(&props);
    obj.floating = crate::parser::scalar::get_str(&props, "floating");
    obj.unit = crate::parser::scalar::get_str(&props, "unit");
    obj.service_policy = crate::parser::scalar::get_str(&props, "service-policy");
    obj.fw_enforced_policy = crate::parser::scalar::get_str(&props, "fw-enforced-policy");
    obj.fw_staged_policy = crate::parser::scalar::get_str(&props, "fw-staged-policy");
    obj.inherited_traffic_group = crate::parser::scalar::get_str(&props, "inherited-traffic-group");
    obj.address_source = crate::parser::scalar::get_str(&props, "address-source");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetStp` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_stp(full_path: &str, body: &str, range: crate::range::Range) -> BigipNetStp {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetStp::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.interfaces = crate::parser::scalar::list_field(&props, "interfaces");
    obj.description = crate::parser::scalar::description(&props);
    obj.mode = crate::parser::scalar::get_str(&props, "mode");
    obj.priority = crate::parser::scalar::get_str(&props, "priority");
    obj.external_path_cost = crate::parser::scalar::get_str(&props, "external-path-cost");
    obj.internal_path_cost = crate::parser::scalar::get_str(&props, "internal-path-cost");
    obj.vlans = crate::parser::scalar::list_field(&props, "vlans");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetTunnel` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_tunnel(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetTunnel {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetTunnel::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.profile = crate::parser::scalar::get_str(&props, "profile");
    obj.local_address = crate::parser::scalar::get_str(&props, "local-address");
    obj.remote_address = crate::parser::scalar::get_str(&props, "remote-address");
    obj.description = crate::parser::scalar::description(&props);
    obj.mtu = crate::parser::scalar::get_str(&props, "mtu");
    obj.mode = crate::parser::scalar::get_str(&props, "mode");
    obj.idle_timeout = crate::parser::scalar::get_str(&props, "idle-timeout");
    obj.auto_lasthop = crate::parser::scalar::get_str(&props, "auto-lasthop");
    obj.secondary_address = crate::parser::scalar::get_str(&props, "secondary-address");
    obj.traffic_group = crate::parser::scalar::get_str(&props, "traffic-group");
    obj.transparent = crate::parser::scalar::get_str(&props, "transparent");
    obj.key = crate::parser::scalar::get_str(&props, "key");
    obj.use_pmtu = crate::parser::scalar::get_str(&props, "use-pmtu");
    obj.tos = crate::parser::scalar::get_str(&props, "tos");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipNetVlan` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_net_vlan(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipNetVlan {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipNetVlan::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.tag = crate::parser::scalar::get_int(&props, "tag");
    obj.interfaces = crate::parser::scalar::list_field(&props, "interfaces");
    obj.description = crate::parser::scalar::description(&props);
    obj.mtu = crate::parser::scalar::get_str(&props, "mtu");
    obj.cmp_hash = crate::parser::scalar::get_str(&props, "cmp-hash");
    obj.failsafe = crate::parser::scalar::get_str(&props, "failsafe");
    obj.failsafe_action = crate::parser::scalar::get_str(&props, "failsafe-action");
    obj.failsafe_timeout = crate::parser::scalar::get_str(&props, "failsafe-timeout");
    obj.fwd_mode = crate::parser::scalar::get_str(&props, "fwd-mode");
    obj.hardware_syncookie = crate::parser::scalar::get_str(&props, "hardware-syncookie");
    obj.learning = crate::parser::scalar::get_str(&props, "learning");
    obj.tag_mode = crate::parser::scalar::get_str(&props, "tag-mode");
    obj.virtual_wire = crate::parser::scalar::get_str(&props, "virtual-wire");
    obj.auto_lasthop = crate::parser::scalar::get_str(&props, "auto-lasthop");
    obj.source_check = crate::parser::scalar::get_str(&props, "source-check");
    obj.source_checking = crate::parser::scalar::get_str(&props, "source-checking");
    obj.syn_flood_rate_limit = crate::parser::scalar::get_str(&props, "syn-flood-rate-limit");
    obj.syncache_threshold = crate::parser::scalar::get_str(&props, "syncache-threshold");
    obj.service_policy = crate::parser::scalar::get_str(&props, "service-policy");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemForwardingEndpoint` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_forwarding_endpoint(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemForwardingEndpoint {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemForwardingEndpoint::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.snat_pool = crate::parser::scalar::get_str(&props, "snat-pool");
    obj.source_ip = crate::parser::scalar::get_str(&props, "source-ip");
    obj.destination_ip = crate::parser::scalar::get_str(&props, "destination-ip");
    obj.type_ = crate::parser::scalar::get_str(&props, "type");
    obj.persistence = crate::parser::scalar::get_str(&props, "persistence");
    obj.translate_address = crate::parser::scalar::get_str(&props, "translate-address");
    obj.translate_service = crate::parser::scalar::get_str(&props, "translate-service");
    obj.fallback = crate::parser::scalar::get_str(&props, "fallback");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemInterceptionEndpoint` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_interception_endpoint(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemInterceptionEndpoint {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemInterceptionEndpoint::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.pool = crate::parser::scalar::get_str(&props, "pool");
    obj.persistence = crate::parser::scalar::get_str(&props, "persistence");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemListener` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_listener(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemListener {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemListener::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.profile_spm = crate::parser::scalar::get_str(&props, "profile-spm");
    obj.profile_subscriber_mgmt = crate::parser::scalar::get_str(&props, "profile-subscriber-mgmt");
    obj.virtual_servers = crate::parser::scalar::list_field(&props, "virtual-servers");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.profile_type = crate::parser::scalar::get_str(&props, "profile-type");
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemRatingGroup` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_rating_group(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemRatingGroup {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemRatingGroup::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rating_group_id = crate::parser::scalar::get_str(&props, "rating-group-id");
    obj.default_quota = crate::parser::scalar::get_str(&props, "default-quota");
    obj.default_quota_holding_time =
        crate::parser::scalar::get_str(&props, "default-quota-holding-time");
    obj.default_validity_time = crate::parser::scalar::get_str(&props, "default-validity-time");
    obj.default_threshold = crate::parser::scalar::get_str(&props, "default-threshold");
    obj.total_octets = crate::parser::scalar::get_str(&props, "total-octets");
    obj.input_octets = crate::parser::scalar::get_str(&props, "input-octets");
    obj.output_octets = crate::parser::scalar::get_str(&props, "output-octets");
    obj.time = crate::parser::scalar::get_str(&props, "time");
    obj.consumption_time = crate::parser::scalar::get_str(&props, "consumption-time");
    obj.usage_time = crate::parser::scalar::get_str(&props, "usage-time");
    obj.volume = crate::parser::scalar::get_str(&props, "volume");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemRule` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_rule(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemRule {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemRule::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.source = crate::parser::scalar::get_str(&props, "source");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipPemServiceChainEndpoint` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_pem_service_chain_endpoint(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipPemServiceChainEndpoint {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipPemServiceChainEndpoint::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.service_endpoints = crate::parser::scalar::list_field(&props, "service-endpoints");
    obj.steering_policy = crate::parser::scalar::get_str(&props, "steering-policy");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityBotDefenseProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_bot_defense_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityBotDefenseProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityBotDefenseProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.app_service = crate::parser::scalar::get_str(&props, "app-service");
    obj.template = crate::parser::scalar::get_str(&props, "template");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityDeviceIdAttribute` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_device_id_attribute(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityDeviceIdAttribute {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityDeviceIdAttribute::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.id_ = crate::parser::scalar::get_str(&props, "id");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityDosProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_dos_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityDosProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityDosProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.app_service = crate::parser::scalar::get_str(&props, "app-service");
    obj.threshold_sensitivity = crate::parser::scalar::get_str(&props, "threshold-sensitivity");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallAddressList` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_address_list(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallAddressList {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallAddressList::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.addresses = crate::parser::scalar::list_field(&props, "addresses");
    obj.address_lists = crate::parser::scalar::list_field(&props, "address-lists");
    obj.fqdns = crate::parser::scalar::list_field(&props, "fqdns");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallConfigChangeLog` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_config_change_log(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallConfigChangeLog {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallConfigChangeLog::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.log_publisher = crate::parser::scalar::get_str(&props, "log-publisher");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallConfigEntityId` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_config_entity_id(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallConfigEntityId {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallConfigEntityId::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.entity_id = crate::parser::scalar::get_str(&props, "entity-id");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallGlobalFqdnPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_global_fqdn_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallGlobalFqdnPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallGlobalFqdnPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.context = crate::parser::scalar::get_str(&props, "context");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallGlobalRules` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_global_rules(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallGlobalRules {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallGlobalRules::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.enforced_policy = crate::parser::scalar::get_str(&props, "enforced-policy");
    obj.staged_policy = crate::parser::scalar::get_str(&props, "staged-policy");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallManagementIpRules` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_management_ip_rules(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallManagementIpRules {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallManagementIpRules::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallOnDemandCompilation` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_on_demand_compilation(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallOnDemandCompilation {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallOnDemandCompilation::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallOnDemandRuleDeploy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_on_demand_rule_deploy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallOnDemandRuleDeploy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallOnDemandRuleDeploy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.rule_lists = crate::parser::scalar::list_field(&props, "rule-lists");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallPortList` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_port_list(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallPortList {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallPortList::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.ports = crate::parser::scalar::list_field(&props, "ports");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallPortMisusePolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_port_misuse_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallPortMisusePolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallPortMisusePolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.default_log = crate::parser::scalar::get_str(&props, "default-log");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallRuleList` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_rule_list(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallRuleList {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallRuleList::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallSchedule` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_schedule(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallSchedule {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallSchedule::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.daily_hour_end = crate::parser::scalar::get_str(&props, "daily-hour-end");
    obj.daily_hour_start = crate::parser::scalar::get_str(&props, "daily-hour-start");
    obj.days_of_week = crate::parser::scalar::list_field(&props, "days-of-week");
    obj.date_valid_end = crate::parser::scalar::get_str(&props, "date-valid-end");
    obj.date_valid_start = crate::parser::scalar::get_str(&props, "date-valid-start");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallUserDomain` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_user_domain(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallUserDomain {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallUserDomain::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.domain = crate::parser::scalar::get_str(&props, "domain");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallUserList` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_user_list(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallUserList {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallUserList::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.users = crate::parser::scalar::list_field(&props, "users");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityFirewallUuidDefaultAutogenerate` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_firewall_uuid_default_autogenerate(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityFirewallUuidDefaultAutogenerate {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityFirewallUuidDefaultAutogenerate::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.auto_generate_uuid = crate::parser::scalar::get_str(&props, "auto-generate-uuid");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityHttpProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_http_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityHttpProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityHttpProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityIpIntelligenceFeedList` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_ip_intelligence_feed_list(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityIpIntelligenceFeedList {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityIpIntelligenceFeedList::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.feeds = crate::parser::scalar::list_field(&props, "feeds");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityIpIntelligenceGlobalPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_ip_intelligence_global_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityIpIntelligenceGlobalPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityIpIntelligenceGlobalPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.log_blacklist_category = crate::parser::scalar::get_str(&props, "log-blacklist-category");
    obj.log_publisher = crate::parser::scalar::get_str(&props, "log-publisher");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityIpIntelligencePolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_ip_intelligence_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityIpIntelligencePolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityIpIntelligencePolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.default_action = crate::parser::scalar::get_str(&props, "default-action");
    obj.default_log_blacklist_hit_only =
        crate::parser::scalar::get_str(&props, "default-log-blacklist-hit-only");
    obj.default_log_blacklist_category =
        crate::parser::scalar::get_str(&props, "default-log-blacklist-category");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityLogProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_log_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityLogProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityLogProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.application_data = crate::parser::scalar::get_str(&props, "application-data");
    obj.network_data = crate::parser::scalar::get_str(&props, "network-data");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityNatDestinationTranslation` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_nat_destination_translation(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityNatDestinationTranslation {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityNatDestinationTranslation::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.type_ = crate::parser::scalar::get_str(&props, "type");
    obj.addresses = crate::parser::scalar::list_field(&props, "addresses");
    obj.ports = crate::parser::scalar::list_field(&props, "ports");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityNatPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_nat_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityNatPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityNatPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.rule_lists = crate::parser::scalar::list_field(&props, "rule-lists");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityNatSourceTranslation` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_nat_source_translation(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityNatSourceTranslation {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityNatSourceTranslation::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.type_ = crate::parser::scalar::get_str(&props, "type");
    obj.addresses = crate::parser::scalar::list_field(&props, "addresses");
    obj.ports = crate::parser::scalar::list_field(&props, "ports");
    obj.traffic_group = crate::parser::scalar::get_str(&props, "traffic-group");
    obj.egress_interfaces_disabled =
        crate::parser::scalar::get_bool(&props, "egress-interfaces-disabled");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityPacketFilterDefaultRules` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_packet_filter_default_rules(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityPacketFilterDefaultRules {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityPacketFilterDefaultRules::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.action = crate::parser::scalar::get_str(&props, "action");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityPacketFilterPolicy` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_packet_filter_policy(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityPacketFilterPolicy {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityPacketFilterPolicy::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.rules = crate::parser::scalar::list_field(&props, "rules");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityProtectedZone` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_protected_zone(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityProtectedZone {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityProtectedZone::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.enabled = crate::parser::scalar::get_str(&props, "enabled");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityProtocolInspectionComplianceMap` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_protocol_inspection_compliance_map(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityProtocolInspectionComplianceMap {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityProtocolInspectionComplianceMap::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.insp_id = crate::parser::scalar::get_str(&props, "insp-id");
    obj.key_type = crate::parser::scalar::get_str(&props, "key-type");
    obj.value_type = crate::parser::scalar::get_str(&props, "value-type");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityProtocolInspectionComplianceObject` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_protocol_inspection_compliance_object(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityProtocolInspectionComplianceObject {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityProtocolInspectionComplianceObject::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.insp_id = crate::parser::scalar::get_str(&props, "insp-id");
    obj.type_ = crate::parser::scalar::get_str(&props, "type");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecuritySshProfile` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_ssh_profile(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecuritySshProfile {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecuritySshProfile::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.defaults_from = crate::parser::scalar::get_str(&props, "defaults-from");
    obj.timeout = crate::parser::scalar::get_str(&props, "timeout");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSecurityZone` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_security_zone(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSecurityZone {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSecurityZone::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.description = crate::parser::scalar::description(&props);
    obj.vlans = crate::parser::scalar::list_field(&props, "vlans");
    obj.tunnels = crate::parser::scalar::list_field(&props, "tunnels");
    obj.interfaces = crate::parser::scalar::list_field(&props, "interfaces");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysDns` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_dns(full_path: &str, body: &str, range: crate::range::Range) -> BigipSysDns {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysDns::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.name_servers = crate::parser::scalar::list_field(&props, "name-servers");
    obj.search = crate::parser::scalar::list_field(&props, "search");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysFileSslCert` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_file_ssl_cert(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysFileSslCert {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysFileSslCert::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.source_path = crate::parser::scalar::get_str(&props, "source-path");
    obj.cache_path = crate::parser::scalar::get_str(&props, "cache-path");
    obj.revision = crate::parser::scalar::get_str(&props, "revision");
    obj.description = crate::parser::scalar::description(&props);
    obj.issuer = crate::parser::scalar::get_str(&props, "issuer");
    obj.subject = crate::parser::scalar::get_str(&props, "subject");
    obj.expiration_string = crate::parser::scalar::get_str(&props, "expiration-string");
    obj.expiration_date = crate::parser::scalar::get_str(&props, "expiration-date");
    obj.fingerprint = crate::parser::scalar::get_str(&props, "fingerprint");
    obj.key_size = crate::parser::scalar::get_str(&props, "key-size");
    obj.key_type = crate::parser::scalar::get_str(&props, "key-type");
    obj.is_bundle = crate::parser::scalar::get_str(&props, "is-bundle");
    obj.certificate_key_size = crate::parser::scalar::get_str(&props, "certificate-key-size");
    obj.issuer_cert = crate::parser::scalar::get_str(&props, "issuer-cert");
    obj.serial_number = crate::parser::scalar::get_str(&props, "serial-number");
    obj.version = crate::parser::scalar::get_str(&props, "version");
    obj.subject_alternative_name =
        crate::parser::scalar::get_str(&props, "subject-alternative-name");
    obj.bundle_certificates = crate::parser::scalar::list_field(&props, "bundle-certificates");
    obj.cert_validation_options =
        crate::parser::scalar::list_field(&props, "cert-validation-options");
    obj.cert_validators = crate::parser::scalar::list_field(&props, "cert-validators");
    obj.checksum = crate::parser::scalar::get_str(&props, "checksum");
    obj.mode = crate::parser::scalar::get_str(&props, "mode");
    obj.size = crate::parser::scalar::get_str(&props, "size");
    obj.create_time = crate::parser::scalar::get_str(&props, "create-time");
    obj.created_by = crate::parser::scalar::get_str(&props, "created-by");
    obj.last_update_time = crate::parser::scalar::get_str(&props, "last-update-time");
    obj.updated_by = crate::parser::scalar::get_str(&props, "updated-by");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysFileSslKey` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_file_ssl_key(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysFileSslKey {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysFileSslKey::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.source_path = crate::parser::scalar::get_str(&props, "source-path");
    obj.cache_path = crate::parser::scalar::get_str(&props, "cache-path");
    obj.revision = crate::parser::scalar::get_str(&props, "revision");
    obj.passphrase = crate::parser::scalar::get_str(&props, "passphrase");
    obj.description = crate::parser::scalar::description(&props);
    obj.key_size = crate::parser::scalar::get_str(&props, "key-size");
    obj.key_type = crate::parser::scalar::get_str(&props, "key-type");
    obj.security_type = crate::parser::scalar::get_str(&props, "security-type");
    obj.checksum = crate::parser::scalar::get_str(&props, "checksum");
    obj.mode = crate::parser::scalar::get_str(&props, "mode");
    obj.size = crate::parser::scalar::get_str(&props, "size");
    obj.create_time = crate::parser::scalar::get_str(&props, "create-time");
    obj.created_by = crate::parser::scalar::get_str(&props, "created-by");
    obj.last_update_time = crate::parser::scalar::get_str(&props, "last-update-time");
    obj.updated_by = crate::parser::scalar::get_str(&props, "updated-by");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysFolder` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_folder(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysFolder {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysFolder::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.device_group = crate::parser::scalar::get_str(&props, "device-group");
    obj.traffic_group = crate::parser::scalar::get_str(&props, "traffic-group");
    obj.hidden = crate::parser::scalar::get_str(&props, "hidden");
    obj.description = crate::parser::scalar::description(&props);
    obj.inherited_device_group = crate::parser::scalar::get_str(&props, "inherited-device-group");
    obj.inherited_traffic_group = crate::parser::scalar::get_str(&props, "inherited-traffic-group");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysGlobalSettings` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_global_settings(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysGlobalSettings {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysGlobalSettings::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.hostname = crate::parser::scalar::get_str(&props, "hostname");
    obj.gui_setup = crate::parser::scalar::get_str(&props, "gui-setup");
    obj.mgmt_dhcp = crate::parser::scalar::get_str(&props, "mgmt-dhcp");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysManagementRoute` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_management_route(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysManagementRoute {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysManagementRoute::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.gateway = crate::parser::scalar::get_str(&props, "gateway");
    obj.network = crate::parser::scalar::get_str(&props, "network");
    obj.mtu = crate::parser::scalar::get_str(&props, "mtu");
    obj.description = crate::parser::scalar::description(&props);
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysNtp` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_ntp(full_path: &str, body: &str, range: crate::range::Range) -> BigipSysNtp {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysNtp::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.servers = crate::parser::scalar::list_field(&props, "servers");
    obj.timezone = crate::parser::scalar::get_str(&props, "timezone");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysNtpRestrict` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_ntp_restrict(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysNtpRestrict {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysNtpRestrict::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.address = crate::parser::scalar::get_str(&props, "address");
    obj.mask = crate::parser::scalar::get_str(&props, "mask");
    obj.default_entry = crate::parser::scalar::get_str(&props, "default-entry");
    obj.flags = crate::parser::scalar::list_field(&props, "flags");
    obj.description = crate::parser::scalar::description(&props);
    obj
}

/// Scalar parser for `BigipSysProvision` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_provision(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysProvision {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysProvision::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.level = crate::parser::scalar::get_str(&props, "level");
    obj.cpu_ratio = crate::parser::scalar::get_str(&props, "cpu-ratio");
    obj.memory_ratio = crate::parser::scalar::get_str(&props, "memory-ratio");
    obj.disk_ratio = crate::parser::scalar::get_str(&props, "disk-ratio");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysSnmp` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_snmp(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysSnmp {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysSnmp::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.full_path = full_path.to_owned();
    obj.agent_addresses = crate::parser::scalar::list_field(&props, "agent-addresses");
    obj.communities = crate::parser::scalar::list_field(&props, "communities");
    obj.sys_contact = crate::parser::scalar::get_str(&props, "sys-contact");
    obj.sys_location = crate::parser::scalar::get_str(&props, "sys-location");
    obj.sys_services = crate::parser::scalar::get_str(&props, "sys-services");
    obj.trap_community = crate::parser::scalar::get_str(&props, "trap-community");
    obj.range = Some(range);
    obj
}

/// Scalar parser for `BigipSysSnmpDiskMonitor` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_snmp_disk_monitor(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysSnmpDiskMonitor {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysSnmpDiskMonitor::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.partition = crate::parser::scalar::get_str(&props, "partition");
    obj.min_space = crate::parser::scalar::get_str(&props, "min-space");
    obj.description = crate::parser::scalar::description(&props);
    obj
}

/// Scalar parser for `BigipSysSnmpProcessMonitor` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_snmp_process_monitor(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysSnmpProcessMonitor {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysSnmpProcessMonitor::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.process = crate::parser::scalar::get_str(&props, "process");
    obj.max_processes = crate::parser::scalar::get_str(&props, "max-processes");
    obj.min_processes = crate::parser::scalar::get_str(&props, "min-processes");
    obj.description = crate::parser::scalar::description(&props);
    obj
}

/// Scalar parser for `BigipSysSnmpTrap` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_snmp_trap(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysSnmpTrap {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysSnmpTrap::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.host = crate::parser::scalar::get_str(&props, "host");
    obj.port = crate::parser::scalar::get_str(&props, "port");
    obj.version = crate::parser::scalar::get_str(&props, "version");
    obj.community = crate::parser::scalar::get_str(&props, "community");
    obj.security_name = crate::parser::scalar::get_str(&props, "security-name");
    obj.security_level = crate::parser::scalar::get_str(&props, "security-level");
    obj.auth_protocol = crate::parser::scalar::get_str(&props, "auth-protocol");
    obj.privacy_protocol = crate::parser::scalar::get_str(&props, "privacy-protocol");
    obj.network = crate::parser::scalar::get_str(&props, "network");
    obj.description = crate::parser::scalar::description(&props);
    obj
}

/// Scalar parser for `BigipSysSnmpUser` (generated). Structured fields stay
/// at their `Default`; faithful for scalar-only kinds.
#[must_use]
pub fn parse_bigip_sys_snmp_user(
    full_path: &str,
    body: &str,
    range: crate::range::Range,
) -> BigipSysSnmpUser {
    let props = crate::parser::scalar::props_map(body);
    let mut obj = BigipSysSnmpUser::default();
    let _ = &props;
    obj.name = crate::parser::scalar::name_leaf(full_path);
    obj.username = crate::parser::scalar::get_str(&props, "username");
    obj.security_level = crate::parser::scalar::get_str(&props, "security-level");
    obj.auth_protocol = crate::parser::scalar::get_str(&props, "auth-protocol");
    obj.privacy_protocol = crate::parser::scalar::get_str(&props, "privacy-protocol");
    obj.oid_subset = crate::parser::scalar::get_str(&props, "oid-subset");
    obj.description = crate::parser::scalar::description(&props);
    obj
}
