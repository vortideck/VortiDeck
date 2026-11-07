<template lang="pug">
div
  Transition(name="fade")
    div(v-if="isOpen" @mousedown="$emit('close')" tabindex="-1" class="login-overlay")

  Transition(name="jump")
    div(v-if="isOpen" class="login-shell" @mousedown="$emit('close')")
      div(class="login-card" @mousedown.stop)
        // Header
        div(class="login-header")
          div(class="login-header-main")
            h3(class="login-title") {{ isForgotPassword ? $t('modals.login.reset_password') : (isRegister ? 'Register' : 'Login') }}
            p(v-if="isForgotPassword" class="login-subtitle") {{$t('modals.login.reset_password_description')}}
            p(v-else class="login-subtitle") Secure sign-in to your VortiDeck workspace
          button(@click="$emit('close')" class="login-close-btn")
              span(class="sr-only") {{$t('modals.login.close')}}
              svg(class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor")
                path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12")

        // Body
        div(class="login-body")
          // Social Login Buttons (hide on forgot password)
          div(v-if="!isForgotPassword" class="space-y-4")
            button(class="login-google-btn" @click="handleGoogleLogin")
              img(src="https://www.google.com/favicon.ico" class="w-5 h-5")
              span {{$t('modals.login.continue_with_google')}}

          div(v-if="!isForgotPassword" class="login-divider")
            span {{$t('modals.login.or_continue_with_email')}}

          // Forgot Password Form
          form(v-if="isForgotPassword" class="login-form" @submit.prevent="forgotPasswordApi()")
            div
              label(for="email" class="login-label") {{$t('modals.login.email')}}
              input#email(type="email" v-model="email" required class="login-input")

            button(type="submit" class="login-submit-btn") {{$t('modals.login.send_reset_link')}}

          // Login/Register Form
          div(v-else class="login-form")
            div
              label(for="email" class="login-label") {{$t('modals.login.email')}}
              input#email(type="email" v-model="email" required @keyup.enter="!isRegister && loginApi()" class="login-input")

            div
              label(for="password" class="login-label") {{$t('modals.login.password')}}
              input#password(type="password" v-model="password" required @keyup.enter="!isRegister && loginApi()" class="login-input")

            div(v-if="isRegister")
              label(for="confirm-password" class="login-label") {{$t('modals.login.confirm_password')}}
              input#confirm-password(type="password" v-model="confirmPassword" required @keyup.enter="isRegister && registerApi()" class="login-input")

            div(class="login-meta-row" v-if="!isRegister")
              div(class="flex items-center")
                input#remember-me(type="checkbox" class="login-checkbox")
                label(for="remember-me" class="ml-2 block text-sm text-themeText3") {{$t('modals.login.remember_me')}}

              button(type="button" @click="isForgotPassword = true" class="login-link-btn") {{$t('modals.login.forgot_password')}}

            button(class="login-submit-btn" @click="isRegister ? registerApi() : loginApi()") {{ isRegister ? 'Register' : 'Sign in' }}

            p(v-if="isRegister" class="text-xs text-themeText3 text-center mt-3") By registering, you agree to our&nbsp;
              a(href="https://vortideck.com/terms" target="_blank" class="text-blue-500 hover:underline") Terms of Service
              |  and&nbsp;
              a(href="https://vortideck.com/privacy" target="_blank" class="text-blue-500 hover:underline") Privacy Policy

        // Footer
        div(class="login-footer")
          p(v-if="isForgotPassword" class="text-sm text-themeText3") {{$t('modals.login.remember_password')}}
            button(@click="isForgotPassword = false" class="login-link-btn ml-2") {{$t('modals.login.sign_in')}}
          p(v-else class="text-sm text-themeText3") {{ isRegister ? $t('modals.login.already_have_account') : $t('modals.login.dont_have_account') }}
            button(@click="isRegister = !isRegister" class="login-link-btn ml-2") {{ isRegister ? $t('modals.login.sign_in') : $t('modals.login.sign_up') }}
</template>

<script setup lang="ts">
import { useMainStore } from "@/stores/Main";
import { listen } from "@tauri-apps/api/event";
import { useAuthStore } from "@/stores/Auth";
import { invoke } from "@tauri-apps/api/core";
import { fetch } from "@tauri-apps/plugin-http";

const authStore = useAuthStore();
const mainStore = useMainStore();
const config = useRuntimeConfig();

defineProps<{
  isOpen: boolean;
}>();

const emit = defineEmits(["close"]);
const isRegister = ref(false);
const isForgotPassword = ref(false);
const email = ref("");
const password = ref("");
const confirmPassword = ref("");

const { t } = useI18n();

let unlistenAuthToken: (() => void) | null = null;

onMounted(async () => {
  unlistenAuthToken = await listen("auth_token_saved", async () => {
    try {
      const accessToken = await invoke<string>("get_access_token");
      authStore.setAccessToken(accessToken);
      console.log(authStore.getAccessToken);
      mainStore.addNotification({
        title: t("pages.account.loggedIn"),
        message: t("pages.account.loggedInSuccess"),
        type: "success",
        autoClose: true,
      });

      emit("close");
    } catch (err) {
      console.error("Failed to retrieve access token securely:", err);
    }
  });
});

onUnmounted(() => {
  if (unlistenAuthToken) {
    unlistenAuthToken();
  }
});

const handleGoogleLogin = async () => {
  try {
    await useLoginWithGoogle();
  } catch (error) {
    mainStore.addNotification({
      title: "Google login failed",
      message: "Could not start Google sign in flow",
      type: "error",
      autoClose: true,
    });
  }
};

const loginApi = async () => {
  // await open("https://github.com/login/oauth/authorize");

  const result = await fetch(`${config.public.API_URL}/api/v1/auth/login`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "x-client-Type": "native",
      Accept: "application/json",
    },
    body: JSON.stringify({
      email: email.value,
      password: password.value,
    }),
  });

  if (result.ok) {
    mainStore.addNotification({
      title: "Login successful",
      message: "You have been logged in successfully",
      type: "success",
      autoClose: true,
    });

    const data = await result.json();
    authStore.setAccessToken(data.access);
    await invoke("set_refresh_token", { refreshToken: data.refresh });

    // 📊 Identify + track login
    const { $posthog } = useNuxtApp();
    const userData = getUserDataFromJwt(data.access);
    if (userData?.id) {
      $posthog?.identify(String(userData.id), {
        email: userData.email,
        username: userData.username,
      });
    }
    $posthog?.capture("user_logged_in", { method: "email" });

    // router.push("/account");
    emit("close");
  } else {
    mainStore.addNotification({
      title: "Login failed",

      message: "You have been logged in failed",
      type: "error",
      autoClose: true,
    });
  }
};

