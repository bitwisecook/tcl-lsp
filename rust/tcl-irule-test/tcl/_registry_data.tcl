# _registry_data.tcl -- AUTO-GENERATED from Python command registry
#
# DO NOT EDIT.  Regenerate with:
#   python -m tooling.irule_test.codegen_registry_data
#
# Source: compiler/registry/
#
# `tooling.irule_test.codegen_registry_data` no longer exists (Python was
# fully retired from this codebase) -- there is no regeneration step. The
# `tcl::mathop::{&&,||,@}` entries below (bogus operators, never real Tcl
# commands -- see rust/tcl-registry/src/commands/tcl/mathop_generated.rs)
# and the missing `tcl::mathop::{lt,le,gt,ge}` entries (real Tcl 9.0+
# TIP 461 operators) were hand-fixed for issue #984 to match the registry.
# The rest of this file's data has not been re-audited against the registry.
#
# Copyright (c) 2024 tcl-lsp contributors.  MIT licence.

namespace eval ::tmm {

    # Standard Tcl 8.4 commands that TMM removes.
    # Derived from: tcl8.4 dialect minus f5-irules dialect.

    variable _gen_disabled_commands {
        !
        !=
        %
        &
        *
        **
        +
        -
        /
        ::tcl::build-info
        ::tcl::mathop::!
        ::tcl::mathop::!=
        ::tcl::mathop::%
        ::tcl::mathop::&
        ::tcl::mathop::*
        ::tcl::mathop::**
        ::tcl::mathop::+
        ::tcl::mathop::-
        ::tcl::mathop::/
        ::tcl::mathop::<
        ::tcl::mathop::<<
        ::tcl::mathop::<=
        ::tcl::mathop::==
        ::tcl::mathop::>
        ::tcl::mathop::>=
        ::tcl::mathop::>>
        ::tcl::mathop::^
        ::tcl::mathop::eq
        ::tcl::mathop::in
        ::tcl::mathop::max
        ::tcl::mathop::min
        ::tcl::mathop::ne
        ::tcl::mathop::ni
        ::tcl::mathop::|
        ::tcl::mathop::~
        ::tcl::unsupported::corotype
        <
        <<
        <=
        ==
        >
        >=
        >>
        ^
        auto_execok
        auto_import
        auto_load
        auto_mkindex
        auto_mkindex_old
        auto_qualify
        auto_reset
        bell
        bgerror
        bind
        button
        canvas
        cd
        checkbutton
        clipboard
        destroy
        entry
        eof
        eq
        exec
        exit
        fblocked
        fconfigure
        fcopy
        file
        fileevent
        filename
        flush
        focus
        font
        frame
        gets
        glob
        grab
        grid
        http
        image
        in
        interp
        label
        labelframe
        listbox
        load
        lower
        max
        memory
        menu
        menubutton
        message
        min
        namespace
        ne
        ni
        open
        option
        pack
        package
        panedwindow
        pid
        pkg_mkindex
        place
        pwd
        radiobutton
        raise
        re_quote
        regex::quote
        regex_quote
        regexp::quote
        registry
        rename
        scale
        scrollbar
        seek
        selection
        socket
        source
        spinbox
        tcl::build-info
        tcl::mathop::!
        tcl::mathop::!=
        tcl::mathop::%
        tcl::mathop::&
        tcl::mathop::*
        tcl::mathop::**
        tcl::mathop::+
        tcl::mathop::-
        tcl::mathop::/
        tcl::mathop::<
        tcl::mathop::<<
        tcl::mathop::<=
        tcl::mathop::==
        tcl::mathop::>
        tcl::mathop::>=
        tcl::mathop::>>
        tcl::mathop::^
        tcl::mathop::eq
        tcl::mathop::in
        tcl::mathop::max
        tcl::mathop::min
        tcl::mathop::ne
        tcl::mathop::ni
        tcl::mathop::|
        tcl::mathop::~
        tcl_findLibrary
        tell
        text
        time
        timerate
        tk
        tk_chooseColor
        tk_chooseDirectory
        tk_getOpenFile
        tk_getSaveFile
        tk_messageBox
        tk_popup
        toplevel
        ttk::button
        ttk::combobox
        ttk::entry
        ttk::frame
        ttk::label
        ttk::notebook
        ttk::progressbar
        ttk::scale
        ttk::separator
        ttk::sizegrip
        ttk::style
        ttk::treeview
        unknown
        unload
        update
        vwait
        winfo
        wm
        |
        ~
    }

