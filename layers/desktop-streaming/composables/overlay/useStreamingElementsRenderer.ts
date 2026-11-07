import { ref, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

export interface ElementRenderConfig {
  id: string;
  type: string;
  position: { x: number; y: number };
  size: { width: number; height: number };
  style: Record<string, any>;
  settings: Record<string, any>;
  dataSources: string[];
  isActive: boolean;
  zIndex?: number;
}

export interface OverlayRenderConfig {
  canvas: {
    width: number;
    height: number;
  };
  elements: ElementRenderConfig[];
  theme: {
    backgroundColor?: string;
    fontFamily?: string;
    primaryColor?: string;
    secondaryColor?: string;
  };
}

export const useStreamingElementsRenderer = () => {
  const loading = ref(false);
  const error = ref<string | null>(null);
  const lastRenderedConfig = ref<OverlayRenderConfig | null>(null);

  // Generate HTML for streaming elements overlay
  const generateOverlayHTML = async (config: OverlayRenderConfig): Promise<string> => {
    loading.value = true;
    error.value = null;

    try {
      console.log('🎨 Generating overlay HTML for streaming elements');
      
      // Base HTML template
      const html = `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>VortiDeck V3 Streaming Overlay</title>
    <style>
        ${generateCSS(config)}
    </style>
    <script src="https://unpkg.com/vue@3/dist/vue.global.js"></script>
</head>
<body>
    <div id="overlay-app">
        ${generateElementsHTML(config.elements)}
    </div>
    
    <script>
        ${generateJavaScript(config)}
    </script>
</body>
</html>`;

      lastRenderedConfig.value = config;
      console.log('✅ Generated overlay HTML successfully');
      return html;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to generate overlay HTML:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  };

  // Generate CSS for overlay
  const generateCSS = (config: OverlayRenderConfig): string => {
    const { canvas, theme } = config;
    
    return `
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            width: ${canvas.width}px;
            height: ${canvas.height}px;
            background: ${theme.backgroundColor || 'transparent'};
            font-family: ${theme.fontFamily || 'Arial, sans-serif'};
            overflow: hidden;
            position: relative;
        }
        
        #overlay-app {
            width: 100%;
            height: 100%;
            position: relative;
        }
        
        .streaming-element {
            position: absolute;
            pointer-events: none;
            transform-origin: top left;
        }
        
        .streaming-element.interactive {
            pointer-events: auto;
        }
        
        /* Element-specific styles */
        .element-stream-info {
            background: rgba(0, 0, 0, 0.8);
            color: white;
            padding: 10px 15px;
            border-radius: 8px;
            backdrop-filter: blur(10px);
        }
        
        .element-viewer-metrics {
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            color: white;
            padding: 8px 12px;
            border-radius: 12px;
            text-align: center;
        }
        
        .element-chat-overlay {
            background: rgba(255, 255, 255, 0.95);
            border-radius: 10px;
            padding: 10px;
            max-height: 300px;
            overflow-y: auto;
            backdrop-filter: blur(5px);
        }
        
        .element-alert-system {
            background: linear-gradient(45deg, #ff6b6b, #ee5a24);
            color: white;
            padding: 15px 20px;
            border-radius: 15px;
            text-align: center;
            box-shadow: 0 4px 20px rgba(255, 107, 107, 0.3);
        }
        
        .element-interactive {
            background: rgba(76, 175, 80, 0.9);
            color: white;
            padding: 12px 16px;
            border-radius: 10px;
            border: 2px solid #4caf50;
        }
        
        .element-social-media {
            background: rgba(33, 150, 243, 0.9);
            color: white;
            padding: 8px 12px;
            border-radius: 8px;
        }
        
        /* Animations */
        @keyframes bounceIn {
            0% { transform: scale(0) rotate(180deg); opacity: 0; }
            50% { transform: scale(1.2) rotate(180deg); opacity: 0.8; }
            100% { transform: scale(1) rotate(0deg); opacity: 1; }
        }
        
        @keyframes slideIn {
            0% { transform: translateX(-100%); opacity: 0; }
            100% { transform: translateX(0); opacity: 1; }
        }
        
        @keyframes pulse {
            0%, 100% { transform: scale(1); }
            50% { transform: scale(1.05); }
        }
        
        @keyframes celebration {
            0%, 100% { transform: rotate(0deg) scale(1); }
            25% { transform: rotate(5deg) scale(1.1); }
            75% { transform: rotate(-5deg) scale(1.1); }
        }
        
        .animate-bounce-in { animation: bounceIn 0.6s ease-out; }
        .animate-slide-in { animation: slideIn 0.5s ease-out; }
        .animate-pulse { animation: pulse 2s infinite; }
        .animate-celebration { animation: celebration 0.8s ease-in-out; }
        
        /* Live data indicators */
        .live-indicator {
            display: inline-block;
            width: 8px;
            height: 8px;
            background: #ff4444;
            border-radius: 50%;
            margin-right: 8px;
            animation: pulse 1s infinite;
        }
        
        .data-loading {
            opacity: 0.6;
            transition: opacity 0.3s ease;
        }
        
        .data-error {
            color: #ff6b6b;
            background: rgba(255, 107, 107, 0.1);
        }
    `;
  };

  // Generate HTML for elements
  const generateElementsHTML = (elements: ElementRenderConfig[]): string => {
    return elements
      .filter(el => el.isActive)
      .sort((a, b) => (a.zIndex || 0) - (b.zIndex || 0))
      .map(element => {
        const style = `
          left: ${element.position.x}px;
          top: ${element.position.y}px;
          width: ${element.size.width}px;
          height: ${element.size.height}px;
          z-index: ${element.zIndex || 1};
          ${Object.entries(element.style).map(([key, value]) => 
            `${key.replace(/([A-Z])/g, '-$1').toLowerCase()}: ${value};`
          ).join(' ')}
        `;
        
        const cssClass = `streaming-element element-${element.type.replace(/([A-Z])/g, '-$1').toLowerCase()}`;
        
        return `
          <div 
            id="element-${element.id}" 
            class="${cssClass}"
            style="${style}"
            data-element-type="${element.type}"
            data-sources="${element.dataSources.join(',')}"
          >
            ${generateElementContent(element)}
          </div>
        `;
      })
      .join('\n');
  };

  // Generate content for specific element types
  const generateElementContent = (element: ElementRenderConfig): string => {
    const { type, settings, id } = element;
    
    switch (type) {
      case 'streamInfo':
        return `
          <div class="stream-info-content">
            <div class="live-status">
              <span class="live-indicator"></span>
              <span v-text="streamData.isLive ? 'LIVE' : 'OFFLINE'">LIVE</span>
            </div>
            <div class="stream-title" v-text="streamData.title">Stream Title</div>
            <div class="stream-game" v-text="streamData.game">Current Game</div>
            <div class="stream-uptime" v-text="streamData.uptime">00:00:00</div>
          </div>
        `;
        
      case 'viewerMetrics':
        return `
          <div class="viewer-metrics-content">
            <div class="metric-item">
              <span class="metric-label">Viewers</span>
              <span class="metric-value" v-text="viewerData.current">0</span>
            </div>
            <div class="metric-item">
              <span class="metric-label">Followers</span>
              <span class="metric-value" v-text="viewerData.followers">0</span>
            </div>
          </div>
        `;
        
      case 'chat':
        return `
          <div class="chat-overlay-content">
            <div class="chat-messages" id="chat-${id}">
              <div v-for="message in chatData.messages" class="chat-message">
                <span class="chat-username" :style="{color: message.color}">{{ message.username }}:</span>
                <span class="chat-text">{{ message.message }}</span>
              </div>
            </div>
          </div>
        `;
        
      case 'alertSystem':
        return `
          <div class="alert-system-content" v-show="alertData.active">
            <div class="alert-title">{{ alertData.type.toUpperCase() }}</div>
            <div class="alert-username">{{ alertData.username }}</div>
            <div class="alert-message" v-if="alertData.message">{{ alertData.message }}</div>
          </div>
        `;
        
      case 'interactive':
        return `
          <div class="interactive-content">
            <div class="poll-question" v-if="interactiveData.type === 'poll'">
              {{ interactiveData.question }}
            </div>
            <div class="poll-options" v-if="interactiveData.type === 'poll'">
              <div v-for="option in interactiveData.options" class="poll-option">
                <span>{{ option.text }}</span>
                <span>{{ option.votes }}%</span>
              </div>
            </div>
          </div>
        `;
        
      case 'socialMedia':
        return `
          <div class="social-media-content">
            <div class="social-links">
              <a v-for="link in socialData.links" :href="link.url" target="_blank">
                {{ link.platform }}
              </a>
            </div>
            <div class="recent-posts" v-if="socialData.recentPost">
              <div class="post-content">{{ socialData.recentPost.content }}</div>
            </div>
          </div>
        `;
        
      default:
        return `
          <div class="custom-element-content">
            <div class="element-placeholder">
              ${element.type} Element
            </div>
          </div>
        `;
    }
  };

  // Generate JavaScript for real-time data
  const generateJavaScript = (config: OverlayRenderConfig): string => {
    return `
      const { createApp } = Vue;
      
      const OverlayApp = {
        data() {
          return {
            streamData: {
              isLive: false,
              title: 'Stream Title',
              game: 'Just Chatting',
              uptime: '00:00:00',
              viewers: 0
            },
            viewerData: {
              current: 0,
              followers: 0,
              subscribers: 0,
              peak: 0
            },
            chatData: {
              messages: []
            },
            alertData: {
              active: false,
              type: '',
              username: '',
              message: ''
            },
            interactiveData: {
              type: 'poll',
              question: '',
              options: []
            },
            socialData: {
              links: [],
              recentPost: null
            }
          };
        },
        mounted() {
          this.initializeWebSocket();
          this.initializeMessageHandlers();
          this.startDataRefresh();
        },
        methods: {
          initializeWebSocket() {
            // WebSocket connection for real-time updates
            if (typeof WebSocket !== 'undefined') {
              try {
                const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                const ws = new WebSocket(protocol + '//' + window.location.host + '/ws/overlay');
                
                ws.onmessage = (event) => {
                  try {
                    const data = JSON.parse(event.data);
                    this.handleStreamingData(data);
                  } catch (err) {
                    console.error('Failed to parse WebSocket message:', err);
                  }
                };
                
                ws.onerror = (error) => {
                  console.error('WebSocket error:', error);
                };
                
                this.websocket = ws;
              } catch (err) {
                console.error('Failed to initialize WebSocket:', err);
              }
            }
          },
          
          initializeMessageHandlers() {
            // PostMessage API for communication with parent window
            window.addEventListener('message', (event) => {
              try {
                const data = event.data;
                if (data.type || data.category) {
                  this.handleStreamingData(data);
                }
              } catch (err) {
                console.error('Failed to handle postMessage:', err);
              }
            });
          },
          
          handleStreamingData(data) {
            // Check for EventBus category-based messages
            if (data.category) {
              if (data.category === 'stream') {
                const streamEvent = data.payload;
                if (streamEvent.type === 'chatMessage') {
                  this.addChatMessage({
                    username: streamEvent.userDisplayName || streamEvent.userName,
                    message: streamEvent.message,
                    color: '#ffffff', // default color
                  });
                } else if (streamEvent.type === 'follow') {
                  this.showAlert({
                    type: 'follow',
                    username: streamEvent.userDisplayName || streamEvent.userName,
                    message: 'has followed!',
                    duration: 5000
                  });
                } else if (streamEvent.type === 'subscription') {
                  this.showAlert({
                    type: 'subscription',
                    username: streamEvent.userName,
                    message: streamEvent.message || 'has subscribed!',
                    duration: 5000
                  });
                } else if (streamEvent.type === 'viewerCountUpdated') {
                  this.viewerData.current = streamEvent.count;
                  this.animateMetricUpdate();
                } else if (streamEvent.type === 'streamOnline') {
                  this.streamData.isLive = true;
                  this.streamData.title = streamEvent.title || 'Stream Live';
                } else if (streamEvent.type === 'streamOffline') {
                  this.streamData.isLive = false;
                }
              } else if (data.category === 'generic') {
                const genericEvent = data.payload;
                // E.g. Ko-fi alert or other custom webhooks
                this.showAlert({
                  type: genericEvent.eventType || 'alert',
                  username: genericEvent.source || 'Generic Webhook',
                  message: JSON.stringify(genericEvent.payload),
                  duration: 5000
                });
              } else if (data.category === 'flow') {
                const flowEvent = data.payload;
                if (flowEvent.type === 'flowCompleted') {
                  console.log('Flow Completed:', flowEvent.flowId, flowEvent.result);
                }
              }
              return;
            }

            switch (data.type) {
              case 'STREAM_STATUS':
                Object.assign(this.streamData, data.payload);
                break;
                
              case 'VIEWER_METRICS':
                Object.assign(this.viewerData, data.payload);
                this.animateMetricUpdate();
                break;
                
              case 'CHAT_MESSAGE':
                this.addChatMessage(data.payload);
                break;
                
              case 'STREAM_ALERT':
                this.showAlert(data.payload);
                break;
                
              case 'INTERACTIVE_DATA':
                Object.assign(this.interactiveData, data.payload);
                break;
                
              case 'SOCIAL_DATA':
                Object.assign(this.socialData, data.payload);
                break;
                
              case 'ELEMENT_UPDATE':
                this.updateElement(data.elementId, data.updates);
                break;
            }
          },
          
          addChatMessage(message) {
            this.chatData.messages.unshift(message);
            if (this.chatData.messages.length > 50) {
              this.chatData.messages = this.chatData.messages.slice(0, 50);
            }
          },
          
          showAlert(alert) {
            Object.assign(this.alertData, alert, { active: true });
            
            // Auto-hide alert after duration
            setTimeout(() => {
              this.alertData.active = false;
            }, alert.duration || 5000);
            
            // Add animation class
            const alertElement = document.querySelector('.alert-system-content');
            if (alertElement) {
              alertElement.classList.add('animate-bounce-in');
              setTimeout(() => {
                alertElement.classList.remove('animate-bounce-in');
              }, 600);
            }
          },
          
          animateMetricUpdate() {
            const metrics = document.querySelectorAll('.metric-value');
            metrics.forEach(metric => {
              metric.classList.add('animate-pulse');
              setTimeout(() => {
                metric.classList.remove('animate-pulse');
              }, 2000);
            });
          },
          
          updateElement(elementId, updates) {
            const element = document.getElementById('element-' + elementId);
            if (element && updates.style) {
              Object.assign(element.style, updates.style);
            }
          },
          
          startDataRefresh() {
            // Periodic data refresh for elements without real-time data
            setInterval(() => {
              // Update stream uptime
              if (this.streamData.isLive && this.streamData.startTime) {
                const uptime = Date.now() - this.streamData.startTime;
                const hours = Math.floor(uptime / 3600000);
                const minutes = Math.floor((uptime % 3600000) / 60000);
                const seconds = Math.floor((uptime % 60000) / 1000);
                this.streamData.uptime = 
                  hours.toString().padStart(2, '0') + ':' +
                  minutes.toString().padStart(2, '0') + ':' +
                  seconds.toString().padStart(2, '0');
              }
            }, 1000);
          }
        }
      };
      
      createApp(OverlayApp).mount('#overlay-app');
    `;
  };

  // Send HTML to Rust backend for serving
  const deployToServer = async (html: string) => {
    try {
      console.log('🚀 Deploying overlay HTML to server');
      
      await invoke('set_overlay_html_content', { html_content: html });
      
      console.log('✅ Deployed overlay HTML to server');
      return true;
    } catch (err) {
      error.value = err instanceof Error ? err.message : String(err);
      console.error('❌ Failed to deploy overlay HTML:', err);
      return false;
    }
  };

  return {
    // State
    loading,
    error,
    lastRenderedConfig,
    
    // Methods
    generateOverlayHTML,
    deployToServer
  };
};