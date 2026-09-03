@file:OptIn(
    ExperimentalMaterial3Api::class,
    ExperimentalAnimationApi::class,
    ExperimentalMaterial3WindowSizeClassApi::class,
    ExperimentalComposeUiApi::class,
    ExperimentalTvMaterial3Api::class,
)

package org.movo.app

import androidx.window.layout.FoldingFeature
import androidx.window.layout.WindowInfoTracker
import org.movo.app.ui.LocalFold
import androidx.compose.runtime.CompositionLocalProvider
import org.movo.app.ui.LocalReducedMotion
import org.movo.app.ui.reducedMotionEnabled
import org.movo.app.account.LoginScreen
import org.movo.app.details.DetailsScreen
import org.movo.app.home.HomeFlow
import org.movo.app.ui.Loading
import org.movo.app.ui.MovoBlue
import org.movo.app.ui.tvFocusScale
import org.movo.app.core.Rating
import org.movo.app.core.AppEffect
import org.movo.app.core.DetailAction
import org.movo.app.core.MovoViewModel
import org.movo.app.core.Screen
import org.movo.app.settings.AppSettings
import org.movo.app.settings.Keys
import org.movo.app.settings.LayoutMode
import org.movo.app.settings.ThemePref
import org.movo.app.settings.save
import org.movo.app.settings.settings
import org.movo.app.player.PlayerActions
import org.movo.app.player.PlayerScreen
import android.annotation.SuppressLint
import android.app.UiModeManager
import android.content.Intent
import android.webkit.WebChromeClient
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import androidx.activity.ComponentActivity
import android.content.Context
import android.content.res.Configuration
import android.os.Build
import android.os.Bundle
import androidx.activity.compose.BackHandler
import androidx.activity.compose.LocalActivity
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.animation.ExperimentalAnimationApi
import androidx.compose.foundation.background
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.displayCutout
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.layout.width
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.material3.windowsizeclass.ExperimentalMaterial3WindowSizeClassApi
import androidx.compose.material3.windowsizeclass.WindowWidthSizeClass
import androidx.compose.material3.windowsizeclass.WindowHeightSizeClass
import androidx.compose.material3.windowsizeclass.calculateWindowSizeClass
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.SideEffect
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.ExperimentalComposeUiApi
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.stringResource
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.LifecycleEventObserver
import androidx.lifecycle.repeatOnLifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.core.net.toUri
import androidx.core.view.WindowCompat
import androidx.lifecycle.viewmodel.compose.viewModel
import kotlinx.coroutines.launch
import kotlinx.coroutines.flow.MutableStateFlow
import androidx.tv.material3.ExperimentalTvMaterial3Api

class MainActivity : ComponentActivity() {
    private val deepLinks = MutableStateFlow<String?>(null)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        takeDeepLink(intent)
        enableEdgeToEdge()
        // The theme's window background only has to cover the launch, before this activity draws.
        // Left in place it is a second opaque fill of the whole screen under a Compose surface
        // that already covers it, which on a television is the single largest piece of wasted
        // fill rate in the app.
        window.setBackgroundDrawable(null)
        setContent { MovoApp(deepLinks = deepLinks, consumeDeepLink = { deepLinks.value = null }) }
    }

    override fun onNewIntent(intent: Intent) {
        super.onNewIntent(intent)
        takeDeepLink(intent)
    }

    /**
     * Reads the link the activity was started with, then strips it off the intent. The intent
     * survives recreation, so a link left in place is handled again on every rotation and drags
     * the user back to the title they had already navigated away from.
     */
    private fun takeDeepLink(source: Intent?) {
        deepLinks.value = source?.data
            ?.takeIf { it.scheme == "https" && it.host == PROVIDER_HOST }
            ?.toString()
        source?.data = null
    }
}