const registerApi = async () => {
  const result = await fetch(`${config.public.API_URL}/api/v1/auth/registration`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Client-Type": "native",
      Accept: "application/json",
    },
    body: JSON.stringify({
      email: email.value,
      password1: password.value,
      password2: confirmPassword.value,
    }),
  });

  if (result.ok) {
    mainStore.addNotification({
      title: "Registration successful",
      message: "You have been registered successfully",
      type: "success",
      autoClose: true,
    });

    const data = await result.json();
    authStore.setAccessToken(data.access);
    await invoke("set_refresh_token", { refreshToken: data.refresh });

    // 📊 Identify + track signup
    const { $posthog } = useNuxtApp();
    const signupUserData = getUserDataFromJwt(data.access);
    if (signupUserData?.id) {
      $posthog?.identify(String(signupUserData.id), {
        email: signupUserData.email,
        username: signupUserData.username,
      });
    }
    $posthog?.capture("user_signed_up");

    emit("close");
  } else {
    mainStore.addNotification({
      title: "Register failed",
      message: "You have been registered failed",
      type: "error",
    });
  }
};

const forgotPasswordApi = async () => {
  const result = await fetch(`${config.public.API_URL}/api/v1/auth/password/reset`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      "X-Client-Type": "native",
      Accept: "application/json",
    },
    body: JSON.stringify({
      email: email.value,
    }),
  });

  if (result.ok) {
    mainStore.addNotification({
      title: t("modals.login.reset_link_sent"),
      message: t("modals.login.check_email"),
      type: "success",
      autoClose: true,
      duration: 10000,
    });

    // Reset to login form
    isForgotPassword.value = false;
    email.value = "";
  } else {
    const errorData = await result.json().catch(() => ({ detail: "Failed to send reset link" }));
    mainStore.addNotification({
      title: t("modals.login.reset_failed"),
      message: errorData.detail || t("modals.login.reset_failed_message"),
      type: "error",
      autoClose: true,
    });
  }
};
</script>

<style lang="scss" scoped>
.login-overlay {
  backdrop-filter: blur(6px);
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 12050;
  width: 100%;
  height: 100vh;
  padding: 1rem;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(15, 23, 42, 0.44);
}

.login-shell {
  position: fixed;
  inset: 0;
  z-index: 12060;
  padding: 1rem;
  display: flex;
  align-items: center;
  justify-content: center;
  pointer-events: auto;
}

.login-card {
  width: 100%;
  max-width: 30rem;
  border-radius: 1rem;
  border: 1px solid hsl(var(--border2) / 0.95);
  background: linear-gradient(
      155deg,
      hsl(var(--background2) / 0.99),
      hsl(var(--background2) / 0.94)
    ),
    linear-gradient(135deg, hsl(var(--button) / 0.05), transparent 50%);
  box-shadow:
    0 28px 52px -36px rgba(13, 23, 38, 0.4),
    0 8px 20px -16px rgba(34, 95, 190, 0.24);
  overflow: hidden;
}

.login-header {
  padding: 1.35rem 1.55rem 1.1rem;
  border-bottom: 1px solid hsl(var(--border2) / 0.72);
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.9rem;
}

