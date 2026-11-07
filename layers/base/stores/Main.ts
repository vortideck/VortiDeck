import { type NotificationInput, type NotificationItem } from "@base/types/Notification";
import { defineStore } from "pinia";
import { emit } from '@tauri-apps/api/event';
// Note: @tauri-store/pinia is configured via Nuxt plugin (plugins/pinia.ts)

export type UpdateItem = {
  update: {
    url: string;
    version: string; // Clean the version number for Tauri
    notes: string;
    pub_date: string;
    signature: string; // Ensure signature is included
    body?: string;
  };

  autoUpdate: boolean;
};

export type WorkflowApprovalPresentation = "notification" | "dialog";

export const useMainStore = defineStore("mainStore", {
  state: () => {
    return {
      // all these properties will have their type inferred automatically
      initialized: false,
      notifications: [] as NotificationItem[],
      openNotifications: [] as NotificationItem[],
      theme: "dark",
      colorMode: "dark" as "light" | "dark" | "system",
      themePalette: "vortideck",
      update: {} as UpdateItem,
      dontShowReleaseNotes: false,
      workflowApprovalPresentation: "notification" as WorkflowApprovalPresentation,
      nativeWorkflowApprovalNotifications: false,
    };
  },
  getters: {
    getOpenNotifications: (state) => {
      return state.openNotifications;
    },
    getOpenNotificationsForWindow: (state) => (windowId: string) => {
      const matches = (target: string | undefined, current: string) => {
        if (!target) return true;
        if (target === current) return true;
        const isSim = (s: string) => s === "simulator" || s === "mobile_simulator";
        return isSim(target) && isSim(current);
      };
      return state.openNotifications.filter((notification) =>
        matches(notification.windowId, windowId),
      );
    },
    getAllNotifications: (state) => {
      return state.notifications;
    },
    getTheme: (state) => {
      return state.theme;
    },
    getColorMode: (state) => {
      return state.colorMode || "dark";
    },
    getThemePalette: (state) => {
      return state.themePalette || "vortideck";
    },
    getUpdate: (state) => {
      return state.update;
    },
    getDontShowReleaseNotes: (state) => {
      return state.dontShowReleaseNotes;
    },
    getWorkflowApprovalPresentation: (state) => state.workflowApprovalPresentation,
    getNativeWorkflowApprovalNotifications: (state) =>
      state.nativeWorkflowApprovalNotifications,
  },
  actions: {
    addNotification(notification: NotificationInput, windowIds?: string | string[]) {
      const targetWindows = windowIds
        ? (Array.isArray(windowIds) ? windowIds : [windowIds])
        : [undefined];

      for (const windowId of targetWindows) {
        const existing = notification.key
          ? this.notifications.find((item) =>
              item.key === notification.key && item.windowId === windowId
            )
          : undefined;
        if (existing) {
          Object.assign(existing, {
            title: notification.title ?? "",
            message: notification.message,
            type: notification.type,
            buttonText: notification.buttonText ?? "",
            buttonFunction: notification.buttonFunction,
            autoClose: notification.autoClose ?? true,
            duration: notification.duration ?? notification.timeout ?? 5000,
            color: notification.color ?? "",
          });
          if (!this.openNotifications.some((item) => item.id === existing.id)) {
            this.openNotifications.push(existing);
          }
          continue;
        }

        const nextId = this.notifications.reduce((max, item) => Math.max(max, item.id), 0) + 1;
        const storedNotification: NotificationItem = {
          id: nextId,
          ...(notification.key ? { key: notification.key } : {}),
          title: notification.title ?? "",
          message: notification.message,
          type: notification.type,
          buttonText: notification.buttonText ?? "",
          buttonFunction: notification.buttonFunction,
          autoClose: notification.autoClose ?? true,
          duration: notification.duration ?? notification.timeout ?? 5000,
          color: notification.color ?? "",
          ...(windowId ? { windowId } : {}),
        };

        this.notifications.push(storedNotification);
        if (this.openNotifications.length >= 3) {
          this.openNotifications.shift();
        }
        this.openNotifications.push(storedNotification);
      }

      if (typeof window !== "undefined") {
        try {
          const serializable: NotificationInput = {
            ...notification,
            buttonFunction: undefined,
          };
          emit("vortideck:cross-window-notification", {
            notification: serializable,
            windowIds,
          }).catch(() => {});
        } catch {
          // ignore outside Tauri or during SSR
        }
      }
    },
    closeNotification(id: number) {
      // remove from open notifications
      console.log("closeNotification", id);
      this.openNotifications = this.openNotifications.filter((notification) => notification.id !== id);
    },
    removeNotification(id: number) {
      this.openNotifications = this.openNotifications.filter((notification) => notification.id !== id);
      this.notifications = this.notifications.filter((notification) => notification.id !== id);
    },
    removeNotificationByKey(key: string) {
      this.openNotifications = this.openNotifications.filter(
        (notification) => notification.key !== key,
      );
      this.notifications = this.notifications.filter(
        (notification) => notification.key !== key,
      );
    },
    removeNotificationsByKeyPrefix(prefix: string, except: string[] = []) {
      const keep = new Set(except);
      const shouldRemove = (notification: NotificationItem) =>
        Boolean(notification.key?.startsWith(prefix) && !keep.has(notification.key));
      this.openNotifications = this.openNotifications.filter((notification) => !shouldRemove(notification));
      this.notifications = this.notifications.filter((notification) => !shouldRemove(notification));
    },
    removeAllNotifications() {
      this.openNotifications = [];
      this.notifications = [];
    },
    removeAllNotificationsOfType(type: string) {
      this.notifications = this.notifications.filter((notification) => notification.type !== type);
      this.openNotifications = this.openNotifications.filter((notification) => notification.type !== type);
    },
    async setColorMode(mode: 'light' | 'dark' | 'system') {
      this.colorMode = mode;
      try {
        localStorage.setItem('vortideck_color_mode', mode);
      } catch {
        // ignore
      }
      try {
        await emit('theme-settings-changed', { colorMode: mode, themePalette: this.themePalette });
      } catch (err) {
        // ignore
      }
    },
    async setThemePalette(palette: string) {
      this.themePalette = palette;
      try {
        localStorage.setItem('vortideck_theme_palette', palette);
      } catch {
        // ignore
      }
      try {
        await emit('theme-settings-changed', { colorMode: this.colorMode, themePalette: palette });
      } catch (err) {
        // ignore
      }
    },
    async setTheme(newTheme: string) {
      if (this.theme === newTheme) return;
      const html = document.documentElement;
      html.classList.add('disable-transitions');

      this.theme = newTheme;
      try {
        localStorage.setItem('vortideck_theme', newTheme);
      } catch {
        // ignore
      }

      try {
        await emit('theme-changed', { theme: newTheme });
      } catch (err) {
        console.error('Failed to emit theme-changed event:', err);
      }

      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          html.classList.remove('disable-transitions');
        });
      });
    },
    async toggleTheme() {
      const isCurrentlyDark = typeof document !== 'undefined'
        ? document.documentElement.classList.contains('dark')
        : (this.colorMode === 'dark');
      const nextMode: 'light' | 'dark' = isCurrentlyDark ? 'light' : 'dark';
      await this.setColorMode(nextMode);
    },
    setUpdate(update: UpdateItem) {
      this.update = update;
    },
    setDontShowReleaseNotes(show: boolean) {
      this.dontShowReleaseNotes = show;
    },
    setWorkflowApprovalPresentation(presentation: WorkflowApprovalPresentation) {
      this.workflowApprovalPresentation = presentation;
    },
    setNativeWorkflowApprovalNotifications(enabled: boolean) {
      this.nativeWorkflowApprovalNotifications = enabled;
    },
  },
  persist: {
    storage: sessionStorage,
    // Notifications contain runtime callbacks and represent transient UI. Durable
    // requests (for example workflow approvals) are rehydrated from their own
    // authoritative repository instead.
    pick: [
      "theme",
      "colorMode",
      "themePalette",
      "update",
      "dontShowReleaseNotes",
      "workflowApprovalPresentation",
      "nativeWorkflowApprovalNotifications",
    ],
  },
  tauri: {
    // Cross-window state is serialized too, so callback-bearing UI must stay
    // local to the window that created it.
    filterKeys: ["notifications", "openNotifications"],
    filterKeysStrategy: "omit",
  },
  // other options...
});
