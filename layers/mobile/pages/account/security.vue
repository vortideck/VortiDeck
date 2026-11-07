<template lang="pug">
div(class="min-h-screen relative overflow-hidden")
  div(class="relative min-h-screen pb-28")
    //- Header
    div(class="flex items-center justify-between px-5 pt-5 pb-3")
      button(@click="navigateTo('/account')" class="w-9 h-9 flex items-center justify-center rounded-xl bg-themeBackground3/80 border border-themeBorder")
        svg(class="w-4 h-4 text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
      h1(class="text-lg font-semibold text-themeText") Security
      div(class="w-9")

    div(class="px-5 pt-2")
      //- Change Password Section
      div(class="mb-6")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Change Password

        div(class="space-y-3")
          //- Current password
          div
            label(class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") Current Password
            div(class="relative")
              input(
                v-model="currentPassword"
                :type="showCurrentPassword ? 'text' : 'password'"
                placeholder="Enter current password"
                autocomplete="current-password"
                class="w-full px-4 py-3 pr-12 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
              )
              button(@click="showCurrentPassword = !showCurrentPassword" type="button" class="absolute right-3 top-1/2 -translate-y-1/2 text-themeText3 hover:text-themeText2 transition-colors")
                svg(class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(v-if="showCurrentPassword" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21")
                  path(v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z")

          //- New password
          div
            label(class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") New Password
            div(class="relative")
              input(
                v-model="newPassword"
                :type="showNewPassword ? 'text' : 'password'"
                placeholder="Enter new password"
                autocomplete="new-password"
                class="w-full px-4 py-3 pr-12 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
              )
              button(@click="showNewPassword = !showNewPassword" type="button" class="absolute right-3 top-1/2 -translate-y-1/2 text-themeText3 hover:text-themeText2 transition-colors")
                svg(class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                  path(v-if="showNewPassword" stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21")
                  path(v-else stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z")

          //- Confirm new password
          div
            label(class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") Confirm New Password
            input(
              v-model="confirmPassword"
              :type="showNewPassword ? 'text' : 'password'"
              placeholder="Confirm new password"
              autocomplete="new-password"
              class="w-full px-4 py-3 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
            )

          //- Password mismatch warning
          div(v-if="confirmPassword && newPassword !== confirmPassword" class="flex items-center gap-2 px-3 py-2 bg-themeConnectingBg border border-themeConnecting/20 rounded-lg")
            svg(class="w-4 h-4 text-themeConnecting flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z")
            p(class="text-themeConnecting text-xs") Passwords do not match

      //- Save button
      button(
        @click="changePassword"
        :disabled="isSaving || !canSave"
        class="w-full py-3.5 bg-themeButton text-themeText rounded-xl font-semibold text-sm hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      )
        template(v-if="isSaving")
          div(class="w-4 h-4 border-2 border-themeText/30 border-t-themeText rounded-full animate-spin")
          span Updating...
        template(v-else)
          span Update Password

      //- Biometric Authentication
      div(class="mt-6")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Biometric Authentication
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder overflow-hidden divide-y divide-themeBorder/40")
          div(class="p-4 flex items-center justify-between")
            div(class="flex-1 pr-4")
              div(class="flex items-center gap-3")
                div(class="w-9 h-9 rounded-lg bg-themeHint flex items-center justify-center")
                  svg(class="w-[18px] h-[18px] text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                    path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 11c0 3.517-1.009 6.799-2.753 9.571m-3.44-2.04l.054-.09A13.916 13.916 0 008 11a4 4 0 118 0c0 1.017-.07 2.019-.203 3m-2.118 6.844A21.88 21.88 0 0015.171 17m3.839 1.132c.645-2.266.99-4.659.99-7.132A8 8 0 008 4.07M3 15.364c.64-1.319 1-2.8 1-4.364 0-1.457.39-2.823 1.07-4")
                div
                  p(class="text-sm font-medium text-themeText") Biometric Lock
                  p(class="text-xs text-themeText3 mt-0.5") {{ biometricStatusText }}
            button(
              v-if="biometricAvailable"
              @click="toggleBiometric"
              :class="biometricEnabled ? 'bg-themePrimary' : 'bg-themeHint'"
              class="relative inline-flex h-6 w-11 items-center rounded-full transition-colors duration-200 flex-shrink-0"
            )
              span(
                :class="biometricEnabled ? 'translate-x-6' : 'translate-x-1'"
                class="inline-block h-4 w-4 transform rounded-full bg-white shadow-sm transition-transform duration-200"
              )
          //- Not available warning
          div(v-if="!biometricAvailable" class="p-3 flex items-center gap-2")
            svg(class="w-4 h-4 text-themeConnecting flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z")
            p(class="text-themeConnecting text-xs") No biometric hardware detected on this device

      //- Login method info
      div(class="mt-6")
        h3(class="text-xs font-bold text-themePrimary tracking-wider uppercase mb-3") Login Method
        div(class="bg-themeBackground3/80 rounded-2xl border border-themeBorder p-4")
          div(class="flex items-center gap-3")
            div(class="w-9 h-9 rounded-lg bg-themeHint flex items-center justify-center")
              svg(class="w-[18px] h-[18px] text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m5.618-4.016A11.955 11.955 0 0112 2.944a11.955 11.955 0 01-8.618 3.04A12.02 12.02 0 003 9c0 5.591 3.824 10.29 9 11.622 5.176-1.332 9-6.03 9-11.622 0-1.042-.133-2.052-.382-3.016z")
            div
              p(class="text-sm font-medium text-themeText") {{ loginMethod }}
              p(class="text-xs text-themeText3") Your current sign-in method

      //- Success message
      div(v-if="successMessage" class="mt-3 flex items-center gap-2 px-4 py-2.5 bg-themeConnected/10 border border-themeConnected/20 rounded-xl")
        svg(class="w-4 h-4 text-themeConnected flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7")
        p(class="text-themeConnected text-sm") {{ successMessage }}

      //- Error message
      div(v-if="errorMessage" class="mt-3 flex items-center gap-2 px-4 py-2.5 bg-themeErrorBg border border-themeError/20 rounded-xl")
        svg(class="w-4 h-4 text-themeError flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z")
        p(class="text-themeError text-sm") {{ errorMessage }}

</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { useHttpClient } from "@/composables/useHttpClient";
import { useSecureTokens } from "@/composables/useSecureTokens";

definePageMeta({
  middleware: "auth",
});

const { post } = useHttpClient();
const { isBiometricAvailable, hasTokens } = useSecureTokens();

const currentPassword = ref("");
const newPassword = ref("");
const confirmPassword = ref("");
const showCurrentPassword = ref(false);
const showNewPassword = ref(false);
const isSaving = ref(false);
const errorMessage = ref("");
const successMessage = ref("");
const loginMethod = ref("Email & Password");
const biometricAvailable = ref(false);
const biometricEnabled = ref(false);

const BIOMETRIC_PREF_KEY = "vortideck_biometric_enabled";

const biometricStatusText = computed(() => {
  if (!biometricAvailable.value) return "Not available on this device";
  return biometricEnabled.value
    ? "Require fingerprint or face to access tokens"
    : "Tokens stored without biometric protection";
});

const canSave = computed(() => {
  return (
    currentPassword.value.length > 0 &&
    newPassword.value.length >= 8 &&
    newPassword.value === confirmPassword.value
  );
});

onMounted(async () => {
  try {
    biometricAvailable.value = await isBiometricAvailable();
    biometricEnabled.value =
      localStorage.getItem(BIOMETRIC_PREF_KEY) !== "false" && biometricAvailable.value;
  } catch {
    // Non-critical
  }

  // Provider is stored in localStorage during login (not in JWT)
  try {
    const provider = localStorage.getItem("vortideck_login_provider");
    if (provider) {
      const providers: Record<string, string> = {
        google: "Google OAuth",
        discord: "Discord OAuth",
        kick: "Kick OAuth",
        email: "Email & Password",
      };
      loginMethod.value = providers[provider] || "Email & Password";
    }
  } catch {
    // Non-critical, keep default
  }
});

const toggleBiometric = () => {
  biometricEnabled.value = !biometricEnabled.value;
  localStorage.setItem(BIOMETRIC_PREF_KEY, String(biometricEnabled.value));
};

const changePassword = async () => {
  if (!canSave.value) return;

  isSaving.value = true;
  errorMessage.value = "";
  successMessage.value = "";

  try {
    const { error } = await post("api/v1/auth/password/change", {
      current_password: currentPassword.value,
      new_password1: newPassword.value,
      new_password2: confirmPassword.value,
    });

    if (error) {
      errorMessage.value = error;
      return;
    }

    successMessage.value = "Password updated successfully";
    currentPassword.value = "";
    newPassword.value = "";
    confirmPassword.value = "";
  } catch (error: any) {
    errorMessage.value = error.message || "Failed to update password";
  } finally {
    isSaving.value = false;
  }
};
</script>
