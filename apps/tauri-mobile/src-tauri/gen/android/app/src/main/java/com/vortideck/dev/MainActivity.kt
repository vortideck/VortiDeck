package com.vortideck.dev

import android.app.Dialog
import android.graphics.Color
import android.graphics.drawable.GradientDrawable
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.View
import android.view.WindowManager
import android.webkit.WebView
import android.widget.FrameLayout
import android.widget.ImageView
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.WindowInsetsControllerCompat

class MainActivity : TauriActivity() {

    private var splashDialog: Dialog? = null
    private var webViewRef: WebView? = null
    private val handler = Handler(Looper.getMainLooper())
    private var splashDismissed = false
    private var splashStartTime = 0L

    private val SAFETY_TIMEOUT_MS = 15_000L
    private val POLL_INTERVAL_MS = 100L
    private val MIN_SPLASH_DURATION_MS = 1500L
    private val FADE_OUT_DURATION_MS = 600L

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Immersive mode
        WindowCompat.setDecorFitsSystemWindows(window, false)
        window.attributes.layoutInDisplayCutoutMode =
            WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        val windowInsetsController = WindowCompat.getInsetsController(window, window.decorView)
        windowInsetsController?.let {
            it.hide(WindowInsetsCompat.Type.statusBars() or WindowInsetsCompat.Type.navigationBars())
            it.systemBarsBehavior = WindowInsetsControllerCompat.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE
        }
        window.statusBarColor = Color.TRANSPARENT

        // Show splash as a Dialog (separate window layer, stays above WebView surface)
        splashStartTime = System.currentTimeMillis()
        showSplash()

        // Safety timeout
        handler.postDelayed({ dismissSplash() }, SAFETY_TIMEOUT_MS)
    }

    override fun onWebViewCreate(webView: WebView) {
        super.onWebViewCreate(webView)
        webViewRef = webView
        webView.setBackgroundColor(Color.parseColor("#0a0a0f"))
        startPolling()
    }

    private fun showSplash() {
        val dialog = Dialog(this, android.R.style.Theme_Black_NoTitleBar_Fullscreen)
        dialog.setCancelable(false)

        // Build splash content
        val splash = FrameLayout(this).apply {
            layoutParams = FrameLayout.LayoutParams(
                FrameLayout.LayoutParams.MATCH_PARENT,
                FrameLayout.LayoutParams.MATCH_PARENT
            )
        }

        // Dark gradient background
        val gradient = GradientDrawable(
            GradientDrawable.Orientation.TOP_BOTTOM,
            intArrayOf(
                Color.parseColor("#0a0a0f"),
                Color.parseColor("#0f0f1a"),
                Color.parseColor("#0a0a0f")
            )
        )
        splash.background = gradient

        // Subtle radial glow behind logo
        val glowView = View(this).apply {
            val glowGradient = GradientDrawable().apply {
                shape = GradientDrawable.OVAL
                setGradientCenter(0.5f, 0.5f)
                gradientType = GradientDrawable.RADIAL_GRADIENT
                gradientRadius = dpToPx(160).toFloat()
                colors = intArrayOf(
                    Color.parseColor("#1a14b8a6"),
                    Color.parseColor("#00000000")
                )
            }
            background = glowGradient
            layoutParams = FrameLayout.LayoutParams(
                dpToPx(320),
                dpToPx(320)
            ).apply {
                gravity = Gravity.CENTER
            }
        }
        splash.addView(glowView)

        // App logo
        val logo = ImageView(this).apply {
            setImageResource(R.mipmap.ic_launcher_foreground)
            layoutParams = FrameLayout.LayoutParams(
                dpToPx(120),
                dpToPx(120)
            ).apply {
                gravity = Gravity.CENTER
            }
        }
        splash.addView(logo)

        dialog.setContentView(splash)

        // Make dialog fullscreen and immersive
        dialog.window?.let { w ->
            w.setLayout(
                WindowManager.LayoutParams.MATCH_PARENT,
                WindowManager.LayoutParams.MATCH_PARENT
            )
            w.setBackgroundDrawableResource(android.R.color.transparent)
            w.addFlags(WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS)
            w.decorView.systemUiVisibility = (
                View.SYSTEM_UI_FLAG_FULLSCREEN
                or View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                or View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
                or View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
                or View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
                or View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            )
        }

        dialog.show()
        splashDialog = dialog
    }

    private fun startPolling() {
        handler.postDelayed(object : Runnable {
            override fun run() {
                if (splashDismissed) return
                val wv = webViewRef ?: return

                wv.evaluateJavascript("window.__APP_READY__ === true") { result ->
                    if (result == "true") {
                        val elapsed = System.currentTimeMillis() - splashStartTime
                        val remaining = MIN_SPLASH_DURATION_MS - elapsed
                        if (remaining > 0) {
                            handler.postDelayed({ dismissSplash() }, remaining)
                        } else {
                            dismissSplash()
                        }
                    } else if (!splashDismissed) {
                        handler.postDelayed(this, POLL_INTERVAL_MS)
                    }
                }
            }
        }, POLL_INTERVAL_MS)
    }

    private fun dismissSplash() {
        if (splashDismissed) return
        splashDismissed = true

        val dialog = splashDialog ?: return

        // Fade out the dialog's content
        val content = dialog.window?.decorView
        content?.animate()
            ?.alpha(0f)
            ?.setDuration(FADE_OUT_DURATION_MS)
            ?.withEndAction {
                try {
                    if (dialog.isShowing) dialog.dismiss()
                } catch (_: Exception) {}
                splashDialog = null
                webViewRef = null
            }
            ?.start()
    }

    private fun dpToPx(dp: Int): Int {
        return (dp * resources.displayMetrics.density).toInt()
    }

    override fun onDestroy() {
        handler.removeCallbacksAndMessages(null)
        try {
            splashDialog?.dismiss()
        } catch (_: Exception) {}
        super.onDestroy()
    }
}