    # Commands from Tcl 8.5+ that do not exist in 8.4 or iRules.
    # Derived from: (tcl8.5 | tcl8.6 | tcl9.0) - tcl8.4 - f5-irules.

    variable _gen_post84_commands {
        ::tcl::idna
        ::tcl::mathop::ge
        ::tcl::mathop::gt
        ::tcl::mathop::le
        ::tcl::mathop::lt
        ::tcl::process
        classvariable
        coroinject
        coroprobe
        coroutine
        dict
        foreachLine
        ge
        gt
        lassign
        le
        ledit
        lmap
        lpop
        lremove
        lseq
        lt
        my
        next
        nextto
        oo::abstract
        oo::class
        oo::configurable
        oo::copy
        oo::define
        oo::objdefine
        oo::object
        oo::singleton
        readFile
        self
        tailcall
        tcl::idna
        tcl::mathop::ge
        tcl::mathop::gt
        tcl::mathop::le
        tcl::mathop::lt
        tcl::process
        throw
        try
        writeFile
        yield
        yieldto
        zlib
    }

}

namespace eval ::tmm::expr_ops {

    # TMM custom infix expression operators for expr rewriting.
    # Derived from: compiler.registry.operators.IRULES_OPERATOR_HOVER

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
    # Count: 1226

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
        {HTML::encode}
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
        {URI::encode_component}
        {URI::escape}
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
        {cmdline::getArgv0}
        {cmdline::getKnownOpt}
        {cmdline::getKnownOptions}
        {cmdline::getfiles}
        {cmdline::getopt}
        {cmdline::getoptions}
        {cmdline::typedGetopt}
        {cmdline::typedGetoptions}
        {cmdline::typedUsage}
        {cmdline::usage}

        # csv::
        {csv::iscomplete}
        {csv::join}
        {csv::joinlist}
        {csv::joinmatrix}
        {csv::read2matrix}
        {csv::read2queue}
        {csv::report}
        {csv::split}
        {csv::split2matrix}
        {csv::split2queue}
        {csv::writematrix}
        {csv::writequeue}

        # dns::
        {dns::address}
        {dns::cleanup}
        {dns::cname}
        {dns::configure}
        {dns::dump}
        {dns::error}
        {dns::errorcode}
        {dns::name}
        {dns::reset}
        {dns::resolve}
        {dns::result}
        {dns::status}
        {dns::wait}

        # fileutil::
        {fileutil::appendToFile}
        {fileutil::cat}
        {fileutil::fileType}
        {fileutil::find}
        {fileutil::findByPattern}
        {fileutil::foreachLine}
        {fileutil::fullnormalize}
        {fileutil::grep}
        {fileutil::insertIntoFile}
        {fileutil::install}
        {fileutil::jail}
        {fileutil::lexnormalize}
        {fileutil::maketempdir}
        {fileutil::relative}
        {fileutil::relativeUrl}
        {fileutil::removeFromFile}
        {fileutil::replaceInFile}
        {fileutil::stripN}
        {fileutil::stripPwd}
        {fileutil::tempdir}
        {fileutil::tempdirReset}
        {fileutil::tempfile}
        {fileutil::test}
        {fileutil::touch}
        {fileutil::updateInPlace}
        {fileutil::writeFile}

        # html::
        {html::html_entities}
        {html::tagstrip}

        # http::
        {http::cleanup}
        {http::code}
        {http::config}
        {http::cookiejar}
        {http::data}
        {http::error}
        {http::formatQuery}
        {http::geturl}
        {http::meta}
        {http::ncode}
        {http::postError}
        {http::quoteString}
        {http::reasonPhrase}
        {http::register}
        {http::registerError}
        {http::requestHeaderValue}
        {http::requestHeaders}
        {http::requestLine}
        {http::reset}
        {http::responseBody}
        {http::responseCode}
        {http::responseHeaderValue}
        {http::responseHeaders}
        {http::responseInfo}
        {http::responseLine}
        {http::size}
        {http::status}
        {http::unregister}
        {http::wait}

        # ip::
        {ip::collapse}
        {ip::contract}
        {ip::equal}
        {ip::is}
        {ip::mask}
        {ip::normalize}
        {ip::prefix}
        {ip::subtract}
        {ip::type}
        {ip::version}

        # json::
        {json::dict2json}
        {json::json2dict}
        {json::list2json}
        {json::many-json2dict}
        {json::string2json}
        {json::validate}

