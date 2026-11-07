export default defineNuxtPlugin((nuxtApp) => {
  if (typeof window !== 'undefined') {
    const { initScale, zoomIn, zoomOut, resetZoom } = useAppScale();

    // Initialize scale on app startup
    initScale();

    // Bind global desktop zoom shortcuts: Ctrl +/-/0 or Cmd +/-/0
    window.addEventListener('keydown', (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        if (e.key === '=' || e.key === '+' || e.code === 'NumpadAdd') {
          e.preventDefault();
          zoomIn();
        } else if (e.key === '-' || e.code === 'NumpadSubtract') {
          e.preventDefault();
          zoomOut();
        } else if (e.key === '0' || e.code === 'Numpad0') {
          e.preventDefault();
          resetZoom();
        }
      }
    });

    // Re-assert root font-size on route changes
    nuxtApp.hook('page:finish', () => {
      const { scale } = useAppScale();
      if (scale.value) {
        document.documentElement.style.zoom = '';
        document.documentElement.style.fontSize = `${16 * scale.value}px`;
      }
    });
  }
});
