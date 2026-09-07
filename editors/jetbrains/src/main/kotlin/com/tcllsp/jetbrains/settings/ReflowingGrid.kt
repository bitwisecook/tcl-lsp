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

package com.tcllsp.jetbrains.settings

import com.intellij.util.ui.JBUI
import java.awt.Component
import java.awt.Container
import java.awt.Dimension
import java.awt.LayoutManager
import javax.swing.JComponent
import javax.swing.JPanel

/** Gap between cells, before HiDPI scaling. */
private const val HGAP = 8
private const val VGAP = 2

/** Columns assumed before the grid has ever been given a width. */
private const val UNMEASURED_COLUMNS = 2

/**
 * Equal-width cells in as many columns as the available width allows.
 *
 * The settings page laid its checkbox groups out with `GridLayout(0, n)`,
 * which sizes every cell to the widest label and multiplies by `n`: the
 * optimiser's four columns of sixty-character labels came to roughly 2000px,
 * and the diagnostics groups to about half that. An IntelliJ settings pane
 * never scrolls horizontally — the platform wraps a configurable's component
 * in a vertical-only scroll pane — so every column past the pane's own width
 * was simply unreachable, and the page ran off the right-hand edge.
 *
 * Width is read from the enclosing form rather than from this grid's own
 * bounds, so a resize settles in a single pass: a container is sized before
 * its children are laid out, so by the time the form's `GridBagLayout` asks
 * this grid for a preferred size, the width the grid is about to be given is
 * already known. Reading `width` here instead would answer with the previous
 * pass's column count and leave the last row clipped after every resize.
 */
internal class ReflowingGrid(children: List<JComponent>) : JPanel() {

    init {
        layout = Reflow()
        isOpaque = false
        children.forEach { add(it) }
    }

    private inner class Reflow : LayoutManager {
        override fun addLayoutComponent(name: String?, component: Component?) = Unit

        override fun removeLayoutComponent(component: Component?) = Unit

        override fun preferredLayoutSize(parent: Container): Dimension =
            synchronized(parent.treeLock) { gridSize(columns(availableWidth())) }

        /**
         * One column. A settings pane that cannot show the grid's preferred
         * width must squeeze it rather than clip it, and one column of one
         * checkbox is as narrow as this can honestly get.
         */
        override fun minimumLayoutSize(parent: Container): Dimension =
            synchronized(parent.treeLock) { gridSize(1) }

        override fun layoutContainer(parent: Container) {
            synchronized(parent.treeLock) {
                val insets = insets
                val cell = cellSize()
                val content = width - insets.left - insets.right
                val columns = columns(if (width > 0) content else availableWidth())
                components.forEachIndexed { index, component ->
                    val column = index % columns
                    val row = index / columns
                    component.setBounds(
                        insets.left + column * (cell.width + JBUI.scale(HGAP)),
                        insets.top + row * (cell.height + JBUI.scale(VGAP)),
                        cell.width,
                        cell.height,
                    )
                }
            }
        }
    }

    /** The widest and tallest child, which every cell is sized to. */
    private fun cellSize(): Dimension {
        var width = 1
        var height = 1
        for (component in components) {
            val preferred = component.preferredSize
            width = maxOf(width, preferred.width)
            height = maxOf(height, preferred.height)
        }
        return Dimension(width, height)
    }

    private fun gridSize(columns: Int): Dimension {
        val cell = cellSize()
        val rows = (componentCount + columns - 1) / columns
        val insets = insets
        return Dimension(
            insets.left + insets.right + columns * cell.width + (columns - 1) * JBUI.scale(HGAP),
            insets.top + insets.bottom + rows * cell.height + (rows - 1).coerceAtLeast(0) * JBUI.scale(VGAP),
        )
    }

    /** Columns that fit in [content] px of content width (insets excluded). */
    private fun columns(content: Int): Int {
        if (componentCount == 0) return 1
        if (content <= 0) return UNMEASURED_COLUMNS.coerceAtMost(componentCount)
        val cell = cellSize().width + JBUI.scale(HGAP)
        return ((content + JBUI.scale(HGAP)) / cell).coerceIn(1, componentCount)
    }

    /**
     * The width this grid is about to be laid out at: the nearest ancestor
     * that has been sized, less every inset between it and here. Falls back
     * to this grid's own width when it has no sized ancestor at all.
     */
    private fun availableWidth(): Int {
        var lost = insets.left + insets.right
        var ancestor: Container? = parent
        while (ancestor != null) {
            val ancestorInsets = ancestor.insets
            lost += ancestorInsets.left + ancestorInsets.right
            if (ancestor.width > 0) return ancestor.width - lost
            ancestor = ancestor.parent
        }
        return width
    }
}
