<template lang="pug">
div(class="topbar-actions-root")
  div(class="topbar-meta" v-if="props.showMeta")
    div(v-if="connectionStore.getStatus === 'connected'" class="text-themeConnected flex items-center gap-x-2 relative group")
      div(class="w-3 h-3 bg-themeConnected rounded-full")
      p {{$t('connection.connected')}}

    div(v-else-if="connectionStore.getStatus === 'disconnected'" class="flex items-center gap-x-2 relative group")
      div(class="w-3 h-3 bg-themeDisconnected rounded-full")
      p(class="text-themeDisconnected") {{$t('connection.disconnected')}}

    div(v-else-if="connectionStore.getStatus === 'paired'" class="flex items-center gap-x-2 relative group")
      div(class="w-3 h-3 bg-themePaired rounded-full")
      p(class="text-themePaired") {{$t('connection.paired')}}
      div(class="absolute group-hover:flex hidden top-[2rem] left-[0.1rem] bg-themeBackground2 px-3 py-2 shadow-md text-nowrap rounded-lg")
        p(v-for="device in connectionStore.getPaired") {{device.device}}

    div(v-else-if="connectionStore.getStatus === 'reconnecting'" class="flex items-center gap-x-2 relative group")
      div.status.reconnecting(class="w-3 h-3 bg-themeConnecting rounded-full")
      p(class="text-themeConnecting") {{$t('connection.reconnecting')}}

    NuxtLink(to="/pair" class="topbar-square-btn" :aria-label="$t('index.pair_device')")
      IcBaselineQrcode(class="w-[1.15rem] h-[1.15rem] text-themeText2")

  div(class="topbar-actions")
    NavbarProfiles(
      @click.stop=""
      :isProfileSearch="isProfileSearch"
      :isProfileSearchClicked="isProfileSearchClicked"
      @click="isProfileSearchClicked = !isProfileSearchClicked"
      class="topbar-profile-switcher"
      @mouseenter="isProfileSearch = true"
      @mouseleave="isProfileSearch = false"
    )

    div(class="relative")
      button(type="button" class="topbar-square-btn notif-trigger" aria-label="Notifications" @click.stop="toggleNotifications")
        MdiBellOutline(class="w-[1.1rem] h-[1.1rem] text-themeText3")
        span(v-if="notificationCount > 0" class="notif-dot") {{ notificationCount > 9 ? "9+" : notificationCount }}

      div(
        v-if="isNotificationsOpen"
        class="topbar-popover notif-dropdown"
        :class="selectedNotification ? 'notif-dropdown--expanded' : ''"
      )
        div(class="notif-content")
          div(class="notif-list-pane")
            div(class="notif-header")
              p(class="topbar-popover-title !mb-0") Notifications
              button(v-if="notificationCount > 0" type="button" class="notif-clear-btn" @click.stop="clearNotifications") Clear
            div(v-if="latestNotifications.length === 0" class="topbar-empty-state") No notifications yet
            div(v-else class="notif-list")
              button(
                v-for="notification in latestNotifications"
                :key="notification.id"
                type="button"
                class="notif-item"
                :class="selectedNotification?.id === notification.id ? 'notif-item--active' : ''"
                @click.stop="handleNotificationClick(notification)"
              )
                p(class="notif-item-title") {{ notification.title || "Notification" }}
                p(class="notif-item-message") {{ notification.message || "New update available." }}

          div(v-if="selectedNotification" class="notif-detail-pane")
            div(class="notif-detail-header")
              div
                p(class="notif-detail-eyebrow") Notification
                h3(class="notif-detail-title") {{ selectedNotification?.title || "Notification" }}
              button(type="button" class="notif-detail-close" @click="closeNotificationDetails") ✕

            div(class="notif-detail-body")
              p(class="notif-detail-message") {{ selectedNotification?.message || "No additional details." }}
              div(class="notif-detail-meta")
                div(class="notif-detail-meta-item")
                  span(class="notif-detail-meta-label") ID
                  span(class="notif-detail-meta-value") {{ "#" + (selectedNotification?.id || "-") }}
                div(class="notif-detail-meta-item")
                  span(class="notif-detail-meta-label") Type
                  span(class="notif-detail-meta-value") {{ selectedNotification?.type || "general" }}
                div(class="notif-detail-meta-item")
                  span(class="notif-detail-meta-label") Window
                  span(class="notif-detail-meta-value") {{ selectedNotification?.windowId || "all" }}

            div(class="notif-detail-actions")
              button(
                v-if="selectedNotification && typeof selectedNotification.buttonFunction === 'function'"
                type="button"
                class="notif-detail-btn notif-detail-btn--primary"
                @click="runNotificationAction"
              ) {{ selectedNotification?.buttonText || "Run Action" }}
              button(
                type="button"
                class="notif-detail-btn"
                @click="closeNotificationDetails"
              ) Back to List

    div(class="relative")
      button(type="button" class="topbar-square-btn plugin-trigger" aria-label="Plugins" @click.stop="togglePluginsDropdown")
        MingcutePluginLine(class="w-[1.1rem] h-[1.1rem] text-themeText3")
      NavbarPluginsDropdown(
        v-if="isPluginsDropdownOpen"
        @close="isPluginsDropdownOpen = false"
        class="absolute right-0 top-10 z-[150] plugins-dropdown"
      )

    button(
      type="button"
      class="avatar-chip avatar-chip-btn"
      aria-label="Account"
      @click.stop="handleClickAccount"
      :title="userData?.username || userData?.name || userStore.username || 'Account'"
    )
      img(v-if="userAvatar" :src="userAvatar" class="w-full h-full object-cover rounded-full")
      span(v-else-if="userInitials") {{ userInitials }}
      MdiAccount(v-else class="w-4 h-4 text-inherit")

  ModalLogin(:isOpen="desktopUtils.getIsUserModal" @close="desktopUtils.setUserModal(false)")
