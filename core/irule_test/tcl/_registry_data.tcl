# _registry_data.tcl -- AUTO-GENERATED from Python command registry
#
# DO NOT EDIT.  Regenerate with:
#   python -m core.irule_test.codegen_registry_data
#
# Source: core/commands/registry/
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::tmm {

    # Standard Tcl 8.4 commands that TMM removes.
    # Derived from: tcl8.4 dialect minus f5-irules dialect.

    variable _gen_disabled_commands {
        auto_execok
        auto_import
        auto_load
        auto_mkindex
        auto_mkindex_old
        auto_qualify
        auto_reset
        bgerror
        cd
        eof
        exec
        exit
        fblocked
        fconfigure
        fcopy
        file
        fileevent
        filename
        flush
        gets
        glob
        http
        interp
        load
        memory
        namespace
        open
        package
        pid
        pkg_mkindex
        pwd
        rename
        seek
        socket
        source
        tcl_findLibrary
        tell
        time
        unknown
        update
        vwait
    }

    # Commands from Tcl 8.5+ that do not exist in 8.4 or iRules.
    # Derived from: (tcl8.5 | tcl8.6 | tcl9.0) - tcl8.4 - f5-irules.

    variable _gen_post84_commands {
        coroutine
        dict
        lassign
        lmap
        oo::class
        oo::copy
        oo::define
        oo::objdefine
        oo::object
        tailcall
        throw
        try
        yield
        yieldto
    }

}

namespace eval ::tmm::expr_ops {

    # TMM custom infix expression operators for expr rewriting.
    # Derived from: core.commands.registry.operators.IRULES_OPERATOR_HOVER

    variable _gen_operators {
        contains
        ends_with
        equals
        matches_glob
        matches_regex
        starts_with
    }

    # All TMM expression operators (including boolean aliases).

    variable _gen_all_operators {
        and
        contains
        ends_with
        equals
        matches_glob
        matches_regex
        not
        or
        starts_with
    }

}

namespace eval ::itest::cmd {

    # All f5-irules namespaced commands (NS::subcommand).
    # Count: 1054