        # logger::
        {logger::disable}
        {logger::enable}
        {logger::import}
        {logger::init}
        {logger::initNamespace}
        {logger::levels}
        {logger::servicecmd}
        {logger::services}
        {logger::setlevel}
        {logger::walk}

        # math::
        {math::statistics::analyse-Kruskal-Wallis}
        {math::statistics::autocorr}
        {math::statistics::basic-stats}
        {math::statistics::control-Rchart}
        {math::statistics::control-xbar}
        {math::statistics::corr}
        {math::statistics::crosscorr}
        {math::statistics::filter}
        {math::statistics::group-rank}
        {math::statistics::histogram}
        {math::statistics::histogram-alt}
        {math::statistics::interval-mean-stdev}
        {math::statistics::lillieforsFit}
        {math::statistics::linear-model}
        {math::statistics::linear-residuals}
        {math::statistics::map}
        {math::statistics::max}
        {math::statistics::mean}
        {math::statistics::mean-histogram-limits}
        {math::statistics::median}
        {math::statistics::min}
        {math::statistics::minmax-histogram-limits}
        {math::statistics::number}
        {math::statistics::print-2x2}
        {math::statistics::pstdev}
        {math::statistics::pvar}
        {math::statistics::quantiles}
        {math::statistics::samplescount}
        {math::statistics::spearman-rank}
        {math::statistics::spearman-rank-extended}
        {math::statistics::stdev}
        {math::statistics::t-test-mean}
        {math::statistics::test-2x2}
        {math::statistics::test-Duckworth}
        {math::statistics::test-Dunnett}
        {math::statistics::test-Kruskal-Wallis}
        {math::statistics::test-Rchart}
        {math::statistics::test-Tukey-range}
        {math::statistics::test-Wilcoxon}
        {math::statistics::test-anova-F}
        {math::statistics::test-normal}
        {math::statistics::test-xbar}
        {math::statistics::var}

        # md5::
        {md5::md5}

        # mime::
        {mime::buildmessage}
        {mime::copymessage}
        {mime::field_decode}
        {mime::finalize}
        {mime::getContentType}
        {mime::getTransferEncoding}
        {mime::getbody}
        {mime::getheader}
        {mime::getproperty}
        {mime::getsize}
        {mime::initialize}
        {mime::mapencoding}
        {mime::parseaddress}
        {mime::parsedatetime}
        {mime::reversemapencoding}
        {mime::setheader}
        {mime::uniqueID}
        {mime::word_decode}
        {mime::word_encode}

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
        {msgcat::mcpackagenamespaceget}
        {msgcat::mcpreferences}
        {msgcat::mcset}
        {msgcat::mcunknown}
        {msgcat::mcutil}

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
        {snit::compile}
        {snit::macro}
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
        {tcl::idna::decode}
        {tcl::idna::encode}
        {tcl::tm::path}
        {tcl::tm::roots}

        # tcltest::
        {tcltest::bytestring}
        {tcltest::cleanupTests}
        {tcltest::configure}
        {tcltest::customMatch}
        {tcltest::debug}
        {tcltest::errorChannel}
        {tcltest::errorFile}
        {tcltest::getMatchingFiles}
        {tcltest::interpreter}
        {tcltest::limitConstraints}
        {tcltest::loadFile}
        {tcltest::loadScript}
        {tcltest::loadTestedCommands}
        {tcltest::mainThread}
        {tcltest::makeDirectory}
        {tcltest::makeFile}
        {tcltest::match}
        {tcltest::matchDirectories}
        {tcltest::matchFiles}
        {tcltest::normalizeMsg}
        {tcltest::normalizePath}
        {tcltest::outputChannel}
        {tcltest::outputFile}
        {tcltest::preserveCore}
        {tcltest::removeDirectory}
        {tcltest::removeFile}
        {tcltest::restoreState}
        {tcltest::runAllTests}
        {tcltest::saveState}
        {tcltest::singleProcess}
        {tcltest::skip}
        {tcltest::skipDirectories}
        {tcltest::skipFiles}
        {tcltest::temporaryDirectory}
        {tcltest::test}
        {tcltest::testConstraint}
        {tcltest::testsDirectory}
        {tcltest::threadReap}
        {tcltest::verbose}
        {tcltest::viewFile}
        {tcltest::workingDirectory}

