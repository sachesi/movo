package org.movo.app

import org.movo.app.core.SessionStore
import org.movo.app.core.accountMigrations
import android.content.Context
import androidx.datastore.preferences.core.PreferenceDataStoreFactory
import androidx.datastore.preferences.preferencesDataStoreFile
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The account store moved from SharedPreferences to DataStore. What matters is that an upgrade
 * keeps what the old file held: losing it signs the account out and forgets every resume point.
 */
@RunWith(RobolectricTestRunner::class)
class SessionStoreTest {
    private val context = ApplicationProvider.getApplicationContext<Context>()

    /**
     * A store of this test's own. The production one is a single instance for the whole process,
     * so whichever test touched it first would decide whether the migration had already run.
     */
    private fun store(name: String) = PreferenceDataStoreFactory.create(
        migrations = accountMigrations(context, name),
        produceFile = { context.preferencesDataStoreFile(name) },
    )

    @Test
    fun theOldPreferencesFileIsCarriedAcross() = runTest {
        val name = "account-migration-test"
        context.getSharedPreferences(name, Context.MODE_PRIVATE).edit()
            .putString("session", "ciphertext-from-an-older-build")
            .putLong("progress:u1:7:0:0", 42_000L)
            .putString("lastep:u1:7", "3|11|900")
            .putString("search-history:u1", """["dune","arrival"]""")
            .putStringSet("seen-notifications:u1", setOf("/films/dune|new episode"))
            .commit()

        val store = SessionStore(context, store(name))

        assertEquals(42_000L, store.progress("u1:7:0:0"))
        assertEquals(Triple(3L, 11L, 900L), store.lastWatchedEpisode("u1", 7))
        assertEquals(listOf("dune", "arrival"), store.searchHistory("u1"))
        assertEquals(setOf("/films/dune|new episode"), store.seenNotifications("u1"))
    }
}
