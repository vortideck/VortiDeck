/**
 * Global Vue directives plugin
 * Auto-registers custom directives for the flow layer
 */

export default defineNuxtPlugin((nuxtApp) => {
  // Auto-focus directive - focuses input when it mounts
  nuxtApp.vueApp.directive('auto-focus', {
    mounted(el: HTMLElement) {
      (el as HTMLInputElement).focus();
    },
  });
});
