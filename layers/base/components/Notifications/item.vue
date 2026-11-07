<template lang="pug">
div(
  class="notification-item"
  :class="[`notification-${notification.type}`, 'animate-slide-in']"
)
  div(class="notification-content")
    // Simple colored indicator dot
    div(class="notification-indicator" :class="`indicator-${notification.type}`")
    
    // Content section
    div(class="notification-body")
      div(class="notification-header")
        h4(class="notification-title") {{ notification.title }}
        button(
          class="notification-close"
          @click="mainStore.closeNotification(notification.id)"
        )
          PixelarticonsClose(class="close-icon")
      
      p(class="notification-message" :class="notification.buttonText ? 'has-button' : ''") {{ notification.message }}
      
      // Action button (if present)
      button(
        v-if="notification.buttonText"
        @click="notification.buttonFunction()"
        class="notification-button"
        :class="`button-${notification.type}`"
      ) {{ notification.buttonText }}
  
  // Simple progress bar for auto-close
  div(
    v-if="notification.autoClose"
    class="notification-progress-container"
  )
    div(
      class="notification-progress"
      :class="`progress-${notification.type}`"
      :style="{ width: progressWidth + '%' }"
    )

</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, type PropType } from 'vue'
import { useMainStore } from "@base/stores/Main";
import PixelarticonsClose from "~icons/pixelarticons/close?width=24px&height=24px";
import { type NotificationItem } from "@base/types/Notification";

const mainStore = useMainStore();

const props = defineProps({
    notification: {
        type: Object as PropType<NotificationItem>,
        required: true,
    }
});

const progressWidth = ref(100);
const progressInterval = ref<NodeJS.Timeout | null>(null);

onMounted(() => {
  if (props.notification?.autoClose) {
    const duration = props.notification.duration || 5000;
    const interval = 50; // Update every 50ms for smooth animation
    const decrement = (100 / duration) * interval;

    progressInterval.value = setInterval(() => {
      progressWidth.value -= decrement;
      if (progressWidth.value <= 0) {
        if (progressInterval.value) {
          clearInterval(progressInterval.value);
        }
        mainStore.closeNotification(props.notification.id);
      }
    }, interval);
  }
});

onUnmounted(() => {
  if (progressInterval.value) {
    clearInterval(progressInterval.value);
  }
});
</script>

<style lang="scss">
@keyframes notification-slide-in {
  0% {
    transform: translateX(100%);
    opacity: 0;
  }
  100% {
    transform: translateX(0);
    opacity: 1;
  }
}

.animate-slide-in {
  animation: notification-slide-in 0.25s cubic-bezier(0.16, 1, 0.3, 1);
}

.notification-item {
  width: 100%;
  max-width: 24rem;
  height: auto;
  border-radius: 0.75rem;
  overflow: hidden;
  transition: all 0.2s ease;
  position: relative;
  background: hsl(var(--background2) / 0.95);
  border: 1px solid hsl(var(--border2));
  backdrop-filter: blur(16px);
  box-shadow: 0 10px 30px -5px rgba(0, 0, 0, 0.4), 0 4px 12px -2px rgba(0, 0, 0, 0.2);

  &:hover {
    border-color: hsl(var(--border));
    box-shadow: 0 14px 35px -5px rgba(0, 0, 0, 0.5);
  }

  &.notification-success {
    border-left: 4px solid #10b981;
  }

  &.notification-error {
    border-left: 4px solid #ef4444;
  }

  &.notification-warning {
    border-left: 4px solid #f59e0b;
  }

  &.notification-info {
    border-left: 4px solid #3b82f6;
  }

  &.notification-logout {
    border-left: 4px solid hsl(var(--button));
  }
}

.notification-content {
  display: flex;
  align-items: flex-start;
  padding: 0.875rem 1rem;
  gap: 0.75rem;
}

.notification-indicator {
  flex-shrink: 0;
  width: 0.5rem;
  height: 0.5rem;
  border-radius: 50%;
  margin-top: 0.375rem;

  &.indicator-success {
    background-color: #10b981;
  }

  &.indicator-error {
    background-color: #ef4444;
  }

  &.indicator-warning {
    background-color: #f59e0b;
  }

  &.indicator-info {
    background-color: #3b82f6;
  }

  &.indicator-logout {
    background-color: hsl(var(--button));
  }
}

.notification-body {
  flex: 1;
  min-width: 0;
}

.notification-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-bottom: 0.25rem;
}

.notification-title {
  font-size: 0.875rem;
  font-weight: 600;
  color: hsl(var(--text));
  margin: 0;
  line-height: 1.25;
}

.notification-close {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 1.5rem;
  height: 1.5rem;
  border-radius: 0.375rem;
  border: 1px solid transparent;
  background: transparent;
  color: hsl(var(--text3));
  cursor: pointer;
  transition: all 0.15s ease;
  flex-shrink: 0;

  &:hover {
    color: hsl(var(--text));
    background-color: hsl(var(--hover));
    border-color: hsl(var(--border));
  }

  .close-icon {
    width: 0.875rem;
    height: 0.875rem;
  }
}

.notification-message {
  font-size: 0.8125rem;
  color: hsl(var(--text2));
  line-height: 1.4;
  margin: 0;

  &.has-button {
    margin-bottom: 0.75rem;
  }
}

.notification-button {
  width: 100%;
  padding: 0.5rem 0.875rem;
  border-radius: 0.5rem;
  font-size: 0.8125rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s ease;
  background: hsl(var(--button));
  color: #ffffff;
  border: 1px solid hsl(var(--buttonHover));
  display: inline-flex;
  align-items: center;
  justify-content: center;

  &:hover {
    background: hsl(var(--buttonHover));
    box-shadow: 0 4px 12px hsl(var(--button) / 0.35);
    transform: translateY(-1px);
  }

  &:active {
    transform: scale(0.98);
  }

  &.button-success {
    background-color: #10b981;
    border-color: #059669;
    &:hover { background-color: #059669; }
  }

  &.button-error {
    background-color: #ef4444;
    border-color: #dc2626;
    &:hover { background-color: #dc2626; }
  }

  &.button-warning {
    background-color: #f59e0b;
    border-color: #d97706;
    &:hover { background-color: #d97706; }
  }

  &.button-info,
  &.button-logout {
    background-color: hsl(var(--button));
    border-color: hsl(var(--buttonHover));
    &:hover { background-color: hsl(var(--buttonHover)); }
  }
}

.notification-progress-container {
  height: 2px;
  background-color: hsl(var(--border) / 0.5);
  position: relative;
  overflow: hidden;
}

.notification-progress {
  position: absolute;
  left: 0;
  top: 0;
  height: 100%;
  transition: width 0.1s ease-linear;

  &.progress-success {
    background-color: #10b981;
  }

  &.progress-error {
    background-color: #ef4444;
  }

  &.progress-warning {
    background-color: #f59e0b;
  }

  &.progress-info {
    background-color: #3b82f6;
  }

  &.progress-logout {
    background-color: hsl(var(--button));
  }
}
</style>