</template>

<script setup lang="ts">
import MdiBellOutline from "~icons/mdi/bell-outline?width=24px&height=24px";
import MingcutePluginLine from "~icons/mingcute/plugin-line";
import IcBaselineQrcode from "~icons/ic/baseline-qrcode?width=24px&height=24px";
import MdiAccount from "~icons/mdi/account?width=24px&height=24px";
import { useDesktopUtils } from "@desktop/stores/desktopUtils";
import { useConnectionStore } from "@/stores/Connection";
import { useAuthStore } from "@/stores/Auth";
import { useMainStore } from "@/stores/Main";
import { useUserStore } from "@/stores/User";
import type { NotificationItem } from "@/types/Notification";

const props = withDefaults(
  defineProps<{
    showMeta?: boolean;
  }>(),
  {
    showMeta: true,
  }
);

const desktopUtils = useDesktopUtils();
const authStore = useAuthStore();
const mainStore = useMainStore();
const userStore = useUserStore();
const connectionStore = useConnectionStore();
const router = useRouter();
const route = useRoute();

const isProfileSearch = ref(false);
const isProfileSearchClicked = ref(false);
const isPluginsDropdownOpen = ref(false);
const isNotificationsOpen = ref(false);
const selectedNotification = ref<Partial<NotificationItem> | null>(null);

const config = useRuntimeConfig();

const userData = computed(() => {
  return (
    getUserDataFromJwt(authStore.getAccessToken?.token) ||
    (userStore.getIsLogged ? userStore.getUser : null)
  );
});

const userAvatar = computed(() => {
  const avatar =
    userData.value?.avatar || (userStore.avatar ? userStore.avatar : null);
  if (!avatar) return null;
  return avatar.startsWith("http") ? avatar : `${config.public.API_URL}${avatar}`;
});

const userInitials = computed(() => {
  const rawName =
    userData.value?.username ||
    userData.value?.name ||
    userData.value?.firstName ||
    (userStore.username ? userStore.username : "");
  if (!rawName || typeof rawName !== "string") return "";
  const parts = rawName.trim().split(/\s+/).filter(Boolean);
  if (!parts.length) return "";
  if (parts.length === 1) return parts[0].slice(0, 2).toUpperCase();
  return `${parts[0][0]}${parts[1][0]}`.toUpperCase();
});

const notificationCount = computed(() => {
  return mainStore.getAllNotifications?.length || 0;
});

const latestNotifications = computed(() => {
  return [...(mainStore.getAllNotifications || [])]
    .sort((a: NotificationItem, b: NotificationItem) => (b.id || 0) - (a.id || 0))
    .slice(0, 6);
});

const handleClickAccount = () => {
  if (authStore.getAccessToken?.token || userStore.getIsLogged) {
    router.push("/account");
  } else {
    desktopUtils.setUserModal(true);
  }
};