        # textutil::
        {textutil::adjust}
        {textutil::blank}
        {textutil::cap}
        {textutil::capEachWord}
        {textutil::chop}
        {textutil::indent}
        {textutil::longestCommonPrefix}
        {textutil::longestCommonPrefixList}
        {textutil::splitn}
        {textutil::splitx}
        {textutil::strRepeat}
        {textutil::tabify}
        {textutil::tabify2}
        {textutil::tail}
        {textutil::trim}
        {textutil::trimEmptyHeading}
        {textutil::trimPrefix}
        {textutil::trimleft}
        {textutil::trimright}
        {textutil::uncap}
        {textutil::undent}
        {textutil::untabify}
        {textutil::untabify2}

        # uri::
        {uri::canonicalize}
        {uri::geturl}
        {uri::isrelative}
        {uri::join}
        {uri::register}
        {uri::resolve}
        {uri::setQuirkOption}
        {uri::split}

        # uuid::
        {uuid::uuid}

        # yaml::
        {yaml::dict2yaml}
        {yaml::huddle2yaml}
        {yaml::list2yaml}
        {yaml::setOptions}
        {yaml::yaml2dict}
        {yaml::yaml2huddle}
    }

    # All f5-irules top-level commands.
    # Count: 273

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
        binary
        break
        call
        catch
        chan
        check
        class
        client_addr
        client_port
        clientside
        clock
        clone
        close
        concat
        connect
        const
        continue
        cpu
        crc32
        decode_uri
        discard
        domain
        drop
        encoding
        error
        eval
        event
        expr
        fasthash
        findclass
        findstr
        for
        foreach
        format
        forward
        getfield
        gettimes
        global
        history
        html_encode
        html_escape
        htmlencode
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
        imid
        incr
        info
        ip_addr
        ip_protocol
        ip_tos
        ip_ttl
        join
        lappend
        lasthop
        lgen
        lindex
        link_qos
        linsert
        list
        listen
        llength
        llookup
        local_addr
        local_port
        log
        lrange
        lrepeat
        lreplace
        lreverse
        lsearch
        lset
        lsort
        lstring
        matchclass
        md4
        md5
        members
        nexthop
        node
        nodes
        noop
        ntohl
        ntohs
        parray
        peer
        pem_dtos
        persist
        pkg_mkIndex
        pool
        priority
        proc
        puts
        radius_authenticate
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
        scan
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
        testapplylambda
        testappverifierpresent
        testasync
        testbigdata
        testbignumobj
        testbooleanobj
        testbumpinterpepoch
        testbytestring
        testchannel
        testchannelevent
        testcmdinfo
        testcmdtoken
        testcmdtrace
        testconcatobj
        testcpuid
        testcreatecommand
        testdcall
        testdel
        testdelassocdata
        testdoubledigits
        testdoubleobj
        testdstring
        testencoding
        testevalex
        testevalobjv
        testevent
        testexithandler
        testexitmainloop
        testexprdouble
        testexprdoubleobj
        testexprlong
        testexprlongobj
        testexprparser
        testexprstring
        testfevent
        testfile
        testfilelink
        testfilesystem
        testfindfirst
        testfindlast
        testfstildeexpand
        testgetassocdata
        testgetindexfromobjstruct
        testgetint
        testgetintforindex
        testgetplatform
        testgetunichar
        testgetvarfullname
        testhandlecount
        testhashsystemhash
        testindexobj
        testinterpdelete
        testinterpresolver
        testintobj
        testlink
        testlinkarray
        testlistobj
        testlistrep
        testlocale
        testlongsize
        testlutil
        testmainthread
        testmsb
        testnrelevels
        testnreunwind
        testnumutfchars
        testobj
        testpanic
        testparseargs
        testparser
        testparsevar
        testparsevarname
        testpreferstable
        testprint
        testpurebytesobj
        testregexp
        testreturn
        testsaveresult
        testservicemode
        testset2
        testsetassocdata
        testsetbytearraylength
        testseterr
        testseterrorcode
        testsetmainloop
        testsetnoerr
        testsetobjerrorcode
        testsetplatform
        testsimplefilesystem
        testsize
        testsocket
        teststaticlibrary
        teststaticpkg
        teststringbytes
        teststringobj
        testtranslatefilename
        testuniclass
        testupvar
        testutfnext
        testutfprev
        testwrongnumargs
        timing
        trace
        traffic_group
        translate
        uniq_ordered_ip_list
        uniq_sorted_ip_list
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
        xff_list
        xff_uniq_ordered_ip_list
        xff_uniq_sorted_ip_list
    }

}
