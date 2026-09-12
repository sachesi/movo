package org.movo.app

import org.movo.app.core.MediaItem
import org.movo.app.core.StoryboardCue
import org.movo.app.core.StreamBundle
import org.movo.app.core.StreamEntry
import org.movo.app.core.Translator
import org.movo.app.settings.safeValueOf
import org.movo.app.catalog.shouldLoadMore
import org.movo.app.details.preferredTranslator
import org.movo.app.details.wantsEpisodes
import org.movo.app.search.searchFilterPath
import org.movo.app.settings.AppSettings
import org.movo.app.settings.QualityMode
import org.movo.app.settings.ThemePref
import org.movo.app.settings.save
import org.movo.app.ui.TvFocusMemory
import org.movo.app.player.PROGRESS_SAVE_INTERVAL_MS
import org.movo.app.player.TV_TIMELINE_MAX_SEEK_SECONDS
import org.movo.app.player.TV_TIMELINE_SEEK_SECONDS
import org.movo.app.player.episodeMenuAnchorIndex
import org.movo.app.player.file
import org.movo.app.player.formatSeekDelta
import org.movo.app.player.isHostFailure
import org.movo.app.player.nextLowerStream
import org.movo.app.player.nextSource
import org.movo.app.player.nextSeekTarget
import org.movo.app.player.pipAspectRatio
import org.movo.app.player.playbackStartPosition
import org.movo.app.player.remainingPlaybackTimeMs
import org.movo.app.player.seekTarget
import org.movo.app.player.selectStream
import org.movo.app.player.shouldAutoHideControls
import org.movo.app.player.shouldShowControlsForPause
import org.movo.app.player.storyboardSpriteSizes
import org.movo.app.player.timelineSeekSeconds
import org.movo.app.player.tvControlsVisibleAfterKey
import org.movo.app.player.updateVideoTransform
import org.movo.app.player.wantsPictureInPicture
import androidx.compose.material3.windowsizeclass.WindowWidthSizeClass
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.input.key.Key
import androidx.compose.ui.unit.IntSize
import androidx.media3.common.PlaybackException
import org.junit.Assert.assertEquals
import androidx.compose.runtime.saveable.SaverScope
import org.junit.Test
import kotlinx.serialization.json.Json

class ModelsTest {
    @Test fun rustPayloadDecodesIntoAndroidModel() {
        val item = Json { ignoreUnknownKeys = true }.decodeFromString<MediaItem>("""{"id":7,"title":"Test","orig_title":null,"url":"/7-test","poster_url":null,"year":2026,"category":"Film","rating":8.5,"info":null}""")
        assertEquals(7, item.id)
        assertEquals("Test", item.title)

        val stream = Json.decodeFromString<StreamBundle>("""{"id":7,"translator_id":1,"streams":[],"subtitles":[],"storyboard":[{"start_ms":1500,"end_ms":3000,"image_url":"https://hdrzk.org/sprite.jpg","x":150,"y":75,"width":150,"height":75}]}""")
        assertEquals(1_500L, stream.storyboard.single().startMs)
        assertEquals(150, stream.storyboard.single().x)
    }

