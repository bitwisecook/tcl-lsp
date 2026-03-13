# _mock_stubs.tcl -- AUTO-GENERATED stub mocks for iRule commands
#
# DO NOT EDIT.  Regenerate with:
#   python -m core.irule_test.codegen_mock_stubs
#
# These stubs provide minimal mock implementations for iRule commands
# that do not have hand-written mocks in command_mocks.tcl.  Each stub
# logs the call to the decision log and returns an empty string.
#
# Source: core/commands/registry/
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.


namespace eval ::itest::cmd {

    # AAA:: stubs (4 commands)

    proc aaa_acct_result {args} {
        ::itest::log_decision aaa acct_result $args
        return ""
    }

    proc aaa_acct_send {args} {
        ::itest::log_decision aaa acct_send $args
        return ""
    }

    proc aaa_auth_result {args} {
        ::itest::log_decision aaa auth_result $args
        return ""
    }

    proc aaa_auth_send {args} {
        ::itest::log_decision aaa auth_send $args
        return ""
    }


    # ACCESS:: stubs (15 commands)

    proc access_acl {args} {
        ::itest::log_decision access acl $args
        return ""
    }

    proc access_disable {args} {
        ::itest::log_decision access disable $args
        return ""
    }

    proc access_enable {args} {
        ::itest::log_decision access enable $args
        return ""
    }

    proc access_ephemeral_auth {args} {
        ::itest::log_decision access ephemeral-auth $args
        return ""
    }

    proc access_flowid {args} {
        ::itest::log_decision access flowid $args
        return ""
    }

    proc access_log {args} {
        ::itest::log_decision access log $args
        return ""
    }

    proc access_oauth {args} {
        ::itest::log_decision access oauth $args
        return ""
    }

    proc access_perflow {args} {
        ::itest::log_decision access perflow $args
        return ""
    }

    proc access_policy {args} {
        ::itest::log_decision access policy $args
        return ""
    }

    proc access_respond {args} {
        ::itest::log_decision access respond $args
        return ""
    }

    proc access_restrict_irule_events {args} {
        ::itest::log_decision access restrict_irule_events $args
        return ""
    }

    proc access_saml {args} {
        ::itest::log_decision access saml $args
        return ""
    }

    proc access_session {args} {
        ::itest::log_decision access session $args
        return ""
    }

    proc access_user {args} {
        ::itest::log_decision access user $args
        return ""
    }

    proc access_uuid {args} {
        ::itest::log_decision access uuid $args
        return ""
    }


    # ACCESS2:: stubs (1 commands)

    proc access2_access2_proc {args} {
        ::itest::log_decision access2 access2_proc $args
        return ""
    }


    # ACL:: stubs (2 commands)

    proc acl_action {args} {
        ::itest::log_decision acl action $args
        return ""
    }

    proc acl_eval {args} {
        ::itest::log_decision acl eval $args
        return ""
    }


    # ADAPT:: stubs (12 commands)

    proc adapt_allow {args} {
        ::itest::log_decision adapt allow $args
        return ""
    }

    proc adapt_context_create {args} {
        ::itest::log_decision adapt context_create $args
        return ""
    }

    proc adapt_context_current {args} {
        ::itest::log_decision adapt context_current $args
        return ""
    }

    proc adapt_context_delete_all {args} {
        ::itest::log_decision adapt context_delete_all $args
        return ""
    }

    proc adapt_context_name {args} {
        ::itest::log_decision adapt context_name $args
        return ""
    }

    proc adapt_context_static {args} {
        ::itest::log_decision adapt context_static $args
        return ""
    }

    proc adapt_enable {args} {
        ::itest::log_decision adapt enable $args
        return ""
    }

    proc adapt_preview_size {args} {
        ::itest::log_decision adapt preview_size $args
        return ""
    }

    proc adapt_result {args} {
        ::itest::log_decision adapt result $args
        return ""
    }

    proc adapt_select {args} {
        ::itest::log_decision adapt select $args
        return ""
    }

    proc adapt_service_down_action {args} {
        ::itest::log_decision adapt service_down_action $args
        return ""
    }

    proc adapt_timeout {args} {
        ::itest::log_decision adapt timeout $args
        return ""
    }


    # AES:: stubs (3 commands)

    proc aes_decrypt {args} {
        ::itest::log_decision aes decrypt $args
        return ""
    }

    proc aes_encrypt {args} {
        ::itest::log_decision aes encrypt $args
        return ""
    }

    proc aes_key {args} {
        ::itest::log_decision aes key $args
        return ""
    }


    # AM:: stubs (7 commands)

    proc am_age {args} {
        ::itest::log_decision am age $args
        return ""
    }

    proc am_application {args} {
        ::itest::log_decision am application $args
        return ""
    }

    proc am_cache {args} {
        ::itest::log_decision am cache $args
        return ""
    }

    proc am_disable {args} {
        ::itest::log_decision am disable $args
        return ""
    }

    proc am_expires {args} {
        ::itest::log_decision am expires $args
        return ""
    }

    proc am_media_playlist {args} {
        ::itest::log_decision am media_playlist $args
        return ""
    }

    proc am_policy_node {args} {
        ::itest::log_decision am policy_node $args
        return ""
    }


    # ANTIFRAUD:: stubs (39 commands)

    proc antifraud_alert_additional_info {args} {
        ::itest::log_decision antifraud alert_additional_info $args
        return ""
    }

    proc antifraud_alert_bait_signatures {args} {
        ::itest::log_decision antifraud alert_bait_signatures $args
        return ""
    }

    proc antifraud_alert_component {args} {
        ::itest::log_decision antifraud alert_component $args
        return ""
    }

    proc antifraud_alert_defined_value {args} {
        ::itest::log_decision antifraud alert_defined_value $args
        return ""
    }

    proc antifraud_alert_details {args} {
        ::itest::log_decision antifraud alert_details $args
        return ""
    }

    proc antifraud_alert_device_id {args} {
        ::itest::log_decision antifraud alert_device_id $args
        return ""
    }

    proc antifraud_alert_expected_value {args} {
        ::itest::log_decision antifraud alert_expected_value $args
        return ""
    }

    proc antifraud_alert_fingerprint {args} {
        ::itest::log_decision antifraud alert_fingerprint $args
        return ""
    }

    proc antifraud_alert_forbidden_added_element {args} {
        ::itest::log_decision antifraud alert_forbidden_added_element $args
        return ""
    }

    proc antifraud_alert_guid {args} {
        ::itest::log_decision antifraud alert_guid $args
        return ""
    }

    proc antifraud_alert_html {args} {
        ::itest::log_decision antifraud alert_html $args
        return ""
    }

    proc antifraud_alert_http_referrer {args} {
        ::itest::log_decision antifraud alert_http_referrer $args
        return ""
    }

    proc antifraud_alert_id {args} {
        ::itest::log_decision antifraud alert_id $args
        return ""
    }

    proc antifraud_alert_license_id {args} {
        ::itest::log_decision antifraud alert_license_id $args
        return ""
    }

    proc antifraud_alert_min {args} {
        ::itest::log_decision antifraud alert_min $args
        return ""
    }

    proc antifraud_alert_origin {args} {
        ::itest::log_decision antifraud alert_origin $args
        return ""
    }

    proc antifraud_alert_resolved_value {args} {
        ::itest::log_decision antifraud alert_resolved_value $args
        return ""
    }

    proc antifraud_alert_score {args} {
        ::itest::log_decision antifraud alert_score $args
        return ""
    }

    proc antifraud_alert_transaction_data {args} {
        ::itest::log_decision antifraud alert_transaction_data $args
        return ""
    }

    proc antifraud_alert_transaction_id {args} {
        ::itest::log_decision antifraud alert_transaction_id $args
        return ""
    }

    proc antifraud_alert_type {args} {
        ::itest::log_decision antifraud alert_type $args
        return ""
    }

    proc antifraud_alert_username {args} {
        ::itest::log_decision antifraud alert_username $args
        return ""
    }

    proc antifraud_alert_view_id {args} {
        ::itest::log_decision antifraud alert_view_id $args
        return ""
    }

    proc antifraud_client_id {args} {
        ::itest::log_decision antifraud client_id $args
        return ""
    }

    proc antifraud_device_id {args} {
        ::itest::log_decision antifraud device_id $args
        return ""
    }

    proc antifraud_disable {args} {
        ::itest::log_decision antifraud disable $args
        return ""
    }

    proc antifraud_disable_alert {args} {
        ::itest::log_decision antifraud disable_alert $args
        return ""
    }

    proc antifraud_disable_app_layer_encryption {args} {
        ::itest::log_decision antifraud disable_app_layer_encryption $args
        return ""
    }

    proc antifraud_disable_auto_transactions {args} {
        ::itest::log_decision antifraud disable_auto_transactions $args
        return ""
    }

    proc antifraud_disable_injection {args} {
        ::itest::log_decision antifraud disable_injection $args
        return ""
    }

    proc antifraud_disable_malware {args} {
        ::itest::log_decision antifraud disable_malware $args
        return ""
    }

    proc antifraud_disable_phishing {args} {
        ::itest::log_decision antifraud disable_phishing $args
        return ""
    }

    proc antifraud_enable {args} {
        ::itest::log_decision antifraud enable $args
        return ""
    }

    proc antifraud_enable_log {args} {
        ::itest::log_decision antifraud enable_log $args
        return ""
    }

    proc antifraud_fingerprint {args} {
        ::itest::log_decision antifraud fingerprint $args
        return ""
    }

    proc antifraud_geo {args} {
        ::itest::log_decision antifraud geo $args
        return ""
    }

    proc antifraud_guid {args} {
        ::itest::log_decision antifraud guid $args
        return ""
    }

    proc antifraud_result {args} {
        ::itest::log_decision antifraud result $args
        return ""
    }

    proc antifraud_username {args} {
        ::itest::log_decision antifraud username $args
        return ""
    }


    # ASM:: stubs (25 commands)

    proc asm_captcha {args} {
        ::itest::log_decision asm captcha $args
        return ""
    }

    proc asm_captcha_age {args} {
        ::itest::log_decision asm captcha_age $args
        return ""
    }

    proc asm_captcha_status {args} {
        ::itest::log_decision asm captcha_status $args
        return ""
    }

    proc asm_client_ip {args} {
        ::itest::log_decision asm client_ip $args
        return ""
    }

    proc asm_conviction {args} {
        ::itest::log_decision asm conviction $args
        return ""
    }

    proc asm_deception {args} {
        ::itest::log_decision asm deception $args
        return ""
    }

    proc asm_disable {args} {
        ::itest::log_decision asm disable $args
        return ""
    }

    proc asm_enable {args} {
        ::itest::log_decision asm enable $args
        return ""
    }

    proc asm_fingerprint {args} {
        ::itest::log_decision asm fingerprint $args
        return ""
    }

    proc asm_is_authenticated {args} {
        ::itest::log_decision asm is_authenticated $args
        return ""
    }

    proc asm_login_status {args} {
        ::itest::log_decision asm login_status $args
        return ""
    }

    proc asm_microservice {args} {
        ::itest::log_decision asm microservice $args
        return ""
    }

    proc asm_payload {args} {
        ::itest::log_decision asm payload $args
        return ""
    }

    proc asm_policy {args} {
        ::itest::log_decision asm policy $args
        return ""
    }

    proc asm_raise {args} {
        ::itest::log_decision asm raise $args
        return ""
    }

    proc asm_severity {args} {
        ::itest::log_decision asm severity $args
        return ""
    }

    proc asm_signature {args} {
        ::itest::log_decision asm signature $args
        return ""
    }

    proc asm_status {args} {
        ::itest::log_decision asm status $args
        return ""
    }

    proc asm_support_id {args} {
        ::itest::log_decision asm support_id $args
        return ""
    }

    proc asm_threat_campaign {args} {
        ::itest::log_decision asm threat_campaign $args
        return ""
    }

    proc asm_unblock {args} {
        ::itest::log_decision asm unblock $args
        return ""
    }

    proc asm_uncaptcha {args} {
        ::itest::log_decision asm uncaptcha $args
        return ""
    }

    proc asm_username {args} {
        ::itest::log_decision asm username $args
        return ""
    }

    proc asm_violation {args} {
        ::itest::log_decision asm violation $args
        return ""
    }

    proc asm_violation_data {args} {
        ::itest::log_decision asm violation_data $args
        return ""
    }


    # ASN1:: stubs (3 commands)

    proc asn1_decode {args} {
        ::itest::log_decision asn1 decode $args
        return ""
    }

    proc asn1_element {args} {
        ::itest::log_decision asn1 element $args
        return ""
    }

    proc asn1_encode {args} {
        ::itest::log_decision asn1 encode $args
        return ""
    }


    # AUTH:: stubs (18 commands)

    proc auth_abort {args} {
        ::itest::log_decision auth abort $args
        return ""
    }

    proc auth_authenticate {args} {
        ::itest::log_decision auth authenticate $args
        return ""
    }

    proc auth_authenticate_continue {args} {
        ::itest::log_decision auth authenticate_continue $args
        return ""
    }

    proc auth_cert_credential {args} {
        ::itest::log_decision auth cert_credential $args
        return ""
    }

    proc auth_cert_issuer_credential {args} {
        ::itest::log_decision auth cert_issuer_credential $args
        return ""
    }

    proc auth_last_event_session_id {args} {
        ::itest::log_decision auth last_event_session_id $args
        return ""
    }

    proc auth_password_credential {args} {
        ::itest::log_decision auth password_credential $args
        return ""
    }

    proc auth_response_data {args} {
        ::itest::log_decision auth response_data $args
        return ""
    }

    proc auth_ssl_cc_ldap_status {args} {
        ::itest::log_decision auth ssl_cc_ldap_status $args
        return ""
    }

    proc auth_ssl_cc_ldap_username {args} {
        ::itest::log_decision auth ssl_cc_ldap_username $args
        return ""
    }

    proc auth_start {args} {
        ::itest::log_decision auth start $args
        return ""
    }

    proc auth_status {args} {
        ::itest::log_decision auth status $args
        return ""
    }

    proc auth_subscribe {args} {
        ::itest::log_decision auth subscribe $args
        return ""
    }

    proc auth_unsubscribe {args} {
        ::itest::log_decision auth unsubscribe $args
        return ""
    }

    proc auth_username_credential {args} {
        ::itest::log_decision auth username_credential $args
        return ""
    }

    proc auth_wantcredential_prompt {args} {
        ::itest::log_decision auth wantcredential_prompt $args
        return ""
    }

    proc auth_wantcredential_prompt_style {args} {
        ::itest::log_decision auth wantcredential_prompt_style $args
        return ""
    }

    proc auth_wantcredential_type {args} {
        ::itest::log_decision auth wantcredential_type $args
        return ""
    }


    # AVR:: stubs (4 commands)

    proc avr_disable {args} {
        ::itest::log_decision avr disable $args
        return ""
    }

    proc avr_disable_cspm_injection {args} {
        ::itest::log_decision avr disable_cspm_injection $args
        return ""
    }

    proc avr_enable {args} {
        ::itest::log_decision avr enable $args
        return ""
    }

    proc avr_log {args} {
        ::itest::log_decision avr log $args
        return ""
    }


    # BIGPROTO:: stubs (1 commands)

    proc bigproto_enable_fix_reset {args} {
        ::itest::log_decision bigproto enable_fix_reset $args
        return ""
    }


    # BIGTCP:: stubs (1 commands)

    proc bigtcp_release_flow {args} {
        ::itest::log_decision bigtcp release_flow $args
        return ""
    }


    # BOTDEFENSE:: stubs (25 commands)

    proc botdefense_action {args} {
        ::itest::log_decision botdefense action $args
        return ""
    }

    proc botdefense_bot_anomalies {args} {
        ::itest::log_decision botdefense bot_anomalies $args
        return ""
    }

    proc botdefense_bot_categories {args} {
        ::itest::log_decision botdefense bot_categories $args
        return ""
    }

    proc botdefense_bot_name {args} {
        ::itest::log_decision botdefense bot_name $args
        return ""
    }

    proc botdefense_bot_signature {args} {
        ::itest::log_decision botdefense bot_signature $args
        return ""
    }

