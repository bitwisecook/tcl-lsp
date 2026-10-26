// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independently supplied host C numeric conversion and thread range state.

/// Actual C thread error state at a reached observation point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericErrorState {
    /// Platform errno value, retained rather than normalized to a baseline.
    pub errno: i32,
    /// Whether that exact state is the selected host's EDOM.
    pub domain_error: bool,
    /// Whether that exact state is the selected host's ERANGE.
    pub range_error: bool,
}

/// A reached unsigned C conversion, including its real thread-state effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnsignedNumericConversion {
    /// Exact C unsigned result widened from the selected long or long-long worker.
    pub value: u64,
    /// End-pointer byte offset in the original counted input.
    pub end: usize,
    /// State before executing the native conversion.
    pub before: NumericErrorState,
    /// State after the conversion; successful libc calls need not clear errno.
    pub after: NumericErrorState,
}

/// A reached signed C long conversion, retaining real thread-state effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedNumericConversion {
    /// Exact signed long result under the independently checked host layout.
    pub value: i64,
    /// End-pointer byte offset in the original counted input.
    pub end: usize,
    /// Actual thread state before the C call, without an implicit reset.
    pub before: NumericErrorState,
    /// Actual thread state afterward; a successful call need not clear errno.
    pub after: NumericErrorState,
}

/// A reached double C conversion with an explicit errno-reset purpose.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoubleNumericConversion {
    /// Exact double returned by the C operation.
    pub value: f64,
    /// End-pointer byte offset in the original counted input.
    pub end: usize,
    /// Actual state before any requested reset.
    pub before: NumericErrorState,
    /// Actual state after parsing, including underflow and overflow.
    pub after: NumericErrorState,
}

/// Unavailable target ABI or invalid conversion geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericEnvironmentUnavailable {
    /// This host cannot independently supply the selected native thread state.
    Target,
    /// The requested offset/base cannot describe a valid C conversion call.
    Conversion,
}

/// Descriptive integer layout of the actual selected host C target.
/// These facts authenticate no interpreter, object header, handler or source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeCIntegerAbi {
    /// Bits per actual C `char`, independently of source Unicode grammar.
    pub char_bits: u8,
    /// Actual C `sizeof(int)` in C characters.
    pub int_bytes: u8,
    /// Actual C `sizeof(long)` in C characters.
    pub long_bytes: u8,
}

/// Host-owned numeric thread state shared by interpreters and host callbacks.
/// There is no interpreter-local errno mirror. Authored hosts can provide a
/// separate implementation; absence does not establish a zero/range-free state.
pub trait NumericEnvironment {
    /// Report this independently supplied host's actual C integer layout.
    /// Default absence supplies neither a width nor a primitive getter recipe.
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// Observe the actual current thread state without resetting it.
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable>;

    /// Execute the explicit reset selected by a reached native conversion.
    /// Cache hits and error-state observations must never call this operation.
    fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// C8.4 unsigned conversion after its independently selected reset/sign
    /// handling. `long` requires strtoul. A wide request uses strtoul when
    /// the host C long and wide types agree, otherwise strtoull.
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        let _ = (input, offset, long);
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// Execute strtoull on the selected original input suffix. `offset` is
    /// measured before the first NUL; base is 0 or 2..=36. Does not reset errno.
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable>;

    /// Execute strtoul on the selected original input suffix. This is an
    /// independently selected unsigned-long stage, not the base-zero C8.4
    /// primitive or an unsigned-long-long conversion. It never resets errno.
    /// Default absence supplies no target operation or primitive recipe.
    fn unsigned_long(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        let _ = (input, offset, base);
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// Execute strtol on original input under the actual host long layout.
    /// Base is zero or 2..=36. This operation never resets errno; availability
    /// and the caller's native purpose remain independent of input spelling.
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<SignedNumericConversion, NumericEnvironmentUnavailable> {
        let _ = (input, base);
        Err(NumericEnvironmentUnavailable::Target)
    }

    /// Execute strtod on original input. Reset is an explicit caller-selected
    /// native operation, not an inferred property of the current errno.
    fn double(
        &self,
        input: &[u8],
        reset_errno: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable>;
}

/// Forward actual numeric stages under a supplied callback-neutral check.
/// The checker supplies currency only; this adapter issues no conversion recipe.
/// Its independently supplied ABI cell rejects changed observations.
pub struct CheckedNumericEnvironment<'a> {
    actual: &'a dyn NumericEnvironment,
    check: &'a dyn Fn() -> Result<(), NumericEnvironmentUnavailable>,
    abi: &'a std::cell::Cell<Option<NativeCIntegerAbi>>,
}

impl<'a> CheckedNumericEnvironment<'a> {
    /// Retain the actual supplied environment, checker and ABI observation cell.
    #[must_use]
    pub const fn new(
        actual: &'a dyn NumericEnvironment,
        check: &'a dyn Fn() -> Result<(), NumericEnvironmentUnavailable>,
        abi: &'a std::cell::Cell<Option<NativeCIntegerAbi>>,
    ) -> Self {
        Self { actual, check, abi }
    }

