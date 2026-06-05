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
            "ilx_workspace",
            module="ilx",
            object_types=("workspace",),
        ),
        header_types=(("ilx", "workspace"),),
        properties=(
            BigipPropertySpec(name="archive", value_type="reference"),
            BigipPropertySpec(name="extension", value_type="reference"),
            BigipPropertySpec(
                name="file",
                value_type="reference",
                references=(
                    "apm_aaa_kerberos_keytab_file",
                    "apm_policy_agent_endpoint_linux_check_file",
                    "apm_policy_agent_endpoint_mac_check_file",
                    "apm_policy_agent_endpoint_windows_check_file",
                    "apm_policy_image_file",
                    "apm_policy_windows_group_policy_file",
                    "apm_resource_remote_desktop_citrix_client_package_file",
                    "ltm_classification_urldb_file",
                    "ltm_tacdb_customdb_file",
                    "security_dos_autodos_file_object",
                    "security_dos_l4bdos_file_object",
                    "security_http_file_type",
                    "sys_file_apache_ssl_cert",
                    "sys_file_browser_capabilities_db",
                    "sys_file_data_group",
                    "sys_file_device_capabilities_db",
                    "sys_file_external_monitor",
                    "sys_file_ifile",
                    "sys_file_lwtunneltbl",
                    "sys_file_rewrite_rule",
                    "sys_file_ssl_cert",
                    "sys_file_ssl_crl",
                    "sys_file_ssl_key",
                ),
            ),
            BigipPropertySpec(name="from-archive", value_type="reference"),
            BigipPropertySpec(name="from-plugin", value_type="reference"),
            BigipPropertySpec(name="from-uri", value_type="unknown"),
            BigipPropertySpec(name="from-workspace", value_type="reference"),
            BigipPropertySpec(name="node-version", value_type="unknown"),
            BigipPropertySpec(
                name="rule",
                value_type="reference",
                references=(
                    "gtm_rule",
                    "ltm_cipher_rule",
                    "ltm_global_settings_rule",
                    "ltm_rule",
                    "ltm_rule_profiler",
                    "security_firewall_matching_rule",
                    "security_firewall_on_demand_rule_deploy",
                    "security_firewall_rule_list",
                    "security_firewall_rule_stat",
                    "security_packet_filter_rule_stat",
                    "sys_file_rewrite_rule",
                ),
            ),
        ),
    )
