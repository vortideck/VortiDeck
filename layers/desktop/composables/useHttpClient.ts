import { invoke } from '@tauri-apps/api/core'

export interface HttpRequest {
  method: 'GET' | 'POST' | 'PUT' | 'DELETE' | 'PATCH'
  url: string
  headers?: Record<string, string>
  body?: any
  timeoutMs?: number
}

export interface HttpResponse {
  status: number
  headers: Record<string, string>
  body: string
  elapsed_ms: number
}

/**
 * Generic HTTP client composable - VortiDeck's answer to n8n's HTTP Request node
 * Use this for ANY API call instead of spawning curl processes
 */
export const useHttpClient = () => {
  /**
   * Generic HTTP request - works with any API
   * @example
   * const response = await request({
   *   method: 'POST',
   *   url: 'https://api.twitch.tv/helix/chat/messages',
   *   headers: {
   *     'Client-ID': 'your-client-id',
   *     'Authorization': 'Bearer token'
   *   },
   *   body: { broadcaster_id: '123', message: 'Hello!' }
   * })
   */
  const request = async (req: HttpRequest): Promise<HttpResponse> => {
    return await invoke('http_request', {
      method: req.method,
      url: req.url,
      headers: req.headers,
      body: req.body,
      timeoutMs: req.timeoutMs,
    })
  }

  /**
   * Send webhook with automatic retry
   * @example
   * await sendWebhook('https://discord.com/api/webhooks/...', {
   *   content: 'Stream starting!'
   * })
   */
  const sendWebhook = async (
    url: string,
    payload: any,
    retryCount?: number
  ): Promise<string> => {
    return await invoke('send_webhook', {
      url,
      payload,
      retryCount,
    })
  }

  /**
   * Send webhook event with standard format
   * @example
   * await sendWebhookEvent(
   *   'https://example.com/webhook',
   *   'stream_start',
   *   { title: 'Going Live!' }
   * )
   */
  const sendWebhookEvent = async (
    url: string,
    eventType: string,
    data: any
  ): Promise<string> => {
    return await invoke('send_webhook_event', {
      url,
      eventType,
      data,
    })
  }

  /**
   * Convenience: GET request
   * @example
   * const response = await get('https://api.example.com/user/123')
   */
  const get = (url: string, headers?: Record<string, string>) =>
    request({ method: 'GET', url, headers })

  /**
   * Convenience: POST request
   * @example
   * const response = await post('https://api.example.com/webhooks', {
   *   event: 'test'
   * })
   */
  const post = (url: string, body: any, headers?: Record<string, string>) =>
    request({ method: 'POST', url, body, headers })

  /**
   * Convenience: PUT request
   */
  const put = (url: string, body: any, headers?: Record<string, string>) =>
    request({ method: 'PUT', url, body, headers })

  /**
   * Convenience: DELETE request
   */
  const del = (url: string, headers?: Record<string, string>) =>
    request({ method: 'DELETE', url, headers })

  /**
   * Convenience: PATCH request
   */
  const patch = (url: string, body: any, headers?: Record<string, string>) =>
    request({ method: 'PATCH', url, body, headers })

  return {
    request,
    sendWebhook,
    sendWebhookEvent,
    get,
    post,
    put,
    delete: del,
    patch,
  }
}

/**
 * Parse JSON response body safely
 */
export function parseResponseJson<T = any>(response: HttpResponse): T {
  try {
    return JSON.parse(response.body)
  } catch (e) {
    throw new Error(`Failed to parse JSON response: ${e}`)
  }
}
