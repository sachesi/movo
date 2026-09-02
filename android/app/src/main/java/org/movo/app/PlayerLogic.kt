package org.movo.app

import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.unit.IntSize
import kotlin.math.abs

private const val MAX_VIDEO_ZOOM = 4f

internal fun selectStream(
    bundle: StreamBundle,
    mode: QualityMode,
    preferredQuality: String? = null,
): StreamEntry? {
    val streams = bundle.streams.filter { it.urls.isNotEmpty() }
    if (streams.isEmpty()) return null
    streams.firstOrNull { it.quality == preferredQuality }?.let { return it }
    val target = preferredQuality?.qualityScore()?.takeIf { it > 0 } ?: mode.targetHeight
    return when {
        target != null -> streams.firstOrNull { it.quality.equals("${target}p", ignoreCase = true) }
            ?: streams.minByOrNull { abs(it.qualityScore() - target) }
        mode == QualityMode.Max -> streams.maxByOrNull { it.qualityScore() }
        else -> streams.first()
    }
}

internal fun nextLowerStream(bundle: StreamBundle, current: StreamEntry?): StreamEntry? {
    val currentScore = current?.qualityScore() ?: return null
    return bundle.streams
        .filter { it.urls.isNotEmpty() && it.qualityScore() < currentScore }
        .maxByOrNull { it.qualityScore() }
}

internal fun updateVideoTransform(
    scale: Float,
    offset: Offset,
    centroid: Offset,
    pan: Offset,
    zoomChange: Float,
    size: IntSize,
): Pair<Float, Offset> {
    val newScale = (scale * zoomChange).coerceIn(1f, MAX_VIDEO_ZOOM)
    if (newScale == 1f || size.width <= 0 || size.height <= 0) return newScale to Offset.Zero

    val ratio = newScale / scale
    val center = Offset(size.width / 2f, size.height / 2f)
    val moved = offset * ratio + (centroid - center) * (1f - ratio) + pan
    val maxX = size.width * (newScale - 1f) / 2f
    val maxY = size.height * (newScale - 1f) / 2f
    return newScale to Offset(
        moved.x.coerceIn(-maxX, maxX),
        moved.y.coerceIn(-maxY, maxY),
    )
}

internal fun playbackStartPosition(sameContent: Boolean, currentPosition: Long, resumePosition: Long) =
    if (sameContent) currentPosition.coerceAtLeast(0L) else resumePosition

internal fun storyboardSpriteSizes(cues: List<StoryboardCue>) = cues
    .groupBy { it.imageUrl }
    .mapValues { (_, imageCues) ->
        imageCues.maxOf { it.x + it.width } to imageCues.maxOf { it.y + it.height }
    }

internal fun episodeMenuAnchorIndex(currentIndex: Int) = (currentIndex - 2).coerceAtLeast(0)

internal fun shouldAutoHideControls(visible: Boolean, playing: Boolean, menuOpen: Boolean) =
    visible && playing && !menuOpen

internal fun seekTarget(currentMs: Long, durationMs: Long, seconds: Int): Long {
    val target = currentMs + seconds * 1_000L
    return if (durationMs > 0L) target.coerceIn(0L, durationMs) else target.coerceAtLeast(0L)
}

internal fun nextSeekTarget(logicalTargetMs: Long?, reportedPositionMs: Long, durationMs: Long, seconds: Int) =
    seekTarget(logicalTargetMs ?: reportedPositionMs, durationMs, seconds)

internal fun seekTargetSettled(targetMs: Long, reportedPositionMs: Long) =
    abs(targetMs - reportedPositionMs) <= 1_000L

internal fun shouldShowControlsForPause(
    playing: Boolean,
    playWhenReady: Boolean,
    isTv: Boolean,
    showOnPause: Boolean,
) = isTv && showOnPause && !playing && !playWhenReady

internal fun timelineSeekSeconds(heldMs: Long): Int {
    val elapsed = heldMs.coerceAtLeast(0L)
    return when {
        elapsed < TV_TIMELINE_HOLD_DELAY_MS -> TV_TIMELINE_SEEK_SECONDS
        elapsed < TV_TIMELINE_FAST_HOLD_MS -> TV_TIMELINE_SEEK_SECONDS + (((elapsed - TV_TIMELINE_HOLD_DELAY_MS) / 250L) * 30L).toInt()
        else -> (TV_TIMELINE_SEEK_SECONDS + 180 + (((elapsed - TV_TIMELINE_FAST_HOLD_MS) / 250L) * 60L).toInt())
            .coerceAtMost(TV_TIMELINE_MAX_SEEK_SECONDS)
    }
}

internal fun formatSeekDelta(seconds: Int): String {
    val sign = if (seconds > 0) "+" else "-"
    val magnitude = abs(seconds)
    return if (magnitude % 60 == 0) "$sign${magnitude / 60}m" else "$sign${magnitude}s"
}

internal fun remainingPlaybackTimeMs(durationMs: Long, positionMs: Long, speed: Float): Long =
    (((durationMs - positionMs).coerceAtLeast(0L)).toDouble() / speed.coerceAtLeast(0.01f)).toLong()

internal fun tvControlsVisibleAfterKey(visible: Boolean, key: Key) =
    if (key == Key.DirectionUp) !visible else visible

private fun StreamEntry.qualityScore() = quality.qualityScore()

private fun String.qualityScore(): Int = when {
    contains("2160", ignoreCase = true) || contains("4K", ignoreCase = true) -> 2160
    contains("1440", ignoreCase = true) || contains("2K", ignoreCase = true) -> 1440
    else -> Regex("""\d{3,4}""").find(this)?.value?.toIntOrNull() ?: 0
}
