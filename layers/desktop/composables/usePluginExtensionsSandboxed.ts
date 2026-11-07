/**
 * Plugin Extensions Helper
 *
 * Helper functions for loading plugin web extensions.
 * Used by plugin-window.vue for iframe-based sandboxing.
 */

import { invoke } from '@tauri-apps/api/core';

interface PluginWebExtension {
  plugin_id: string;
  file_type: 'js' | 'css' | 'html';
  content: string;
  file_name?: string;
}

/**
 * Plugin extension helper composable
 */
export function usePluginExtensionsSandboxed() {
  /**
   * Get web extension HTML for a specific plugin
   * Returns combined HTML/CSS/JS ready to render
   */
  async function getPluginWebExtension(pluginId: string): Promise<string> {
    const extensions = await invoke<PluginWebExtension[]>('get_plugin_web_extensions', {
      pluginId,
    });

    let htmlContent = '';
    let cssContent = '';
    let jsContent = '';

    for (const ext of extensions) {
      if (ext.plugin_id === pluginId) {
        if (ext.file_type === 'html') {
          htmlContent = ext.content;
        } else if (ext.file_type === 'css') {
          cssContent = ext.content;
        } else if (ext.file_type === 'js') {
          jsContent = ext.content;
        }
      }
    }

    // Build complete web extension HTML (just raw content, no wrappers)
    let result = '';
    if (htmlContent) {
      result = htmlContent;
    }
    if (cssContent) {
      result += `\n<style>${cssContent}</style>`;
    }
    if (jsContent) {
      result += `\n<script>${jsContent}</script>`;
    }

    if (!result) {
      throw new Error(`No web extensions found for plugin: ${pluginId}`);
    }

    return result;
  }

  return {
    getPluginWebExtension,
  };
}
