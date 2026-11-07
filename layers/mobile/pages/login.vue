<template lang="pug">
div(class="min-h-screen relative overflow-hidden bg-themeBackground")
  //- Centered content
  div(class="relative min-h-screen flex flex-col items-center justify-center px-4 py-6")
    //- Card
    div(class="w-full max-w-sm rounded-2xl border border-themePrimary/20 bg-themeBackground overflow-hidden")
      //- Header: Logo
      div(class="flex items-center gap-2 px-5 pt-5 pb-1")
        img(src="/logo.png" alt="VortiDeck" width="32" height="32" class="w-8 h-8 rounded-lg")
        span(class="text-themePrimary font-bold text-sm tracking-wide") VortiDeck

      //- Illustration area
      div(class="mx-5 mt-2 rounded-xl overflow-hidden relative h-32 bg-themePrimary/10")
        div(class="absolute inset-0 bg-gradient-to-t from-themeBackground/60 to-transparent")
        div(class="absolute inset-0 flex items-center justify-center")
          svg(class="w-16 h-16 text-themePrimary/30" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 15v2m-6 4h12a2 2 0 002-2v-6a2 2 0 00-2-2H6a2 2 0 00-2 2v6a2 2 0 002 2zm10-10V7a4 4 0 00-8 0v4h8z")
        //- Decorative dots
        div(class="absolute top-4 right-6 w-2 h-2 rounded-full bg-themePrimary/40 animate-pulse")
        div(class="absolute bottom-8 left-8 w-1.5 h-1.5 rounded-full bg-themePrimary/40 animate-pulse delay-700")
        div(class="absolute top-10 left-12 w-1 h-1 rounded-full bg-themePrimary/30 animate-pulse delay-1000")

      //- Welcome text
      div(class="px-5 pt-4 pb-1")
        h1(class="text-2xl font-bold text-themeText") Welcome Back
        p(class="text-themeText2 text-sm mt-1") Enter your credentials to access your secure workstation.

      //- Biometric warning
      div(v-if="!biometricAvailable" class="mx-5 mt-2 flex items-center gap-2 px-3 py-2 bg-themeConnectingBg border border-themeConnecting/20 rounded-lg")
        svg(class="w-4 h-4 text-themeConnecting flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z")
        p(class="text-themeConnecting text-xs") Biometric authentication not available

      //- Form
      form(@submit.prevent="handleEmailLogin" class="px-5 pt-3 pb-1 space-y-3")
        //- Email
        div
          label(for="email" class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") Email
          input#email(
            v-model="email"
            type="email"
            required
            placeholder="name@company.com"
            autocomplete="email"
            class="w-full px-4 py-3 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
          )

        //- Password
        div
          div(class="flex items-center justify-between mb-1.5")
            label(for="password" class="block text-xs font-semibold text-themeText2 tracking-wider uppercase") Password
            button(type="button" class="text-themePrimary text-xs font-medium hover:opacity-80 transition-opacity") Forgot?
          input#password(
            v-model="password"
            type="password"
            required
            placeholder="••••••••"
            autocomplete="current-password"
            class="w-full px-4 py-3 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
          )

        //- Submit button
        button(
          type="submit"
          :disabled="isLoading || !biometricAvailable"
          class="w-full py-3.5 bg-themeButton text-themeText rounded-xl font-semibold text-sm hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        )
          template(v-if="isLoading")
            div(class="w-4 h-4 border-2 border-themeText/30 border-t-themeText rounded-full animate-spin")
            span Signing in...
          template(v-else)
            span Sign In
            svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 5l7 7m0 0l-7 7m7-7H3")

      //- Divider
      div(class="px-5 py-2")
        div(class="flex items-center gap-3")
          div(class="flex-1 h-px bg-themeBorder")
          span(class="text-xs text-themeText3 font-medium uppercase tracking-wider") or continue with
          div(class="flex-1 h-px bg-themeBorder")

      //- OAuth buttons
      div(class="px-5 pb-3 flex gap-3")
        button(
          @click="handleGoogleLogin"
          :disabled="isLoading || !biometricAvailable"
          class="flex-1 flex items-center justify-center gap-2 py-3 bg-themeBackground3 border border-themeBorder rounded-xl text-sm font-medium text-themeText2 hover:bg-themeHover hover:border-themeBorder2 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        )
          svg(class="w-4 h-4" viewBox="0 0 24 24")
            path(fill="#4285F4" d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z")
            path(fill="#34A853" d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z")
            path(fill="#FBBC05" d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z")
            path(fill="#EA4335" d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z")
          span Google
        button(
          @click="handleDiscordLogin"
          :disabled="isLoading || !biometricAvailable"
          class="flex-1 flex items-center justify-center gap-2 py-3 bg-themeBackground3 border border-themeBorder rounded-xl text-sm font-medium text-themeText2 hover:bg-themeHover hover:border-themeBorder2 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
        )
          svg(class="w-4 h-4 text-[#5865F2]" viewBox="0 0 24 24" fill="currentColor")
            path(d="M20.317 4.37a19.791 19.791 0 0 0-4.885-1.515a.074.074 0 0 0-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 0 0-5.487 0a12.64 12.64 0 0 0-.617-1.25a.077.077 0 0 0-.079-.037A19.736 19.736 0 0 0 3.677 4.37a.07.07 0 0 0-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 0 0 .031.057a19.9 19.9 0 0 0 5.993 3.03a.078.078 0 0 0 .084-.028a14.09 14.09 0 0 0 1.226-1.994a.076.076 0 0 0-.041-.106a13.107 13.107 0 0 1-1.872-.892a.077.077 0 0 1-.008-.128a10.2 10.2 0 0 0 .372-.292a.074.074 0 0 1 .077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 0 1 .078.01c.12.098.246.198.373.292a.077.077 0 0 1-.006.127a12.299 12.299 0 0 1-1.873.892a.077.077 0 0 0-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 0 0 .084.028a19.839 19.839 0 0 0 6.002-3.03a.077.077 0 0 0 .032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 0 0-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419c0-1.333.956-2.419 2.157-2.419c1.21 0 2.176 1.096 2.157 2.42c0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419c0-1.333.955-2.419 2.157-2.419c1.21 0 2.176 1.096 2.157 2.42c0 1.333-.946 2.418-2.157 2.418z")
          span Discord

      //- Sign up link
      div(class="px-5 pb-4 text-center")
        p(class="text-themeText3 text-xs")
          | Don't have an account?&nbsp;
          button(@click="navigateTo('/register')" class="text-themePrimary font-semibold hover:opacity-80 transition-opacity") Sign up

    //- Loading overlay
    div(v-if="isLoading || oauthProgress" class="mt-3 w-full max-w-sm")
      div(class="flex items-center justify-center gap-3 px-4 py-2.5 bg-themeBackground3/80 backdrop-blur rounded-xl border border-themeBorder")
        div(class="w-4 h-4 border-2 border-themePrimary/30 border-t-themePrimary rounded-full animate-spin")
        p(class="text-themeText2 text-sm") {{ oauthProgress || "Signing in..." }}

    //- Error message
    div(v-if="errorMessage || oauthError" class="mt-3 w-full max-w-sm flex items-center gap-2 px-4 py-2.5 bg-themeErrorBg border border-themeError/20 rounded-xl")
      svg(class="w-4 h-4 text-themeError flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
        path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
      p(class="text-themeError text-xs") {{ oauthError || errorMessage }}

    //- Footer
    div(class="mt-4 text-center")
      p(class="text-themeText3 text-[10px] tracking-widest uppercase font-medium") SECURE CONNECTION &middot; VERSION 0.4.2

</template>

<script setup lang="ts">
import { ref, onMounted, type Ref } from "vue";
import { useOAuthLogin } from "~/composables/useOAuthLogin";
import { useSecureTokens } from "~/composables/useSecureTokens";
import { useAuthStore } from "@/stores/Auth";

definePageMeta({
  layout: false,
});

const { loginWithEmail, loginWithGoogle, loginWithDiscord } = useOAuthLogin();
const { isBiometricAvailable } = useSecureTokens();
const authStore = useAuthStore();

// Inject OAuth progress state from plugin
const { $oauthProgress, $oauthError } = useNuxtApp();
const oauthProgress = $oauthProgress as Ref<string>;
const oauthError = $oauthError as Ref<string>;

const email = ref("");
const password = ref("");
const isLoading = ref(false);
const errorMessage = ref("");
const biometricAvailable = ref(true);

onMounted(async () => {
  biometricAvailable.value = await isBiometricAvailable();

  if (authStore.isLoggedIn) {
    await navigateTo("/");
  }
});

const handleGoogleLogin = async () => {
  try {
    isLoading.value = true;
    errorMessage.value = "";
    await loginWithGoogle();
    isLoading.value = false;
  } catch (error: any) {
    errorMessage.value = error.message || "Failed to open Google OAuth";
    isLoading.value = false;
  }
};

const handleDiscordLogin = async () => {
  try {
    isLoading.value = true;
    errorMessage.value = "";
    await loginWithDiscord();
    isLoading.value = false;
  } catch (error: any) {
    errorMessage.value = error.message || "Failed to open Discord OAuth";
    isLoading.value = false;
  }
};

const handleEmailLogin = async () => {
  try {
    isLoading.value = true;
    errorMessage.value = "";
    await loginWithEmail(email.value, password.value);
    await navigateTo("/deck");
  } catch (error: any) {
    errorMessage.value = error.message || "Invalid email or password";
    isLoading.value = false;
  }
};
</script>

<style scoped>
.delay-700 {
  animation-delay: 700ms;
}

.delay-1000 {
  animation-delay: 1000ms;
}
</style>