    variable _gen_namespaced_commands {
        # AAA::
        {AAA::acct_result}
        {AAA::acct_send}
        {AAA::auth_result}
        {AAA::auth_send}

        # ACCESS2::
        {ACCESS2::access2_proc}

        # ACCESS::
        {ACCESS::acl}
        {ACCESS::disable}
        {ACCESS::enable}
        {ACCESS::ephemeral-auth}
        {ACCESS::flowid}
        {ACCESS::log}
        {ACCESS::oauth}
        {ACCESS::perflow}
        {ACCESS::policy}
        {ACCESS::respond}
        {ACCESS::restrict_irule_events}
        {ACCESS::saml}
        {ACCESS::session}
        {ACCESS::user}
        {ACCESS::uuid}

        # ACL::
        {ACL::action}
        {ACL::eval}

        # ADAPT::
        {ADAPT::allow}
        {ADAPT::context_create}
        {ADAPT::context_current}
        {ADAPT::context_delete_all}
        {ADAPT::context_name}
        {ADAPT::context_static}
        {ADAPT::enable}
        {ADAPT::preview_size}
        {ADAPT::result}
        {ADAPT::select}
        {ADAPT::service_down_action}
        {ADAPT::timeout}

        # AES::
        {AES::decrypt}
        {AES::encrypt}
        {AES::key}

        # AM::
        {AM::age}
        {AM::application}
        {AM::cache}
        {AM::disable}
        {AM::expires}
        {AM::media_playlist}
        {AM::policy_node}

        # ANTIFRAUD::
        {ANTIFRAUD::alert_additional_info}
        {ANTIFRAUD::alert_bait_signatures}
        {ANTIFRAUD::alert_component}
        {ANTIFRAUD::alert_defined_value}
        {ANTIFRAUD::alert_details}
        {ANTIFRAUD::alert_device_id}
        {ANTIFRAUD::alert_expected_value}
        {ANTIFRAUD::alert_fingerprint}
        {ANTIFRAUD::alert_forbidden_added_element}
        {ANTIFRAUD::alert_guid}
        {ANTIFRAUD::alert_html}
        {ANTIFRAUD::alert_http_referrer}
        {ANTIFRAUD::alert_id}
        {ANTIFRAUD::alert_license_id}
        {ANTIFRAUD::alert_min}
        {ANTIFRAUD::alert_origin}
        {ANTIFRAUD::alert_resolved_value}
        {ANTIFRAUD::alert_score}
        {ANTIFRAUD::alert_transaction_data}
        {ANTIFRAUD::alert_transaction_id}
        {ANTIFRAUD::alert_type}
        {ANTIFRAUD::alert_username}
        {ANTIFRAUD::alert_view_id}
        {ANTIFRAUD::client_id}
        {ANTIFRAUD::device_id}
        {ANTIFRAUD::disable}
        {ANTIFRAUD::disable_alert}
        {ANTIFRAUD::disable_app_layer_encryption}
        {ANTIFRAUD::disable_auto_transactions}
        {ANTIFRAUD::disable_injection}
        {ANTIFRAUD::disable_malware}
        {ANTIFRAUD::disable_phishing}
        {ANTIFRAUD::enable}
        {ANTIFRAUD::enable_log}
        {ANTIFRAUD::fingerprint}
        {ANTIFRAUD::geo}
        {ANTIFRAUD::guid}
        {ANTIFRAUD::result}
        {ANTIFRAUD::username}

        # ASM::
        {ASM::captcha}
        {ASM::captcha_age}
        {ASM::captcha_status}
        {ASM::client_ip}
        {ASM::conviction}
        {ASM::deception}
        {ASM::disable}
        {ASM::enable}
        {ASM::fingerprint}
        {ASM::is_authenticated}
        {ASM::login_status}
        {ASM::microservice}
        {ASM::payload}
        {ASM::policy}
        {ASM::raise}
        {ASM::severity}
        {ASM::signature}
        {ASM::status}
        {ASM::support_id}
        {ASM::threat_campaign}
        {ASM::unblock}
        {ASM::uncaptcha}
        {ASM::username}
        {ASM::violation}
        {ASM::violation_data}

        # ASN1::
        {ASN1::decode}
        {ASN1::element}
        {ASN1::encode}

        # AUTH::
        {AUTH::abort}
        {AUTH::authenticate}
        {AUTH::authenticate_continue}
        {AUTH::cert_credential}
        {AUTH::cert_issuer_credential}
        {AUTH::last_event_session_id}
        {AUTH::password_credential}
        {AUTH::response_data}
        {AUTH::ssl_cc_ldap_status}
        {AUTH::ssl_cc_ldap_username}
        {AUTH::start}
        {AUTH::status}
        {AUTH::subscribe}
        {AUTH::unsubscribe}
        {AUTH::username_credential}
        {AUTH::wantcredential_prompt}
        {AUTH::wantcredential_prompt_style}
        {AUTH::wantcredential_type}

        # AVR::
        {AVR::disable}
        {AVR::disable_cspm_injection}
        {AVR::enable}
        {AVR::log}

        # BIGPROTO::
        {BIGPROTO::enable_fix_reset}

        # BIGTCP::
        {BIGTCP::release_flow}

        # BOTDEFENSE::
        {BOTDEFENSE::action}
        {BOTDEFENSE::bot_anomalies}
        {BOTDEFENSE::bot_categories}
        {BOTDEFENSE::bot_name}
        {BOTDEFENSE::bot_signature}
        {BOTDEFENSE::bot_signature_category}
        {BOTDEFENSE::captcha_age}
        {BOTDEFENSE::captcha_status}
        {BOTDEFENSE::client_class}
        {BOTDEFENSE::client_type}
        {BOTDEFENSE::cookie_age}
        {BOTDEFENSE::cookie_status}
        {BOTDEFENSE::cs_allowed}
        {BOTDEFENSE::cs_attribute}
        {BOTDEFENSE::cs_possible}
        {BOTDEFENSE::device_id}
        {BOTDEFENSE::disable}
        {BOTDEFENSE::enable}
        {BOTDEFENSE::intent}
        {BOTDEFENSE::micro_service}
        {BOTDEFENSE::previous_action}
        {BOTDEFENSE::previous_request_age}
        {BOTDEFENSE::previous_support_id}
        {BOTDEFENSE::reason}
        {BOTDEFENSE::support_id}

        # BWC::
        {BWC::color}
        {BWC::debug}
        {BWC::mark}
        {BWC::measure}
        {BWC::policy}
        {BWC::pps}
        {BWC::priority}
        {BWC::rate}

        # CACHE::
        {CACHE::accept_encoding}
        {CACHE::age}
        {CACHE::disable}
        {CACHE::disabled}
        {CACHE::enable}
        {CACHE::expire}
        {CACHE::fresh}
        {CACHE::header}
        {CACHE::headers}
        {CACHE::hits}
        {CACHE::payload}
        {CACHE::priority}
        {CACHE::statskey}
        {CACHE::trace}
        {CACHE::uri}
        {CACHE::useragent}
        {CACHE::userkey}

        # CATEGORY::
        {CATEGORY::analytics}
        {CATEGORY::filetype}
        {CATEGORY::lookup}
        {CATEGORY::matchtype}
        {CATEGORY::result}
        {CATEGORY::safesearch}

        # CLASSIFICATION::
        {CLASSIFICATION::app}
        {CLASSIFICATION::category}
        {CLASSIFICATION::disable}
        {CLASSIFICATION::enable}
        {CLASSIFICATION::protocol}
        {CLASSIFICATION::result}
        {CLASSIFICATION::urlcat}
        {CLASSIFICATION::username}

        # CLASSIFY::
        {CLASSIFY::application}
        {CLASSIFY::category}
        {CLASSIFY::defer}
        {CLASSIFY::disable}
        {CLASSIFY::urlcat}
        {CLASSIFY::username}

        # COMPRESS::
        {COMPRESS::buffer_size}
        {COMPRESS::disable}
        {COMPRESS::enable}
        {COMPRESS::gzip}
        {COMPRESS::method}
        {COMPRESS::nodelay}

        # CONNECTOR::
        {CONNECTOR::disable}
        {CONNECTOR::enable}
        {CONNECTOR::profile}
        {CONNECTOR::remap}

        # CRYPTO::
        {CRYPTO::decrypt}
        {CRYPTO::encrypt}
        {CRYPTO::hash}
        {CRYPTO::keygen}
        {CRYPTO::sign}
        {CRYPTO::verify}

        # DATAGRAM::
        {DATAGRAM::dns}
        {DATAGRAM::ip}
        {DATAGRAM::ip6}
        {DATAGRAM::l2}
        {DATAGRAM::tcp}
        {DATAGRAM::udp}

        # DECOMPRESS::
        {DECOMPRESS::disable}
        {DECOMPRESS::enable}

        # DEMANGLE::
        {DEMANGLE::disable}
        {DEMANGLE::enable}

        # DHCP::
        {DHCP::version}

        # DHCPv4::
        {DHCPv4::chaddr}
        {DHCPv4::ciaddr}
        {DHCPv4::drop}
        {DHCPv4::giaddr}
        {DHCPv4::hlen}
        {DHCPv4::hops}
        {DHCPv4::htype}
        {DHCPv4::len}
        {DHCPv4::opcode}
        {DHCPv4::option}
        {DHCPv4::reject}
        {DHCPv4::secs}
        {DHCPv4::siaddr}
        {DHCPv4::type}
        {DHCPv4::xid}
        {DHCPv4::yiaddr}

        # DHCPv6::
        {DHCPv6::drop}
        {DHCPv6::hop_count}
        {DHCPv6::len}
        {DHCPv6::link_address}
        {DHCPv6::msg_type}
        {DHCPv6::option}
        {DHCPv6::peer_address}
        {DHCPv6::reject}
        {DHCPv6::transaction_id}

        # DIAG::
        {DIAG::test}

        # DIAMETER::
        {DIAMETER::avp}
        {DIAMETER::command}
        {DIAMETER::disconnect}
        {DIAMETER::drop}
        {DIAMETER::dynamic_route_insertion}
        {DIAMETER::dynamic_route_lookup}
        {DIAMETER::header}
        {DIAMETER::host}
        {DIAMETER::is_request}
        {DIAMETER::is_response}
        {DIAMETER::is_retransmission}
        {DIAMETER::length}
        {DIAMETER::message}
        {DIAMETER::payload}
        {DIAMETER::persist}
        {DIAMETER::realm}
        {DIAMETER::respond}
        {DIAMETER::result}
        {DIAMETER::retransmission}
        {DIAMETER::retransmission_default}
        {DIAMETER::retransmission_reason}
        {DIAMETER::retransmit}
        {DIAMETER::retry}
        {DIAMETER::route_status}
        {DIAMETER::session}
        {DIAMETER::skip_capabilities_exchange}
        {DIAMETER::state}

        # DNS::
        {DNS::additional}
        {DNS::answer}
        {DNS::authority}
        {DNS::class}
        {DNS::disable}
        {DNS::drop}
        {DNS::edns0}
        {DNS::enable}
        {DNS::header}
        {DNS::is_wideip}
        {DNS::last_act}
        {DNS::len}
        {DNS::log}
        {DNS::name}
        {DNS::origin}
        {DNS::ptype}
        {DNS::query}
        {DNS::question}
        {DNS::rdata}
        {DNS::return}
        {DNS::rpz_policy}
        {DNS::rr}
        {DNS::scrape}
        {DNS::tsig}
        {DNS::ttl}
        {DNS::type}

        # DNSMSG::
        {DNSMSG::header}
        {DNSMSG::record}
        {DNSMSG::section}

        # DOSL7::
        {DOSL7::disable}
        {DOSL7::enable}
        {DOSL7::health}
        {DOSL7::is_ip_slowdown}
        {DOSL7::is_mitigated}
        {DOSL7::profile}
        {DOSL7::slowdown}

        # DSLITE::
        {DSLITE::remote_addr}

        # ECA::
        {ECA::client_machine_name}
        {ECA::disable}
        {ECA::domainname}
        {ECA::enable}
        {ECA::select}
        {ECA::status}
        {ECA::username}

        # FIX::
        {FIX::tag}

        # FLOW::
        {FLOW::create_related}
        {FLOW::idle_duration}
        {FLOW::idle_timeout}
        {FLOW::peer}
        {FLOW::priority}
        {FLOW::refresh}
        {FLOW::this}

        # FLOWTABLE::
        {FLOWTABLE::count}
        {FLOWTABLE::limit}

        # FTP::
        {FTP::allow_active_mode}
        {FTP::disable}
        {FTP::enable}
        {FTP::enforce_tls_session_reuse}
        {FTP::ftps_mode}
        {FTP::port}

        # GENERICMESSAGE::
        {GENERICMESSAGE::message}
        {GENERICMESSAGE::peer}
        {GENERICMESSAGE::route}

        # GTP::
        {GTP::clone}
        {GTP::discard}
        {GTP::forward}
        {GTP::header}
        {GTP::ie}
        {GTP::length}
        {GTP::message}
        {GTP::new}
        {GTP::parse}
        {GTP::payload}
        {GTP::respond}
        {GTP::tunnel}

        # HA::
        {HA::status}

        # HSL::
        {HSL::open}
        {HSL::send}

        # HTML::
        {HTML::comment}
        {HTML::disable}
        {HTML::enable}
        {HTML::tag}

        # HTTP2::
        {HTTP2::active}
        {HTTP2::concurrency}
        {HTTP2::disable}
        {HTTP2::disconnect}
        {HTTP2::enable}
        {HTTP2::header}
        {HTTP2::push}
        {HTTP2::requests}
        {HTTP2::stream}
        {HTTP2::version}

        # HTTP::
        {HTTP::class}
        {HTTP::close}
        {HTTP::collect}
        {HTTP::cookie}
        {HTTP::disable}
        {HTTP::enable}
        {HTTP::fallback}
        {HTTP::has_responded}
        {HTTP::header}
        {HTTP::host}
        {HTTP::hsts}
        {HTTP::is_keepalive}
        {HTTP::is_redirect}
        {HTTP::method}
        {HTTP::passthrough_reason}
        {HTTP::password}
        {HTTP::path}
        {HTTP::payload}
        {HTTP::proxy}
        {HTTP::query}
        {HTTP::redirect}
        {HTTP::reject_reason}
        {HTTP::release}
        {HTTP::request}
        {HTTP::request_num}
        {HTTP::respond}
        {HTTP::response}
        {HTTP::retry}
        {HTTP::status}
        {HTTP::uri}
        {HTTP::username}
        {HTTP::version}

        # HTTPLOG::
        {HTTPLOG::disable}
        {HTTPLOG::enable}

        # ICAP::
        {ICAP::header}
        {ICAP::method}
        {ICAP::status}
        {ICAP::uri}

        # IKE::
        {IKE::auth_success}
        {IKE::cert}
        {IKE::san_dirname}
        {IKE::san_dns}
        {IKE::san_ediparty}
        {IKE::san_email}
        {IKE::san_ipadd}
        {IKE::san_othername}
        {IKE::san_rid}
        {IKE::san_uri}
        {IKE::san_x400}
        {IKE::subjectAltName}

        # ILX::
        {ILX::call}
        {ILX::init}
        {ILX::notify}

        # IMAP::
        {IMAP::activation_mode}
        {IMAP::disable}
        {IMAP::enable}

        # IP::
        {IP::addr}
        {IP::client_addr}
        {IP::hops}
        {IP::idle_timeout}
        {IP::ingress_drop_rate}
        {IP::ingress_rate_limit}
        {IP::intelligence}
        {IP::local_addr}
        {IP::protocol}
        {IP::remote_addr}
        {IP::reputation}
        {IP::server_addr}
        {IP::stats}
        {IP::tos}
        {IP::ttl}
        {IP::version}

        # IPFIX::
        {IPFIX::destination}
        {IPFIX::msg}
        {IPFIX::template}

        # ISESSION::
        {ISESSION::deduplication}

        # ISTATS::
        {ISTATS::get}
        {ISTATS::incr}
        {ISTATS::remove}
        {ISTATS::set}

        # IVS_ENTRY::
        {IVS_ENTRY::result}

        # JSON::
        {JSON::array}
        {JSON::create}
        {JSON::get}
        {JSON::object}
        {JSON::parse}
        {JSON::render}
        {JSON::root}
        {JSON::set}
        {JSON::type}

        # L7CHECK::
        {L7CHECK::protocol}

        # LB::
        {LB::bias}
        {LB::class}
        {LB::command}
        {LB::connect}
        {LB::connlimit}
        {LB::context_id}
        {LB::detach}
        {LB::down}
        {LB::dst_tag}
        {LB::enable_decisionlog}
        {LB::mode}
        {LB::persist}
        {LB::prime}
        {LB::queue}
        {LB::reselect}
        {LB::select}
        {LB::server}
        {LB::snat}
        {LB::src_tag}
        {LB::status}
        {LB::up}

        # LDAP::
        {LDAP::activation_mode}
        {LDAP::disable}
        {LDAP::enable}

        # LINE::
        {LINE::get}
        {LINE::set}

        # LINK::
        {LINK::lasthop}
        {LINK::nexthop}
        {LINK::qos}
        {LINK::vlan_id}

        # LSN::
        {LSN::address}
        {LSN::disable}
        {LSN::inbound}
        {LSN::inbound-entry}
        {LSN::persistence}
        {LSN::persistence-entry}
        {LSN::pool}
        {LSN::port}

        # MESSAGE::
        {MESSAGE::field}
        {MESSAGE::proto}
        {MESSAGE::type}

        # MQTT::
        {MQTT::clean_session}
        {MQTT::client_id}
        {MQTT::collect}
        {MQTT::disable}
        {MQTT::disconnect}
        {MQTT::drop}
        {MQTT::dup}
        {MQTT::enable}
        {MQTT::insert}
        {MQTT::keep_alive}
        {MQTT::length}
        {MQTT::message}
        {MQTT::packet_id}
        {MQTT::password}
        {MQTT::payload}
        {MQTT::protocol_name}
        {MQTT::protocol_version}
        {MQTT::qos}
        {MQTT::release}
        {MQTT::replace}
        {MQTT::respond}
        {MQTT::retain}
        {MQTT::return_code}
        {MQTT::return_code_list}
        {MQTT::session_present}
        {MQTT::topic}
        {MQTT::type}
        {MQTT::username}
        {MQTT::will}

        # MR::
        {MR::always_match_port}
        {MR::available_for_routing}
        {MR::collect}
        {MR::connect_back_port}
        {MR::connection_instance}
        {MR::connection_mode}
        {MR::equivalent_transport}
        {MR::flow_id}
        {MR::ignore_peer_port}
        {MR::instance}
        {MR::max_retries}
        {MR::message}
        {MR::payload}
        {MR::peer}
        {MR::prime}
        {MR::protocol}
        {MR::release}
        {MR::restore}
        {MR::retry}
        {MR::return}
        {MR::store}
        {MR::stream}
        {MR::transport}

        # NAME::
        {NAME::lookup}
        {NAME::response}

        # NSH::
        {NSH::chain}
        {NSH::context}
        {NSH::md1}
        {NSH::mocksf}
        {NSH::path_id}
        {NSH::service_index}

        # NTLM::
        {NTLM::disable}
        {NTLM::enable}

        # OFFBOX::
        {OFFBOX::request}

        # ONECONNECT::
        {ONECONNECT::detach}
        {ONECONNECT::label}
        {ONECONNECT::reuse}
        {ONECONNECT::select}

        # PCP::
        {PCP::reject}
        {PCP::request}
        {PCP::response}

        # PEM::
        {PEM::disable}
        {PEM::enable}
        {PEM::flow}
        {PEM::session}
        {PEM::subscriber}

        # PLUGIN::
        {PLUGIN::disable}
        {PLUGIN::enable}

        # POLICY::
        {POLICY::controls}
        {POLICY::names}
        {POLICY::rules}
        {POLICY::targets}

        # POP3::
        {POP3::activation_mode}
        {POP3::disable}
        {POP3::enable}

        # PROFILE::
        {PROFILE::access}
        {PROFILE::antifraud}
        {PROFILE::auth}
        {PROFILE::avr}
        {PROFILE::clientssl}
        {PROFILE::diameter}
        {PROFILE::exchange}
        {PROFILE::exists}
        {PROFILE::fastL4}
        {PROFILE::fasthttp}
        {PROFILE::ftp}
        {PROFILE::http}
        {PROFILE::httpclass}
        {PROFILE::httpcompression}
        {PROFILE::list}
        {PROFILE::oneconnect}
        {PROFILE::persist}
        {PROFILE::serverssl}
        {PROFILE::stream}
        {PROFILE::tcp}
        {PROFILE::tftp}
        {PROFILE::udp}
        {PROFILE::vdi}
        {PROFILE::webacceleration}
        {PROFILE::xml}

        # PROTOCOL_INSPECTION::
        {PROTOCOL_INSPECTION::disable}
        {PROTOCOL_INSPECTION::id}

        # PSC::
        {PSC::aaa_reporting_interval}
        {PSC::attr}
        {PSC::calling_id}
        {PSC::imeisv}
        {PSC::imsi}
        {PSC::ip_address}
        {PSC::lease_time}
        {PSC::policy}
        {PSC::subscriber_id}
        {PSC::tower_id}
        {PSC::user_name}

        # PSM::
        {PSM::FTP::disable}
        {PSM::FTP::enable}
        {PSM::HTTP::disable}
        {PSM::HTTP::enable}
        {PSM::SMTP::disable}
        {PSM::SMTP::enable}

        # QOE::
        {QOE::disable}
        {QOE::enable}
        {QOE::video}

        # RADIUS::
        {RADIUS::avp}
        {RADIUS::code}
        {RADIUS::id}
        {RADIUS::rtdom}
        {RADIUS::subscriber}

        # RESOLV::
        {RESOLV::lookup}

        # RESOLVER::
        {RESOLVER::name_lookup}
        {RESOLVER::summarize}

        # REST::
        {REST::send}

        # REWRITE::
        {REWRITE::disable}
        {REWRITE::enable}
        {REWRITE::payload}
        {REWRITE::post_process}

        # ROUTE::
        {ROUTE::age}
        {ROUTE::bandwidth}
        {ROUTE::clear}
        {ROUTE::cwnd}
        {ROUTE::domain}
        {ROUTE::expiration}
        {ROUTE::mtu}
        {ROUTE::rtt}
        {ROUTE::rttvar}

        # RTSP::
        {RTSP::collect}
        {RTSP::header}
        {RTSP::method}
        {RTSP::msg_source}
        {RTSP::payload}
        {RTSP::release}
        {RTSP::respond}
        {RTSP::status}
        {RTSP::uri}
        {RTSP::version}

        # SCTP::
        {SCTP::client_port}
        {SCTP::collect}
        {SCTP::local_port}
        {SCTP::mss}
        {SCTP::payload}
        {SCTP::ppi}
        {SCTP::release}
        {SCTP::remote_port}
        {SCTP::respond}
        {SCTP::rto_initial}
        {SCTP::rto_max}
        {SCTP::rto_min}
        {SCTP::sack_timeout}
        {SCTP::server_port}

        # SDP::
        {SDP::field}
        {SDP::media}
        {SDP::session_id}

        # SIP::
        {SIP::call_id}
        {SIP::discard}
        {SIP::from}
        {SIP::header}
        {SIP::message}
        {SIP::method}
        {SIP::payload}
        {SIP::persist}
        {SIP::record-route}
        {SIP::respond}
        {SIP::response}
        {SIP::route}
        {SIP::route_status}
        {SIP::to}
        {SIP::uri}
        {SIP::via}

        # SIPALG::
        {SIPALG::hairpin}
        {SIPALG::hairpin_default}
        {SIPALG::nonregister_subscriber_listener}

        # SMTPS::
        {SMTPS::activation_mode}
        {SMTPS::disable}
        {SMTPS::enable}

        # SOCKS::
        {SOCKS::allowed}
        {SOCKS::destination}
        {SOCKS::version}

        # SSE::
        {SSE::field}

        # SSL::
        {SSL::allow_dynamic_record_sizing}
        {SSL::allow_nonssl}
        {SSL::alpn}
        {SSL::authenticate}
        {SSL::c3d}
        {SSL::cert}
        {SSL::cert_constraint}
        {SSL::cipher}
        {SSL::clientrandom}
        {SSL::collect}
        {SSL::disable}
        {SSL::enable}
        {SSL::extensions}
        {SSL::forward_proxy}
        {SSL::handshake}
        {SSL::is_renegotiation_secure}
        {SSL::maximum_record_size}
        {SSL::mode}
        {SSL::modssl_sessionid_headers}
        {SSL::nextproto}
        {SSL::payload}
        {SSL::profile}
        {SSL::release}
        {SSL::renegotiate}
        {SSL::respond}
        {SSL::secure_renegotiation}
        {SSL::session}
        {SSL::sessionid}
        {SSL::sessionsecret}
        {SSL::sessionticket}
        {SSL::sni}
        {SSL::tls13_secret}
        {SSL::unclean_shutdown}
        {SSL::verify_result}

        # STATS::
        {STATS::get}
        {STATS::incr}
        {STATS::set}
        {STATS::setmax}
        {STATS::setmin}

        # STREAM::
        {STREAM::disable}
        {STREAM::enable}
        {STREAM::encoding}
        {STREAM::expression}
        {STREAM::match}
        {STREAM::max_matchsize}
        {STREAM::replace}

        # TAP::
        {TAP::action}
        {TAP::config}
        {TAP::insight}
        {TAP::insight_requested}
        {TAP::score}

        # TCP::
        {TCP::abc}
        {TCP::analytics}
        {TCP::autowin}
        {TCP::bandwidth}
        {TCP::client_port}
        {TCP::close}
        {TCP::collect}
        {TCP::congestion}
        {TCP::delayed_ack}
        {TCP::dsack}
        {TCP::earlyrxmit}
        {TCP::ecn}
        {TCP::enhanced_loss_recovery}
        {TCP::idletime}
        {TCP::keepalive}
        {TCP::limxmit}
        {TCP::local_port}
        {TCP::lossfilter}
        {TCP::lossfilterburst}
        {TCP::lossfilterrate}
        {TCP::mss}
        {TCP::nagle}
        {TCP::naglemode}
        {TCP::naglestate}
        {TCP::notify}
        {TCP::offset}
        {TCP::option}
        {TCP::pacing}
        {TCP::payload}
        {TCP::proxybuffer}
        {TCP::proxybufferhigh}
        {TCP::proxybufferlow}
        {TCP::push_flag}
        {TCP::rcv_scale}
        {TCP::rcv_size}
        {TCP::recvwnd}
        {TCP::release}
        {TCP::remote_port}
        {TCP::respond}
        {TCP::rexmt_thresh}
        {TCP::rt_metrics_timeout}
        {TCP::rto}
        {TCP::rtt}
        {TCP::rttvar}
        {TCP::sendbuf}
        {TCP::server_port}
        {TCP::setmss}
        {TCP::snd_cwnd}
        {TCP::snd_scale}
        {TCP::snd_ssthresh}
        {TCP::snd_wnd}
        {TCP::unused_port}

        # TDS::
        {TDS::msg}
        {TDS::session}

        # TMM::
        {TMM::cmp_count}
        {TMM::cmp_group}
        {TMM::cmp_groups}
        {TMM::cmp_primary_group}
        {TMM::cmp_unit}

        # UDP::
        {UDP::client_port}
        {UDP::debug_queue}
        {UDP::drop}
        {UDP::hold}
        {UDP::local_port}
        {UDP::max_buf_pkts}
        {UDP::max_rate}
        {UDP::mss}
        {UDP::payload}
        {UDP::release}
        {UDP::remote_port}
        {UDP::respond}
        {UDP::sendbuffer}
        {UDP::server_port}
        {UDP::unused_port}

        # URI::
        {URI::basename}
        {URI::compare}
        {URI::decode}
        {URI::encode}
        {URI::host}
        {URI::path}
        {URI::port}
        {URI::protocol}
        {URI::query}

        # VALIDATE::
        {VALIDATE::protocol}

        # VDI::
        {VDI::disable}
        {VDI::enable}

        # WAM::
        {WAM::disable}
        {WAM::enable}

        # WEBSSO::
        {WEBSSO::disable}
        {WEBSSO::enable}
        {WEBSSO::select}

        # WS::
        {WS::collect}
        {WS::disconnect}
        {WS::enabled}
        {WS::frame}
        {WS::masking}
        {WS::message}
        {WS::payload}
        {WS::payload_ivs}
        {WS::payload_processing}
        {WS::release}
        {WS::request}
        {WS::response}

        # X509::
        {X509::cert_fields}
        {X509::extensions}
        {X509::hash}
        {X509::issuer}
        {X509::not_valid_after}
        {X509::not_valid_before}
        {X509::pem2der}
        {X509::serial_number}
        {X509::signature_algorithm}
        {X509::subject}
        {X509::subject_public_key}
        {X509::subject_public_key_RSA_bits}
        {X509::subject_public_key_type}
        {X509::verify_cert_error_string}
        {X509::version}
        {X509::whole}

        # XLAT::
        {XLAT::listen}
        {XLAT::listen_lifetime}
        {XLAT::src_addr}
        {XLAT::src_config}
        {XLAT::src_endpoint_reservation}
        {XLAT::src_nat_valid_range}
        {XLAT::src_port}

        # XML::
        {XML::address}
        {XML::collect}
        {XML::disable}
        {XML::element}
        {XML::enable}
        {XML::event}
        {XML::eventid}
        {XML::parse}
        {XML::payload}
        {XML::release}
        {XML::soap}
        {XML::subscribe}

        # base64::
        {base64::decode}
        {base64::encode}

        # cmdline::
        {cmdline::getopt}
        {cmdline::getoptions}
        {cmdline::usage}

        # csv::
        {csv::join}
        {csv::read}
        {csv::report}
        {csv::split}

        # dns::
        {dns::address}
        {dns::cleanup}
        {dns::name}
        {dns::resolve}

        # fileutil::
        {fileutil::cat}
        {fileutil::tempdir}
        {fileutil::tempfile}
        {fileutil::writeFile}

        # html::
        {html::html_entities}
        {html::tagstrip}

        # http::
        {http::cleanup}
        {http::code}
        {http::config}
        {http::cookiejar}
        {http::cookiejar::IDNAdecode}
        {http::cookiejar::IDNAencode}
        {http::data}
        {http::error}
        {http::formatQuery}
        {http::geturl}
        {http::meta}
        {http::ncode}
        {http::quoteString}
        {http::register}
        {http::reset}
        {http::size}
        {http::status}
        {http::unregister}
        {http::wait}

        # ip::
        {ip::contract}
        {ip::equal}
        {ip::normalize}
        {ip::prefix}
        {ip::version}

        # json::
        {json::dict2json}
        {json::json2dict}

        # logger::
        {logger::init}
        {logger::levels}
        {logger::servicecmd}
        {logger::services}

        # math::
        {math::statistics::basic-stats}
        {math::statistics::mean}
        {math::statistics::median}
        {math::statistics::quantiles}
        {math::statistics::stdev}
        {math::statistics::var}

        # md5::
        {md5::md5}

        # mime::
        {mime::finalize}
        {mime::getbody}
        {mime::getproperty}
        {mime::initialize}

        # msgcat::
        {msgcat::mc}
        {msgcat::mcexists}
        {msgcat::mcflmset}
        {msgcat::mcflset}
        {msgcat::mcforgetpackage}
        {msgcat::mcload}
        {msgcat::mcloadedlocales}
        {msgcat::mclocale}
        {msgcat::mcmax}
        {msgcat::mcmset}
        {msgcat::mcn}
        {msgcat::mcpackageconfig}
        {msgcat::mcpackagelocale}
        {msgcat::mcpreferences}
        {msgcat::mcset}
        {msgcat::mcunknown}

        # pkg::
        {pkg::create}

        # platform::
        {platform::generic}
        {platform::identify}
        {platform::patterns}
        {platform::shell::generic}
        {platform::shell::identify}

        # safe::
        {safe::interpAddToAccessPath}
        {safe::interpConfigure}
        {safe::interpCreate}
        {safe::interpDelete}
        {safe::interpFindInAccessPath}
        {safe::interpInit}
        {safe::setLogCmd}
        {safe::setSyncMode}

        # sha1::
        {sha1::sha1}

        # sha2::
        {sha2::sha256}

        # smtp::
        {smtp::sendmessage}

        # snit::
        {snit::method}
        {snit::type}
        {snit::typemethod}
        {snit::widget}
        {snit::widgetadaptor}

        # struct::
        {struct::list}
        {struct::queue}
        {struct::set}
        {struct::stack}

        # tcl::
        {tcl::OptKeyDelete}
        {tcl::OptKeyError}
        {tcl::OptKeyParse}
        {tcl::OptKeyRegister}
        {tcl::OptParse}
        {tcl::OptProc}
        {tcl::OptProcArgGiven}
        {tcl::tm::path}
        {tcl::tm::roots}

        # tcltest::
        {tcltest::cleanupTests}
        {tcltest::configure}
        {tcltest::customMatch}
        {tcltest::errorChannel}
        {tcltest::interpreter}
        {tcltest::loadTestedCommands}
        {tcltest::makeDirectory}
        {tcltest::makeFile}
        {tcltest::outputChannel}
        {tcltest::removeDirectory}
        {tcltest::removeFile}
        {tcltest::runAllTests}
        {tcltest::test}
        {tcltest::testConstraint}
        {tcltest::viewFile}

        # textutil::
        {textutil::adjust}
        {textutil::indent}
        {textutil::splitx}
        {textutil::trim}
        {textutil::undent}

        # ttk::
        {ttk::button}
        {ttk::combobox}
        {ttk::entry}
        {ttk::frame}
        {ttk::label}
        {ttk::notebook}
        {ttk::progressbar}
        {ttk::scale}
        {ttk::separator}
        {ttk::sizegrip}
        {ttk::style}
        {ttk::treeview}

        # uri::
        {uri::join}
        {uri::resolve}
        {uri::split}

        # uuid::
        {uuid::uuid}

        # yaml::
        {yaml::dict2yaml}
        {yaml::huddle2yaml}
        {yaml::yaml2dict}
    }

