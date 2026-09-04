package org.movo.app

import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.core.app.ApplicationProvider
import org.movo.app.core.AppState
import org.movo.app.core.CoreTransport
import org.movo.app.core.MediaItem
import org.movo.app.core.MovoViewModel
import org.movo.app.core.NativeBridge
import org.movo.app.core.UserProfile
import org.movo.app.core.MediaDetails
import org.movo.app.details.DetailsScreen
import org.movo.app.home.HomeFlow
import org.movo.app.settings.SettingsActions
import org.movo.app.settings.SettingsContent
import androidx.datastore.preferences.core.Preferences
import org.movo.app.settings.AppSettings
import org.junit.After
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

private object EmptyCore2 : CoreTransport {
    override fun send(request: String) = """{"error":"offline"}"""
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [23, 28, 34])
class ScreenRenderTest {
    @get:Rule val compose = createComposeRule()

    @After fun tearDown() { NativeBridge.transport = null }

    private fun model(): MovoViewModel {
        NativeBridge.transport = EmptyCore2
        return MovoViewModel(ApplicationProvider.getApplicationContext())
    }

    private val state = AppState(
        user = UserProfile(userId = "1", username = "u", loggedIn = true, vip = false),
        items = listOf(MediaItem(id = 1, title = "Dune", url = "/films/dune")),
    )

    @Test
    fun theHomeShellRendersOnAPhone() {
        val m = model()
        compose.setContent {
            MaterialTheme {
                HomeFlow(state, isTv = false, useRail = false, compactHeight = false, model = m, settings = AppSettings())
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theHomeShellRendersOnATv() {
        val m = model()
        compose.setContent {
            MaterialTheme {
                HomeFlow(state, isTv = true, useRail = true, compactHeight = false, model = m, settings = AppSettings())
            }
        }
        compose.waitForIdle()
    }

    private val details = MediaDetails(
        id = 1, title = "Dune", url = "/films/dune", description = "d",
        mediaType = "Film", genres = listOf("sci-fi"), countries = listOf("US"),
        directors = emptyList(), actors = emptyList(),
        translators = emptyList(), seasons = emptyList(), franchises = emptyList(),
    )

    @Test
    fun theDetailsScreenRendersOnAPhone() {
        val m = model()
        compose.setContent {
            MaterialTheme {
                DetailsScreen(state.copy(details = details), isTv = false, wideContent = false, settings = AppSettings(), model = m)
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theDetailsScreenRendersOnATv() {
        val m = model()
        compose.setContent {
            MaterialTheme {
                DetailsScreen(state.copy(details = details), isTv = true, wideContent = true, settings = AppSettings(), model = m)
            }
        }
        compose.waitForIdle()
    }

    @Test
    fun theSettingsScreenRenders() {
        compose.setContent {
            MaterialTheme {
                SettingsContent(
                    settings = AppSettings(),
                    isTv = false,
                    actions = object : SettingsActions {
                        override fun <T> save(key: Preferences.Key<T>, value: T) = Unit
                    },
                )
            }
        }
        compose.waitForIdle()
    }
}