    @Test fun configuredQualitySelectsMatchingStream() {
        val streams = listOf(
            StreamEntry("4K (2160p)", true, listOf("4k")),
            StreamEntry("1080p Ultra", true, listOf("1080")),
            StreamEntry("1080p", false, listOf("1080-standard")),
            StreamEntry("720p", false, listOf("720")),
            StreamEntry("480p", false, listOf("480")),
        )
        val bundle = StreamBundle(1, 1, streams = streams, subtitles = emptyList())

        assertEquals("4K (2160p)", selectStream(bundle, QualityMode.Max)?.quality)
        assertEquals("4K (2160p)", selectStream(bundle, QualityMode.P2160)?.quality)
        assertEquals("1080p", selectStream(bundle, QualityMode.P1080)?.quality)
        assertEquals("720p", selectStream(bundle, QualityMode.P720)?.quality)
        assertEquals("480p", selectStream(bundle, QualityMode.P480)?.quality)
        assertEquals("720p", selectStream(bundle, QualityMode.Max, "720p")?.quality)
        assertEquals(
            "480p",
            selectStream(
                bundle.copy(streams = streams.filter { it.quality == "480p" || it.quality == "1080p" }),
                QualityMode.Max,
                "720p",
            )?.quality,
        )
        assertEquals(
            "4K (2160p)",
            selectStream(
                bundle.copy(streams = listOf(StreamEntry("8K", true, emptyList())) + streams),
                QualityMode.Max,
            )?.quality,
        )
        assertEquals(QualityMode.P1080, AppSettings().qualityMode)
        assertEquals(QualityMode.P1080, safeValueOf("Auto", QualityMode.P1080))
        assertEquals(QualityMode.Max, safeValueOf("Max", QualityMode.P1080))
    }

    @Test fun progressPersistenceIsThrottled() {
        assertEquals(5_000L, PROGRESS_SAVE_INTERVAL_MS)
        assertEquals(42_000L, playbackStartPosition(true, 42_000L, 7_000L))
        assertEquals(0L, playbackStartPosition(true, 0L, 7_000L))
        assertEquals(7_000L, playbackStartPosition(false, 42_000L, 7_000L))
        assertEquals(0, episodeMenuAnchorIndex(0))
        assertEquals(5, episodeMenuAnchorIndex(7))
        assertEquals(true, shouldAutoHideControls(true, true, false))
        assertEquals(false, shouldAutoHideControls(true, true, true))
    }

    @Test fun storyboardSpriteDimensionsAreCachedPerImage() {
        val sizes = storyboardSpriteSizes(listOf(
            StoryboardCue(0, 1_000, "sprite-a", 0, 0, 160, 90),
            StoryboardCue(1_000, 2_000, "sprite-a", 160, 90, 160, 90),
            StoryboardCue(2_000, 3_000, "sprite-b", 0, 0, 120, 68),
        ))

        assertEquals(320 to 180, sizes["sprite-a"])
        assertEquals(120 to 68, sizes["sprite-b"])
    }

    @Test fun tvTimelineSeeksTwoMinutesAndAccumulatesFromLogicalTarget() {
        val duration = 30 * 60_000L
        var target = 5 * 60_000L
        repeat(3) { target = nextSeekTarget(target, 0L, duration, TV_TIMELINE_SEEK_SECONDS) }

        assertEquals(11 * 60_000L, target)
        assertEquals(0L, seekTarget(60_000L, duration, -TV_TIMELINE_SEEK_SECONDS))
        assertEquals(duration, seekTarget(29 * 60_000L, duration, TV_TIMELINE_SEEK_SECONDS))
        assertEquals(70_000L, seekTarget(60_000L, 0L, 10))
        assertEquals("-2m", formatSeekDelta(-TV_TIMELINE_SEEK_SECONDS))
        assertEquals("+2m", formatSeekDelta(TV_TIMELINE_SEEK_SECONDS))
    }

    @Test fun hiddenSeekRepeatsAccumulateFromLogicalTarget() {
        val duration = 30 * 60_000L
        var target = 5 * 60_000L
        repeat(10) { target = nextSeekTarget(target, 0L, duration, 10) }

        assertEquals(5 * 60_000L + 100_000L, target)
    }

    @Test fun tvTimelineHoldAccelerationIsMonotonicAndCapped() {
        val values = listOf(0L, 500L, 750L, 2_000L, 2_250L, 30_000L).map(::timelineSeekSeconds)

        assertEquals(listOf(120, 120, 150, 300, 360, TV_TIMELINE_MAX_SEEK_SECONDS), values)
        assertEquals(0L, nextSeekTarget(0L, 0L, 60_000L, -TV_TIMELINE_MAX_SEEK_SECONDS))
        assertEquals(60_000L, nextSeekTarget(0L, 0L, 60_000L, TV_TIMELINE_MAX_SEEK_SECONDS))
    }

