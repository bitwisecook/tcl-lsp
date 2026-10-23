// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unset command option grammar; original operands are obtained lazily.

use tcl_syntax::native_string::NativeStringProtocol;

/// The selected command grammar, independent of its variable/cache authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsetOptionProtocol {
    /// C accepts one initial `-nocomplain` and then an optional `--`.
    C,
    /// Jim accepts repeated `-nocomplain` before an optional `--`.
    Jim084,
}

/// Original variable operands begin after this prefix, without reordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsetOptions {
    /// Index in the supplied argument slice, excluding the command name.
    pub names_from: usize,
    /// Whether failed selected variable lookups request a guest diagnostic.
    pub complain: bool,
}

impl UnsetOptionProtocol {
    /// Consume only original option candidates, stopping at the first name.
    /// The getter owns actual native string materialisation; this pure grammar
    /// does not issue a string, primary representation or variable receiver.
    ///
    /// # Errors
    /// Preserves a reached original operand getter's error.
    pub fn parse<B: AsRef<[u8]>, E>(
        self,
        count: usize,
        original_bytes: impl FnMut(usize) -> Result<B, E>,
    ) -> Result<UnsetOptions, E> {
        self.parse_with_extent(count, original_bytes, tcl_core_types::c_string_extent)
    }

    /// Known compiler words use their counted lengths; a raw NUL suffix is
    /// not an option. This does not replace the native all-word decline pass.
    ///
    /// # Errors
    /// Preserves an unavailable original source operand.
    pub fn parse_known_source<B: AsRef<[u8]>, E>(
        self,
        count: usize,
        original_bytes: impl FnMut(usize) -> Result<B, E>,
    ) -> Result<UnsetOptions, E> {
        self.parse_with_extent(count, original_bytes, |bytes| bytes)
    }

    fn parse_with_extent<B: AsRef<[u8]>, E>(
        self,
        count: usize,
        mut original_bytes: impl FnMut(usize) -> Result<B, E>,
        extent: impl Fn(&[u8]) -> &[u8],
    ) -> Result<UnsetOptions, E> {
        let mut options = UnsetOptions {
            names_from: 0,
            complain: true,
        };
        while options.names_from < count {
            let original = original_bytes(options.names_from)?;
            let bytes = extent(original.as_ref());
            if bytes == b"--" {
                options.names_from += 1;
                break;
            }
            if bytes != b"-nocomplain" {
                break;
            }
            options.complain = false;
            options.names_from += 1;
            if self == Self::C {
                if options.names_from < count {
                    let original = original_bytes(options.names_from)?;
                    if extent(original.as_ref()) == b"--" {
                        options.names_from += 1;
                    }
                }
                break;
            }
        }
        Ok(options)
    }
}

impl crate::InvocationDialect {
    /// Select command option grammar from the invocation's original domain.
    /// Logical F5 has an authored C8.4 command grammar; it receives no native
    /// string or variable cache authority from this grammar query.
    #[must_use]
    pub fn unset_option_protocol(self) -> Option<UnsetOptionProtocol> {
        match self.native_string_protocol() {
            Some(NativeStringProtocol::C(_)) => Some(UnsetOptionProtocol::C),
            Some(NativeStringProtocol::Jim084) => Some(UnsetOptionProtocol::Jim084),
            None => self
                .authored_f5_tcl84_core()
                .map(|_| UnsetOptionProtocol::C),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unset_options_keep_unknown_names_and_obtain_only_reached_originals() {
        for (protocol, args, from, complain, reached) in [
            (UnsetOptionProtocol::C, vec!["-bad", "x"], 0, true, vec![0]),
            (
                UnsetOptionProtocol::C,
                vec!["-nocomplain", "-nocomplain", "x"],
                1,
                false,
                vec![0, 1],
            ),
            (
                UnsetOptionProtocol::Jim084,
                vec!["-nocomplain", "-nocomplain", "x"],
                2,
                false,
                vec![0, 1, 2],
            ),
            (
                UnsetOptionProtocol::C,
                vec!["--", "-nocomplain"],
                1,
                true,
                vec![0],
            ),
            (
                UnsetOptionProtocol::C,
                vec!["-nocomplain", "--", "x"],
                2,
                false,
                vec![0, 1],
            ),
            (UnsetOptionProtocol::Jim084, vec![], 0, true, vec![]),
        ] {
            let mut getters = Vec::new();
            let options = protocol
                .parse(args.len(), |index| {
                    getters.push(index);
                    Ok::<_, ()>(args[index].as_bytes())
                })
                .unwrap();
            assert_eq!(
                options,
                UnsetOptions {
                    names_from: from,
                    complain
                }
            );
            assert_eq!(getters, reached);
        }
    }

    #[test]
    fn counted_compiler_option_extent_is_separate_from_runtime_cstring_extent() {
        let original = b"-nocomplain\0tail";
        let runtime = UnsetOptionProtocol::C
            .parse(1, |_| Ok::<_, ()>(original))
            .unwrap();
        let compiled = UnsetOptionProtocol::C
            .parse_known_source(1, |_| Ok::<_, ()>(original))
            .unwrap();
        assert_eq!(
            runtime,
            UnsetOptions {
                names_from: 1,
                complain: false
            }
        );
        assert_eq!(
            compiled,
            UnsetOptions {
                names_from: 0,
                complain: true
            }
        );
    }

    #[test]
    fn unset_options_preserve_getter_failure_before_variable_operations() {
        let mut getters = Vec::new();
        let failed = UnsetOptionProtocol::C.parse(3, |index| {
            getters.push(index);
            if index == 0 {
                Ok(b"-nocomplain".as_slice())
            } else {
                Err("original updater unavailable")
            }
        });
        assert_eq!(failed, Err("original updater unavailable"));
        assert_eq!(getters, [0, 1]);
    }
}
