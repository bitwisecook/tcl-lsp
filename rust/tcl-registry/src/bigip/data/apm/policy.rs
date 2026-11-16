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

//! BIG-IP object specs for `apm policy`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod aaa_active_directory;
mod aaa_client_cert;
mod aaa_crldp;
mod aaa_http;
mod aaa_ldap;
mod aaa_oauth;
mod aaa_radius;
mod aaa_saml;
mod aaa_securid;
mod acct_radius;
mod acct_tacacsplus;
mod api_authentication;
mod api_server_selection;
mod decision_box;
mod dynamic_acl;
mod ending_allow;
mod ending_deny;
mod ending_redirect;
mod endpoint_check_machine_cert;
mod endpoint_check_software;
mod endpoint_linux_check_file;
mod endpoint_linux_check_process;
mod endpoint_mac_check_file;
mod endpoint_mac_check_process;
mod endpoint_machine_info;
mod endpoint_windows_browser_cache_cleaner;
mod endpoint_windows_check_file;
mod endpoint_windows_check_process;
mod endpoint_windows_check_registry;
mod endpoint_windows_group_policy;
mod endpoint_windows_info_os;
mod endpoint_windows_protected_workspace;
mod external_logon_page;
mod http_header_modify;
mod ip_geolocation_lookup;
mod ip_reputation_lookup;
mod irule_event;
mod kerberos;
mod l7_protocol_lookup;
mod logging;
mod logon_page;
mod message_box;
mod oam;
mod oauth_authz;
mod request_classification;
mod resource_assign;
mod response_selection;
mod route_domain_selection;
mod server_cert_response_control;
mod server_cert_status;
mod session_check;
mod ssl_check;
mod tacacsplus;
mod variable_assign;

/// The spec tables of `apm policy`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        aaa_active_directory::SPECS,
        aaa_client_cert::SPECS,
        aaa_crldp::SPECS,
        aaa_http::SPECS,
        aaa_ldap::SPECS,
        aaa_oauth::SPECS,
        aaa_radius::SPECS,
        aaa_saml::SPECS,
        aaa_securid::SPECS,
        acct_radius::SPECS,
        acct_tacacsplus::SPECS,
        api_authentication::SPECS,
        api_server_selection::SPECS,
        decision_box::SPECS,
        dynamic_acl::SPECS,
        ending_allow::SPECS,
        ending_deny::SPECS,
        ending_redirect::SPECS,
        endpoint_check_machine_cert::SPECS,
        endpoint_check_software::SPECS,
        endpoint_linux_check_file::SPECS,
        endpoint_linux_check_process::SPECS,
        endpoint_mac_check_file::SPECS,
        endpoint_mac_check_process::SPECS,
        endpoint_machine_info::SPECS,
        endpoint_windows_browser_cache_cleaner::SPECS,
        endpoint_windows_check_file::SPECS,
        endpoint_windows_check_process::SPECS,
        endpoint_windows_check_registry::SPECS,
        endpoint_windows_group_policy::SPECS,
        endpoint_windows_info_os::SPECS,
        endpoint_windows_protected_workspace::SPECS,
        external_logon_page::SPECS,
        http_header_modify::SPECS,
        ip_geolocation_lookup::SPECS,
        ip_reputation_lookup::SPECS,
        irule_event::SPECS,
        kerberos::SPECS,
        l7_protocol_lookup::SPECS,
        logging::SPECS,
        logon_page::SPECS,
        message_box::SPECS,
        oam::SPECS,
        oauth_authz::SPECS,
        request_classification::SPECS,
        resource_assign::SPECS,
        response_selection::SPECS,
        route_domain_selection::SPECS,
        server_cert_response_control::SPECS,
        server_cert_status::SPECS,
        session_check::SPECS,
        ssl_check::SPECS,
        tacacsplus::SPECS,
        variable_assign::SPECS,
    ]
}