    proc botdefense_bot_signature_category {args} {
        ::itest::log_decision botdefense bot_signature_category $args
        return ""
    }

    proc botdefense_captcha_age {args} {
        ::itest::log_decision botdefense captcha_age $args
        return ""
    }

    proc botdefense_captcha_status {args} {
        ::itest::log_decision botdefense captcha_status $args
        return ""
    }

    proc botdefense_client_class {args} {
        ::itest::log_decision botdefense client_class $args
        return ""
    }

    proc botdefense_client_type {args} {
        ::itest::log_decision botdefense client_type $args
        return ""
    }

    proc botdefense_cookie_age {args} {
        ::itest::log_decision botdefense cookie_age $args
        return ""
    }

    proc botdefense_cookie_status {args} {
        ::itest::log_decision botdefense cookie_status $args
        return ""
    }

    proc botdefense_cs_allowed {args} {
        ::itest::log_decision botdefense cs_allowed $args
        return ""
    }

    proc botdefense_cs_attribute {args} {
        ::itest::log_decision botdefense cs_attribute $args
        return ""
    }

    proc botdefense_cs_possible {args} {
        ::itest::log_decision botdefense cs_possible $args
        return ""
    }

    proc botdefense_device_id {args} {
        ::itest::log_decision botdefense device_id $args
        return ""
    }

    proc botdefense_disable {args} {
        ::itest::log_decision botdefense disable $args
        return ""
    }

    proc botdefense_enable {args} {
        ::itest::log_decision botdefense enable $args
        return ""
    }

    proc botdefense_intent {args} {
        ::itest::log_decision botdefense intent $args
        return ""
    }

    proc botdefense_micro_service {args} {
        ::itest::log_decision botdefense micro_service $args
        return ""
    }

    proc botdefense_previous_action {args} {
        ::itest::log_decision botdefense previous_action $args
        return ""
    }

    proc botdefense_previous_request_age {args} {
        ::itest::log_decision botdefense previous_request_age $args
        return ""
    }

    proc botdefense_previous_support_id {args} {
        ::itest::log_decision botdefense previous_support_id $args
        return ""
    }

    proc botdefense_reason {args} {
        ::itest::log_decision botdefense reason $args
        return ""
    }

    proc botdefense_support_id {args} {
        ::itest::log_decision botdefense support_id $args
        return ""
    }


    # BWC:: stubs (8 commands)

    proc bwc_color {args} {
        ::itest::log_decision bwc color $args
        return ""
    }

    proc bwc_debug {args} {
        ::itest::log_decision bwc debug $args
        return ""
    }

    proc bwc_mark {args} {
        ::itest::log_decision bwc mark $args
        return ""
    }

    proc bwc_measure {args} {
        ::itest::log_decision bwc measure $args
        return ""
    }

    proc bwc_policy {args} {
        ::itest::log_decision bwc policy $args
        return ""
    }

    proc bwc_pps {args} {
        ::itest::log_decision bwc pps $args
        return ""
    }

    proc bwc_priority {args} {
        ::itest::log_decision bwc priority $args
        return ""
    }

    proc bwc_rate {args} {
        ::itest::log_decision bwc rate $args
        return ""
    }


    # CACHE:: stubs (17 commands)

    proc cache_accept_encoding {args} {
        ::itest::log_decision cache accept_encoding $args
        return ""
    }

    proc cache_age {args} {
        ::itest::log_decision cache age $args
        return ""
    }

    proc cache_disable {args} {
        ::itest::log_decision cache disable $args
        return ""
    }

    proc cache_disabled {args} {
        ::itest::log_decision cache disabled $args
        return ""
    }

    proc cache_enable {args} {
        ::itest::log_decision cache enable $args
        return ""
    }

    proc cache_expire {args} {
        ::itest::log_decision cache expire $args
        return ""
    }

    proc cache_fresh {args} {
        ::itest::log_decision cache fresh $args
        return ""
    }

    proc cache_header {args} {
        ::itest::log_decision cache header $args
        return ""
    }

    proc cache_headers {args} {
        ::itest::log_decision cache headers $args
        return ""
    }

    proc cache_hits {args} {
        ::itest::log_decision cache hits $args
        return ""
    }

    proc cache_payload {args} {
        ::itest::log_decision cache payload $args
        return ""
    }

    proc cache_priority {args} {
        ::itest::log_decision cache priority $args
        return ""
    }

    proc cache_statskey {args} {
        ::itest::log_decision cache statskey $args
        return ""
    }

    proc cache_trace {args} {
        ::itest::log_decision cache trace $args
        return ""
    }

    proc cache_uri {args} {
        ::itest::log_decision cache uri $args
        return ""
    }

    proc cache_useragent {args} {
        ::itest::log_decision cache useragent $args
        return ""
    }

    proc cache_userkey {args} {
        ::itest::log_decision cache userkey $args
        return ""
    }


    # CATEGORY:: stubs (6 commands)

    proc category_analytics {args} {
        ::itest::log_decision category analytics $args
        return ""
    }

    proc category_filetype {args} {
        ::itest::log_decision category filetype $args
        return ""
    }

    proc category_lookup {args} {
        ::itest::log_decision category lookup $args
        return ""
    }

    proc category_matchtype {args} {
        ::itest::log_decision category matchtype $args
        return ""
    }

    proc category_result {args} {
        ::itest::log_decision category result $args
        return ""
    }

    proc category_safesearch {args} {
        ::itest::log_decision category safesearch $args
        return ""
    }


    # CLASSIFICATION:: stubs (8 commands)

    proc classification_app {args} {
        ::itest::log_decision classification app $args
        return ""
    }

    proc classification_category {args} {
        ::itest::log_decision classification category $args
        return ""
    }

    proc classification_disable {args} {
        ::itest::log_decision classification disable $args
        return ""
    }

    proc classification_enable {args} {
        ::itest::log_decision classification enable $args
        return ""
    }

    proc classification_protocol {args} {
        ::itest::log_decision classification protocol $args
        return ""
    }

    proc classification_result {args} {
        ::itest::log_decision classification result $args
        return ""
    }

    proc classification_urlcat {args} {
        ::itest::log_decision classification urlcat $args
        return ""
    }

    proc classification_username {args} {
        ::itest::log_decision classification username $args
        return ""
    }


    # CLASSIFY:: stubs (6 commands)

    proc classify_application {args} {
        ::itest::log_decision classify application $args
        return ""
    }

    proc classify_category {args} {
        ::itest::log_decision classify category $args
        return ""
    }

    proc classify_defer {args} {
        ::itest::log_decision classify defer $args
        return ""
    }

    proc classify_disable {args} {
        ::itest::log_decision classify disable $args
        return ""
    }

    proc classify_urlcat {args} {
        ::itest::log_decision classify urlcat $args
        return ""
    }

    proc classify_username {args} {
        ::itest::log_decision classify username $args
        return ""
    }


    # COMPRESS:: stubs (6 commands)

    proc compress_buffer_size {args} {
        ::itest::log_decision compress buffer_size $args
        return ""
    }

    proc compress_disable {args} {
        ::itest::log_decision compress disable $args
        return ""
    }

    proc compress_enable {args} {
        ::itest::log_decision compress enable $args
        return ""
    }

    proc compress_gzip {args} {
        ::itest::log_decision compress gzip $args
        return ""
    }

    proc compress_method {args} {
        ::itest::log_decision compress method $args
        return ""
    }

    proc compress_nodelay {args} {
        ::itest::log_decision compress nodelay $args
        return ""
    }


    # CONNECTOR:: stubs (4 commands)

    proc connector_disable {args} {
        ::itest::log_decision connector disable $args
        return ""
    }

    proc connector_enable {args} {
        ::itest::log_decision connector enable $args
        return ""
    }

    proc connector_profile {args} {
        ::itest::log_decision connector profile $args
        return ""
    }

    proc connector_remap {args} {
        ::itest::log_decision connector remap $args
        return ""
    }


    # CRYPTO:: stubs (6 commands)

    proc crypto_decrypt {args} {
        ::itest::log_decision crypto decrypt $args
        return ""
    }

    proc crypto_encrypt {args} {
        ::itest::log_decision crypto encrypt $args
        return ""
    }

    proc crypto_hash {args} {
        ::itest::log_decision crypto hash $args
        return ""
    }

    proc crypto_keygen {args} {
        ::itest::log_decision crypto keygen $args
        return ""
    }

    proc crypto_sign {args} {
        ::itest::log_decision crypto sign $args
        return ""
    }

    proc crypto_verify {args} {
        ::itest::log_decision crypto verify $args
        return ""
    }


    # DATAGRAM:: stubs (6 commands)

    proc datagram_dns {args} {
        ::itest::log_decision datagram dns $args
        return ""
    }

    proc datagram_ip {args} {
        ::itest::log_decision datagram ip $args
        return ""
    }

    proc datagram_ip6 {args} {
        ::itest::log_decision datagram ip6 $args
        return ""
    }

    proc datagram_l2 {args} {
        ::itest::log_decision datagram l2 $args
        return ""
    }

    proc datagram_tcp {args} {
        ::itest::log_decision datagram tcp $args
        return ""
    }

    proc datagram_udp {args} {
        ::itest::log_decision datagram udp $args
        return ""
    }


    # DECOMPRESS:: stubs (2 commands)

    proc decompress_disable {args} {
        ::itest::log_decision decompress disable $args
        return ""
    }

    proc decompress_enable {args} {
        ::itest::log_decision decompress enable $args
        return ""
    }


    # DEMANGLE:: stubs (2 commands)

    proc demangle_disable {args} {
        ::itest::log_decision demangle disable $args
        return ""
    }

    proc demangle_enable {args} {
        ::itest::log_decision demangle enable $args
        return ""
    }


    # DHCP:: stubs (1 commands)

    proc dhcp_version {args} {
        ::itest::log_decision dhcp version $args
        return ""
    }


    # DHCPv4:: stubs (16 commands)

    proc dhcpv4_chaddr {args} {
        ::itest::log_decision dhcpv4 chaddr $args
        return ""
    }

    proc dhcpv4_ciaddr {args} {
        ::itest::log_decision dhcpv4 ciaddr $args
        return ""
    }

    proc dhcpv4_drop {args} {
        ::itest::log_decision dhcpv4 drop $args
        return ""
    }

    proc dhcpv4_giaddr {args} {
        ::itest::log_decision dhcpv4 giaddr $args
        return ""
    }

    proc dhcpv4_hlen {args} {
        ::itest::log_decision dhcpv4 hlen $args
        return ""
    }

    proc dhcpv4_hops {args} {
        ::itest::log_decision dhcpv4 hops $args
        return ""
    }

    proc dhcpv4_htype {args} {
        ::itest::log_decision dhcpv4 htype $args
        return ""
    }

    proc dhcpv4_len {args} {
        ::itest::log_decision dhcpv4 len $args
        return ""
    }

    proc dhcpv4_opcode {args} {
        ::itest::log_decision dhcpv4 opcode $args
        return ""
    }

    proc dhcpv4_option {args} {
        ::itest::log_decision dhcpv4 option $args
        return ""
    }

    proc dhcpv4_reject {args} {
        ::itest::log_decision dhcpv4 reject $args
        return ""
    }

    proc dhcpv4_secs {args} {
        ::itest::log_decision dhcpv4 secs $args
        return ""
    }

    proc dhcpv4_siaddr {args} {
        ::itest::log_decision dhcpv4 siaddr $args
        return ""
    }

    proc dhcpv4_type {args} {
        ::itest::log_decision dhcpv4 type $args
        return ""
    }

    proc dhcpv4_xid {args} {
        ::itest::log_decision dhcpv4 xid $args
        return ""
    }

    proc dhcpv4_yiaddr {args} {
        ::itest::log_decision dhcpv4 yiaddr $args
        return ""
    }


    # DHCPv6:: stubs (9 commands)

    proc dhcpv6_drop {args} {
        ::itest::log_decision dhcpv6 drop $args
        return ""
    }

    proc dhcpv6_hop_count {args} {
        ::itest::log_decision dhcpv6 hop_count $args
        return ""
    }

    proc dhcpv6_len {args} {
        ::itest::log_decision dhcpv6 len $args
        return ""
    }

    proc dhcpv6_link_address {args} {
        ::itest::log_decision dhcpv6 link_address $args
        return ""
    }

    proc dhcpv6_msg_type {args} {
        ::itest::log_decision dhcpv6 msg_type $args
        return ""
    }

    proc dhcpv6_option {args} {
        ::itest::log_decision dhcpv6 option $args
        return ""
    }

    proc dhcpv6_peer_address {args} {
        ::itest::log_decision dhcpv6 peer_address $args
        return ""
    }

    proc dhcpv6_reject {args} {
        ::itest::log_decision dhcpv6 reject $args
        return ""
    }

    proc dhcpv6_transaction_id {args} {
        ::itest::log_decision dhcpv6 transaction_id $args
        return ""
    }


    # DIAG:: stubs (1 commands)

    proc diag_test {args} {
        ::itest::log_decision diag test $args
        return ""
    }


    # DIAMETER:: stubs (27 commands)

    proc diameter_avp {args} {
        ::itest::log_decision diameter avp $args
        return ""
    }

    proc diameter_command {args} {
        ::itest::log_decision diameter command $args
        return ""
    }

    proc diameter_disconnect {args} {
        ::itest::log_decision diameter disconnect $args
        return ""
    }

    proc diameter_drop {args} {
        ::itest::log_decision diameter drop $args
        return ""
    }

    proc diameter_dynamic_route_insertion {args} {
        ::itest::log_decision diameter dynamic_route_insertion $args
        return ""
    }

    proc diameter_dynamic_route_lookup {args} {
        ::itest::log_decision diameter dynamic_route_lookup $args
        return ""
    }

    proc diameter_header {args} {
        ::itest::log_decision diameter header $args
        return ""
    }

    proc diameter_host {args} {
        ::itest::log_decision diameter host $args
        return ""
    }

    proc diameter_is_request {args} {
        ::itest::log_decision diameter is_request $args
        return ""
    }

    proc diameter_is_response {args} {
        ::itest::log_decision diameter is_response $args
        return ""
    }

    proc diameter_is_retransmission {args} {
        ::itest::log_decision diameter is_retransmission $args
        return ""
    }

    proc diameter_length {args} {
        ::itest::log_decision diameter length $args
        return ""
    }

    proc diameter_message {args} {
        ::itest::log_decision diameter message $args
        return ""
    }

    proc diameter_payload {args} {
        ::itest::log_decision diameter payload $args
        return ""
    }

    proc diameter_persist {args} {
        ::itest::log_decision diameter persist $args
        return ""
    }

    proc diameter_realm {args} {
        ::itest::log_decision diameter realm $args
        return ""
    }

    proc diameter_respond {args} {
        ::itest::log_decision diameter respond $args
        return ""
    }

    proc diameter_result {args} {
        ::itest::log_decision diameter result $args
        return ""
    }

    proc diameter_retransmission {args} {
        ::itest::log_decision diameter retransmission $args
        return ""
    }

    proc diameter_retransmission_default {args} {
        ::itest::log_decision diameter retransmission_default $args
        return ""
    }

    proc diameter_retransmission_reason {args} {
        ::itest::log_decision diameter retransmission_reason $args
        return ""
    }

    proc diameter_retransmit {args} {
        ::itest::log_decision diameter retransmit $args
        return ""
    }

    proc diameter_retry {args} {
        ::itest::log_decision diameter retry $args
        return ""
    }

    proc diameter_route_status {args} {
        ::itest::log_decision diameter route_status $args
        return ""
    }

    proc diameter_session {args} {
        ::itest::log_decision diameter session $args
        return ""
    }

    proc diameter_skip_capabilities_exchange {args} {
        ::itest::log_decision diameter skip_capabilities_exchange $args
        return ""
    }

    proc diameter_state {args} {
        ::itest::log_decision diameter state $args
        return ""
    }


    # DNS:: stubs (23 commands)

    proc dns_additional {args} {
        ::itest::log_decision dns additional $args
        return ""
    }

    proc dns_authority {args} {
        ::itest::log_decision dns authority $args
        return ""
    }

    proc dns_class {args} {
        ::itest::log_decision dns class $args
        return ""
    }

    proc dns_disable {args} {
        ::itest::log_decision dns disable $args
        return ""
    }

