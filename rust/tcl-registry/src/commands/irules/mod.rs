// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! F5 iRules command specifications.

// iRules commands use `::` namespace separator → double underscores in
// module names (e.g. `http__header` for `HTTP::header`). Suppress the
// snake_case warnings for these.
#![allow(non_snake_case)]

mod aaa__acct_result;
mod aaa__acct_send;
mod aaa__auth_result;
mod aaa__auth_send;
mod access2__access2_proc;
mod access__acl;
mod access__disable;
mod access__enable;
mod access__ephemeral_auth;
mod access__flowid;
mod access__log;
mod access__oauth;
mod access__perflow;
mod access__policy;
mod access__respond;
mod access__restrict_irule_events;
mod access__saml;
mod access__session;
mod access__user;
mod access__uuid;
mod accumulate;
mod acl__action;
mod acl__eval;
mod active_members;
mod active_nodes;
mod adapt__allow;
mod adapt__context_create;
mod adapt__context_current;
mod adapt__context_delete_all;
mod adapt__context_name;
mod adapt__context_static;
mod adapt__enable;
mod adapt__preview_size;
mod adapt__result;
mod adapt__select;
mod adapt__service_down_action;
mod adapt__timeout;
mod aes__decrypt;
mod aes__encrypt;
mod aes__key;
mod after;
mod am__age;
mod am__application;
mod am__cache;
mod am__disable;
mod am__expires;
mod am__media_playlist;
mod am__policy_node;
mod antifraud__alert_additional_info;
mod antifraud__alert_bait_signatures;
mod antifraud__alert_component;
mod antifraud__alert_defined_value;
mod antifraud__alert_details;
mod antifraud__alert_device_id;
mod antifraud__alert_expected_value;
mod antifraud__alert_fingerprint;
mod antifraud__alert_forbidden_added_element;
mod antifraud__alert_guid;
mod antifraud__alert_html;
mod antifraud__alert_http_referrer;
mod antifraud__alert_id;
mod antifraud__alert_license_id;
mod antifraud__alert_min;
mod antifraud__alert_origin;
mod antifraud__alert_resolved_value;
mod antifraud__alert_score;
mod antifraud__alert_transaction_data;
mod antifraud__alert_transaction_id;
mod antifraud__alert_type;
mod antifraud__alert_username;
mod antifraud__alert_view_id;
mod antifraud__client_id;
mod antifraud__device_id;
mod antifraud__disable;
mod antifraud__disable_alert;
mod antifraud__disable_app_layer_encryption;
mod antifraud__disable_auto_transactions;
mod antifraud__disable_injection;
mod antifraud__disable_malware;
mod antifraud__disable_phishing;
mod antifraud__enable;
mod antifraud__enable_log;
mod antifraud__fingerprint;
mod antifraud__geo;
mod antifraud__guid;
mod antifraud__result;
mod antifraud__username;
mod asm__captcha;
mod asm__captcha_age;
mod asm__captcha_status;
mod asm__client_ip;
mod asm__conviction;
mod asm__deception;
mod asm__disable;
mod asm__enable;
mod asm__fingerprint;
mod asm__is_authenticated;
mod asm__login_status;
mod asm__microservice;
mod asm__payload;
mod asm__policy;
mod asm__raise;
mod asm__severity;
mod asm__signature;
mod asm__status;
mod asm__support_id;
mod asm__threat_campaign;
mod asm__unblock;
mod asm__uncaptcha;
mod asm__username;
mod asm__violation;
mod asm__violation_data;
mod asn1__decode;
mod asn1__element;
mod asn1__encode;
mod auth__abort;
mod auth__authenticate;
mod auth__authenticate_continue;
mod auth__cert_credential;
mod auth__cert_issuer_credential;
mod auth__last_event_session_id;
mod auth__password_credential;
mod auth__response_data;
mod auth__ssl_cc_ldap_status;
mod auth__ssl_cc_ldap_username;
mod auth__start;
mod auth__status;
mod auth__subscribe;
mod auth__unsubscribe;
mod auth__username_credential;
mod auth__wantcredential_prompt;
mod auth__wantcredential_prompt_style;
mod auth__wantcredential_type;
mod avr__disable;
mod avr__disable_cspm_injection;
mod avr__enable;
mod avr__log;
mod b64decode;
mod b64encode;
mod bigproto__enable_fix_reset;
mod bigtcp__release_flow;
mod botdefense__action;
mod botdefense__bot_anomalies;
mod botdefense__bot_categories;
mod botdefense__bot_name;
mod botdefense__bot_signature;
mod botdefense__bot_signature_category;
mod botdefense__captcha_age;
mod botdefense__captcha_status;
mod botdefense__client_class;
mod botdefense__client_type;
mod botdefense__cookie_age;
mod botdefense__cookie_status;
mod botdefense__cs_allowed;
mod botdefense__cs_attribute;
mod botdefense__cs_possible;
mod botdefense__device_id;
mod botdefense__disable;
mod botdefense__enable;
mod botdefense__intent;
mod botdefense__micro_service;
mod botdefense__previous_action;
mod botdefense__previous_request_age;
mod botdefense__previous_support_id;
mod botdefense__reason;
mod botdefense__support_id;
mod bwc__color;
mod bwc__debug;
mod bwc__mark;
mod bwc__measure;
mod bwc__policy;
mod bwc__pps;
mod bwc__priority;
mod bwc__rate;
mod cache__accept_encoding;
mod cache__age;
mod cache__disable;
mod cache__disabled;
mod cache__enable;
mod cache__expire;
mod cache__fresh;
mod cache__header;
mod cache__headers;
mod cache__hits;
mod cache__payload;
mod cache__priority;
mod cache__statskey;
mod cache__trace;
mod cache__uri;
mod cache__useragent;
mod cache__userkey;
mod call;
mod category__analytics;
mod category__filetype;
mod category__lookup;
mod category__matchtype;
mod category__result;
mod category__safesearch;
mod check;
mod class;
mod classification__app;
mod classification__category;
mod classification__disable;
mod classification__enable;
mod classification__protocol;
mod classification__result;
mod classification__urlcat;
mod classification__username;
mod classify__application;
mod classify__category;
mod classify__defer;
mod classify__disable;
mod classify__urlcat;
mod classify__username;
mod client_addr;
mod client_port;
mod clientside;
mod clone;
mod close;
mod compress__buffer_size;
mod compress__disable;
mod compress__enable;
mod compress__gzip;
mod compress__method;
mod compress__nodelay;
mod connect;
mod connector__disable;
mod connector__enable;
mod connector__profile;
mod connector__remap;
mod cpu;
mod crc32;
mod crypto__decrypt;
mod crypto__encrypt;
mod crypto__hash;
mod crypto__keygen;
mod crypto__sign;
mod crypto__verify;
mod datagram__dns;
mod datagram__ip;
mod datagram__ip6;
mod datagram__l2;
mod datagram__tcp;
mod datagram__udp;
mod decode_uri;
mod decompress__disable;
mod decompress__enable;
mod demangle__disable;
mod demangle__enable;
mod dhcp__version;
mod dhcpv4__chaddr;
mod dhcpv4__ciaddr;
mod dhcpv4__drop;
mod dhcpv4__giaddr;
mod dhcpv4__hlen;
mod dhcpv4__hops;
mod dhcpv4__htype;
mod dhcpv4__len;
mod dhcpv4__opcode;
mod dhcpv4__option;
mod dhcpv4__reject;
mod dhcpv4__secs;
mod dhcpv4__siaddr;
mod dhcpv4__type;
mod dhcpv4__xid;
mod dhcpv4__yiaddr;
mod dhcpv6__drop;
mod dhcpv6__hop_count;
mod dhcpv6__len;
mod dhcpv6__link_address;
mod dhcpv6__msg_type;
mod dhcpv6__option;
mod dhcpv6__peer_address;
mod dhcpv6__reject;
mod dhcpv6__transaction_id;
mod diag__test;
mod diameter__avp;
mod diameter__command;
mod diameter__disconnect;
mod diameter__drop;
mod diameter__dynamic_route_insertion;
mod diameter__dynamic_route_lookup;
mod diameter__header;
mod diameter__host;
mod diameter__is_request;
mod diameter__is_response;
mod diameter__is_retransmission;
mod diameter__length;
mod diameter__message;
mod diameter__payload;
mod diameter__persist;
mod diameter__realm;
mod diameter__respond;
mod diameter__result;
mod diameter__retransmission;
mod diameter__retransmission_default;
mod diameter__retransmission_reason;
mod diameter__retransmit;
mod diameter__retry;
mod diameter__route_status;
mod diameter__session;
mod diameter__skip_capabilities_exchange;
mod diameter__state;
mod discard;
mod dns__additional;
mod dns__answer;
mod dns__authority;
mod dns__class;
mod dns__disable;
mod dns__drop;
mod dns__edns0;
mod dns__enable;
mod dns__header;
mod dns__is_wideip;
mod dns__last_act;
mod dns__len;
mod dns__log;
mod dns__name;
mod dns__origin;
mod dns__ptype;
mod dns__query;
mod dns__question;
mod dns__rdata;
mod dns__return;
mod dns__rpz_policy;
mod dns__rr;
mod dns__scrape;
mod dns__tsig;
mod dns__ttl;
mod dns__type;
mod dnsmsg__header;
mod dnsmsg__record;
mod dnsmsg__section;
mod domain;
mod dosl7__disable;
mod dosl7__enable;
mod dosl7__health;
mod dosl7__is_ip_slowdown;
mod dosl7__is_mitigated;
mod dosl7__profile;
mod dosl7__slowdown;
mod drop;
mod dslite__remote_addr;
mod eca__client_machine_name;
mod eca__disable;
mod eca__domainname;
mod eca__enable;
mod eca__select;
mod eca__status;
mod eca__username;
mod event;
mod fasthash;
mod findclass;
mod findstr;
mod fix__tag;
mod flow__create_related;
mod flow__idle_duration;
mod flow__idle_timeout;
mod flow__peer;
mod flow__priority;
mod flow__refresh;
mod flow__this;
mod flowtable__count;
mod flowtable__limit;
mod forward;
mod ftp__allow_active_mode;
mod ftp__disable;
mod ftp__enable;
mod ftp__enforce_tls_session_reuse;
mod ftp__ftps_mode;
mod ftp__port;
mod genericmessage__message;
mod genericmessage__peer;
mod genericmessage__route;
mod getfield;
mod gtp__clone;
mod gtp__discard;
mod gtp__forward;
mod gtp__header;
mod gtp__ie;
mod gtp__length;
mod gtp__message;
mod gtp__new;
mod gtp__parse;
mod gtp__payload;
mod gtp__respond;
mod gtp__tunnel;
mod ha__status;
mod hsl__open;
mod hsl__send;
mod html__comment;
mod html__disable;
mod html__enable;
mod html__encode;
mod html__tag;
mod html_encode;
mod html_escape;
mod htmlencode;
mod htonl;
mod htons;
mod http2__active;
mod http2__concurrency;
mod http2__disable;
mod http2__disconnect;
mod http2__enable;
mod http2__header;
mod http2__push;
mod http2__requests;
mod http2__stream;
mod http2__version;
mod http__class;
mod http__close;
mod http__collect;
mod http__cookie;
mod http__disable;
mod http__enable;
mod http__fallback;
mod http__has_responded;
mod http__header;
mod http__host;
mod http__hsts;
mod http__is_keepalive;
mod http__is_redirect;
mod http__method;
mod http__passthrough_reason;
mod http__password;
mod http__path;
mod http__payload;
mod http__proxy;
mod http__query;
mod http__redirect;
mod http__reject_reason;
mod http__release;
mod http__request;
mod http__request_num;
mod http__respond;
mod http__response;
mod http__retry;
mod http__status;
mod http__uri;
mod http__username;
mod http__version;
mod http_client_ip;
mod http_content_len_max;
mod http_cookie;
mod http_header;
mod http_host;
mod http_method;
mod http_uri;
mod http_version;
mod httplog__disable;
mod httplog__enable;
mod icap__header;
mod icap__method;
mod icap__status;
mod icap__uri;
mod ifile;
mod ike__auth_success;
mod ike__cert;
mod ike__san_dirname;
mod ike__san_dns;
mod ike__san_ediparty;
mod ike__san_email;
mod ike__san_ipadd;
mod ike__san_othername;
mod ike__san_rid;
mod ike__san_uri;
mod ike__san_x400;
mod ike__subjectaltname;
mod ilx__call;
mod ilx__init;
mod ilx__notify;
mod imap__activation_mode;
mod imap__disable;
mod imap__enable;
mod imid;
mod ip__addr;
mod ip__client_addr;
mod ip__hops;
mod ip__idle_timeout;
mod ip__ingress_drop_rate;
mod ip__ingress_rate_limit;
mod ip__intelligence;
mod ip__local_addr;
mod ip__protocol;
mod ip__remote_addr;
mod ip__reputation;
mod ip__server_addr;
mod ip__stats;
mod ip__tos;
mod ip__ttl;
mod ip__version;
mod ip_addr;
mod ip_protocol;
mod ip_tos;
mod ip_ttl;
mod ipfix__destination;
mod ipfix__msg;
mod ipfix__template;
mod isession__deduplication;
mod istats__get;
mod istats__incr;
mod istats__remove;
mod istats__set;
mod ivs_entry__result;
mod json__array;
mod json__create;
mod json__get;
mod json__object;
mod json__parse;
mod json__render;
mod json__root;
mod json__set;
mod json__type;
mod l7check__protocol;
mod lasthop;
mod lb__bias;
mod lb__class;
mod lb__command;
mod lb__connect;
mod lb__connlimit;
mod lb__context_id;
mod lb__detach;
mod lb__down;
mod lb__dst_tag;
mod lb__enable_decisionlog;
mod lb__mode;
mod lb__persist;
mod lb__prime;
mod lb__queue;
mod lb__reselect;
mod lb__select;
mod lb__server;
mod lb__snat;
mod lb__src_tag;
mod lb__status;
mod lb__up;
mod ldap__activation_mode;
mod ldap__disable;
mod ldap__enable;
mod line__get;
mod line__set;
mod link__lasthop;
mod link__nexthop;
mod link__qos;
mod link__vlan_id;
mod link_qos;
mod listen;
mod llookup;
mod local_addr;
mod local_port;
mod log;
mod lsn__address;
mod lsn__disable;
mod lsn__inbound;
mod lsn__inbound_entry;
mod lsn__persistence;
mod lsn__persistence_entry;
mod lsn__pool;
mod lsn__port;
mod matchclass;
mod md4;
mod md5;
mod members;
mod message__field;
mod message__proto;
mod message__type;
mod mqtt__clean_session;
mod mqtt__client_id;
mod mqtt__collect;
mod mqtt__disable;
mod mqtt__disconnect;
mod mqtt__drop;
mod mqtt__dup;
mod mqtt__enable;
mod mqtt__insert;
mod mqtt__keep_alive;
mod mqtt__length;
mod mqtt__message;
mod mqtt__packet_id;
mod mqtt__password;
mod mqtt__payload;
mod mqtt__protocol_name;
mod mqtt__protocol_version;
mod mqtt__qos;
mod mqtt__release;
mod mqtt__replace;
mod mqtt__respond;
mod mqtt__retain;
mod mqtt__return_code;
mod mqtt__return_code_list;
mod mqtt__session_present;
mod mqtt__topic;
mod mqtt__type;
mod mqtt__username;
mod mqtt__will;
mod mr__always_match_port;
mod mr__available_for_routing;
mod mr__collect;
mod mr__connect_back_port;
mod mr__connection_instance;
mod mr__connection_mode;
mod mr__equivalent_transport;
mod mr__flow_id;
mod mr__ignore_peer_port;
mod mr__instance;
mod mr__max_retries;
mod mr__message;
mod mr__payload;
mod mr__peer;
mod mr__prime;
mod mr__protocol;
mod mr__release;
mod mr__restore;
mod mr__retry;
mod mr__return;
mod mr__store;
mod mr__stream;
mod mr__transport;
mod name__lookup;
mod name__response;
mod nexthop;
mod node;
mod nodes;
mod nsh__chain;
mod nsh__context;
mod nsh__md1;
mod nsh__mocksf;
mod nsh__path_id;
mod nsh__service_index;
mod ntlm__disable;
mod ntlm__enable;
mod ntohl;
mod ntohs;
mod offbox__request;
mod oneconnect__detach;
mod oneconnect__label;
mod oneconnect__reuse;
mod oneconnect__select;
mod pcp__reject;
mod pcp__request;
mod pcp__response;
mod peer;
mod pem__disable;
mod pem__enable;
mod pem__flow;
mod pem__session;
mod pem__subscriber;
mod pem_dtos;
mod persist;
mod plugin__disable;
mod plugin__enable;
mod policy__controls;
mod policy__names;
mod policy__rules;
mod policy__targets;
mod pool;
mod pop3__activation_mode;
mod pop3__disable;
mod pop3__enable;
mod priority;
mod proc;
mod profile__access;
mod profile__antifraud;
mod profile__auth;
mod profile__avr;
mod profile__clientssl;
mod profile__diameter;
mod profile__exchange;
mod profile__exists;
mod profile__fasthttp;
mod profile__fastl4;
mod profile__ftp;
mod profile__http;
mod profile__httpclass;
mod profile__httpcompression;
mod profile__list;
mod profile__oneconnect;
mod profile__persist;
mod profile__serverssl;
mod profile__stream;
mod profile__tcp;
mod profile__tftp;
mod profile__udp;
mod profile__vdi;
mod profile__webacceleration;
mod profile__xml;
mod protocol_inspection__disable;
mod protocol_inspection__id;
mod psc__aaa_reporting_interval;
mod psc__attr;
mod psc__calling_id;
mod psc__imeisv;
mod psc__imsi;
mod psc__ip_address;
mod psc__lease_time;
mod psc__policy;
mod psc__subscriber_id;
mod psc__tower_id;
mod psc__user_name;
mod psm__ftp__disable;
mod psm__ftp__enable;
mod psm__http__disable;
mod psm__http__enable;
mod psm__smtp__disable;
mod psm__smtp__enable;
mod qoe__disable;
mod qoe__enable;
mod qoe__video;
mod radius__avp;
mod radius__code;
mod radius__id;
mod radius__rtdom;
mod radius__subscriber;
mod radius_authenticate;
mod rateclass;
mod recv;
mod redirect;
mod reject;
mod relate_client;
mod relate_server;
mod remote_addr;
mod remote_port;
mod resolv__lookup;
mod resolver__name_lookup;
mod resolver__summarize;
mod rest__send;
mod rewrite__disable;
mod rewrite__enable;
mod rewrite__payload;
mod rewrite__post_process;
mod rmd160;
mod route__age;
mod route__bandwidth;
mod route__clear;
mod route__cwnd;
mod route__domain;
mod route__expiration;
mod route__mtu;
mod route__rtt;
mod route__rttvar;
mod rtsp__collect;
mod rtsp__header;
mod rtsp__method;
mod rtsp__msg_source;
mod rtsp__payload;
mod rtsp__release;
mod rtsp__respond;
mod rtsp__status;
mod rtsp__uri;
mod rtsp__version;
mod sctp__client_port;
mod sctp__collect;
mod sctp__local_port;
mod sctp__mss;
mod sctp__payload;
mod sctp__ppi;
mod sctp__release;
mod sctp__remote_port;
mod sctp__respond;
mod sctp__rto_initial;
mod sctp__rto_max;
mod sctp__rto_min;
mod sctp__sack_timeout;
mod sctp__server_port;
mod sdp__field;
mod sdp__media;
mod sdp__session_id;
mod send;
mod server_addr;
mod server_port;
mod serverside;
mod session;
mod sha1;
mod sha256;
mod sha384;
mod sha512;
mod sharedvar;
mod sip__call_id;
mod sip__discard;
mod sip__from;
mod sip__header;
mod sip__message;
mod sip__method;
mod sip__payload;
mod sip__persist;
mod sip__record_route;
mod sip__respond;
mod sip__response;
mod sip__route;
mod sip__route_status;
mod sip__to;
mod sip__uri;
mod sip__via;
mod sipalg__hairpin;
mod sipalg__hairpin_default;
mod sipalg__nonregister_subscriber_listener;
mod smtps__activation_mode;
mod smtps__disable;
mod smtps__enable;
mod snat;
mod snatpool;
mod socks__allowed;
mod socks__destination;
mod socks__version;
mod sse__field;
mod ssl__allow_dynamic_record_sizing;
mod ssl__allow_nonssl;
mod ssl__alpn;
mod ssl__authenticate;
mod ssl__c3d;
mod ssl__cert;
mod ssl__cert_constraint;
mod ssl__cipher;
mod ssl__clientrandom;
mod ssl__collect;
mod ssl__disable;
mod ssl__enable;
mod ssl__extensions;
mod ssl__forward_proxy;
mod ssl__handshake;
mod ssl__is_renegotiation_secure;
mod ssl__maximum_record_size;
mod ssl__mode;
mod ssl__modssl_sessionid_headers;
mod ssl__nextproto;
mod ssl__payload;
mod ssl__profile;
mod ssl__release;
mod ssl__renegotiate;
mod ssl__respond;
mod ssl__secure_renegotiation;
mod ssl__session;
mod ssl__sessionid;
mod ssl__sessionsecret;
mod ssl__sessionticket;
mod ssl__sni;
mod ssl__tls13_secret;
mod ssl__unclean_shutdown;
mod ssl__verify_result;
mod stats__get;
mod stats__incr;
mod stats__set;
mod stats__setmax;
mod stats__setmin;
mod stream__disable;
mod stream__enable;
mod stream__encoding;
mod stream__expression;
mod stream__match;
mod stream__max_matchsize;
mod stream__replace;
mod substr;
mod table;
mod tap__action;
mod tap__config;
mod tap__insight;
mod tap__insight_requested;
mod tap__score;
mod tcp__abc;
mod tcp__analytics;
mod tcp__autowin;
mod tcp__bandwidth;
mod tcp__client_port;
mod tcp__close;
mod tcp__collect;
mod tcp__congestion;
mod tcp__delayed_ack;
mod tcp__dsack;
mod tcp__earlyrxmit;
mod tcp__ecn;
mod tcp__enhanced_loss_recovery;
mod tcp__idletime;
mod tcp__keepalive;
mod tcp__limxmit;
mod tcp__local_port;
mod tcp__lossfilter;
mod tcp__lossfilterburst;
mod tcp__lossfilterrate;
mod tcp__mss;
mod tcp__nagle;
mod tcp__naglemode;
mod tcp__naglestate;
mod tcp__notify;
mod tcp__offset;
mod tcp__option;
mod tcp__pacing;
mod tcp__payload;
mod tcp__proxybuffer;
mod tcp__proxybufferhigh;
mod tcp__proxybufferlow;
mod tcp__push_flag;
mod tcp__rcv_scale;
mod tcp__rcv_size;
mod tcp__recvwnd;
mod tcp__release;
mod tcp__remote_port;
mod tcp__respond;
mod tcp__rexmt_thresh;
mod tcp__rt_metrics_timeout;
mod tcp__rto;
mod tcp__rtt;
mod tcp__rttvar;
mod tcp__sendbuf;
mod tcp__server_port;
mod tcp__setmss;
mod tcp__snd_cwnd;
mod tcp__snd_scale;
mod tcp__snd_ssthresh;
mod tcp__snd_wnd;
mod tcp__unused_port;
mod tcpdump;
mod tds__msg;
mod tds__session;
mod timing;
mod tmm__cmp_count;
mod tmm__cmp_group;
mod tmm__cmp_groups;
mod tmm__cmp_primary_group;
mod tmm__cmp_unit;
mod traffic_group;
mod translate;
mod udp__client_port;
mod udp__debug_queue;
mod udp__drop;
mod udp__hold;
mod udp__local_port;
mod udp__max_buf_pkts;
mod udp__max_rate;
mod udp__mss;
mod udp__payload;
mod udp__release;
mod udp__remote_port;
mod udp__respond;
mod udp__sendbuffer;
mod udp__server_port;
mod udp__unused_port;
mod uniq_ordered_ip_list;
mod uniq_sorted_ip_list;
mod uri__basename;
mod uri__compare;
mod uri__decode;
mod uri__encode;
mod uri__encode_component;
mod uri__escape;
mod uri__host;
mod uri__path;
mod uri__port;
mod uri__protocol;
mod uri__query;
mod urlcatblindquery;
mod urlcatquery;
mod use_;
mod validate__protocol;
mod vdi__disable;
mod vdi__enable;
mod virtual_;
mod vlan_id;
mod wam__disable;
mod wam__enable;
mod websso__disable;
mod websso__enable;
mod websso__select;
mod when;
mod whereis;
mod ws__collect;
mod ws__disconnect;
mod ws__enabled;
mod ws__frame;
mod ws__masking;
mod ws__message;
mod ws__payload;
mod ws__payload_ivs;
mod ws__payload_processing;
mod ws__release;
mod ws__request;
mod ws__response;
mod x509__cert_fields;
mod x509__extensions;
mod x509__hash;
mod x509__issuer;
mod x509__not_valid_after;
mod x509__not_valid_before;
mod x509__pem2der;
mod x509__serial_number;
mod x509__signature_algorithm;
mod x509__subject;
mod x509__subject_public_key;
mod x509__subject_public_key_rsa_bits;
mod x509__subject_public_key_type;
mod x509__verify_cert_error_string;
mod x509__version;
mod x509__whole;
mod xff_list;
mod xff_uniq_ordered_ip_list;
mod xff_uniq_sorted_ip_list;
mod xlat__listen;
mod xlat__listen_lifetime;
mod xlat__src_addr;
mod xlat__src_config;
mod xlat__src_endpoint_reservation;
mod xlat__src_nat_valid_range;
mod xlat__src_port;
mod xml__address;
mod xml__collect;
mod xml__disable;
mod xml__element;
mod xml__enable;
mod xml__event;
mod xml__eventid;
mod xml__parse;
mod xml__payload;
mod xml__release;
mod xml__soap;
mod xml__subscribe;