    fn call<T>(
        &self,
        operation: impl FnOnce() -> Result<T, NumericEnvironmentUnavailable>,
    ) -> Result<T, NumericEnvironmentUnavailable> {
        (self.check)()?;
        let reached = operation();
        (self.check)()?;
        reached
    }
}

impl NumericEnvironment for CheckedNumericEnvironment<'_> {
    fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
        let actual = self.call(|| self.actual.c_integer_abi())?;
        if self.abi.get().is_some_and(|original| original != actual) {
            return Err(NumericEnvironmentUnavailable::Target);
        }
        self.abi.set(Some(actual));
        Ok(actual)
    }
    fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
        self.call(|| self.actual.state())
    }
    fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
        self.call(|| self.actual.reset())
    }
    fn unsigned_c84(
        &self,
        input: &[u8],
        offset: usize,
        long: bool,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        self.call(|| self.actual.unsigned_c84(input, offset, long))
    }
    fn unsigned(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        self.call(|| self.actual.unsigned(input, offset, base))
    }
    fn unsigned_long(
        &self,
        input: &[u8],
        offset: usize,
        base: u32,
    ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
        self.call(|| self.actual.unsigned_long(input, offset, base))
    }
    fn signed_long(
        &self,
        input: &[u8],
        base: u32,
    ) -> Result<SignedNumericConversion, NumericEnvironmentUnavailable> {
        self.call(|| self.actual.signed_long(input, base))
    }
    fn double(
        &self,
        input: &[u8],
        reset: bool,
    ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
        self.call(|| self.actual.double(input, reset))
    }
}

