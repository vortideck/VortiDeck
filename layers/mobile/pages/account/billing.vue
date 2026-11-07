<template lang="pug">
div(class="min-h-screen relative overflow-hidden")
  div(class="relative min-h-screen pb-28")
    //- Header
    div(class="flex items-center justify-between px-5 pt-5 pb-3")
      button(@click="navigateTo('/account')" class="w-9 h-9 flex items-center justify-center rounded-xl bg-themeBackground3/80 border border-themeBorder")
        svg(class="w-4 h-4 text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
      h1(class="text-lg font-semibold text-themeText") Billing
      div(class="w-9")

    //- Loading
    div(v-if="isLoading" class="px-5 pt-6 space-y-4")
      div(class="animate-pulse space-y-4")
        div(class="h-32 bg-themeHint rounded-2xl")
        div(class="h-20 bg-themeHint rounded-2xl")

    div(v-else class="px-5 pt-2")
      //- Current Plan Card
      div(class="mb-5")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Current Plan
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden p-5")
          template(v-if="premium")
            div(class="flex items-center justify-between mb-3")
              div
                h4(class="text-lg font-bold text-themeText") {{ premium.plan_name || 'Premium' }}
                p(class="text-xs text-themeText3 mt-0.5") Active subscription
              div(class="px-3 py-1 rounded-full bg-themeConnected/10 border border-themeConnected/30")
                span(class="text-xs font-semibold text-themeConnected") Active

            //- Renewal date
            div(v-if="premium.current_period_end" class="flex items-center gap-2 p-3 bg-themeHint rounded-xl")
              svg(class="w-4 h-4 text-themeText3 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7V3m8 4V3m-9 8h10M5 21h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v12a2 2 0 002 2z")
              div
                p(class="text-xs text-themeText3") Renews on
                p(class="text-sm font-medium text-themeText") {{ formatDate(premium.current_period_end) }}

          template(v-else)
            div(class="text-center py-4")
              div(class="w-12 h-12 mx-auto mb-3 rounded-full bg-themeHint flex items-center justify-center")
                svg(class="w-6 h-6 text-themeText3" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
              h4(class="text-base font-semibold text-themeText mb-1") Free Plan
              p(class="text-sm text-themeText3") You're on the free tier

      //- Features
      div(class="mb-5")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") {{ premium ? 'Your Features' : 'Upgrade to Premium' }}
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden divide-y divide-themeBorder/40")
          div(v-for="feature in features" :key="feature.label" class="p-4 flex items-center gap-3")
            svg(class="w-4 h-4 flex-shrink-0" :class="feature.included ? 'text-themeConnected' : 'text-themeText3'" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(v-if="feature.included" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
              path(v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")
            span(class="text-sm text-themeText") {{ feature.label }}

      //- Manage subscription button
      button(
        v-if="premium"
        @click="openBillingPortal"
        class="w-full py-3.5 bg-themeBackground3/80 border border-themeBorder rounded-xl font-medium text-sm text-themeText2 hover:bg-themeHover transition-all flex items-center justify-center gap-2"
      )
        svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14")
        span Manage on Web

      button(
        v-else
        @click="openUpgrade"
        class="w-full py-3.5 bg-themeButton text-themeText rounded-xl font-semibold text-sm hover:opacity-90 transition-opacity flex items-center justify-center gap-2"
      )
        svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6")
        span Upgrade Now

</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useOAuthLogin } from "@/composables/useOAuthLogin";

definePageMeta({
  middleware: "auth",
});

const { getUserInfo } = useOAuthLogin();
const config = useRuntimeConfig();

const isLoading = ref(true);
const premium = ref<any>(null);

const features = [
  { label: "Unlimited deck configurations", included: true },
  { label: "Custom themes & layouts", included: true },
  { label: "Priority relay servers", included: false },
  { label: "Advanced macros & scripting", included: false },
  { label: "Priority support", included: false },
];

onMounted(async () => {
  try {
    const user = await getUserInfo();
    if (user?.premium) {
      premium.value = user.premium;
      // Mark all features as included for premium users
      features.forEach((f) => (f.included = true));
    }
  } catch {
    // Non-critical
  } finally {
    isLoading.value = false;
  }
});

const formatDate = (dateString: string) => {
  try {
    return new Date(dateString).toLocaleDateString("en-US", {
      year: "numeric",
      month: "long",
      day: "numeric",
    });
  } catch {
    return dateString;
  }
};

const openBillingPortal = async () => {
  try {
    await openUrl(`${config.public.APP_URL}/account/billing`);
  } catch (error) {
    console.error("Failed to open billing portal:", error);
  }
};

const openUpgrade = async () => {
  try {
    await openUrl(`${config.public.APP_URL}/pricing`);
  } catch (error) {
    console.error("Failed to open pricing page:", error);
  }
};
</script>
