import posthog from "posthog-js/dist/module.no-external"
import { captureEvent } from "tauri-plugin-better-posthog"
import { invoke } from "@tauri-apps/api/core"
import { listen } from "@tauri-apps/api/event"
import { useAuthStore } from "../../base/stores/Auth"

export default defineNuxtPlugin(() => {
  posthog.init("dummy_api_key", {
    capture_pageview: false,
    capture_pageleave: false,
    autocapture: false,
    rageclick: false,
    capture_dead_clicks: false,
    capture_exceptions: false,
    capture_heatmaps: false,
    capture_performance: false,
    save_referrer: false,
    save_campaign_params: false,
    disable_external_dependency_loading: true,
    disable_persistence: false,
    disable_surveys: true,
    disable_session_recording: true,
    disable_scroll_properties: true,
    disable_web_experiments: true,
    enable_recording_console_log: false,
    enable_heatmaps: false,
    advanced_enable_surveys: false,
    advanced_disable_flags: true,
    advanced_disable_feature_flags: true,
    advanced_disable_toolbar_metrics: true,
    advanced_disable_feature_flags_on_first_load: true,
    advanced_only_evaluate_survey_feature_flags: true,

    // Route all events through the Rust backend
    // PostHog's opt_out_capturing() makes capture() a no-op, so before_send never fires when opted out
    before_send: [
      (captureResult) => {
        if (captureResult) {
          const { event, properties } = captureResult
          const enriched = { ...properties, environment: import.meta.dev ? "development" : "production" }
          captureEvent(event, enriched).catch(console.error)
        }
        return null
      },
    ],
  })

  // Default to opted-OUT until user explicitly consents
  posthog.opt_out_capturing()

  // Sync analytics opt-out state from DB on startup (fire-and-forget, non-blocking)
  invoke<{ data: { analytics_enabled: boolean } }>("get_app_config")
    .then((config) => {
      if (config?.data?.analytics_enabled === true) {
        posthog.opt_in_capturing()
      }
    })
    .catch(() => { }) // DB not ready — stay opted-out

  // Identify user if already logged in (links anonymous device ID to real user)
  const authStore = useAuthStore()
  const token = authStore.getAccessToken?.token
  if (token) {
    const user = getUserDataFromJwt(token)
    if (user?.id) {
      posthog.identify(String(user.id), {
        email: user.email,
        username: user.username,
        name: [user.first_name, user.last_name].filter(Boolean).join(' ') || undefined,
      })
    }
  }

  // Sync analytics opt-out state whenever app_config changes (emitted by update_app_config)
  listen<{ data: { analytics_enabled: boolean } }>("app_config_updated", (event) => {
    if (event.payload?.data?.analytics_enabled === false) {
      if (!posthog.has_opted_out_capturing()) posthog.opt_out_capturing()
    } else {
      if (posthog.has_opted_out_capturing()) posthog.opt_in_capturing()
    }
  })

  // Track SPA route changes
  const router = useRouter()
  router.afterEach((to) => {
    posthog.capture("$pageview", { path: to.fullPath })
  })

  return {
    provide: {
      posthog,
    },
  }
})
