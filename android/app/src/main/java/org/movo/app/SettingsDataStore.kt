package org.movo.app

import android.content.Context
import androidx.datastore.preferences.core.booleanPreferencesKey
import androidx.datastore.preferences.core.floatPreferencesKey
import androidx.datastore.preferences.core.intPreferencesKey
import androidx.datastore.preferences.core.edit
import androidx.datastore.preferences.core.emptyPreferences
import androidx.datastore.preferences.core.stringPreferencesKey
import androidx.datastore.preferences.preferencesDataStore
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.catch
import kotlinx.coroutines.flow.map
import java.io.IOException

/**
 * UI-only configuration persisted with Jetpack DataStore Preferences (typed, coroutine-first,
 * no blocking `SharedPreferences` reads). Observed as state so Settings changes re-compose
 * the whole UI live — e.g. switching Phone ↔ TV mode takes effect instantly.
 */

enum class LayoutMode { Auto, Phone, Tv }
enum class ThemePref { System, Light, Dark }
enum class VideoFit { Contain, Cover, Fill }

/**
 * How the player resolves a stream's quality.
 * `Max` selects the highest resolution; fixed modes prefer their exact label and otherwise
 * select the closest available resolution.
 */
enum class QualityMode(val label: Int, val targetHeight: Int? = null) {
    Max(R.string.quality_max),
    P480(R.string.quality_480, 480),
    P720(R.string.quality_720, 720),
    P1080(R.string.quality_1080, 1080),
    P2160(R.string.quality_4k, 2160),
}

private const val SETTINGS_NAME = "movo_settings"

private val Context.settingsDataStore by preferencesDataStore(name = SETTINGS_NAME)

private object Keys {
    val LAYOUT_MODE = stringPreferencesKey("layout_mode")
    val THEME = stringPreferencesKey("theme")
    val USE_DYNAMIC_COLOR = booleanPreferencesKey("use_dynamic_color")
    val QUALITY_MODE = stringPreferencesKey("quality_mode")
    val AUTO_NEXT = booleanPreferencesKey("auto_next")
    val SEEK_SECONDS = intPreferencesKey("seek_seconds")
    val PLAYBACK_SPEED = floatPreferencesKey("playback_speed")
    val VIDEO_FIT = stringPreferencesKey("video_fit")
    val SHOW_BUFFER = booleanPreferencesKey("show_buffer")
    val SHOW_END_TIME = booleanPreferencesKey("show_end_time")
    val BUFFER_SECONDS = intPreferencesKey("buffer_seconds")
    val TV_CENTER_PAUSES = booleanPreferencesKey("tv_center_pauses")
    val TV_PAUSE_SHOWS_CONTROLS = booleanPreferencesKey("tv_pause_shows_controls")
    val ASK_QUALITY = booleanPreferencesKey("ask_quality")
    val SAVE_QUALITY = booleanPreferencesKey("save_quality")
    val LAST_QUALITY = stringPreferencesKey("last_quality")
    val SORT_VOICES = booleanPreferencesKey("sort_voices")
    val INITIAL_TAB = stringPreferencesKey("initial_tab")
}

data class AppSettings(
    val layoutMode: LayoutMode = LayoutMode.Auto,
    val theme: ThemePref = ThemePref.System,
    val useDynamicColor: Boolean = true,
    val qualityMode: QualityMode = QualityMode.P1080,
    val autoNext: Boolean = true,
    val seekSeconds: Int = 10,
    val playbackSpeed: Float = 1f,
    val videoFit: VideoFit = VideoFit.Contain,
    val showBuffer: Boolean = true,
    val showEndTime: Boolean = true,
    val bufferSeconds: Int = 0,
    val tvCenterPauses: Boolean = true,
    val tvPauseShowsControls: Boolean = true,
    val askQuality: Boolean = true,
    val saveQuality: Boolean = true,
    val lastQuality: String? = null,
    val sortVoices: Boolean = false,
    val initialTab: Tab = Tab.Catalog,
)

internal inline fun <reified T : Enum<T>> safeValueOf(name: String?, default: T): T {
    if (name == null) return default
    return runCatching { enumValueOf<T>(name) }.getOrDefault(default)
}

