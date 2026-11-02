package org.movo.app

import androidx.test.core.app.ActivityScenario
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertNull
import org.junit.Test
import org.junit.runner.RunWith
import java.lang.ref.WeakReference

/** An activity left reachable keeps its whole view tree alive across every rotation. */
@RunWith(AndroidJUnit4::class)
class ActivityReleaseTest {
    @Test
    fun aClosedActivityIsReleased() {
        var reference: WeakReference<MainActivity>? = null
        ActivityScenario.launch(MainActivity::class.java).use { scenario ->
            scenario.onActivity { reference = WeakReference(it) }
        }

        repeat(20) {
            if (reference?.get() != null) {
                Runtime.getRuntime().gc()
                Runtime.getRuntime().runFinalization()
                Thread.sleep(500)
            }
        }

        assertNull("the closed activity is still reachable", reference?.get())
    }
}