private const val PROVIDER_HOST = "hdrzk.org"

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

    val activity = checkNotNull(LocalActivity.current) { "MovoApp must be hosted by an activity" }
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

    val reducedMotion = remember(context) { reducedMotionEnabled(context) }
    val layout by remember(activity) { WindowInfoTracker.getOrCreate(activity).windowLayoutInfo(activity) }
        .collectAsStateWithLifecycle(initialValue = null)
    val fold = layout?.displayFeatures
        ?.filterIsInstance<FoldingFeature>()
        ?.firstOrNull { it.state == FoldingFeature.State.HALF_OPENED }
    CompositionLocalProvider(
        LocalReducedMotion provides reducedMotion,
        LocalFold provides fold,
    ) {
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
                val snackbars = remember { SnackbarHostState() }
                val lifecycle = LocalLifecycleOwner.current.lifecycle
                // Collected once here rather than per screen: a message can be raised while playback
                // is on top, and the player is not inside the home Scaffold.
                LaunchedEffect(model, lifecycle) {
                    lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
                        model.effects.collect { effect ->
                            when (effect) {
                                is AppEffect.Message -> snackbars.showSnackbar(effect.text)
                            }
                        }
                    }
                }
                Box(Modifier.fillMaxSize()) {
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
                    SnackbarHost(
                        snackbars,
                        // Outside every Scaffold, so nothing else keeps it clear of the gesture bar.
                        Modifier
                            .align(Alignment.BottomCenter)
                            .windowInsetsPadding(WindowInsets.safeDrawing.only(WindowInsetsSides.Bottom)),
                    )
                }
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
    // Read once per stream rather than passed straight through: the position is written back on
    // every periodic save, and threading that into the player would recompose all of it every
    // five seconds for a value only its first composition reads.
    val resumePositionMs = remember(stream) { model.state.value.playbackPositionMs }
    val actions = remember(model, settings.saveQuality, scope, context) {
        object : PlayerActions {
            override fun saveProgress(positionMs: Long) = model.saveProgress(positionMs)
            override fun playbackStarted() = model.playbackStarted()
            override fun close(completed: Boolean, positionMs: Long) {
                model.closePlayer(completed, positionMs)
            }

            override fun previousEpisode() {
                model.previousEpisode()
            }

            override fun nextEpisode(completed: Boolean) {
                model.nextEpisode(completed)
            }

            override fun playEpisode(season: Long, episode: Long) {
                model.playEpisode(season, episode)
            }

            override fun openRating(positionMs: Long) =
                model.openPlayerAction(DetailAction.Rating, positionMs)

            override fun qualityChanged(quality: String, persist: Boolean) {
                model.selectPlaybackQuality(quality)
                if (persist && settings.saveQuality) scope.launch { context.save(Keys.LAST_QUALITY, quality) }
            }
        }
    }
    PlayerScreen(
        bundle = stream,
        title = state.details?.title.orEmpty(),
        resumePositionMs = resumePositionMs,
        preferredQuality = state.playbackQuality,
        syncError = state.error,
        isTv = isTv,
        hasPreviousEpisode = adjacentEpisodes.first,
        hasNextEpisode = adjacentEpisodes.second,
        seasons = state.details?.seasons.orEmpty(),
        settings = settings,
        actions = actions,
    )
}

/** Keeps a [WebView] on the one host it started on, over https only. */
private class SameHostWebViewClient(private val host: String?) : WebViewClient() {
    override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
        val url = request.url
        return !(url.scheme == "https" && url.host == host)
    }
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
            // The embed comes from the provider, so it is scripted content this app does not
            // control. Pin it to the host it was loaded from: a trailer never needs to navigate
            // anywhere else, and without this the page picks where the user goes next.
            webViewClient = SameHostWebViewClient(url.toUri().host)
            loadUrl(url)
        }
    }
    val lifecycleOwner = LocalLifecycleOwner.current
    // Leaving the app does not leave the composition, so without this the trailer keeps playing
    // its audio over whatever the user switched to.
    DisposableEffect(lifecycleOwner, webView) {
        val observer = LifecycleEventObserver { _, event ->
            when (event) {
                Lifecycle.Event.ON_PAUSE -> webView.onPause()
                Lifecycle.Event.ON_RESUME -> webView.onResume()
                else -> Unit
            }
        }
        lifecycleOwner.lifecycle.addObserver(observer)
        onDispose {
            lifecycleOwner.lifecycle.removeObserver(observer)
            webView.stopLoading()
            webView.destroy()
        }
    }
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