const togglePluginsDropdown = () => {
  isPluginsDropdownOpen.value = !isPluginsDropdownOpen.value;
  if (isPluginsDropdownOpen.value) {
    isProfileSearchClicked.value = false;
    isNotificationsOpen.value = false;
  }
};

const toggleNotifications = () => {
  const nextOpen = !isNotificationsOpen.value;
  isNotificationsOpen.value = nextOpen;
  if (nextOpen) {
    isProfileSearchClicked.value = false;
    isPluginsDropdownOpen.value = false;
  } else {
    selectedNotification.value = null;
  }
};

const clearNotifications = () => {
  mainStore.removeAllNotifications();
  selectedNotification.value = null;
};

const handleNotificationClick = (notification: Partial<NotificationItem>) => {
  selectedNotification.value = notification;
};

const closeNotificationDetails = () => {
  selectedNotification.value = null;
};

const runNotificationAction = () => {
  if (selectedNotification.value && typeof selectedNotification.value.buttonFunction === "function") {
    selectedNotification.value.buttonFunction();
  }
  closeNotificationDetails();
};

const handleClickOutside = (event: MouseEvent) => {
  const target = event.target as HTMLElement | null;
  if (!target) return;

  if (!target.closest("#navbarProfiles") && !target.closest(".topbar-profile-switcher")) {
    isProfileSearchClicked.value = false;
  }

  if (!target.closest(".topbar-popover") && !target.closest(".notif-trigger")) {
    isNotificationsOpen.value = false;
    selectedNotification.value = null;
  }

  if (!target.closest(".plugins-dropdown") && !target.closest(".plugin-trigger")) {
    isPluginsDropdownOpen.value = false;
  }
};

onMounted(() => {
  document.addEventListener("click", handleClickOutside);
});

onBeforeUnmount(() => {
  document.removeEventListener("click", handleClickOutside);
});

watch(
  () => route.fullPath,
  () => {
    isProfileSearchClicked.value = false;
    isNotificationsOpen.value = false;
    isPluginsDropdownOpen.value = false;
    closeNotificationDetails();
  }
);
</script>

<style lang="scss" scoped>
.topbar-actions-root {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 0.95rem;
}

.topbar-meta {
  display: flex;
  align-items: center;
  gap: 0.85rem;
  min-width: 9.75rem;
}

.topbar-actions {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  position: relative;
}

.topbar-profile-switcher {
  margin-bottom: 0;
}

.topbar-square-btn {
  width: 2.15rem;
  height: 2.15rem;
  border-radius: 0.55rem;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background2));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  position: relative;
  transition: border-color 0.2s ease, background 0.2s ease;
}

.topbar-square-btn:hover {
  background: hsl(var(--hover));
  border-color: hsl(var(--border2) / 0.85);
}

.topbar-popover {
  position: absolute;
  right: 0;
  top: calc(100% + 0.45rem);
  z-index: 140;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background2));
  border-radius: 0.7rem;
  box-shadow: 0 14px 30px hsl(var(--text) / 0.16);
  padding: 0.45rem;
}

.topbar-popover-title {
  font-size: 0.68rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: hsl(var(--text3));
  margin-bottom: 0.4rem;
}

.topbar-empty-state {
  padding: 0.65rem 0.45rem;
  font-size: 0.78rem;
  color: hsl(var(--text3));
}

.notif-dropdown {
  width: 20.5rem;
  max-height: 21rem;
  overflow: hidden;
}

.notif-dropdown--expanded {
  width: min(42rem, calc(100vw - 1.25rem));
}

.notif-content {
  display: grid;
  grid-template-columns: 1fr;
  min-height: 19.2rem;
}

.notif-dropdown--expanded .notif-content {
  grid-template-columns: 19rem minmax(16rem, 1fr);
}

.notif-list-pane {
  min-width: 0;
  padding-right: 0.25rem;
}

.notif-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 0.25rem;
}

.notif-clear-btn {
  border: none;
  background: transparent;
  color: hsl(var(--editor-accent));
  font-size: 0.75rem;
  font-weight: 700;
}

.notif-list {
  display: flex;
  flex-direction: column;
  gap: 0.28rem;
  max-height: 16rem;
  overflow-y: auto;
  padding-right: 0.1rem;
}

