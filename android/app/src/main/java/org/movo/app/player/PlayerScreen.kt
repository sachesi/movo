package org.movo.app.player

import androidx.window.layout.FoldingFeature
import org.movo.app.ui.LocalFold
import org.movo.app.ui.topHeight
import androidx.compose.animation.core.spring
import org.movo.app.ui.LocalReducedMotion
import org.movo.app.ui.motionSpec
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.windowInsetsPadding
import org.movo.app.ui.TV_OVERSCAN_HORIZONTAL
import org.movo.app.ui.TV_OVERSCAN_VERTICAL
import org.movo.app.ui.tvFocusScale
import org.movo.app.settings.settings
import org.movo.app.core.Season
import org.movo.app.core.StoryboardCue
import org.movo.app.core.StreamBundle
import org.movo.app.core.StreamEntry
import org.movo.app.core.SubtitleTrack
import org.movo.app.settings.AppSettings
import org.movo.app.settings.VideoFit
import org.movo.app.R
import android.os.SystemClock
import android.content.pm.ActivityInfo
import androidx.activity.compose.BackHandler
import androidx.activity.compose.LocalActivity
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.scaleIn
import androidx.compose.animation.scaleOut
import androidx.compose.animation.slideInVertically
import androidx.compose.animation.slideOutVertically
import androidx.compose.foundation.background
import androidx.compose.foundation.focusable
import androidx.compose.foundation.gestures.detectTapGestures
import androidx.compose.foundation.gestures.detectTransformGestures
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.IconButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Slider
import androidx.compose.material3.SliderDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.mutableLongStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.Stable
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.focus.FocusDirection
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.draw.clipToBounds
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.input.pointer.PointerEventPass
import androidx.compose.ui.input.pointer.pointerInput
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.input.key.KeyEvent
import androidx.compose.ui.input.key.KeyEventType
import androidx.compose.ui.input.key.key
import androidx.compose.ui.input.key.onKeyEvent
import androidx.compose.ui.input.key.onPreviewKeyEvent
import androidx.compose.ui.input.key.type
import androidx.compose.ui.layout.onSizeChanged
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.media3.common.AudioAttributes
import androidx.media3.common.C
import androidx.media3.common.MediaItem
import androidx.media3.common.MediaMetadata
import androidx.media3.common.MimeTypes
import androidx.media3.common.Player
import androidx.media3.common.PlaybackException
import androidx.media3.common.PlaybackParameters
import androidx.media3.datasource.DefaultHttpDataSource
import androidx.media3.exoplayer.DefaultRenderersFactory
import androidx.media3.exoplayer.DefaultLoadControl
import androidx.media3.exoplayer.ExoPlayer
import androidx.media3.exoplayer.source.DefaultMediaSourceFactory
import androidx.media3.session.MediaSession
import androidx.media3.ui.AspectRatioFrameLayout
import androidx.media3.ui.PlayerView
import coil3.compose.AsyncImage
import androidx.core.net.toUri
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat
import kotlinx.coroutines.delay
import kotlinx.coroutines.isActive
import java.text.DateFormat
import java.util.Date
import java.util.concurrent.atomic.AtomicLong

internal const val PROGRESS_SAVE_INTERVAL_MS = 5_000L
internal const val TV_TIMELINE_SEEK_SECONDS = 2 * 60
internal const val TV_TIMELINE_HOLD_DELAY_MS = 500L
internal const val TV_TIMELINE_FAST_HOLD_MS = 2_000L
internal const val TV_TIMELINE_MAX_SEEK_SECONDS = 20 * 60

/** Distinguishes concurrently live [MediaSession]s; Media3 refuses two sharing an id. */
private val sessionCounter = AtomicLong()

private const val PHONE_CONTROLS_TIMEOUT_MS = 3_000L
private const val SYNC_ERROR_VISIBLE_MS = 6_000L
private const val TV_CONTROLS_TIMEOUT_MS = 5_000L
private const val BUTTON_VIDEO_ZOOM = 1.5f
private val EPISODE_MENU_MAX_HEIGHT = 320.dp

/** What the player asks of the rest of the app. */
@Stable
interface PlayerActions {
    fun saveProgress(positionMs: Long)
    fun playbackStarted()
    fun close(completed: Boolean, positionMs: Long)
    fun previousEpisode()
    fun nextEpisode(completed: Boolean)
    fun playEpisode(season: Long, episode: Long)
    fun openRating(positionMs: Long)

    /** [persist] is set when the user picked the quality, rather than a fallback choosing it. */
    fun qualityChanged(quality: String, persist: Boolean)
}

/**
 * First-class playback screen.
 *
 * Best practices applied:
 *  - Media3 [ExoPlayer] shared with a single [MediaSession], so hardware media keys, the lock
 *    screen, Bluetooth remotes and TV media buttons all drive playback.
 *  - Edge-to-edge video surface with a comfortable overlay: play/pause, a precise seekbar,
 *    quality + subtitle chips, always in immersive landscape.
 *  - Controls auto-hide on touch devices; TV users show and hide them with D-pad Up/Down.
 *  - Resume position restored from the last saved point; progress saved periodically and on
 *    pause/stop. Completion marks the item watched (delegated to the view model).
 *  - Directional (DPAD) focus on TV: control buttons are [focusable].
 */