use crate::spec::CommandSpec;
use crate::traits::Traits;

/// Return all iRules command specifications.
#[must_use]
pub fn irules_command_specs() -> Vec<CommandSpec> {
    IRULES_SPECS.to_vec()
}

/// Mark a registry spec as requiring the live HTTP transaction context.
///
/// The explicit wrappers in [`IRULES_SPECS`] are the source of truth for
/// IRULE1201. Consumers never infer the fact from an `HTTP::` name prefix.
const fn http_context(mut spec: CommandSpec) -> CommandSpec {
    spec.traits = spec.traits.union(Traits::REQUIRES_HTTP_CONTEXT);
    spec
}

/// All iRules command specifications as a compile-time constant.
///
/// Every `spec()` is a `const fn`, so this array is built at compile
/// time — letting the registry derive the taint-source index
/// ([`crate::registry::taint_source_index`]) as a `const` table rather
/// than at runtime. `irules_command_specs` simply clones it into the
/// owned `Vec` the registry loader expects.
pub const IRULES_SPECS: &[CommandSpec] = &[
    aaa__acct_result::spec(),
    aaa__acct_send::spec(),
    aaa__auth_result::spec(),
    aaa__auth_send::spec(),
    access2__access2_proc::spec(),
    access__acl::spec(),
    access__disable::spec(),
    access__enable::spec(),
    access__ephemeral_auth::spec(),
    access__flowid::spec(),
    access__log::spec(),
    access__oauth::spec(),
    access__perflow::spec(),
    access__policy::spec(),
    access__respond::spec(),
    access__restrict_irule_events::spec(),
    access__saml::spec(),
    access__session::spec(),
    access__user::spec(),
    access__uuid::spec(),
    accumulate::spec(),
    acl__action::spec(),
    acl__eval::spec(),
    active_members::spec(),
    active_nodes::spec(),
    adapt__allow::spec(),
    adapt__context_create::spec(),
    adapt__context_current::spec(),
    adapt__context_delete_all::spec(),
    adapt__context_name::spec(),
    adapt__context_static::spec(),
    adapt__enable::spec(),
    adapt__preview_size::spec(),
    adapt__result::spec(),
    adapt__select::spec(),
    adapt__service_down_action::spec(),
    adapt__timeout::spec(),
    aes__decrypt::spec(),
    aes__encrypt::spec(),
    aes__key::spec(),
    after::spec(),
    am__age::spec(),
    am__application::spec(),
    am__cache::spec(),
    am__disable::spec(),
    am__expires::spec(),
    am__media_playlist::spec(),
    am__policy_node::spec(),
    antifraud__alert_additional_info::spec(),
    antifraud__alert_bait_signatures::spec(),
    antifraud__alert_component::spec(),
    antifraud__alert_defined_value::spec(),
    antifraud__alert_details::spec(),
    antifraud__alert_device_id::spec(),
    antifraud__alert_expected_value::spec(),
    antifraud__alert_fingerprint::spec(),
    antifraud__alert_forbidden_added_element::spec(),
    antifraud__alert_guid::spec(),
    antifraud__alert_html::spec(),
    antifraud__alert_http_referrer::spec(),
    antifraud__alert_id::spec(),
    antifraud__alert_license_id::spec(),
    antifraud__alert_min::spec(),
    antifraud__alert_origin::spec(),
    antifraud__alert_resolved_value::spec(),
    antifraud__alert_score::spec(),
    antifraud__alert_transaction_data::spec(),
    antifraud__alert_transaction_id::spec(),
    antifraud__alert_type::spec(),
    antifraud__alert_username::spec(),
    antifraud__alert_view_id::spec(),
    antifraud__client_id::spec(),
    antifraud__device_id::spec(),
    antifraud__disable::spec(),
    antifraud__disable_alert::spec(),
    antifraud__disable_app_layer_encryption::spec(),
    antifraud__disable_auto_transactions::spec(),
    antifraud__disable_injection::spec(),
    antifraud__disable_malware::spec(),
    antifraud__disable_phishing::spec(),
    antifraud__enable::spec(),
    antifraud__enable_log::spec(),
    antifraud__fingerprint::spec(),
    antifraud__geo::spec(),
    antifraud__guid::spec(),
    antifraud__result::spec(),
    antifraud__username::spec(),
    asm__captcha::spec(),
    asm__captcha_age::spec(),
    asm__captcha_status::spec(),
    asm__client_ip::spec(),
    asm__conviction::spec(),
    asm__deception::spec(),
    asm__disable::spec(),
    asm__enable::spec(),
    asm__fingerprint::spec(),
    asm__is_authenticated::spec(),
    asm__login_status::spec(),
    asm__microservice::spec(),
    asm__payload::spec(),
    asm__policy::spec(),
    asm__raise::spec(),
    asm__severity::spec(),
    asm__signature::spec(),
    asm__status::spec(),
    asm__support_id::spec(),
    asm__threat_campaign::spec(),
    asm__unblock::spec(),
    asm__uncaptcha::spec(),
    asm__username::spec(),
    asm__violation::spec(),
    asm__violation_data::spec(),
    asn1__decode::spec(),
    asn1__element::spec(),
    asn1__encode::spec(),
    auth__abort::spec(),
    auth__authenticate::spec(),
    auth__authenticate_continue::spec(),
    auth__cert_credential::spec(),
    auth__cert_issuer_credential::spec(),
    auth__last_event_session_id::spec(),
    auth__password_credential::spec(),
    auth__response_data::spec(),
    auth__ssl_cc_ldap_status::spec(),
    auth__ssl_cc_ldap_username::spec(),
    auth__start::spec(),
    auth__status::spec(),
    auth__subscribe::spec(),
    auth__unsubscribe::spec(),
    auth__username_credential::spec(),
    auth__wantcredential_prompt::spec(),
    auth__wantcredential_prompt_style::spec(),
    auth__wantcredential_type::spec(),
    avr__disable::spec(),
    avr__disable_cspm_injection::spec(),
    avr__enable::spec(),
    avr__log::spec(),
    b64decode::spec(),
    b64encode::spec(),
    bigproto__enable_fix_reset::spec(),
    bigtcp__release_flow::spec(),
    botdefense__action::spec(),
    botdefense__bot_anomalies::spec(),
    botdefense__bot_categories::spec(),
    botdefense__bot_name::spec(),
    botdefense__bot_signature::spec(),
    botdefense__bot_signature_category::spec(),
    botdefense__captcha_age::spec(),
    botdefense__captcha_status::spec(),
    botdefense__client_class::spec(),
    botdefense__client_type::spec(),
    botdefense__cookie_age::spec(),
    botdefense__cookie_status::spec(),
    botdefense__cs_allowed::spec(),
    botdefense__cs_attribute::spec(),
    botdefense__cs_possible::spec(),
    botdefense__device_id::spec(),
    botdefense__disable::spec(),
    botdefense__enable::spec(),
    botdefense__intent::spec(),
    botdefense__micro_service::spec(),
    botdefense__previous_action::spec(),
    botdefense__previous_request_age::spec(),
    botdefense__previous_support_id::spec(),
    botdefense__reason::spec(),
    botdefense__support_id::spec(),
    bwc__color::spec(),
    bwc__debug::spec(),
    bwc__mark::spec(),
    bwc__measure::spec(),
    bwc__policy::spec(),
    bwc__pps::spec(),
    bwc__priority::spec(),
    bwc__rate::spec(),
    cache__accept_encoding::spec(),
    cache__age::spec(),
    cache__disable::spec(),
    cache__disabled::spec(),
    cache__enable::spec(),
    cache__expire::spec(),
    cache__fresh::spec(),
    cache__header::spec(),
    cache__headers::spec(),
    cache__hits::spec(),
    cache__payload::spec(),
    cache__priority::spec(),
    cache__statskey::spec(),
    cache__trace::spec(),
    cache__uri::spec(),
    cache__useragent::spec(),
    cache__userkey::spec(),
    call::spec(),
    category__analytics::spec(),
    category__filetype::spec(),
    category__lookup::spec(),
    category__matchtype::spec(),
    category__result::spec(),
    category__safesearch::spec(),
    check::spec(),
    class::spec(),
    classification__app::spec(),
    classification__category::spec(),
    classification__disable::spec(),
    classification__enable::spec(),
    classification__protocol::spec(),
    classification__result::spec(),
    classification__urlcat::spec(),
    classification__username::spec(),
    classify__application::spec(),
    classify__category::spec(),
    classify__defer::spec(),
    classify__disable::spec(),
    classify__urlcat::spec(),
    classify__username::spec(),
    client_addr::spec(),
    client_port::spec(),
    clientside::spec(),
    clone::spec(),
    close::spec(),
    compress__buffer_size::spec(),
    compress__disable::spec(),
    compress__enable::spec(),
    compress__gzip::spec(),
    compress__method::spec(),
    compress__nodelay::spec(),
    connect::spec(),
    connector__disable::spec(),
    connector__enable::spec(),
    connector__profile::spec(),
    connector__remap::spec(),
    cpu::spec(),
    crc32::spec(),
    crypto__decrypt::spec(),
    crypto__encrypt::spec(),
    crypto__hash::spec(),
    crypto__keygen::spec(),
    crypto__sign::spec(),
    crypto__verify::spec(),
    datagram__dns::spec(),
    datagram__ip::spec(),
    datagram__ip6::spec(),
    datagram__l2::spec(),
    datagram__tcp::spec(),
    datagram__udp::spec(),
    decode_uri::spec(),
    decompress__disable::spec(),
    decompress__enable::spec(),
    demangle__disable::spec(),
    demangle__enable::spec(),
    dhcp__version::spec(),
    dhcpv4__chaddr::spec(),
    dhcpv4__ciaddr::spec(),
    dhcpv4__drop::spec(),
    dhcpv4__giaddr::spec(),
    dhcpv4__hlen::spec(),
    dhcpv4__hops::spec(),
    dhcpv4__htype::spec(),
    dhcpv4__len::spec(),
    dhcpv4__opcode::spec(),
    dhcpv4__option::spec(),
    dhcpv4__reject::spec(),
    dhcpv4__secs::spec(),
    dhcpv4__siaddr::spec(),
    dhcpv4__type::spec(),
    dhcpv4__xid::spec(),
    dhcpv4__yiaddr::spec(),
    dhcpv6__drop::spec(),
    dhcpv6__hop_count::spec(),
    dhcpv6__len::spec(),
    dhcpv6__link_address::spec(),
    dhcpv6__msg_type::spec(),
    dhcpv6__option::spec(),
    dhcpv6__peer_address::spec(),
    dhcpv6__reject::spec(),
    dhcpv6__transaction_id::spec(),
    diag__test::spec(),
    diameter__avp::spec(),
    diameter__command::spec(),
    diameter__disconnect::spec(),
    diameter__drop::spec(),
    diameter__dynamic_route_insertion::spec(),
    diameter__dynamic_route_lookup::spec(),
    diameter__header::spec(),
    diameter__host::spec(),
    diameter__is_request::spec(),
    diameter__is_response::spec(),
    diameter__is_retransmission::spec(),
    diameter__length::spec(),
    diameter__message::spec(),
    diameter__payload::spec(),
    diameter__persist::spec(),
    diameter__realm::spec(),
    diameter__respond::spec(),
    diameter__result::spec(),
    diameter__retransmission::spec(),
    diameter__retransmission_default::spec(),
    diameter__retransmission_reason::spec(),
    diameter__retransmit::spec(),
    diameter__retry::spec(),
    diameter__route_status::spec(),
    diameter__session::spec(),
    diameter__skip_capabilities_exchange::spec(),
    diameter__state::spec(),
    discard::spec(),
    dns__additional::spec(),
    dns__answer::spec(),
    dns__authority::spec(),
    dns__class::spec(),
    dns__disable::spec(),
    dns__drop::spec(),
    dns__edns0::spec(),
    dns__enable::spec(),
    dns__header::spec(),
    dns__is_wideip::spec(),
    dns__last_act::spec(),
    dns__len::spec(),
    dns__log::spec(),
    dns__name::spec(),
    dns__origin::spec(),
    dns__ptype::spec(),
    dns__query::spec(),
    dns__question::spec(),
    dns__rdata::spec(),
    dns__return::spec(),
    dns__rpz_policy::spec(),
    dns__rr::spec(),
    dns__scrape::spec(),
    dns__tsig::spec(),
    dns__ttl::spec(),
    dns__type::spec(),
    dnsmsg__header::spec(),
    dnsmsg__record::spec(),
    dnsmsg__section::spec(),
    domain::spec(),
    dosl7__disable::spec(),
    dosl7__enable::spec(),
    dosl7__health::spec(),
    dosl7__is_ip_slowdown::spec(),
    dosl7__is_mitigated::spec(),
    dosl7__profile::spec(),
    dosl7__slowdown::spec(),
    drop::spec(),
    dslite__remote_addr::spec(),
    eca__client_machine_name::spec(),
    eca__disable::spec(),
    eca__domainname::spec(),
    eca__enable::spec(),
    eca__select::spec(),
    eca__status::spec(),
    eca__username::spec(),
    event::spec(),
    fasthash::spec(),
    findclass::spec(),
    findstr::spec(),
    fix__tag::spec(),
    flow__create_related::spec(),
    flow__idle_duration::spec(),
    flow__idle_timeout::spec(),
    flow__peer::spec(),
    flow__priority::spec(),
    flow__refresh::spec(),
    flow__this::spec(),
    flowtable__count::spec(),
    flowtable__limit::spec(),
    forward::spec(),
    ftp__allow_active_mode::spec(),
    ftp__disable::spec(),
    ftp__enable::spec(),
    ftp__enforce_tls_session_reuse::spec(),
    ftp__ftps_mode::spec(),
    ftp__port::spec(),
    genericmessage__message::spec(),
    genericmessage__peer::spec(),
    genericmessage__route::spec(),
    getfield::spec(),
    gtp__clone::spec(),
    gtp__discard::spec(),
    gtp__forward::spec(),
    gtp__header::spec(),
    gtp__ie::spec(),
    gtp__length::spec(),
    gtp__message::spec(),
    gtp__new::spec(),
    gtp__parse::spec(),
    gtp__payload::spec(),
    gtp__respond::spec(),
    gtp__tunnel::spec(),
    ha__status::spec(),
    hsl__open::spec(),
    hsl__send::spec(),
    html__comment::spec(),
    html__disable::spec(),
    html__enable::spec(),
    html__encode::spec(),
    html__tag::spec(),
    html_encode::spec(),
    html_escape::spec(),
    htmlencode::spec(),
    htonl::spec(),
    htons::spec(),
    http2__active::spec(),
    http2__concurrency::spec(),
    http2__disable::spec(),
    http2__disconnect::spec(),
    http2__enable::spec(),
    http2__header::spec(),
    http2__push::spec(),
    http2__requests::spec(),
    http2__stream::spec(),
    http2__version::spec(),
    http_context(http__class::spec()),
    http_context(http__close::spec()),
    http_context(http__collect::spec()),
    http_context(http__cookie::spec()),
    http_context(http__disable::spec()),
    http_context(http__enable::spec()),
    http_context(http__fallback::spec()),
    http__has_responded::spec(),
    http_context(http__header::spec()),
    http_context(http__host::spec()),
    http_context(http__hsts::spec()),
    http_context(http__is_keepalive::spec()),
    http_context(http__is_redirect::spec()),
    http_context(http__method::spec()),
    http_context(http__passthrough_reason::spec()),
    http_context(http__password::spec()),
    http_context(http__path::spec()),
    http_context(http__payload::spec()),
    http_context(http__proxy::spec()),
    http_context(http__query::spec()),
    http_context(http__redirect::spec()),
    http_context(http__reject_reason::spec()),
    http_context(http__release::spec()),
    http_context(http__request::spec()),
    http_context(http__request_num::spec()),
    http_context(http__respond::spec()),
    http_context(http__response::spec()),
    http_context(http__retry::spec()),
    http_context(http__status::spec()),
    http_context(http__uri::spec()),
    http_context(http__username::spec()),
    http_context(http__version::spec()),
    http_client_ip::spec(),
    http_content_len_max::spec(),
    http_cookie::spec(),
    http_header::spec(),
    http_host::spec(),
    http_method::spec(),
    http_uri::spec(),
    http_version::spec(),
    httplog__disable::spec(),
    httplog__enable::spec(),
    icap__header::spec(),
    icap__method::spec(),
    icap__status::spec(),
    icap__uri::spec(),
    ifile::spec(),
    ike__auth_success::spec(),
    ike__cert::spec(),
    ike__san_dirname::spec(),
    ike__san_dns::spec(),
    ike__san_ediparty::spec(),
    ike__san_email::spec(),
    ike__san_ipadd::spec(),
    ike__san_othername::spec(),
    ike__san_rid::spec(),
    ike__san_uri::spec(),
    ike__san_x400::spec(),
    ike__subjectaltname::spec(),
    ilx__call::spec(),
    ilx__init::spec(),
    ilx__notify::spec(),
    imap__activation_mode::spec(),
    imap__disable::spec(),
    imap__enable::spec(),
    imid::spec(),
    ip__addr::spec(),
    ip__client_addr::spec(),
    ip__hops::spec(),
    ip__idle_timeout::spec(),
    ip__ingress_drop_rate::spec(),
    ip__ingress_rate_limit::spec(),
    ip__intelligence::spec(),
    ip__local_addr::spec(),
    ip__protocol::spec(),
    ip__remote_addr::spec(),
    ip__reputation::spec(),
    ip__server_addr::spec(),
    ip__stats::spec(),
    ip__tos::spec(),
    ip__ttl::spec(),
    ip__version::spec(),
    ip_addr::spec(),
    ip_protocol::spec(),
    ip_tos::spec(),
    ip_ttl::spec(),
    ipfix__destination::spec(),
    ipfix__msg::spec(),
    ipfix__template::spec(),
    isession__deduplication::spec(),
    istats__get::spec(),
    istats__incr::spec(),
    istats__remove::spec(),
    istats__set::spec(),
    ivs_entry__result::spec(),
    json__array::spec(),
    json__create::spec(),
    json__get::spec(),
    json__object::spec(),
    json__parse::spec(),
    json__render::spec(),
    json__root::spec(),
    json__set::spec(),
    json__type::spec(),
    l7check__protocol::spec(),
    lasthop::spec(),
    lb__bias::spec(),
    lb__class::spec(),
    lb__command::spec(),
    lb__connect::spec(),
    lb__connlimit::spec(),
    lb__context_id::spec(),
    lb__detach::spec(),
    lb__down::spec(),
    lb__dst_tag::spec(),
    lb__enable_decisionlog::spec(),
    lb__mode::spec(),
    lb__persist::spec(),
    lb__prime::spec(),
    lb__queue::spec(),
    lb__reselect::spec(),
    lb__select::spec(),
    lb__server::spec(),
    lb__snat::spec(),
    lb__src_tag::spec(),
    lb__status::spec(),
    lb__up::spec(),
    ldap__activation_mode::spec(),
    ldap__disable::spec(),
    ldap__enable::spec(),
    line__get::spec(),
    line__set::spec(),
    link__lasthop::spec(),
    link__nexthop::spec(),
    link__qos::spec(),
    link__vlan_id::spec(),
    link_qos::spec(),
    listen::spec(),
    llookup::spec(),
    local_addr::spec(),
    local_port::spec(),
    log::spec(),
    lsn__address::spec(),
    lsn__disable::spec(),
    lsn__inbound::spec(),
    lsn__inbound_entry::spec(),
    lsn__persistence::spec(),
    lsn__persistence_entry::spec(),
    lsn__pool::spec(),
    lsn__port::spec(),
    matchclass::spec(),
    md4::spec(),
    md5::spec(),
    members::spec(),
    message__field::spec(),
    message__proto::spec(),
    message__type::spec(),
    mqtt__clean_session::spec(),
    mqtt__client_id::spec(),
    mqtt__collect::spec(),
    mqtt__disable::spec(),
    mqtt__disconnect::spec(),
    mqtt__drop::spec(),
    mqtt__dup::spec(),
    mqtt__enable::spec(),
    mqtt__insert::spec(),
    mqtt__keep_alive::spec(),
    mqtt__length::spec(),
    mqtt__message::spec(),
    mqtt__packet_id::spec(),
    mqtt__password::spec(),
    mqtt__payload::spec(),
    mqtt__protocol_name::spec(),
    mqtt__protocol_version::spec(),
    mqtt__qos::spec(),
    mqtt__release::spec(),
    mqtt__replace::spec(),
    mqtt__respond::spec(),
    mqtt__retain::spec(),
    mqtt__return_code::spec(),
    mqtt__return_code_list::spec(),
    mqtt__session_present::spec(),
    mqtt__topic::spec(),
    mqtt__type::spec(),
    mqtt__username::spec(),
    mqtt__will::spec(),
    mr__always_match_port::spec(),
    mr__available_for_routing::spec(),
    mr__collect::spec(),
    mr__connect_back_port::spec(),
    mr__connection_instance::spec(),
    mr__connection_mode::spec(),
    mr__equivalent_transport::spec(),
    mr__flow_id::spec(),
    mr__ignore_peer_port::spec(),
    mr__instance::spec(),
    mr__max_retries::spec(),
    mr__message::spec(),
    mr__payload::spec(),
    mr__peer::spec(),
    mr__prime::spec(),
    mr__protocol::spec(),
    mr__release::spec(),
    mr__restore::spec(),
    mr__retry::spec(),
    mr__return::spec(),
    mr__store::spec(),
    mr__stream::spec(),
    mr__transport::spec(),
    name__lookup::spec(),
    name__response::spec(),
    nexthop::spec(),
    node::spec(),
    nodes::spec(),
    nsh__chain::spec(),
    nsh__context::spec(),
    nsh__md1::spec(),
    nsh__mocksf::spec(),
    nsh__path_id::spec(),
    nsh__service_index::spec(),
    ntlm__disable::spec(),
    ntlm__enable::spec(),
    ntohl::spec(),
    ntohs::spec(),
    offbox__request::spec(),
    oneconnect__detach::spec(),
    oneconnect__label::spec(),
    oneconnect__reuse::spec(),
    oneconnect__select::spec(),
    pcp__reject::spec(),
    pcp__request::spec(),
    pcp__response::spec(),
    peer::spec(),
    pem__disable::spec(),
    pem__enable::spec(),
    pem__flow::spec(),
    pem__session::spec(),
    pem__subscriber::spec(),
    pem_dtos::spec(),
    persist::spec(),
    plugin__disable::spec(),
    plugin__enable::spec(),
    policy__controls::spec(),
    policy__names::spec(),
    policy__rules::spec(),
    policy__targets::spec(),
    pool::spec(),
    pop3__activation_mode::spec(),
    pop3__disable::spec(),
    pop3__enable::spec(),
    priority::spec(),
    proc::spec(),
    profile__access::spec(),
    profile__antifraud::spec(),
    profile__auth::spec(),
    profile__avr::spec(),
    profile__clientssl::spec(),
    profile__diameter::spec(),
    profile__exchange::spec(),
    profile__exists::spec(),
    profile__fasthttp::spec(),
    profile__fastl4::spec(),
    profile__ftp::spec(),
    profile__http::spec(),
    profile__httpclass::spec(),
    profile__httpcompression::spec(),
    profile__list::spec(),
    profile__oneconnect::spec(),
    profile__persist::spec(),
    profile__serverssl::spec(),
    profile__stream::spec(),
    profile__tcp::spec(),
    profile__tftp::spec(),
    profile__udp::spec(),
    profile__vdi::spec(),
    profile__webacceleration::spec(),
    profile__xml::spec(),
    protocol_inspection__disable::spec(),
    protocol_inspection__id::spec(),
    psc__aaa_reporting_interval::spec(),
    psc__attr::spec(),
    psc__calling_id::spec(),
    psc__imeisv::spec(),
    psc__imsi::spec(),
    psc__ip_address::spec(),
    psc__lease_time::spec(),
    psc__policy::spec(),
    psc__subscriber_id::spec(),
    psc__tower_id::spec(),
    psc__user_name::spec(),
    psm__ftp__disable::spec(),
    psm__ftp__enable::spec(),
    psm__http__disable::spec(),
    psm__http__enable::spec(),
    psm__smtp__disable::spec(),
    psm__smtp__enable::spec(),
    qoe__disable::spec(),
    qoe__enable::spec(),
    qoe__video::spec(),
    radius__avp::spec(),
    radius__code::spec(),
    radius__id::spec(),
    radius__rtdom::spec(),
    radius__subscriber::spec(),
    radius_authenticate::spec(),
    rateclass::spec(),
    recv::spec(),
    redirect::spec(),
    reject::spec(),
    relate_client::spec(),
    relate_server::spec(),
    remote_addr::spec(),
    remote_port::spec(),
    resolv__lookup::spec(),
    resolver__name_lookup::spec(),
    resolver__summarize::spec(),
    rest__send::spec(),
    rewrite__disable::spec(),
    rewrite__enable::spec(),
    rewrite__payload::spec(),
    rewrite__post_process::spec(),
    rmd160::spec(),
    route__age::spec(),
    route__bandwidth::spec(),
    route__clear::spec(),
    route__cwnd::spec(),
    route__domain::spec(),
    route__expiration::spec(),
    route__mtu::spec(),
    route__rtt::spec(),
    route__rttvar::spec(),
    rtsp__collect::spec(),
    rtsp__header::spec(),
    rtsp__method::spec(),
    rtsp__msg_source::spec(),
    rtsp__payload::spec(),
    rtsp__release::spec(),
    rtsp__respond::spec(),
    rtsp__status::spec(),
    rtsp__uri::spec(),
    rtsp__version::spec(),
    sctp__client_port::spec(),
    sctp__collect::spec(),
    sctp__local_port::spec(),
    sctp__mss::spec(),
    sctp__payload::spec(),
    sctp__ppi::spec(),
    sctp__release::spec(),
    sctp__remote_port::spec(),
    sctp__respond::spec(),
    sctp__rto_initial::spec(),
    sctp__rto_max::spec(),
    sctp__rto_min::spec(),
    sctp__sack_timeout::spec(),
    sctp__server_port::spec(),
    sdp__field::spec(),
    sdp__media::spec(),
    sdp__session_id::spec(),
    send::spec(),
    server_addr::spec(),
    server_port::spec(),
    serverside::spec(),
    session::spec(),
    sha1::spec(),
    sha256::spec(),
    sha384::spec(),
    sha512::spec(),
    sharedvar::spec(),
    sip__call_id::spec(),
    sip__discard::spec(),
    sip__from::spec(),
    sip__header::spec(),
    sip__message::spec(),
    sip__method::spec(),
    sip__payload::spec(),
    sip__persist::spec(),
    sip__record_route::spec(),
    sip__respond::spec(),
    sip__response::spec(),
    sip__route::spec(),
    sip__route_status::spec(),
    sip__to::spec(),
    sip__uri::spec(),
    sip__via::spec(),
    sipalg__hairpin::spec(),
    sipalg__hairpin_default::spec(),
    sipalg__nonregister_subscriber_listener::spec(),
    smtps__activation_mode::spec(),
    smtps__disable::spec(),
    smtps__enable::spec(),
    snat::spec(),
    snatpool::spec(),
    socks__allowed::spec(),
    socks__destination::spec(),
    socks__version::spec(),
    sse__field::spec(),
    ssl__allow_dynamic_record_sizing::spec(),
    ssl__allow_nonssl::spec(),
    ssl__alpn::spec(),
    ssl__authenticate::spec(),
    ssl__c3d::spec(),
    ssl__cert::spec(),
    ssl__cert_constraint::spec(),
    ssl__cipher::spec(),
    ssl__clientrandom::spec(),
    ssl__collect::spec(),
    ssl__disable::spec(),
    ssl__enable::spec(),
    ssl__extensions::spec(),
    ssl__forward_proxy::spec(),
    ssl__handshake::spec(),
    ssl__is_renegotiation_secure::spec(),
    ssl__maximum_record_size::spec(),
    ssl__mode::spec(),
    ssl__modssl_sessionid_headers::spec(),
    ssl__nextproto::spec(),
    ssl__payload::spec(),
    ssl__profile::spec(),
    ssl__release::spec(),
    ssl__renegotiate::spec(),
    ssl__respond::spec(),
    ssl__secure_renegotiation::spec(),
    ssl__session::spec(),
    ssl__sessionid::spec(),
    ssl__sessionsecret::spec(),
    ssl__sessionticket::spec(),
    ssl__sni::spec(),
    ssl__tls13_secret::spec(),
    ssl__unclean_shutdown::spec(),
    ssl__verify_result::spec(),
    stats__get::spec(),
    stats__incr::spec(),
    stats__set::spec(),
    stats__setmax::spec(),
    stats__setmin::spec(),
    stream__disable::spec(),
    stream__enable::spec(),
    stream__encoding::spec(),
    stream__expression::spec(),
    stream__match::spec(),
    stream__max_matchsize::spec(),
    stream__replace::spec(),
    substr::spec(),
    table::spec(),
    tap__action::spec(),
    tap__config::spec(),
    tap__insight::spec(),
    tap__insight_requested::spec(),
    tap__score::spec(),
    tcp__abc::spec(),
    tcp__analytics::spec(),
    tcp__autowin::spec(),
    tcp__bandwidth::spec(),
    tcp__client_port::spec(),
    tcp__close::spec(),
    tcp__collect::spec(),
    tcp__congestion::spec(),
    tcp__delayed_ack::spec(),
    tcp__dsack::spec(),
    tcp__earlyrxmit::spec(),
    tcp__ecn::spec(),
    tcp__enhanced_loss_recovery::spec(),
    tcp__idletime::spec(),
    tcp__keepalive::spec(),
    tcp__limxmit::spec(),
    tcp__local_port::spec(),
    tcp__lossfilter::spec(),
    tcp__lossfilterburst::spec(),
    tcp__lossfilterrate::spec(),
    tcp__mss::spec(),
    tcp__nagle::spec(),
    tcp__naglemode::spec(),
    tcp__naglestate::spec(),
    tcp__notify::spec(),
    tcp__offset::spec(),
    tcp__option::spec(),
    tcp__pacing::spec(),
    tcp__payload::spec(),
    tcp__proxybuffer::spec(),
    tcp__proxybufferhigh::spec(),
    tcp__proxybufferlow::spec(),
    tcp__push_flag::spec(),
    tcp__rcv_scale::spec(),
    tcp__rcv_size::spec(),
    tcp__recvwnd::spec(),
    tcp__release::spec(),
    tcp__remote_port::spec(),
    tcp__respond::spec(),
    tcp__rexmt_thresh::spec(),
    tcp__rt_metrics_timeout::spec(),
    tcp__rto::spec(),
    tcp__rtt::spec(),
    tcp__rttvar::spec(),
    tcp__sendbuf::spec(),
    tcp__server_port::spec(),
    tcp__setmss::spec(),
    tcp__snd_cwnd::spec(),
    tcp__snd_scale::spec(),
    tcp__snd_ssthresh::spec(),
    tcp__snd_wnd::spec(),
    tcp__unused_port::spec(),
    tcpdump::spec(),
    tds__msg::spec(),
    tds__session::spec(),
    timing::spec(),
    tmm__cmp_count::spec(),
    tmm__cmp_group::spec(),
    tmm__cmp_groups::spec(),
    tmm__cmp_primary_group::spec(),
    tmm__cmp_unit::spec(),
    traffic_group::spec(),
    translate::spec(),
    udp__client_port::spec(),
    udp__debug_queue::spec(),
    udp__drop::spec(),
    udp__hold::spec(),
    udp__local_port::spec(),
    udp__max_buf_pkts::spec(),
    udp__max_rate::spec(),
    udp__mss::spec(),
    udp__payload::spec(),
    udp__release::spec(),
    udp__remote_port::spec(),
    udp__respond::spec(),
    udp__sendbuffer::spec(),
    udp__server_port::spec(),
    udp__unused_port::spec(),
    uniq_ordered_ip_list::spec(),
    uniq_sorted_ip_list::spec(),
    uri__basename::spec(),
    uri__compare::spec(),
    uri__decode::spec(),
    uri__encode::spec(),
    uri__encode_component::spec(),
    uri__escape::spec(),
    uri__host::spec(),
    uri__path::spec(),
    uri__port::spec(),
    uri__protocol::spec(),
    uri__query::spec(),
    urlcatblindquery::spec(),
    urlcatquery::spec(),
    use_::spec(),
    validate__protocol::spec(),
    vdi__disable::spec(),
    vdi__enable::spec(),
    virtual_::spec(),
    vlan_id::spec(),
    wam__disable::spec(),
    wam__enable::spec(),
    websso__disable::spec(),
    websso__enable::spec(),
    websso__select::spec(),
    when::spec(),
    whereis::spec(),
    ws__collect::spec(),
    ws__disconnect::spec(),
    ws__enabled::spec(),
    ws__frame::spec(),
    ws__masking::spec(),
    ws__message::spec(),
    ws__payload::spec(),
    ws__payload_ivs::spec(),
    ws__payload_processing::spec(),
    ws__release::spec(),
    ws__request::spec(),
    ws__response::spec(),
    x509__cert_fields::spec(),
    x509__extensions::spec(),
    x509__hash::spec(),
    x509__issuer::spec(),
    x509__not_valid_after::spec(),
    x509__not_valid_before::spec(),
    x509__pem2der::spec(),
    x509__serial_number::spec(),
    x509__signature_algorithm::spec(),
    x509__subject::spec(),
    x509__subject_public_key::spec(),
    x509__subject_public_key_rsa_bits::spec(),
    x509__subject_public_key_type::spec(),
    x509__verify_cert_error_string::spec(),
    x509__version::spec(),
    x509__whole::spec(),
    xff_list::spec(),
    xff_uniq_ordered_ip_list::spec(),
    xff_uniq_sorted_ip_list::spec(),
    xlat__listen::spec(),
    xlat__listen_lifetime::spec(),
    xlat__src_addr::spec(),
    xlat__src_config::spec(),
    xlat__src_endpoint_reservation::spec(),
    xlat__src_nat_valid_range::spec(),
    xlat__src_port::spec(),
    xml__address::spec(),
    xml__collect::spec(),
    xml__disable::spec(),
    xml__element::spec(),
    xml__enable::spec(),
    xml__event::spec(),
    xml__eventid::spec(),
    xml__parse::spec(),
    xml__payload::spec(),
    xml__release::spec(),
    xml__soap::spec(),
    xml__subscribe::spec(),
];

#[cfg(test)]
mod spec_runtime_test;