    @Test fun bufferingDoesNotCountAsTvPause() {
        assertEquals(false, shouldShowControlsForPause(false, true, true, true))
        assertEquals(true, shouldShowControlsForPause(false, false, true, true))
        assertEquals(false, shouldShowControlsForPause(false, false, false, true))
    }

    @Test fun endTimeAccountsForPlaybackSpeed() {
        val duration = 120 * 60_000L
        val position = 20 * 60_000L
        assertEquals(50 * 60_000L, remainingPlaybackTimeMs(duration, position, 2f))
        assertEquals(200 * 60_000L, remainingPlaybackTimeMs(duration, position, .5f))
    }

    @Test fun playbackFallsBackOneQualityAtATime() {
        val streams = listOf(
            StreamEntry("1080p", false, listOf("1080")),
            StreamEntry("720p", false, listOf("720")),
            StreamEntry("480p", false, listOf("480")),
        )
        val bundle = StreamBundle(1, 1, streams = streams, subtitles = emptyList())

        assertEquals("720p", nextLowerStream(bundle, streams[0])?.quality)
        assertEquals("480p", nextLowerStream(bundle, streams[1])?.quality)
        assertEquals(null, nextLowerStream(bundle, streams[2]))
    }

    @Test fun onlyASeriesInAnotherVoiceOverFetchesEpisodes() {
        assertEquals(true, wantsEpisodes(series = true, listedFor = 1, translatorId = 2))
        assertEquals(false, wantsEpisodes(series = true, listedFor = 2, translatorId = 2))
        assertEquals(false, wantsEpisodes(series = false, listedFor = 1, translatorId = 2))
    }

    /** Each file on two hosts, the way the core lists them: the host that answered first. */
    private val mirroredSources = listOf(
        "https://a.example/v/720.m3u8",
        "https://a.example/v/720.mp4",
        "https://b.example/v/720.m3u8",
        "https://b.example/v/720.mp4",
    )

    @Test fun aHostThatFailsHandsOnToTheNextSource() {
        assertEquals(1, nextSource(mirroredSources, 0, emptySet()))
        assertEquals(null, nextSource(mirroredSources, 3, emptySet()))
    }

    @Test fun aFileThePlayerGaveUpOnIsNotFetchedFromAnotherHost() {
        val manifest = mirroredSources[0].file()
        val video = mirroredSources[1].file()

        assertEquals(1, nextSource(mirroredSources, 0, setOf(manifest)))
        assertEquals(3, nextSource(mirroredSources, 1, setOf(manifest)))
        assertEquals(null, nextSource(mirroredSources, 1, setOf(manifest, video)))
    }

    @Test fun onlyAFailureToReachOrReadTheSourceIsTheHostsDoing() {
        assertEquals(true, isHostFailure(PlaybackException.ERROR_CODE_IO_NETWORK_CONNECTION_FAILED))
        assertEquals(true, isHostFailure(PlaybackException.ERROR_CODE_IO_BAD_HTTP_STATUS))
        assertEquals(false, isHostFailure(PlaybackException.ERROR_CODE_DECODING_FAILED))
        assertEquals(false, isHostFailure(PlaybackException.ERROR_CODE_PARSING_CONTAINER_MALFORMED))
        assertEquals("v/720.mp4", "https://b.example/v/720.mp4".file())
    }

    @Test fun videoZoomKeepsContentBoundedAndResets() {
        val size = IntSize(1_000, 500)
        val zoomed = updateVideoTransform(1f, Offset.Zero, Offset(500f, 250f), Offset.Zero, 2f, size)
        assertEquals(2f, zoomed.first)
        assertEquals(Offset.Zero, zoomed.second)

        val panned = updateVideoTransform(2f, Offset.Zero, Offset.Zero, Offset(2_000f, 2_000f), 1f, size)
        assertEquals(Offset(500f, 250f), panned.second)
        assertEquals(1f to Offset.Zero, updateVideoTransform(2f, panned.second, Offset.Zero, Offset.Zero, .1f, size))
    }

