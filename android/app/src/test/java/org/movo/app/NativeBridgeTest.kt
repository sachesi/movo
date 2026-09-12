package org.movo.app

import org.movo.app.core.CoreTransport
import org.movo.app.core.NativeBridge
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.advanceUntilIdle
import kotlinx.coroutines.test.runCurrent
import kotlinx.coroutines.test.runTest
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.long
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlin.coroutines.CoroutineContext

/**
 * Holds every request until [release] opens, and notes what the bridge sent and dropped. Gives up
 * holding after a while, so a bridge that waits for it fails the test instead of hanging it.
 */
private class StuckCore : CoreTransport {
    val started = CountDownLatch(1)
    val release = CountDownLatch(1)
    val sent = CopyOnWriteArrayList<String>()
    val dropped = CopyOnWriteArrayList<Long>()

    override fun send(request: String): String {
        sent += request
        started.countDown()
        release.await(10, TimeUnit.SECONDS)
        return """{"data":null}"""
    }

    override fun cancel(id: Long) {
        dropped += id
    }
}

/** Runs what is dispatched to it only when told to, one piece at a time. */
private class ManualDispatcher : CoroutineDispatcher() {
    private val queue = ArrayDeque<Runnable>()

    override fun dispatch(context: CoroutineContext, block: Runnable) {
        synchronized(queue) { queue.addLast(block) }
    }

    fun runNext() = synchronized(queue) { queue.removeFirst() }.run()

    fun runAll() {
        while (synchronized(queue) { queue.isNotEmpty() }) runNext()
    }
}

@OptIn(ExperimentalCoroutinesApi::class)
class NativeBridgeTest {
    private val core = StuckCore()

    @Before
    fun setUp() {
        NativeBridge.transport = core
    }

    @After
    fun tearDown() {
        core.release.countDown()
        NativeBridge.transport = null
        NativeBridge.dispatcher = Dispatchers.IO
    }

    /**
     * What every timeout around a request relies on, the one on the startup restore among them.
     * The core answers only once the test lets it, so a caller held to the core stays unfinished.
     */
    @Test
    fun aCallerThatStopsWaitingIsLetGoAndTheCoreAskedToDropIt() = runTest {
        val call = launch(Dispatchers.Default) { NativeBridge.call("home") }
        assertTrue("the request never reached the core", core.started.await(5, TimeUnit.SECONDS))

        call.cancel()
        val letGo = eventually { call.isCompleted }
        core.release.countDown()

        assertTrue("the cancelled caller was held until the core answered", letGo)
        val id = NativeBridge.json.parseToJsonElement(core.sent.single())
            .jsonObject.getValue("request_id").jsonPrimitive.long
        assertEquals(listOf(id), core.dropped.toList())
    }

    /** Whether [condition] holds within a few seconds of real time. */
    private fun eventually(condition: () -> Boolean): Boolean {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(5)
        while (!condition()) {
            if (System.nanoTime() > deadline) return false
            Thread.sleep(10)
        }
        return true
    }

    @Test
    fun aRequestAbandonedBeforeItLeavesNeverReachesTheCore() = runTest {
        val bridge = ManualDispatcher()
        NativeBridge.dispatcher = bridge
        val call = launch { NativeBridge.call("home") }
        runCurrent()
        // Builds the request and queues it for the core, and no further.
        bridge.runNext()

        call.cancel()
        bridge.runAll()
        advanceUntilIdle()

        assertTrue(call.isCancelled)
        assertEquals(emptyList<String>(), core.sent.toList())
        assertEquals(emptyList<Long>(), core.dropped.toList())
    }
}
