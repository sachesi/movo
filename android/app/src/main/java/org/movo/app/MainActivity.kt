@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app

import android.annotation.SuppressLint
import android.app.UiModeManager
import android.content.Intent
import android.speech.RecognizerIntent
import android.webkit.WebChromeClient
import android.webkit.WebView
import androidx.activity.ComponentActivity
import android.content.Context
import android.content.res.Configuration
import android.os.Build
import android.os.Bundle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.animation.core.LinearEasing
import androidx.compose.animation.core.LinearOutSlowInEasing
import androidx.compose.animation.core.RepeatMode
import androidx.compose.animation.core.animateFloat
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.infiniteRepeatable
import androidx.compose.animation.core.rememberInfiniteTransition
import androidx.compose.animation.core.tween
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.slideInHorizontally
import androidx.compose.animation.slideOutHorizontally
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.clickable
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.focusGroup
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.interaction.collectIsFocusedAsState
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.aspectRatio
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyRow
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.rememberLazyGridState
import androidx.compose.foundation.lazy.grid.items
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.automirrored.filled.Comment
import androidx.compose.material.icons.automirrored.filled.Logout
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.material3.windowsizeclass.WindowWidthSizeClass
import androidx.compose.material3.windowsizeclass.WindowHeightSizeClass
import androidx.compose.material3.windowsizeclass.calculateWindowSizeClass
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.key
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.Stable
import androidx.compose.runtime.saveable.Saver
import androidx.compose.runtime.saveable.mapSaver
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.saveable.rememberSaveableStateHolder
import androidx.compose.runtime.setValue
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.runtime.snapshotFlow
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.TransformOrigin
import androidx.compose.ui.graphics.graphicsLayer
import androidx.compose.ui.draw.drawBehind
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.layout.onPlaced
import androidx.compose.ui.focus.onFocusChanged
import androidx.compose.ui.focus.FocusRequester
import androidx.compose.ui.focus.focusRequester
import androidx.compose.ui.focus.focusProperties
import androidx.compose.ui.focus.focusRestorer
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.res.pluralStringResource
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import androidx.compose.ui.zIndex
import androidx.compose.ui.viewinterop.AndroidView
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.core.view.WindowCompat
import androidx.lifecycle.viewmodel.compose.viewModel
import coil3.compose.AsyncImage
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.collect
import kotlinx.coroutines.flow.distinctUntilChanged
import androidx.tv.material3.DrawerValue
import androidx.tv.material3.Card as TvCard
import androidx.tv.material3.CardScale
import androidx.tv.material3.Button as TvButton
import androidx.tv.material3.ExperimentalTvMaterial3Api
import androidx.tv.material3.FilterChip as TvFilterChip
import androidx.tv.material3.IconButton as TvIconButton
import androidx.tv.material3.ListItem as TvListItem
import androidx.tv.material3.ListItemScale
import androidx.tv.material3.SelectableChipScale
import androidx.tv.material3.Surface as TvSurface
import androidx.tv.material3.Icon as TvIcon
import androidx.tv.material3.MaterialTheme as TvMaterialTheme
import androidx.tv.material3.NavigationDrawer
import androidx.tv.material3.NavigationDrawerItem
import androidx.tv.material3.NavigationDrawerItemScale
import androidx.tv.material3.Text as TvText
import androidx.tv.material3.darkColorScheme as tvDarkColorScheme
import androidx.tv.material3.lightColorScheme as tvLightColorScheme

private val MovoBlue = Color(0xFF9CCAFF)
/**
 * Per-destination record of the last focused element on the TV surface, so returning to a
 * destination puts the highlight back where it was.
 *
 * Deliberately *not* snapshot state: every D-pad move records a new key, and observable state
 * here would invalidate whichever composition provides it — the whole destination — on every
 * single focus change. The entries are write-mostly and read only when a node first composes,
 * so a plain map behind a stable holder is both correct and free.
 */
@Stable
internal class TvFocusMemory(entries: Map<String, String> = emptyMap()) {
    private val entries = HashMap(entries)

    /** Destination currently on screen. Assigned by the TV shell before its content composes. */
    var destination: String = ""

    /** Key to fall back to when the destination has no recorded focus yet. */
    var fallback: String? = null

    fun record(key: String) {
        if (destination.isNotEmpty()) entries[destination] = key
    }

    fun restoreKey(): String? = entries[destination] ?: fallback

    companion object {
        val Saver: Saver<TvFocusMemory, Any> = mapSaver(
            save = { HashMap<String, Any?>(it.entries) },
            restore = { saved -> TvFocusMemory(saved.mapValues { (_, value) -> value as String }) },
        )
    }
}

internal val LocalTvFocusMemory = staticCompositionLocalOf<TvFocusMemory?> { null }

/** Records focus gains for [key] and, on the destination's first composition, restores it. */
@Composable
internal fun Modifier.tvFocusMemory(key: String): Modifier {
    val memory = LocalTvFocusMemory.current ?: return this
    val focusRequester = remember { FocusRequester() }
    val restoreOnEntry = remember { memory.restoreKey() == key }
    val tracked = this
        .focusRequester(focusRequester)
        .onFocusChanged { if (it.isFocused) memory.record(key) }
    if (!restoreOnEntry) return tracked
    // Requested from onPlaced rather than a launched effect: a focus request against a node that
    // has not been placed yet is dropped, which is how restoration silently lands on the wrong
    // element. The guard keeps it to the first placement so scrolling cannot steal focus back.
    val restored = remember { RestoreOnce() }
    return tracked.onPlaced {
        if (!restored.done) {
            restored.done = true
            runCatching { focusRequester.requestFocus() }
        }
    }
}

private class RestoreOnce {
    var done = false
}

class MainActivity : ComponentActivity() {
    private val deepLinks = MutableStateFlow<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        deepLinks.value = intent?.data?.takeIf { it.scheme == "https" && it.host == "hdrzk.org" }?.toString()
        enableEdgeToEdge()
        setContent { MovoApp(deepLinks = deepLinks, consumeDeepLink = { deepLinks.value = null }) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        deepLinks.value = intent.data?.takeIf { it.scheme == "https" && it.host == "hdrzk.org" }?.toString()
    }
}

enum class Section { Home, Settings }

/**
 * Root composable. Resolves the Material 3 color scheme (dynamic color when available),
 * determines the effective Phone/TV mode (manual override in Settings wins over auto-detection)
 * and routes between full-screen surfaces: splash, login, the first-class player, the detail view
 * and the adaptive home flow (tabs + Settings).
 */
@Composable
private fun MovoApp(
    deepLinks: kotlinx.coroutines.flow.StateFlow<String?>,
    consumeDeepLink: () -> Unit,
    model: MovoViewModel = viewModel(),
) {
    val context = LocalContext.current
    val configuration = LocalConfiguration.current
    val loadedSettings by context.settings.collectAsStateWithLifecycle(initialValue = null)

    val isSystemTv =
        configuration.uiMode and Configuration.UI_MODE_TYPE_MASK ==
            Configuration.UI_MODE_TYPE_TELEVISION ||
            (context.getSystemService(Context.UI_MODE_SERVICE) as UiModeManager)
                .currentModeType == Configuration.UI_MODE_TYPE_TELEVISION

    val settings = loadedSettings
    if (settings == null) {
        MaterialTheme {
            Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
                Splash { Loading(stringResource(R.string.restoring_session)) }
            }
        }
        return
    }

    val isTv = when (settings.layoutMode) {
        LayoutMode.Auto -> isSystemTv
        LayoutMode.Phone -> false
        LayoutMode.Tv -> true
    }

    val activity = context as ComponentActivity
    val windowSize = calculateWindowSizeClass(activity)
    val useRail = usesNavigationRail(isTv, windowSize.widthSizeClass)
    val compactHeight = windowSize.heightSizeClass == WindowHeightSizeClass.Compact

    val isDark = resolveDarkTheme(settings.theme, isSystemInDarkTheme())
    val dynamic = settings.useDynamicColor && Build.VERSION.SDK_INT >= 31
    var initialTabApplied by rememberSaveable { mutableStateOf(false) }
    val colorScheme = when {
        dynamic && isDark -> dynamicDarkColorScheme(context)
        dynamic -> dynamicLightColorScheme(context)
        isDark -> darkColorScheme(
            primary = MovoBlue,
            secondary = MovoBlue,
            tertiary = Color(0xFFB9E5FF),
            surface = Color(0xFF101114),
        )
        else -> lightColorScheme(
            primary = Color(0xFF0061A4),
            secondary = Color(0xFF2D70D4),
            tertiary = Color(0xFF003B6B),
        )
    }

    val systemUi = WindowCompat.getInsetsController(activity.window, activity.window.decorView)
    SideEffect {
        systemUi.isAppearanceLightStatusBars = !isDark
        systemUi.isAppearanceLightNavigationBars = !isDark
    }

    MaterialTheme(colorScheme = colorScheme) {
        Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.background) {
            val screen by model.screen.collectAsStateWithLifecycle(initialValue = Screen.Restoring)
            val deepLink by deepLinks.collectAsStateWithLifecycle()
            LaunchedEffect(screen, settings.initialTab, isTv) {
                if (screen == Screen.Login) initialTabApplied = false
                if (screen == Screen.Home && !initialTabApplied) {
                    initialTabApplied = true
                    model.selectTab(settings.initialTab, isTv)
                }
            }
            LaunchedEffect(screen, deepLink) {
                val url = deepLink ?: return@LaunchedEffect
                if (screen == Screen.Home || screen == Screen.Details) {
                    consumeDeepLink()
                    model.openDetails(url)
                }
            }
            when (screen) {
                Screen.Restoring -> Splash { Loading(stringResource(R.string.restoring_session)) }
                Screen.Login -> LoginRoute(model, isTv)
                Screen.Player -> PlayerRoute(model, settings, isTv)
                Screen.Trailer -> TrailerRoute(model, isTv)
                Screen.Details -> DetailsRoute(
                    model,
                    isTv,
                    windowSize.widthSizeClass != WindowWidthSizeClass.Compact,
                    settings,
                )
                Screen.Home -> HomeRoute(model, isTv, useRail, compactHeight, settings, isDark)
            }
        }
    }
}

internal fun usesNavigationRail(isTv: Boolean, width: WindowWidthSizeClass) =
    isTv || width != WindowWidthSizeClass.Compact

internal fun resolveDarkTheme(theme: ThemePref, systemDark: Boolean) = when (theme) {
    ThemePref.System -> systemDark
    ThemePref.Light -> false
    ThemePref.Dark -> true
}

@Composable
private fun LoginRoute(model: MovoViewModel, isTv: Boolean) {
    val state by model.state.collectAsStateWithLifecycle()
    Splash {
        LoginScreen(
            loading = state.loading,
            error = state.error,
            isTv = isTv,
            login = model::login,
            clearError = model::clearError,
        )
    }
}

@Composable
private fun PlayerRoute(model: MovoViewModel, settings: AppSettings, isTv: Boolean) {
    val state by model.state.collectAsStateWithLifecycle()
    val stream = state.stream ?: return
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    // Both scan every episode of every season, so they are kept off the recomposition path.
    val adjacentEpisodes = remember(state.details, stream) {
        model.hasAdjacentEpisode(-1) to model.hasAdjacentEpisode(1)
    }
    PlayerScreen(
        bundle = stream,
        title = state.details?.title.orEmpty(),
        resumePositionMs = state.playbackPositionMs,
        qualityMode = settings.qualityMode,
        preferredQuality = state.playbackQuality,
        saveProgress = model::saveProgress,
        playbackStarted = model::playbackStarted,
        syncError = state.error,
        close = model::closePlayer,
        isTv = isTv,
        seekSeconds = settings.seekSeconds,
        initialSpeed = settings.playbackSpeed,
        videoFit = settings.videoFit,
        previousEpisode = model::previousEpisode,
        nextEpisode = model::nextEpisode,
        hasPreviousEpisode = adjacentEpisodes.first,
        hasNextEpisode = adjacentEpisodes.second,
        autoNext = settings.autoNext,
        seasons = state.details?.seasons.orEmpty(),
        playEpisode = model::playEpisode,
        showBuffer = settings.showBuffer,
        showEndTime = settings.showEndTime,
        bufferSeconds = settings.bufferSeconds,
        tvCenterPauses = settings.tvCenterPauses,
        tvPauseShowsControls = settings.tvPauseShowsControls,
        openRating = { model.openPlayerAction(DetailAction.Rating, it) },
        qualityChanged = { quality, persist ->
            model.selectPlaybackQuality(quality)
            if (persist && settings.saveQuality) scope.launch { context.saveLastQuality(quality) }
        },
    )
}

@Composable
@SuppressLint("SetJavaScriptEnabled")
private fun TrailerRoute(model: MovoViewModel, isTv: Boolean) {
    val state by model.state.collectAsStateWithLifecycle()
    val url = state.trailerUrl ?: return
    val context = LocalContext.current
    val webView = remember(url) {
        WebView(context).apply {
            settings.javaScriptEnabled = true
            settings.mediaPlaybackRequiresUserGesture = false
            settings.allowFileAccess = false
            settings.allowContentAccess = false
            settings.mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
            webChromeClient = WebChromeClient()
            loadUrl(url)
        }
    }
    DisposableEffect(webView) { onDispose { webView.stopLoading(); webView.destroy() } }
    BackHandler { model.clearTrailer() }
    Box(Modifier.fillMaxSize().background(Color.Black)) {
        AndroidView(factory = { webView }, modifier = Modifier.fillMaxSize())
        IconButton(model::clearTrailer, Modifier.align(Alignment.TopStart).padding(16.dp).tvFocusScale(isTv)) {
            Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back), tint = Color.White)
        }
    }
}

@Composable
private fun DetailsRoute(
    model: MovoViewModel,
    isTv: Boolean,
    wideContent: Boolean,
    settings: AppSettings,
) {
    val state by model.state.collectAsStateWithLifecycle()
    DetailsScreen(state, isTv, wideContent, settings, model)
}