.login-header-main {
  min-width: 0;
}

.login-title {
  font-size: 2rem;
  line-height: 1.05;
  font-weight: 800;
  color: hsl(var(--text));
  letter-spacing: -0.02em;
}

.login-subtitle {
  margin-top: 0.38rem;
  color: hsl(var(--text2));
  font-size: 0.83rem;
}

.login-close-btn {
  width: 2rem;
  height: 2rem;
  border-radius: 0.6rem;
  border: 1px solid hsl(var(--border2) / 0.82);
  display: inline-flex;
  align-items: center;
  justify-content: center;
  color: hsl(var(--text2));
  transition: color 0.2s ease, border-color 0.2s ease, background 0.2s ease;
}

.login-close-btn:hover {
  color: hsl(var(--text));
  border-color: hsl(var(--border2));
  background: hsl(var(--background3) / 0.75);
}

.login-body {
  padding: 1.25rem 1.55rem;
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.login-google-btn {
  width: 100%;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.55rem;
  border-radius: 0.75rem;
  border: 1px solid hsl(var(--border2) / 0.88);
  background: hsl(var(--background2) / 1);
  color: hsl(var(--text));
  padding: 0.67rem 0.95rem;
  font-size: 0.95rem;
  font-weight: 700;
  transition: background 0.2s ease, border-color 0.2s ease;
}

.login-google-btn:hover {
  background: hsl(var(--background3) / 0.78);
  border-color: hsl(var(--border2));
}

.login-divider {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 0.8rem;
  color: hsl(var(--text3));
}

.login-divider::before {
  content: "";
  position: absolute;
  left: 0;
  right: 0;
  top: 50%;
  height: 1px;
  background: hsl(var(--border2) / 0.9);
}

.login-divider > span {
  position: relative;
  z-index: 1;
  background: hsl(var(--background2));
  padding: 0 0.55rem;
}

.login-form {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
}

.login-label {
  display: block;
  font-size: 0.83rem;
  font-weight: 700;
  color: hsl(var(--text2));
  margin-bottom: 0.38rem;
}

.login-input {
  width: 100%;
  border-radius: 0.72rem;
  border: 1px solid hsl(var(--border2) / 0.9);
  background: hsl(var(--background2) / 1);
  color: hsl(var(--text));
  font-size: 0.95rem;
  padding: 0.68rem 0.76rem;
  transition: border-color 0.2s ease, box-shadow 0.2s ease;
}

.login-input:focus {
  outline: none;
  border-color: hsl(var(--editor-accent));
  box-shadow: 0 0 0 3px hsl(var(--editor-accent) / 0.15);
}

.login-meta-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.7rem;
}

.login-checkbox {
  width: 1rem;
  height: 1rem;
  border-radius: 0.25rem;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background2));
}

.login-submit-btn {
  width: 100%;
  border-radius: 0.74rem;
  border: none;
  color: #fff;
  padding: 0.73rem 1rem;
  font-size: 1rem;
  font-weight: 700;
  background: linear-gradient(
    100deg,
    hsl(var(--editor-accent, 20 96% 55%) / 1),
    hsl(var(--editor-accent-2, 20 95% 62%) / 1)
  );
  box-shadow: 0 14px 24px -18px rgba(251, 113, 63, 0.78);
  transition: transform 0.2s ease, filter 0.2s ease;
}

.login-submit-btn:hover {
  transform: translateY(-1px);
  filter: brightness(1.04);
}

.login-link-btn {
  color: hsl(var(--editor-accent, 20 96% 55%) / 1);
  font-weight: 700;
  font-size: 0.86rem;
}

.login-link-btn:hover {
  color: hsl(var(--editor-accent-2, 20 95% 62%) / 1);
}

.login-footer {
  padding: 0.95rem 1.55rem 1.15rem;
  border-top: 1px solid hsl(var(--border2) / 0.72);
  text-align: right;
  background: hsl(var(--background2) / 0.86);
}

:global(.theme-default-dark) .login-overlay,
:global(.theme-glassy-dark) .login-overlay {
  background: rgba(2, 6, 23, 0.62);
}

.fade-enter-active {
  animation: fade-in 0.3s;
}

.fade-leave-active {
  animation: fade-out 0.3s;
}

@keyframes fade-in {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}

@keyframes fade-out {
  from {
    opacity: 1;
  }
  to {
    opacity: 0;
  }
}

.jump-enter-active {
  animation: jump-in 0.3s;
}

.jump-leave-active {
  animation: jump-out 0.3s;
}

@keyframes jump-in {
  0% {
    transform: translateY(-22px) scale(0.98);
    opacity: 0;
  }
  100% {
    transform: translateY(0) scale(1);
    opacity: 1;
  }
}

@keyframes jump-out {
  0% {
    transform: translateY(0) scale(1);
    opacity: 1;
  }
  100% {
    transform: translateY(22px) scale(0.98);
    opacity: 0;
  }
}
</style>
