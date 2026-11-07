<template lang="pug">
div(class="min-h-screen relative overflow-hidden")
  div(class="relative min-h-screen pb-28")
    //- Header
    div(class="flex items-center justify-between px-5 pt-5 pb-3")
      button(@click="navigateTo('/account')" class="w-9 h-9 flex items-center justify-center rounded-xl bg-themeBackground3/80 border border-themeBorder")
        svg(class="w-4 h-4 text-themeText2" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7")
      h1(class="text-lg font-semibold text-themeText") Personal Info
      div(class="w-9")

    //- Loading
    div(v-if="isLoading" class="px-5 pt-6 space-y-4")
      div(class="animate-pulse space-y-4")
        div(class="h-24 w-24 bg-themeHint rounded-full mx-auto")
        div(class="h-10 bg-themeHint rounded-xl")
        div(class="h-10 bg-themeHint rounded-xl")

    //- Form
    div(v-else class="px-5 pt-2")
      //- Avatar section
      div(class="flex flex-col items-center mb-6")
        div(class="relative mb-2")
          div(class="w-24 h-24 rounded-full bg-gradient-to-br from-themePrimary to-themeSecondary p-[3px]")
            div(class="w-full h-full rounded-full bg-themeBackground p-[3px]")
              div(class="w-full h-full rounded-full bg-themeHint overflow-hidden flex items-center justify-center")
                img(v-if="displayAvatarUrl" :src="displayAvatarUrl" alt="Profile" class="w-full h-full object-cover")
                span(v-else class="text-xl font-bold text-themePrimary") {{ initials }}
          //- Camera overlay
          label(for="avatar-input" class="absolute bottom-0 right-0 w-7 h-7 bg-themePrimary rounded-full flex items-center justify-center cursor-pointer border-2 border-themeBackground")
            svg(class="w-3.5 h-3.5 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 9a2 2 0 012-2h.93a2 2 0 001.664-.89l.812-1.22A2 2 0 0110.07 4h3.86a2 2 0 011.664.89l.812 1.22A2 2 0 0018.07 7H19a2 2 0 012 2v9a2 2 0 01-2 2H5a2 2 0 01-2-2V9z")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 13a3 3 0 11-6 0 3 3 0 016 0z")
        input#avatar-input(type="file" accept="image/*" @change="onAvatarChange" class="hidden")
        button(@click="triggerAvatarPicker" class="text-themePrimary text-xs font-medium hover:opacity-80 transition-opacity") Change Photo

      //- Nickname field
      div(class="mb-4")
        label(class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") Nickname
        input(
          v-model="formUsername"
          type="text"
          placeholder="Your nickname"
          class="w-full px-4 py-3 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
        )

      //- First Name field
      div(class="mb-4")
        label(class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") First Name
        input(
          v-model="formFirstName"
          type="text"
          placeholder="First name"
          class="w-full px-4 py-3 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
        )

      //- Last Name field
      div(class="mb-4")
        label(class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") Last Name
        input(
          v-model="formLastName"
          type="text"
          placeholder="Last name"
          class="w-full px-4 py-3 bg-themeBackground3 text-themeText border border-themeBorder rounded-xl placeholder-themeText3 text-sm focus:outline-none focus:ring-2 focus:ring-themePrimary/50 focus:border-themePrimary/50 transition-all"
        )

      //- Email field (read-only)
      div(class="mb-6")
        label(class="block text-xs font-semibold text-themeText2 tracking-wider uppercase mb-1.5") Email
        input(
          :value="formEmail"
          type="email"
          disabled
          class="w-full px-4 py-3 bg-themeBackground3/50 text-themeText3 border border-themeBorder rounded-xl text-sm cursor-not-allowed"
        )
        p(class="text-themeText3 text-[11px] mt-1") Email cannot be changed

      //- Save button
      button(
        @click="saveProfile"
        :disabled="isSaving || !hasChanges"
        class="w-full py-3.5 bg-themeButton text-themeText rounded-xl font-semibold text-sm hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
      )
        template(v-if="isSaving")
          div(class="w-4 h-4 border-2 border-themeText/30 border-t-themeText rounded-full animate-spin")
          span Saving...
        template(v-else)
          span Save Changes

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
import { useOAuthLogin } from "@/composables/useOAuthLogin";
import { useHttpClient } from "@/composables/useHttpClient";

definePageMeta({
  middleware: "auth",
});

const MAX_AVATAR_SIZE = 512;
const JPEG_QUALITY = 0.8;

const { getUserInfo } = useOAuthLogin();
const { request } = useHttpClient();
const config = useRuntimeConfig();

const isLoading = ref(true);
const isSaving = ref(false);
const errorMessage = ref("");
const successMessage = ref("");

const originalUsername = ref("");
const originalFirstName = ref("");
const originalLastName = ref("");
const formUsername = ref("");
const formFirstName = ref("");
const formLastName = ref("");
const formEmail = ref("");
const avatarRaw = ref<string | null>(null);
const avatarPreview = ref<string | null>(null);
const avatarFile = ref<File | null>(null);

const avatarUrl = computed(() => {
  if (!avatarRaw.value) return null;
  return avatarRaw.value.startsWith("http")
    ? avatarRaw.value
    : `${config.public.API_URL}${avatarRaw.value}`;
});

const displayAvatarUrl = computed(() => {
  return avatarPreview.value || avatarUrl.value;
});

const initials = computed(() => {
  const name = formFirstName.value || formUsername.value || formEmail.value;
  return name?.[0]?.toUpperCase() || "?";
});

const hasChanges = computed(() => {
  return (
    formUsername.value !== originalUsername.value ||
    formFirstName.value !== originalFirstName.value ||
    formLastName.value !== originalLastName.value ||
    avatarFile.value !== null
  );
});

/**
 * Compress and resize an image file using Canvas.
 */
const compressImage = (file: File): Promise<File> => {
  return new Promise((resolve, reject) => {
    const img = new Image();
    const url = URL.createObjectURL(file);

    img.onload = () => {
      URL.revokeObjectURL(url);
      let { width, height } = img;

      if (width > MAX_AVATAR_SIZE || height > MAX_AVATAR_SIZE) {
        if (width > height) {
          height = Math.round(height * (MAX_AVATAR_SIZE / width));
          width = MAX_AVATAR_SIZE;
        } else {
          width = Math.round(width * (MAX_AVATAR_SIZE / height));
          height = MAX_AVATAR_SIZE;
        }
      }

      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      const ctx = canvas.getContext("2d");
      if (!ctx) { reject(new Error("Canvas not supported")); return; }

      ctx.drawImage(img, 0, 0, width, height);
      canvas.toBlob(
        (blob) => {
          if (!blob) { reject(new Error("Failed to compress image")); return; }
          const compressed = new File([blob], "avatar.jpg", { type: "image/jpeg" });
          console.log(`📷 Avatar: ${(file.size / 1024).toFixed(0)}KB → ${(compressed.size / 1024).toFixed(0)}KB (${width}x${height})`);
          resolve(compressed);
        },
        "image/jpeg",
        JPEG_QUALITY
      );
    };

    img.onerror = () => { URL.revokeObjectURL(url); reject(new Error("Failed to load image")); };
    img.src = url;
  });
};

onMounted(async () => {
  try {
    const user = await getUserInfo();
    if (user) {
      formUsername.value = user.username || "";
      formFirstName.value = user.first_name || user.name || "";
      formLastName.value = user.last_name || "";
      formEmail.value = user.email || "";
      originalUsername.value = formUsername.value;
      originalFirstName.value = formFirstName.value;
      originalLastName.value = formLastName.value;
      avatarRaw.value = user.avatar || null;
    }
  } catch (error) {
    console.error("Failed to load user info:", error);
    errorMessage.value = "Failed to load profile data";
  } finally {
    isLoading.value = false;
  }
});

const triggerAvatarPicker = () => {
  document.getElementById("avatar-input")?.click();
};

const onAvatarChange = async (event: Event) => {
  const target = event.target as HTMLInputElement;
  if (!target.files || !target.files[0]) return;

  try {
    const compressed = await compressImage(target.files[0]);
    avatarFile.value = compressed;
    const reader = new FileReader();
    reader.onload = (e) => { avatarPreview.value = e.target?.result as string; };
    reader.readAsDataURL(compressed);
  } catch (err) {
    console.error("Failed to compress avatar:", err);
    errorMessage.value = "Failed to process image. Try a different photo.";
  }
};

const saveProfile = async () => {
  if (!hasChanges.value) return;

  isSaving.value = true;
  errorMessage.value = "";
  successMessage.value = "";

  try {
    // Use PATCH /api/v1/accounts/update-profile with FormData (same as web app)
    const formData = new FormData();
    formData.append("username", formUsername.value);
    formData.append("first_name", formFirstName.value);
    formData.append("last_name", formLastName.value);
    if (avatarFile.value) {
      formData.append("avatar", avatarFile.value);
    }

    // useHttpClient handles auth (token from Pinia) + auto-refresh on 401
    const { data, error } = await request(
      "api/v1/accounts/update-profile",
      { method: "PATCH", body: formData }
    );

    if (error) {
      errorMessage.value = error;
      return;
    }

    // Update avatar URL from backend response so the new avatar shows immediately
    if (data?.avatar) {
      avatarRaw.value = data.avatar;
    }
    avatarFile.value = null;
    avatarPreview.value = null;
    originalUsername.value = formUsername.value;
    originalFirstName.value = formFirstName.value;
    originalLastName.value = formLastName.value;
    successMessage.value = "Profile updated successfully";
  } catch (error: any) {
    errorMessage.value = error.message || "Failed to update profile";
  } finally {
    isSaving.value = false;
  }
};
</script>
