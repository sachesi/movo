package org.movo.app.core

import org.movo.app.settings.settings
import android.content.Context
import androidx.datastore.core.DataStore
import androidx.datastore.core.IOException
import androidx.datastore.preferences.SharedPreferencesMigration
import androidx.datastore.preferences.core.Preferences
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.emptyPreferences
import androidx.datastore.preferences.core.longPreferencesKey
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.core.stringSetPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.first
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import kotlinx.coroutines.CoroutineDispatcher
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.booleanOrNull
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import kotlinx.serialization.encodeToString
import kotlinx.serialization.decodeFromString
import java.security.GeneralSecurityException
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec

/**
 * A command the core refused. [code] names the kind of failure (timeout, network, provider,
 * session, malformed, other), which is what the app puts into words; the message is for the log
 * and for a failure of no particular kind. [sessionRejected] is set when the provider turned the
 * stored session down, which is the only case where the app should forget it: a request that never
 * reached the provider says nothing about whether the session is still good.
 */
class BridgeException(message: String, val code: String, val sessionRejected: Boolean) : IllegalStateException(message)

/** Sends one serialized request to the core and returns its serialized reply. */
internal fun interface CoreTransport {
    fun send(request: String): String
}

object NativeBridge {
    val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    /**
     * The entry point into the Rust core.
     *
     * Declared here, private, and named exactly this. The symbol the core exports spells out the
     * package and class name of whatever declares it, and nothing checks the two against each
     * other at build time: moving this method to another class, marking it `internal` (which
     * mangles the name it gets on the JVM) or moving the file to another package all compile
     * cleanly and then fail with an UnsatisfiedLinkError on the app's first request.
     */
    private external fun invoke(request: String): String

    /** Holds the library load off until something actually sends, so a test can swap it out first. */
    private object JniTransport : CoreTransport {
        init { System.loadLibrary("movo_android") }

        override fun send(request: String) = invoke(request)
    }

    /**
     * Where requests go. Held behind a lazy default so the Rust library is loaded on first use
     * rather than on first touch of this object, which is what lets a JVM test swap it out.
     */
    internal var transport: CoreTransport? = null

    /**
     * Where the blocking call and the parse of its reply run. Injected rather than named inline so
     * a test can hand the work to its own scheduler and know when it has finished.
     */
    @PublishedApi
    internal var dispatcher: CoroutineDispatcher = Dispatchers.IO

    private val core: CoreTransport get() = transport ?: JniTransport

    /** Resolves [core], which loads the Rust library on the calling thread. */
    internal fun warmUp() { core }

    suspend fun call(type: String, fields: JsonObject = buildJsonObject {}): String = withContext(dispatcher) {
        val request = buildJsonObject { put("type", type); fields.forEach { (key, value) -> put(key, value) } }
        val response = json.parseToJsonElement(core.send(request.toString())).jsonObject
        response["error"]?.jsonPrimitive?.content?.let {
            throw BridgeException(
                it,
                response["code"]?.jsonPrimitive?.content ?: "other",
                response["rejected"]?.jsonPrimitive?.booleanOrNull == true,
            )
        }
        response.getValue("data").toString()
    }

    /**
     * Runs [type] on the core and parses its payload, both off the main thread. Callers must use
     * this rather than decoding the [call] string themselves: catalog pages and media details are
     * large enough that parsing them on the UI thread drops frames.
     */
    suspend inline fun <reified T> decode(type: String, fields: JsonObject = buildJsonObject {}): T =
        withContext(dispatcher) { json.decodeFromString<T>(call(type, fields)) }
}

/**
 * Loads the Rust library from a worker thread, so the first real request does not pay for the
 * dlopen on whichever thread happens to make it.
 */
suspend fun warmUpNativeBridge() = withContext(Dispatchers.IO) { NativeBridge.warmUp() }

/**
 * Account-scoped storage: the encrypted session, playback progress, and the per-user search and
 * notification history.
 *
 * Backed by the same DataStore the settings use, with a one-shot migration off the
 * SharedPreferences file earlier versions wrote, so an upgrade keeps the session it already had
 * rather than signing the account out.
 */
private val Context.accountDataStore by preferencesDataStore(
    name = ACCOUNT_STORE_NAME,
    produceMigrations = { context -> accountMigrations(context, ACCOUNT_STORE_NAME) },
)

/**
 * The one-shot move off the SharedPreferences file earlier versions wrote. Shared with the test
 * that proves it, which builds a store of its own: the delegate above is a single instance for the
 * whole process, so a test cannot get a fresh one.
 */
internal fun accountMigrations(context: Context, name: String) =
    listOf(SharedPreferencesMigration(context, name))

private const val ACCOUNT_STORE_NAME = "account"

private val SESSION = stringPreferencesKey("session")
private val PROFILE = stringPreferencesKey("profile")

private fun progressKeyOf(key: String) = longPreferencesKey("progress:$key")

private fun lastEpisodeKeyOf(userId: String, postId: Long) = stringPreferencesKey("lastep:$userId:$postId")

private fun searchHistoryKeyOf(userId: String) = stringPreferencesKey("search-history:$userId")