    proc dns_drop {args} {
        ::itest::log_decision dns drop $args
        return ""
    }

    proc dns_edns0 {args} {
        ::itest::log_decision dns edns0 $args
        return ""
    }

    proc dns_enable {args} {
        ::itest::log_decision dns enable $args
        return ""
    }

    proc dns_is_wideip {args} {
        ::itest::log_decision dns is_wideip $args
        return ""
    }

    proc dns_last_act {args} {
        ::itest::log_decision dns last_act $args
        return ""
    }

    proc dns_len {args} {
        ::itest::log_decision dns len $args
        return ""
    }

    proc dns_log {args} {
        ::itest::log_decision dns log $args
        return ""
    }

    proc dns_name {args} {
        ::itest::log_decision dns name $args
        return ""
    }

    proc dns_origin {args} {
        ::itest::log_decision dns origin $args
        return ""
    }

    proc dns_ptype {args} {
        ::itest::log_decision dns ptype $args
        return ""
    }

    proc dns_query {args} {
        ::itest::log_decision dns query $args
        return ""
    }

    proc dns_question {args} {
        ::itest::log_decision dns question $args
        return ""
    }

    proc dns_rdata {args} {
        ::itest::log_decision dns rdata $args
        return ""
    }

    proc dns_rpz_policy {args} {
        ::itest::log_decision dns rpz_policy $args
        return ""
    }

    proc dns_rr {args} {
        ::itest::log_decision dns rr $args
        return ""
    }

    proc dns_scrape {args} {
        ::itest::log_decision dns scrape $args
        return ""
    }

    proc dns_tsig {args} {
        ::itest::log_decision dns tsig $args
        return ""
    }

    proc dns_ttl {args} {
        ::itest::log_decision dns ttl $args
        return ""
    }

    proc dns_type {args} {
        ::itest::log_decision dns type $args
        return ""
    }


    # DNSMSG:: stubs (3 commands)

    proc dnsmsg_header {args} {
        ::itest::log_decision dnsmsg header $args
        return ""
    }

    proc dnsmsg_record {args} {
        ::itest::log_decision dnsmsg record $args
        return ""
    }

    proc dnsmsg_section {args} {
        ::itest::log_decision dnsmsg section $args
        return ""
    }


    # DOSL7:: stubs (7 commands)

    proc dosl7_disable {args} {
        ::itest::log_decision dosl7 disable $args
        return ""
    }

    proc dosl7_enable {args} {
        ::itest::log_decision dosl7 enable $args
        return ""
    }

    proc dosl7_health {args} {
        ::itest::log_decision dosl7 health $args
        return ""
    }

    proc dosl7_is_ip_slowdown {args} {
        ::itest::log_decision dosl7 is_ip_slowdown $args
        return ""
    }

    proc dosl7_is_mitigated {args} {
        ::itest::log_decision dosl7 is_mitigated $args
        return ""
    }

    proc dosl7_profile {args} {
        ::itest::log_decision dosl7 profile $args
        return ""
    }

    proc dosl7_slowdown {args} {
        ::itest::log_decision dosl7 slowdown $args
        return ""
    }


    # DSLITE:: stubs (1 commands)

    proc dslite_remote_addr {args} {
        ::itest::log_decision dslite remote_addr $args
        return ""
    }


    # ECA:: stubs (7 commands)

    proc eca_client_machine_name {args} {
        ::itest::log_decision eca client_machine_name $args
        return ""
    }

    proc eca_disable {args} {
        ::itest::log_decision eca disable $args
        return ""
    }

    proc eca_domainname {args} {
        ::itest::log_decision eca domainname $args
        return ""
    }

    proc eca_enable {args} {
        ::itest::log_decision eca enable $args
        return ""
    }

    proc eca_select {args} {
        ::itest::log_decision eca select $args
        return ""
    }

    proc eca_status {args} {
        ::itest::log_decision eca status $args
        return ""
    }

    proc eca_username {args} {
        ::itest::log_decision eca username $args
        return ""
    }


    # FIX:: stubs (1 commands)

    proc fix_tag {args} {
        ::itest::log_decision fix tag $args
        return ""
    }


    # FLOW:: stubs (7 commands)

    proc flow_create_related {args} {
        ::itest::log_decision flow create_related $args
        return ""
    }

    proc flow_idle_duration {args} {
        ::itest::log_decision flow idle_duration $args
        return ""
    }

    proc flow_idle_timeout {args} {
        ::itest::log_decision flow idle_timeout $args
        return ""
    }

    proc flow_peer {args} {
        ::itest::log_decision flow peer $args
        return ""
    }

    proc flow_priority {args} {
        ::itest::log_decision flow priority $args
        return ""
    }

    proc flow_refresh {args} {
        ::itest::log_decision flow refresh $args
        return ""
    }

    proc flow_this {args} {
        ::itest::log_decision flow this $args
        return ""
    }


    # FLOWTABLE:: stubs (2 commands)

    proc flowtable_count {args} {
        ::itest::log_decision flowtable count $args
        return ""
    }

    proc flowtable_limit {args} {
        ::itest::log_decision flowtable limit $args
        return ""
    }


    # FTP:: stubs (6 commands)

    proc ftp_allow_active_mode {args} {
        ::itest::log_decision ftp allow_active_mode $args
        return ""
    }

    proc ftp_disable {args} {
        ::itest::log_decision ftp disable $args
        return ""
    }

    proc ftp_enable {args} {
        ::itest::log_decision ftp enable $args
        return ""
    }

    proc ftp_enforce_tls_session_reuse {args} {
        ::itest::log_decision ftp enforce_tls_session_reuse $args
        return ""
    }

    proc ftp_ftps_mode {args} {
        ::itest::log_decision ftp ftps_mode $args
        return ""
    }

    proc ftp_port {args} {
        ::itest::log_decision ftp port $args
        return ""
    }


    # GENERICMESSAGE:: stubs (3 commands)

    proc genericmessage_message {args} {
        ::itest::log_decision genericmessage message $args
        return ""
    }

    proc genericmessage_peer {args} {
        ::itest::log_decision genericmessage peer $args
        return ""
    }

    proc genericmessage_route {args} {
        ::itest::log_decision genericmessage route $args
        return ""
    }


    # GTP:: stubs (12 commands)

    proc gtp_clone {args} {
        ::itest::log_decision gtp clone $args
        return ""
    }

    proc gtp_discard {args} {
        ::itest::log_decision gtp discard $args
        return ""
    }

    proc gtp_forward {args} {
        ::itest::log_decision gtp forward $args
        return ""
    }

    proc gtp_header {args} {
        ::itest::log_decision gtp header $args
        return ""
    }

    proc gtp_ie {args} {
        ::itest::log_decision gtp ie $args
        return ""
    }

    proc gtp_length {args} {
        ::itest::log_decision gtp length $args
        return ""
    }

    proc gtp_message {args} {
        ::itest::log_decision gtp message $args
        return ""
    }

    proc gtp_new {args} {
        ::itest::log_decision gtp new $args
        return ""
    }

    proc gtp_parse {args} {
        ::itest::log_decision gtp parse $args
        return ""
    }

    proc gtp_payload {args} {
        ::itest::log_decision gtp payload $args
        return ""
    }

    proc gtp_respond {args} {
        ::itest::log_decision gtp respond $args
        return ""
    }

    proc gtp_tunnel {args} {
        ::itest::log_decision gtp tunnel $args
        return ""
    }


    # HA:: stubs (1 commands)

    proc ha_status {args} {
        ::itest::log_decision ha status $args
        return ""
    }


    # HSL:: stubs (2 commands)

    proc hsl_open {args} {
        ::itest::log_decision hsl open $args
        return ""
    }

    proc hsl_send {args} {
        ::itest::log_decision hsl send $args
        return ""
    }


    # HTML:: stubs (4 commands)

    proc html_comment {args} {
        ::itest::log_decision html comment $args
        return ""
    }

    proc html_disable {args} {
        ::itest::log_decision html disable $args
        return ""
    }

    proc html_enable {args} {
        ::itest::log_decision html enable $args
        return ""
    }

    proc html_tag {args} {
        ::itest::log_decision html tag $args
        return ""
    }


    # HTTP:: stubs (9 commands)

    proc http_class {args} {
        ::itest::log_decision http class $args
        return ""
    }

    proc http_has_responded {args} {
        ::itest::log_decision http has_responded $args
        return ""
    }

    proc http_hsts {args} {
        ::itest::log_decision http hsts $args
        return ""
    }

    proc http_passthrough_reason {args} {
        ::itest::log_decision http passthrough_reason $args
        return ""
    }

    proc http_password {args} {
        ::itest::log_decision http password $args
        return ""
    }

    proc http_proxy {args} {
        ::itest::log_decision http proxy $args
        return ""
    }

    proc http_reject_reason {args} {
        ::itest::log_decision http reject_reason $args
        return ""
    }

    proc http_response {args} {
        ::itest::log_decision http response $args
        return ""
    }

    proc http_username {args} {
        ::itest::log_decision http username $args
        return ""
    }


    # HTTP2:: stubs (10 commands)

    proc http2_active {args} {
        ::itest::log_decision http2 active $args
        return ""
    }

    proc http2_concurrency {args} {
        ::itest::log_decision http2 concurrency $args
        return ""
    }

    proc http2_disable {args} {
        ::itest::log_decision http2 disable $args
        return ""
    }

    proc http2_disconnect {args} {
        ::itest::log_decision http2 disconnect $args
        return ""
    }

    proc http2_enable {args} {
        ::itest::log_decision http2 enable $args
        return ""
    }

    proc http2_header {args} {
        ::itest::log_decision http2 header $args
        return ""
    }

    proc http2_push {args} {
        ::itest::log_decision http2 push $args
        return ""
    }

    proc http2_requests {args} {
        ::itest::log_decision http2 requests $args
        return ""
    }

    proc http2_stream {args} {
        ::itest::log_decision http2 stream $args
        return ""
    }

    proc http2_version {args} {
        ::itest::log_decision http2 version $args
        return ""
    }


    # HTTPLOG:: stubs (2 commands)

    proc httplog_disable {args} {
        ::itest::log_decision httplog disable $args
        return ""
    }

    proc httplog_enable {args} {
        ::itest::log_decision httplog enable $args
        return ""
    }


    # ICAP:: stubs (4 commands)

    proc icap_header {args} {
        ::itest::log_decision icap header $args
        return ""
    }

    proc icap_method {args} {
        ::itest::log_decision icap method $args
        return ""
    }

    proc icap_status {args} {
        ::itest::log_decision icap status $args
        return ""
    }

    proc icap_uri {args} {
        ::itest::log_decision icap uri $args
        return ""
    }


    # IKE:: stubs (12 commands)

    proc ike_auth_success {args} {
        ::itest::log_decision ike auth_success $args
        return ""
    }

    proc ike_cert {args} {
        ::itest::log_decision ike cert $args
        return ""
    }

    proc ike_san_dirname {args} {
        ::itest::log_decision ike san_dirname $args
        return ""
    }

    proc ike_san_dns {args} {
        ::itest::log_decision ike san_dns $args
        return ""
    }

    proc ike_san_ediparty {args} {
        ::itest::log_decision ike san_ediparty $args
        return ""
    }

    proc ike_san_email {args} {
        ::itest::log_decision ike san_email $args
        return ""
    }

    proc ike_san_ipadd {args} {
        ::itest::log_decision ike san_ipadd $args
        return ""
    }

    proc ike_san_othername {args} {
        ::itest::log_decision ike san_othername $args
        return ""
    }

    proc ike_san_rid {args} {
        ::itest::log_decision ike san_rid $args
        return ""
    }

    proc ike_san_uri {args} {
        ::itest::log_decision ike san_uri $args
        return ""
    }

    proc ike_san_x400 {args} {
        ::itest::log_decision ike san_x400 $args
        return ""
    }

    proc ike_subjectAltName {args} {
        ::itest::log_decision ike subjectAltName $args
        return ""
    }


    # ILX:: stubs (3 commands)

    proc ilx_call {args} {
        ::itest::log_decision ilx call $args
        return ""
    }

    proc ilx_init {args} {
        ::itest::log_decision ilx init $args
        return ""
    }

    proc ilx_notify {args} {
        ::itest::log_decision ilx notify $args
        return ""
    }


    # IMAP:: stubs (3 commands)

    proc imap_activation_mode {args} {
        ::itest::log_decision imap activation_mode $args
        return ""
    }

    proc imap_disable {args} {
        ::itest::log_decision imap disable $args
        return ""
    }

    proc imap_enable {args} {
        ::itest::log_decision imap enable $args
        return ""
    }


    # IP:: stubs (9 commands)

    proc ip_addr {args} {
        ::itest::log_decision ip addr $args
        return ""
    }

    proc ip_hops {args} {
        ::itest::log_decision ip hops $args
        return ""
    }

    proc ip_idle_timeout {args} {
        ::itest::log_decision ip idle_timeout $args
        return ""
    }

    proc ip_ingress_drop_rate {args} {
        ::itest::log_decision ip ingress_drop_rate $args
        return ""
    }

    proc ip_ingress_rate_limit {args} {
        ::itest::log_decision ip ingress_rate_limit $args
        return ""
    }

    proc ip_intelligence {args} {
        ::itest::log_decision ip intelligence $args
        return ""
    }

    proc ip_reputation {args} {
        ::itest::log_decision ip reputation $args
        return ""
    }

    proc ip_stats {args} {
        ::itest::log_decision ip stats $args
        return ""
    }

    proc ip_version {args} {
        ::itest::log_decision ip version $args
        return ""
    }


    # IPFIX:: stubs (3 commands)

    proc ipfix_destination {args} {
        ::itest::log_decision ipfix destination $args
        return ""
    }

    proc ipfix_msg {args} {
        ::itest::log_decision ipfix msg $args
        return ""
    }

    proc ipfix_template {args} {
        ::itest::log_decision ipfix template $args
        return ""
    }


    # ISESSION:: stubs (1 commands)

    proc isession_deduplication {args} {
        ::itest::log_decision isession deduplication $args
        return ""
    }


    # ISTATS:: stubs (4 commands)

    proc istats_get {args} {
        ::itest::log_decision istats get $args
        return ""
    }

    proc istats_incr {args} {
        ::itest::log_decision istats incr $args
        return ""
    }

    proc istats_remove {args} {
        ::itest::log_decision istats remove $args
        return ""
    }

    proc istats_set {args} {
        ::itest::log_decision istats set $args
        return ""
    }


    # IVS_ENTRY:: stubs (1 commands)

    proc ivs_entry_result {args} {
        ::itest::log_decision ivs_entry result $args
        return ""
    }


    # JSON:: stubs (9 commands)

    proc json_array {args} {
        ::itest::log_decision json array $args
        return ""
    }

    proc json_create {args} {
        ::itest::log_decision json create $args
        return ""
    }

    proc json_get {args} {
        ::itest::log_decision json get $args
        return ""
    }

    proc json_object {args} {
        ::itest::log_decision json object $args
        return ""
    }

    proc json_parse {args} {
        ::itest::log_decision json parse $args
        return ""
    }

    proc json_render {args} {
        ::itest::log_decision json render $args
        return ""
    }

    proc json_root {args} {
        ::itest::log_decision json root $args
        return ""
    }

    proc json_set {args} {
        ::itest::log_decision json set $args
        return ""
    }

    proc json_type {args} {
        ::itest::log_decision json type $args
        return ""
    }


    # L7CHECK:: stubs (1 commands)

    proc l7check_protocol {args} {
        ::itest::log_decision l7check protocol $args
        return ""
    }


    # LB:: stubs (16 commands)

    proc lb_bias {args} {
        ::itest::log_decision lb bias $args
        return ""
    }

    proc lb_class {args} {
        ::itest::log_decision lb class $args
        return ""
    }

    proc lb_command {args} {
        ::itest::log_decision lb command $args
        return ""
    }

    proc lb_connect {args} {
        ::itest::log_decision lb connect $args
        return ""
    }

    proc lb_connlimit {args} {
        ::itest::log_decision lb connlimit $args
        return ""
    }

    proc lb_context_id {args} {
        ::itest::log_decision lb context_id $args
        return ""
    }