@Composable
fun PlayerScreen(
    bundle: StreamBundle,
    title: String,
    resumePositionMs: Long,
    preferredQuality: String?,
    syncError: String?,
    isTv: Boolean,
    hasPreviousEpisode: Boolean,
    hasNextEpisode: Boolean,
    seasons: List<Season>,
    settings: AppSettings,
    actions: PlayerActions,
) {
    val context = LocalContext.current
    val activity = LocalActivity.current
    val lifecycleOwner = LocalLifecycleOwner.current
    val playerFocusRequester = remember { FocusRequester() }
    val controlsFocusRequester = remember { FocusRequester() }
    val contentKey = Triple(bundle.id, bundle.season, bundle.episode)
    val episodeTitle = seasons
        .firstOrNull { it.id == bundle.season }
        ?.episodes
        ?.firstOrNull { it.id == bundle.episode }
        ?.title
    val displayTitle = episodeTitle?.takeUnless { it == title }?.let { "$title • $it" } ?: title

    val content = remember(bundle) {
        PlayerContentState(
            initialStream = selectStream(bundle, settings.qualityMode, preferredQuality),
            initialSubtitle = bundle.subtitles.firstOrNull { it.default },
            initialPositionMs = resumePositionMs,
        )
    }
    val ui = remember { PlayerUiState(initialSpeed = settings.playbackSpeed) }
    // Written on every remote key press, so it is deliberately never read from composition:
    // observing it here would recompose the whole player on each auto-repeat event.
    val lastInteractionMs = remember { mutableLongStateOf(0L) }
    // The player's listener outlives the composition that built it, so it reaches the actions
    // through this rather than capturing whichever instance was current when it was created.
    val latestActions by rememberUpdatedState(actions)
    val latestAutoNext by rememberUpdatedState(settings.autoNext && hasNextEpisode)
    val latestIsTv by rememberUpdatedState(isTv)
    val latestPauseShowsControls by rememberUpdatedState(settings.tvPauseShowsControls)

    // A failed history write is worth a glance, not a banner over the whole film: it fades on
    // its own, while a playback error stays until the player recovers or is closed.
    var shownSyncError by remember { mutableStateOf<String?>(null) }
    LaunchedEffect(syncError) {
        shownSyncError = syncError
        if (syncError != null) {
            delay(SYNC_ERROR_VISIBLE_MS)
            shownSyncError = null
        }
    }
    val noStreamsText = stringResource(R.string.no_streams)
    val playbackFailedText = stringResource(R.string.playback_failed)
    val mirrorUnavailableText = stringResource(R.string.mirror_unavailable)

    val player = remember(bundle, settings.bufferSeconds) {
        val http = DefaultHttpDataSource.Factory()
            .setUserAgent(bundle.userAgent)
            .setDefaultRequestProperties(mapOf("Referer" to bundle.referer))
        val renderers = DefaultRenderersFactory(context).setEnableDecoderFallback(true)
        val audio = AudioAttributes.Builder()
            .setUsage(C.USAGE_MEDIA)
            .setContentType(C.AUDIO_CONTENT_TYPE_MOVIE)
            .build()
        val builder = ExoPlayer.Builder(context, renderers)
            .setMediaSourceFactory(DefaultMediaSourceFactory(http))
            // Audio focus, so a call or another player pauses this one rather than talking over it,
            // and a pause when the headphones come out instead of playing on through the speaker.
            .setAudioAttributes(audio, true)
            .setHandleAudioBecomingNoisy(true)
        if (settings.bufferSeconds > 0) {
            val bufferMs = settings.bufferSeconds * 1_000
            builder.setLoadControl(DefaultLoadControl.Builder().setBufferDurationsMs(bufferMs, bufferMs, 2_500, 5_000).build())
        }
        builder.build()
            .apply {
                addListener(object : Player.Listener {
                    override fun onPlaybackStateChanged(state: Int) {
                        ui.isBuffering = state == Player.STATE_BUFFERING
                        if (state == Player.STATE_READY) content.playbackError = null
                        if (state == Player.STATE_ENDED) {
                            content.completed = true
                            ui.controlsVisible = true
                            if (latestAutoNext) latestActions.nextEpisode(true)
                        }
                    }

                    override fun onIsPlayingChanged(playing: Boolean) {
                        if (shouldShowControlsForPause(playing, content.playWhenReady, latestIsTv, latestPauseShowsControls)) {
                            ui.controlsVisible = true
                        }
                        if (playing && !content.historySynced) {
                            content.historySynced = true
                            latestActions.playbackStarted()
                        }
                    }

                    override fun onPlayWhenReadyChanged(ready: Boolean, reason: Int) {
                        content.playWhenReady = ready
                        if (shouldShowControlsForPause(false, ready, latestIsTv, latestPauseShowsControls)) {
                            ui.controlsVisible = true
                        }
                    }

                    override fun onPlayerError(error: PlaybackException) {
                        val next = content.urlIndex + 1
                        if (next < (content.stream?.urls?.size ?: 0)) {
                            content.urlIndex = next
                        } else {
                            val fallback = nextLowerStream(bundle, content.stream)
                            if (fallback != null) {
                                content.stream = fallback
                                content.urlIndex = 0
                                latestActions.qualityChanged(fallback.quality, false)
                            } else {
                                content.playbackError = error.message ?: playbackFailedText
                                ui.controlsVisible = true
                            }
                        }
                    }
                })
            }
    }

    // MediaSession: surfaces playback to lock screen, media switches and TV remotes.
    // Switching episodes replaces the player while this screen stays composed, so the
    // replacement session is built before the outgoing one is released. Media3 rejects a
    // second session holding an id already in its process-wide registry, so each one is
    // given an id of its own instead of the default empty string.
    val mediaSession = remember(player) {
        MediaSession.Builder(context, player)
            .setId("movo-${sessionCounter.getAndIncrement()}")
            .build()
    }
    DisposableEffect(mediaSession, player) {
        onDispose {
            mediaSession.release()
            player.release()
        }
    }

    fun buildMediaItem(): MediaItem {
        val uri = content.stream?.urls?.getOrNull(content.urlIndex) ?: ""
        val subs = content.subtitle?.let {
            listOf(
                MediaItem.SubtitleConfiguration.Builder(it.url.toUri())
                    .setMimeType(MimeTypes.TEXT_VTT)
                    .setLanguage(it.languageCode)
                    .setSelectionFlags(C.SELECTION_FLAG_DEFAULT)
                    .build(),
            )
        }.orEmpty()
        val metadata = MediaMetadata.Builder()
            .setTitle(displayTitle)
            .setMediaType(MediaMetadata.MEDIA_TYPE_MOVIE)
            .build()
        return MediaItem.Builder()
            .setUri(uri)
            .setSubtitleConfigurations(subs)
            .setMediaMetadata(metadata)
            .build()
    }

    fun prepare() {
        val uri = content.stream?.urls?.getOrNull(content.urlIndex)
        if (uri.isNullOrEmpty()) {
            content.playbackError = if (content.stream == null) noStreamsText else mirrorUnavailableText
            ui.controlsVisible = true
            return
        }
        val position = playbackStartPosition(
            sameContent = ui.preparedContentKey == contentKey,
            currentPosition = player.currentPosition,
            resumePosition = resumePositionMs,
        )
        ui.preparedContentKey = contentKey
        player.setMediaItem(buildMediaItem())
        player.prepare()
        if (position > 0L) player.seekTo(position)
        player.playWhenReady = true
        ui.isPlaying = true
    }

    LaunchedEffect(player, content.stream, content.subtitle, content.urlIndex) { prepare() }
    // Apply playback speed changes immediately (ExoPlayer rescales media clock pitch-aware).
    LaunchedEffect(player, ui.playbackSpeed) { player.playbackParameters = PlaybackParameters(ui.playbackSpeed) }
    // Poll playback state for a smooth seekbar.
    LaunchedEffect(player) {
        while (isActive) {
            val playing = player.isPlaying
            val reportedPosition = player.currentPosition.coerceAtLeast(0L)
            val pending = content.pendingSeekTargetMs
            if (pending == null || seekTargetSettled(pending, reportedPosition)) {
                content.positionMs = reportedPosition
                if (pending != null) content.pendingSeekTargetMs = null
            } else {
                content.positionMs = pending
            }
            val dur = player.duration
            if (dur > 0) content.durationMs = dur
            content.bufferedMs = player.bufferedPosition.coerceAtLeast(0L)
            ui.isPlaying = playing
            delay(250)
        }
    }
    // Persist periodically without queueing a preferences write for every seekbar frame.
    LaunchedEffect(player) {
        while (isActive) {
            delay(PROGRESS_SAVE_INTERVAL_MS)
            if (player.isPlaying) actions.saveProgress(player.currentPosition)
        }
    }
    LaunchedEffect(ui.controlsVisible, ui.isPlaying, isTv, ui.menuOpen) {
        if (!shouldAutoHideControls(ui.controlsVisible, ui.isPlaying, ui.menuOpen)) return@LaunchedEffect
        val timeout = if (isTv) TV_CONTROLS_TIMEOUT_MS else PHONE_CONTROLS_TIMEOUT_MS
        lastInteractionMs.longValue = SystemClock.uptimeMillis()
        // Poll the last interaction instead of restarting on a state key: key auto-repeat would
        // otherwise tear down and re-arm this effect dozens of times a second.
        var idle = SystemClock.uptimeMillis() - lastInteractionMs.longValue
        while (idle < timeout) {
            delay(timeout - idle)
            idle = SystemClock.uptimeMillis() - lastInteractionMs.longValue
        }
        ui.controlsVisible = false
    }
    // The control-bar requester is attached to the play button, which only exists while the
    // overlay is composed, so the request can land before the node is there.
    LaunchedEffect(ui.controlsVisible, isTv) {
        if (!isTv) return@LaunchedEffect
        runCatching {
            if (ui.controlsVisible) controlsFocusRequester.requestFocus()
            else playerFocusRequester.requestFocus()
        }
    }
    LaunchedEffect(ui.seekFeedback) {
        if (ui.seekFeedback != null) {
            delay(700)
            ui.seekFeedback = null
        }
    }
    // Persist progress + pause when the user leaves the screen or backgrounded.
    DisposableEffect(lifecycleOwner, player) {
        val observer = LifecycleEventObserver { _, event ->
            if (event == Lifecycle.Event.ON_STOP && !content.completed) {
                actions.saveProgress(player.currentPosition)
                player.pause()
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose { lifecycleOwner.lifecycle.removeObserver(observer) }
    }
    // Fullscreen immersive landscape for the lifetime of the player.
    // Keyed on identity-stable values only: `activity` and the insets controller never change
    // across recompositions, so this effect installs exactly once and does not flip the
    // orientation (and with it the whole activity) back and forth during playback.
    val insetsController = remember(activity) {
        activity?.let { WindowCompat.getInsetsController(it.window, it.window.decorView) }
    }
    DisposableEffect(activity, insetsController) {
        val types = WindowInsetsCompat.Type.systemBars()
        val previousOrientation = activity?.requestedOrientation
        activity?.requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_SENSOR_LANDSCAPE
        insetsController?.hide(types)
        insetsController?.setSystemBarsBehavior(WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE)
        onDispose {
            previousOrientation?.let { activity?.requestedOrientation = it }
            insetsController?.show(types)
        }
    }

    fun exitPlayer() {
        player.pause()
        actions.close(content.completed, player.currentPosition)
    }
    BackHandler {
        // On a television Back takes the overlay down first, as the players around it do; the
        // next press leaves. A paused player keeps its overlay, so there Back leaves at once.
        if (isTv && ui.controlsVisible && ui.isPlaying) ui.controlsVisible = false else exitPlayer()
    }

    fun togglePlayback() {
        if (content.completed) {
            player.seekTo(0)
            content.completed = false
            player.play()
        } else if (player.isPlaying) {
            player.pause()
        } else {
            player.play()
        }
    }

    fun seekTo(targetMs: Long) {
        val duration = player.duration.takeIf { it > 0L } ?: content.durationMs
        val target = seekTarget(targetMs, duration, 0)
        content.pendingSeekTargetMs = target
        player.seekTo(target)
        content.positionMs = target
    }

    fun commitPendingSeek() {
        content.pendingSeekTargetMs?.let { target ->
            player.seekTo(target)
            content.positionMs = target
        }
    }

    fun seekBy(seconds: Int, commit: Boolean = true) {
        val duration = player.duration.takeIf { it > 0L } ?: content.durationMs
        val target = nextSeekTarget(
            logicalTargetMs = content.pendingSeekTargetMs,
            reportedPositionMs = player.currentPosition,
            durationMs = duration,
            seconds = seconds,
        )
        content.pendingSeekTargetMs = target
        content.positionMs = target
        if (commit) player.seekTo(target)
        if (seconds != 0) {
            val previous = ui.seekFeedback?.first
            val total = if (previous != null && (previous > 0) == (seconds > 0)) previous + seconds else seconds
            ui.seekFeedback = total to System.nanoTime()
        }
    }

    // The video fills the screen, but nothing the user reads or aims at may: a television crops
    // the outer 5% and a landscape phone puts its camera cutout in the same corner as the back
    // button. Both overlay layers inset by this; the scrims behind them still run edge to edge.
    val overlayInsets = if (isTv) {
        Modifier.padding(horizontal = TV_OVERSCAN_HORIZONTAL, vertical = TV_OVERSCAN_VERTICAL)
    } else {
        Modifier.windowInsetsPadding(WindowInsets.displayCutout)
    }

    // Tabletop: the screen is bent across the middle, so a full-screen video is creased in half.
    // The picture takes the upper panel and the controls keep the lower one.
    val fold = LocalFold.current?.takeIf { it.orientation == FoldingFeature.Orientation.HORIZONTAL }
    val videoHeight = fold?.topHeight()

    val overlay = ui.controlsVisible
    Box(
        Modifier
            .fillMaxSize()
            .background(Color.Black)
            // Any touch counts as interaction, observed on the initial pass so nothing below
            // has to give it up: without this a seek drag or a tap on a control did not delay
            // the auto-hide, and the overlay vanished under the finger mid-drag.
            .pointerInput(Unit) {
                awaitPointerEventScope {
                    while (true) {
                        awaitPointerEvent(PointerEventPass.Initial)
                        lastInteractionMs.longValue = SystemClock.uptimeMillis()
                    }
                }
            }
            // A tap toggles the overlay; a double tap on either half seeks that way. On every
            // layout: a tablet showing the television layout is still touched.
            .pointerInput(settings.seekSeconds) {
                detectTapGestures(
                    onTap = { ui.controlsVisible = !ui.controlsVisible },
                    onDoubleTap = { offset ->
                        seekBy(if (offset.x < size.width / 2) -settings.seekSeconds else settings.seekSeconds)
                    },
                )
            }
            .then(
                if (isTv) {
                    Modifier
                        .focusRequester(playerFocusRequester)
                        .onPreviewKeyEvent { event ->
                            if (event.type == KeyEventType.KeyDown) lastInteractionMs.longValue = SystemClock.uptimeMillis()
                            false
                        }
                        .onKeyEvent { event ->
                            val direction = when (event.key) {
                                Key.DirectionLeft -> -1
                                Key.DirectionRight -> 1
                                else -> 0
                            }
                            if (direction != 0) {
                                if (event.type == KeyEventType.KeyUp && content.hiddenSeekDirection == event.key) {
                                    content.hiddenSeekDirection = null
                                    true
                                } else if (event.type == KeyEventType.KeyDown && !ui.controlsVisible) {
                                    content.hiddenSeekDirection = event.key
                                    seekBy(direction * settings.seekSeconds)
                                    true
                                } else {
                                    false
                                }
                            } else {
                                if (event.type != KeyEventType.KeyDown) return@onKeyEvent false
                                // Only act as a fallback while controls are hidden. When the overlay
                                // is up, real focusable controls own the D-pad so the user can move
                                // focus between buttons and the timeline with Up/Down/Left/Right.
                                if (ui.controlsVisible) return@onKeyEvent false
                                // Down is not listed: it reveals nothing while the overlay is
                                // hidden, and claiming it would swallow the key for no reason.
                                if (event.key == Key.DirectionUp || event.key == Key.DirectionCenter ||
                                    event.key == Key.Enter
                                ) {
                                    if (event.key == Key.DirectionUp) {
                                        ui.controlsVisible = tvControlsVisibleAfterKey(ui.controlsVisible, event.key)
                                    } else if (settings.tvCenterPauses) {
                                        togglePlayback()
                                        if (settings.tvPauseShowsControls && !player.isPlaying) ui.controlsVisible = true
                                    } else {
                                        ui.controlsVisible = true
                                    }
                                    true
                                } else {
                                    false
                                }
                            }
                        }
                        .focusable()
                } else {
                    Modifier
                }
            ),
    ) {
        AndroidView(
            factory = { ctx ->
                PlayerView(ctx).apply {
                    this.player = player
                    useController = false
                }
            },
            update = {
                it.player = player
                it.keepScreenOn = ui.isPlaying
                it.resizeMode = when (settings.videoFit) {
                    VideoFit.Contain -> AspectRatioFrameLayout.RESIZE_MODE_FIT
                    VideoFit.Cover -> AspectRatioFrameLayout.RESIZE_MODE_ZOOM
                    VideoFit.Fill -> AspectRatioFrameLayout.RESIZE_MODE_FILL
                }
            },
            modifier = Modifier
                .then(
                    if (videoHeight != null) {
                        Modifier.fillMaxWidth().height(videoHeight).align(Alignment.TopCenter)
                    } else {
                        Modifier.fillMaxSize()
                    },
                )
                .onSizeChanged { ui.videoSize = it }
                .pointerInput(ui.videoSize) {
                    detectTransformGestures(panZoomLock = true) { centroid, pan, zoom, _ ->
                        val transformed = updateVideoTransform(
                            ui.videoScale,
                            ui.videoOffset,
                            centroid,
                            pan,
                            zoom,
                            ui.videoSize,
                        )
                        ui.videoScale = transformed.first
                        ui.videoOffset = transformed.second
                    }
                }
                .graphicsLayer {
                    scaleX = ui.videoScale
                    scaleY = ui.videoScale
                    translationX = ui.videoOffset.x
                    translationY = ui.videoOffset.y
                },
        )

        AnimatedVisibility(
            visible = overlay,
            modifier = Modifier.align(Alignment.TopCenter),
            enter = fadeIn(),
            exit = fadeOut(),
        ) {
            Row(
                Modifier
                    .fillMaxWidth()
                    .background(
                        Brush.verticalGradient(
                            listOf(Color.Black.copy(alpha = 0.62f), Color.Transparent),
                        ),
                    )
                    .then(overlayInsets)
                    .padding(
                        start = 12.dp,
                        end = 12.dp,
                        top = 8.dp,
                        bottom = 36.dp,
                    ),
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                PlayerIconButton(
                    onClick = ::exitPlayer,
                    icon = Icons.AutoMirrored.Filled.ArrowBack,
                    contentDescription = stringResource(R.string.back),
                    isTv = isTv,
                )
                Column(Modifier.weight(1f)) {
                    Text(
                        text = displayTitle,
                        color = Color.White,
                        style = if (isTv) MaterialTheme.typography.titleMedium else MaterialTheme.typography.titleLarge,
                        maxLines = 1,
                        overflow = TextOverflow.Ellipsis,
                    )
                    // Quiet secondary line: episode-of-season context without the "Title • Ep" blob.
                    if (isTv && episodeTitle != null && episodeTitle != title) {
                        Text(
                            text = title,
                            color = Color.White.copy(alpha = 0.72f),
                            style = MaterialTheme.typography.labelMedium,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                        )
                    }
                }
            }
        }

        (content.playbackError ?: shownSyncError)?.let { error ->
            Text(
                text = error,
                color = MaterialTheme.colorScheme.onErrorContainer,
                modifier = Modifier
                    .align(Alignment.TopCenter)
                    .padding(horizontal = 16.dp, vertical = 84.dp)
                    .background(
                        MaterialTheme.colorScheme.errorContainer,
                        RoundedCornerShape(12.dp),
                    )
                    .padding(horizontal = 16.dp, vertical = 12.dp),
                style = MaterialTheme.typography.bodyMedium,
            )
        }

        AnimatedVisibility(
            visible = ui.isBuffering && content.playbackError == null,
            modifier = Modifier.align(Alignment.Center),
            enter = fadeIn(),
            exit = fadeOut(),
        ) {
            CircularProgressIndicator(color = Color.White)
        }

        ui.seekFeedback?.let { (seconds, _) ->
            Text(
                text = formatSeekDelta(seconds),
                color = Color.White,
                style = MaterialTheme.typography.headlineSmall,
                modifier = Modifier
                    .align(Alignment.Center)
                    .background(Color.Black.copy(alpha = 0.55f), RoundedCornerShape(24.dp))
                    .padding(horizontal = 20.dp, vertical = 10.dp),
            )
        }

        AnimatedVisibility(
            visible = overlay && ui.seekFeedback == null && !ui.isBuffering && content.playbackError == null,
            modifier = Modifier.align(Alignment.Center),
            enter = if (LocalReducedMotion.current) fadeIn() else fadeIn() + scaleIn(initialScale = 0.88f),
            exit = if (LocalReducedMotion.current) fadeOut() else fadeOut() + scaleOut(targetScale = 0.88f),
        ) {
            IconButton(
                onClick = ::togglePlayback,
                modifier = Modifier.size(if (isTv) 88.dp else 76.dp).tvFocusScale(isTv, 1.1f),
                colors = IconButtonDefaults.iconButtonColors(
                    containerColor = Color.White.copy(alpha = 0.94f),
                    contentColor = Color.Black,
                ),
            ) {
                Icon(
                    imageVector = when {
                        content.completed -> Icons.Default.Replay
                        ui.isPlaying -> Icons.Default.Pause
                        else -> Icons.Default.PlayArrow
                    },
                    contentDescription = stringResource(
                        when {
                            content.completed -> R.string.replay
                            ui.isPlaying -> R.string.pause
                            else -> R.string.play
                        },
                    ),
                    modifier = Modifier.size(if (isTv) 48.dp else 40.dp),
                )
            }
        }

        AnimatedVisibility(
            visible = overlay,
            modifier = Modifier.align(Alignment.BottomCenter),
            enter = if (LocalReducedMotion.current) fadeIn() else fadeIn() + slideInVertically { it / 3 },
            exit = if (LocalReducedMotion.current) fadeOut() else fadeOut() + slideOutVertically { it / 3 },
        ) {
            Column(
                Modifier
                    .fillMaxWidth()
                    .background(
                        Brush.verticalGradient(
                            listOf(Color.Transparent, Color.Black.copy(alpha = 0.92f)),
                        ),
                    )
                    .then(overlayInsets)
                    .padding(top = 36.dp, bottom = 8.dp),
            ) {
                SeekRow(
                    content.positionMs,
                    content.durationMs,
                    content.bufferedMs,
                    bundle.storyboard,
                    settings.showBuffer,
                    settings.showEndTime,
                    ui.playbackSpeed,
                    isTv,
                    { seconds, commit -> seekBy(seconds, commit) },
                    ::seekTo,
                    ::commitPendingSeek,
                )
                ControlBar(
                    bundle = bundle,
                    stream = content.stream,
                    subtitle = content.subtitle,
                    playbackSpeed = ui.playbackSpeed,
                    isPlaying = ui.isPlaying,
                    completed = content.completed,
                    onTogglePlay = ::togglePlayback,
                    onSelectStream = {
                        content.stream = it
                        content.urlIndex = 0
                        actions.qualityChanged(it.quality, true)
                    },
                    onSelectSubtitle = { content.subtitle = it },
                    onSelectSpeed = { ui.playbackSpeed = it },
                    zoomed = ui.videoScale > 1f,
                    onToggleZoom = {
                        ui.videoScale = if (ui.videoScale > 1f) 1f else BUTTON_VIDEO_ZOOM
                        ui.videoOffset = Offset.Zero
                    },
                    isTv = isTv,
                    playFocusRequester = controlsFocusRequester,
                    previousEpisode = {
                        player.pause()
                        actions.saveProgress(player.currentPosition)
                        actions.previousEpisode()
                    },
                    nextEpisode = {
                        player.pause()
                        actions.saveProgress(player.currentPosition)
                        actions.nextEpisode(false)
                    },
                    hasPreviousEpisode = hasPreviousEpisode,
                    hasNextEpisode = hasNextEpisode,
                    seasons = seasons,
                    currentSeason = bundle.season,
                    currentEpisode = bundle.episode,
                    playEpisode = { season, episode ->
                        player.pause()
                        actions.saveProgress(player.currentPosition)
                        actions.playEpisode(season, episode)
                    },
                    openRating = { actions.openRating(player.currentPosition) },
                    onMenuOpenChange = { ui.menuOpen = it },
                    onHideControls = { ui.controlsVisible = false },
                )
            }
        }
    }
}

@Composable
private fun SeekRow(
    positionMs: Long,
    durationMs: Long,
    bufferedMs: Long,
    storyboard: List<StoryboardCue>,
    showBuffer: Boolean,
    showEndTime: Boolean,
    playbackSpeed: Float,
    isTv: Boolean,
    seekBy: (Int, Boolean) -> Unit,
    seekTo: (Long) -> Unit,
    commitPendingSeek: () -> Unit,
) {
    val focusManager = LocalFocusManager.current
    val endTimeFormat = remember { DateFormat.getTimeInstance(DateFormat.SHORT) }
    val spriteSizes = remember(storyboard) { storyboardSpriteSizes(storyboard) }
    val duration = durationMs.coerceAtLeast(1L)
    var dragging by remember { mutableStateOf(false) }
    var sliderPosition by remember { mutableFloatStateOf(positionMs.toFloat()) }
    val hold = remember { TimelineHoldState() }
    LaunchedEffect(positionMs, duration) {
        if (!dragging) sliderPosition = positionMs.coerceIn(0L, duration).toFloat()
    }
    Column {
        val previewMs = sliderPosition.toLong()
        val cue = if (dragging) storyboard.firstOrNull { previewMs in it.startMs until it.endMs } else null
        val spriteSize = cue?.let { spriteSizes[it.imageUrl] }
        if (dragging && cue != null && spriteSize != null && cue.width > 0 && cue.height > 0) {
            val scale = 192f / cue.width
            Box(
                Modifier
                    .align(Alignment.CenterHorizontally)
                    .padding(bottom = 6.dp)
                    .size(192.dp, (cue.height * scale).dp)
                    .clipToBounds()
                    .background(Color.Black.copy(alpha = 0.4f), RoundedCornerShape(10.dp)),
            ) {
                AsyncImage(
                    model = cue.imageUrl,
                    contentDescription = stringResource(R.string.seek_preview),
                    contentScale = ContentScale.FillBounds,
                    modifier = Modifier
                        .size((spriteSize.first * scale).dp, (spriteSize.second * scale).dp)
                        .offset((-cue.x * scale).dp, (-cue.y * scale).dp),
                )
            }
        }
        Row(
            Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 4.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
        Text(
            formatTime(if (dragging) sliderPosition.toLong() else positionMs.coerceIn(0L, duration)),
            color = Color.White.copy(alpha = 0.92f),
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.widthIn(min = 44.dp),
        )
        Slider(
            value = sliderPosition.coerceIn(0f, duration.toFloat()),
            onValueChange = { dragging = true; sliderPosition = it },
            onValueChangeFinished = {
                seekTo(sliderPosition.toLong())
                dragging = false
            },
            valueRange = 0f..duration.toFloat(),
            enabled = durationMs > 0,
            colors = SliderDefaults.colors(
                thumbColor = Color.White,
                activeTrackColor = Color.White,
                inactiveTrackColor = Color.White.copy(alpha = 0.24f),
                disabledThumbColor = Color.White.copy(alpha = 0.5f),
                disabledActiveTrackColor = Color.White.copy(alpha = 0.25f),
                disabledInactiveTrackColor = Color.White.copy(alpha = 0.15f),
            ),
            modifier = Modifier
                .weight(1f)
                .onPreviewKeyEvent { event ->
                    if (!isTv) return@onPreviewKeyEvent false
                    when (event.key) {
                        Key.DirectionUp -> {
                            if (event.type == KeyEventType.KeyDown) {
                                focusManager.moveFocus(FocusDirection.Up)
                                true
                            } else false
                        }
                        Key.DirectionDown -> {
                            if (event.type == KeyEventType.KeyDown) {
                                focusManager.moveFocus(FocusDirection.Down)
                                true
                            } else false
                        }
                        Key.DirectionLeft -> {
                            handleTimelineKey(event, -1, hold, seekBy, commitPendingSeek)
                        }
                        Key.DirectionRight -> {
                            handleTimelineKey(event, 1, hold, seekBy, commitPendingSeek)
                        }
                        else -> false
                    }
                },
        )
        Text(
            formatTime(durationMs),
            color = Color.White.copy(alpha = 0.92f),
            style = MaterialTheme.typography.labelMedium,
            modifier = Modifier.widthIn(min = 44.dp),
        )
        if (showBuffer) Text("+${formatTime((bufferedMs - positionMs).coerceAtLeast(0L))}", color = Color.White.copy(alpha = 0.6f), style = MaterialTheme.typography.labelSmall)
        if (showEndTime && durationMs > positionMs) Text(
            endTimeFormat.format(
                Date(System.currentTimeMillis() + remainingPlaybackTimeMs(durationMs, positionMs, playbackSpeed)),
            ),
            color = Color.White.copy(alpha = 0.6f),
            style = MaterialTheme.typography.labelSmall,
        )
        }
    }
}

private val SPEEDS = listOf(
    0.25f, 0.5f, 0.75f, 1.0f, 1.1f, 1.25f, 1.5f, 1.75f, 2.0f, 2.25f, 2.5f, 2.75f, 3.0f,
)

@Composable
private fun PlayerIconButton(
    onClick: () -> Unit,
    icon: ImageVector,
    contentDescription: String,
    isTv: Boolean,
    modifier: Modifier = Modifier,
    selected: Boolean = false,
) {
    var focused by remember { mutableStateOf(false) }
    val scale by animateFloatAsState(
        targetValue = if (focused && isTv) 1.1f else 1f,
        animationSpec = motionSpec(spring()),
        label = "player control focus",
    )
    val containerColor = when {
        focused && isTv -> MaterialTheme.colorScheme.primary
        selected -> Color.White.copy(alpha = 0.2f)
        else -> Color.Transparent
    }
    IconButton(
        onClick = onClick,
        modifier = modifier
            .size(if (isTv) 56.dp else 48.dp)
            .onFocusChanged { focused = it.isFocused }
            .graphicsLayer {
                scaleX = scale
                scaleY = scale
            },
        colors = IconButtonDefaults.iconButtonColors(
            containerColor = containerColor,
            contentColor = if (focused && isTv) MaterialTheme.colorScheme.onPrimary else Color.White,
        ),
    ) {
        Icon(icon, contentDescription)
    }
}

@Composable
private fun ControlBar(
    bundle: StreamBundle,
    stream: StreamEntry?,
    subtitle: SubtitleTrack?,
    playbackSpeed: Float,
    isPlaying: Boolean,
    completed: Boolean,
    onTogglePlay: () -> Unit,
    onSelectStream: (StreamEntry) -> Unit,
    onSelectSubtitle: (SubtitleTrack?) -> Unit,
    onSelectSpeed: (Float) -> Unit,
    zoomed: Boolean,
    onToggleZoom: () -> Unit,
    isTv: Boolean,
    playFocusRequester: FocusRequester,
    previousEpisode: () -> Unit,
    nextEpisode: () -> Unit,
    hasPreviousEpisode: Boolean,
    hasNextEpisode: Boolean,
    seasons: List<Season>,
    currentSeason: Long?,
    currentEpisode: Long?,
    playEpisode: (Long, Long) -> Unit,
    openRating: () -> Unit,
    onMenuOpenChange: (Boolean) -> Unit,
    onHideControls: () -> Unit,
) {
    // Two-tier hierarchy: transport + episode controls on the left (primary), settings
    // (quality/subtitles/speed/zoom) tucked to the right at reduced prominence.
    Row(
        Modifier
            .fillMaxWidth()
            .onPreviewKeyEvent { event ->
                // Down from the control bar dismisses the overlay (matches hide-on-Down
                // convention); the root player surface picks the key back up when hidden.
                if (isTv && event.type == KeyEventType.KeyDown && event.key == Key.DirectionDown) {
                    onHideControls()
                    true
                } else {
                    false
                }
            }
            .padding(horizontal = 8.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.SpaceBetween,
    ) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(if (isTv) 10.dp else 4.dp),
        ) {
            PlayerIconButton(
                onClick = onTogglePlay,
                icon = when {
                    completed -> Icons.Default.Replay
                    isPlaying -> Icons.Default.Pause
                    else -> Icons.Default.PlayArrow
                },
                contentDescription = stringResource(
                    when {
                        completed -> R.string.replay
                        isPlaying -> R.string.pause
                        else -> R.string.play
                    },
                ),
                isTv = isTv,
                modifier = Modifier.focusRequester(playFocusRequester),
            )
            if (hasPreviousEpisode) {
                PlayerIconButton(previousEpisode, Icons.Default.SkipPrevious, stringResource(R.string.previous_episode), isTv)
            }
            if (hasNextEpisode) {
                PlayerIconButton(nextEpisode, Icons.Default.SkipNext, stringResource(R.string.next_episode), isTv)
            }
            if (seasons.isNotEmpty()) {
                EpisodeSelector(seasons, currentSeason, currentEpisode, playEpisode, isTv, onMenuOpenChange)
            }
            PlayerIconButton(openRating, Icons.Default.StarRate, stringResource(R.string.rate_title), isTv)
        }
        Row(
            horizontalArrangement = Arrangement.spacedBy(if (isTv) 6.dp else 2.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            PlayerIconButton(
                onClick = onToggleZoom,
                icon = if (zoomed) Icons.Default.FitScreen else Icons.Default.ZoomIn,
                contentDescription = stringResource(if (zoomed) R.string.fit_video else R.string.zoom_video),
                isTv = isTv,
                selected = zoomed,
            )
            if (bundle.streams.size > 1) {
                PlayerMenu(
                    items = bundle.streams,
                    selected = stream,
                    label = { it.label() },
                    onSelect = onSelectStream,
                    isTv = isTv,
                    onMenuOpenChange = onMenuOpenChange,
                    openKey = bundle.streams,
                ) { open ->
                    PlayerIconButton(
                        onClick = open,
                        icon = Icons.Default.HighQuality,
                        contentDescription = stringResource(R.string.quality),
                        isTv = isTv,
                    )
                }
            }
            if (bundle.subtitles.isNotEmpty()) {
                PlayerMenu(
                    items = bundle.subtitles,
                    selected = subtitle,
                    label = { it.title.ifEmpty { it.code } },
                    onSelect = onSelectSubtitle,
                    isTv = isTv,
                    onMenuOpenChange = onMenuOpenChange,
                    reset = stringResource(R.string.subtitles_off) to { onSelectSubtitle(null) },
                ) { open ->
                    PlayerIconButton(
                        onClick = open,
                        icon = Icons.Default.Subtitles,
                        contentDescription = stringResource(R.string.subtitles),
                        isTv = isTv,
                        selected = subtitle != null,
                    )
                }
            }
            PlayerMenu(
                items = SPEEDS,
                selected = playbackSpeed,
                label = { formatSpeed(it) },
                onSelect = onSelectSpeed,
                isTv = isTv,
                onMenuOpenChange = onMenuOpenChange,
            ) { open ->
                if (playbackSpeed != 1f) {
                    // Text pill instead of an icon: shows the active speed without opening anything.
                    TextButton(
                        onClick = open,
                        modifier = Modifier.tvFocusScale(isTv, 1.06f),
                        colors = ButtonDefaults.textButtonColors(contentColor = Color.White),
                    ) { Text(formatSpeed(playbackSpeed), style = MaterialTheme.typography.labelLarge) }
                } else {
                    PlayerIconButton(
                        onClick = open,
                        icon = Icons.Default.Speed,
                        contentDescription = stringResource(R.string.speed),
                        isTv = isTv,
                        selected = false,
                    )
                }
            }
        }
    }
}

@Composable
private fun EpisodeSelector(
    seasons: List<Season>,
    currentSeason: Long?,
    currentEpisode: Long?,
    playEpisode: (Long, Long) -> Unit,
    isTv: Boolean,
    onMenuOpenChange: (Boolean) -> Unit,
) {
    var expanded by remember(seasons, currentSeason, currentEpisode) { mutableStateOf(false) }
    DisposableEffect(expanded) {
        if (expanded) onMenuOpenChange(true)
        onDispose { if (expanded) onMenuOpenChange(false) }
    }
    val episodes = remember(seasons) {
        seasons.flatMap { season -> season.episodes.map { episode -> season to episode } }
    }
    val currentIndex = remember(episodes, currentSeason, currentEpisode) {
        episodes.indexOfFirst { (season, episode) ->
            season.id == currentSeason && episode.id == currentEpisode
        }
    }
    val listState = rememberLazyListState()
    LaunchedEffect(expanded, currentIndex) {
        if (expanded && currentIndex >= 0) {
            listState.scrollToItem(episodeMenuAnchorIndex(currentIndex))
        }
    }
    Box {
        PlayerIconButton({ expanded = true }, Icons.Default.VideoLibrary, stringResource(R.string.episode), isTv)
        DropdownMenu(expanded, { expanded = false }) {
            // A show with many seasons has hundreds of episodes; composing them all when the menu
            // opens stalls the frame, so the rows are virtualized. The bounded height is required
            // because the menu already places its content inside a vertical scroll.
            LazyColumn(state = listState, modifier = Modifier.heightIn(max = EPISODE_MENU_MAX_HEIGHT)) {
                items(episodes, key = { (season, episode) -> "${season.id}:${episode.id}" }) { (season, episode) ->
                    DropdownMenuItem(
                        onClick = { expanded = false; playEpisode(season.id, episode.id) },
                        text = { Text("${season.title} • ${episode.title}", maxLines = 1, overflow = TextOverflow.Ellipsis) },
                        modifier = Modifier.tvFocusScale(isTv, 1.03f),
                        trailingIcon = if (season.id == currentSeason && episode.id == currentEpisode) {
                            { Icon(Icons.Default.Check, null) }
                        } else null,
                    )
                }
            }
        }
    }
}

/**
 * One of the player's dropdown menus: a control that opens a list with the current entry ticked.
 *
 * [trigger] draws the control and is handed the action that opens the menu. [reset] adds an entry
 * above the list, for a menu whose selection can be cleared. [openKey] closes the menu when the
 * thing it lists is replaced.
 */
@Composable
private fun <T> PlayerMenu(
    items: List<T>,
    selected: T?,
    label: @Composable (T) -> String,
    onSelect: (T) -> Unit,
    isTv: Boolean,
    onMenuOpenChange: (Boolean) -> Unit,
    openKey: Any? = Unit,
    reset: Pair<String, () -> Unit>? = null,
    trigger: @Composable (open: () -> Unit) -> Unit,
) {
    var expanded by remember(openKey) { mutableStateOf(false) }
    DisposableEffect(expanded) {
        if (expanded) onMenuOpenChange(true)
        onDispose { if (expanded) onMenuOpenChange(false) }
    }
    Box {
        trigger { expanded = true }
        DropdownMenu(
            expanded = expanded,
            onDismissRequest = { expanded = false },
        ) {
            if (reset != null) {
                val (resetLabel, onReset) = reset
                DropdownMenuItem(
                    onClick = { expanded = false; onReset() },
                    modifier = Modifier.tvFocusScale(isTv, 1.03f),
                    text = { Text(resetLabel) },
                    trailingIcon = tickWhen(selected == null),
                )
            }
            items.forEach { item ->
                DropdownMenuItem(
                    onClick = { expanded = false; onSelect(item) },
                    modifier = Modifier.tvFocusScale(isTv, 1.03f),
                    text = { Text(label(item), style = MaterialTheme.typography.bodyLarge) },
                    trailingIcon = tickWhen(item == selected),
                )
            }
        }
    }
}

/** Trailing tick for the entry a menu currently has selected. */
private fun tickWhen(selected: Boolean): (@Composable () -> Unit)? =
    if (selected) {
        { Icon(Icons.Default.Check, contentDescription = null) }
    } else {
        null
    }

private fun formatSpeed(speed: Float) = if (speed == 1f) "1×" else "${speed}×"

/** The quality, marked when the stream needs a premium account. */
@Composable
internal fun StreamEntry.label(): String =
    if (premium) "$quality · ${stringResource(R.string.tag_premium)}" else quality

private fun handleTimelineKey(
    event: KeyEvent,
    direction: Int,
    hold: TimelineHoldState,
    seekBy: (Int, Boolean) -> Unit,
    commitPendingSeek: () -> Unit,
): Boolean {
    val key = event.key
    val native = event.nativeKeyEvent
    return when (event.type) {
        KeyEventType.KeyDown -> {
            if (hold.direction != key) {
                hold.direction = key
                hold.startedAtMs = native.eventTime
                hold.appliedSeconds = TV_TIMELINE_SEEK_SECONDS
                seekBy(direction * TV_TIMELINE_SEEK_SECONDS, false)
            } else {
                val total = timelineSeekSeconds(native.eventTime - hold.startedAtMs)
                val delta = total - hold.appliedSeconds
                if (delta != 0) {
                    seekBy(direction * delta, false)
                    hold.appliedSeconds = total
                }
            }
            true
        }
        KeyEventType.KeyUp -> {
            if (hold.direction == key) {
                commitPendingSeek()
                hold.direction = null
                hold.startedAtMs = 0L
                hold.appliedSeconds = 0
                true
            } else {
                false
            }
        }
        else -> false
    }
}

private fun formatTime(ms: Long): String {
    val total = (if (ms < 0) 0 else ms) / 1000
    val h = total / 3600
    val m = (total % 3600) / 60
    val s = total % 60
    return if (h > 0) "$h:${m.toString().padStart(2, '0')}:${s.toString().padStart(2, '0')}"
    else "${m}:${s.toString().padStart(2, '0')}"
}
