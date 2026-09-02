package org.movo.app

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.Base64
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.put
import kotlinx.serialization.encodeToString
import kotlinx.serialization.decodeFromString
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.spec.GCMParameterSpec

object NativeBridge {
    init { System.loadLibrary("movo_android") }
    private external fun invoke(request: String): String
    val json = Json { ignoreUnknownKeys = true; encodeDefaults = true }

    suspend fun call(type: String, fields: JsonObject = buildJsonObject {}): String = withContext(Dispatchers.IO) {
        val request = buildJsonObject { put("type", type); fields.forEach { (key, value) -> put(key, value) } }
        val response = json.parseToJsonElement(invoke(request.toString())).jsonObject
        response["error"]?.jsonPrimitive?.content?.let { throw IllegalStateException(it) }
        response.getValue("data").toString()
    }

    /**
     * Runs [type] on the core and parses its payload, both off the main thread. Callers must use
     * this rather than decoding the [call] string themselves: catalog pages and media details are
     * large enough that parsing them on the UI thread drops frames.
     */
    suspend inline fun <reified T> decode(type: String, fields: JsonObject = buildJsonObject {}): T =
        withContext(Dispatchers.IO) { json.decodeFromString<T>(call(type, fields)) }
}

/**
 * Touches [NativeBridge] from a worker thread so the Rust library is dlopen-ed there. Referencing
 * the object at all triggers its initializer, so this has to live outside it to be of any use.
 */
suspend fun warmUpNativeBridge() = withContext(Dispatchers.IO) { NativeBridge.json }

class SessionStore(context: Context) {
    private val preferences by lazy { context.getSharedPreferences("account", Context.MODE_PRIVATE) }
    private val keyStore by lazy { KeyStore.getInstance("AndroidKeyStore").apply { load(null) } }
    private val key get() = keyStore.getKey("movo-session", null) ?: KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").run {
        init(KeyGenParameterSpec.Builder("movo-session", KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT).setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build())
        generateKey()
    }
    suspend fun secret(): String? = withContext(Dispatchers.IO) {
        preferences.getString("session", null)?.let { encoded ->
            val bytes = Base64.decode(encoded, Base64.NO_WRAP)
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(128, bytes, 0, 12))
            cipher.doFinal(bytes, 12, bytes.size - 12).decodeToString()
        }
    }

    suspend fun saveSecret(value: String?) = withContext(Dispatchers.IO) {
        if (value == null) {
            preferences.edit().remove("session").apply()
        } else {
            val cipher = Cipher.getInstance("AES/GCM/NoPadding")
            cipher.init(Cipher.ENCRYPT_MODE, key)
            val bytes = cipher.iv + cipher.doFinal(value.encodeToByteArray())
            preferences.edit().putString("session", Base64.encodeToString(bytes, Base64.NO_WRAP)).apply()
        }
    }

    suspend fun progress(key: String) = withContext(Dispatchers.IO) {
        preferences.getLong("progress:$key", 0L)
    }

    suspend fun saveProgress(key: String, position: Long) = withContext(Dispatchers.IO) {
        preferences.edit().putLong("progress:$key", position).apply()
    }

    suspend fun clearProgress(key: String) = withContext(Dispatchers.IO) {
        preferences.edit().remove("progress:$key").apply()
    }

    /** Last episode played for a show (season, episode, translatorId), for resume-on-reopen. */
    suspend fun lastWatchedEpisode(userId: String, postId: Long): Triple<Long, Long, Long>? = withContext(Dispatchers.IO) {
        val raw = preferences.getString("lastep:$userId:$postId", null) ?: return@withContext null
        val parts = raw.split('|')
        if (parts.size != 3) null
        else try { Triple(parts[0].toLong(), parts[1].toLong(), parts[2].toLong()) }
        catch (_: NumberFormatException) { null }
    }

    suspend fun saveLastEpisode(userId: String, postId: Long, season: Long, episode: Long, translatorId: Long) = withContext(Dispatchers.IO) {
        preferences.edit().putString("lastep:$userId:$postId", "$season|$episode|$translatorId").apply()
    }

    suspend fun searchHistory(userId: String): List<String> = withContext(Dispatchers.IO) {
        preferences.getString("search-history:$userId", null)?.let {
            runCatching { NativeBridge.json.decodeFromString<List<String>>(it) }.getOrDefault(emptyList())
        }.orEmpty()
    }

    suspend fun saveSearch(userId: String, query: String): List<String> = withContext(Dispatchers.IO) {
        val values = (listOf(query) + searchHistory(userId).filterNot { it.equals(query, true) }).take(20)
        preferences.edit().putString("search-history:$userId", NativeBridge.json.encodeToString(values)).apply()
        values
    }

    suspend fun clearSearchHistory(userId: String) = withContext(Dispatchers.IO) {
        preferences.edit().remove("search-history:$userId").apply()
    }

    suspend fun seenNotifications(userId: String): Set<String> = withContext(Dispatchers.IO) {
        preferences.getStringSet("seen-notifications:$userId", emptySet()).orEmpty().toSet()
    }

    suspend fun markNotificationsSeen(userId: String, keys: Set<String>) = withContext(Dispatchers.IO) {
        preferences.edit().putStringSet("seen-notifications:$userId", keys).apply()
    }
}
