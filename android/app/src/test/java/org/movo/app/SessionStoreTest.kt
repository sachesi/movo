package org.movo.app

import org.movo.app.core.SessionStore
import org.movo.app.core.isUnreadable
import android.security.keystore.KeyPermanentlyInvalidatedException
import android.content.Context
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.preferencesDataStoreFile
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import java.security.InvalidKeyException
import java.security.KeyStoreException
import java.security.ProviderException
import javax.crypto.AEADBadTagException

@RunWith(RobolectricTestRunner::class)
class SessionStoreTest {
    private val context = ApplicationProvider.getApplicationContext<Context>()

    /**
     * A store of this test's own. The production one is a single instance for the whole process,
     * so the tests would otherwise see each other's writes.
     */
    private fun store(name: String) = PreferenceDataStoreFactory.create(
        produceFile = { context.preferencesDataStoreFile(name) },
    )

    @Test
    fun aMovieRemembersItsVoiceOverWithoutAnEpisode() = runTest {
        val store = SessionStore(context, store("account-movie-test"))

        store.saveLastEpisode("u1", 8, null, null, 56)

        assertEquals(Triple(null, null, 56L), store.lastWatchedEpisode("u1", 8))
    }

    /** Signing the user out is for a session that is gone, not for a keystore that faltered. */
    @Test
    fun onlyASessionThatCanNeverBeDecryptedIsForgotten() {
        assertTrue(isUnreadable(AEADBadTagException()))
        assertTrue(isUnreadable(KeyPermanentlyInvalidatedException()))
        assertTrue(isUnreadable(IllegalArgumentException()))
        assertFalse(isUnreadable(InvalidKeyException()))
        assertFalse(isUnreadable(KeyStoreException()))
        assertFalse(isUnreadable(ProviderException()))
    }

    @Test
    fun removingAHistoryTitleClearsOnlyItsResumePoints() = runTest {
        val store = SessionStore(context, store("account-remove-history-test"))
        store.saveProgress("u1:7:1:2", 42_000L)
        store.saveProgress("u1:8:1:2", 84_000L)

        store.clearProgressForMedia("u1", 7)

        assertEquals(0L, store.progress("u1:7:1:2"))
        assertEquals(84_000L, store.progress("u1:8:1:2"))
    }
}
