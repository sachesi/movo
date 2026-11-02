package org.movo.app

import androidx.activity.ComponentActivity
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.graphics.asAndroidBitmap
import androidx.compose.ui.test.captureToImage
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onRoot
import androidx.test.core.app.ApplicationProvider
import android.graphics.Bitmap
import kotlinx.coroutines.runBlocking
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.FixMethodOrder
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.junit.runners.MethodSorters
import org.movo.app.core.BridgeException
import org.movo.app.core.HomeSection
import org.movo.app.core.HistoryResult
import org.movo.app.core.LoginResult
import org.movo.app.core.MediaDetails
import org.movo.app.core.MovoViewModel
import org.movo.app.core.NativeBridge
import org.movo.app.core.StreamBundle
import org.movo.app.core.UserProfile
import org.movo.app.details.DetailsScreen
import org.movo.app.settings.AppSettings
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode
import java.io.File

/**
 * The app's Kotlin, the real Rust core over JNI, and real HTTP to the fake provider. Only
 * `tests/headless/android.sh` sets up what these need, the library built for this machine and a
 * running provider, so they skip themselves anywhere else.
 */
@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@FixMethodOrder(MethodSorters.NAME_ASCENDING)
class ProviderFlowTest {
    @get:Rule val compose = createAndroidComposeRule<ComponentActivity>()

    @Before
    fun needsTheProvider() {
        assumeTrue(System.getenv("MOVO_TEST_PROVIDER") != null)
        // Other test classes leave their stand-in in place on purpose: a view model outliving its
        // test still calls the bridge, and without a transport that loads the missing library.
        NativeBridge.transport = null
    }

    private fun pageOf(path: String) = System.getenv("MOVO_TEST_PROVIDER") + path

    /**
     * The session ends either way. The core built for this machine then fails to clear the desktop
     * keyring, which is unavailable here and is never involved on Android.
     */
    private suspend fun signOut() {
        runCatching { NativeBridge.call("logout") }
    }

    private fun save(name: String) {
        val folder = File(System.getenv("MOVO_ANDROID_SHOTS") ?: return).apply { mkdirs() }
        File(folder, "$name.png").outputStream().use {
            compose.onRoot().captureToImage().asAndroidBitmap().compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }

    @Test
    fun theHomeRailsComeThroughTheCore() = runBlocking {
        val sections = NativeBridge.decode<List<HomeSection>>("home")

        assertEquals(listOf("hot", "new", "watching", "popular", "awaiting"), sections.map { it.id })
        assertEquals("Тихий поезд 1", sections[1].items[0].title)
    }

    @Test
    fun aRefusedSignInKeepsTheProvidersWords() = runBlocking {
        val refused = runCatching {
            NativeBridge.decode<LoginResult>(
                "login",
                buildJsonObject { put("login", "tester"); put("password", "wrong") },
            )
        }.exceptionOrNull() as BridgeException

        assertEquals("Неверный логин или пароль", refused.message)
    }

    @Test
    fun aSeriesListsItsSeasonsAndPlays() = runBlocking {
        val details = NativeBridge.decode<MediaDetails>(
            "details",
            buildJsonObject { put("url", pageOf("/series/drama/1-title-1991.html")) },
        )
        assertEquals(2, details.seasons.size)
        assertEquals(6, details.seasons[0].episodes.size)

        val stream = NativeBridge.decode<StreamBundle>(
            "episode_stream",
            buildJsonObject {
                put("post_id", details.id)
                put("translator_id", details.translators[0].id)
                put("season", 1)
                put("episode", 2)
            },
        )
        assertEquals("1080p Ultra", stream.streams[0].quality)
        assertTrue(stream.streams[1].urls[0].contains("/stream/1/238/1x2/720.mp4"))
    }

    /**
     * Through the bridge rather than the view model, which keeps the session in the platform
     * keystore: Robolectric has none, so that half needs a device.
     */
    @Test
    fun aSignedInSessionSurvivesBeingRestored() = runBlocking {
        val signedIn = NativeBridge.decode<LoginResult>(
            "login",
            buildJsonObject { put("login", "tester"); put("password", "secret") },
        )
        assertEquals("Тестер", signedIn.user.username)
        assertEquals("Ночной дом 5", NativeBridge.decode<HistoryResult>("history").entries[0].title)

        signOut()
        assertTrue(runCatching { NativeBridge.call("history") }.isFailure)

        val restored = NativeBridge.decode<UserProfile>("restore", buildJsonObject { put("secret", signedIn.secret) })
        assertEquals("42", restored.userId)
        assertEquals("Ночной дом 5", NativeBridge.decode<HistoryResult>("history").entries[0].title)
        signOut()
    }

    @Test
    fun aRealDetailsPageRenders() {
        val model = MovoViewModel(ApplicationProvider.getApplicationContext())
        val details = runBlocking {
            NativeBridge.decode<MediaDetails>(
                "details",
                buildJsonObject { put("url", pageOf("/films/fiction/4-title-1994.html")) },
            )
        }
        compose.setContent {
            MaterialTheme {
                DetailsScreen(
                    state = model.state.value.copy(details = details),
                    isTv = false,
                    wideContent = false,
                    settings = AppSettings(),
                    model = model,
                )
            }
        }
        compose.waitUntil(5_000) { compose.onAllNodes(hasText(details.title)).fetchSemanticsNodes().isNotEmpty() }
        save("details")
    }
}
