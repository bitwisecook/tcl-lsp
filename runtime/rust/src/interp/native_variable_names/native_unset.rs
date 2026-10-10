// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unset retains the selected physical receiver and original reporting operands.

use super::*;
use tcl_syntax::naming::{
    NativeVariableDiagnosticReason as Reason, NativeVariableFailureSite as Site,
    NativeVariableInputForm as Input,
};

struct UnsetReport {
    original: Option<*mut TclObj>,
    root: Vec<u8>,
    element: Option<Vec<u8>>,
    combined: bool,
}

impl Interp {
    pub(crate) fn unset_original_c_variable(
        &mut self,
        original: *mut TclObj,
        complain: bool,
    ) -> Result<(), Code> {
        self.unset_original_c_parts(original, None, complain)
    }

    /// The two original stack operands remain owned by the caller. No combined
    /// object is created, and part1's existing array parser cache is bypassed.
    pub(crate) fn unset_original_c_parts(
        &mut self,
        original: *mut TclObj,
        element: Option<*mut TclObj>,
        complain: bool,
    ) -> Result<(), Code> {
        let purpose = unset_purpose(complain);
        let element_bytes = element
            .map(|element| {
                self.native_string_bytes(&element)
                    .map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let lookup = (|| {
            let selected = self.prepare_original_c_name_parts(
                original,
                purpose,
                None,
                element_bytes.as_deref(),
            )?;
            let Some(selected) = selected else {
                return Ok(None);
            };
            Ok(self
                .capture_original_c_parts_selection(original, element, &selected, purpose)?
                .map(|(receiver, home)| OriginalCVariableCapture {
                    receiver,
                    home,
                    root: selected.root,
                    element: selected.element,
                }))
        })();
        let capture = match lookup {
            Ok(capture) => capture,
            Err(code) if complain || self.host_refusal_pending() => return Err(code),
            Err(_) => return Ok(()),
        };
        let report = UnsetReport {
            original: Some(original),
            root: capture
                .as_ref()
                .map_or_else(Vec::new, |capture| capture.root.clone()),
            element: element_bytes,
            combined: element.is_none(),
        };
        self.unset_original_c_capture(capture, report, complain)
    }

    /// An indexed opcode selects its actual slot, including duplicate formal
    /// names. Canonical name bytes are reporting data and never select storage.
    pub(in crate::interp) fn unset_original_c_indexed(
        &mut self,
        slot: usize,
        root: &[u8],
        element: Option<*mut TclObj>,
        complain: bool,
    ) -> Result<(), Code> {
        let element = element
            .map(|element| {
                self.native_string_bytes(&element)
                    .map(|bytes| bytes.to_vec())
            })
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let report = UnsetReport {
            original: None,
            root: root.to_vec(),
            element: element.clone(),
            combined: false,
        };
        let capture = crate::vars::capture_original_indexed_receiver(
            &mut self.frames.borrow_mut(),
            &mut self.namespaces.borrow_mut(),
            slot,
            element.clone(),
            false,
        );
        let capture = match capture {
            Ok(capture) => capture.map(|(receiver, home)| OriginalCVariableCapture {
                receiver,
                home,
                root: root.to_vec(),
                element,
            }),
            Err(crate::frame::VarError::NameProtocolUnavailable) => {
                return Err(self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("actual indexed unset receiver").into(),
                ));
            }
            Err(error) => {
                return self.finish_original_unset_receiver_failure(&report, complain, error);
            }
        };
        self.unset_original_c_capture(capture, report, complain)
    }

    fn unset_original_c_capture(
        &mut self,
        capture: Option<OriginalCVariableCapture>,
        report: UnsetReport,
        complain: bool,
    ) -> Result<(), Code> {
        let Some(capture) = capture else {
            return self.finish_original_unset_failure(
                &report,
                complain,
                Reason::NoSuchVariable,
                Site::NameLookup,
            );
        };
        let receiver = capture.receiver;
        if receiver.is_constant() {
            return self.finish_original_unset_failure(
                &report,
                complain,
                Reason::Constant,
                Site::ValueUnset,
            );
        }
        let observes = self.original_variable_trace_requires_name(
            &capture.home,
            capture.element.as_deref(),
            b"unset",
        );
        let spelling = if observes {
            self.original_unset_spelling(&report)?
        } else {
            capture.root.clone()
        };
        if let Some(array) = receiver.begin_array_destruction() {
            self.finish_array_unset(&spelling, &capture.root, Some(capture.home), array);
            return if self.host_refusal_pending() {
                Err(Code::Error)
            } else {
                Ok(())
            };
        }
        let reason = if capture.element.is_some() && receiver.is_array() {
            Reason::NoSuchElement
        } else {
            Reason::NoSuchVariable
        };
        let existed = match receiver.unset() {
            Ok(existed) => existed,
            Err(crate::frame::VarError::NameProtocolUnavailable) => {
                return Err(self.report_cmd_error(
                    ValueError::CommandProtocolUnavailable("selected unset receiver").into(),
                ));
            }
            Err(error) => {
                return self.finish_original_unset_receiver_failure(&report, complain, error);
            }
        };
        if observes {
            self.fire_selected_unset_callbacks(
                &capture.home,
                &capture.root,
                capture.element.as_deref(),
                &spelling,
                !report.combined,
            );
        } else {
            // Destruction retires read/write registrations even when no unset
            // callback needs the original name, including an active read trace.
            self.detach_destroyed_trace_group(&capture.home, capture.element.as_deref());
        }
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        if existed {
            Ok(())
        } else {
            self.finish_original_unset_failure(&report, complain, reason, Site::NameLookup)
        }
    }

    fn original_unset_spelling(&mut self, report: &UnsetReport) -> Result<Vec<u8>, Code> {
        let mut spelling = match report.original {
            Some(original) => self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?
                .to_vec(),
            None => report.root.clone(),
        };
        if !report.combined {
            if let Some(element) = &report.element {
                spelling.push(b'(');
                spelling.extend_from_slice(element);
                spelling.push(b')');
            }
        }
        Ok(spelling)
    }

    fn finish_original_unset_receiver_failure(
        &mut self,
        report: &UnsetReport,
        complain: bool,
        error: crate::frame::VarError,
    ) -> Result<(), Code> {
        if !complain {
            return Ok(());
        }
        let bytes = match report.original {
            Some(original) => self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?
                .to_vec(),
            None => report.root.clone(),
        };
        let input = if report.combined {
            Input::Combined(&bytes)
        } else {
            Input::Separate {
                root: &bytes,
                element: report.element.as_deref(),
            }
        };
        Err(self.original_c_variable_receiver_error(
            input,
            unset_purpose(true),
            error,
            Some(Site::NameLookup),
        ))
    }

    fn finish_original_unset_failure(
        &mut self,
        report: &UnsetReport,
        complain: bool,
        reason: Reason,
        site: Site,
    ) -> Result<(), Code> {
        if !complain {
            return Ok(());
        }
        let bytes = match report.original {
            Some(original) => self
                .native_string_bytes(&original)
                .map_err(|error| self.report_cmd_error(error.into()))?
                .to_vec(),
            None => report.root.clone(),
        };
        let input = if report.combined {
            Input::Combined(&bytes)
        } else {
            Input::Separate {
                root: &bytes,
                element: report.element.as_deref(),
            }
        };
        Err(self.original_c_variable_failure_input(input, unset_purpose(true), reason, site))
    }
}

fn unset_purpose(complain: bool) -> NativeVariableNameLookupPurpose {
    if complain {
        NativeVariableNameLookupPurpose::Unset
    } else {
        NativeVariableNameLookupPurpose::QuietUnset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn interpreter(engine: &str) -> Interp {
        Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("actual native core issuer")
    }

    #[test]
    fn original_unset_execution_matches_85_native_completion_windows() {
        let mut compared = 0;
        for row in include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_unset_compilation/windows.tsv"
        )
        .lines()
        .skip(1)
        {
            let columns: Vec<_> = row.split('\t').collect();
            if columns[0] == "jim0.84" {
                continue;
            }
            let mut interp = interpreter(&format!("tcl{}", columns[0]));
            let body = String::from_utf8(decode(columns[2])).unwrap();
            let source = format!(
                "proc f {{name other names}} {{\n{body}\n}}\nset ::seen BEFORE\nset code [catch {{f x y {{x y}}}} value]\nlist RESULT $code $value $::seen"
            );
            let code = interp.eval_str(source.as_bytes());
            assert_eq!(code, Code::Ok, "{}:{}", columns[0], columns[1]);
            assert_eq!(
                interp.result_bytes(),
                decode(columns[5]),
                "{}:{}",
                columns[0],
                columns[1]
            );
            assert!(
                !interp.host_refusal_pending(),
                "{}:{}",
                columns[0],
                columns[1]
            );
            compared += 1;
        }
        assert_eq!(compared, 85);
    }

    #[test]
    fn original_unset_traces_and_operand_order_match_36_native_results() {
        let mut compared = 0;
        for (engine, rows) in [
            (
                "tcl8.4",
                include_str!("../../../tests/data/native_unset_execution/8.4.tsv"),
            ),
            (
                "tcl8.5",
                include_str!("../../../tests/data/native_unset_execution/8.5.tsv"),
            ),
            (
                "tcl8.6",
                include_str!("../../../tests/data/native_unset_execution/8.6.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../tests/data/native_unset_execution/9.0.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../tests/data/native_unset_execution/9.1.tsv"),
            ),
            (
                "jim",
                include_str!("../../../tests/data/native_unset_execution/jim.tsv"),
            ),
        ] {
            for row in rows.lines() {
                let columns: Vec<_> = row.split('\t').collect();
                let mut interp = interpreter(engine);
                let code = interp.eval_str(&decode(columns[1]));
                assert_eq!(
                    code.as_int(),
                    columns[2].parse::<i64>().unwrap(),
                    "{engine}/{}",
                    columns[0]
                );
                assert_eq!(
                    interp.result_bytes(),
                    decode(columns[3]),
                    "{engine}/{}",
                    columns[0]
                );
                assert!(!interp.host_refusal_pending(), "{engine}/{}", columns[0]);
                compared += 1;
            }
        }
        assert_eq!(compared, 36);
    }
    #[test]
    fn original_unset_options_match_66_native_results() {
        fn decode(hex: &str) -> Vec<u8> {
            hex.as_bytes()
                .chunks_exact(2)
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect()
        }
        let mut compared = 0;
        for (engine, rows) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_unset_options/8.4.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_unset_options/8.5.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_unset_options/8.6.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_unset_options/9.0.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_unset_options/9.1.tsv"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_unset_options/jim.tsv"
                ),
            ),
        ] {
            for row in rows.lines() {
                let columns: Vec<_> = row.split('\t').collect();
                let source = String::from_utf8(decode(columns[1])).unwrap();
                // Match the native probe's same-interpreter binary observer.
                let observed = format!(
                    "set c [catch {{{source}}} m];binary scan $m H* h;set observed \"$c\\t$h\""
                );
                let expected = format!("{}\t{}", columns[2], columns[3]);
                let mut interp = interpreter(engine);
                if engine == "jim" {
                    assert_eq!(interp.eval_str(b"unset"), Code::Ok);
                    assert_eq!(interp.result_bytes(), b"");
                    assert!(!interp.host_refusal_pending());
                    // The native observer runs under jimsh's binary extension;
                    // that distribution-owned script is absent from core entry.
                    crate::cmd_binary::install(&mut interp);
                }
                assert_eq!(
                    interp.eval_str(observed.as_bytes()),
                    Code::Ok,
                    "{engine}/{}",
                    columns[0]
                );
                assert_eq!(
                    interp.result_bytes(),
                    expected.as_bytes(),
                    "{engine}/{}",
                    columns[0]
                );
                assert!(!interp.host_refusal_pending(), "{engine}/{}", columns[0]);
                compared += 1;
            }
        }
        assert_eq!(compared, 66);
    }
}
