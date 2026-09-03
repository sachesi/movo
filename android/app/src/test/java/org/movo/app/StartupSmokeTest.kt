package org.movo.app

import org.movo.app.core.CoreTransport
import org.movo.app.core.NativeBridge
import org.junit.After
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config

private object EmptyCore : CoreTransport {
    override fun send(request: String) = """{"error":"offline"}"""
}

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [23, 28, 34])
class StartupSmokeTest {

    @After
    fun tearDown() {
        NativeBridge.transport = null
    }

    @Test
    fun theActivityReachesItsFirstFrame() {
        NativeBridge.transport = EmptyCore
        Robolectric.buildActivity(MainActivity::class.java).setup().use { }
    }
}
