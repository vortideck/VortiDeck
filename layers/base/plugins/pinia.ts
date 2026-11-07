import type { Pinia } from 'pinia';
import { TauriPluginPinia } from '@tauri-store/pinia';

export default defineNuxtPlugin(({ $pinia }) => {
  if (!("__TAURI_INTERNALS__" in window)) return;
  ($pinia as Pinia).use(TauriPluginPinia());
});
