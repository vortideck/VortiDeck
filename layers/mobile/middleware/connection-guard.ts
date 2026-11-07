import { useConnectionStore } from "@/stores/Connection"

export default defineNuxtRouteMiddleware((to) => {
  // Pages that require WebSocket connection
  const connectionRequiredPages = ['/deck', '/deck/simulator']
  
  // Pages that should work without connection
  const offlinePages = ['/', '/qr', '/settings']
  
  const connectionStore = useConnectionStore()
  const currentStatus = connectionStore.getStatus
  
  // Check if navigating to a page that requires connection
  const requiresConnection = connectionRequiredPages.some(page => 
    to.path.startsWith(page)
  )
  
  if (requiresConnection) {
    // If connection is not established, allow navigation but show appropriate UI state
    if (currentStatus !== 'paired' && currentStatus !== 'connected') {
      console.warn(`⚠️ Navigating to ${to.path} without WebSocket connection`)

      // Don't block navigation - let the page handle the disconnected state
      // The page components should show appropriate "connecting" or "disconnected" UI
      return
    }
  }

  // For all other cases, allow navigation
  console.log(`✅ Navigation to ${to.path} allowed (status: ${currentStatus})`)
}) 