/** Flow of the current settings, defaulting sensibly for first-run users. */
val Context.settings: Flow<AppSettings>
    get() = settingsDataStore.data
        .catch { if (it is IOException) emit(emptyPreferences()) else throw it }
        .map { prefs ->
            AppSettings(
                layoutMode = safeValueOf(prefs[Keys.LAYOUT_MODE], LayoutMode.Auto),
                theme = safeValueOf(prefs[Keys.THEME], ThemePref.System),
                useDynamicColor = prefs[Keys.USE_DYNAMIC_COLOR] ?: true,
                qualityMode = safeValueOf(prefs[Keys.QUALITY_MODE], QualityMode.P1080),
                autoNext = prefs[Keys.AUTO_NEXT] ?: true,
                seekSeconds = prefs[Keys.SEEK_SECONDS] ?: 10,
                playbackSpeed = prefs[Keys.PLAYBACK_SPEED] ?: 1f,
                videoFit = safeValueOf(prefs[Keys.VIDEO_FIT], VideoFit.Contain),
                showBuffer = prefs[Keys.SHOW_BUFFER] ?: true,
                showEndTime = prefs[Keys.SHOW_END_TIME] ?: true,
                bufferSeconds = prefs[Keys.BUFFER_SECONDS] ?: 0,
                tvCenterPauses = prefs[Keys.TV_CENTER_PAUSES] ?: true,
                tvPauseShowsControls = prefs[Keys.TV_PAUSE_SHOWS_CONTROLS] ?: true,
                askQuality = prefs[Keys.ASK_QUALITY] ?: true,
                saveQuality = prefs[Keys.SAVE_QUALITY] ?: true,
                lastQuality = prefs[Keys.LAST_QUALITY],
                sortVoices = prefs[Keys.SORT_VOICES] ?: false,
                initialTab = safeValueOf(prefs[Keys.INITIAL_TAB], Tab.Catalog),
            )
        }

suspend fun Context.saveLayoutMode(mode: LayoutMode) =
    settingsDataStore.edit { it[Keys.LAYOUT_MODE] = mode.name }

suspend fun Context.saveTheme(theme: ThemePref) =
    settingsDataStore.edit { it[Keys.THEME] = theme.name }

suspend fun Context.saveUseDynamicColor(enabled: Boolean) =
    settingsDataStore.edit { it[Keys.USE_DYNAMIC_COLOR] = enabled }

suspend fun Context.saveQualityMode(mode: QualityMode) =
    settingsDataStore.edit { it[Keys.QUALITY_MODE] = mode.name }

suspend fun Context.saveAutoNext(enabled: Boolean) =
    settingsDataStore.edit { it[Keys.AUTO_NEXT] = enabled }

suspend fun Context.saveSeekSeconds(seconds: Int) =
    settingsDataStore.edit { it[Keys.SEEK_SECONDS] = seconds }

suspend fun Context.savePlaybackSpeed(speed: Float) =
    settingsDataStore.edit { it[Keys.PLAYBACK_SPEED] = speed }

suspend fun Context.saveVideoFit(mode: VideoFit) =
    settingsDataStore.edit { it[Keys.VIDEO_FIT] = mode.name }

suspend fun Context.saveShowBuffer(enabled: Boolean) = settingsDataStore.edit { it[Keys.SHOW_BUFFER] = enabled }
suspend fun Context.saveShowEndTime(enabled: Boolean) = settingsDataStore.edit { it[Keys.SHOW_END_TIME] = enabled }
suspend fun Context.saveBufferSeconds(seconds: Int) = settingsDataStore.edit { it[Keys.BUFFER_SECONDS] = seconds }
suspend fun Context.saveTvCenterPauses(enabled: Boolean) = settingsDataStore.edit { it[Keys.TV_CENTER_PAUSES] = enabled }
suspend fun Context.saveTvPauseShowsControls(enabled: Boolean) = settingsDataStore.edit { it[Keys.TV_PAUSE_SHOWS_CONTROLS] = enabled }
suspend fun Context.saveAskQuality(enabled: Boolean) = settingsDataStore.edit { it[Keys.ASK_QUALITY] = enabled }
suspend fun Context.saveSaveQuality(enabled: Boolean) = settingsDataStore.edit { it[Keys.SAVE_QUALITY] = enabled }
suspend fun Context.saveLastQuality(quality: String) = settingsDataStore.edit { it[Keys.LAST_QUALITY] = quality }
suspend fun Context.saveSortVoices(enabled: Boolean) = settingsDataStore.edit { it[Keys.SORT_VOICES] = enabled }
suspend fun Context.saveInitialTab(tab: Tab) = settingsDataStore.edit { it[Keys.INITIAL_TAB] = tab.name }
