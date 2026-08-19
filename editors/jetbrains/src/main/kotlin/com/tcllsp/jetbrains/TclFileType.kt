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

package com.tcllsp.jetbrains

import com.intellij.openapi.fileTypes.LanguageFileType
import com.intellij.openapi.vfs.VirtualFile
import javax.swing.Icon

class TclFileType private constructor() : LanguageFileType(TclLanguage) {
    override fun getName(): String = "Tcl"
    override fun getDescription(): String = "Tcl script file"
    override fun getDefaultExtension(): String = "tcl"
    override fun getIcon(): Icon = TclIcons.Tcl

    companion object {
        @JvmField
        val INSTANCE = TclFileType()

        // @generated:supported-extensions:begin -- cargo xtask gen-editor-extensions
        private val SUPPORTED_EXTENSIONS = setOf(
            "tcl",
            "tk",
            "itcl",
            "tm",
            "test",
            "globals",
            "exp",
            "expect",
            "scf",
            "iapp",
            "iappimpl",
            "impl",
            "irul",
            "irule",
            "irules",
            "tmsh",
            "qsf",
            "qpf",
            "qip",
            "do",
            "tclspec",
            "sdc",
            "upf",
            "xdc",
            "apl",
        )
        // @generated:supported-extensions:end

        @JvmStatic
        fun isSupported(file: VirtualFile): Boolean {
            val ext = file.extension?.lowercase() ?: return false
            return ext in SUPPORTED_EXTENSIONS
        }
    }
}
