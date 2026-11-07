<template>
  <BaseModal
    :is-open="isVisible"
    overlay-class="permission-dialog-overlay"
    @close="handleOverlayClick"
  >
    <div class="permission-dialog vd-modal-card">
      <!-- Header -->
      <div class="dialog-header">
        <div class="icon">🔒</div>
        <h2>Permission Request</h2>
      </div>

      <!-- Content -->
      <div class="dialog-content">
        <p class="plugin-name">
          <strong>{{ pluginName }}</strong> is requesting the following permission:
        </p>

        <div class="permission-card">
          <div class="permission-icon">{{ getPermissionIcon(permission) }}</div>
          <div class="permission-info">
            <h3>{{ getPermissionLabel(permission) }}</h3>
            <p>{{ getPermissionDescription(permission) }}</p>
          </div>
        </div>

        <div v-if="reason" class="permission-reason">
          <strong>Why this plugin needs it:</strong>
          <p>{{ reason }}</p>
        </div>

        <div class="warning-message">
          <span class="warning-icon">⚠️</span>
          <p>Only grant permissions to plugins you trust.</p>
        </div>
      </div>

      <!-- Actions -->
      <div class="dialog-actions">
        <button @click="deny" class="btn btn-secondary">
          Deny
        </button>
        <button @click="allow" class="btn btn-primary">
          Allow
        </button>
      </div>
    </div>
  </BaseModal>
</template>

<script setup lang="ts">
import BaseModal from "../Modal/BaseModal.vue";
import { ref } from 'vue';

interface Props {
  pluginId: string;
  pluginName: string;
  permission: string;
  reason?: string;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  allow: [pluginId: string, permission: string];
  deny: [pluginId: string, permission: string];
  close: [];
}>();

const isVisible = ref(true);

function allow() {
  emit('allow', props.pluginId, props.permission);
  isVisible.value = false;
  emit('close');
}

function deny() {
  emit('deny', props.pluginId, props.permission);
  isVisible.value = false;
  emit('close');
}

function handleOverlayClick() {
  // Clicking overlay denies the permission
  deny();
}

function getPermissionIcon(permission: string): string {
  const icons: Record<string, string> = {
    dom: '🎨',
    css: '🎨',
    network: '🌐',
    filesystem: '📁',
    executecommands: '⚡',
    accesscamera: '📷',
    accessmicrophone: '🎤',
    flows: '🔄',
    storage: '💾',
  };
  return icons[permission.toLowerCase()] || '🔐';
}

function getPermissionLabel(permission: string): string {
  const labels: Record<string, string> = {
    dom: 'UI Rendering',
    css: 'CSS Styling',
    network: 'Network Access',
    filesystem: 'File System Access',
    executecommands: 'Command Execution',
    accesscamera: 'Camera Access',
    accessmicrophone: 'Microphone Access',
    flows: 'Flow Execution',
    storage: 'Local Storage',
  };
  return labels[permission.toLowerCase()] || permission;
}

function getPermissionDescription(permission: string): string {
  const descriptions: Record<string, string> = {
    dom: 'Display UI elements and custom components',
    css: 'Inject custom CSS styles into the app',
    network: 'Make HTTP requests to external services',
    filesystem: 'Read and write files (scoped to plugin folder)',
    executecommands: 'Execute system commands on your computer',
    accesscamera: 'Access your camera for capturing images or video',
    accessmicrophone: 'Access your microphone for audio recording',
    flows: 'Run and interact with flows in your workspace',
    storage: 'Save and retrieve plugin-specific data locally',
  };
  return descriptions[permission.toLowerCase()] || 'Access system resources';
}
</script>

<style scoped lang="scss">
@use "../Modal/modalTheme.scss" as *;

.permission-dialog-overlay {
  z-index: 13000;
}

.permission-dialog {
  max-width: 30rem;
  width: 100%;
  overflow: hidden;
}

.dialog-header {
  background: linear-gradient(
    100deg,
    hsl(var(--editor-accent, 20 96% 55%) / 0.18),
    hsl(var(--editor-accent-2, 20 95% 62%) / 0.12)
  );
  color: hsl(var(--text));
  padding: 1.5rem;
  text-align: center;
  border-bottom: 1px solid hsl(var(--border2) / 0.72);
}

.dialog-header .icon {
  font-size: 3rem;
  margin-bottom: 0.5rem;
}

.dialog-header h2 {
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
}

.dialog-content {
  padding: 1.5rem;
}

.plugin-name {
  margin: 0 0 1.5rem 0;
  color: hsl(var(--text2));
  font-size: 1rem;
}

.plugin-name strong {
  color: hsl(var(--text));
  font-weight: 600;
}

.permission-card {
  display: flex;
  gap: 1rem;
  padding: 1rem;
  background: hsl(var(--background3) / 0.52);
  border: 1px solid hsl(var(--border2));
  border-radius: 8px;
  margin-bottom: 1rem;
}

.permission-icon {
  font-size: 2.5rem;
  flex-shrink: 0;
}

.permission-info h3 {
  margin: 0 0 0.5rem 0;
  font-size: 1.125rem;
  font-weight: 600;
  color: hsl(var(--text));
}

.permission-info p {
  margin: 0;
  font-size: 0.875rem;
  color: hsl(var(--text2));
  line-height: 1.5;
}

.permission-reason {
  padding: 1rem;
  background: hsl(var(--background3) / 0.64);
  border-left: 4px solid hsl(var(--editor-accent) / 0.92);
  border-radius: 4px;
  margin-bottom: 1rem;
}

.permission-reason strong {
  display: block;
  margin-bottom: 0.5rem;
  color: hsl(var(--text));
  font-size: 0.875rem;
}

.permission-reason p {
  margin: 0;
  color: hsl(var(--text2));
  font-size: 0.875rem;
  line-height: 1.5;
}

.warning-message {
  display: flex;
  align-items: start;
  gap: 0.75rem;
  padding: 0.75rem;
  background: rgba(245, 158, 11, 0.13);
  border: 1px solid rgba(245, 158, 11, 0.28);
  border-radius: 6px;
}

.warning-icon {
  font-size: 1.25rem;
  flex-shrink: 0;
}

.warning-message p {
  margin: 0;
  color: hsl(var(--text));
  font-size: 0.875rem;
  font-weight: 500;
}

.dialog-actions {
  display: flex;
  gap: 0.75rem;
  padding: 1.5rem;
  background: hsl(var(--background2) / 0.86);
  border-top: 1px solid hsl(var(--border2) / 0.72);
}

.btn {
  flex: 1;
  padding: 0.75rem 1.5rem;
  border: none;
  border-radius: 6px;
  font-size: 1rem;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.btn-secondary {
  background: hsl(var(--background2));
  color: hsl(var(--text));
  border: 1px solid hsl(var(--border2));
}

.btn-secondary:hover {
  background: hsl(var(--background3) / 0.72);
  border-color: hsl(var(--border2));
}

.btn-primary {
  border: none;
  color: #fff;
  background: linear-gradient(
    100deg,
    hsl(var(--editor-accent, 20 96% 55%) / 1),
    hsl(var(--editor-accent-2, 20 95% 62%) / 1)
  );
  box-shadow: 0 14px 24px -18px rgba(251, 113, 63, 0.78);
  transition: transform 0.2s ease, filter 0.2s ease;
}

.btn-primary:hover {
  transform: translateY(-1px);
}

.btn-primary:active {
  transform: translateY(0);
}
</style>
