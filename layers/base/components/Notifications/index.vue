<template lang="pug">
div(
  class="notification-viewport fixed z-[12050] h-auto flex flex-col-reverse pointer-events-none"
  :class="placement === 'top-right' ? 'notification-viewport--top' : 'notification-viewport--bottom'"
  aria-live="polite"
)
    .pointer-events-auto(v-for="notification in notifications" :key="notification.id")
        NotificationsItem(:notification="notification")
</template>

<script setup lang="ts">
import { type NotificationItem } from "../../types/Notification";

withDefaults(defineProps<{
  notifications: NotificationItem[];
  placement?: "top-right" | "bottom-right";
}>(), {
  placement: "bottom-right",
});
</script>

<style lang="scss">
.notification-viewport {
  right: 1.5rem;
  width: 24rem;
  max-width: calc(100vw - 3rem);
  gap: 0.75rem;
}

.notification-viewport--top {
  top: 4.5rem;
}

.notification-viewport--bottom {
  bottom: 1.5rem;
}

// Additional styling for stacked notifications
.notification-stack {
  perspective: 1000px;
}

.notification-stack > div {
  transform-style: preserve-3d;
}

// Animation for notification entrance
@keyframes notification-enter {
  0% {
    transform: translateX(100%) scale(0.8);
    opacity: 0;
  }
  100% {
    transform: translateX(0) scale(1);
    opacity: 1;
  }
}

.notification-enter {
  animation: notification-enter 0.4s ease-out;
}
</style>
