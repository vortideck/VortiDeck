export default defineNuxtPlugin(async (nuxtApp) => {
  // Only run on client side
  if (typeof window !== 'undefined') {
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      
      // Check if this is a dynamically created window (starts with "window_")
      // and NOT the main window
      const currentWindow = getCurrentWindow()
      const windowLabel = currentWindow.label
      
      if (windowLabel.startsWith('window_') && windowLabel !== 'main') {
        // Show the window when the Nuxt app is ready
        nuxtApp.hook('app:mounted', async () => {
          try {
            await currentWindow.show()
          } catch (error) {
            console.error('Failed to show window:', error)
          }
        })
      }
    } catch (error) {
      console.error('Error in window visibility plugin:', error)
    }
  }
}) 