    proc lb_down {args} {
        ::itest::log_decision lb down $args
        return ""
    }

    proc lb_dst_tag {args} {
        ::itest::log_decision lb dst_tag $args
        return ""
    }

    proc lb_enable_decisionlog {args} {
        ::itest::log_decision lb enable_decisionlog $args
        return ""
    }

    proc lb_mode {args} {
        ::itest::log_decision lb mode $args
        return ""
    }

    proc lb_persist {args} {
        ::itest::log_decision lb persist $args
        return ""
    }

    proc lb_prime {args} {
        ::itest::log_decision lb prime $args
        return ""
    }

    proc lb_queue {args} {
        ::itest::log_decision lb queue $args
        return ""
    }

    proc lb_snat {args} {
        ::itest::log_decision lb snat $args
        return ""
    }

    proc lb_src_tag {args} {
        ::itest::log_decision lb src_tag $args
        return ""
    }

    proc lb_up {args} {
        ::itest::log_decision lb up $args
        return ""
    }


    # LDAP:: stubs (3 commands)

    proc ldap_activation_mode {args} {
        ::itest::log_decision ldap activation_mode $args
        return ""
    }

    proc ldap_disable {args} {
        ::itest::log_decision ldap disable $args
        return ""
    }

    proc ldap_enable {args} {
        ::itest::log_decision ldap enable $args
        return ""
    }


    # LINE:: stubs (2 commands)

    proc line_get {args} {
        ::itest::log_decision line get $args
        return ""
    }

    proc line_set {args} {
        ::itest::log_decision line set $args
        return ""
    }


    # LINK:: stubs (4 commands)

    proc link_lasthop {args} {
        ::itest::log_decision link lasthop $args
        return ""
    }

    proc link_nexthop {args} {
        ::itest::log_decision link nexthop $args
        return ""
    }

    proc link_qos {args} {
        ::itest::log_decision link qos $args
        return ""
    }

    proc link_vlan_id {args} {
        ::itest::log_decision link vlan_id $args
        return ""
    }


    # LSN:: stubs (8 commands)

    proc lsn_address {args} {
        ::itest::log_decision lsn address $args
        return ""
    }

    proc lsn_disable {args} {
        ::itest::log_decision lsn disable $args
        return ""
    }

    proc lsn_inbound {args} {
        ::itest::log_decision lsn inbound $args
        return ""
    }

    proc lsn_inbound_entry {args} {
        ::itest::log_decision lsn inbound-entry $args
        return ""
    }

    proc lsn_persistence {args} {
        ::itest::log_decision lsn persistence $args
        return ""
    }

    proc lsn_persistence_entry {args} {
        ::itest::log_decision lsn persistence-entry $args
        return ""
    }

    proc lsn_pool {args} {
        ::itest::log_decision lsn pool $args
        return ""
    }

    proc lsn_port {args} {
        ::itest::log_decision lsn port $args
        return ""
    }


    # MESSAGE:: stubs (3 commands)

    proc message_field {args} {
        ::itest::log_decision message field $args
        return ""
    }

    proc message_proto {args} {
        ::itest::log_decision message proto $args
        return ""
    }

    proc message_type {args} {
        ::itest::log_decision message type $args
        return ""
    }


    # MQTT:: stubs (29 commands)

    proc mqtt_clean_session {args} {
        ::itest::log_decision mqtt clean_session $args
        return ""
    }

    proc mqtt_client_id {args} {
        ::itest::log_decision mqtt client_id $args
        return ""
    }

    proc mqtt_collect {args} {
        ::itest::log_decision mqtt collect $args
        return ""
    }

    proc mqtt_disable {args} {
        ::itest::log_decision mqtt disable $args
        return ""
    }

    proc mqtt_disconnect {args} {
        ::itest::log_decision mqtt disconnect $args
        return ""
    }

    proc mqtt_drop {args} {
        ::itest::log_decision mqtt drop $args
        return ""
    }

    proc mqtt_dup {args} {
        ::itest::log_decision mqtt dup $args
        return ""
    }

    proc mqtt_enable {args} {
        ::itest::log_decision mqtt enable $args
        return ""
    }

    proc mqtt_insert {args} {
        ::itest::log_decision mqtt insert $args
        return ""
    }

    proc mqtt_keep_alive {args} {
        ::itest::log_decision mqtt keep_alive $args
        return ""
    }

    proc mqtt_length {args} {
        ::itest::log_decision mqtt length $args
        return ""
    }

    proc mqtt_message {args} {
        ::itest::log_decision mqtt message $args
        return ""
    }

    proc mqtt_packet_id {args} {
        ::itest::log_decision mqtt packet_id $args
        return ""
    }

    proc mqtt_password {args} {
        ::itest::log_decision mqtt password $args
        return ""
    }

    proc mqtt_payload {args} {
        ::itest::log_decision mqtt payload $args
        return ""
    }

    proc mqtt_protocol_name {args} {
        ::itest::log_decision mqtt protocol_name $args
        return ""
    }

    proc mqtt_protocol_version {args} {
        ::itest::log_decision mqtt protocol_version $args
        return ""
    }

    proc mqtt_qos {args} {
        ::itest::log_decision mqtt qos $args
        return ""
    }

    proc mqtt_release {args} {
        ::itest::log_decision mqtt release $args
        return ""
    }

    proc mqtt_replace {args} {
        ::itest::log_decision mqtt replace $args
        return ""
    }

    proc mqtt_respond {args} {
        ::itest::log_decision mqtt respond $args
        return ""
    }

    proc mqtt_retain {args} {
        ::itest::log_decision mqtt retain $args
        return ""
    }

    proc mqtt_return_code {args} {
        ::itest::log_decision mqtt return_code $args
        return ""
    }

    proc mqtt_return_code_list {args} {
        ::itest::log_decision mqtt return_code_list $args
        return ""
    }

    proc mqtt_session_present {args} {
        ::itest::log_decision mqtt session_present $args
        return ""
    }

    proc mqtt_topic {args} {
        ::itest::log_decision mqtt topic $args
        return ""
    }

    proc mqtt_type {args} {
        ::itest::log_decision mqtt type $args
        return ""
    }

    proc mqtt_username {args} {
        ::itest::log_decision mqtt username $args
        return ""
    }

    proc mqtt_will {args} {
        ::itest::log_decision mqtt will $args
        return ""
    }


    # MR:: stubs (23 commands)

    proc mr_always_match_port {args} {
        ::itest::log_decision mr always_match_port $args
        return ""
    }

    proc mr_available_for_routing {args} {
        ::itest::log_decision mr available_for_routing $args
        return ""
    }

    proc mr_collect {args} {
        ::itest::log_decision mr collect $args
        return ""
    }

    proc mr_connect_back_port {args} {
        ::itest::log_decision mr connect_back_port $args
        return ""
    }

    proc mr_connection_instance {args} {
        ::itest::log_decision mr connection_instance $args
        return ""
    }

    proc mr_connection_mode {args} {
        ::itest::log_decision mr connection_mode $args
        return ""
    }

    proc mr_equivalent_transport {args} {
        ::itest::log_decision mr equivalent_transport $args
        return ""
    }

    proc mr_flow_id {args} {
        ::itest::log_decision mr flow_id $args
        return ""
    }

    proc mr_ignore_peer_port {args} {
        ::itest::log_decision mr ignore_peer_port $args
        return ""
    }

    proc mr_instance {args} {
        ::itest::log_decision mr instance $args
        return ""
    }

    proc mr_max_retries {args} {
        ::itest::log_decision mr max_retries $args
        return ""
    }

    proc mr_message {args} {
        ::itest::log_decision mr message $args
        return ""
    }

    proc mr_payload {args} {
        ::itest::log_decision mr payload $args
        return ""
    }

    proc mr_peer {args} {
        ::itest::log_decision mr peer $args
        return ""
    }

    proc mr_prime {args} {
        ::itest::log_decision mr prime $args
        return ""
    }

    proc mr_protocol {args} {
        ::itest::log_decision mr protocol $args
        return ""
    }

    proc mr_release {args} {
        ::itest::log_decision mr release $args
        return ""
    }

    proc mr_restore {args} {
        ::itest::log_decision mr restore $args
        return ""
    }

    proc mr_retry {args} {
        ::itest::log_decision mr retry $args
        return ""
    }

    proc mr_return {args} {
        ::itest::log_decision mr return $args
        return ""
    }

    proc mr_store {args} {
        ::itest::log_decision mr store $args
        return ""
    }

    proc mr_stream {args} {
        ::itest::log_decision mr stream $args
        return ""
    }

    proc mr_transport {args} {
        ::itest::log_decision mr transport $args
        return ""
    }


    # NAME:: stubs (2 commands)

    proc name_lookup {args} {
        ::itest::log_decision name lookup $args
        return ""
    }

    proc name_response {args} {
        ::itest::log_decision name response $args
        return ""
    }


    # NSH:: stubs (6 commands)

    proc nsh_chain {args} {
        ::itest::log_decision nsh chain $args
        return ""
    }

    proc nsh_context {args} {
        ::itest::log_decision nsh context $args
        return ""
    }

    proc nsh_md1 {args} {
        ::itest::log_decision nsh md1 $args
        return ""
    }

    proc nsh_mocksf {args} {
        ::itest::log_decision nsh mocksf $args
        return ""
    }

    proc nsh_path_id {args} {
        ::itest::log_decision nsh path_id $args
        return ""
    }

    proc nsh_service_index {args} {
        ::itest::log_decision nsh service_index $args
        return ""
    }


    # NTLM:: stubs (2 commands)

    proc ntlm_disable {args} {
        ::itest::log_decision ntlm disable $args
        return ""
    }

    proc ntlm_enable {args} {
        ::itest::log_decision ntlm enable $args
        return ""
    }


    # OFFBOX:: stubs (1 commands)

    proc offbox_request {args} {
        ::itest::log_decision offbox request $args
        return ""
    }


    # ONECONNECT:: stubs (4 commands)

    proc oneconnect_detach {args} {
        ::itest::log_decision oneconnect detach $args
        return ""
    }

    proc oneconnect_label {args} {
        ::itest::log_decision oneconnect label $args
        return ""
    }

    proc oneconnect_reuse {args} {
        ::itest::log_decision oneconnect reuse $args
        return ""
    }

    proc oneconnect_select {args} {
        ::itest::log_decision oneconnect select $args
        return ""
    }


    # PCP:: stubs (3 commands)

    proc pcp_reject {args} {
        ::itest::log_decision pcp reject $args
        return ""
    }

    proc pcp_request {args} {
        ::itest::log_decision pcp request $args
        return ""
    }

    proc pcp_response {args} {
        ::itest::log_decision pcp response $args
        return ""
    }


    # PEM:: stubs (5 commands)

    proc pem_disable {args} {
        ::itest::log_decision pem disable $args
        return ""
    }

    proc pem_enable {args} {
        ::itest::log_decision pem enable $args
        return ""
    }

    proc pem_flow {args} {
        ::itest::log_decision pem flow $args
        return ""
    }

    proc pem_session {args} {
        ::itest::log_decision pem session $args
        return ""
    }

    proc pem_subscriber {args} {
        ::itest::log_decision pem subscriber $args
        return ""
    }


    # PLUGIN:: stubs (2 commands)

    proc plugin_disable {args} {
        ::itest::log_decision plugin disable $args
        return ""
    }

    proc plugin_enable {args} {
        ::itest::log_decision plugin enable $args
        return ""
    }


    # POLICY:: stubs (4 commands)

    proc policy_controls {args} {
        ::itest::log_decision policy controls $args
        return ""
    }

    proc policy_names {args} {
        ::itest::log_decision policy names $args
        return ""
    }

    proc policy_rules {args} {
        ::itest::log_decision policy rules $args
        return ""
    }

    proc policy_targets {args} {
        ::itest::log_decision policy targets $args
        return ""
    }


    # POP3:: stubs (3 commands)

    proc pop3_activation_mode {args} {
        ::itest::log_decision pop3 activation_mode $args
        return ""
    }

    proc pop3_disable {args} {
        ::itest::log_decision pop3 disable $args
        return ""
    }

    proc pop3_enable {args} {
        ::itest::log_decision pop3 enable $args
        return ""
    }


    # PROFILE:: stubs (25 commands)

    proc profile_access {args} {
        ::itest::log_decision profile access $args
        return ""
    }

    proc profile_antifraud {args} {
        ::itest::log_decision profile antifraud $args
        return ""
    }

    proc profile_auth {args} {
        ::itest::log_decision profile auth $args
        return ""
    }

    proc profile_avr {args} {
        ::itest::log_decision profile avr $args
        return ""
    }

    proc profile_clientssl {args} {
        ::itest::log_decision profile clientssl $args
        return ""
    }

    proc profile_diameter {args} {
        ::itest::log_decision profile diameter $args
        return ""
    }

    proc profile_exchange {args} {
        ::itest::log_decision profile exchange $args
        return ""
    }

    proc profile_exists {args} {
        ::itest::log_decision profile exists $args
        return ""
    }

    proc profile_fastL4 {args} {
        ::itest::log_decision profile fastL4 $args
        return ""
    }

    proc profile_fasthttp {args} {
        ::itest::log_decision profile fasthttp $args
        return ""
    }

    proc profile_ftp {args} {
        ::itest::log_decision profile ftp $args
        return ""
    }

    proc profile_http {args} {
        ::itest::log_decision profile http $args
        return ""
    }

    proc profile_httpclass {args} {
        ::itest::log_decision profile httpclass $args
        return ""
    }

    proc profile_httpcompression {args} {
        ::itest::log_decision profile httpcompression $args
        return ""
    }

    proc profile_list {args} {
        ::itest::log_decision profile list $args
        return ""
    }

    proc profile_oneconnect {args} {
        ::itest::log_decision profile oneconnect $args
        return ""
    }

    proc profile_persist {args} {
        ::itest::log_decision profile persist $args
        return ""
    }

    proc profile_serverssl {args} {
        ::itest::log_decision profile serverssl $args
        return ""
    }

    proc profile_stream {args} {
        ::itest::log_decision profile stream $args
        return ""
    }

    proc profile_tcp {args} {
        ::itest::log_decision profile tcp $args
        return ""
    }

    proc profile_tftp {args} {
        ::itest::log_decision profile tftp $args
        return ""
    }

    proc profile_udp {args} {
        ::itest::log_decision profile udp $args
        return ""
    }

    proc profile_vdi {args} {
        ::itest::log_decision profile vdi $args
        return ""
    }

    proc profile_webacceleration {args} {
        ::itest::log_decision profile webacceleration $args
        return ""
    }

    proc profile_xml {args} {
        ::itest::log_decision profile xml $args
        return ""
    }


    # PROTOCOL_INSPECTION:: stubs (2 commands)

    proc protocol_inspection_disable {args} {
        ::itest::log_decision protocol_inspection disable $args
        return ""
    }

    proc protocol_inspection_id {args} {
        ::itest::log_decision protocol_inspection id $args
        return ""
    }


    # PSC:: stubs (11 commands)

    proc psc_aaa_reporting_interval {args} {
        ::itest::log_decision psc aaa_reporting_interval $args
        return ""
    }

    proc psc_attr {args} {
        ::itest::log_decision psc attr $args
        return ""
    }

    proc psc_calling_id {args} {
        ::itest::log_decision psc calling_id $args
        return ""
    }

    proc psc_imeisv {args} {
        ::itest::log_decision psc imeisv $args
        return ""
    }

    proc psc_imsi {args} {
        ::itest::log_decision psc imsi $args
        return ""
    }

    proc psc_ip_address {args} {
        ::itest::log_decision psc ip_address $args
        return ""
    }

    proc psc_lease_time {args} {
        ::itest::log_decision psc lease_time $args
        return ""
    }

    proc psc_policy {args} {
        ::itest::log_decision psc policy $args
        return ""
    }

    proc psc_subscriber_id {args} {
        ::itest::log_decision psc subscriber_id $args
        return ""
    }

    proc psc_tower_id {args} {
        ::itest::log_decision psc tower_id $args
        return ""
    }

    proc psc_user_name {args} {
        ::itest::log_decision psc user_name $args
        return ""
    }


    # PSM:: stubs (6 commands)