private fun seenNotificationsKeyOf(userId: String) = stringSetPreferencesKey("seen-notifications:$userId")

class SessionStore(
    context: Context,
    private val store: DataStore<Preferences> = context.accountDataStore,
) {
    private val keyStore by lazy { KeyStore.getInstance("AndroidKeyStore").apply { load(null) } }

    // Resolved once: two callers racing here would each generate a key, and the second would
    // replace the alias the first had already encrypted the session under.
    private val key by lazy { keyStore.getKey("movo-session", null) ?: KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").run {
        init(KeyGenParameterSpec.Builder("movo-session", KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        generateKey()
    } }

    private suspend fun read(): Preferences = store.data
        .catch { if (it is IOException) emit(emptyPreferences()) else throw it }
        .first()

    /**
     * The stored session, or null when there is none and when the stored one can no longer be
     * decrypted. A ciphertext the keystore key no longer opens is gone for good, so it is dropped
     * rather than thrown over: the user signs in again instead of meeting a crash on every start.
     */
    suspend fun secret(): String? = withContext(Dispatchers.IO) {
        read()[SESSION]?.let { encoded ->
            try {
                val bytes = Base64.decode(encoded, Base64.NO_WRAP)
                val cipher = Cipher.getInstance("AES/GCM/NoPadding")
                cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, bytes, 0, 12))
                cipher.doFinal(bytes, 12, bytes.size - 12).decodeToString()
            } catch (_: GeneralSecurityException) {
                forgetSession()
            } catch (_: IllegalArgumentException) {
                forgetSession()
            }
        }
    }

    private suspend fun forgetSession(): String? {
        store.edit { it.remove(SESSION); it.remove(PROFILE) }
        return null
    }

    /** The account as last seen, so the app can open on it while the provider is out of reach. */
    suspend fun profile(): UserProfile? = read()[PROFILE]?.let {
        runCatching { NativeBridge.json.decodeFromString<UserProfile>(it) }.getOrNull()
    }

    suspend fun saveProfile(user: UserProfile) {
        store.edit { it[PROFILE] = NativeBridge.json.encodeToString(user) }
    }

    // Off the main thread: resolving the keystore key and running the cipher can take tens of
    // milliseconds, which is a dropped frame on the sign-in button and on every restore.
    suspend fun saveSecret(value: String?) = withContext(Dispatchers.IO) {
        if (value == null) {
            forgetSession()
            return@withContext
        }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key)
        val bytes = cipher.iv + cipher.doFinal(value.encodeToByteArray())
        store.edit { it[SESSION] = Base64.encodeToString(bytes, Base64.NO_WRAP) }
    }

    suspend fun progress(key: String) = read()[progressKeyOf(key)] ?: 0L

    suspend fun saveProgress(key: String, position: Long) {
        store.edit { it[progressKeyOf(key)] = position }
    }

    suspend fun clearProgress(key: String) {
        store.edit { it.remove(progressKeyOf(key)) }
    }

    suspend fun clearProgressForMedia(userId: String, mediaId: Long) {
        val prefix = "progress:$userId:$mediaId:"
        store.edit { preferences ->
            preferences.asMap().keys
                .filter { it.name.startsWith(prefix) }
                .forEach { preferences.remove(it) }
        }
    }

    /**
     * What a title was last played as (season, episode, translatorId), for resume-on-reopen. A
     * movie has no season or episode and keeps only its voice-over.
     */
    suspend fun lastWatchedEpisode(userId: String, postId: Long): Triple<Long?, Long?, Long>? {
        val parts = read()[lastEpisodeKeyOf(userId, postId)]?.split('|') ?: return null
        if (parts.size != 3) return null
        val translatorId = parts[2].toLongOrNull() ?: return null
        return Triple(parts[0].toLongOrNull(), parts[1].toLongOrNull(), translatorId)
    }

    suspend fun saveLastEpisode(userId: String, postId: Long, season: Long?, episode: Long?, translatorId: Long) {
        store.edit { it[lastEpisodeKeyOf(userId, postId)] = "${season ?: ""}|${episode ?: ""}|$translatorId" }
    }

    suspend fun searchHistory(userId: String): List<String> =
        read()[searchHistoryKeyOf(userId)]?.let {
            runCatching { NativeBridge.json.decodeFromString<List<String>>(it) }.getOrDefault(emptyList())
        }.orEmpty()

    suspend fun saveSearch(userId: String, query: String): List<String> {
        val values = (listOf(query) + searchHistory(userId).filterNot { it.equals(query, true) }).take(20)
        store.edit { it[searchHistoryKeyOf(userId)] = NativeBridge.json.encodeToString(values) }
        return values
    }

    suspend fun clearSearchHistory(userId: String) {
        store.edit { it.remove(searchHistoryKeyOf(userId)) }
    }

    suspend fun seenNotifications(userId: String): Set<String> =
        read()[seenNotificationsKeyOf(userId)].orEmpty()

    suspend fun markNotificationsSeen(userId: String, keys: Set<String>) {
        store.edit { it[seenNotificationsKeyOf(userId)] = keys }
    }
}
