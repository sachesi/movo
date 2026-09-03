package org.movo.app.player

import org.movo.app.core.StreamEntry
import org.movo.app.core.SubtitleTrack
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.unit.IntSize

/**
 * Playback state belonging to one stream bundle. Remembered with the bundle as
 * its key, so moving to another episode starts every field here from scratch.
 */
@Stable
class PlayerContentState(
    initialStream: StreamEntry?,
    initialSubtitle: SubtitleTrack?,
    initialPositionMs: Long,
) {
    var stream by mutableStateOf(initialStream)
    var urlIndex by mutableIntStateOf(0)
    var subtitle by mutableStateOf(initialSubtitle)
    var playbackError by mutableStateOf<String?>(null)
    var completed by mutableStateOf(false)
    var historySynced by mutableStateOf(false)
    var playWhenReady by mutableStateOf(false)
    var positionMs by mutableLongStateOf(initialPositionMs)
    var pendingSeekTargetMs by mutableStateOf<Long?>(null)
    var hiddenSeekDirection by mutableStateOf<Key?>(null)
    var durationMs by mutableLongStateOf(0L)
    var bufferedMs by mutableLongStateOf(0L)
}

/**
 * A D-pad direction held down on the timeline: which key, when it went down, and how far the
 * seek has been carried so far, so a repeat can extend it instead of restarting it.
 */
@Stable
class TimelineHoldState {
    var direction by mutableStateOf<Key?>(null)
    var startedAtMs by mutableLongStateOf(0L)
    var appliedSeconds by mutableIntStateOf(0)
}

/**
 * Surface state that outlives a bundle change: the zoom, the playback speed and
 * whether the controls are on screen all survive moving to the next episode.
 */
@Stable
class PlayerUiState(initialSpeed: Float) {
    var playbackSpeed by mutableFloatStateOf(initialSpeed)
    var controlsVisible by mutableStateOf(true)
    var isPlaying by mutableStateOf(false)
    var isBuffering by mutableStateOf(false)
    var videoScale by mutableFloatStateOf(1f)
    var videoOffset by mutableStateOf(Offset.Zero)
    var videoSize by mutableStateOf(IntSize.Zero)
    var seekFeedback by mutableStateOf<Pair<Int, Long>?>(null)
    var menuOpen by mutableStateOf(false)
    var preparedContentKey by mutableStateOf<Triple<Long, Long?, Long?>?>(null)
}