    proc psm_disable {args} {
        ::itest::log_decision psm disable $args
        return ""
    }

    proc psm_enable {args} {
        ::itest::log_decision psm enable $args
        return ""
    }

    proc psm_disable {args} {
        ::itest::log_decision psm disable $args
        return ""
    }

    proc psm_enable {args} {
        ::itest::log_decision psm enable $args
        return ""
    }

    proc psm_disable {args} {
        ::itest::log_decision psm disable $args
        return ""
    }

    proc psm_enable {args} {
        ::itest::log_decision psm enable $args
        return ""
    }


    # QOE:: stubs (3 commands)

    proc qoe_disable {args} {
        ::itest::log_decision qoe disable $args
        return ""
    }

    proc qoe_enable {args} {
        ::itest::log_decision qoe enable $args
        return ""
    }

    proc qoe_video {args} {
        ::itest::log_decision qoe video $args
        return ""
    }


    # RADIUS:: stubs (5 commands)

    proc radius_avp {args} {
        ::itest::log_decision radius avp $args
        return ""
    }

    proc radius_code {args} {
        ::itest::log_decision radius code $args
        return ""
    }

    proc radius_id {args} {
        ::itest::log_decision radius id $args
        return ""
    }

    proc radius_rtdom {args} {
        ::itest::log_decision radius rtdom $args
        return ""
    }

    proc radius_subscriber {args} {
        ::itest::log_decision radius subscriber $args
        return ""
    }


    # RESOLV:: stubs (1 commands)

    proc resolv_lookup {args} {
        ::itest::log_decision resolv lookup $args
        return ""
    }


    # RESOLVER:: stubs (2 commands)

    proc resolver_name_lookup {args} {
        ::itest::log_decision resolver name_lookup $args
        return ""
    }

    proc resolver_summarize {args} {
        ::itest::log_decision resolver summarize $args
        return ""
    }


    # REST:: stubs (1 commands)

    proc rest_send {args} {
        ::itest::log_decision rest send $args
        return ""
    }


    # REWRITE:: stubs (4 commands)

    proc rewrite_disable {args} {
        ::itest::log_decision rewrite disable $args
        return ""
    }

    proc rewrite_enable {args} {
        ::itest::log_decision rewrite enable $args
        return ""
    }

    proc rewrite_payload {args} {
        ::itest::log_decision rewrite payload $args
        return ""
    }

    proc rewrite_post_process {args} {
        ::itest::log_decision rewrite post_process $args
        return ""
    }


    # ROUTE:: stubs (9 commands)

    proc route_age {args} {
        ::itest::log_decision route age $args
        return ""
    }

    proc route_bandwidth {args} {
        ::itest::log_decision route bandwidth $args
        return ""
    }

    proc route_clear {args} {
        ::itest::log_decision route clear $args
        return ""
    }

    proc route_cwnd {args} {
        ::itest::log_decision route cwnd $args
        return ""
    }

    proc route_domain {args} {
        ::itest::log_decision route domain $args
        return ""
    }

    proc route_expiration {args} {
        ::itest::log_decision route expiration $args
        return ""
    }

    proc route_mtu {args} {
        ::itest::log_decision route mtu $args
        return ""
    }

    proc route_rtt {args} {
        ::itest::log_decision route rtt $args
        return ""
    }

    proc route_rttvar {args} {
        ::itest::log_decision route rttvar $args
        return ""
    }


    # RTSP:: stubs (10 commands)

    proc rtsp_collect {args} {
        ::itest::log_decision rtsp collect $args
        return ""
    }

    proc rtsp_header {args} {
        ::itest::log_decision rtsp header $args
        return ""
    }

    proc rtsp_method {args} {
        ::itest::log_decision rtsp method $args
        return ""
    }

    proc rtsp_msg_source {args} {
        ::itest::log_decision rtsp msg_source $args
        return ""
    }

    proc rtsp_payload {args} {
        ::itest::log_decision rtsp payload $args
        return ""
    }

    proc rtsp_release {args} {
        ::itest::log_decision rtsp release $args
        return ""
    }

    proc rtsp_respond {args} {
        ::itest::log_decision rtsp respond $args
        return ""
    }

    proc rtsp_status {args} {
        ::itest::log_decision rtsp status $args
        return ""
    }

    proc rtsp_uri {args} {
        ::itest::log_decision rtsp uri $args
        return ""
    }

    proc rtsp_version {args} {
        ::itest::log_decision rtsp version $args
        return ""
    }


    # SCTP:: stubs (14 commands)

    proc sctp_client_port {args} {
        ::itest::log_decision sctp client_port $args
        return ""
    }

    proc sctp_collect {args} {
        ::itest::log_decision sctp collect $args
        return ""
    }

    proc sctp_local_port {args} {
        ::itest::log_decision sctp local_port $args
        return ""
    }

    proc sctp_mss {args} {
        ::itest::log_decision sctp mss $args
        return ""
    }

    proc sctp_payload {args} {
        ::itest::log_decision sctp payload $args
        return ""
    }

    proc sctp_ppi {args} {
        ::itest::log_decision sctp ppi $args
        return ""
    }

    proc sctp_release {args} {
        ::itest::log_decision sctp release $args
        return ""
    }

    proc sctp_remote_port {args} {
        ::itest::log_decision sctp remote_port $args
        return ""
    }

    proc sctp_respond {args} {
        ::itest::log_decision sctp respond $args
        return ""
    }

    proc sctp_rto_initial {args} {
        ::itest::log_decision sctp rto_initial $args
        return ""
    }

    proc sctp_rto_max {args} {
        ::itest::log_decision sctp rto_max $args
        return ""
    }

    proc sctp_rto_min {args} {
        ::itest::log_decision sctp rto_min $args
        return ""
    }

    proc sctp_sack_timeout {args} {
        ::itest::log_decision sctp sack_timeout $args
        return ""
    }

    proc sctp_server_port {args} {
        ::itest::log_decision sctp server_port $args
        return ""
    }


    # SDP:: stubs (3 commands)

    proc sdp_field {args} {
        ::itest::log_decision sdp field $args
        return ""
    }

    proc sdp_media {args} {
        ::itest::log_decision sdp media $args
        return ""
    }

    proc sdp_session_id {args} {
        ::itest::log_decision sdp session_id $args
        return ""
    }


    # SIP:: stubs (16 commands)

    proc sip_call_id {args} {
        ::itest::log_decision sip call_id $args
        return ""
    }

    proc sip_discard {args} {
        ::itest::log_decision sip discard $args
        return ""
    }

    proc sip_from {args} {
        ::itest::log_decision sip from $args
        return ""
    }

    proc sip_header {args} {
        ::itest::log_decision sip header $args
        return ""
    }

    proc sip_message {args} {
        ::itest::log_decision sip message $args
        return ""
    }

    proc sip_method {args} {
        ::itest::log_decision sip method $args
        return ""
    }

    proc sip_payload {args} {
        ::itest::log_decision sip payload $args
        return ""
    }

    proc sip_persist {args} {
        ::itest::log_decision sip persist $args
        return ""
    }

    proc sip_record_route {args} {
        ::itest::log_decision sip record-route $args
        return ""
    }

    proc sip_respond {args} {
        ::itest::log_decision sip respond $args
        return ""
    }

    proc sip_response {args} {
        ::itest::log_decision sip response $args
        return ""
    }

    proc sip_route {args} {
        ::itest::log_decision sip route $args
        return ""
    }

    proc sip_route_status {args} {
        ::itest::log_decision sip route_status $args
        return ""
    }

    proc sip_to {args} {
        ::itest::log_decision sip to $args
        return ""
    }

    proc sip_uri {args} {
        ::itest::log_decision sip uri $args
        return ""
    }

    proc sip_via {args} {
        ::itest::log_decision sip via $args
        return ""
    }


    # SIPALG:: stubs (3 commands)

    proc sipalg_hairpin {args} {
        ::itest::log_decision sipalg hairpin $args
        return ""
    }

    proc sipalg_hairpin_default {args} {
        ::itest::log_decision sipalg hairpin_default $args
        return ""
    }

    proc sipalg_nonregister_subscriber_listener {args} {
        ::itest::log_decision sipalg nonregister_subscriber_listener $args
        return ""
    }


    # SMTPS:: stubs (3 commands)

    proc smtps_activation_mode {args} {
        ::itest::log_decision smtps activation_mode $args
        return ""
    }

    proc smtps_disable {args} {
        ::itest::log_decision smtps disable $args
        return ""
    }

    proc smtps_enable {args} {
        ::itest::log_decision smtps enable $args
        return ""
    }


    # SOCKS:: stubs (3 commands)

    proc socks_allowed {args} {
        ::itest::log_decision socks allowed $args
        return ""
    }

    proc socks_destination {args} {
        ::itest::log_decision socks destination $args
        return ""
    }

    proc socks_version {args} {
        ::itest::log_decision socks version $args
        return ""
    }


    # SSE:: stubs (1 commands)

    proc sse_field {args} {
        ::itest::log_decision sse field $args
        return ""
    }


    # SSL:: stubs (26 commands)

    proc ssl_allow_dynamic_record_sizing {args} {
        ::itest::log_decision ssl allow_dynamic_record_sizing $args
        return ""
    }

    proc ssl_allow_nonssl {args} {
        ::itest::log_decision ssl allow_nonssl $args
        return ""
    }

    proc ssl_alpn {args} {
        ::itest::log_decision ssl alpn $args
        return ""
    }

    proc ssl_authenticate {args} {
        ::itest::log_decision ssl authenticate $args
        return ""
    }

    proc ssl_c3d {args} {
        ::itest::log_decision ssl c3d $args
        return ""
    }

    proc ssl_cert_constraint {args} {
        ::itest::log_decision ssl cert_constraint $args
        return ""
    }

    proc ssl_clientrandom {args} {
        ::itest::log_decision ssl clientrandom $args
        return ""
    }

    proc ssl_collect {args} {
        ::itest::log_decision ssl collect $args
        return ""
    }

    proc ssl_forward_proxy {args} {
        ::itest::log_decision ssl forward_proxy $args
        return ""
    }

    proc ssl_handshake {args} {
        ::itest::log_decision ssl handshake $args
        return ""
    }

    proc ssl_is_renegotiation_secure {args} {
        ::itest::log_decision ssl is_renegotiation_secure $args
        return ""
    }

    proc ssl_maximum_record_size {args} {
        ::itest::log_decision ssl maximum_record_size $args
        return ""
    }

    proc ssl_mode {args} {
        ::itest::log_decision ssl mode $args
        return ""
    }

    proc ssl_modssl_sessionid_headers {args} {
        ::itest::log_decision ssl modssl_sessionid_headers $args
        return ""
    }

    proc ssl_nextproto {args} {
        ::itest::log_decision ssl nextproto $args
        return ""
    }

    proc ssl_payload {args} {
        ::itest::log_decision ssl payload $args
        return ""
    }

    proc ssl_profile {args} {
        ::itest::log_decision ssl profile $args
        return ""
    }

    proc ssl_release {args} {
        ::itest::log_decision ssl release $args
        return ""
    }

    proc ssl_renegotiate {args} {
        ::itest::log_decision ssl renegotiate $args
        return ""
    }

    proc ssl_secure_renegotiation {args} {
        ::itest::log_decision ssl secure_renegotiation $args
        return ""
    }

    proc ssl_session {args} {
        ::itest::log_decision ssl session $args
        return ""
    }

    proc ssl_sessionsecret {args} {
        ::itest::log_decision ssl sessionsecret $args
        return ""
    }

    proc ssl_sessionticket {args} {
        ::itest::log_decision ssl sessionticket $args
        return ""
    }

    proc ssl_tls13_secret {args} {
        ::itest::log_decision ssl tls13_secret $args
        return ""
    }

    proc ssl_unclean_shutdown {args} {
        ::itest::log_decision ssl unclean_shutdown $args
        return ""
    }

    proc ssl_verify_result {args} {
        ::itest::log_decision ssl verify_result $args
        return ""
    }


    # STATS:: stubs (5 commands)

    proc stats_get {args} {
        ::itest::log_decision stats get $args
        return ""
    }

    proc stats_incr {args} {
        ::itest::log_decision stats incr $args
        return ""
    }

    proc stats_set {args} {
        ::itest::log_decision stats set $args
        return ""
    }

    proc stats_setmax {args} {
        ::itest::log_decision stats setmax $args
        return ""
    }

    proc stats_setmin {args} {
        ::itest::log_decision stats setmin $args
        return ""
    }


    # STREAM:: stubs (7 commands)

    proc stream_disable {args} {
        ::itest::log_decision stream disable $args
        return ""
    }

    proc stream_enable {args} {
        ::itest::log_decision stream enable $args
        return ""
    }

    proc stream_encoding {args} {
        ::itest::log_decision stream encoding $args
        return ""
    }

    proc stream_expression {args} {
        ::itest::log_decision stream expression $args
        return ""
    }

    proc stream_match {args} {
        ::itest::log_decision stream match $args
        return ""
    }

    proc stream_max_matchsize {args} {
        ::itest::log_decision stream max_matchsize $args
        return ""
    }

    proc stream_replace {args} {
        ::itest::log_decision stream replace $args
        return ""
    }


    # TAP:: stubs (5 commands)

    proc tap_action {args} {
        ::itest::log_decision tap action $args
        return ""
    }

    proc tap_config {args} {
        ::itest::log_decision tap config $args
        return ""
    }

    proc tap_insight {args} {
        ::itest::log_decision tap insight $args
        return ""
    }

    proc tap_insight_requested {args} {
        ::itest::log_decision tap insight_requested $args
        return ""
    }

    proc tap_score {args} {
        ::itest::log_decision tap score $args
        return ""
    }


    # TCP:: stubs (40 commands)

    proc tcp_abc {args} {
        ::itest::log_decision tcp abc $args
        return ""
    }

    proc tcp_analytics {args} {
        ::itest::log_decision tcp analytics $args
        return ""
    }

    proc tcp_autowin {args} {
        ::itest::log_decision tcp autowin $args
        return ""
    }

    proc tcp_congestion {args} {
        ::itest::log_decision tcp congestion $args
        return ""
    }

    proc tcp_delayed_ack {args} {
        ::itest::log_decision tcp delayed_ack $args
        return ""
    }

    proc tcp_dsack {args} {
        ::itest::log_decision tcp dsack $args
        return ""
    }

    proc tcp_earlyrxmit {args} {
        ::itest::log_decision tcp earlyrxmit $args
        return ""
    }

    proc tcp_ecn {args} {
        ::itest::log_decision tcp ecn $args
        return ""
    }

    proc tcp_enhanced_loss_recovery {args} {
        ::itest::log_decision tcp enhanced_loss_recovery $args
        return ""
    }

    proc tcp_idletime {args} {
        ::itest::log_decision tcp idletime $args
        return ""
    }

    proc tcp_keepalive {args} {
        ::itest::log_decision tcp keepalive $args
        return ""
    }

    proc tcp_limxmit {args} {
        ::itest::log_decision tcp limxmit $args
        return ""
    }

    proc tcp_lossfilter {args} {
        ::itest::log_decision tcp lossfilter $args
        return ""
    }

    proc tcp_lossfilterburst {args} {
        ::itest::log_decision tcp lossfilterburst $args
        return ""
    }

    proc tcp_lossfilterrate {args} {
        ::itest::log_decision tcp lossfilterrate $args
        return ""
    }

    proc tcp_nagle {args} {
        ::itest::log_decision tcp nagle $args
        return ""
    }

    proc tcp_naglemode {args} {
        ::itest::log_decision tcp naglemode $args
        return ""
    }

    proc tcp_naglestate {args} {
        ::itest::log_decision tcp naglestate $args
        return ""
    }

    proc tcp_notify {args} {
        ::itest::log_decision tcp notify $args
        return ""
    }

    proc tcp_offset {args} {
        ::itest::log_decision tcp offset $args
        return ""
    }

    proc tcp_option {args} {
        ::itest::log_decision tcp option $args
        return ""
    }

    proc tcp_pacing {args} {
        ::itest::log_decision tcp pacing $args
        return ""
    }

    proc tcp_proxybuffer {args} {
        ::itest::log_decision tcp proxybuffer $args
        return ""
    }