#[cfg(test)]
mod checked_environment_tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    type Call = (&'static str, Vec<u8>, usize, u32, bool);
    struct Actual {
        calls: RefCell<Vec<Call>>,
        abi: Cell<NativeCIntegerAbi>,
    }
    fn state() -> NumericErrorState {
        NumericErrorState {
            errno: 123,
            domain_error: false,
            range_error: true,
        }
    }
    impl Actual {
        fn new() -> Self {
            Self {
                calls: RefCell::new(Vec::new()),
                abi: Cell::new(NativeCIntegerAbi {
                    char_bits: 8,
                    int_bytes: 4,
                    long_bytes: 8,
                }),
            }
        }
        fn record(&self, name: &'static str, input: &[u8], offset: usize, base: u32, flag: bool) {
            self.calls
                .borrow_mut()
                .push((name, input.to_vec(), offset, base, flag));
        }
        fn unsigned_value() -> UnsignedNumericConversion {
            UnsignedNumericConversion {
                value: 91,
                end: 4,
                before: state(),
                after: state(),
            }
        }
    }
    impl NumericEnvironment for Actual {
        fn c_integer_abi(&self) -> Result<NativeCIntegerAbi, NumericEnvironmentUnavailable> {
            self.record("abi", &[], 0, 0, false);
            Ok(self.abi.get())
        }
        fn state(&self) -> Result<NumericErrorState, NumericEnvironmentUnavailable> {
            self.record("state", &[], 0, 0, false);
            Ok(state())
        }
        fn reset(&self) -> Result<(), NumericEnvironmentUnavailable> {
            self.record("reset", &[], 0, 0, false);
            Ok(())
        }
        fn unsigned_c84(
            &self,
            input: &[u8],
            offset: usize,
            long: bool,
        ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
            self.record("c84", input, offset, 0, long);
            Ok(Self::unsigned_value())
        }
        fn unsigned(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
            self.record("unsigned", input, offset, base, false);
            Ok(Self::unsigned_value())
        }
        fn unsigned_long(
            &self,
            input: &[u8],
            offset: usize,
            base: u32,
        ) -> Result<UnsignedNumericConversion, NumericEnvironmentUnavailable> {
            self.record("long", input, offset, base, false);
            Ok(Self::unsigned_value())
        }
        fn signed_long(
            &self,
            input: &[u8],
            base: u32,
        ) -> Result<SignedNumericConversion, NumericEnvironmentUnavailable> {
            self.record("signed", input, 0, base, false);
            Ok(SignedNumericConversion {
                value: -91,
                end: 4,
                before: state(),
                after: state(),
            })
        }
        fn double(
            &self,
            input: &[u8],
            reset: bool,
        ) -> Result<DoubleNumericConversion, NumericEnvironmentUnavailable> {
            self.record("double", input, 0, 0, reset);
            Ok(DoubleNumericConversion {
                value: 1.25,
                end: 4,
                before: state(),
                after: state(),
            })
        }
    }
    #[test]
    fn original_checked_numeric_forwarding_keeps_every_selected_stage_and_parameters() {

        // Software forwarding only; the authored worker is not a libc/provider observation.
        let actual = Actual::new();
        let abi = Cell::new(None);
        let checks = Cell::new(0);
        let check = || {
            checks.set(checks.get() + 1);
            Ok(())
        };
        let selected = CheckedNumericEnvironment::new(&actual, &check, &abi);
        assert_eq!(selected.c_integer_abi().unwrap(), actual.abi.get());
        assert_eq!(selected.state().unwrap(), state());
        selected.reset().unwrap();
        assert_eq!(
            selected.unsigned_c84(b"xx91\0tail", 2, true).unwrap(),
            Actual::unsigned_value()
        );
        assert_eq!(
            selected.unsigned(b"xx91\0tail", 2, 16).unwrap(),
            Actual::unsigned_value()
        );
        assert_eq!(
            selected.unsigned_long(b"xx91\0tail", 2, 10).unwrap(),
            Actual::unsigned_value()
        );
        assert_eq!(selected.signed_long(b"-91\0tail", 10).unwrap().value, -91);
        assert_eq!(
            selected
                .double(b"1.25\0tail", false)
                .unwrap()
                .value
                .to_bits(),
            1.25_f64.to_bits()
        );
        assert_eq!(checks.get(), 16);
        assert_eq!(abi.get(), Some(actual.abi.get()));
        assert_eq!(
            *actual.calls.borrow(),
            vec![
                ("abi", vec![], 0, 0, false),
                ("state", vec![], 0, 0, false),
                ("reset", vec![], 0, 0, false),
                ("c84", b"xx91\0tail".to_vec(), 2, 0, true),
                ("unsigned", b"xx91\0tail".to_vec(), 2, 16, false),
                ("long", b"xx91\0tail".to_vec(), 2, 10, false),
                ("signed", b"-91\0tail".to_vec(), 0, 10, false),
                ("double", b"1.25\0tail".to_vec(), 0, 0, false),
            ]
        );
    }
    #[test]
    fn original_checked_numeric_forwarding_stops_before_and_after_a_reached_worker() {

        for refuse_at in [1, 2] {
            let actual = Actual::new();
            let abi = Cell::new(None);
            let checks = Cell::new(0);
            let check = || {
                let reached = checks.get() + 1;
                checks.set(reached);
                if reached >= refuse_at {
                    Err(NumericEnvironmentUnavailable::Target)
                } else {
                    Ok(())
                }
            };
            let selected = CheckedNumericEnvironment::new(&actual, &check, &abi);
            assert_eq!(
                selected.unsigned_long(b"91", 0, 10),
                Err(NumericEnvironmentUnavailable::Target)
            );
            assert_eq!(actual.calls.borrow().len(), usize::from(refuse_at == 2));
            assert_eq!(selected.reset(), Err(NumericEnvironmentUnavailable::Target));
            assert_eq!(actual.calls.borrow().len(), usize::from(refuse_at == 2));
            assert_eq!(abi.get(), None);
        }
    }
    #[test]
    fn original_checked_numeric_forwarding_refuses_a_changed_observed_abi() {

        let actual = Actual::new();
        let old = NativeCIntegerAbi {
            char_bits: 8,
            int_bytes: 4,
            long_bytes: 4,
        };
        let abi = Cell::new(Some(old));
        let check = || Ok(());
        let selected = CheckedNumericEnvironment::new(&actual, &check, &abi);
        assert_eq!(
            selected.c_integer_abi(),
            Err(NumericEnvironmentUnavailable::Target)
        );
        assert_eq!(abi.get(), Some(old));
        assert_eq!(actual.calls.borrow().len(), 1);
    }
}