.notif-item {
  width: 100%;
  text-align: left;
  border: 1px solid hsl(var(--border2));
  background: hsl(var(--background));
  border-radius: 0.55rem;
  padding: 0.5rem 0.58rem;
  transition: border-color 0.2s ease, background 0.2s ease;
}

.notif-item:hover {
  border-color: hsl(var(--border2) / 0.85);
  background: hsl(var(--hover));
}

.notif-item--active {
  border-color: hsl(var(--editor-accent) / 0.6);
  background: hsl(var(--editor-accent) / 0.08);
}

.notif-item-title {
  font-size: 0.76rem;
  font-weight: 700;
  color: hsl(var(--text));
  line-height: 1.2;
}

.notif-item-message {
  margin-top: 0.2rem;
  font-size: 0.72rem;
  color: hsl(var(--text3));
  line-height: 1.25;
  max-height: 2.35em;
  overflow: hidden;
}

.notif-dot {
  position: absolute;
  top: -0.2rem;
  right: -0.24rem;
  min-width: 1.05rem;
  height: 1.05rem;
  padding: 0 0.2rem;
  border-radius: 999px;
  background: hsl(var(--editor-accent));
  color: white;
  font-size: 0.58rem;
  font-weight: 700;
  line-height: 1.05rem;
  text-align: center;
}

.notif-detail-pane {
  min-width: 0;
  display: flex;
  flex-direction: column;
  border-left: 1px solid hsl(var(--border2));
  padding-left: 0.6rem;
}

.notif-detail-header {
  padding: 0.2rem 0.3rem 0.75rem;
  border-bottom: 1px solid hsl(var(--border2));
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.7rem;
}

.notif-detail-eyebrow {
  margin: 0 0 0.2rem;
  font-size: 0.65rem;
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: hsl(var(--text3));
}

.notif-detail-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 700;
  color: hsl(var(--text));
  line-height: 1.2;
}

.notif-detail-close {
  width: 2rem;
  height: 2rem;
  border: 1px solid hsl(var(--border2));
  border-radius: 0.55rem;
  background: hsl(var(--background));
  color: hsl(var(--text2));
}

.notif-detail-close:hover {
  background: hsl(var(--hover));
}

.notif-detail-body {
  flex: 1;
  padding: 0.85rem 0.3rem;
  overflow-y: auto;
}

.notif-detail-message {
  margin: 0 0 1rem;
  color: hsl(var(--text2));
  font-size: 0.87rem;
  line-height: 1.45;
  white-space: pre-wrap;
}

.notif-detail-meta {
  display: flex;
  flex-direction: column;
  gap: 0.55rem;
}

.notif-detail-meta-item {
  border: 1px solid hsl(var(--border2));
  border-radius: 0.6rem;
  background: hsl(var(--background));
  padding: 0.55rem 0.65rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.6rem;
}

.notif-detail-meta-label {
  font-size: 0.7rem;
  font-weight: 700;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: hsl(var(--text3));
}

.notif-detail-meta-value {
  font-size: 0.8rem;
  color: hsl(var(--text));
  font-weight: 600;
}

.notif-detail-actions {
  padding: 0.8rem 0.3rem 0.6rem;
  border-top: 1px solid hsl(var(--border2));
  display: flex;
  justify-content: flex-end;
  gap: 0.55rem;
}

.notif-detail-btn {
  border: 1px solid hsl(var(--border2));
  border-radius: 0.56rem;
  background: hsl(var(--background));
  color: hsl(var(--text2));
  font-size: 0.8rem;
  font-weight: 600;
  padding: 0.48rem 0.75rem;
}

.notif-detail-btn--primary {
  border-color: hsl(var(--editor-accent));
  background: linear-gradient(180deg, hsl(var(--editor-accent) / 0.85), hsl(var(--editor-accent)));
  color: #fff;
}

.avatar-chip {
  width: 2.15rem;
  height: 2.15rem;
  border-radius: 999px;
  background: hsl(var(--text2));
  color: hsl(var(--background));
  display: inline-flex;
  align-items: center;
  justify-content: center;
  font-size: 0.7rem;
  font-weight: 700;
  overflow: hidden;
}

.avatar-chip-btn {
  border: 1px solid hsl(var(--border2));
  transition: filter 0.2s ease;
}

.avatar-chip-btn:hover {
  filter: brightness(1.03);
}
</style>
