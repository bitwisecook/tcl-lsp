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
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains

import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.openapi.wm.ToolWindowFactory
import kotlin.test.Test
import kotlin.test.assertEquals

class TclPlatformCompatibilityTest {
    @Test
    fun `tool window factories inherit platform defaults without compatibility forwarding methods`() {
        val factories = listOf(
            CompilerExplorerToolWindowFactory::class.java,
            SpecStudioToolWindowFactory::class.java,
        )
        val inheritedDefaults = setOf("isApplicable", "isDoNotActivateOnStart", "getAnchor", "getIcon", "manage")
        for (factory in factories) {
            for (method in ToolWindowFactory::class.java.methods.filter { it.name in inheritedDefaults }) {
                assertEquals(
                    method.declaringClass,
                    factory.getMethod(method.name, *method.parameterTypes).declaringClass,
                    "${factory.simpleName}.${method.name} should inherit the platform default",
                )
            }
        }
    }

    @Test
    fun `the status widget inherits the platform overload and supplies its own presentation`() {
        val widget = Class.forName("com.tcllsp.jetbrains.TclLspStatusBarWidget")
        val platformOverload = StatusBarWidget::class.java.methods.single {
            it.name == "getPresentation" && it.parameterCount == 1
        }
        assertEquals(
            StatusBarWidget::class.java,
            widget.getMethod(platformOverload.name, *platformOverload.parameterTypes).declaringClass,
        )
        assertEquals(widget, widget.getMethod("getPresentation").declaringClass)
    }
}