    @Test fun tvDpadShowsPlayerControlsOnlyFromHiddenState() {
        assertEquals(false, tvControlsVisibleAfterKey(true, Key.DirectionUp))
        assertEquals(true, tvControlsVisibleAfterKey(false, Key.DirectionUp))
        assertEquals(false, tvControlsVisibleAfterKey(false, Key.DirectionDown))
    }

    @Test fun adaptiveNavigationAndThemeFollowLiveInputs() {
        assertEquals(false, usesNavigationRail(false, WindowWidthSizeClass.Compact))
        assertEquals(true, usesNavigationRail(false, WindowWidthSizeClass.Medium))
        assertEquals(true, usesNavigationRail(true, WindowWidthSizeClass.Compact))
        assertEquals(true, resolveDarkTheme(ThemePref.System, true))
        assertEquals(false, resolveDarkTheme(ThemePref.System, false))
        assertEquals(false, resolveDarkTheme(ThemePref.Light, true))
        assertEquals(true, resolveDarkTheme(ThemePref.Dark, false))
    }

    @Test fun pictureInPictureFollowsLivePlaybackAndKeepsTheVideoShape() {
        assertEquals(true, wantsPictureInPicture(true, false, false))
        assertEquals(false, wantsPictureInPicture(false, false, false))
        assertEquals(false, wantsPictureInPicture(true, true, false))
        assertEquals(false, wantsPictureInPicture(true, false, true))
        assertEquals(1920 to 1080, pipAspectRatio(1920, 1080))
        assertEquals(16 to 9, pipAspectRatio(0, 0))
        assertEquals(239 to 100, pipAspectRatio(4000, 1000))
        assertEquals(100 to 239, pipAspectRatio(1000, 4000))
    }

    @Test fun mediaGridLoadsOnceNearTheEnd() {
        assertEquals(false, shouldLoadMore(17, 30, false, -1))
        assertEquals(true, shouldLoadMore(18, 30, false, -1))
        assertEquals(false, shouldLoadMore(29, 30, true, -1))
        assertEquals(false, shouldLoadMore(29, 30, false, 30))
    }

    @Test fun resumePrefersTheLastPlayedTranslator() {
        val translators = listOf(
            Translator(1, "Default", false),
            Translator(2, "Preferred", false),
        )
        assertEquals(2L, preferredTranslator(translators, 2)?.id)
        assertEquals(1L, preferredTranslator(translators, 99)?.id)
    }

    @Test fun advancedSearchBuildsProviderPaths() {
        assertEquals("/films/drama", searchFilterPath("/films/drama/best", "-1"))
        assertEquals("/films/drama/", searchFilterPath("/films/drama/", "0"))
        assertEquals("/films/drama/year/2026/", searchFilterPath("/films/drama/year/", "2026/"))
    }

    @Test fun tvFocusMemoryRestoresPerDestination() {
        val memory = TvFocusMemory()

        memory.destination = "Catalog"
        memory.fallback = "/fallback"
        assertEquals("/fallback", memory.restoreKey())
        memory.record("/films/a")
        assertEquals("/films/a", memory.restoreKey())

        // A second destination keeps its own entry and does not see the first one's.
        memory.destination = "Favorites"
        memory.fallback = null
        assertEquals(null, memory.restoreKey())
        memory.record("/films/b")

        memory.destination = "Catalog"
        assertEquals("/films/a", memory.restoreKey())

        // Entries survive the save/restore round trip used for process death.
        val saved = with(TvFocusMemory.Saver) { FakeSaverScope.save(memory) }
        val restored = TvFocusMemory.Saver.restore(saved!!)!!
        restored.destination = "Favorites"
        assertEquals("/films/b", restored.restoreKey())
    }

}

private object FakeSaverScope : SaverScope {
    override fun canBeSaved(value: Any) = true
}