@Composable
private fun HomeRoute(
    model: MovoViewModel,
    isTv: Boolean,
    useRail: Boolean,
    compactHeight: Boolean,
    settings: AppSettings,
    isDark: Boolean,
) {
    val state by model.state.collectAsStateWithLifecycle()
    HomeFlow(state, isTv, useRail, compactHeight, model, settings, isDark)
}

@Composable
private fun Splash(content: @Composable () -> Unit) {
    // Edge-to-edge draws under the camera cutout; keep the branding clear of display
    // cutouts and system bars instead of centering in the full screen.
    Box(
        Modifier
            .fillMaxSize()
            .windowInsetsPadding(WindowInsets.displayCutout)
            .windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Bottom)),
        contentAlignment = Alignment.Center,
    ) {
        Column(horizontalAlignment = Alignment.CenterHorizontally, verticalArrangement = Arrangement.spacedBy(12.dp)) {
            Text(
                stringResource(R.string.app_name),
                style = MaterialTheme.typography.headlineMedium,
                color = MaterialTheme.colorScheme.primary,
            )
            Text(
                stringResource(R.string.splash_tagline),
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
            content()
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun HomeFlow(
    state: AppState,
    isTv: Boolean,
    useRail: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
    settings: AppSettings,
    isDark: Boolean,
) {
    if (isTv) {
        TvHomeFlow(state, compactHeight, model, settings, isDark)
        return
    }
    var section by rememberSaveable { mutableStateOf(Section.Home) }

    Scaffold(
        topBar = {
            if (section == Section.Home) {
                HomeTopAppBar(state, model, isTv) { section = Section.Settings }
            } else {
                CenterAlignedTopAppBar(
                    title = { Text(stringResource(R.string.settings_title)) },
                    navigationIcon = {
                        IconButton({ section = Section.Home }, Modifier.tvFocusScale(isTv)) {
                            Icon(
                                imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                                contentDescription = stringResource(R.string.back),
                            )
                        }
                    },
                )
            }
        },
        bottomBar = {
            if (section == Section.Home && !useRail) HomeBottomBar(state, model)
        },
    ) { padding ->
        Row(Modifier.padding(padding).fillMaxSize()) {
            if (section == Section.Home && useRail) HomeNavigationRail(state, model, isTv) { section = Section.Settings }
            Box(
                Modifier
                    .weight(1f)
                    .fillMaxHeight()
                    .focusGroup(),
            ) {
                AnimatedContent(targetState = section, transitionSpec = { fadeIn() togetherWith fadeOut() }) { s ->
                    when (s) {
                        Section.Home -> HomeTabs(state, false, compactHeight, model)
                        Section.Settings -> SettingsDestination(settings, false)
                    }
                }
                if (state.loading) {
                    LinearProgressIndicator(Modifier.fillMaxWidth().align(Alignment.TopCenter))
                }
                state.error?.let { ErrorBanner(it, model::clearError, Modifier.align(Alignment.BottomCenter), retry = { model.retry(isTv) }, isTv = isTv) }
            }
        }
    }

    BackHandler(enabled = section == Section.Settings) { section = Section.Home }
}

@Composable
private fun TvHomeFlow(
    state: AppState,
    compactHeight: Boolean,
    model: MovoViewModel,
    settings: AppSettings,
    isDark: Boolean,
) {
    var section by rememberSaveable { mutableStateOf(Section.Home) }
    val focusMemory = rememberSaveable(saver = TvFocusMemory.Saver) { TvFocusMemory() }
    val stateHolder = rememberSaveableStateHolder()
    val destinationKey = if (section == Section.Settings) "settings" else state.tab.name
    focusMemory.destination = destinationKey
    focusMemory.fallback = state.focusedUrl

    TvMaterialTheme(
        colorScheme = if (isDark) {
            tvDarkColorScheme(primary = MovoBlue)
        } else {
            tvLightColorScheme(primary = Color(0xFF0061A4))
        },
    ) {
        TvNavigationDrawer(
            selectedTab = state.tab,
            settingsSelected = section == Section.Settings,
            notificationCount = state.notificationCount,
            selectTab = { tab ->
                section = Section.Home
                if (state.tab != tab) model.selectTab(tab, true)
            },
            openSettings = {
                section = Section.Settings
            },
        ) {
            Box(
                Modifier
                    .fillMaxSize()
                    .focusRestorer()
                    .focusGroup(),
            ) {
                stateHolder.SaveableStateProvider(destinationKey) {
                    CompositionLocalProvider(LocalTvFocusMemory provides focusMemory) {
                        when (section) {
                            Section.Home -> HomeTabContent(state.tab, state, true, compactHeight, model)
                            Section.Settings -> SettingsDestination(settings, true)
                        }
                    }
                }
                if (state.loading) {
                    LinearProgressIndicator(Modifier.fillMaxWidth().align(Alignment.TopCenter))
                }
                state.error?.let {
                    ErrorBanner(
                        it,
                        model::clearError,
                        Modifier.align(Alignment.BottomCenter),
                        retry = { model.retry(true) },
                        isTv = true,
                    )
                }
            }
        }
    }

    BackHandler(enabled = section == Section.Settings) { section = Section.Home }
}

@Composable
internal fun TvNavigationDrawer(
    selectedTab: Tab,
    settingsSelected: Boolean,
    notificationCount: Int,
    selectTab: (Tab) -> Unit,
    openSettings: () -> Unit,
    content: @Composable () -> Unit,
) {
    NavigationDrawer(
        modifier = Modifier.testTag("tv-navigation-drawer"),
        drawerContent = { drawerValue ->
            val expanded = drawerValue == DrawerValue.Open
            Column(
                Modifier
                    .fillMaxHeight()
                    .padding(horizontal = 12.dp, vertical = 20.dp)
                    .selectableGroup(),
                verticalArrangement = Arrangement.spacedBy(6.dp),
            ) {
                Tab.entries.filter { it != Tab.Account }.forEach { tab ->
                    NavigationDrawerItem(
                        selected = !settingsSelected && selectedTab == tab,
                        onClick = { selectTab(tab) },
                        leadingContent = { TvIcon(tab.icon, contentDescription = stringResource(tab.label)) },
                        modifier = Modifier.testTag("tv-drawer-${tab.name.lowercase()}"),
                        trailingContent = if (
                            expanded && tab == Tab.Notifications && notificationCount > 0
                        ) {
                            { Badge { Text(notificationCount.coerceAtMost(99).toString()) } }
                        } else {
                            null
                        },
                        scale = NavigationDrawerItemScale.None,
                    ) {
                        if (expanded) TvText(stringResource(tab.label))
                    }
                }
                Spacer(Modifier.weight(1f))
                NavigationDrawerItem(
                    selected = !settingsSelected && selectedTab == Tab.Account,
                    onClick = { selectTab(Tab.Account) },
                    leadingContent = { TvIcon(Tab.Account.icon, contentDescription = stringResource(Tab.Account.label)) },
                    modifier = Modifier.testTag("tv-drawer-account"),
                    scale = NavigationDrawerItemScale.None,
                ) {
                    if (expanded) TvText(stringResource(Tab.Account.label))
                }
                NavigationDrawerItem(
                    selected = settingsSelected,
                    onClick = openSettings,
                    leadingContent = { TvIcon(Icons.Default.Settings, contentDescription = stringResource(R.string.settings_title)) },
                    modifier = Modifier.testTag("tv-drawer-settings"),
                    scale = NavigationDrawerItemScale.None,
                ) {
                    if (expanded) TvText(stringResource(R.string.settings_title))
                }
            }
        },
        content = content,
    )
}

@Composable
private fun SettingsDestination(settings: AppSettings, isTv: Boolean) {
    val context = LocalContext.current
    val scope = rememberCoroutineScope()
    SettingsContent(
        settings = settings,
        isTv = isTv,
        onLayoutModeChange = { scope.launch { context.saveLayoutMode(it) } },
        onThemeChange = { scope.launch { context.saveTheme(it) } },
        onDynamicColorChange = { scope.launch { context.saveUseDynamicColor(it) } },
        onQualityModeChange = { scope.launch { context.saveQualityMode(it) } },
        onAutoNextChange = { scope.launch { context.saveAutoNext(it) } },
        onSeekSecondsChange = { scope.launch { context.saveSeekSeconds(it) } },
        onPlaybackSpeedChange = { scope.launch { context.savePlaybackSpeed(it) } },
        onVideoFitChange = { scope.launch { context.saveVideoFit(it) } },
        onShowBufferChange = { scope.launch { context.saveShowBuffer(it) } },
        onShowEndTimeChange = { scope.launch { context.saveShowEndTime(it) } },
        onBufferSecondsChange = { scope.launch { context.saveBufferSeconds(it) } },
        onTvCenterPausesChange = { scope.launch { context.saveTvCenterPauses(it) } },
        onTvPauseShowsControlsChange = { scope.launch { context.saveTvPauseShowsControls(it) } },
        onAskQualityChange = { scope.launch { context.saveAskQuality(it) } },
        onSaveQualityChange = { scope.launch { context.saveSaveQuality(it) } },
        onSortVoicesChange = { scope.launch { context.saveSortVoices(it) } },
        onInitialTabChange = { scope.launch { context.saveInitialTab(it) } },
    )
}

@Composable
private fun HomeTopAppBar(state: AppState, model: MovoViewModel, isTv: Boolean, onOpenSettings: () -> Unit) {
    var expanded by remember { mutableStateOf(false) }
    var confirmLogout by remember { mutableStateOf(false) }
    TopAppBar(
        title = { Text(stringResource(R.string.app_name)) },
        actions = {
            state.user?.let {
                Text(
                    text = it.username,
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(end = 8.dp),
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            IconButton(onClick = onOpenSettings, modifier = Modifier.tvFocusScale(isTv)) {
                Icon(Icons.Default.Settings, contentDescription = stringResource(R.string.settings_title))
            }
            IconButton(onClick = { expanded = true }, modifier = Modifier.tvFocusScale(isTv)) {
                Icon(imageVector = Icons.Default.MoreVert, contentDescription = stringResource(R.string.menu))
            }
            DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
                DropdownMenuItem(
                    onClick = { expanded = false; confirmLogout = true },
                    text = { Text(stringResource(R.string.menu_sign_out)) },
                )
            }
        },
    )
    if (confirmLogout) ConfirmLogoutDialog(isTv, { confirmLogout = false }) {
        confirmLogout = false
        model.logout()
    }
}

@Composable
internal fun ConfirmLogoutDialog(isTv: Boolean, dismiss: () -> Unit, confirm: () -> Unit) {
    val cancelFocusRequester = remember { FocusRequester() }
    if (isTv) LaunchedEffect(Unit) { cancelFocusRequester.requestFocus() }
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text(stringResource(R.string.sign_out_title)) },
        text = { Text(stringResource(R.string.sign_out_message)) },
        confirmButton = {
            if (isTv) TvButton(onClick = confirm) { TvText(stringResource(R.string.menu_sign_out)) }
            else TextButton(confirm) { Text(stringResource(R.string.menu_sign_out)) }
        },
        dismissButton = {
            if (isTv) {
                TvButton(
                    onClick = dismiss,
                    modifier = Modifier.focusRequester(cancelFocusRequester).testTag("tv-dialog-cancel"),
                ) {
                    TvText(stringResource(R.string.cancel))
                }
            } else {
                TextButton(dismiss) { Text(stringResource(R.string.cancel)) }
            }
        },
    )
}

@Composable
private fun HomeBottomBar(state: AppState, model: MovoViewModel) {
    val tabs = Tab.entries
    NavigationBar {
        tabs.forEach { tab ->
            NavigationBarItem(
                selected = state.tab == tab,
                onClick = { if (state.tab != tab) model.selectTab(tab, false) },
                icon = { TabIcon(tab, state.notificationCount) },
                label = { Text(stringResource(tab.label)) },
            )
        }
    }
}

@Composable
private fun HomeNavigationRail(
    state: AppState,
    model: MovoViewModel,
    isTv: Boolean,
    openSettings: () -> Unit,
) {
    val tabs = Tab.entries
    NavigationRail {
        Spacer(Modifier.height(12.dp))
        tabs.forEach { tab ->
            MovoNavigationRailItem(
                selected = state.tab == tab,
                onClick = { if (state.tab != tab) model.selectTab(tab, isTv) },
                icon = { TabIcon(tab, state.notificationCount) },
                label = { Text(stringResource(tab.label)) },
                isTv = isTv,
            )
        }
        MovoNavigationRailItem(
            selected = false,
            onClick = openSettings,
            icon = { Icon(Icons.Default.Settings, contentDescription = null) },
            label = { Text(stringResource(R.string.settings_title)) },
            isTv = isTv,
        )
    }
}

@Composable
private fun MovoNavigationRailItem(
    selected: Boolean,
    onClick: () -> Unit,
    icon: @Composable () -> Unit,
    label: @Composable () -> Unit,
    isTv: Boolean,
    modifier: Modifier = Modifier,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val focused by interactionSource.collectIsFocusedAsState()
    val tvFocused = isTv && focused
    val scale by animateFloatAsState(if (tvFocused) 1.06f else 1f, label = "navigation focus")
    NavigationRailItem(
        selected = selected,
        onClick = onClick,
        icon = icon,
        label = label,
        modifier = modifier
            .padding(horizontal = 6.dp, vertical = 2.dp)
            .background(
                if (tvFocused) MaterialTheme.colorScheme.primaryContainer else Color.Transparent,
                RoundedCornerShape(20.dp),
            )
            .graphicsLayer { scaleX = scale; scaleY = scale },
        colors = NavigationRailItemDefaults.colors(
            selectedIconColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSecondaryContainer,
            selectedTextColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurface,
            indicatorColor = if (tvFocused) Color.Transparent else MaterialTheme.colorScheme.secondaryContainer,
            unselectedIconColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
            unselectedTextColor = if (tvFocused) MaterialTheme.colorScheme.onPrimaryContainer else MaterialTheme.colorScheme.onSurfaceVariant,
        ),
        interactionSource = interactionSource,
    )
}

@Composable
private fun TabIcon(tab: Tab, notificationCount: Int) {
    if (tab == Tab.Notifications && notificationCount > 0) {
        BadgedBox(badge = { Badge { Text(notificationCount.coerceAtMost(99).toString()) } }) {
            Icon(tab.icon, contentDescription = null)
        }
    } else Icon(tab.icon, contentDescription = null)
}

@Composable
private fun HomeTabs(
    state: AppState,
    isTv: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
) {
    if (isTv) {
        HomeTabContent(state.tab, state, true, compactHeight, model)
        return
    }
    // Directional slide: forward for tabs later in the list, back for earlier ones, so
    // navigation reads spatially instead of a flat cross-fade.
    AnimatedContent(
        targetState = state.tab,
        transitionSpec = {
            val forward = targetState.ordinal >= initialState.ordinal
            val enter = fadeIn(tween(220)) +
                slideInHorizontally(tween(260)) { if (forward) it / 8 else -it / 8 }
            val exit = fadeOut(tween(180)) +
                slideOutHorizontally(tween(260)) { if (forward) -it / 8 else it / 8 }
            enter togetherWith exit
        },
        label = "home tabs",
    ) { tab ->
        HomeTabContent(tab, state, false, compactHeight, model)
    }
}

@Composable
private fun HomeTabContent(
    tab: Tab,
    state: AppState,
    isTv: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
) {
    when (tab) {
        Tab.Catalog -> CatalogScreen(state, isTv, compactHeight, model)
        Tab.Search -> SearchScreen(state, isTv, model)
        Tab.Collections -> CollectionsScreen(state, isTv, model)
        Tab.Favorites -> FavoritesScreen(state, isTv, model)
        Tab.History -> HistoryScreen(state, isTv, model)
        Tab.Notifications -> NotificationsScreen(state, isTv, model)
        Tab.Account -> AccountScreen(state, isTv, model)
    }
}

private val Tab.icon get() = when (this) {
    Tab.Catalog -> Icons.Default.GridView
    Tab.Search -> Icons.Default.Search
    Tab.Collections -> Icons.Default.CollectionsBookmark
    Tab.Favorites -> Icons.Default.Favorite
    Tab.History -> Icons.Default.History
    Tab.Notifications -> Icons.Default.Notifications
    Tab.Account -> Icons.Default.AccountCircle
}

@Composable
private fun LoginScreen(
    loading: Boolean,
    error: String?,
    isTv: Boolean,
    login: (String, String) -> Unit,
    clearError: () -> Unit,
) {
    var name by rememberSaveable { mutableStateOf("") }
    var password by remember { mutableStateOf("") }
    var passwordVisible by remember { mutableStateOf(false) }
    Box(
        Modifier
            .fillMaxSize()
            .imePadding()
            .verticalScroll(rememberScrollState()),
        contentAlignment = Alignment.Center,
    ) {
        Card(Modifier.widthIn(max = 420.dp).padding(24.dp)) {
            Column(
                Modifier.padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Text(stringResource(R.string.app_name), style = MaterialTheme.typography.headlineLarge)
                Text(stringResource(R.string.sign_in_account))
                OutlinedTextField(
                    value = name,
                    onValueChange = { name = it; clearError() },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(stringResource(R.string.login_hint)) },
                    singleLine = true,
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Email,
                        imeAction = ImeAction.Next,
                    ),
                )
                OutlinedTextField(
                    value = password,
                    onValueChange = { password = it; clearError() },
                    modifier = Modifier.fillMaxWidth(),
                    label = { Text(stringResource(R.string.password_hint)) },
                    singleLine = true,
                    visualTransformation = if (passwordVisible) VisualTransformation.None else PasswordVisualTransformation(),
                    trailingIcon = {
                        IconButton({ passwordVisible = !passwordVisible }, Modifier.tvFocusScale(isTv)) {
                            Icon(
                                if (passwordVisible) Icons.Default.VisibilityOff else Icons.Default.Visibility,
                                contentDescription = stringResource(
                                    if (passwordVisible) R.string.hide_password else R.string.show_password,
                                ),
                            )
                        }
                    },
                    keyboardOptions = KeyboardOptions(
                        keyboardType = KeyboardType.Password,
                        imeAction = ImeAction.Done,
                    ),
                    keyboardActions = KeyboardActions(
                        onDone = { if (name.isNotBlank() && password.isNotBlank()) login(name, password) },
                    ),
                )
                error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                Button(
                    onClick = { login(name, password) },
                    modifier = Modifier.fillMaxWidth().tvFocusScale(isTv),
                    enabled = !loading && name.isNotBlank() && password.isNotBlank(),
                ) {
                    Text(if (loading) stringResource(R.string.signing_in) else stringResource(R.string.sign_in))
                }
                Text(
                    stringResource(R.string.password_disclaimer),
                    style = MaterialTheme.typography.bodySmall,
                )
            }
        }
    }
}

