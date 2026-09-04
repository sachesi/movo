package org.movo.app

import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.movo.app.core.Tab
import org.movo.app.home.OVERFLOW_TABS
import org.movo.app.home.PRIMARY_TABS
import org.movo.app.ui.toTvColorScheme
import org.junit.Test

class NavigationLayoutTest {

    @Test
    fun everyTabButTheAccountIsReachableFromTheBottomBar() {
        // The account page opens from the user's name in the top bar.
        assertEquals(Tab.entries.toSet() - Tab.Account, (PRIMARY_TABS + OVERFLOW_TABS).toSet())
        assertEquals(Tab.entries.size - 1, PRIMARY_TABS.size + OVERFLOW_TABS.size)
    }

    @Test
    fun theBottomBarStaysWithinFiveItems() {
        // The primary tabs plus the one that opens the rest.
        assertTrue(PRIMARY_TABS.size + 1 <= 5)
    }

    @Test
    fun theTvSchemeIsTheAppScheme() {
        val dark = darkColorScheme()
        val tvDark = dark.toTvColorScheme(isDark = true)
        assertEquals(dark.primary, tvDark.primary)
        assertEquals(dark.onPrimary, tvDark.onPrimary)
        assertEquals(dark.surface, tvDark.surface)
        assertEquals(dark.onSurfaceVariant, tvDark.onSurfaceVariant)
        assertEquals(dark.errorContainer, tvDark.errorContainer)

        val light = lightColorScheme()
        assertEquals(light.primary, light.toTvColorScheme(isDark = false).primary)
    }
}