    proc tcp_proxybufferhigh {args} {
        ::itest::log_decision tcp proxybufferhigh $args
        return ""
    }

    proc tcp_proxybufferlow {args} {
        ::itest::log_decision tcp proxybufferlow $args
        return ""
    }

    proc tcp_push_flag {args} {
        ::itest::log_decision tcp push_flag $args
        return ""
    }

    proc tcp_rcv_scale {args} {
        ::itest::log_decision tcp rcv_scale $args
        return ""
    }

    proc tcp_rcv_size {args} {
        ::itest::log_decision tcp rcv_size $args
        return ""
    }

    proc tcp_recvwnd {args} {
        ::itest::log_decision tcp recvwnd $args
        return ""
    }

    proc tcp_rexmt_thresh {args} {
        ::itest::log_decision tcp rexmt_thresh $args
        return ""
    }

    proc tcp_rt_metrics_timeout {args} {
        ::itest::log_decision tcp rt_metrics_timeout $args
        return ""
    }

    proc tcp_rto {args} {
        ::itest::log_decision tcp rto $args
        return ""
    }

    proc tcp_rttvar {args} {
        ::itest::log_decision tcp rttvar $args
        return ""
    }

    proc tcp_sendbuf {args} {
        ::itest::log_decision tcp sendbuf $args
        return ""
    }

    proc tcp_setmss {args} {
        ::itest::log_decision tcp setmss $args
        return ""
    }

    proc tcp_snd_cwnd {args} {
        ::itest::log_decision tcp snd_cwnd $args
        return ""
    }

    proc tcp_snd_scale {args} {
        ::itest::log_decision tcp snd_scale $args
        return ""
    }

    proc tcp_snd_ssthresh {args} {
        ::itest::log_decision tcp snd_ssthresh $args
        return ""
    }

    proc tcp_snd_wnd {args} {
        ::itest::log_decision tcp snd_wnd $args
        return ""
    }

    proc tcp_unused_port {args} {
        ::itest::log_decision tcp unused_port $args
        return ""
    }


    # TDS:: stubs (2 commands)

    proc tds_msg {args} {
        ::itest::log_decision tds msg $args
        return ""
    }

    proc tds_session {args} {
        ::itest::log_decision tds session $args
        return ""
    }


    # TMM:: stubs (5 commands)

    proc tmm_cmp_count {args} {
        ::itest::log_decision tmm cmp_count $args
        return ""
    }

    proc tmm_cmp_group {args} {
        ::itest::log_decision tmm cmp_group $args
        return ""
    }

    proc tmm_cmp_groups {args} {
        ::itest::log_decision tmm cmp_groups $args
        return ""
    }

    proc tmm_cmp_primary_group {args} {
        ::itest::log_decision tmm cmp_primary_group $args
        return ""
    }

    proc tmm_cmp_unit {args} {
        ::itest::log_decision tmm cmp_unit $args
        return ""
    }


    # UDP:: stubs (15 commands)

    proc udp_client_port {args} {
        ::itest::log_decision udp client_port $args
        return ""
    }

    proc udp_debug_queue {args} {
        ::itest::log_decision udp debug_queue $args
        return ""
    }

    proc udp_drop {args} {
        ::itest::log_decision udp drop $args
        return ""
    }

    proc udp_hold {args} {
        ::itest::log_decision udp hold $args
        return ""
    }

    proc udp_local_port {args} {
        ::itest::log_decision udp local_port $args
        return ""
    }

    proc udp_max_buf_pkts {args} {
        ::itest::log_decision udp max_buf_pkts $args
        return ""
    }

    proc udp_max_rate {args} {
        ::itest::log_decision udp max_rate $args
        return ""
    }

    proc udp_mss {args} {
        ::itest::log_decision udp mss $args
        return ""
    }

    proc udp_payload {args} {
        ::itest::log_decision udp payload $args
        return ""
    }

    proc udp_release {args} {
        ::itest::log_decision udp release $args
        return ""
    }

    proc udp_remote_port {args} {
        ::itest::log_decision udp remote_port $args
        return ""
    }

    proc udp_respond {args} {
        ::itest::log_decision udp respond $args
        return ""
    }

    proc udp_sendbuffer {args} {
        ::itest::log_decision udp sendbuffer $args
        return ""
    }

    proc udp_server_port {args} {
        ::itest::log_decision udp server_port $args
        return ""
    }

    proc udp_unused_port {args} {
        ::itest::log_decision udp unused_port $args
        return ""
    }


    # URI:: stubs (9 commands)

    proc uri_basename {args} {
        ::itest::log_decision uri basename $args
        return ""
    }

    proc uri_compare {args} {
        ::itest::log_decision uri compare $args
        return ""
    }

    proc uri_decode {args} {
        ::itest::log_decision uri decode $args
        return ""
    }

    proc uri_encode {args} {
        ::itest::log_decision uri encode $args
        return ""
    }

    proc uri_host {args} {
        ::itest::log_decision uri host $args
        return ""
    }

    proc uri_path {args} {
        ::itest::log_decision uri path $args
        return ""
    }

    proc uri_port {args} {
        ::itest::log_decision uri port $args
        return ""
    }

    proc uri_protocol {args} {
        ::itest::log_decision uri protocol $args
        return ""
    }

    proc uri_query {args} {
        ::itest::log_decision uri query $args
        return ""
    }


    # VALIDATE:: stubs (1 commands)

    proc validate_protocol {args} {
        ::itest::log_decision validate protocol $args
        return ""
    }


    # VDI:: stubs (2 commands)

    proc vdi_disable {args} {
        ::itest::log_decision vdi disable $args
        return ""
    }

    proc vdi_enable {args} {
        ::itest::log_decision vdi enable $args
        return ""
    }


    # WAM:: stubs (2 commands)

    proc wam_disable {args} {
        ::itest::log_decision wam disable $args
        return ""
    }

    proc wam_enable {args} {
        ::itest::log_decision wam enable $args
        return ""
    }


    # WEBSSO:: stubs (3 commands)

    proc websso_disable {args} {
        ::itest::log_decision websso disable $args
        return ""
    }

    proc websso_enable {args} {
        ::itest::log_decision websso enable $args
        return ""
    }

    proc websso_select {args} {
        ::itest::log_decision websso select $args
        return ""
    }


    # WS:: stubs (12 commands)

    proc ws_collect {args} {
        ::itest::log_decision ws collect $args
        return ""
    }

    proc ws_disconnect {args} {
        ::itest::log_decision ws disconnect $args
        return ""
    }

    proc ws_enabled {args} {
        ::itest::log_decision ws enabled $args
        return ""
    }

    proc ws_frame {args} {
        ::itest::log_decision ws frame $args
        return ""
    }

    proc ws_masking {args} {
        ::itest::log_decision ws masking $args
        return ""
    }

    proc ws_message {args} {
        ::itest::log_decision ws message $args
        return ""
    }

    proc ws_payload {args} {
        ::itest::log_decision ws payload $args
        return ""
    }

    proc ws_payload_ivs {args} {
        ::itest::log_decision ws payload_ivs $args
        return ""
    }

    proc ws_payload_processing {args} {
        ::itest::log_decision ws payload_processing $args
        return ""
    }

    proc ws_release {args} {
        ::itest::log_decision ws release $args
        return ""
    }

    proc ws_request {args} {
        ::itest::log_decision ws request $args
        return ""
    }

    proc ws_response {args} {
        ::itest::log_decision ws response $args
        return ""
    }


    # X509:: stubs (16 commands)

    proc x509_cert_fields {args} {
        ::itest::log_decision x509 cert_fields $args
        return ""
    }

    proc x509_extensions {args} {
        ::itest::log_decision x509 extensions $args
        return ""
    }

    proc x509_hash {args} {
        ::itest::log_decision x509 hash $args
        return ""
    }

    proc x509_issuer {args} {
        ::itest::log_decision x509 issuer $args
        return ""
    }

    proc x509_not_valid_after {args} {
        ::itest::log_decision x509 not_valid_after $args
        return ""
    }

    proc x509_not_valid_before {args} {
        ::itest::log_decision x509 not_valid_before $args
        return ""
    }

    proc x509_pem2der {args} {
        ::itest::log_decision x509 pem2der $args
        return ""
    }

    proc x509_serial_number {args} {
        ::itest::log_decision x509 serial_number $args
        return ""
    }

    proc x509_signature_algorithm {args} {
        ::itest::log_decision x509 signature_algorithm $args
        return ""
    }

    proc x509_subject {args} {
        ::itest::log_decision x509 subject $args
        return ""
    }

    proc x509_subject_public_key {args} {
        ::itest::log_decision x509 subject_public_key $args
        return ""
    }

    proc x509_subject_public_key_RSA_bits {args} {
        ::itest::log_decision x509 subject_public_key_RSA_bits $args
        return ""
    }

    proc x509_subject_public_key_type {args} {
        ::itest::log_decision x509 subject_public_key_type $args
        return ""
    }

    proc x509_verify_cert_error_string {args} {
        ::itest::log_decision x509 verify_cert_error_string $args
        return ""
    }

    proc x509_version {args} {
        ::itest::log_decision x509 version $args
        return ""
    }

    proc x509_whole {args} {
        ::itest::log_decision x509 whole $args
        return ""
    }


    # XLAT:: stubs (7 commands)

    proc xlat_listen {args} {
        ::itest::log_decision xlat listen $args
        return ""
    }

    proc xlat_listen_lifetime {args} {
        ::itest::log_decision xlat listen_lifetime $args
        return ""
    }

    proc xlat_src_addr {args} {
        ::itest::log_decision xlat src_addr $args
        return ""
    }

    proc xlat_src_config {args} {
        ::itest::log_decision xlat src_config $args
        return ""
    }

    proc xlat_src_endpoint_reservation {args} {
        ::itest::log_decision xlat src_endpoint_reservation $args
        return ""
    }

    proc xlat_src_nat_valid_range {args} {
        ::itest::log_decision xlat src_nat_valid_range $args
        return ""
    }

    proc xlat_src_port {args} {
        ::itest::log_decision xlat src_port $args
        return ""
    }


    # XML:: stubs (12 commands)

    proc xml_address {args} {
        ::itest::log_decision xml address $args
        return ""
    }

    proc xml_collect {args} {
        ::itest::log_decision xml collect $args
        return ""
    }

    proc xml_disable {args} {
        ::itest::log_decision xml disable $args
        return ""
    }

    proc xml_element {args} {
        ::itest::log_decision xml element $args
        return ""
    }

    proc xml_enable {args} {
        ::itest::log_decision xml enable $args
        return ""
    }

    proc xml_event {args} {
        ::itest::log_decision xml event $args
        return ""
    }

    proc xml_eventid {args} {
        ::itest::log_decision xml eventid $args
        return ""
    }

    proc xml_parse {args} {
        ::itest::log_decision xml parse $args
        return ""
    }

    proc xml_payload {args} {
        ::itest::log_decision xml payload $args
        return ""
    }

    proc xml_release {args} {
        ::itest::log_decision xml release $args
        return ""
    }

    proc xml_soap {args} {
        ::itest::log_decision xml soap $args
        return ""
    }

    proc xml_subscribe {args} {
        ::itest::log_decision xml subscribe $args
        return ""
    }


    # base64:: stubs (2 commands)

    proc base64_decode {args} {
        ::itest::log_decision base64 decode $args
        return ""
    }

    proc base64_encode {args} {
        ::itest::log_decision base64 encode $args
        return ""
    }


    # cmdline:: stubs (3 commands)

    proc cmdline_getopt {args} {
        ::itest::log_decision cmdline getopt $args
        return ""
    }

    proc cmdline_getoptions {args} {
        ::itest::log_decision cmdline getoptions $args
        return ""
    }

    proc cmdline_usage {args} {
        ::itest::log_decision cmdline usage $args
        return ""
    }


    # csv:: stubs (4 commands)

    proc csv_join {args} {
        ::itest::log_decision csv join $args
        return ""
    }

    proc csv_read {args} {
        ::itest::log_decision csv read $args
        return ""
    }

    proc csv_report {args} {
        ::itest::log_decision csv report $args
        return ""
    }

    proc csv_split {args} {
        ::itest::log_decision csv split $args
        return ""
    }


    # dns:: stubs (4 commands)

    proc dns_address {args} {
        ::itest::log_decision dns address $args
        return ""
    }

    proc dns_cleanup {args} {
        ::itest::log_decision dns cleanup $args
        return ""
    }

    proc dns_name {args} {
        ::itest::log_decision dns name $args
        return ""
    }

    proc dns_resolve {args} {
        ::itest::log_decision dns resolve $args
        return ""
    }


    # fileutil:: stubs (4 commands)

    proc fileutil_cat {args} {
        ::itest::log_decision fileutil cat $args
        return ""
    }

    proc fileutil_tempdir {args} {
        ::itest::log_decision fileutil tempdir $args
        return ""
    }

    proc fileutil_tempfile {args} {
        ::itest::log_decision fileutil tempfile $args
        return ""
    }

    proc fileutil_writeFile {args} {
        ::itest::log_decision fileutil writeFile $args
        return ""
    }


    # html:: stubs (2 commands)

    proc html_html_entities {args} {
        ::itest::log_decision html html_entities $args
        return ""
    }

    proc html_tagstrip {args} {
        ::itest::log_decision html tagstrip $args
        return ""
    }


    # http:: stubs (18 commands)

    proc http_cleanup {args} {
        ::itest::log_decision http cleanup $args
        return ""
    }

    proc http_code {args} {
        ::itest::log_decision http code $args
        return ""
    }

    proc http_config {args} {
        ::itest::log_decision http config $args
        return ""
    }

    proc http_cookiejar {args} {
        ::itest::log_decision http cookiejar $args
        return ""
    }

    proc http_IDNAdecode {args} {
        ::itest::log_decision http IDNAdecode $args
        return ""
    }

    proc http_IDNAencode {args} {
        ::itest::log_decision http IDNAencode $args
        return ""
    }

    proc http_data {args} {
        ::itest::log_decision http data $args
        return ""
    }

    proc http_error {args} {
        ::itest::log_decision http error $args
        return ""
    }

    proc http_formatQuery {args} {
        ::itest::log_decision http formatQuery $args
        return ""
    }

    proc http_geturl {args} {
        ::itest::log_decision http geturl $args
        return ""
    }

    proc http_meta {args} {
        ::itest::log_decision http meta $args
        return ""
    }

    proc http_ncode {args} {
        ::itest::log_decision http ncode $args
        return ""
    }

    proc http_quoteString {args} {
        ::itest::log_decision http quoteString $args
        return ""
    }

    proc http_register {args} {
        ::itest::log_decision http register $args
        return ""
    }

    proc http_reset {args} {
        ::itest::log_decision http reset $args
        return ""
    }

    proc http_size {args} {
        ::itest::log_decision http size $args
        return ""
    }

    proc http_unregister {args} {
        ::itest::log_decision http unregister $args
        return ""
    }

    proc http_wait {args} {
        ::itest::log_decision http wait $args
        return ""
    }


    # ip:: stubs (5 commands)

    proc ip_contract {args} {
        ::itest::log_decision ip contract $args
        return ""
    }

    proc ip_equal {args} {
        ::itest::log_decision ip equal $args
        return ""
    }

    proc ip_normalize {args} {
        ::itest::log_decision ip normalize $args
        return ""
    }

    proc ip_prefix {args} {
        ::itest::log_decision ip prefix $args
        return ""
    }

    proc ip_version {args} {
        ::itest::log_decision ip version $args
        return ""
    }


    # json:: stubs (2 commands)

    proc json_dict2json {args} {
        ::itest::log_decision json dict2json $args
        return ""
    }

    proc json_json2dict {args} {
        ::itest::log_decision json json2dict $args
        return ""
    }


    # logger:: stubs (4 commands)

    proc logger_init {args} {
        ::itest::log_decision logger init $args
        return ""
    }

    proc logger_levels {args} {
        ::itest::log_decision logger levels $args
        return ""
    }

    proc logger_servicecmd {args} {
        ::itest::log_decision logger servicecmd $args
        return ""
    }

    proc logger_services {args} {
        ::itest::log_decision logger services $args
        return ""
    }


    # math:: stubs (6 commands)

