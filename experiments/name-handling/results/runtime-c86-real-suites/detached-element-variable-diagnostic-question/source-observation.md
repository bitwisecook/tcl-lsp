# Detached alias operation selection

Original C86 var-1.17 links scalar operand foo to x(1), reads, unsets the array, then writes foo. The operand has no element suffix; its link denotes an element. These facts come from actual source operations, not the diagnostic text.

`vars::capture_original_indexed_receiver` checks the genuine retained compiled link's native element entry before the store and can return `VarError::DeletedArray`. `body_capture_target_with_check` currently forwards this error at NameLookup unless it is DeletedNamespace. The central receiver reporter maps DeletedArray to DetachedElement. The central C86+ diagnostic table supports DetachedElement at ValueRead/ValueWrite, but not NameLookup. This is a source path consistent with the orchestration refusal; the exact live failing site is unmeasured.

The original C `TclPtrSetVarIdx` dead-entry setter branch initially issues LOOKUP ELEMENT for an array element, then earlyError reaches cleanup and issues WRITE VARNAME. The original lookup/read and increment flows differ: the new read probe reports NoSuchVariable/TCL READ VARNAME; increment ends with the setter's deleted-element message/WRITE VARNAME. Therefore a blanket DeletedArray->ValueWrite mapping, regardless of purpose or actual retained target kind, lacks support.

A passive Runtime probe should log the actual admitted slot and evaluated element presence, typed purpose/site/VarError, and physical link element/entry/array-kind flags at the already-reached capture. It must not derive receiver kind from foo's spelling, cached compiled-name state, or rendered message. No policy repair, Runtime source edit or build occurred for this question.
