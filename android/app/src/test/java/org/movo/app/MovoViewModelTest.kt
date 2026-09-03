package org.movo.app

import org.movo.app.core.CoreTransport
import org.movo.app.core.MovoViewModel
import org.movo.app.core.NativeBridge
import org.movo.app.core.Tab
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.test.StandardTestDispatcher
import kotlinx.coroutines.test.advanceTimeBy
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.resetMain
import kotlinx.coroutines.test.runTest
import kotlinx.coroutines.test.setMain
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** Answers bridge requests from a canned table and records what was asked. */
private class FakeCore(private val replies: Map<String, String>) : CoreTransport {
    val requested = mutableListOf<String>()

    override fun send(request: String): String {
        val type = NativeBridge.json.parseToJsonElement(request)
            .let { it as kotlinx.serialization.json.JsonObject }
            .getValue("type")
            .let { (it as kotlinx.serialization.json.JsonPrimitive).content }
        requested += type
        return replies[type] ?: """{"error":"no reply for $type"}"""
    }
}

private const val CATALOG_REPLY =
    """{"data":[{"id":1,"title":"Dune","url":"/films/dune"}]}"""
private const val SUGGESTIONS_REPLY = """{"data":["dune","dune two"]}"""

/** Comfortably shorter than the debounce, so four keystrokes still make one request. */
private const val SUGGEST_KEYSTROKE_GAP_MS = 50L

@OptIn(ExperimentalCoroutinesApi::class)
@RunWith(RobolectricTestRunner::class)
class MovoViewModelTest {
    private val core = FakeCore(
        mapOf("catalog" to CATALOG_REPLY, "search_suggestions" to SUGGESTIONS_REPLY),
    )

    private fun model(scheduler: kotlinx.coroutines.test.TestCoroutineScheduler): MovoViewModel {
        val dispatcher = StandardTestDispatcher(scheduler)
        Dispatchers.setMain(dispatcher)
        NativeBridge.transport = core
        NativeBridge.dispatcher = dispatcher
        return MovoViewModel(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        Dispatchers.resetMain()
        NativeBridge.transport = null
        NativeBridge.dispatcher = Dispatchers.IO
    }

    @Test
    fun typingASuggestionDoesNotCancelTheLoadBesideIt() = runTest {
        val model = model(testScheduler)
        model.loadCatalog()
        model.suggest("du")
        advanceUntilIdle()

        assertEquals(listOf("Dune"), model.state.value.items.map { it.title })
    }

    @Test
    fun typingSendsOneSuggestionRequestPerPause() = runTest {
        val model = model(testScheduler)
        model.selectTab(Tab.Search, isTv = false)
        // Time has to pass between the keystrokes: without it every launch is cancelled before it
        // is ever dispatched, and the test would pass whether or not the typing is debounced.
        "dune".forEach {
            model.suggest("query$it")
            advanceTimeBy(SUGGEST_KEYSTROKE_GAP_MS)
        }
        advanceUntilIdle()

        assertEquals(1, core.requested.count { it == "search_suggestions" })
    }
}