@Composable
private fun CatalogScreen(
    state: AppState,
    isTv: Boolean,
    compactHeight: Boolean,
    model: MovoViewModel,
) {
    if (isTv) {
        if (state.homeSections.isNotEmpty()) {
            TvHomeScreen(state.homeSections, state.loading) { url, key ->
                model.openDetails(url, key)
            }
        } else {
            // TV home is the primary surface — never flash the flat catalog grid while
            // the sections load (the "page loads then swaps to hot news" symptom).
            Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) {
                if (state.loading || state.error == null) TvHomeSkeleton()
                else Empty(stringResource(R.string.home_failed), hint = stringResource(R.string.home_failed_hint))
            }
        }
        return
    }
    Column(Modifier.fillMaxSize()) {
        if (compactHeight) {
            LazyRow(
                contentPadding = PaddingValues(horizontal = 12.dp, vertical = 4.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                item { Text(stringResource(R.string.category), style = MaterialTheme.typography.labelLarge) }
                items(CatalogCategory.entries) { category ->
                    MovoChoiceChip(
                        selected = state.category == category,
                        onClick = { model.setCatalog(category = category) },
                        label = { Text(stringResource(category.label)) },
                        isTv = isTv,
                    )
                }
                item { VerticalDivider(Modifier.height(32.dp)) }
                item { Text(stringResource(R.string.sort_by), style = MaterialTheme.typography.labelLarge) }
                items(SORT_OPTIONS) { (value, label) ->
                    MovoChoiceChip(
                        selected = state.sort == value,
                        onClick = { model.setCatalog(sort = value) },
                        label = { Text(stringResource(label)) },
                        isTv = isTv,
                    )
                }
            }
        } else {
            Text(
                stringResource(R.string.category),
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.padding(start = 12.dp, top = 12.dp),
            )
            LazyRow(
                contentPadding = PaddingValues(horizontal = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(CatalogCategory.entries) { category ->
                    MovoChoiceChip(
                        selected = state.category == category,
                        onClick = { model.setCatalog(category = category) },
                        label = { Text(stringResource(category.label)) },
                        isTv = isTv,
                    )
                }
            }
            Text(
                stringResource(R.string.sort_by),
                style = MaterialTheme.typography.labelLarge,
                modifier = Modifier.padding(start = 12.dp, top = 8.dp),
            )
            LazyRow(
                contentPadding = PaddingValues(horizontal = 12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(SORT_OPTIONS) { (value, label) ->
                    MovoChoiceChip(
                        selected = state.sort == value,
                        onClick = { model.setCatalog(sort = value) },
                        label = { Text(stringResource(label)) },
                        isTv = isTv,
                    )
                }
            }
        }
        key(state.category, state.sort) {
            MediaGrid(
                state.items,
                isTv,
                state.loading,
                state.focusedUrl,
                { url -> model.openDetails(url, url) },
                emptyTitle = stringResource(R.string.catalog_empty),
                emptyHint = stringResource(R.string.catalog_empty_hint),
            ) { model.loadCatalog(true) }
        }
    }
}

@Composable
internal fun TvHomeScreen(
    sections: List<HomeSection>,
    loading: Boolean,
    open: (String, String) -> Unit,
) {
    LazyColumn(
        Modifier.fillMaxSize().testTag("tv-home-list").focusRestorer().focusGroup(),
        contentPadding = PaddingValues(horizontal = 48.dp, vertical = 27.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        sections.filter { it.items.isNotEmpty() }.forEach { section ->
            item(section.id) {
                Column(
                    Modifier.testTag("tv-home-rail-${section.id}"),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                ) {
                    Text(
                        stringResource(section.title),
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.onSurface,
                    )
                    LazyRow(
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                        modifier = Modifier.focusRestorer().focusGroup(),
                    ) {
                        items(section.items, key = { media -> media.url }) { media ->
                            val key = "${section.id}:${media.url}"
                            Box(Modifier.width(176.dp).testTag("tv-home-card-$key")) {
                                MediaCard(
                                    media,
                                    true,
                                    focusKey = key,
                                ) { open(media.url, key) }
                            }
                        }
                    }
                }
            }
        }
        if (loading) item { LinearProgressIndicator(Modifier.fillMaxWidth()) }
    }
}

private val HomeSection.title: Int get() = when (id) {
    "hot" -> R.string.home_hot
    "new" -> R.string.sort_new
    "watching" -> R.string.sort_watching
    "popular" -> R.string.sort_popular
    "awaiting" -> R.string.home_awaiting
    else -> R.string.nav_catalog
}

private val SORT_OPTIONS = listOf(
    "popular" to R.string.sort_popular,
    "watching" to R.string.sort_watching,
    "new" to R.string.sort_new,
    "last" to R.string.sort_last,
)

private val CatalogCategory.label: Int get() = when (this) {
    CatalogCategory.All -> R.string.category_all
    CatalogCategory.Films -> R.string.category_films
    CatalogCategory.Series -> R.string.category_series
    CatalogCategory.Cartoons -> R.string.category_cartoons
    CatalogCategory.Animation -> R.string.category_animation
}

private val Tab.label: Int get() = when (this) {
    Tab.Catalog -> R.string.nav_catalog
    Tab.Search -> R.string.nav_search
    Tab.Collections -> R.string.nav_collections
    Tab.Favorites -> R.string.nav_favorites
    Tab.History -> R.string.nav_history
    Tab.Notifications -> R.string.nav_notifications
    Tab.Account -> R.string.nav_account
}

@Composable
private fun SearchScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    var query by rememberSaveable { mutableStateOf(state.query) }
    var showFilters by remember { mutableStateOf(false) }
    val context = LocalContext.current
    val voiceIntent = remember {
        Intent(RecognizerIntent.ACTION_RECOGNIZE_SPEECH)
            .putExtra(RecognizerIntent.EXTRA_LANGUAGE_MODEL, RecognizerIntent.LANGUAGE_MODEL_FREE_FORM)
    }
    val voiceAvailable = remember(context) { voiceIntent.resolveActivity(context.packageManager) != null }
    val voice = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        result.data?.getStringArrayListExtra(RecognizerIntent.EXTRA_RESULTS)?.firstOrNull()?.let {
            query = it
            model.search(it)
        }
    }
    LaunchedEffect(state.query) { query = state.query }
    LaunchedEffect(Unit) { model.loadSearchFilters() }
    if (state.collectionPath != null) {
        Column(Modifier.fillMaxSize()) {
            PathHeader(state.collectionTitle.orEmpty(), isTv, model::closeCollection)
            key(state.collectionPath) {
                MediaGrid(
                    state.items,
                    isTv,
                    state.loading,
                    state.focusedUrl,
                    { url -> model.openDetails(url, url) },
                    emptyTitle = stringResource(R.string.collection_empty),
                ) { model.loadPath(append = true) }
            }
        }
        BackHandler { model.closeCollection() }
        return
    }
    if (isTv) {
        TvSearchContent(
            state = state,
            query = query,
            updateQuery = { value -> query = value; model.suggest(value) },
            voiceAvailable = voiceAvailable,
            launchVoice = { voice.launch(voiceIntent) },
            search = { model.search(query) },
            chooseHistory = { value -> query = value; model.search(value) },
            clearHistory = model::clearSearchHistory,
            openFilters = { showFilters = true },
            open = { url -> model.openDetails(url, url) },
            loadMore = { model.search(query, true) },
        )
        if (showFilters) SearchFiltersDialog(state.searchFilters, true, { showFilters = false }) { title, path ->
            showFilters = false
            model.openPath(title, path)
        }
        return
    }
    Column(Modifier.fillMaxSize().padding(top = 12.dp)) {
        OutlinedTextField(
            value = query,
            onValueChange = { query = it; model.suggest(it) },
            modifier = Modifier
                .widthIn(max = if (isTv) 840.dp else 720.dp)
                .fillMaxWidth()
                .align(Alignment.CenterHorizontally)
                .padding(horizontal = 12.dp),
            label = { Text(stringResource(R.string.search_hint)) },
            trailingIcon = {
                Row {
                    IconButton({ voice.launch(voiceIntent) }, Modifier.tvFocusScale(isTv), enabled = voiceAvailable) {
                        Icon(Icons.Default.Mic, stringResource(R.string.voice_search))
                    }
                    IconButton({ model.search(query) }, Modifier.tvFocusScale(isTv), enabled = query.isNotBlank()) {
                        Icon(Icons.Default.Search, stringResource(R.string.search))
                    }
                }
            },
            singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
            keyboardActions = KeyboardActions(onSearch = { if (query.isNotBlank()) model.search(query) }),
        )
        if (state.suggestions.isNotEmpty()) {
            LazyRow(contentPadding = PaddingValues(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(state.suggestions, key = { it }) { suggestion ->
                    SuggestionChip(onClick = { query = suggestion; model.search(suggestion) }, label = { Text(suggestion) }, modifier = Modifier.tvFocusScale(isTv))
                }
            }
        }
        if (query.isBlank() && state.searchHistory.isNotEmpty()) {
            Row(Modifier.fillMaxWidth().padding(horizontal = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                Text(stringResource(R.string.recent_searches), style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
                TextButton(model::clearSearchHistory, Modifier.tvFocusScale(isTv)) { Text(stringResource(R.string.clear)) }
            }
            LazyRow(contentPadding = PaddingValues(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                items(state.searchHistory, key = { it }) { value ->
                    AssistChip(onClick = { query = value; model.search(value) }, label = { Text(value) }, modifier = Modifier.tvFocusScale(isTv), leadingIcon = { Icon(Icons.Default.History, null) })
                }
            }
        }
        if (state.searchFilters.isNotEmpty()) {
            LazyRow(contentPadding = PaddingValues(horizontal = 12.dp), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                item { FilledTonalButton({ showFilters = true }, Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Tune, null); Text(stringResource(R.string.advanced_search)) } }
            }
        }
        Spacer(Modifier.height(8.dp))
        key(state.query) {
            MediaGrid(
                state.items,
                isTv,
                state.loading,
                state.focusedUrl,
                { url -> model.openDetails(url, url) },
                emptyTitle = searchEmptyTitle(state.query),
                emptyHint = searchEmptyHint(state.query),
            ) { model.search(query, true) }
        }
    }
    if (showFilters) SearchFiltersDialog(state.searchFilters, isTv, { showFilters = false }) { title, path ->
        showFilters = false
        model.openPath(title, path)
    }
}

@Composable
internal fun TvSearchContent(
    state: AppState,
    query: String,
    updateQuery: (String) -> Unit,
    voiceAvailable: Boolean,
    launchVoice: () -> Unit,
    search: () -> Unit,
    chooseHistory: (String) -> Unit,
    clearHistory: () -> Unit,
    openFilters: () -> Unit,
    open: (String) -> Unit,
    loadMore: () -> Unit,
) {
    Column(
        Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
        Row(
            Modifier
                .fillMaxWidth()
                .padding(start = 48.dp, end = 48.dp, top = 27.dp)
                .focusRestorer()
                .focusGroup(),
            horizontalArrangement = Arrangement.spacedBy(12.dp),
            verticalAlignment = Alignment.CenterVertically,
        ) {
            OutlinedTextField(
                value = query,
                onValueChange = updateQuery,
                modifier = Modifier
                    .weight(1f)
                    .tvFocusMemory("search:field")
                    .testTag("tv-search-field"),
                label = { Text(stringResource(R.string.search_hint)) },
                singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                keyboardActions = KeyboardActions(onSearch = { if (query.isNotBlank()) search() }),
            )
            TvIconButton(
                onClick = launchVoice,
                modifier = Modifier.tvFocusMemory("search:voice").testTag("tv-search-voice"),
                enabled = voiceAvailable,
            ) {
                TvIcon(Icons.Default.Mic, stringResource(R.string.voice_search))
            }
            TvIconButton(
                onClick = search,
                modifier = Modifier.tvFocusMemory("search:submit").testTag("tv-search-submit"),
                enabled = query.isNotBlank(),
            ) {
                TvIcon(Icons.Default.Search, stringResource(R.string.search))
            }
        }
        if (state.suggestions.isNotEmpty()) {
            LazyRow(
                contentPadding = PaddingValues(horizontal = 48.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.focusRestorer().focusGroup(),
            ) {
                items(state.suggestions, key = { it }) { suggestion ->
                    TvFilterChip(
                        selected = false,
                        onClick = { chooseHistory(suggestion) },
                        modifier = Modifier.tvFocusMemory("search:suggestion:$suggestion"),
                        scale = SelectableChipScale.None,
                    ) {
                        TvText(suggestion)
                    }
                }
            }
        }
        if (query.isBlank() && state.searchHistory.isNotEmpty()) {
            Row(
                Modifier.fillMaxWidth().padding(horizontal = 48.dp),
                verticalAlignment = Alignment.CenterVertically,
            ) {
                Text(
                    stringResource(R.string.recent_searches),
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.weight(1f),
                )
                TvButton(
                    onClick = clearHistory,
                    modifier = Modifier.tvFocusMemory("search:clear"),
                ) { TvText(stringResource(R.string.clear)) }
            }
            LazyRow(
                contentPadding = PaddingValues(horizontal = 48.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
                modifier = Modifier.focusRestorer().focusGroup(),
            ) {
                items(state.searchHistory, key = { it }) { value ->
                    TvFilterChip(
                        selected = false,
                        onClick = { chooseHistory(value) },
                        modifier = Modifier.tvFocusMemory("search:history:$value"),
                        scale = SelectableChipScale.None,
                    ) {
                        TvText(value)
                    }
                }
            }
        }
        if (state.searchFilters.isNotEmpty()) {
            Box(Modifier.padding(horizontal = 48.dp)) {
                TvButton(
                    onClick = openFilters,
                    modifier = Modifier.tvFocusMemory("search:filters").testTag("tv-search-filter-row"),
                ) {
                    TvIcon(Icons.Default.Tune, contentDescription = null)
                    TvText(stringResource(R.string.advanced_search))
                }
            }
        }
        Box(Modifier.weight(1f).fillMaxWidth()) {
            MediaGrid(
                state.items,
                true,
                state.loading,
                state.focusedUrl,
                open,
                emptyTitle = searchEmptyTitle(state.query),
                emptyHint = searchEmptyHint(state.query),
                loadMore = loadMore,
            )
        }
    }
}

internal fun searchFilterPath(genre: String, year: String) = when (year) {
    "-1" -> genre.removeSuffix("/best")
    "0", "" -> genre
    else -> genre + year
}

@Composable
private fun SearchFiltersDialog(filters: List<SearchFilter>, isTv: Boolean, dismiss: () -> Unit, apply: (String, String) -> Unit) {
    val recentLabel = stringResource(R.string.recent)
    var filter by remember(filters) { mutableStateOf(filters.first()) }
    var genre by remember(filter) { mutableStateOf(filter.genres.firstOrNull()) }
    var year by remember(filter) { mutableStateOf(filter.years.firstOrNull()) }
    AlertDialog(
        onDismissRequest = dismiss,
        title = { Text(stringResource(R.string.advanced_search)) },
        text = {
            Column(
                Modifier.then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                if (isTv) {
                    TvChoiceRow(stringResource(R.string.category), filters, filter, { it.name }, { it.name }, {
                        filter = it; genre = it.genres.firstOrNull(); year = it.years.firstOrNull()
                    })
                    TvChoiceRow(stringResource(R.string.genre), filter.genres, genre, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { genre = it })
                    TvChoiceRow(stringResource(R.string.year), filter.years, year, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { year = it })
                } else {
                    ChoiceRow(stringResource(R.string.category), filters, filter, { it.name }, { it.name }, {
                        filter = it; genre = it.genres.firstOrNull(); year = it.years.firstOrNull()
                    })
                    ChoiceRow(stringResource(R.string.genre), filter.genres, genre, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { genre = it })
                    ChoiceRow(stringResource(R.string.year), filter.years, year, { if (it.url == "-1") recentLabel else it.name }, { it.url }, { year = it })
                }
            }
        },
        confirmButton = {
            if (isTv) {
                TvButton(
                    onClick = { genre?.let { apply(filter.name, searchFilterPath(it.url, year?.url.orEmpty())) } },
                    enabled = genre != null,
                ) { TvText(stringResource(R.string.search)) }
            } else {
                TextButton(onClick = { genre?.let { apply(filter.name, searchFilterPath(it.url, year?.url.orEmpty())) } }, enabled = genre != null) { Text(stringResource(R.string.search)) }
            }
        },
        dismissButton = {
            if (isTv) TvButton(onClick = dismiss) { TvText(stringResource(R.string.cancel)) }
            else TextButton(dismiss) { Text(stringResource(R.string.cancel)) }
        },
    )
}

@Composable
private fun <T> TvChoiceRow(
    title: String,
    values: List<T>,
    selected: T?,
    label: (T) -> String,
    itemKey: (T) -> Any,
    choose: (T) -> Unit,
) {
    Column {
        Text(title, style = MaterialTheme.typography.titleMedium)
        LazyRow(
            horizontalArrangement = Arrangement.spacedBy(8.dp),
            modifier = Modifier.focusRestorer().focusGroup(),
        ) {
            items(values, key = itemKey) { value ->
                TvFilterChip(
                    selected = value == selected,
                    onClick = { choose(value) },
                    scale = SelectableChipScale.None,
                ) {
                    TvText(label(value))
                }
            }
        }
    }
}

@Composable
private fun CollectionsScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    if (state.collectionPath != null) {
        Column(Modifier.fillMaxSize()) {
            PathHeader(state.collectionTitle.orEmpty(), isTv, model::closeCollection)
            key(state.collectionPath) {
                MediaGrid(
                    state.items,
                    isTv,
                    state.loading,
                    state.focusedUrl,
                    { url -> model.openDetails(url, url) },
                    emptyTitle = stringResource(R.string.collection_empty),
                ) { model.loadPath(append = true) }
            }
        }
        BackHandler { model.closeCollection() }
        return
    }
    if (state.collections.isEmpty()) {
        if (state.loading) Loading(stringResource(R.string.loading_content))
        else Empty(stringResource(R.string.collections_empty))
        return
    }
    val initialIndex = remember(state.collections, state.focusedUrl) {
        state.collections.indexOfFirst { it.url == state.focusedUrl }.coerceAtLeast(0)
    }
    val gridState = rememberLazyGridState(initialFirstVisibleItemIndex = initialIndex)
    LazyVerticalGrid(
        state = gridState,
        columns = if (isTv) GridCells.Fixed(5) else GridCells.Adaptive(220.dp),
        contentPadding = if (isTv) PaddingValues(horizontal = 48.dp, vertical = 27.dp) else PaddingValues(12.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
        modifier = if (isTv) Modifier.focusRestorer().focusGroup() else Modifier.focusGroup(),
    ) {
        items(state.collections, key = { it.url }) { collection ->
            val modifier = if (isTv) Modifier.tvFocusMemory(collection.url) else Modifier
            if (isTv) {
                TvCard(
                    onClick = { model.openCollection(collection) },
                    modifier = modifier,
                    scale = CardScale.None,
                ) {
                    CollectionCardContent(collection)
                }
            } else {
                Card(onClick = { model.openCollection(collection) }, modifier = modifier) {
                    CollectionCardContent(collection)
                }
            }
        }
        item(span = { GridItemSpan(maxLineSpan) }) {
            if (isTv) {
                TvButton(
                    onClick = { model.loadCollections(true) },
                    modifier = Modifier.fillMaxWidth().tvFocusMemory("collections:more"),
                ) { TvText(stringResource(R.string.load_more)) }
            } else {
                TextButton({ model.loadCollections(true) }, Modifier.fillMaxWidth()) {
                    Text(stringResource(R.string.load_more))
                }
            }
        }
    }
}

@Composable
private fun PathHeader(title: String, isTv: Boolean, back: () -> Unit) {
    Row(
        Modifier
            .fillMaxWidth()
            .padding(horizontal = if (isTv) 48.dp else 12.dp, vertical = if (isTv) 12.dp else 0.dp)
            .then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        if (isTv) {
            TvIconButton(onClick = back, modifier = Modifier.tvFocusMemory("path:back")) {
                TvIcon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back))
            }
        } else {
            IconButton(back) { Icon(Icons.AutoMirrored.Filled.ArrowBack, stringResource(R.string.back)) }
        }
        Text(title, style = MaterialTheme.typography.titleLarge)
    }
}

@Composable
private fun CollectionCardContent(collection: CollectionItem) {
                AsyncImage(collection.imageUrl, null, Modifier.fillMaxWidth().aspectRatio(16f / 9f), contentScale = ContentScale.Crop)
                ListItem(
                    headlineContent = { Text(collection.title, maxLines = 2, overflow = TextOverflow.Ellipsis) },
                    supportingContent = { Text(pluralStringResource(R.plurals.collection_items, collection.count, collection.count)) },
                    colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                )
}

@Composable
private fun NotificationsScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    if (state.notifications.isEmpty()) {
        if (state.loading) Loading(stringResource(R.string.loading_content)) else Empty(stringResource(R.string.notifications_empty))
        return
    }
    val focusedIndex = remember(state.notifications, state.focusedUrl) {
        state.notifications
            .flatMap { group -> listOf(group.date) + group.items.map { "${group.date}:${it.url}:${it.info}" } }
            .indexOf(state.focusedUrl)
            .coerceAtLeast(0)
    }
    val listState = rememberLazyListState(initialFirstVisibleItemIndex = focusedIndex)
    LazyColumn(
        state = listState,
        contentPadding = if (isTv) PaddingValues(horizontal = 48.dp, vertical = 27.dp) else PaddingValues(12.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        modifier = if (isTv) Modifier.focusRestorer().focusGroup() else Modifier.focusGroup(),
    ) {
        state.notifications.forEach { group ->
            item(group.date) {
                Text(
                    group.date,
                    style = MaterialTheme.typography.labelLarge,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(start = 4.dp, top = 8.dp),
                )
            }
            items(group.items, key = { "${group.date}:${it.url}:${it.info}" }) { item ->
                val focusKey = "${group.date}:${item.url}:${item.info}"
                val modifier = Modifier
                    .fillMaxWidth()
                    .then(if (isTv) Modifier.tvFocusMemory(focusKey) else Modifier)
                if (isTv) {
                    TvListItem(
                        selected = false,
                        onClick = { model.openDetails(item.url, focusKey) },
                        headlineContent = { TvText(item.title) },
                        supportingContent = { TvText(item.info) },
                        leadingContent = { TvIcon(Icons.Default.NewReleases, null) },
                        modifier = modifier,
                        scale = ListItemScale.None,
                    )
                } else {
                    ElevatedCard(
                        onClick = { model.openDetails(item.url, focusKey) },
                        modifier = modifier,
                    ) {
                    ListItem(
                        headlineContent = { Text(item.title) },
                        supportingContent = { Text(item.info) },
                        leadingContent = { Icon(Icons.Default.NewReleases, null) },
                        colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                    )
                    }
                }
            }
        }
    }
}

@Composable
private fun AccountScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    val user = state.user ?: return
    var confirmLogout by remember { mutableStateOf(false) }
    if (isTv) {
        TvAccountScreen(state) { confirmLogout = true }
        if (confirmLogout) ConfirmLogoutDialog(true, { confirmLogout = false }) {
            confirmLogout = false
            model.logout()
        }
        return
    }
    Box(Modifier.fillMaxSize(), contentAlignment = Alignment.TopCenter) {
        ElevatedCard(Modifier.widthIn(max = 720.dp).padding(if (isTv) 40.dp else 20.dp)) {
            Column(
                Modifier.padding(24.dp).verticalScroll(rememberScrollState()),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    AsyncImage(user.avatarUrl, null, Modifier.size(if (isTv) 112.dp else 80.dp).clip(RoundedCornerShape(18.dp)), contentScale = ContentScale.Crop)
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        Text(user.username, style = MaterialTheme.typography.headlineSmall)
                        user.email?.let { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                        val days = state.premiumDays ?: user.premiumDays
                        if (user.vip || days != null) {
                            AssistChip(
                                onClick = {},
                                enabled = false,
                                label = {
                                    Text(
                                        days?.let { pluralStringResource(R.plurals.premium_days, it, it) }
                                            ?: stringResource(R.string.premium_active),
                                    )
                                },
                                leadingIcon = { Icon(Icons.Default.WorkspacePremium, null, Modifier.size(16.dp)) },
                            )
                        }
                    }
                }
                HorizontalDivider()
                Text(stringResource(R.string.official_account), style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                OutlinedButton({ confirmLogout = true }, Modifier.tvFocusScale(isTv)) {
                    Icon(Icons.AutoMirrored.Filled.Logout, null); Text(stringResource(R.string.menu_sign_out))
                }
                HorizontalDivider()
                AboutSection(isTv)
            }
        }
    }
    if (confirmLogout) ConfirmLogoutDialog(isTv, { confirmLogout = false }) {
        confirmLogout = false
        model.logout()
    }
}

@Composable
private fun TvAccountScreen(state: AppState, requestLogout: () -> Unit) {
    val user = state.user ?: return
    val context = LocalContext.current
    val version = remember(context) {
        runCatching { context.packageManager.getPackageInfo(context.packageName, 0).versionName }
            .getOrNull()
            .orEmpty()
    }
    var aboutExpanded by rememberSaveable { mutableStateOf(false) }
    Box(
        Modifier.fillMaxSize().padding(horizontal = 48.dp, vertical = 27.dp),
        contentAlignment = Alignment.TopCenter,
    ) {
        TvSurface(Modifier.widthIn(max = 720.dp).fillMaxHeight()) {
            LazyColumn(
                Modifier.fillMaxSize().focusRestorer().focusGroup(),
                contentPadding = PaddingValues(24.dp),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                item("profile") {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.spacedBy(16.dp),
                    ) {
                        AsyncImage(
                            user.avatarUrl,
                            null,
                            Modifier.size(112.dp).clip(RoundedCornerShape(18.dp)),
                            contentScale = ContentScale.Crop,
                        )
                        Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                            Text(user.username, style = MaterialTheme.typography.headlineSmall)
                            user.email?.let { Text(it, color = MaterialTheme.colorScheme.onSurfaceVariant) }
                            val days = state.premiumDays ?: user.premiumDays
                            if (user.vip || days != null) {
                                Text(
                                    days?.let { pluralStringResource(R.plurals.premium_days, it, it) }
                                        ?: stringResource(R.string.premium_active),
                                    color = MaterialTheme.colorScheme.primary,
                                )
                            }
                        }
                    }
                }
                item("official") {
                    Text(
                        stringResource(R.string.official_account),
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                item("logout") {
                    TvButton(
                        onClick = requestLogout,
                        modifier = Modifier.tvFocusMemory("account:logout"),
                    ) {
                        TvIcon(Icons.AutoMirrored.Filled.Logout, contentDescription = null)
                        TvText(stringResource(R.string.menu_sign_out))
                    }
                }
                item("about") {
                    TvListItem(
                        selected = aboutExpanded,
                        onClick = { aboutExpanded = !aboutExpanded },
                        modifier = Modifier.tvFocusMemory("account:about"),
                        headlineContent = { TvText(stringResource(R.string.about_title)) },
                        leadingContent = { TvIcon(Icons.Default.Info, contentDescription = null) },
                        trailingContent = {
                            TvIcon(
                                if (aboutExpanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                                contentDescription = stringResource(if (aboutExpanded) R.string.collapse else R.string.more),
                            )
                        },
                        scale = ListItemScale.None,
                    )
                    if (aboutExpanded) {
                        Column(
                            Modifier.padding(horizontal = 16.dp, vertical = 12.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            Text(stringResource(R.string.about_body), style = MaterialTheme.typography.bodyMedium)
                            Text(
                                stringResource(R.string.about_disclaimer),
                                style = MaterialTheme.typography.bodySmall,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                            Text(
                                stringResource(R.string.about_version, version),
                                style = MaterialTheme.typography.labelMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun AboutSection(isTv: Boolean) {
    val context = LocalContext.current
    val version = remember(context) {
        runCatching {
            context.packageManager.getPackageInfo(context.packageName, 0).versionName
        }.getOrNull().orEmpty()
    }
    var expanded by rememberSaveable { mutableStateOf(false) }
    Column(
        Modifier
            .fillMaxWidth()
            .tvFocusScale(isTv, 1.01f)
            .clickable { expanded = !expanded },
    ) {
        Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Icon(
                Icons.Default.Info,
                contentDescription = null,
                tint = MaterialTheme.colorScheme.primary,
                modifier = Modifier.size(20.dp),
            )
            Text(
                stringResource(R.string.about_title),
                style = MaterialTheme.typography.titleMedium,
                modifier = Modifier.weight(1f),
            )
            Icon(
                if (expanded) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                contentDescription = stringResource(if (expanded) R.string.collapse else R.string.more),
                tint = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
        AnimatedVisibility(visible = expanded) {
            Column(Modifier.padding(top = 8.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(stringResource(R.string.about_body), style = MaterialTheme.typography.bodyMedium)
                Text(
                    stringResource(R.string.about_disclaimer),
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
                Text(
                    stringResource(R.string.about_version, version),
                    style = MaterialTheme.typography.labelMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
    }
}

@Composable
private fun FavoritesScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    val gridFocusRequester = remember { FocusRequester() }
    Column(Modifier.fillMaxSize()) {
        if (isTv) {
            TvFavoriteFilters(
                state.favoriteGroups,
                state.favoriteGroup,
                gridFocusRequester,
                model::loadFavorites,
            )
        } else {
            LazyRow(
                contentPadding = PaddingValues(12.dp),
                horizontalArrangement = Arrangement.spacedBy(8.dp),
            ) {
                items(state.favoriteGroups, key = { it.id ?: it.name }) { group ->
                    MovoChoiceChip(
                        selected = state.favoriteGroup == group.id,
                        onClick = { model.loadFavorites(group.id) },
                        label = { Text("${group.name} (${group.count})") },
                        isTv = false,
                    )
                }
            }
        }
        key(state.favoriteGroup) {
            MediaGrid(
                state.items,
                isTv,
                state.loading,
                state.focusedUrl,
                { url -> model.openDetails(url, url) },
                emptyTitle = stringResource(R.string.favorites_empty),
                emptyHint = stringResource(R.string.favorites_empty_hint),
                entryFocusRequester = if (isTv) gridFocusRequester else null,
            ) { model.loadFavorites(append = true) }
        }
    }
}

@Composable
internal fun TvFavoriteFilters(
    groups: List<FavoriteGroup>,
    selectedGroup: Long?,
    gridFocusRequester: FocusRequester,
    select: (Long?) -> Unit,
) {
    LazyRow(
        contentPadding = PaddingValues(horizontal = 48.dp, vertical = 12.dp),
        horizontalArrangement = Arrangement.spacedBy(8.dp),
        modifier = Modifier
            .focusProperties { down = gridFocusRequester }
            .focusRestorer()
            .focusGroup(),
    ) {
        items(groups, key = { it.id ?: it.name }) { group ->
            TvFilterChip(
                selected = selectedGroup == group.id,
                onClick = { select(group.id) },
                modifier = Modifier
                    .tvFocusMemory("favorites:group:${group.id}")
                    .testTag("tv-favorite-group-${group.id ?: group.name}"),
                scale = SelectableChipScale.None,
            ) {
                TvText("${group.name} (${group.count})")
            }
        }
    }
}

@Composable
private fun MediaGrid(
    items: List<MediaItem>,
    isTv: Boolean,
    loading: Boolean,
    focusedUrl: String?,
    open: (String) -> Unit,
    emptyTitle: String,
    emptyHint: String? = null,
    entryFocusRequester: FocusRequester? = null,
    loadMore: (() -> Unit)? = null,
) {
    if (items.isEmpty()) {
        if (loading) MediaGridSkeleton(isTv)
        else Empty(emptyTitle, hint = emptyHint)
        return
    }
    val initialIndex = remember(items, focusedUrl) { items.indexOfFirst { it.url == focusedUrl }.coerceAtLeast(0) }
    val gridState = rememberLazyGridState(initialFirstVisibleItemIndex = initialIndex)
    var requestedItemCount by remember(items.first().url) { mutableStateOf(-1) }
    // `loading` is read through rememberUpdatedState so the collector keeps the live value
    // without being torn down and re-armed every time it flips (which previously missed
    // trigger frames and stalled autoload during fast scrolls).
    val isLoading by rememberUpdatedState(loading)
    LaunchedEffect(gridState, items.size, loadMore) {
        val request = loadMore ?: return@LaunchedEffect
        snapshotFlow { gridState.layoutInfo.visibleItemsInfo.lastOrNull()?.index ?: -1 }
            .distinctUntilChanged()
            .collect { lastVisibleIndex ->
                if (shouldLoadMore(lastVisibleIndex, items.size, isLoading, requestedItemCount)) {
                    requestedItemCount = items.size
                    request()
                }
            }
    }
    LazyVerticalGrid(
        state = gridState,
        columns = if (isTv) GridCells.Fixed(7) else GridCells.Adaptive(140.dp),
        contentPadding = if (isTv) {
            PaddingValues(horizontal = 48.dp, vertical = 27.dp)
        } else {
            PaddingValues(12.dp)
        },
        horizontalArrangement = Arrangement.spacedBy(if (isTv) 16.dp else 12.dp),
        verticalArrangement = Arrangement.spacedBy(if (isTv) 20.dp else 16.dp),
        modifier = if (isTv) {
            Modifier
                .then(entryFocusRequester?.let { Modifier.focusRequester(it) } ?: Modifier)
                .focusRestorer(entryFocusRequester ?: FocusRequester.Default)
                .focusGroup()
        } else {
            Modifier.focusGroup()
        },
    ) {
        items(items, key = { it.url }) { item ->
            MediaCard(item, isTv) { open(item.url) }
        }
    }
}

internal fun shouldLoadMore(lastVisibleIndex: Int, itemCount: Int, loading: Boolean, requestedItemCount: Int) =
    itemCount > 0 && !loading && requestedItemCount != itemCount && lastVisibleIndex >= itemCount - 12

@Composable
private fun MediaCard(
    item: MediaItem,
    isTv: Boolean,
    focusKey: String = item.url,
    open: () -> Unit,
) {
    val shape = RoundedCornerShape(14.dp)
    val modifier = Modifier
        .fillMaxWidth()
        .then(if (isTv) Modifier.tvFocusMemory(focusKey) else Modifier)
    if (isTv) {
        TvCard(onClick = open, modifier = modifier, scale = CardScale.None) {
            MediaCardContent(item, true, shape)
        }
    } else {
        Card(onClick = open, modifier = modifier, shape = shape) {
            MediaCardContent(item, false, shape)
        }
    }
}

@Composable
private fun MediaCardContent(item: MediaItem, isTv: Boolean, shape: RoundedCornerShape) {
        AsyncImage(
            model = item.posterUrl,
            contentDescription = null,
            modifier = Modifier
                .fillMaxWidth()
                .aspectRatio(2f / 3f)
                .clip(shape)
                .background(MaterialTheme.colorScheme.surfaceVariant),
            contentScale = ContentScale.Crop,
        )
        Column(Modifier.padding(if (isTv) 12.dp else 10.dp)) {
            Text(
                item.title,
                maxLines = 2,
                overflow = TextOverflow.Ellipsis,
                style = if (isTv) MaterialTheme.typography.titleSmall else MaterialTheme.typography.bodyLarge,
            )
            val metadata = listOfNotNull(item.year?.toString(), item.category).joinToString(" • ")
            if (metadata.isNotEmpty()) {
                Text(
                    metadata,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
        }
}

@Composable
private fun HistoryScreen(state: AppState, isTv: Boolean, model: MovoViewModel) {
    if (state.history.isEmpty()) {
        if (state.loading) Loading(stringResource(R.string.loading_content))
        else Empty(stringResource(R.string.history_empty))
        return
    }
    val initialIndex = remember(state.history, state.focusedUrl) {
        state.history.indexOfFirst { state.focusedUrl?.startsWith(it.id) == true }.coerceAtLeast(0)
    }
    val listState = rememberLazyListState(initialFirstVisibleItemIndex = initialIndex)
    Box(Modifier.fillMaxSize()) {
        LazyColumn(
            state = listState,
            contentPadding = if (isTv) PaddingValues(horizontal = 48.dp, vertical = 27.dp) else PaddingValues(12.dp),
            verticalArrangement = Arrangement.spacedBy(if (isTv) 16.dp else 8.dp),
            modifier = Modifier
                .widthIn(max = 1100.dp)
                .fillMaxSize()
                .align(Alignment.TopCenter)
                .then(if (isTv) Modifier.focusRestorer().focusGroup() else Modifier.focusGroup()),
        ) {
            items(state.history, key = { it.id }) { entry ->
                HistoryCard(entry, isTv, model)
            }
        }
    }
}

@Composable
private fun HistoryCard(entry: HistoryEntry, isTv: Boolean, model: MovoViewModel) {
    if (isTv) {
        TvHistoryCard(
            entry,
            open = { model.openDetails(entry.url, entry.id) },
            toggle = { model.toggleHistory(entry) },
            remove = { model.removeHistory(entry) },
        )
        return
    }
    var focused by remember { mutableStateOf(false) }
    var hasFocus by remember { mutableStateOf(false) }
    var confirmRemoval by remember { mutableStateOf(false) }
    val shape = RoundedCornerShape(16.dp)
    ElevatedCard(
        onClick = { model.openDetails(entry.url, entry.id) },
        modifier = Modifier
            .fillMaxWidth()
            .onFocusChanged {
                focused = it.isFocused
                hasFocus = it.hasFocus
            }
            .then(
                if (focused && isTv) Modifier.border(3.dp, MaterialTheme.colorScheme.primary, shape)
                else Modifier,
            )
            .alpha(if (entry.watched && !(hasFocus && isTv)) .72f else 1f),
        shape = shape,
    ) {
        ListItem(
            headlineContent = { Text(entry.title.ifEmpty { stringResource(R.string.unknown) }) },
            supportingContent = {
                Text(listOfNotNull(entry.info, entry.additionalInfo, entry.date).joinToString(" • "))
            },
            leadingContent = {
                AsyncImage(
                    model = entry.posterUrl,
                    contentDescription = null,
                    modifier = Modifier
                        .size(if (isTv) 88.dp else 64.dp, if (isTv) 132.dp else 92.dp)
                        .clip(RoundedCornerShape(10.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant),
                    contentScale = ContentScale.Crop,
                )
            },
            trailingContent = {
                val watched = entry.watched
                Row {
                    IconButton({ model.toggleHistory(entry) }, Modifier.tvFocusScale(isTv)) {
                        Icon(
                            if (watched) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                            contentDescription = stringResource(
                                if (watched) R.string.mark_unwatched else R.string.mark_watched,
                            ),
                        )
                    }
                    IconButton({ confirmRemoval = true }, Modifier.tvFocusScale(isTv)) {
                        Icon(Icons.Default.Delete, contentDescription = stringResource(R.string.remove_history))
                    }
                }
            },
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
        )
    }
    if (confirmRemoval) {
        AlertDialog(
            onDismissRequest = { confirmRemoval = false },
            title = { Text(stringResource(R.string.remove_history_title)) },
            text = { Text(stringResource(R.string.remove_history_message, entry.title)) },
            confirmButton = {
                TextButton({ confirmRemoval = false; model.removeHistory(entry) }, Modifier.tvFocusScale(isTv)) {
                    Text(stringResource(R.string.remove))
                }
            },
            dismissButton = {
                TextButton({ confirmRemoval = false }, Modifier.tvFocusScale(isTv)) { Text(stringResource(R.string.cancel)) }
            },
        )
    }
}

@Composable
internal fun TvHistoryCard(
    entry: HistoryEntry,
    open: () -> Unit,
    toggle: () -> Unit,
    remove: () -> Unit,
) {
    var confirmRemoval by remember { mutableStateOf(false) }
    val cancelFocusRequester = remember { FocusRequester() }

    Row(
        Modifier.fillMaxWidth().focusRestorer().focusGroup(),
        horizontalArrangement = Arrangement.spacedBy(12.dp),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        TvListItem(
            selected = entry.watched,
            onClick = open,
            headlineContent = { TvText(entry.title.ifEmpty { stringResource(R.string.unknown) }) },
            supportingContent = {
                TvText(listOfNotNull(entry.info, entry.additionalInfo, entry.date).joinToString(" • "))
            },
            leadingContent = {
                AsyncImage(
                    model = entry.posterUrl,
                    contentDescription = null,
                    modifier = Modifier
                        .size(88.dp, 132.dp)
                        .clip(RoundedCornerShape(10.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant),
                    contentScale = ContentScale.Crop,
                )
            },
            modifier = Modifier
                .weight(1f)
                .tvFocusMemory(entry.id)
                .testTag("tv-history-primary"),
            scale = ListItemScale.None,
        )
        TvIconButton(
            onClick = toggle,
            modifier = Modifier.tvFocusMemory("${entry.id}:toggle").testTag("tv-history-toggle"),
        ) {
            TvIcon(
                if (entry.watched) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked,
                contentDescription = stringResource(
                    if (entry.watched) R.string.mark_unwatched else R.string.mark_watched,
                ),
            )
        }
        TvIconButton(
            onClick = { confirmRemoval = true },
            modifier = Modifier.tvFocusMemory("${entry.id}:delete").testTag("tv-history-delete"),
        ) {
            TvIcon(Icons.Default.Delete, contentDescription = stringResource(R.string.remove_history))
        }
    }

    if (confirmRemoval) {
        LaunchedEffect(Unit) { cancelFocusRequester.requestFocus() }
        AlertDialog(
            onDismissRequest = { confirmRemoval = false },
            title = { Text(stringResource(R.string.remove_history_title)) },
            text = { Text(stringResource(R.string.remove_history_message, entry.title)) },
            confirmButton = {
                TvButton(onClick = { confirmRemoval = false; remove() }) {
                    TvText(stringResource(R.string.remove))
                }
            },
            dismissButton = {
                TvButton(
                    onClick = { confirmRemoval = false },
                    modifier = Modifier
                        .focusRequester(cancelFocusRequester)
                        .testTag("tv-history-cancel"),
                ) {
                    TvText(stringResource(R.string.cancel))
                }
            },
        )
    }
}

@Composable
private fun DetailsScreen(
    state: AppState,
    isTv: Boolean,
    wideContent: Boolean,
    settings: AppSettings,
    model: MovoViewModel,
) {
    val details = state.details ?: return
    val shareLabel = stringResource(R.string.share)
    val scope = rememberCoroutineScope()
    val translators = remember(details.translators, details.voiceRatings, settings.sortVoices) {
        if (!settings.sortVoices) details.translators else details.translators.sortedByDescending { voice ->
            details.voiceRatings.firstOrNull { it.title == voice.name }?.rating ?: 0f
        }
    }
    val playbackDetails = remember(details, translators) {
        if (translators === details.translators) details else details.copy(translators = translators)
    }
    val series = details.mediaType == "TVSeries" || details.seasons.isNotEmpty()
    var translator by remember(details.url) {
        mutableStateOf(preferredTranslator(translators, state.resumeTranslatorId))
    }
    var selectedSeasonId by remember(details.url) {
        mutableStateOf(state.resumeSeasonId ?: details.seasons.firstOrNull()?.id)
    }
    LaunchedEffect(details.seasons, state.episodesTranslatorId, translator?.id) {
        if (state.episodesTranslatorId == translator?.id &&
            (selectedSeasonId == null || details.seasons.none { it.id == selectedSeasonId })
        ) {
            selectedSeasonId = state.resumeSeasonId
                ?.takeIf { translator?.id == state.resumeTranslatorId }
                ?: details.seasons.firstOrNull()?.id
        }
    }
    var descExpanded by remember(details.url) { mutableStateOf(false) }
    var showFavorites by remember(details.url) { mutableStateOf(false) }
    var showPlayback by remember(details.url) { mutableStateOf(false) }
    var automaticPlayback by remember(details.url) { mutableStateOf(false) }
    var showRating by remember(details.url) { mutableStateOf(false) }
    val context = LocalContext.current
    LaunchedEffect(state.detailAction) {
        when (state.detailAction) {
            DetailAction.Favorites -> showFavorites = true
            DetailAction.Comments -> model.loadComments()
            DetailAction.Rating -> showRating = true
            null -> return@LaunchedEffect
        }
        model.consumeDetailAction()
    }
    var selectedQuality by remember(state.preparedStream, settings.qualityMode, settings.lastQuality) {
        mutableStateOf(state.preparedStream?.let { selectStream(it, settings.qualityMode, settings.lastQuality.takeIf { _ -> settings.saveQuality }) })
    }
    val selectedSeason = details.seasons.firstOrNull { it.id == selectedSeasonId }
        ?: details.seasons.firstOrNull()
    val previewEpisode = selectedSeason?.episodes?.firstOrNull {
        it.id == state.resumeEpisodeId &&
            translator?.id == state.resumeTranslatorId &&
            selectedSeason.id == state.resumeSeasonId
    } ?: selectedSeason?.episodes?.firstOrNull()
    LaunchedEffect(showPlayback) {
        if (showPlayback && state.episodesTranslatorId != translator?.id) {
            translator?.let(model::loadEpisodes)
        }
    }
    LaunchedEffect(
        showPlayback,
        translator?.id,
        state.episodesTranslatorId,
        selectedSeason?.id,
        previewEpisode?.id,
    ) {
        val voice = translator ?: return@LaunchedEffect
        if (!showPlayback) return@LaunchedEffect
        if (!series) {
            model.preparePlayback(voice)
        } else if (state.episodesTranslatorId == voice.id && selectedSeason != null && previewEpisode != null) {
            model.preparePlayback(voice, selectedSeason.id, previewEpisode.id)
        }
    }
    LaunchedEffect(automaticPlayback, state.preparedStream, selectedSeason?.id, previewEpisode?.id) {
        if (!automaticPlayback) return@LaunchedEffect
        val voice = translator ?: return@LaunchedEffect
        val prepared = state.preparedStream ?: return@LaunchedEffect
        val quality = selectStream(prepared, settings.qualityMode, settings.lastQuality.takeIf { settings.saveQuality }) ?: return@LaunchedEffect
        val season = if (series) selectedSeason?.id ?: return@LaunchedEffect else null
        val episode = if (series) previewEpisode?.id ?: return@LaunchedEffect else null
        automaticPlayback = false
        showPlayback = false
        if (settings.saveQuality) scope.launch { context.saveLastQuality(quality.quality) }
        model.startPlayback(voice, quality.quality, settings.qualityMode, season, episode)
    }
    BackHandler { model.closeDetails() }
    Scaffold(
        topBar = {
            CenterAlignedTopAppBar(
                title = { Text(details.title, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                navigationIcon = {
                    IconButton(model::closeDetails, Modifier.tvFocusScale(isTv)) {
                        Icon(
                            imageVector = Icons.AutoMirrored.Filled.ArrowBack,
                            contentDescription = stringResource(R.string.back),
                        )
                    }
                },
            )
        },
    ) { padding ->
        Box(Modifier.padding(padding).fillMaxSize()) {
            key(details.url) {
                LazyColumn(
                    Modifier
                        .widthIn(max = 1200.dp)
                        .fillMaxSize()
                        .align(Alignment.TopCenter)
                        .focusGroup(),
                    contentPadding = PaddingValues(if (isTv) 32.dp else if (wideContent) 24.dp else 16.dp),
                    verticalArrangement = Arrangement.spacedBy(16.dp),
                ) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(18.dp)) {
                    Box {
                        AsyncImage(
                            model = details.posterHqUrl ?: details.posterUrl,
                            contentDescription = null,
                            modifier = Modifier
                                .width(if (isTv) 260.dp else if (wideContent) 200.dp else 120.dp)
                                .aspectRatio(2f / 3f)
                                .clip(RoundedCornerShape(if (isTv) 16.dp else 12.dp))
                                .background(MaterialTheme.colorScheme.surfaceVariant),
                            contentScale = ContentScale.Crop,
                        )
                        if (details.trailerAvailable) {
                            // Small affordance: trailer is also reachable from the action row,
                            // the badge just makes it discoverable on the poster.
                            FilledTonalIconButton(
                                onClick = model::loadTrailer,
                                modifier = Modifier
                                    .align(Alignment.BottomEnd)
                                    .padding(6.dp)
                                    .size(32.dp)
                                    .tvFocusScale(isTv),
                            ) {
                                Icon(
                                    Icons.Default.Movie,
                                    contentDescription = stringResource(R.string.trailer),
                                    modifier = Modifier.size(18.dp),
                                )
                            }
                        }
                    }
                    Column(
                        Modifier.weight(1f),
                        verticalArrangement = Arrangement.spacedBy(8.dp),
                    ) {
                        Text(
                            details.title,
                            style = if (wideContent || isTv) {
                                MaterialTheme.typography.headlineMedium
                            } else {
                                MaterialTheme.typography.titleLarge
                            },
                            maxLines = 3,
                            overflow = TextOverflow.Ellipsis,
                        )
                        details.originalTitle?.let {
                            Text(
                                it,
                                maxLines = 2,
                                overflow = TextOverflow.Ellipsis,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        val metadata = listOfNotNull(
                            details.year?.toString(),
                            details.duration,
                            details.genres.joinToString().ifBlank { null },
                        ).joinToString(" • ")
                        if (metadata.isNotEmpty()) {
                            Text(
                                metadata,
                                style = MaterialTheme.typography.bodyMedium,
                                color = MaterialTheme.colorScheme.onSurfaceVariant,
                            )
                        }
                        RatingRow(details)
                        FlowRow(
                            horizontalArrangement = Arrangement.spacedBy(8.dp),
                            verticalArrangement = Arrangement.spacedBy(8.dp),
                        ) {
                            FilledTonalIconButton(onClick = { showFavorites = true }, modifier = Modifier.tvFocusScale(isTv)) {
                                Icon(
                                    if (details.favoriteCategoryIds.isEmpty()) Icons.Default.FavoriteBorder else Icons.Default.Favorite,
                                    contentDescription = stringResource(R.string.favorite),
                                )
                            }
                            Button(
                                onClick = { automaticPlayback = !settings.askQuality; showPlayback = true },
                                enabled = translators.isNotEmpty() && !state.loading,
                                modifier = Modifier.tvFocusScale(isTv),
                            ) {
                                if (state.loading) {
                                    CircularProgressIndicator(
                                        Modifier.size(18.dp),
                                        strokeWidth = 2.dp,
                                        color = LocalContentColor.current,
                                    )
                                } else {
                                    Icon(Icons.Default.PlayArrow, contentDescription = null)
                                }
                                Text(stringResource(R.string.play))
                            }
                            if (details.trailerAvailable && (wideContent || isTv)) {
                                FilledTonalIconButton(model::loadTrailer, Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Movie, stringResource(R.string.trailer)) }
                            }
                            FilledTonalIconButton(onClick = { showRating = true }, modifier = Modifier.tvFocusScale(isTv), enabled = !details.ratingPosted) {
                                Icon(Icons.Default.StarRate, stringResource(R.string.rate_title))
                            }
                            FilledTonalIconButton(onClick = { model.loadComments() }, modifier = Modifier.tvFocusScale(isTv)) {
                                Icon(Icons.AutoMirrored.Filled.Comment, stringResource(R.string.comments))
                            }
                            FilledTonalIconButton(onClick = {
                                context.startActivity(Intent.createChooser(Intent(Intent.ACTION_SEND).apply {
                                    type = "text/plain"
                                    putExtra(Intent.EXTRA_TEXT, details.url)
                                }, shareLabel))
                            }, modifier = Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Share, stringResource(R.string.share)) }
                        }
                    }
                }
            }
            item {
                Text(
                    text = details.description,
                    maxLines = if (descExpanded) Int.MAX_VALUE else 8,
                    overflow = TextOverflow.Ellipsis,
                )
                if (details.description.length > 400) {
                    TextButton(onClick = { descExpanded = !descExpanded }, modifier = Modifier.tvFocusScale(isTv)) {
                        Text(if (descExpanded) stringResource(R.string.collapse) else stringResource(R.string.more))
                    }
                }
            }
            if (details.genreLinks.isNotEmpty() || details.countryLinks.isNotEmpty()) {
                item {
                    LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        items(details.genreLinks + details.countryLinks, key = { "${it.url}:${it.name}" }) { link ->
                            AssistChip(onClick = { model.openDiscoveryPath(Tab.Search, link.name, link.url) }, label = { Text(link.name) }, modifier = Modifier.tvFocusScale(isTv))
                        }
                    }
                }
            }
            if (details.franchises.size > 1) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(
                            stringResource(R.string.franchise),
                            style = if (isTv) {
                                MaterialTheme.typography.titleLarge
                            } else {
                                MaterialTheme.typography.titleMedium
                            },
                        )
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            items(
                                details.franchises,
                                key = { "${it.url}:${it.title}" },
                            ) { part ->
                                MovoChoiceChip(
                                    selected = part.current,
                                    onClick = {
                                        if (!part.current && part.url.isNotBlank()) {
                                            model.openDetails(part.url)
                                        }
                                    },
                                    enabled = part.current || part.url.isNotBlank(),
                                    label = {
                                        Text(
                                            part.title,
                                            maxLines = 1,
                                            overflow = TextOverflow.Ellipsis,
                                            style = if (isTv) {
                                                MaterialTheme.typography.titleSmall
                                            } else {
                                                MaterialTheme.typography.labelLarge
                                            },
                                        )
                                    },
                                    trailingIcon = if (!part.current) {
                                        { Icon(Icons.Default.ChevronRight, contentDescription = null) }
                                    } else null,
                                    modifier = Modifier
                                        .widthIn(min = if (isTv) 200.dp else 150.dp, max = 300.dp)
                                        .height(if (isTv) 52.dp else 40.dp),
                                    isTv = isTv,
                                )
                            }
                        }
                    }
                }
            }
            if (details.actorDetails.isNotEmpty() || details.directorDetails.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.people), style = MaterialTheme.typography.titleLarge)
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            items(details.directorDetails + details.actorDetails, key = { "${it.url}:${it.name}" }) { person ->
                                AssistChip(
                                    onClick = { if (person.url.isNotBlank()) model.openActor(person.url) },
                                    label = { Text(person.name) },
                                    modifier = Modifier.tvFocusScale(isTv),
                                    leadingIcon = { Icon(Icons.Default.Person, null) },
                                    enabled = person.url.isNotBlank(),
                                )
                            }
                        }
                    }
                }
            }
            if (details.schedules.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.schedule), style = MaterialTheme.typography.titleLarge)
                        details.schedules.forEach { group ->
                            if (group.name.isNotBlank()) Text(group.name, style = MaterialTheme.typography.titleMedium)
                            group.items.forEach { episode ->
                                ListItem(
                                    headlineContent = { Text("${episode.episode}  ${episode.title}") },
                                    supportingContent = { Text(listOfNotNull(episode.originalTitle, episode.date).joinToString(" • ")) },
                                    trailingContent = {
                                        IconButton({ model.toggleSchedule(episode) }, Modifier.tvFocusScale(isTv), enabled = episode.id.isNotBlank()) {
                                            Icon(if (episode.watched) Icons.Default.CheckCircle else Icons.Default.RadioButtonUnchecked, stringResource(if (episode.watched) R.string.mark_unwatched else R.string.mark_watched))
                                        }
                                    },
                                    colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                                )
                            }
                        }
                    }
                }
            }
            if (details.includedIn.isNotEmpty() || details.fromCollections.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.from_collections), style = MaterialTheme.typography.titleLarge)
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                            items(details.includedIn + details.fromCollections, key = { "${it.url}:${it.name}" }) { link ->
                                AssistChip(onClick = { model.openDiscoveryPath(Tab.Collections, link.name, link.url) }, label = { Text(link.name) }, modifier = Modifier.tvFocusScale(isTv))
                            }
                        }
                    }
                }
            }
            if (details.related.isNotEmpty()) {
                item {
                    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
                        Text(stringResource(R.string.related), style = MaterialTheme.typography.titleLarge)
                        LazyRow(horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                            items(details.related, key = { it.url }) { item ->
                                Box(Modifier.width(if (isTv) 180.dp else 150.dp)) { MediaCard(item, isTv) { model.openDetails(item.url) } }
                            }
                        }
                    }
                }
            }
            state.error?.let { item { ErrorBanner(it, model::clearError) } }
                }
            }
        }
    }

    if (showFavorites) {
        FavoriteFoldersSheet(
            isTv = isTv,
            groups = state.favoriteGroups,
            selectedIds = details.favoriteCategoryIds,
            dismiss = { showFavorites = false },
            toggle = model::toggleFavorite,
        )
    }
    if (showPlayback) {
        PlaybackSheet(
            isTv = isTv,
            details = playbackDetails,
            series = series,
            translator = translator,
            selectedSeasonId = selectedSeasonId,
            preparedStream = state.preparedStream,
            selectedQuality = selectedQuality,
            loading = state.loading,
            episodesLoading = series && state.episodesTranslatorId != translator?.id,
            error = state.error,
            dismiss = {
                automaticPlayback = false
                showPlayback = false
                model.cancelPlaybackPreparation()
            },
            chooseTranslator = {
                model.cancelPlaybackPreparation()
                translator = it
                selectedSeasonId = null
                if (series) model.loadEpisodes(it)
            },
            chooseSeason = {
                model.cancelPlaybackPreparation()
                selectedSeasonId = it
            },
            chooseQuality = { selectedQuality = it },
            play = { season, episode ->
                automaticPlayback = false
                val voice = translator
                val quality = selectedQuality
                if (voice != null && quality != null) {
                    if (settings.saveQuality) scope.launch { context.saveLastQuality(quality.quality) }
                    model.startPlayback(voice, quality.quality, settings.qualityMode, season, episode)
                }
            },
        )
    }
    if (showRating) {
        AlertDialog(
            onDismissRequest = { showRating = false },
            title = { Text(stringResource(R.string.rate_title)) },
            text = {
                LazyRow(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                    items((1..10).toList()) { rating ->
                        FilledTonalButton(
                            onClick = { showRating = false; model.rate(rating) },
                            modifier = Modifier.tvFocusScale(isTv),
                            contentPadding = PaddingValues(horizontal = 12.dp),
                        ) { Text(rating.toString()) }
                    }
                }
            },
            confirmButton = {},
            dismissButton = { TextButton({ showRating = false }, Modifier.tvFocusScale(isTv)) { Text(stringResource(R.string.cancel)) } },
        )
    }
    state.actor?.let { ActorDialog(it, isTv, model) }
    state.comments?.let { CommentsDialog(it, isTv, model) }
}

@Composable
private fun ActorDialog(actor: ActorDetails, isTv: Boolean, model: MovoViewModel) {
    AdaptiveModal(isTv, model::closeActor) {
        LazyColumn(Modifier.fillMaxHeight(.9f), contentPadding = PaddingValues(24.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            item {
                Row(horizontalArrangement = Arrangement.spacedBy(16.dp)) {
                    AsyncImage(actor.photoUrl, null, Modifier.width(150.dp).aspectRatio(2f / 3f), contentScale = ContentScale.Crop)
                    Column {
                        Text(actor.name, style = MaterialTheme.typography.headlineSmall)
                        actor.originalName?.let { Text(it) }
                        actor.birthDate?.let { Text(stringResource(R.string.born, it)) }
                        actor.birthPlace?.let { Text(it) }
                        actor.height?.let { Text(it) }
                        if (actor.careers.isNotEmpty()) Text(actor.careers.joinToString())
                    }
                }
            }
            if (actor.roles.isEmpty()) {
                items(actor.films, key = { it.url }) { film -> ActorFilmRow(film, isTv, model) }
            } else actor.roles.forEach { role ->
                item("role:${role.name}") { Column { Text(role.name, style = MaterialTheme.typography.titleLarge); if (role.info.isNotBlank()) Text(role.info) } }
                items(role.films, key = { "${role.name}:${it.url}" }) { film -> ActorFilmRow(film, isTv, model) }
            }
        }
    }
}

@Composable
private fun ActorFilmRow(film: MediaItem, isTv: Boolean, model: MovoViewModel) {
    ListItem(
        headlineContent = { Text(film.title) },
        supportingContent = { Text(listOfNotNull(film.year?.toString(), film.info).joinToString(" • ")) },
        modifier = Modifier.tvFocusScale(isTv, 1.03f).clickable { model.closeActor(); model.openDetails(film.url) },
    )
}

@Composable
private fun CommentsDialog(page: CommentsPage, isTv: Boolean, model: MovoViewModel) {
    var revealedSpoilers by remember(page.page) { mutableStateOf(emptySet<String>()) }
    AdaptiveModal(isTv, model::closeComments) {
        LazyColumn(Modifier.fillMaxHeight(.9f), contentPadding = PaddingValues(24.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            item { Text(stringResource(R.string.comments), style = MaterialTheme.typography.headlineSmall) }
            items(page.items, key = { it.id }) { comment ->
                ListItem(
                    headlineContent = { Text(comment.username) },
                    supportingContent = { Column {
                        if (comment.hasSpoiler && comment.id !in revealedSpoilers) {
                            TextButton({ revealedSpoilers = revealedSpoilers + comment.id }, Modifier.tvFocusScale(isTv)) { Icon(Icons.Default.Visibility, null); Text(stringResource(R.string.show_spoiler)) }
                        } else Text(comment.text)
                        Text(comment.date, style = MaterialTheme.typography.labelSmall)
                    } },
                    leadingContent = { AsyncImage(comment.avatarUrl, null, Modifier.size(48.dp).clip(RoundedCornerShape(24.dp)), contentScale = ContentScale.Crop) },
                    trailingContent = { TextButton({ model.likeComment(comment.id) }, Modifier.tvFocusScale(isTv), colors = ButtonDefaults.textButtonColors(contentColor = if (comment.liked) MaterialTheme.colorScheme.primary else LocalContentColor.current)) { Icon(Icons.Default.ThumbUp, null); Text(comment.likes.toString()) } },
                    modifier = Modifier.padding(start = (comment.indent.coerceAtMost(4) * 12).dp),
                )
            }
            if (page.totalPages > 1) {
                item {
                    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                        OutlinedButton({ model.loadComments(page.page - 1) }, Modifier.tvFocusScale(isTv), enabled = page.page > 1) { Text(stringResource(R.string.previous)) }
                        OutlinedButton({ model.loadComments(page.page + 1) }, Modifier.tvFocusScale(isTv), enabled = page.page < page.totalPages) { Text(stringResource(R.string.next)) }
                    }
                }
            }
        }
    }
}

internal fun preferredTranslator(translators: List<Translator>, translatorId: Long?) =
    translators.firstOrNull { it.id == translatorId } ?: translators.firstOrNull()

@Composable
private fun RatingRow(details: MediaDetails) {
    val ratings = (listOfNotNull(
        details.ratingRezka?.let { stringResource(R.string.rating_rezka, it) },
        details.ratingImdb?.let { stringResource(R.string.rating_imdb, it) },
        details.ratingKp?.let { stringResource(R.string.rating_kp, it) },
    ) + details.ratings.map { rating ->
        buildString {
            append(rating.name)
            append(' ')
            append(rating.value)
            rating.votes?.let { append(" ($it)") }
        }
    }).distinct()
    if (ratings.isNotEmpty()) Text(ratings.joinToString(" • "), style = MaterialTheme.typography.labelLarge)
}

@Composable
private fun FavoriteFoldersSheet(
    isTv: Boolean,
    groups: List<FavoriteGroup>,
    selectedIds: List<Long>,
    dismiss: () -> Unit,
    toggle: (Long, Boolean) -> Unit,
) {
    AdaptiveModal(isTv, dismiss) {
        LazyColumn {
            item {
                Text(
                    stringResource(R.string.favorite_folders),
                    style = MaterialTheme.typography.headlineSmall,
                    modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
                )
            }
            items(groups, key = { it.id ?: it.name }) { group ->
                group.id?.let { id ->
                    val selected = id in selectedIds
                    ListItem(
                        headlineContent = { Text(group.name) },
                        trailingContent = { Checkbox(selected, null) },
                        modifier = Modifier
                            .tvFocusScale(isTv, 1.02f)
                            .toggleable(
                                value = selected,
                                role = Role.Checkbox,
                                onValueChange = { toggle(id, it) },
                            ),
                    )
                }
            }
            item { Spacer(Modifier.height(24.dp)) }
        }
    }
}

@Composable
private fun PlaybackSheet(
    isTv: Boolean,
    details: MediaDetails,
    series: Boolean,
    translator: Translator?,
    selectedSeasonId: Long?,
    preparedStream: StreamBundle?,
    selectedQuality: StreamEntry?,
    loading: Boolean,
    episodesLoading: Boolean,
    error: String?,
    dismiss: () -> Unit,
    chooseTranslator: (Translator) -> Unit,
    chooseSeason: (Long) -> Unit,
    chooseQuality: (StreamEntry) -> Unit,
    play: (Long?, Long?) -> Unit,
) {
    val season = details.seasons.firstOrNull { it.id == selectedSeasonId } ?: details.seasons.firstOrNull()
    val qualities = preparedStream?.streams?.filter { it.urls.isNotEmpty() }.orEmpty()
    var startingEpisodeId by remember(translator?.id, season?.id) { mutableStateOf<Long?>(null) }
    LaunchedEffect(loading) {
        if (!loading) startingEpisodeId = null
    }
    AdaptiveModal(isTv, dismiss) {
        Column(
            modifier = Modifier.fillMaxHeight(0.85f),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Text(
                stringResource(R.string.choose_playback),
                style = MaterialTheme.typography.headlineSmall,
                modifier = Modifier.padding(horizontal = 24.dp),
            )
            error?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.padding(horizontal = 24.dp),
                )
            }
            ChoiceRow(
                title = stringResource(R.string.translation),
                values = details.translators,
                selected = translator,
                label = { it.name },
                itemKey = { it.id },
                choose = chooseTranslator,
                modifier = Modifier.padding(horizontal = 24.dp),
                isTv = isTv,
            )
            if (qualities.isNotEmpty()) {
                ChoiceRow(
                    title = stringResource(R.string.quality),
                    values = qualities,
                    selected = selectedQuality,
                    label = { it.quality },
                    itemKey = { it.quality },
                    choose = chooseQuality,
                    modifier = Modifier.padding(horizontal = 24.dp),
                    isTv = isTv,
                )
            } else {
                Column(Modifier.padding(horizontal = 24.dp)) {
                    Text(stringResource(R.string.quality), style = MaterialTheme.typography.titleMedium)
                    if (loading) {
                        CircularProgressIndicator(Modifier.size(32.dp), strokeWidth = 3.dp)
                    } else {
                        Text(stringResource(R.string.no_streams))
                    }
                }
            }
            if (!series) {
                Button(
                    onClick = { play(null, null) },
                    enabled = translator != null && selectedQuality != null && !loading,
                    modifier = Modifier.padding(horizontal = 24.dp).tvFocusScale(isTv),
                ) {
                    if (loading) {
                        CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                    } else {
                        Icon(Icons.Default.PlayArrow, contentDescription = null)
                    }
                    Text(stringResource(R.string.start_watching))
                }
            } else if (episodesLoading) {
                Box(Modifier.fillMaxWidth().weight(1f), contentAlignment = Alignment.Center) {
                    CircularProgressIndicator()
                }
            } else if (season != null) {
                ChoiceRow(
                    title = stringResource(R.string.season),
                    values = details.seasons,
                    selected = season,
                    label = { it.title },
                    itemKey = { it.id },
                    choose = { chooseSeason(it.id) },
                    modifier = Modifier.padding(horizontal = 24.dp),
                    isTv = isTv,
                )
                HorizontalDivider()
                Text(
                    stringResource(R.string.episode),
                    style = MaterialTheme.typography.titleMedium,
                    modifier = Modifier.padding(horizontal = 24.dp),
                )
                key(season.id) {
                    LazyColumn(
                        modifier = Modifier.weight(1f),
                        contentPadding = PaddingValues(bottom = 24.dp),
                    ) {
                        items(season.episodes, key = { "${season.id}:${it.id}" }) { episode ->
                            val contentAlpha = if (episode.watched) .6f else 1f
                            ListItem(
                                headlineContent = { Text(episode.title, modifier = Modifier.alpha(contentAlpha)) },
                                supportingContent = episode.watchId?.let { id -> { Text(id, Modifier.alpha(contentAlpha), maxLines = 1) } },
                                trailingContent = {
                                    FilledTonalIconButton(
                                        onClick = {
                                            startingEpisodeId = episode.id
                                            play(season.id, episode.id)
                                        },
                                        modifier = Modifier.tvFocusScale(isTv),
                                        enabled = selectedQuality != null && !loading,
                                    ) {
                                        if (loading && startingEpisodeId == episode.id) {
                                            CircularProgressIndicator(Modifier.size(18.dp), strokeWidth = 2.dp)
                                        } else {
                                            Icon(
                                                Icons.Default.PlayArrow,
                                                contentDescription = stringResource(
                                                    R.string.play_episode,
                                                    episode.title,
                                                ),
                                            )
                                        }
                                    }
                                },
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun AdaptiveModal(
    isTv: Boolean,
    dismiss: () -> Unit,
    content: @Composable () -> Unit,
) {
    if (isTv) {
        BasicAlertDialog(onDismissRequest = dismiss) {
            Surface(
                modifier = Modifier
                    .widthIn(max = 760.dp)
                    .fillMaxHeight(0.88f),
                shape = RoundedCornerShape(24.dp),
                tonalElevation = 6.dp,
            ) { content() }
        }
    } else {
        ModalBottomSheet(
            onDismissRequest = dismiss,
            sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true),
        ) { content() }
    }
}

@Composable
fun <T> ChoiceRow(
    title: String,
    values: List<T>,
    selected: T?,
    label: (T) -> String,
    itemKey: (T) -> Any,
    choose: (T) -> Unit,
    modifier: Modifier = Modifier,
    isTv: Boolean = false,
) {
    Column(modifier) {
        Text(title, style = MaterialTheme.typography.titleMedium)
        LazyRow(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            items(values, key = itemKey) { value ->
                MovoChoiceChip(
                    selected = value == selected,
                    onClick = { choose(value) },
                    label = { Text(label(value)) },
                    isTv = isTv,
                )
            }
        }
    }
}

@Composable
internal fun MovoChoiceChip(
    selected: Boolean,
    onClick: () -> Unit,
    label: @Composable () -> Unit,
    isTv: Boolean,
    modifier: Modifier = Modifier,
    enabled: Boolean = true,
    trailingIcon: (@Composable () -> Unit)? = null,
) {
    val interactionSource = remember { MutableInteractionSource() }
    val focused by interactionSource.collectIsFocusedAsState()
    val tvFocused = isTv && focused
    val scale by animateFloatAsState(if (tvFocused) 1.06f else 1f, label = "choice focus")
    val focusContainer = MaterialTheme.colorScheme.primary
    val focusContent = MaterialTheme.colorScheme.onPrimary
    FilterChip(
        selected = selected,
        onClick = onClick,
        label = label,
        modifier = modifier
            .zIndex(if (tvFocused) 1f else 0f)
            .graphicsLayer { scaleX = scale; scaleY = scale },
        enabled = enabled,
        leadingIcon = {
            Icon(
                Icons.Default.Check,
                contentDescription = null,
                modifier = Modifier.alpha(if (selected) 1f else 0f),
            )
        },
        trailingIcon = trailingIcon,
        colors = FilterChipDefaults.filterChipColors(
            containerColor = if (tvFocused) focusContainer else Color.Transparent,
            labelColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSurfaceVariant,
            iconColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSurfaceVariant,
            selectedContainerColor = if (tvFocused) focusContainer else MaterialTheme.colorScheme.secondaryContainer,
            selectedLabelColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSecondaryContainer,
            selectedLeadingIconColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSecondaryContainer,
            selectedTrailingIconColor = if (tvFocused) focusContent else MaterialTheme.colorScheme.onSecondaryContainer,
        ),
        interactionSource = interactionSource,
    )
}

/**
 * TV focus treatment: gentle scale-up while focused.
 *
 * Smoothness notes:
 * - Only [graphicsLayer] reads the animated value, so focus changes never trigger
 *   recomposition — the layer just re-draws.
 * - A symmetric pivot (center) plus snap (no spring overshoot) keeps neighboring
 *   items from being clipped mid-animation.
 */
@Composable
internal fun Modifier.tvFocusScale(isTv: Boolean, focusedScale: Float = 1.08f): Modifier {
    var focused by remember { mutableStateOf(false) }
    val scale by animateFloatAsState(
        targetValue = if (isTv && focused) focusedScale else 1f,
        animationSpec = tween(durationMillis = 150, easing = LinearOutSlowInEasing),
        label = "TV focus",
    )
    return this
        .onFocusChanged { focused = it.isFocused }
        .graphicsLayer {
            scaleX = scale
            scaleY = scale
            transformOrigin = TransformOrigin.Center
        }
        .zIndex(if (focused) 1f else 0f)
}

@Composable
private fun Loading(label: String) = Box(
    Modifier.fillMaxSize(),
    contentAlignment = Alignment.Center,
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        CircularProgressIndicator()
        Spacer(Modifier.height(12.dp))
        Text(label)
    }
}

@Composable
internal fun TvHomeSkeleton() {
    LazyColumn(
        modifier = Modifier.fillMaxSize().testTag("tv-home-placeholder-list"),
        contentPadding = PaddingValues(horizontal = 48.dp, vertical = 27.dp),
        verticalArrangement = Arrangement.spacedBy(20.dp),
    ) {
        items(5) { rail ->
            Column(
                Modifier.testTag("tv-home-placeholder-rail-$rail"),
                verticalArrangement = Arrangement.spacedBy(10.dp),
            ) {
                Box(
                    Modifier
                        .width(180.dp)
                        .height(24.dp)
                        .clip(RoundedCornerShape(8.dp))
                        .background(MaterialTheme.colorScheme.surfaceVariant),
                )
                LazyRow(
                    horizontalArrangement = Arrangement.spacedBy(16.dp),
                    userScrollEnabled = false,
                    modifier = Modifier.testTag("tv-home-placeholder-row-$rail"),
                ) {
                    items(7) { card ->
                        Column(
                            Modifier.width(176.dp).testTag("tv-home-placeholder-card-$rail-$card"),
                        ) {
                            Box(
                                Modifier
                                    .fillMaxWidth()
                                    .aspectRatio(2f / 3f)
                                    .clip(RoundedCornerShape(14.dp))
                                    .background(MaterialTheme.colorScheme.surfaceVariant),
                            )
                            Spacer(Modifier.height(8.dp))
                            Box(
                                Modifier
                                    .fillMaxWidth(.8f)
                                    .height(14.dp)
                                    .clip(RoundedCornerShape(7.dp))
                                    .background(MaterialTheme.colorScheme.surfaceVariant),
                            )
                        }
                    }
                }
            }
        }
    }
}

/** Shimmering poster placeholders that mirror the real grid geometry. */
@Composable
private fun MediaGridSkeleton(isTv: Boolean) {
    val transition = rememberInfiniteTransition(label = "skeleton")
    val pulse = transition.animateFloat(
        initialValue = 0.4f,
        targetValue = 0.9f,
        animationSpec = infiniteRepeatable(
            animation = tween(700, easing = LinearEasing),
            repeatMode = RepeatMode.Reverse,
        ),
        label = "skeleton alpha",
    )
    val alpha = { pulse.value }
    LazyVerticalGrid(
        columns = if (isTv) GridCells.Fixed(7) else GridCells.Adaptive(140.dp),
        contentPadding = PaddingValues(if (isTv) 24.dp else 12.dp),
        horizontalArrangement = Arrangement.spacedBy(if (isTv) 16.dp else 12.dp),
        verticalArrangement = Arrangement.spacedBy(if (isTv) 20.dp else 16.dp),
        modifier = Modifier.fillMaxSize(),
        userScrollEnabled = false,
    ) {
        items(12) {
            Column {
                SkeletonBlock(alpha, Modifier.fillMaxWidth().aspectRatio(2f / 3f), 14.dp)
                Spacer(Modifier.height(8.dp))
                SkeletonBlock(alpha, Modifier.fillMaxWidth(0.8f).height(14.dp), 7.dp)
                Spacer(Modifier.height(4.dp))
                SkeletonBlock(alpha, Modifier.fillMaxWidth(0.5f).height(11.dp), 6.dp)
            }
        }
    }
}

/**
 * One shimmering placeholder bar. The pulse is passed as a lambda and applied in the draw phase,
 * so the animation never invalidates composition — it would otherwise recompose every placeholder
 * on every frame, for the whole time a load is in flight.
 */
@Composable
private fun SkeletonBlock(alpha: () -> Float, modifier: Modifier, corner: Dp) {
    val color = MaterialTheme.colorScheme.surfaceVariant
    val shape = RoundedCornerShape(corner)
    Box(
        modifier
            .clip(shape)
            .drawBehind { drawRect(color.copy(alpha = alpha())) },
    )
}

/** Before a search runs the grid invites one; afterwards it names the query that found nothing. */
@Composable
private fun searchEmptyTitle(query: String) =
    if (query.isBlank()) stringResource(R.string.search_start)
    else stringResource(R.string.search_no_results, query)

@Composable
private fun searchEmptyHint(query: String) =
    if (query.isBlank()) stringResource(R.string.search_start_hint)
    else stringResource(R.string.search_no_results_hint)

@Composable
private fun Empty(label: String, hint: String? = null) = Box(
    Modifier.fillMaxSize(),
    contentAlignment = Alignment.Center,
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally, modifier = Modifier.padding(32.dp)) {
        Icon(
            Icons.Default.Inbox,
            contentDescription = null,
            modifier = Modifier.size(56.dp),
            tint = MaterialTheme.colorScheme.onSurfaceVariant.copy(alpha = 0.5f),
        )
        Spacer(Modifier.height(12.dp))
        Text(
            label,
            style = MaterialTheme.typography.titleLarge,
            textAlign = TextAlign.Center,
        )
        if (hint != null) {
            Spacer(Modifier.height(4.dp))
            Text(
                hint,
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                textAlign = TextAlign.Center,
            )
        }
    }
}

@Composable
private fun ErrorBanner(
    message: String,
    dismiss: () -> Unit,
    modifier: Modifier = Modifier,
    retry: (() -> Unit)? = null,
    isTv: Boolean = false,
) {
    Snackbar(
        modifier = modifier.padding(12.dp),
        containerColor = MaterialTheme.colorScheme.errorContainer,
        contentColor = MaterialTheme.colorScheme.onErrorContainer,
        action = {
            Row(horizontalArrangement = Arrangement.spacedBy(4.dp)) {
                if (retry != null) {
                    TextButton(
                        onClick = { dismiss(); retry() },
                        modifier = Modifier.tvFocusScale(isTv),
                    ) { Text(stringResource(R.string.retry)) }
                }
                TextButton(onClick = dismiss) { Text(stringResource(R.string.dismiss)) }
            }
        },
    ) { Text(message) }
}
