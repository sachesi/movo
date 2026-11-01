package org.movo.app

import android.graphics.Bitmap
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onAllNodesWithText
import androidx.compose.ui.test.onFirst
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performTextInput
import androidx.lifecycle.ViewModelProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.After
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.movo.app.core.MovoViewModel
import org.movo.app.core.SessionStore
import org.movo.app.core.Tab
import java.io.File

private const val WAIT_MS = 30_000L

/**
 * The installed app on a device: real keystore, real Compose, real core, with the fake provider on
 * the host. `tests/headless/device.sh` builds the app to reach it and passes `movoProvider`; without
 * that argument these skip themselves, since the core would be talking to the real site.
 */
@RunWith(AndroidJUnit4::class)
class ProviderDeviceTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    private val instrumentation get() = InstrumentationRegistry.getInstrumentation()
    private val model get() = ViewModelProvider(compose.activity)[MovoViewModel::class.java]

    @Before
    fun needsTheProvider() {
        assumeTrue(InstrumentationRegistry.getArguments().getString("movoProvider") != null)
    }

    private fun expect(text: String) {
        compose.waitUntil(WAIT_MS) {
            compose.onAllNodes(hasText(text, substring = true), useUnmergedTree = true)
                .fetchSemanticsNodes().isNotEmpty()
        }
    }

    private fun shot(name: String) {
        compose.waitForIdle()
        val folder = compose.activity.getExternalFilesDir("shots") ?: return
        File(folder, "$name.png").outputStream().use {
            instrumentation.uiAutomation.takeScreenshot()?.compress(Bitmap.CompressFormat.PNG, 100, it)
        }
    }

    private fun text(id: Int) = compose.activity.getString(id)

    private fun signIn(password: String = "secret") {
        compose.waitUntil(WAIT_MS) {
            compose.onAllNodes(hasText(text(R.string.login_hint))).fetchSemanticsNodes().isNotEmpty()
        }
        compose.onNodeWithText(text(R.string.login_hint)).performTextInput("tester")
        compose.onNodeWithText(text(R.string.password_hint)).performTextInput(password)
        compose.onNodeWithText(text(R.string.sign_in)).performClick()
    }

    @After
    fun signOut() {
        if (model.state.value.user != null) {
            compose.runOnUiThread { model.logout() }
            compose.waitUntil(WAIT_MS) { model.state.value.user == null }
        }
    }

    @Test
    fun aWrongPasswordIsRefusedInTheProvidersWords() {
        signIn(password = "wrong")
        expect("Неверный логин или пароль")
        shot("sign-in-refused")
    }

    @Test
    fun signingInOpensTheCatalogue() {
        signIn()
        expect("Тихий поезд 1")
        shot("home")
    }

    @Test
    fun aSignedInSessionComesBackThroughTheKeystore() {
        signIn()
        compose.waitUntil(WAIT_MS) { model.state.value.user != null }
        assertEquals("Тестер", model.state.value.user?.username)

        val stored = runBlocking { SessionStore(compose.activity).secret() }
        assertNotNull("the session should decrypt with the keystore's key", stored)

        // Signing in lands on the catalogue; a tab chosen before that would be overruled.
        expect("Тихий поезд 1")
        compose.runOnUiThread { model.selectTab(Tab.History, isTv = false) }
        expect("Ночной дом 5")
        shot("history")

        val restarted = MovoViewModel(compose.activity.application)
        compose.waitUntil(WAIT_MS) { restarted.state.value.user != null }
        assertEquals("42", restarted.state.value.user?.userId)
    }

    @Test
    fun aDetailsPageOpensFromHome() {
        signIn()
        expect("Тихий поезд 1")
        compose.onAllNodesWithText("Тихий поезд 1").onFirst().performClick()
        expect("Описание тайтла номер 1")
        shot("details-series")
    }
}
