package org.movo.app.player

import android.app.Activity
import android.app.PendingIntent
import android.app.PictureInPictureParams
import android.app.RemoteAction
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Rect
import android.graphics.drawable.Icon
import android.os.Build
import android.util.Rational
import androidx.annotation.DrawableRes
import androidx.annotation.RequiresApi

/** Broadcast the buttons on the picture-in-picture window send back to the player. */
internal const val PIP_CONTROL_ACTION = "org.movo.app.PIP_CONTROL"
internal const val PIP_CONTROL_EXTRA = "control"
internal const val PIP_PLAY_PAUSE = 1
internal const val PIP_PREVIOUS = 2
internal const val PIP_NEXT = 3

/** The system refuses a window narrower than 1:2.39 or wider than 2.39:1. */
private const val PIP_MAX_ASPECT = 2.39f

/**
 * The window keeps the video's shape as far as the system allows; a portrait or very wide
 * stream is clamped rather than refused. An unknown size falls back to 16:9.
 */
internal fun pipAspectRatio(width: Int, height: Int): Pair<Int, Int> {
    if (width <= 0 || height <= 0) return 16 to 9
    val ratio = width.toFloat() / height
    return when {
        ratio > PIP_MAX_ASPECT -> 239 to 100
        ratio < 1f / PIP_MAX_ASPECT -> 100 to 239
        else -> width to height
    }
}

/** Only playback that is under way follows the user out of the app. */
internal fun wantsPictureInPicture(playWhenReady: Boolean, completed: Boolean, failed: Boolean) =
    playWhenReady && !completed && !failed

/** One button on the picture-in-picture window. */
internal class PipControl(val control: Int, @DrawableRes val icon: Int, val title: String)

/** The activity's picture-in-picture support, or null where the platform or device has none. */
internal fun pictureInPicture(activity: Activity): PictureInPicture? {
    if (Build.VERSION.SDK_INT < 26) return null
    if (!activity.packageManager.hasSystemFeature(PackageManager.FEATURE_PICTURE_IN_PICTURE)) return null
    return PictureInPicture(activity)
}

/**
 * Picture-in-picture for the phone layout. Playback moves into a floating window when the user
 * leaves the app or taps the button in the controls; the window carries play/pause and episode
 * buttons, and the player closes when the window is dismissed.
 */
internal class PictureInPicture(private val activity: Activity) {
    val active: Boolean get() = Build.VERSION.SDK_INT >= 26 && activity.isInPictureInPictureMode

    /**
     * Keeps the activity's parameters current, so an automatic entry on Android 12 and later
     * already has the right shape, buttons and source rectangle.
     */
    fun update(autoEnter: Boolean, aspect: Pair<Int, Int>, sourceRect: Rect?, title: String, controls: List<PipControl>) {
        if (Build.VERSION.SDK_INT < 26) return
        val params = PictureInPictureParams.Builder()
            .setAspectRatio(Rational(aspect.first, aspect.second))
            .setActions(controls.take(activity.maxNumPictureInPictureActions).map(::action))
        if (sourceRect != null) params.setSourceRectHint(sourceRect)
        if (Build.VERSION.SDK_INT >= 31) params.setAutoEnterEnabled(autoEnter)
        if (Build.VERSION.SDK_INT >= 33) params.setTitle(title)
        activity.setPictureInPictureParams(params.build())
    }

    fun enter(): Boolean {
        if (Build.VERSION.SDK_INT < 26) return false
        return runCatching { activity.enterPictureInPictureMode(PictureInPictureParams.Builder().build()) }
            .getOrDefault(false)
    }

    /** The parameters outlive the player, so the automatic entry is switched off with it. */
    fun disarm() {
        if (Build.VERSION.SDK_INT >= 31) {
            activity.setPictureInPictureParams(PictureInPictureParams.Builder().setAutoEnterEnabled(false).build())
        }
    }

    @RequiresApi(26)
    private fun action(control: PipControl): RemoteAction {
        val intent = Intent(PIP_CONTROL_ACTION)
            .setPackage(activity.packageName)
            .putExtra(PIP_CONTROL_EXTRA, control.control)
        val pending = PendingIntent.getBroadcast(
            activity,
            control.control,
            intent,
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT,
        )
        return RemoteAction(Icon.createWithResource(activity, control.icon), control.title, control.title, pending)
    }
}
