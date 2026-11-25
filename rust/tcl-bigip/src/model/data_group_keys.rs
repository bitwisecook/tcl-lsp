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

//! Keys from resolved, typed BIG-IP data-group declarations.

use std::collections::BTreeMap;
use std::ops::RangeInclusive;

use super::{BigipDataGroup, ModelObject};
use crate::parser::driver::BigipConfig;

/// Typed data groups from the available configuration images.
pub struct DataGroupKeys<'a> {
    groups: BTreeMap<&'a str, Option<&'a BigipDataGroup>>,
}

impl<'a> DataGroupKeys<'a> {
    /// Index actual group declarations by their model-owned full path.
    /// Distinct declarations at the same path make that path ambiguous.
    #[must_use]
    pub fn from_configs(configs: impl IntoIterator<Item = &'a BigipConfig>) -> Self {
        let mut groups = BTreeMap::new();
        for placed in configs.into_iter().flat_map(|config| &config.objects) {
            if let ModelObject::DataGroup(group) = &placed.object {
                groups
                    .entry(placed.full_path.as_str())
                    .and_modify(|group| *group = None)
                    .or_insert(Some(group));
            }
        }
        Self { groups }
    }

    /// Find one known unsigned key in the selected integer group. A declaration
    /// in an owner partition shadows the Common fallback, including when its
    /// contents are missing or ambiguous.
    #[must_use]
    pub fn unsigned_key_in(
        &self,
        reference: &str,
        owner: &str,
        default_partition: &str,
        range: &RangeInclusive<u64>,
    ) -> Option<u64> {
        let group = crate::object_names::owner_partition_reference_paths(
            reference,
            owner,
            default_partition,
        )
        .iter()
        .find_map(|path| self.groups.get(path.as_str()))?
        .as_ref()?;
        if group.value_type != "integer" {
            return None;
        }
        group.records.iter().find_map(|record| {
            let key = record.parse::<u64>().ok()?;
            range.contains(&key).then_some(key)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::driver::parse_bigip_conf;

    #[test]
    fn workspace_group_keys_reject_ambiguous_paths_and_respect_partition_shadowing() {
        let common = parse_bigip_conf(
            "ltm data-group internal /Common/key_group { type integer\n records { 129 { data hit } } }",
            "Common",
        );
        let tenant = parse_bigip_conf(
            "ltm data-group internal /Tenant/key_group { type string\n records { 129 { data hit } } }",
            "Tenant",
        );
        let group_keys = DataGroupKeys::from_configs([&common, &tenant]);
        let unsigned = 128..=255;
        assert_eq!(
            group_keys.unsigned_key_in("key_group", "", "Common", &unsigned),
            Some(129),
        );
        assert_eq!(
            group_keys.unsigned_key_in("key_group", "/Tenant/rule", "Common", &unsigned),
            None,
        );
        assert_eq!(
            group_keys.unsigned_key_in("/Common/key_group", "/Tenant/rule", "Common", &unsigned),
            Some(129),
        );

        let duplicate = common.clone();
        assert_eq!(
            DataGroupKeys::from_configs([&common, &duplicate]).unsigned_key_in(
                "key_group",
                "",
                "Common",
                &unsigned
            ),
            None,
        );
    }
}
