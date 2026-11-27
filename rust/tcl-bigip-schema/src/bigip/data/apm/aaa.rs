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

//! BIG-IP object specs for `apm aaa`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod active_directory;
mod active_directory_trusted_domains;
mod crldp;
mod endpoint_management_system;
mod f5_mfa_configuration;
mod f5_service_connector;
mod http;
mod http_connector_request;
mod kerberos;
mod kerberos_keytab_file;
mod ldap;
mod oam;
mod oauth_provider;
mod oauth_request;
mod oauth_server;
mod ocsp;
mod okta_connector;
mod radius;
mod saml;
mod saml_idp_automation;
mod saml_idp_connector;
mod securid;
mod tacacsplus;

/// The spec tables of `apm aaa`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        active_directory::SPECS,
        active_directory_trusted_domains::SPECS,
        crldp::SPECS,
        endpoint_management_system::SPECS,
        f5_mfa_configuration::SPECS,
        f5_service_connector::SPECS,
        http::SPECS,
        http_connector_request::SPECS,
        kerberos::SPECS,
        kerberos_keytab_file::SPECS,
        ldap::SPECS,
        oam::SPECS,
        oauth_provider::SPECS,
        oauth_request::SPECS,
        oauth_server::SPECS,
        ocsp::SPECS,
        okta_connector::SPECS,
        radius::SPECS,
        saml::SPECS,
        saml_idp_automation::SPECS,
        saml_idp_connector::SPECS,
        securid::SPECS,
        tacacsplus::SPECS,
    ]
}