    # All f5-irules top-level commands.
    # Count: 207

    variable _gen_toplevel_commands {
        accumulate
        active_members
        active_nodes
        after
        append
        apply
        array
        b64decode
        b64encode
        bell
        binary
        bind
        break
        button
        call
        canvas
        catch
        chan
        check
        checkbutton
        class
        client_addr
        client_port
        clientside
        clipboard
        clock
        clone
        close
        concat
        connect
        continue
        cpu
        crc32
        decode_uri
        destroy
        discard
        domain
        drop
        encoding
        entry
        error
        eval
        event
        expr
        fasthash
        findclass
        findstr
        focus
        font
        for
        foreach
        format
        forward
        frame
        getfield
        global
        grab
        grid
        history
        htonl
        htons
        http_client_ip
        http_content_len_max
        http_cookie
        http_header
        http_host
        http_method
        http_uri
        http_version
        if
        ifile
        image
        imid
        incr
        info
        ip_addr
        ip_protocol
        ip_tos
        ip_ttl
        join
        label
        labelframe
        lappend
        lasthop
        lindex
        link_qos
        linsert
        list
        listbox
        listen
        llength
        llookup
        local_addr
        local_port
        log
        lower
        lrange
        lrepeat
        lreplace
        lreverse
        lsearch
        lset
        lsort
        matchclass
        md4
        md5
        members
        menu
        menubutton
        message
        nexthop
        node
        nodes
        ntohl
        ntohs
        option
        pack
        panedwindow
        parray
        peer
        pem_dtos
        persist
        pkg_mkIndex
        place
        pool
        priority
        proc
        puts
        radiobutton
        radius_authenticate
        raise
        rateclass
        read
        recv
        redirect
        regexp
        regsub
        reject
        relate_client
        relate_server
        remote_addr
        remote_port
        return
        rmd160
        scale
        scan
        scrollbar
        selection
        send
        server_addr
        server_port
        serverside
        session
        set
        sha1
        sha256
        sha384
        sha512
        sharedvar
        snat
        snatpool
        spinbox
        split
        string
        subst
        substr
        switch
        table
        tcl_endOfWord
        tcl_startOfNextWord
        tcl_startOfPreviousWord
        tcl_wordBreakAfter
        tcl_wordBreakBefore
        tcpdump
        text
        timing
        tk
        tk_chooseColor
        tk_chooseDirectory
        tk_getOpenFile
        tk_getSaveFile
        tk_messageBox
        tk_popup
        toplevel
        trace
        traffic_group
        translate
        uniq_ordered_ip_list
        uniq_sorted_ip_list
        unload
        unset
        uplevel
        upvar
        urlcatblindquery
        urlcatquery
        use
        variable
        virtual
        vlan_id
        when
        whereis
        while
        winfo
        wm
        xff_list
        xff_uniq_ordered_ip_list
        xff_uniq_sorted_ip_list
    }

}
