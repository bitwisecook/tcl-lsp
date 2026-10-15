// SPDX-License-Identifier: AGPL-3.0-or-later
//! Same-original ParseSubCmd primary with its authentic static table owner.

use super::{IntRep, Value};
use tcl_core_types::NativeJimSubcommandCache;
use tcl_syntax::value::ValueError;

impl Value {
    pub(crate) fn native_jim_subcommand_cache(&self) -> Option<NativeJimSubcommandCache> {
        match &*self.0.intrep.borrow() {
            IntRep::JimSubcommand(cache) => Some(cache.clone()),
            _ => None,
        }
    }

    pub(crate) fn install_native_jim_subcommand_cache_checked<E: From<ValueError>>(
        &self,
        cache: NativeJimSubcommandCache,
        dialect: tcl_registry::InvocationDialect,
        check: &mut dyn FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        check()?;
        self.check_native_header()?;
        let table = dialect.native_jim_namespace_subcommand_table().ok_or(
            ValueError::CommandProtocolUnavailable("Jim subcommand table issuer"),
        )?;
        if table.cached_entry(&cache).is_none() || self.resident_string_bytes().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "Jim subcommand original table or resident spelling",
            )
            .into());
        }
        self.replace_primary_using_with(
            || {
                if table.cached_entry(&cache).is_none() || self.resident_string_bytes().is_none() {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "Jim subcommand original table or resident spelling",
                    )
                    .into());
                }
                Ok(IntRep::JimSubcommand(cache))
            },
            &mut Some(check),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{rc::Rc, sync::Arc};
    use tcl_core_types::NativeIndexCache;

    #[test]
    fn subcommand_primary_retains_only_the_actual_table_birth() {
        let vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jimtcl"));
        let dialect = vm.actual_native_invocation_dialect();
        let table = dialect.native_jim_namespace_subcommand_table().unwrap();
        let original = Value::new_native_string_bytes(b"parent".as_slice());
        let foreign = NativeJimSubcommandCache::new(
            NativeIndexCache::new(
                Rc::new(table),
                std::mem::size_of::<tcl_registry::native_jim_lookup::NativeJimNamespaceSubcommand>(
                ),
                table.lookup(b"parent").unwrap().index(),
            ),
            Arc::new(()),
        );
        assert!(
            original
                .install_native_jim_subcommand_cache_checked::<ValueError>(
                    foreign,
                    dialect,
                    &mut || Ok(())
                )
                .is_err()
        );
        assert_eq!(original.native_object_type_name(), "none");
        let cache = table.lookup(b"parent").unwrap();
        original
            .install_native_jim_subcommand_cache_checked::<ValueError>(
                cache.clone(),
                dialect,
                &mut || Ok(()),
            )
            .unwrap();
        assert_eq!(original.native_object_type_name(), "subcmd-lookup");
        let duplicate = original
            .duplicate_native_object_in(tcl_syntax::native_string::NativeStringProtocol::Jim084);
        assert!(
            table
                .cached_entry(&duplicate.native_jim_subcommand_cache().unwrap())
                .is_some()
        );
        assert_eq!(
            duplicate.native_object_snapshot().cache,
            tcl_syntax::native_object::NativeObjectCacheSnapshot::JimSubcommand {
                index: cache.index()
            }
        );
    }
}
