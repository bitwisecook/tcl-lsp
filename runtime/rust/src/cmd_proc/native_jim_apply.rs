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

//! Original Jim lambda objects retain their namespace without a display getter.

use super::*;

pub(super) fn apply(
    interp: &mut Interp,
    argv: &[*mut TclObj],
    original: &Owned,
    diagnostics: tcl_registry::native_lambda::NativeLambdaDiagnosticProtocol,
) -> Code {
    let mut parts = match ValueOps::list_elements(interp, &original.as_ptr()) {
        Ok(parts) => parts.into_iter().map(Owned::retain).collect::<Vec<_>>(),
        Err(error) => {
            let error = lambda_list_error(interp, original, diagnostics, error);
            return interp.report_cmd_error(error);
        }
    };
    if !(2..=3).contains(&parts.len()) {
        let lambda = match ValueOps::native_string_bytes(interp, &original.as_ptr()) {
            Ok(lambda) => lambda,
            Err(error) => return interp.report_cmd_error(error.into()),
        };
        return interp.report_cmd_error(diagnostics.conversion_error(&lambda));
    }
    let params = match parse_params_object(interp, parts[0].as_ptr(), b"") {
        Ok(params) => params,
        Err(error) => return interp.report_cmd_error(error),
    };
    let namespace = if parts.len() == 3 {
        parts.pop().expect("actual original lambda namespace")
    } else {
        match interp.native_jim_object_context() {
            Ok(context) => context.empty_object().clone(),
            Err(error) => return interp.report_cmd_error(error.into()),
        }
    };
    let ns = crate::namespace::Namespaces::retain_unmaterialized_jim_namespace(
        &mut interp.namespaces_mut(),
        namespace,
    );
    let jim_namespace = interp.namespaces_mut().take_jim_namespace_owner(ns);
    let Some(jim_namespace) = jim_namespace else {
        return interp.report_cmd_error(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original Jim lambda namespace owner",
            )
            .into(),
        );
    };
    let body = match ValueOps::native_string_bytes(interp, &parts[1].as_ptr()) {
        Ok(body) => body,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let location = if obj::native_object_snapshot(original.as_ptr())
        .is_ok_and(|snapshot| snapshot.resident.is_some())
    {
        interp.list_element_location(original.as_ptr(), 1)
    } else {
        interp
            .arg_location(parts[1].as_ptr())
            .and_then(|(file, line)| file.map(|file| (file, line)))
    };
    let (source, body_line_base) = location.map_or((None, 0), |(file, line)| {
        (Some(file), line.saturating_sub(1))
    });
    // JimCallProcedure borrows an argv with this actual dummy head and the supplied objects.
    let head = Owned::fresh(obj::new_string_bytes(b"apply lambdaExpr"));
    let mut invocation = Vec::with_capacity(argv.len() - 1);
    invocation.push(head.as_ptr());
    invocation.extend_from_slice(&argv[2..]);
    interp.run_proc(
        &params,
        &body,
        ns,
        &argv[2..],
        b"apply lambdaExpr",
        CallMeta {
            original_argv: Some(&invocation),
            err: ProcFrame::JimLambda,
            fqn: None,
            source,
            body_line_base,
            link_vars: &[],
            oo_variable_resolver: None,
            keep_loop_codes: false,
            same_level: false,
            usage_prefix: None,
            level_words: None,
            quote_name: false,
            native: None,
            statics: None,
            c_procedure: None,
            c_method_client_data: None,
            jim_parameters: Some(parts[0].as_ptr()),
            jim_body: Some(parts[1].as_ptr()),
            jim_namespace: Some(&jim_namespace),
        },
    )
}

#[cfg(test)]
mod tests;
