from __future__ import annotations

from ..models import (
    BigipObjectKindSpec,
    BigipObjectSpec,
    BigipPropertySpec,
)
from ._base import register


@register
def register_spec() -> BigipObjectSpec:
    return BigipObjectSpec(
        kind_spec=BigipObjectKindSpec(
            "ltm_lsn_pool",
            module="ltm",
            object_types=("lsn-pool",),
        ),
        header_types=(("ltm", "lsn-pool"),),
        properties=(
            BigipPropertySpec(name="client-connection-limit", value_type="integer"),
            BigipPropertySpec(name="description", value_type="string"),
            BigipPropertySpec(
                name="hairpin-mode", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="icmp-echo", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(
                name="inbound-connections",
                value_type="enum",
                enum_values=("automatic", "explicit", "disabled"),
            ),
            BigipPropertySpec(name="log-publisher", value_type="boolean", allow_none=True),
            BigipPropertySpec(name="log-profile", value_type="boolean", allow_none=True),
            BigipPropertySpec(
                name="mode", value_type="enum", enum_values=("deterministic", "napt", "pba")
            ),
            BigipPropertySpec(
                name="persistence",
                value_type="reference",
                references=(
                    "ltm_persistence_cookie",
                    "ltm_persistence_dest_addr",
                    "ltm_persistence_global_settings",
                    "ltm_persistence_hash",
                    "ltm_persistence_host",
                    "ltm_persistence_msrdp",
                    "ltm_persistence_persist_records",
                    "ltm_persistence_sip",
                    "ltm_persistence_source_addr",
                    "ltm_persistence_ssl",
                    "ltm_persistence_universal",
                ),
            ),
            BigipPropertySpec(
                name="mode",
                value_type="enum",
                in_sections=("persistence",),
                allow_none=True,
                enum_values=("none", "address", "address-port"),
            ),
            BigipPropertySpec(name="timeout", value_type="integer", in_sections=("persistence",)),
            BigipPropertySpec(name="pcp", value_type="string"),
            BigipPropertySpec(
                name="profile",
                value_type="reference",
                in_sections=("pcp",),
                allow_none=True,
                references=(
                    "ltm_profile_analytics",
                    "ltm_profile_certificate_authority",
                    "ltm_profile_classification",
                    "ltm_profile_client_ldap",
                    "ltm_profile_client_ssl",
                    "ltm_profile_connector",
                    "ltm_profile_dhcpv4",
                    "ltm_profile_dhcpv6",
                    "ltm_profile_diameter",
                    "ltm_profile_dns",
                    "ltm_profile_dns_logging",
                    "ltm_profile_doh_proxy",
                    "ltm_profile_doh_server",
                    "ltm_profile_fasthttp",
                    "ltm_profile_fastl4",
                    "ltm_profile_fix",
                    "ltm_profile_ftp",
                    "ltm_profile_georedundancy",
                    "ltm_profile_gtp",
                    "ltm_profile_html",
                    "ltm_profile_http",
                    "ltm_profile_http2",
                    "ltm_profile_http3",
                    "ltm_profile_http_compression",
                    "ltm_profile_httprouter",
                    "ltm_profile_icap",
                    "ltm_profile_iiop",
                    "ltm_profile_ilx",
                    "ltm_profile_imap",
                    "ltm_profile_ipother",
                    "ltm_profile_ipsecalg",
                    "ltm_profile_json",
                    "ltm_profile_mapt",
                    "ltm_profile_mblb",
                    "ltm_profile_mqtt",
                    "ltm_profile_mr_ratelimit",
                    "ltm_profile_mr_ratelimit_action",
                    "ltm_profile_mssql",
                    "ltm_profile_netflow",
                    "ltm_profile_ntlm",
                    "ltm_profile_ocsp",
                    "ltm_profile_ocsp_stapling_params",
                    "ltm_profile_one_connect",
                    "ltm_profile_pcp",
                    "ltm_profile_pop3",
                    "ltm_profile_pptp",
                    "ltm_profile_qoe",
                    "ltm_profile_quic",
                    "ltm_profile_radius",
                    "ltm_profile_ramcache",
                    "ltm_profile_request_adapt",
                    "ltm_profile_request_log",
                    "ltm_profile_response_adapt",
                    "ltm_profile_rewrite",
                    "ltm_profile_rtsp",
                    "ltm_profile_sctp",
                    "ltm_profile_server_ldap",
                    "ltm_profile_server_ssl",
                    "ltm_profile_sip",
                    "ltm_profile_smtp",
                    "ltm_profile_smtps",
                    "ltm_profile_socks",
                    "ltm_profile_splitsessionclient",
                    "ltm_profile_splitsessionserver",
                    "ltm_profile_sse",
                    "ltm_profile_statistics",
                    "ltm_profile_stream",
                    "ltm_profile_tcp",
                    "ltm_profile_tcp_analytics",
                    "ltm_profile_tdr",
                    "ltm_profile_tftp",
                    "ltm_profile_traffic_acceleration",
                    "ltm_profile_udp",
                    "ltm_profile_wa_cache",
                    "ltm_profile_web_acceleration",
                    "ltm_profile_web_security",
                    "ltm_profile_websocket",
                    "ltm_profile_xml",
                ),
            ),
            BigipPropertySpec(
                name="selfip", value_type="boolean", in_sections=("pcp",), allow_none=True
            ),
            BigipPropertySpec(name="port-block-allocation", value_type="string"),
            BigipPropertySpec(
                name="block-idle-timeout",
                value_type="integer",
                in_sections=("port-block-allocation",),
            ),
            BigipPropertySpec(
                name="block-lifetime", value_type="integer", in_sections=("port-block-allocation",)
            ),
            BigipPropertySpec(
                name="block-size", value_type="integer", in_sections=("port-block-allocation",)
            ),
            BigipPropertySpec(
                name="client-block-limit",
                value_type="integer",
                in_sections=("port-block-allocation",),
            ),
            BigipPropertySpec(
                name="zombie-timeout", value_type="integer", in_sections=("port-block-allocation",)
            ),
            BigipPropertySpec(
                name="route-advertisement", value_type="enum", enum_values=("enabled", "disabled")
            ),
            BigipPropertySpec(name="translation-port-range", value_type="integer"),
            BigipPropertySpec(name="reset-stats", value_type="string"),
        ),
    )