    proc math_basic_stats {args} {
        ::itest::log_decision math basic-stats $args
        return ""
    }

    proc math_mean {args} {
        ::itest::log_decision math mean $args
        return ""
    }

    proc math_median {args} {
        ::itest::log_decision math median $args
        return ""
    }

    proc math_quantiles {args} {
        ::itest::log_decision math quantiles $args
        return ""
    }

    proc math_stdev {args} {
        ::itest::log_decision math stdev $args
        return ""
    }

    proc math_var {args} {
        ::itest::log_decision math var $args
        return ""
    }


    # md5:: stubs (1 commands)

    proc md5_md5 {args} {
        ::itest::log_decision md5 md5 $args
        return ""
    }


    # mime:: stubs (4 commands)

    proc mime_finalize {args} {
        ::itest::log_decision mime finalize $args
        return ""
    }

    proc mime_getbody {args} {
        ::itest::log_decision mime getbody $args
        return ""
    }

    proc mime_getproperty {args} {
        ::itest::log_decision mime getproperty $args
        return ""
    }

    proc mime_initialize {args} {
        ::itest::log_decision mime initialize $args
        return ""
    }


    # msgcat:: stubs (16 commands)

    proc msgcat_mc {args} {
        ::itest::log_decision msgcat mc $args
        return ""
    }

    proc msgcat_mcexists {args} {
        ::itest::log_decision msgcat mcexists $args
        return ""
    }

    proc msgcat_mcflmset {args} {
        ::itest::log_decision msgcat mcflmset $args
        return ""
    }

    proc msgcat_mcflset {args} {
        ::itest::log_decision msgcat mcflset $args
        return ""
    }

    proc msgcat_mcforgetpackage {args} {
        ::itest::log_decision msgcat mcforgetpackage $args
        return ""
    }

    proc msgcat_mcload {args} {
        ::itest::log_decision msgcat mcload $args
        return ""
    }

    proc msgcat_mcloadedlocales {args} {
        ::itest::log_decision msgcat mcloadedlocales $args
        return ""
    }

    proc msgcat_mclocale {args} {
        ::itest::log_decision msgcat mclocale $args
        return ""
    }

    proc msgcat_mcmax {args} {
        ::itest::log_decision msgcat mcmax $args
        return ""
    }

    proc msgcat_mcmset {args} {
        ::itest::log_decision msgcat mcmset $args
        return ""
    }

    proc msgcat_mcn {args} {
        ::itest::log_decision msgcat mcn $args
        return ""
    }

    proc msgcat_mcpackageconfig {args} {
        ::itest::log_decision msgcat mcpackageconfig $args
        return ""
    }

    proc msgcat_mcpackagelocale {args} {
        ::itest::log_decision msgcat mcpackagelocale $args
        return ""
    }

    proc msgcat_mcpreferences {args} {
        ::itest::log_decision msgcat mcpreferences $args
        return ""
    }

    proc msgcat_mcset {args} {
        ::itest::log_decision msgcat mcset $args
        return ""
    }

    proc msgcat_mcunknown {args} {
        ::itest::log_decision msgcat mcunknown $args
        return ""
    }


    # pkg:: stubs (1 commands)

    proc pkg_create {args} {
        ::itest::log_decision pkg create $args
        return ""
    }


    # platform:: stubs (5 commands)

    proc platform_generic {args} {
        ::itest::log_decision platform generic $args
        return ""
    }

    proc platform_identify {args} {
        ::itest::log_decision platform identify $args
        return ""
    }

    proc platform_patterns {args} {
        ::itest::log_decision platform patterns $args
        return ""
    }

    proc platform_generic {args} {
        ::itest::log_decision platform generic $args
        return ""
    }

    proc platform_identify {args} {
        ::itest::log_decision platform identify $args
        return ""
    }


    # safe:: stubs (8 commands)

    proc safe_interpAddToAccessPath {args} {
        ::itest::log_decision safe interpAddToAccessPath $args
        return ""
    }

    proc safe_interpConfigure {args} {
        ::itest::log_decision safe interpConfigure $args
        return ""
    }

    proc safe_interpCreate {args} {
        ::itest::log_decision safe interpCreate $args
        return ""
    }

    proc safe_interpDelete {args} {
        ::itest::log_decision safe interpDelete $args
        return ""
    }

    proc safe_interpFindInAccessPath {args} {
        ::itest::log_decision safe interpFindInAccessPath $args
        return ""
    }

    proc safe_interpInit {args} {
        ::itest::log_decision safe interpInit $args
        return ""
    }

    proc safe_setLogCmd {args} {
        ::itest::log_decision safe setLogCmd $args
        return ""
    }

    proc safe_setSyncMode {args} {
        ::itest::log_decision safe setSyncMode $args
        return ""
    }


    # sha1:: stubs (1 commands)

    proc sha1_sha1 {args} {
        ::itest::log_decision sha1 sha1 $args
        return ""
    }


    # sha2:: stubs (1 commands)

    proc sha2_sha256 {args} {
        ::itest::log_decision sha2 sha256 $args
        return ""
    }


    # smtp:: stubs (1 commands)

    proc smtp_sendmessage {args} {
        ::itest::log_decision smtp sendmessage $args
        return ""
    }


    # snit:: stubs (5 commands)

    proc snit_method {args} {
        ::itest::log_decision snit method $args
        return ""
    }

    proc snit_type {args} {
        ::itest::log_decision snit type $args
        return ""
    }

    proc snit_typemethod {args} {
        ::itest::log_decision snit typemethod $args
        return ""
    }

    proc snit_widget {args} {
        ::itest::log_decision snit widget $args
        return ""
    }

    proc snit_widgetadaptor {args} {
        ::itest::log_decision snit widgetadaptor $args
        return ""
    }


    # struct:: stubs (4 commands)

    proc struct_list {args} {
        ::itest::log_decision struct list $args
        return ""
    }

    proc struct_queue {args} {
        ::itest::log_decision struct queue $args
        return ""
    }

    proc struct_set {args} {
        ::itest::log_decision struct set $args
        return ""
    }

    proc struct_stack {args} {
        ::itest::log_decision struct stack $args
        return ""
    }


    # tcl:: stubs (9 commands)

    proc tcl_OptKeyDelete {args} {
        ::itest::log_decision tcl OptKeyDelete $args
        return ""
    }

    proc tcl_OptKeyError {args} {
        ::itest::log_decision tcl OptKeyError $args
        return ""
    }

    proc tcl_OptKeyParse {args} {
        ::itest::log_decision tcl OptKeyParse $args
        return ""
    }

    proc tcl_OptKeyRegister {args} {
        ::itest::log_decision tcl OptKeyRegister $args
        return ""
    }

    proc tcl_OptParse {args} {
        ::itest::log_decision tcl OptParse $args
        return ""
    }

    proc tcl_OptProc {args} {
        ::itest::log_decision tcl OptProc $args
        return ""
    }

    proc tcl_OptProcArgGiven {args} {
        ::itest::log_decision tcl OptProcArgGiven $args
        return ""
    }

    proc tcl_path {args} {
        ::itest::log_decision tcl path $args
        return ""
    }

    proc tcl_roots {args} {
        ::itest::log_decision tcl roots $args
        return ""
    }


    # tcltest:: stubs (15 commands)

    proc tcltest_cleanupTests {args} {
        ::itest::log_decision tcltest cleanupTests $args
        return ""
    }

    proc tcltest_configure {args} {
        ::itest::log_decision tcltest configure $args
        return ""
    }

    proc tcltest_customMatch {args} {
        ::itest::log_decision tcltest customMatch $args
        return ""
    }

    proc tcltest_errorChannel {args} {
        ::itest::log_decision tcltest errorChannel $args
        return ""
    }

    proc tcltest_interpreter {args} {
        ::itest::log_decision tcltest interpreter $args
        return ""
    }

    proc tcltest_loadTestedCommands {args} {
        ::itest::log_decision tcltest loadTestedCommands $args
        return ""
    }

    proc tcltest_makeDirectory {args} {
        ::itest::log_decision tcltest makeDirectory $args
        return ""
    }

    proc tcltest_makeFile {args} {
        ::itest::log_decision tcltest makeFile $args
        return ""
    }

    proc tcltest_outputChannel {args} {
        ::itest::log_decision tcltest outputChannel $args
        return ""
    }

    proc tcltest_removeDirectory {args} {
        ::itest::log_decision tcltest removeDirectory $args
        return ""
    }

    proc tcltest_removeFile {args} {
        ::itest::log_decision tcltest removeFile $args
        return ""
    }

    proc tcltest_runAllTests {args} {
        ::itest::log_decision tcltest runAllTests $args
        return ""
    }

    proc tcltest_test {args} {
        ::itest::log_decision tcltest test $args
        return ""
    }

    proc tcltest_testConstraint {args} {
        ::itest::log_decision tcltest testConstraint $args
        return ""
    }

    proc tcltest_viewFile {args} {
        ::itest::log_decision tcltest viewFile $args
        return ""
    }


    # textutil:: stubs (5 commands)

    proc textutil_adjust {args} {
        ::itest::log_decision textutil adjust $args
        return ""
    }

    proc textutil_indent {args} {
        ::itest::log_decision textutil indent $args
        return ""
    }

    proc textutil_splitx {args} {
        ::itest::log_decision textutil splitx $args
        return ""
    }

    proc textutil_trim {args} {
        ::itest::log_decision textutil trim $args
        return ""
    }

    proc textutil_undent {args} {
        ::itest::log_decision textutil undent $args
        return ""
    }


    # ttk:: stubs (12 commands)

    proc ttk_button {args} {
        ::itest::log_decision ttk button $args
        return ""
    }

    proc ttk_combobox {args} {
        ::itest::log_decision ttk combobox $args
        return ""
    }

    proc ttk_entry {args} {
        ::itest::log_decision ttk entry $args
        return ""
    }

    proc ttk_frame {args} {
        ::itest::log_decision ttk frame $args
        return ""
    }

    proc ttk_label {args} {
        ::itest::log_decision ttk label $args
        return ""
    }

    proc ttk_notebook {args} {
        ::itest::log_decision ttk notebook $args
        return ""
    }

    proc ttk_progressbar {args} {
        ::itest::log_decision ttk progressbar $args
        return ""
    }

    proc ttk_scale {args} {
        ::itest::log_decision ttk scale $args
        return ""
    }

    proc ttk_separator {args} {
        ::itest::log_decision ttk separator $args
        return ""
    }

    proc ttk_sizegrip {args} {
        ::itest::log_decision ttk sizegrip $args
        return ""
    }

    proc ttk_style {args} {
        ::itest::log_decision ttk style $args
        return ""
    }

    proc ttk_treeview {args} {
        ::itest::log_decision ttk treeview $args
        return ""
    }


    # uri:: stubs (3 commands)

    proc uri_join {args} {
        ::itest::log_decision uri join $args
        return ""
    }

    proc uri_resolve {args} {
        ::itest::log_decision uri resolve $args
        return ""
    }

    proc uri_split {args} {
        ::itest::log_decision uri split $args
        return ""
    }


    # uuid:: stubs (1 commands)

    proc uuid_uuid {args} {
        ::itest::log_decision uuid uuid $args
        return ""
    }


    # yaml:: stubs (3 commands)

    proc yaml_dict2yaml {args} {
        ::itest::log_decision yaml dict2yaml $args
        return ""
    }

    proc yaml_huddle2yaml {args} {
        ::itest::log_decision yaml huddle2yaml $args
        return ""
    }

    proc yaml_yaml2dict {args} {
        ::itest::log_decision yaml yaml2dict $args
        return ""
    }


    # Top-level stubs (193 commands)

    proc cmd_accumulate {args} {
        ::itest::log_decision toplevel accumulate $args
        return ""
    }

    proc cmd_active_members {args} {
        ::itest::log_decision toplevel active_members $args
        return ""
    }

    proc cmd_active_nodes {args} {
        ::itest::log_decision toplevel active_nodes $args
        return ""
    }

    proc cmd_append {args} {
        ::itest::log_decision toplevel append $args
        return ""
    }

    proc cmd_apply {args} {
        ::itest::log_decision toplevel apply $args
        return ""
    }

    proc cmd_array {args} {
        ::itest::log_decision toplevel array $args
        return ""
    }

    proc cmd_b64decode {args} {
        ::itest::log_decision toplevel b64decode $args
        return ""
    }

    proc cmd_b64encode {args} {
        ::itest::log_decision toplevel b64encode $args
        return ""
    }

    proc cmd_bell {args} {
        ::itest::log_decision toplevel bell $args
        return ""
    }

    proc cmd_binary {args} {
        ::itest::log_decision toplevel binary $args
        return ""
    }

    proc cmd_bind {args} {
        ::itest::log_decision toplevel bind $args
        return ""
    }

    proc cmd_break {args} {
        ::itest::log_decision toplevel break $args
        return ""
    }

    proc cmd_button {args} {
        ::itest::log_decision toplevel button $args
        return ""
    }

    proc cmd_call {args} {
        ::itest::log_decision toplevel call $args
        return ""
    }

    proc cmd_canvas {args} {
        ::itest::log_decision toplevel canvas $args
        return ""
    }

    proc cmd_catch {args} {
        ::itest::log_decision toplevel catch $args
        return ""
    }

    proc cmd_chan {args} {
        ::itest::log_decision toplevel chan $args
        return ""
    }

    proc cmd_check {args} {
        ::itest::log_decision toplevel check $args
        return ""
    }

    proc cmd_checkbutton {args} {
        ::itest::log_decision toplevel checkbutton $args
        return ""
    }

    proc cmd_client_addr {args} {
        ::itest::log_decision toplevel client_addr $args
        return ""
    }

    proc cmd_client_port {args} {
        ::itest::log_decision toplevel client_port $args
        return ""
    }

    proc cmd_clientside {args} {
        ::itest::log_decision toplevel clientside $args
        return ""
    }

    proc cmd_clipboard {args} {
        ::itest::log_decision toplevel clipboard $args
        return ""
    }

    proc cmd_clock {args} {
        ::itest::log_decision toplevel clock $args
        return ""
    }

    proc cmd_clone {args} {
        ::itest::log_decision toplevel clone $args
        return ""
    }

    proc cmd_close {args} {
        ::itest::log_decision toplevel close $args
        return ""
    }

    proc cmd_concat {args} {
        ::itest::log_decision toplevel concat $args
        return ""
    }

    proc cmd_connect {args} {
        ::itest::log_decision toplevel connect $args
        return ""
    }

    proc cmd_continue {args} {
        ::itest::log_decision toplevel continue $args
        return ""
    }

    proc cmd_cpu {args} {
        ::itest::log_decision toplevel cpu $args
        return ""
    }

    proc cmd_crc32 {args} {
        ::itest::log_decision toplevel crc32 $args
        return ""
    }

    proc cmd_decode_uri {args} {
        ::itest::log_decision toplevel decode_uri $args
        return ""
    }

    proc cmd_destroy {args} {
        ::itest::log_decision toplevel destroy $args
        return ""
    }

    proc cmd_domain {args} {
        ::itest::log_decision toplevel domain $args
        return ""
    }

    proc cmd_encoding {args} {
        ::itest::log_decision toplevel encoding $args
        return ""
    }

    proc cmd_entry {args} {
        ::itest::log_decision toplevel entry $args
        return ""
    }

    proc cmd_error {args} {
        ::itest::log_decision toplevel error $args
        return ""
    }

    proc cmd_eval {args} {
        ::itest::log_decision toplevel eval $args
        return ""
    }

    proc cmd_expr {args} {
        ::itest::log_decision toplevel expr $args
        return ""
    }

    proc cmd_fasthash {args} {
        ::itest::log_decision toplevel fasthash $args
        return ""
    }

    proc cmd_findclass {args} {
        ::itest::log_decision toplevel findclass $args
        return ""
    }

    proc cmd_findstr {args} {
        ::itest::log_decision toplevel findstr $args
        return ""
    }

    proc cmd_focus {args} {
        ::itest::log_decision toplevel focus $args
        return ""
    }

    proc cmd_font {args} {
        ::itest::log_decision toplevel font $args
        return ""
    }

    proc cmd_for {args} {
        ::itest::log_decision toplevel for $args
        return ""
    }

