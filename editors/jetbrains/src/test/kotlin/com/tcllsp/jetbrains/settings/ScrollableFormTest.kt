// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

package com.tcllsp.jetbrains.settings

import com.intellij.ui.components.JBCheckBox
import javax.swing.JPanel
import javax.swing.JViewport
import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * `ConfigurableCardPanel` already wraps a configurable's component in a scroll
 * pane, so the settings page must hand it a plain [Scrollable] form. Returning
 * a `JScrollPane` nested one pane inside another: the inner one was always its
 * own preferred size, so it never scrolled, and it swallowed the wheel events
 * meant for the outer one.
 */
class ScrollableFormTest {

    private fun form(): ScrollableForm =
        ScrollableForm(JPanel().apply { add(JBCheckBox("a setting with a label")) })

    @Test
    fun theFormTakesTheViewportWidthUntilItCannotBeSqueezedFurther() {
        val form = form()
        val viewport = JViewport().apply { view = form }

        viewport.setSize(form.minimumSize.width + 200, 400)
        assertTrue(
            form.scrollableTracksViewportWidth,
            "a pane wider than the form's minimum must reflow the form, not scroll it sideways",
        )

        viewport.setSize(form.minimumSize.width - 1, 400)
        assertFalse(
            form.scrollableTracksViewportWidth,
            "a pane narrower than the form's minimum must scroll rather than clip",
        )
    }

    @Test
    fun theFormScrollsVerticallyRatherThanBeingSquashedIntoThePane() {
        assertFalse(form().scrollableTracksViewportHeight)
    }
}