    proc cmd_foreach {args} {
        ::itest::log_decision toplevel foreach $args
        return ""
    }

    proc cmd_format {args} {
        ::itest::log_decision toplevel format $args
        return ""
    }

    proc cmd_forward {args} {
        ::itest::log_decision toplevel forward $args
        return ""
    }

    proc cmd_frame {args} {
        ::itest::log_decision toplevel frame $args
        return ""
    }

    proc cmd_getfield {args} {
        ::itest::log_decision toplevel getfield $args
        return ""
    }

    proc cmd_global {args} {
        ::itest::log_decision toplevel global $args
        return ""
    }

    proc cmd_grab {args} {
        ::itest::log_decision toplevel grab $args
        return ""
    }

    proc cmd_grid {args} {
        ::itest::log_decision toplevel grid $args
        return ""
    }

    proc cmd_history {args} {
        ::itest::log_decision toplevel history $args
        return ""
    }

    proc cmd_htonl {args} {
        ::itest::log_decision toplevel htonl $args
        return ""
    }

    proc cmd_htons {args} {
        ::itest::log_decision toplevel htons $args
        return ""
    }

    proc cmd_http_client_ip {args} {
        ::itest::log_decision toplevel http_client_ip $args
        return ""
    }

    proc cmd_http_content_len_max {args} {
        ::itest::log_decision toplevel http_content_len_max $args
        return ""
    }

    proc cmd_http_cookie {args} {
        ::itest::log_decision toplevel http_cookie $args
        return ""
    }

    proc cmd_http_header {args} {
        ::itest::log_decision toplevel http_header $args
        return ""
    }

    proc cmd_http_host {args} {
        ::itest::log_decision toplevel http_host $args
        return ""
    }

    proc cmd_http_method {args} {
        ::itest::log_decision toplevel http_method $args
        return ""
    }

    proc cmd_http_uri {args} {
        ::itest::log_decision toplevel http_uri $args
        return ""
    }

    proc cmd_http_version {args} {
        ::itest::log_decision toplevel http_version $args
        return ""
    }

    proc cmd_if {args} {
        ::itest::log_decision toplevel if $args
        return ""
    }

    proc cmd_ifile {args} {
        ::itest::log_decision toplevel ifile $args
        return ""
    }

    proc cmd_image {args} {
        ::itest::log_decision toplevel image $args
        return ""
    }

    proc cmd_imid {args} {
        ::itest::log_decision toplevel imid $args
        return ""
    }

    proc cmd_incr {args} {
        ::itest::log_decision toplevel incr $args
        return ""
    }

    proc cmd_info {args} {
        ::itest::log_decision toplevel info $args
        return ""
    }

    proc cmd_ip_addr {args} {
        ::itest::log_decision toplevel ip_addr $args
        return ""
    }

    proc cmd_ip_protocol {args} {
        ::itest::log_decision toplevel ip_protocol $args
        return ""
    }

    proc cmd_ip_tos {args} {
        ::itest::log_decision toplevel ip_tos $args
        return ""
    }

    proc cmd_ip_ttl {args} {
        ::itest::log_decision toplevel ip_ttl $args
        return ""
    }

    proc cmd_join {args} {
        ::itest::log_decision toplevel join $args
        return ""
    }

    proc cmd_label {args} {
        ::itest::log_decision toplevel label $args
        return ""
    }

    proc cmd_labelframe {args} {
        ::itest::log_decision toplevel labelframe $args
        return ""
    }

    proc cmd_lappend {args} {
        ::itest::log_decision toplevel lappend $args
        return ""
    }

    proc cmd_lasthop {args} {
        ::itest::log_decision toplevel lasthop $args
        return ""
    }

    proc cmd_lindex {args} {
        ::itest::log_decision toplevel lindex $args
        return ""
    }

    proc cmd_link_qos {args} {
        ::itest::log_decision toplevel link_qos $args
        return ""
    }

    proc cmd_linsert {args} {
        ::itest::log_decision toplevel linsert $args
        return ""
    }

    proc cmd_list {args} {
        ::itest::log_decision toplevel list $args
        return ""
    }

    proc cmd_listbox {args} {
        ::itest::log_decision toplevel listbox $args
        return ""
    }

    proc cmd_listen {args} {
        ::itest::log_decision toplevel listen $args
        return ""
    }

    proc cmd_llength {args} {
        ::itest::log_decision toplevel llength $args
        return ""
    }

    proc cmd_llookup {args} {
        ::itest::log_decision toplevel llookup $args
        return ""
    }

    proc cmd_local_addr {args} {
        ::itest::log_decision toplevel local_addr $args
        return ""
    }

    proc cmd_local_port {args} {
        ::itest::log_decision toplevel local_port $args
        return ""
    }

    proc cmd_lower {args} {
        ::itest::log_decision toplevel lower $args
        return ""
    }

    proc cmd_lrange {args} {
        ::itest::log_decision toplevel lrange $args
        return ""
    }

    proc cmd_lrepeat {args} {
        ::itest::log_decision toplevel lrepeat $args
        return ""
    }

    proc cmd_lreplace {args} {
        ::itest::log_decision toplevel lreplace $args
        return ""
    }

    proc cmd_lreverse {args} {
        ::itest::log_decision toplevel lreverse $args
        return ""
    }

    proc cmd_lsearch {args} {
        ::itest::log_decision toplevel lsearch $args
        return ""
    }

    proc cmd_lset {args} {
        ::itest::log_decision toplevel lset $args
        return ""
    }

    proc cmd_lsort {args} {
        ::itest::log_decision toplevel lsort $args
        return ""
    }

    proc cmd_matchclass {args} {
        ::itest::log_decision toplevel matchclass $args
        return ""
    }

    proc cmd_md4 {args} {
        ::itest::log_decision toplevel md4 $args
        return ""
    }

    proc cmd_md5 {args} {
        ::itest::log_decision toplevel md5 $args
        return ""
    }

    proc cmd_members {args} {
        ::itest::log_decision toplevel members $args
        return ""
    }

    proc cmd_menu {args} {
        ::itest::log_decision toplevel menu $args
        return ""
    }

    proc cmd_menubutton {args} {
        ::itest::log_decision toplevel menubutton $args
        return ""
    }

    proc cmd_message {args} {
        ::itest::log_decision toplevel message $args
        return ""
    }

    proc cmd_nexthop {args} {
        ::itest::log_decision toplevel nexthop $args
        return ""
    }

    proc cmd_nodes {args} {
        ::itest::log_decision toplevel nodes $args
        return ""
    }

    proc cmd_ntohl {args} {
        ::itest::log_decision toplevel ntohl $args
        return ""
    }

    proc cmd_ntohs {args} {
        ::itest::log_decision toplevel ntohs $args
        return ""
    }

    proc cmd_option {args} {
        ::itest::log_decision toplevel option $args
        return ""
    }

    proc cmd_pack {args} {
        ::itest::log_decision toplevel pack $args
        return ""
    }

    proc cmd_panedwindow {args} {
        ::itest::log_decision toplevel panedwindow $args
        return ""
    }

    proc cmd_parray {args} {
        ::itest::log_decision toplevel parray $args
        return ""
    }

    proc cmd_peer {args} {
        ::itest::log_decision toplevel peer $args
        return ""
    }

    proc cmd_pem_dtos {args} {
        ::itest::log_decision toplevel pem_dtos $args
        return ""
    }

    proc cmd_pkg_mkIndex {args} {
        ::itest::log_decision toplevel pkg_mkIndex $args
        return ""
    }

    proc cmd_place {args} {
        ::itest::log_decision toplevel place $args
        return ""
    }

    proc cmd_priority {args} {
        ::itest::log_decision toplevel priority $args
        return ""
    }

    proc cmd_proc {args} {
        ::itest::log_decision toplevel proc $args
        return ""
    }

    proc cmd_puts {args} {
        ::itest::log_decision toplevel puts $args
        return ""
    }

    proc cmd_radiobutton {args} {
        ::itest::log_decision toplevel radiobutton $args
        return ""
    }

    proc cmd_radius_authenticate {args} {
        ::itest::log_decision toplevel radius_authenticate $args
        return ""
    }

    proc cmd_raise {args} {
        ::itest::log_decision toplevel raise $args
        return ""
    }

    proc cmd_rateclass {args} {
        ::itest::log_decision toplevel rateclass $args
        return ""
    }

    proc cmd_read {args} {
        ::itest::log_decision toplevel read $args
        return ""
    }

    proc cmd_recv {args} {
        ::itest::log_decision toplevel recv $args
        return ""
    }

    proc cmd_redirect {args} {
        ::itest::log_decision toplevel redirect $args
        return ""
    }

    proc cmd_regexp {args} {
        ::itest::log_decision toplevel regexp $args
        return ""
    }

    proc cmd_regsub {args} {
        ::itest::log_decision toplevel regsub $args
        return ""
    }

    proc cmd_relate_client {args} {
        ::itest::log_decision toplevel relate_client $args
        return ""
    }

    proc cmd_relate_server {args} {
        ::itest::log_decision toplevel relate_server $args
        return ""
    }

    proc cmd_remote_addr {args} {
        ::itest::log_decision toplevel remote_addr $args
        return ""
    }

    proc cmd_remote_port {args} {
        ::itest::log_decision toplevel remote_port $args
        return ""
    }

    proc cmd_return {args} {
        ::itest::log_decision toplevel return $args
        return ""
    }

    proc cmd_rmd160 {args} {
        ::itest::log_decision toplevel rmd160 $args
        return ""
    }

    proc cmd_scale {args} {
        ::itest::log_decision toplevel scale $args
        return ""
    }

    proc cmd_scan {args} {
        ::itest::log_decision toplevel scan $args
        return ""
    }

    proc cmd_scrollbar {args} {
        ::itest::log_decision toplevel scrollbar $args
        return ""
    }

    proc cmd_selection {args} {
        ::itest::log_decision toplevel selection $args
        return ""
    }

    proc cmd_send {args} {
        ::itest::log_decision toplevel send $args
        return ""
    }

    proc cmd_server_addr {args} {
        ::itest::log_decision toplevel server_addr $args
        return ""
    }

    proc cmd_server_port {args} {
        ::itest::log_decision toplevel server_port $args
        return ""
    }

    proc cmd_serverside {args} {
        ::itest::log_decision toplevel serverside $args
        return ""
    }

    proc cmd_session {args} {
        ::itest::log_decision toplevel session $args
        return ""
    }

    proc cmd_set {args} {
        ::itest::log_decision toplevel set $args
        return ""
    }

    proc cmd_sha1 {args} {
        ::itest::log_decision toplevel sha1 $args
        return ""
    }

    proc cmd_sha256 {args} {
        ::itest::log_decision toplevel sha256 $args
        return ""
    }

    proc cmd_sha384 {args} {
        ::itest::log_decision toplevel sha384 $args
        return ""
    }

    proc cmd_sha512 {args} {
        ::itest::log_decision toplevel sha512 $args
        return ""
    }

    proc cmd_sharedvar {args} {
        ::itest::log_decision toplevel sharedvar $args
        return ""
    }

    proc cmd_spinbox {args} {
        ::itest::log_decision toplevel spinbox $args
        return ""
    }

    proc cmd_split {args} {
        ::itest::log_decision toplevel split $args
        return ""
    }

    proc cmd_string {args} {
        ::itest::log_decision toplevel string $args
        return ""
    }

    proc cmd_subst {args} {
        ::itest::log_decision toplevel subst $args
        return ""
    }

    proc cmd_substr {args} {
        ::itest::log_decision toplevel substr $args
        return ""
    }

    proc cmd_switch {args} {
        ::itest::log_decision toplevel switch $args
        return ""
    }

    proc cmd_tcl_endOfWord {args} {
        ::itest::log_decision toplevel tcl_endOfWord $args
        return ""
    }

    proc cmd_tcl_startOfNextWord {args} {
        ::itest::log_decision toplevel tcl_startOfNextWord $args
        return ""
    }

    proc cmd_tcl_startOfPreviousWord {args} {
        ::itest::log_decision toplevel tcl_startOfPreviousWord $args
        return ""
    }

    proc cmd_tcl_wordBreakAfter {args} {
        ::itest::log_decision toplevel tcl_wordBreakAfter $args
        return ""
    }

    proc cmd_tcl_wordBreakBefore {args} {
        ::itest::log_decision toplevel tcl_wordBreakBefore $args
        return ""
    }

    proc cmd_tcpdump {args} {
        ::itest::log_decision toplevel tcpdump $args
        return ""
    }

    proc cmd_text {args} {
        ::itest::log_decision toplevel text $args
        return ""
    }

    proc cmd_timing {args} {
        ::itest::log_decision toplevel timing $args
        return ""
    }

    proc cmd_tk {args} {
        ::itest::log_decision toplevel tk $args
        return ""
    }

    proc cmd_tk_chooseColor {args} {
        ::itest::log_decision toplevel tk_chooseColor $args
        return ""
    }

    proc cmd_tk_chooseDirectory {args} {
        ::itest::log_decision toplevel tk_chooseDirectory $args
        return ""
    }

    proc cmd_tk_getOpenFile {args} {
        ::itest::log_decision toplevel tk_getOpenFile $args
        return ""
    }

    proc cmd_tk_getSaveFile {args} {
        ::itest::log_decision toplevel tk_getSaveFile $args
        return ""
    }

    proc cmd_tk_messageBox {args} {
        ::itest::log_decision toplevel tk_messageBox $args
        return ""
    }

    proc cmd_tk_popup {args} {
        ::itest::log_decision toplevel tk_popup $args
        return ""
    }

    proc cmd_toplevel {args} {
        ::itest::log_decision toplevel toplevel $args
        return ""
    }

    proc cmd_trace {args} {
        ::itest::log_decision toplevel trace $args
        return ""
    }

    proc cmd_traffic_group {args} {
        ::itest::log_decision toplevel traffic_group $args
        return ""
    }

    proc cmd_translate {args} {
        ::itest::log_decision toplevel translate $args
        return ""
    }

    proc cmd_uniq_ordered_ip_list {args} {
        ::itest::log_decision toplevel uniq_ordered_ip_list $args
        return ""
    }

    proc cmd_uniq_sorted_ip_list {args} {
        ::itest::log_decision toplevel uniq_sorted_ip_list $args
        return ""
    }

    proc cmd_unload {args} {
        ::itest::log_decision toplevel unload $args
        return ""
    }

    proc cmd_unset {args} {
        ::itest::log_decision toplevel unset $args
        return ""
    }

    proc cmd_uplevel {args} {
        ::itest::log_decision toplevel uplevel $args
        return ""
    }

    proc cmd_upvar {args} {
        ::itest::log_decision toplevel upvar $args
        return ""
    }

    proc cmd_urlcatblindquery {args} {
        ::itest::log_decision toplevel urlcatblindquery $args
        return ""
    }

    proc cmd_urlcatquery {args} {
        ::itest::log_decision toplevel urlcatquery $args
        return ""
    }

    proc cmd_use {args} {
        ::itest::log_decision toplevel use $args
        return ""
    }

    proc cmd_variable {args} {
        ::itest::log_decision toplevel variable $args
        return ""
    }

    proc cmd_vlan_id {args} {
        ::itest::log_decision toplevel vlan_id $args
        return ""
    }

    proc cmd_when {args} {
        ::itest::log_decision toplevel when $args
        return ""
    }

    proc cmd_whereis {args} {
        ::itest::log_decision toplevel whereis $args
        return ""
    }

    proc cmd_while {args} {
        ::itest::log_decision toplevel while $args
        return ""
    }

    proc cmd_winfo {args} {
        ::itest::log_decision toplevel winfo $args
        return ""
    }

    proc cmd_wm {args} {
        ::itest::log_decision toplevel wm $args
        return ""
    }

    proc cmd_xff_list {args} {
        ::itest::log_decision toplevel xff_list $args
        return ""
    }

    proc cmd_xff_uniq_ordered_ip_list {args} {
        ::itest::log_decision toplevel xff_uniq_ordered_ip_list $args
        return ""
    }

    proc cmd_xff_uniq_sorted_ip_list {args} {
        ::itest::log_decision toplevel xff_uniq_sorted_ip_list $args
        return ""
    }

}

# Total stub mocks generated: 1188
