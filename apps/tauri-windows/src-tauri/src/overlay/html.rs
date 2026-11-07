use super::state::{OverlayDesign, OverlayElement};

const OVERLAY_BROWSER_PROTOCOL: &str = include_str!("browser_protocol.cjs");

/// Generate complete HTML for the overlay from the design
pub fn generate_overlay_html(
    design: &OverlayDesign,
    theme_vars: &std::collections::HashMap<String, String>,
) -> String {
    let elements_html = generate_elements_html(&design.elements);
    let background_style = generate_background_style(design);
    // Bake theme vars into :root so non-iframe widgets (which use var(--theme-*))
    // are colored on first paint, before any UPDATE_VARS arrives. UPDATE_VARS
    // overrides these live via setProperty on documentElement.
    let theme_vars_css = if theme_vars.is_empty() {
        String::new()
    } else {
        // Keys and values come from the editor; any pair that could leave its
        // declaration (or the <style> element) is dropped rather than emitted.
        let vars: String = theme_vars
            .iter()
            .filter(|(k, v)| is_safe_css_var(k, v))
            .map(|(k, v)| format!("{}:{};", k, v))
            .collect();
        format!(":root {{ {} }}", vars)
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>VortiDeck Overlay</title>
    <style>
        {}
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        
        body {{
            width: {}px;
            height: {}px;
            background: transparent;
            font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
            overflow: hidden;
            position: relative;
            {}
        }}
        
        .overlay-container {{
            width: 100%;
            height: 100%;
            position: relative;
            background: transparent;
        }}
        
        .overlay-element {{
            position: absolute;
            transition: all 0.3s ease;
        }}
        
        /* Ad overlay styles */
        .ad-overlay {{
            position: fixed;
            top: 20px;
            right: 20px;
            width: 300px;
            min-height: 100px;
            background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
            border-radius: 12px;
            padding: 20px;
            color: white;
            font-family: 'Segoe UI', sans-serif;
            box-shadow: 0 10px 30px rgba(0, 0, 0, 0.3);
            z-index: 1000;
            opacity: 0;
            transform: translateX(320px);
            transition: all 0.5s cubic-bezier(0.68, -0.55, 0.265, 1.55);
        }}
        
        .ad-overlay.show {{
            opacity: 1;
            transform: translateX(0);
        }}
        
        .ad-container {{
            position: relative;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 8px;
            padding: 15px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.2);
        }}
        
        .ad-content {{
            font-size: 14px;
            line-height: 1.4;
        }}
        
        /* No floating animations - match static editor */
        
        /* No pulse animation - match static editor */
        
        /* Emote Rain Effect Styles */
        .emote-rain-container {{
            position: fixed;
            top: 0;
            left: 0;
            width: 100%;
            height: 100%;
            pointer-events: none;
            z-index: 1000;
            overflow: hidden;
        }}
        
        .emote-particle {{
            position: absolute;
            user-select: none;
            pointer-events: none;
            filter: drop-shadow(0 0 8px rgba(0,0,0,0.5));
            will-change: transform;
            backface-visibility: hidden;
        }}
    </style>
</head>
<body>
    <div class="overlay-container">
        {}
        
        <!-- Emote Rain Effect -->
        <div class="emote-rain-container" id="emote-rain-container"></div>
        
        <!-- Ad overlay container -->
        <div class="ad-overlay" id="ad-overlay">
            <div class="ad-container" id="ad-container">
                <div class="ad-content" id="ad-content"></div>
            </div>
        </div>
        <div id="vortideck-alert-runtime" style="position:fixed;inset:0;display:none;align-items:center;justify-content:center;pointer-events:none;z-index:2147483645"></div>
        <div id="vortideck-media-runtime" style="position:fixed;inset:0;display:none;align-items:center;justify-content:center;pointer-events:none;z-index:2147483646"></div>
    </div>
    
    <script>{}</script>
    <script>
        // WebSocket connection for updates with improved error handling
        let ws = null;
        let lastUpdated = 0;
        let reconnectAttempts = 0;
        let reconnectDelay = 2000;
        // Raw WS stream messages, replayed to iframes that load late or reload
        // (e.g. after a structural srcdoc change). Keeps chat history alive across
        // iframe reloads and cold start, since history is pushed on WS connect.
        let streamEventBuffer = [];
        let observedStreamValues = [];
        function deliverObservedStreamValues(iframe) {{
            const values = observedStreamValues.filter(value => iframeAllowsStreamEvent(iframe, {{ category:'stream',payload:{{type:{{followers:'follow',subscribers:'subscription',bits:'bits'}}[value.metric]}} }}));
            try {{ iframe.contentWindow.postMessage({{category:'streamValues',payload:values}}, '*'); }} catch (_) {{}}
        }}
        const STREAM_BUFFER_MAX = 50;
        const APPLIED_COMMANDS_KEY = 'vortideck-overlay-applied-commands-v1';
        const INSTANCE_ID_KEY = 'vortideck-overlay-instance-id-v1';
        function loadOverlayInstanceId() {{
            try {{
                let id = localStorage.getItem(INSTANCE_ID_KEY);
                if (!id) {{
                    id = self.crypto && self.crypto.randomUUID
                        ? self.crypto.randomUUID()
                        : `overlay-${{Date.now()}}-${{Math.random().toString(16).slice(2)}}`;
                    localStorage.setItem(INSTANCE_ID_KEY, id);
                }}
                return id;
            }} catch (_) {{
                return `overlay-session-${{Date.now()}}-${{Math.random().toString(16).slice(2)}}`;
            }}
        }}
        function loadAppliedOverlayCommands() {{
            try {{ return new Map(JSON.parse(localStorage.getItem(APPLIED_COMMANDS_KEY) || '[]')); }}
            catch (_) {{ return new Map(); }}
        }}
        function rememberAppliedOverlayCommand(commandId, revision) {{
            VortiDeckOverlayProtocol.rememberApplied(appliedOverlayCommands, commandId, revision, 500);
            try {{ localStorage.setItem(APPLIED_COMMANDS_KEY, JSON.stringify([...appliedOverlayCommands])); }}
            catch (_) {{}}
        }}
        const overlayInstanceId = loadOverlayInstanceId();
        const appliedOverlayCommands = loadAppliedOverlayCommands();
        const pendingVisualAcknowledgements = new Map();
        const readyWidgetFrames = new WeakSet();
        let overlayReadyTimer = null;
        function notifyOverlayReady() {{
            clearTimeout(overlayReadyTimer);
            // Trailing debounce exceeds the host's replay rate limit. A quick
            // source refresh must still request the durable pending commands.
            overlayReadyTimer = setTimeout(() => {{
                if (ws && ws.readyState === WebSocket.OPEN
                    && Array.from(document.querySelectorAll('iframe')).every(frame => readyWidgetFrames.has(frame))) {{
                    ws.send(JSON.stringify({{type:'OVERLAY_READY', version:1}}));
                }}
            }}, 300);
        }}
        // Install during parsing, before any embedded widget can finish loading.
        document.addEventListener('load', event => {{
            if (event.target?.tagName !== 'IFRAME') return;
            readyWidgetFrames.add(event.target);
            setTimeout(notifyOverlayReady, 50);
        }}, true);
        function sendOverlayAcknowledgement(commandId, status, revision, error, receiptPhase) {{
            if (!ws || ws.readyState !== WebSocket.OPEN) return;
            ws.send(JSON.stringify({{
                type: 'OVERLAY_COMMAND_ACK',
                commandId,
                status,
                receiptPhase: receiptPhase || 'hostAccepted',
                instanceId: overlayInstanceId,
                appliedRevision: revision || 1,
                ...(error ? {{ error }} : {{}})
            }}));
        }}
        const builtinPlayback = new Map();
        function finishBuiltinPlayback(commandId, status, error) {{
            const active = builtinPlayback.get(commandId);
            if (!active || active.finished) return;
            active.finished = true;
            if (active.timer) clearTimeout(active.timer);
            if (active.element && typeof active.element.pause === 'function') active.element.pause();
            if (active.container) {{
                active.container.replaceChildren();
                active.container.style.display = 'none';
            }}
            builtinPlayback.delete(commandId);
            sendOverlayAcknowledgement(
                commandId,
                status || 'accepted',
                active.revision,
                error,
                status === 'rejected' ? 'playbackStarted' : 'playbackCompleted'
            );
        }}
        function handleBuiltinOverlayPlayback(command) {{
            if (!command || !command.commandId || !command.command) return false;
            const revision = command.deliveryRevision || 1;
            const payload = command.payload || {{}};
            if (command.command === 'overlay.alert.play') {{
                const container = document.getElementById('vortideck-alert-runtime');
                if (!container) return false;
                const card = document.createElement('div');
                card.style.cssText = 'max-width:70vw;padding:28px 36px;border-radius:18px;background:rgba(8,12,24,.92);color:white;text-align:center;font-family:system-ui,sans-serif;box-shadow:0 18px 70px rgba(0,0,0,.55);border:1px solid rgba(255,255,255,.18)';
                const title = document.createElement('div');
                title.style.cssText = 'font-size:34px;font-weight:800;line-height:1.15';
                title.textContent = String(payload.title || 'Alert');
                card.appendChild(title);
                if (payload.message) {{
                    const message = document.createElement('div');
                    message.style.cssText = 'font-size:20px;margin-top:10px;opacity:.9';
                    message.textContent = String(payload.message);
                    card.appendChild(message);
                }}
                if (payload.mediaUrl) {{
                    const image = document.createElement('img');
                    image.src = String(payload.mediaUrl);
                    image.alt = '';
                    image.style.cssText = 'display:block;max-width:50vw;max-height:45vh;margin:18px auto 0;border-radius:12px';
                    card.appendChild(image);
                }}
                container.replaceChildren(card);
                container.style.display = 'flex';
                const duration = Math.max(500, Math.min(60000, Number(payload.durationMs) || 5000));
                const active = {{ container, element: null, revision, finished: false, timer: null }};
                builtinPlayback.set(command.commandId, active);
                sendOverlayAcknowledgement(command.commandId, 'accepted', revision, null, 'playbackStarted');
                active.timer = setTimeout(() => finishBuiltinPlayback(command.commandId), duration);
                return true;
            }}
            if (command.command !== 'overlay.media.play') return false;
            const container = document.getElementById('vortideck-media-runtime');
            if (!container) return false;
            const source = String(payload.url || '');
            const declared = String(payload.mediaType || 'auto').toLowerCase();
            const inferred = /\.(png|jpe?g|gif|webp|avif)(\?|$)/i.test(source) ? 'image'
                : /\.(mp3|wav|ogg|m4a|flac)(\?|$)/i.test(source) ? 'audio' : 'video';
            const mediaType = declared === 'auto' ? inferred : declared;
            const element = document.createElement(mediaType === 'image' ? 'img' : mediaType === 'audio' ? 'audio' : 'video');
            element.src = source;
            element.style.cssText = mediaType === 'audio'
                ? 'width:min(700px,80vw)'
                : 'max-width:100vw;max-height:100vh;object-fit:contain';
            if ('volume' in element) element.volume = Math.max(0, Math.min(1, Number(payload.volume ?? 1)));
            if ('autoplay' in element) element.autoplay = true;
            container.replaceChildren(element);
            container.style.display = 'flex';
            const maximum = Math.max(1000, Math.min(300000, Number(payload.maxDurationMs) || 60000));
            const active = {{ container, element, revision, finished: false, timer: null, started: false }};
            builtinPlayback.set(command.commandId, active);
            const started = () => {{
                if (active.started) return;
                active.started = true;
                sendOverlayAcknowledgement(command.commandId, 'accepted', revision, null, 'playbackStarted');
            }};
            element.addEventListener(mediaType === 'image' ? 'load' : 'playing', started, {{ once: true }});
            element.addEventListener('ended', () => finishBuiltinPlayback(command.commandId), {{ once: true }});
            element.addEventListener('error', () => finishBuiltinPlayback(command.commandId, 'rejected', 'Media failed to load or decode'), {{ once: true }});
            active.timer = setTimeout(() => finishBuiltinPlayback(command.commandId), maximum);
            if (mediaType === 'image') {{
                const imageDuration = Math.max(1000, Math.min(maximum, Number(payload.durationMs) || 5000));
                active.timer = setTimeout(() => finishBuiltinPlayback(command.commandId), imageDuration);
            }} else {{
                const playResult = element.play();
                if (playResult && typeof playResult.catch === 'function') {{
                    playResult.catch(error => finishBuiltinPlayback(command.commandId, 'rejected', String(error)));
                }}
            }}
            return true;
        }}
        function widgetFrameForSource(source) {{
            return Array.from(document.querySelectorAll('iframe[data-widget-package-id]'))
                .find(iframe => iframe.contentWindow === source);
        }}
        function authoritativeWidgetAttribute(iframe, name) {{
            const hostElement = iframe.closest('.overlay-element');
            if (!hostElement || !hostElement.hasAttribute('data-authorized-package-id')) return '';
            return hostElement.getAttribute(`data-authorized-${{name}}`) || '';
        }}
        function authoritativeWidgetPackageId(iframe) {{
            return authoritativeWidgetAttribute(iframe, 'package-id');
        }}
        function rejectWidgetAction(source, requestId, error) {{
            try {{ source.postMessage({{
                type: 'VORTIDECK_WIDGET_ACTION_RESULT', requestId, ok: false, error
            }}, '*'); }} catch (_) {{}}
        }}
        window.addEventListener('message', event => {{
            const request = event.data;
            if (!request) return;
            const iframe = widgetFrameForSource(event.source);
            if (!iframe) return;
            if (request.type === 'VORTIDECK_WIDGET_RENDERED') {{
                const commandId = typeof request.commandId === 'string' ? request.commandId : '';
                const pending = pendingVisualAcknowledgements.get(commandId);
                const instanceId = iframe.getAttribute('data-widget-instance-id') || '';
                const outcome = VortiDeckOverlayProtocol.acceptRenderedReceipt(
                    pending,
                    instanceId,
                    request.appliedRevision
                );
                if (!outcome.accepted) return;
                if (outcome.completed) {{
                    pendingVisualAcknowledgements.delete(commandId);
                    sendOverlayAcknowledgement(commandId, 'accepted', pending.revision, null, 'widgetRendered');
                }}
                return;
            }}
            if (request.type !== 'VORTIDECK_WIDGET_ACTION_REQUEST') return;
            const requestId = typeof request.requestId === 'string' ? request.requestId : '';
            const action = typeof request.action === 'string' ? request.action : '';
            if (!requestId || !action) {{
                rejectWidgetAction(event.source, requestId, 'Malformed widget action request');
                return;
            }}
            const allowed = authoritativeWidgetAttribute(iframe, 'actions')
                .split(',').map(value => value.trim()).filter(Boolean);
            if (!allowed.includes(action)) {{
                rejectWidgetAction(event.source, requestId, 'Widget action is not declared for this contribution');
                return;
            }}
            if (!ws || ws.readyState !== WebSocket.OPEN) {{
                rejectWidgetAction(event.source, requestId, 'VortiDeck is disconnected');
                return;
            }}
            ws.send(JSON.stringify({{
                type: 'WIDGET_ACTION_REQUEST',
                requestId,
                packageId: authoritativeWidgetPackageId(iframe),
                elementInstanceId: iframe.getAttribute('data-widget-instance-id') || '',
                overlayInstanceId,
                action,
                payload: request.payload ?? null
            }}));
        }});
        // Latest theme vars snapshot — sent to reloaded/new iframes so they render
        // with the correct colors before any further UPDATE_VARS arrives.
        let currentThemeVars = {{}};
        
        function connectWebSocket() {{
            try {{
                // Close existing connection if any
                if (ws && ws.readyState === WebSocket.OPEN) {{
                    ws.close();
                }}
                
                // Try to connect to the overlay-specific WebSocket (dynamic URL with HTTPS support)
                const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
                // The capability token is retained in this browser-source URL
                // and must accompany its WebSocket session. Do not log it.
                const wsUrl = `${{protocol}}//${{window.location.host}}/ws/overlay${{window.location.search}}`;
                ws = new WebSocket(wsUrl);
                
                ws.onopen = () => {{
                    console.log('✅ Connected to VortiDeck WebSocket - Real-time updates enabled');
                    reconnectAttempts = 0; // Reset counter on successful connection
                    notifyOverlayReady();
                }};
                
                ws.onmessage = (event) => {{
                    try {{
                        console.log('📨 Received WebSocket message:', event.data);
                        const message = JSON.parse(event.data);
                        handleMessage(message);
                    }} catch (e) {{
                        console.error('❌ Error parsing WebSocket message:', e);
                    }}
                }};
                
                ws.onclose = (event) => {{
                    if (event.wasClean) {{
                        console.log('🔌 WebSocket closed cleanly');
                    }} else {{
                        console.log('🔌 WebSocket disconnected unexpectedly');
                        reconnectAttempts++;
                        // Infinite reconnects with exponential backoff capped at 10 seconds
                        const delay = Math.min(10000, reconnectDelay * Math.pow(2, Math.min(5, reconnectAttempts - 1)));
                        console.log(`🔄 Retrying WebSocket connection in ${{delay}}ms... (attempt ${{reconnectAttempts}})`);
                        setTimeout(connectWebSocket, delay);
                    }}
                }};
                
                ws.onerror = (error) => {{
                    console.warn('⚠️ WebSocket connection failed, will use polling fallback');
                }};
            }} catch (e) {{
                console.error('🚨 WebSocket connection error:', e);
                reconnectAttempts++;
                const delay = Math.min(10000, reconnectDelay * reconnectAttempts);
                setTimeout(connectWebSocket, delay);
            }}
        }}
        
        // Check for updates via polling (fallback when WebSocket is disconnected)
        function checkForUpdates() {{
            // Skip polling when WebSocket is connected — WS handles real-time updates
            if (ws && ws.readyState === WebSocket.OPEN) return;

            fetch('/check-update')
                .then(response => response.json())
                .then(data => {{
                    if (lastUpdated > 0 && data.last_updated > lastUpdated) {{
                        console.log('Overlay updated, fetching new content...');
                        updateOverlayContent();
                    }}
                    lastUpdated = data.last_updated;
                }})
                .catch(e => console.warn('Update check failed:', e));
        }}
        
        // Update overlay content without full page reload
        let updateThrottleTimer = null;
        let pendingUpdate = false;

        function updateOverlayContent() {{
            // If currently throttled, mark that we need an update later
            if (updateThrottleTimer) {{
                pendingUpdate = true;
                console.log('⏳ Update throttled, will fetch latest state after cooldown');
                return;
            }}

            // Perform the actual fetch
            performOverlayFetch();

            // Set throttle timer
            updateThrottleTimer = setTimeout(() => {{
                updateThrottleTimer = null;
                // If there was a pending update, fetch the latest state now
                if (pendingUpdate) {{
                    pendingUpdate = false;
                    console.log('🔄 Processing pending update with latest state');
                    updateOverlayContent(); // This will fetch the current state with ALL elements
                }}
            }}, 50); // Throttle to max 20 updates per second
        }}

        function performOverlayFetch() {{
            fetch(`/overlay.html${{window.location.search}}`)
                .then(response => response.text())
                .then(html => {{
                    // Parse the new HTML
                    const parser = new DOMParser();
                    const newDoc = parser.parseFromString(html, 'text/html');
                    const newContainer = newDoc.querySelector('.overlay-container');

                    if (newContainer) {{
                        // Get current container
                        const currentContainer = document.querySelector('.overlay-container');
                        if (currentContainer) {{
                            // Parse and patch child elements instead of replacing innerHTML,
                            // preventing unmodified elements/iframes from reloading.
                            const currentElements = Array.from(currentContainer.children);
                            const newElements = Array.from(newContainer.children);
                            const newIds = new Set(newElements.map(el => el.id).filter(id => id));

                            // 1. Remove deleted elements
                            currentElements.forEach(el => {{
                                if (el.id && el.id !== 'emote-rain-container' && el.id !== 'ad-overlay' && !newIds.has(el.id)) {{
                                    el.remove();
                                    console.log('🗑️ Removed element:', el.id);
                                }}
                            }});

                            // 2. Add or update elements
                            newElements.forEach((newEl, index) => {{
                                if (!newEl.id) return;
                                
                                const existingEl = document.getElementById(newEl.id);
                                if (existingEl) {{
                                    // Update styling and wrapper classes
                                    if (existingEl.getAttribute('style') !== newEl.getAttribute('style')) {{
                                        existingEl.setAttribute('style', newEl.getAttribute('style'));
                                    }}
                                    if (existingEl.className !== newEl.className) {{
                                        existingEl.className = newEl.className;
                                    }}
                                    // Native grant projections can change without changing
                                    // srcdoc. Reconcile them before delivering more events
                                    // or accepting requests from the existing frame.
                                    for (const attribute of ['data-authorized-package-id', 'data-authorized-subscribe', 'data-authorized-actions']) {{
                                        if (newEl.hasAttribute(attribute)) {{
                                            existingEl.setAttribute(attribute, newEl.getAttribute(attribute));
                                        }} else {{
                                            existingEl.removeAttribute(attribute);
                                        }}
                                    }}
                                    const incomingConfigRevision = Number(newEl.getAttribute('data-widget-config-revision') || 0);
                                    const currentConfigRevision = Number(existingEl.getAttribute('data-widget-config-revision') || 0);
                                    if (incomingConfigRevision >= currentConfigRevision) {{
                                        existingEl.setAttribute('data-widget-config', newEl.getAttribute('data-widget-config') || '{{}}');
                                        existingEl.setAttribute('data-widget-config-revision', String(incomingConfigRevision));
                                        deliverWidgetConfig(existingEl);
                                    }}
                                    // Patch inner content
                                    const existingIframe = existingEl.querySelector('iframe');
                                    const newIframe = newEl.querySelector('iframe');
                                    
                                    if (existingIframe && newIframe) {{
                                        // Check structural difference (presence of scaling div wrapper)
                                        const existingHasWrapper = existingIframe.parentElement !== existingEl;
                                        const newHasWrapper = newIframe.parentElement !== newEl;
                                        
                                        if (existingHasWrapper !== newHasWrapper) {{
                                            // Structure changed (e.g. scaleContent toggled) - replace entirely
                                            existingEl.innerHTML = newEl.innerHTML;
                                            console.log('🔄 Replaced structure (scale content toggled):', newEl.id);
                                        }} else {{
                                            // If both have scale wrapper, sync its style (contains transform: scale and custom styles)
                                            if (existingHasWrapper) {{
                                                const existingWrapper = existingIframe.parentElement;
                                                const newWrapper = newIframe.parentElement;
                                                if (existingWrapper.getAttribute('style') !== newWrapper.getAttribute('style')) {{
                                                    existingWrapper.setAttribute('style', newWrapper.getAttribute('style'));
                                                    console.log('📏 Updated inner scale style:', newEl.id);
                                                }}
                                            }}
                                            
                                            // Sync iframe srcdoc
                                            const oldSrcdoc = existingIframe.getAttribute('srcdoc');
                                            const newSrcdoc = newIframe.getAttribute('srcdoc');
                                            if (oldSrcdoc !== newSrcdoc) {{
                                                readyWidgetFrames.delete(existingIframe);
                                                existingIframe.setAttribute('srcdoc', newSrcdoc);
                                                console.log('🔄 Updated iframe srcdoc:', newEl.id);
                                            }}
                                        }}
                                    }} else if (!existingIframe && !newIframe) {{
                                        if (existingEl.innerHTML !== newEl.innerHTML) {{
                                            existingEl.innerHTML = newEl.innerHTML;
                                            console.log('✏️ Updated innerHTML:', newEl.id);
                                        }}
                                    }} else {{
                                        // One has iframe, other doesn't - replace entirely
                                        existingEl.innerHTML = newEl.innerHTML;
                                        console.log('🔄 Replaced element structure:', newEl.id);
                                    }}
                                }} else {{
                                    // Add new element at correct position
                                    const referenceEl = currentContainer.children[index];
                                    if (referenceEl) {{
                                        currentContainer.insertBefore(newEl.cloneNode(true), referenceEl);
                                    }} else {{
                                        currentContainer.appendChild(newEl.cloneNode(true));
                                    }}
                                    console.log('➕ Added element:', newEl.id);
                                }}
                            }});
                        }}
                    }}

                    console.log('✅ Overlay updated - OBS should refresh');
                    notifyOverlayReady();
                }})
                .catch(e => {{
                    console.warn('Failed to update content:', e);
                }});
        }}
        
        function iframeAllowsStreamEvent(iframe, message) {{
            const subscribe = authoritativeWidgetAttribute(iframe, 'subscribe');
            // Native, non-widget iframes have no subscription attribute.
            // Every package widget is explicit and default-deny, including
            // embedded first-party contributions with an empty subscription.
            if (!iframe.hasAttribute('data-widget-package-id')) return true;
            const allowed = subscribe.split(',').map(s => s.trim()).filter(Boolean);
            const eventType = message.payload && message.payload.type;
            return eventType === 'clearChat' || allowed.includes('*') || !eventType || allowed.includes(eventType);
        }}

        function iframeMatchesOverlayTarget(iframe, target) {{
            const hostElement = iframe.closest('.overlay-element');
            return Boolean(hostElement) && VortiDeckOverlayProtocol.frameMatchesTarget({{
                elementInstanceId: hostElement.id,
                packageId: authoritativeWidgetPackageId(iframe)
            }}, target);
        }}

        function handleMessage(message) {{
            // Payloads may contain chat or creator data; do not dump them into OBS logs.

            if (message.type === 'OVERLAY_UPDATED'
                && message.reason === 'package_authority_changed') {{
                // Buffered events were accepted under an older grant. Clearing
                // the shared bounded buffer is conservative and prevents a
                // newly re-granted/replaced frame from receiving revoked-era
                // data when it loads.
                streamEventBuffer.length = 0;
                pendingVisualAcknowledgements.forEach((pending, commandId) => {{
                    sendOverlayAcknowledgement(
                        commandId,
                        'rejected',
                        pending.revision,
                        'Target widget authority changed before render',
                        'widgetRendered'
                    );
                }});
                pendingVisualAcknowledgements.clear();
            }}

            if (message.type === 'WIDGET_ACTION_RESULT') {{
                if (message.overlayInstanceId !== overlayInstanceId) return;
                const iframe = Array.from(document.querySelectorAll('iframe[data-widget-package-id]'))
                    .find(candidate => candidate.getAttribute('data-widget-instance-id') === message.elementInstanceId);
                if (iframe) {{
                    try {{ iframe.contentWindow.postMessage({{
                        type: 'VORTIDECK_WIDGET_ACTION_RESULT',
                        requestId: message.requestId,
                        ok: Boolean(message.ok),
                        result: message.result ?? null,
                        error: message.error ?? null
                    }}, '*'); }} catch (_) {{}}
                }}
                return;
            }}
            
            // Intercept EventBus category-based messages to avoid triggering full page refetches
            if (message.category === 'streamValues' && Array.isArray(message.payload)) {{
                observedStreamValues = message.payload;
                document.querySelectorAll('iframe').forEach(deliverObservedStreamValues);
                return;
            }}
            if (message.category === 'stream' || message.category === 'generic' || message.category === 'flow' || message.category === 'overlay') {{
                // Expired workflow commands are never delivered after a slow
                // reconnect or a suspended browser source resumes.
                if (message.category === 'overlay' && message.payload && message.payload.expiresAt) {{
                    if (Date.parse(message.payload.expiresAt) <= Date.now()) return;
                }}
                if (message.category === 'overlay' && VortiDeckOverlayProtocol.missingDeliveryInstances(
                    Array.from(document.querySelectorAll('iframe')).map(frame => ({{
                        elementInstanceId: frame.closest('.overlay-element')?.id,
                        ready: readyWidgetFrames.has(frame)
                    }})), message.eligibleWidgetIds).length > 0) {{
                    // Nothing was forwarded or marked applied. The readiness
                    // handshake replays the existing durable outbox once the
                    // published frame is loaded; there is no second effect queue.
                    return;
                }}
                // Buffer raw stream events so they can be replayed to iframes that
                // load late or reload (keeps chat history across reloads).
                if (message.category === 'stream' && message.payload?.type === 'chatMessage') {{
                    streamEventBuffer.push(message);
                    if (streamEventBuffer.length > STREAM_BUFFER_MAX) streamEventBuffer.shift();
                }}
                const isDuplicateOverlayCommand = message.category === 'overlay'
                    && VortiDeckOverlayProtocol.shouldSuppressHostAccepted(
                        appliedOverlayCommands,
                        message.payload
                    );
                if (message.category === 'overlay' && message.payload && message.payload.commandId) {{
                    rememberAppliedOverlayCommand(
                        message.payload.commandId,
                        message.payload.deliveryRevision || 1
                    );
                }}
                // Forward event-bus categories to all iframes, with capability
                // filtering for stream events (Phase 3): a widget only receives
                // stream events it subscribed to (data-subscribe on its iframe).
                // Builtin widgets (no data-subscribe) + generic/flow → all events.
                const targetedWidgetFrames = [];
                document.querySelectorAll('iframe').forEach(iframe => {{
                    if (isDuplicateOverlayCommand) return;
                    if (message.category === 'overlay' && !iframeMatchesOverlayTarget(iframe, message.payload && message.payload.target)) return;
                    if (message.category === 'overlay' && Array.isArray(message.eligibleWidgetIds)
                        && !message.eligibleWidgetIds.includes(iframe.closest('.overlay-element')?.id)) return;
                    if (message.category === 'stream') {{
                        if (!iframeAllowsStreamEvent(iframe, message)) return;
                    }} else if ((message.category === 'generic' || message.category === 'flow')
                        && iframe.hasAttribute('data-widget-package-id')) {{
                        // Package widgets need an explicit future capability
                        // before generic data or workflow results may cross the
                        // iframe boundary.
                        return;
                    }}
                    if (message.category === 'overlay'
                        && iframe.hasAttribute('data-widget-package-id')) {{
                        targetedWidgetFrames.push(iframe);
                    }}
                    try {{
                        iframe.contentWindow.postMessage(message, '*');
                    }} catch (e) {{}}
                }});

                // Dispatch a custom event on the parent window for any inline scripts
                if (!isDuplicateOverlayCommand) {{
                    const event = new CustomEvent('vortideck_event', {{ detail: message }});
                    window.dispatchEvent(event);
                    if (message.category === 'overlay' && message.payload?.target?.type !== 'widget') {{
                        handleBuiltinOverlayPlayback(message.payload);
                    }}
                }}

                // Host-accepted ACK is immediate. Widget-rendered ACK is
                // optional and completes only after every targeted package
                // iframe explicitly reports VORTIDECK_WIDGET_RENDERED.
                if (message.category === 'overlay' && message.payload && message.payload.requiresAck) {{
                    const revision = message.payload.deliveryRevision || 1;
                    if (message.payload.acknowledgementMode === 'widgetRendered') {{
                        const remaining = VortiDeckOverlayProtocol.targetedWidgetInstanceIds(
                            targetedWidgetFrames.map(iframe => ({{
                                elementInstanceId: iframe.getAttribute('data-widget-instance-id') || '',
                                packageId: authoritativeWidgetPackageId(iframe)
                            }})),
                            null
                        );
                        if (remaining.size === 0) {{
                            sendOverlayAcknowledgement(
                                message.payload.commandId,
                                'rejected',
                                revision,
                                'No targeted package widget supports a visual ACK'
                            );
                        }} else {{
                            pendingVisualAcknowledgements.set(message.payload.commandId, {{
                                revision,
                                remaining
                            }});
                        }}
                    }} else {{
                        sendOverlayAcknowledgement(message.payload.commandId, 'accepted', revision);
                    }}
                }}

                return;
            }}

            // Live CSS variable updates (theme changes). Applied to the overlay
            // :root (so non-iframe widgets recolor via the cascade) AND forwarded
            // to each iframe (so iframe widgets update their own :root). No
            // srcdoc regeneration, no reload.
            if (message.type === 'WIDGET_CONFIG_UPDATED' && message.version === 1) {{
                const element = document.getElementById(message.elementId);
                if (element && message.configRevision >= Number(element.getAttribute('data-widget-config-revision') || 0)) {{
                    element.setAttribute('data-widget-config', JSON.stringify(message.config));
                    element.setAttribute('data-widget-config-revision', String(message.configRevision));
                    deliverWidgetConfig(element);
                }}
                return;
            }}
            if (message.type === 'UPDATE_VARS') {{
                currentThemeVars = {{ ...currentThemeVars, ...message.payload }};
                if (message.payload) {{
                    for (const [key, value] of Object.entries(message.payload)) {{
                        document.documentElement.style.setProperty(key, value);
                    }}
                }}
                document.querySelectorAll('iframe').forEach(iframe => {{
                    try {{
                        iframe.contentWindow.postMessage(message, '*');
                    }} catch (e) {{}}
                }});
                return;
            }}
            
            switch (message.type) {{
                case 'ELEMENT_PATCH': {{
                    // Generic live style patch (drag position, z-order, …) applied
                    // inline on the outer .overlay-element div — no fetch, no reload.
                    // Keys are camelCase CSS-DOM property names; values are strings.
                    const patchEl = document.getElementById(message.element_id);
                    if (patchEl && message.styles) {{
                        for (const [prop, value] of Object.entries(message.styles)) {{
                            patchEl.style[prop] = value;
                        }}
                    }}
                    break;
                }}
                case 'SHOW_ADVERTISEMENT':
                    showAd(message.ad);
                    break;
                case 'HIDE_ADVERTISEMENT':
                    hideAd();
                    break;
                case 'SHOW_ALERT':
                    showAlert(message.alert);
                    break;
                case 'DYNAMIC_EFFECT':
                    handleDynamicEffect(message.effect);
                    break;
                default:
                    // OVERLAY_UPDATED, ELEMENT_ADDED, ELEMENT_REMOVED, ELEMENT_UPDATE,
                    // OVERLAY_CLEARED, etc. — fetch fresh HTML and diff in place.
                    updateOverlayContent();
                    break;
            }}
        }}
        
        function updateElementPosition(elementId, updates) {{
            const element = document.getElementById(elementId);
            if (!element) return;
            
            if (updates.x !== undefined) element.style.left = updates.x + 'px';
            if (updates.y !== undefined) element.style.top = updates.y + 'px';
            if (updates.width !== undefined) element.style.width = updates.width + 'px';
            if (updates.height !== undefined) element.style.height = updates.height + 'px';
            if (updates.visible !== undefined) element.style.display = updates.visible ? 'block' : 'none';
        }}
        
        function showAd(ad) {{
            const container = document.getElementById('ad-container');
            const content = document.getElementById('ad-content');
            const overlay = document.getElementById('ad-overlay');
            
            if (container && content && overlay) {{
                // Set ad content
                if (ad.content_type === 'html') {{
                    content.innerHTML = ad.image_url || ad.content || 'Advertisement';
                }} else {{
                    content.innerHTML = `
                        <h3>${{ad.name || 'Advertisement'}}</h3>
                        <p>${{ad.content || ad.advertiser || 'Sponsored content'}}</p>
                    `;
                }}
                
                // Show the overlay
                overlay.classList.add('show');
                
                // Auto-hide after duration (if specified)
                if (ad.duration_seconds && ad.duration_seconds > 0) {{
                    setTimeout(() => {{
                        hideAd();
                    }}, ad.duration_seconds * 1000);
                }}
            }}
        }}
        
        function hideAd() {{
            const overlay = document.getElementById('ad-overlay');
            if (overlay) {{
                overlay.classList.remove('show');
            }}
        }}
        
        function showAlert(alert) {{
            console.log('Alert:', alert);
            // Could implement alert system here
        }}
        
        // 🚀 DYNAMIC EFFECT SYSTEM - Fully Scalable
        let activeEffects = new Map(); // Track all active effects
        
        function handleDynamicEffect(effect) {{
            console.log('🎬 Dynamic effect received:', effect);
            
            if (effect.action === 'start') {{
                startDynamicEffect(effect);
            }} else if (effect.action === 'stop') {{
                stopDynamicEffect(effect.id);
            }} else if (effect.action === 'update') {{
                updateDynamicEffect(effect);
            }}
        }}
        
        function startDynamicEffect(effect) {{
            // Stop existing effect with same ID
            if (activeEffects.has(effect.id)) {{
                stopDynamicEffect(effect.id);
            }}
            
            const effectState = {{
                id: effect.id,
                type: effect.type,
                config: effect.config,
                particles: [],
                animationId: null,
                container: null,
                lastSpawnTime: 0
            }};
            
            // Create effect container
            let container = document.getElementById(effect.id);
            if (!container) {{
                container = document.createElement('div');
                container.id = effect.id;
                container.className = 'dynamic-effect-container';
                container.style.cssText = `
                    position: fixed;
                    top: 0;
                    left: 0;
                    width: 100%;
                    height: 100%;
                    pointer-events: none;
                    z-index: ${{effect.config.zIndex || 1000}};
                    overflow: hidden;
                `;
                document.body.appendChild(container);
            }}
            container.innerHTML = ''; // Clear existing content
            effectState.container = container;
            
            // Start the appropriate effect type
            if (effect.type === 'particle_rain') {{
                startParticleRain(effectState);
            }} else if (effect.type === 'explosion') {{
                startExplosion(effectState);
            }} else if (effect.type === 'floating_text') {{
                startFloatingText(effectState);
            }} else if (effect.type === 'image_shower') {{
                startImageShower(effectState);
            }} else if (effect.type === 'custom_animation') {{
                startCustomAnimation(effectState);
            }}
            
            activeEffects.set(effect.id, effectState);
            console.log(`✅ Started dynamic effect: ${{effect.type}} (${{effect.id}})`);
        }}
        
        function stopDynamicEffect(effectId) {{
            const effectState = activeEffects.get(effectId);
            if (!effectState) return;
            
            // Cancel animation
            if (effectState.animationId) {{
                cancelAnimationFrame(effectState.animationId);
            }}
            
            // Remove container
            if (effectState.container) {{
                effectState.container.remove();
            }}
            
            activeEffects.delete(effectId);
            console.log(`🛑 Stopped dynamic effect: ${{effectId}}`);
        }}
        
        function updateDynamicEffect(effect) {{
            const effectState = activeEffects.get(effect.id);
            if (!effectState) return;
            
            // Update configuration
            effectState.config = {{ ...effectState.config, ...effect.config }};
            console.log(`🔄 Updated dynamic effect: ${{effect.id}}`);
        }}
        
        // 🌧️ Particle Rain Effect (emotes, images, text, anything!)
        function startParticleRain(effectState) {{
            const config = effectState.config;
            
            function animate(currentTime) {{
                if (!activeEffects.has(effectState.id)) return;
                
                const spawnInterval = config.spawnInterval || 100;
                const particlesPerSpawn = config.particlesPerSpawn || 4;
                
                // Spawn new particles
                if (currentTime - effectState.lastSpawnTime > spawnInterval) {{
                    for (let i = 0; i < particlesPerSpawn; i++) {{
                        const particle = {{
                            id: `${{effectState.id}}_${{currentTime}}_${{i}}`,
                            x: Math.random() * window.innerWidth,
                            y: config.startY || -50,
                            size: (config.minSize || 20) + Math.random() * (config.maxSize || 40),
                            speed: (config.minSpeed || 1) + Math.random() * (config.maxSpeed || 4),
                            rotate: Math.random() * 360,
                            rotateSpeed: (Math.random() * 8 - 4) * (config.rotationMultiplier || 1),
                            content: config.content[Math.floor(Math.random() * config.content.length)],
                            opacity: config.opacity || 1
                        }};
                        
                        effectState.particles.push(particle);
                        
                        // Create DOM element
                        const element = document.createElement('div');
                        element.className = 'dynamic-particle';
                        element.id = particle.id;
                        
                        // Handle different content types
                        if (config.contentType === 'image') {{
                            const img = document.createElement('img');
                            img.src = particle.content;
                            img.style.width = '100%';
                            img.style.height = '100%';
                            img.style.objectFit = 'contain';
                            element.appendChild(img);
                        }} else {{
                            element.textContent = particle.content;
                        }}
                        
                        // Apply styles
                        element.style.cssText = `
                            position: absolute;
                            left: ${{particle.x}}px;
                            top: ${{particle.y}}px;
                            font-size: ${{particle.size}}px;
                            transform: rotate(${{particle.rotate}}deg);
                            opacity: ${{particle.opacity}};
                            filter: drop-shadow(0 0 8px rgba(0,0,0,0.5));
                            will-change: transform;
                            backface-visibility: hidden;
                            user-select: none;
                            pointer-events: none;
                            z-index: ${{config.particleZIndex || 1001}};
                            ${{config.customCSS || ''}}
                        `;
                        
                        effectState.container.appendChild(element);
                    }}
                    effectState.lastSpawnTime = currentTime;
                }}
                
                // Update existing particles
                effectState.particles = effectState.particles.filter(particle => {{
                    particle.y += particle.speed;
                    particle.rotate += particle.rotateSpeed;
                    
                    const element = document.getElementById(particle.id);
                    if (element) {{
                        element.style.top = particle.y + 'px';
                        element.style.transform = `rotate(${{particle.rotate}}deg)`;
                        
                        // Apply fade out if configured
                        if (config.fadeOut) {{
                            const progress = particle.y / (window.innerHeight + 100);
                            element.style.opacity = Math.max(0, particle.opacity * (1 - progress));
                        }}
                    }}
                    
                    // Remove if off screen
                    if (particle.y > window.innerHeight + 100) {{
                        if (element) element.remove();
                        return false;
                    }}
                    return true;
                }});
                
                effectState.animationId = requestAnimationFrame(animate);
            }}
            
            effectState.animationId = requestAnimationFrame(animate);
        }}
        
        // 💥 Explosion Effect
        function startExplosion(effectState) {{
            const config = effectState.config;
            const centerX = config.x || window.innerWidth / 2;
            const centerY = config.y || window.innerHeight / 2;
            const particleCount = config.particleCount || 20;
            
            for (let i = 0; i < particleCount; i++) {{
                const angle = (i / particleCount) * Math.PI * 2;
                const speed = config.speed || 5;
                const particle = {{
                    id: `explosion_${{effectState.id}}_${{i}}`,
                    x: centerX,
                    y: centerY,
                    vx: Math.cos(angle) * speed,
                    vy: Math.sin(angle) * speed,
                    size: config.particleSize || 30,
                    content: config.content[Math.floor(Math.random() * config.content.length)],
                    life: config.lifetime || 2000
                }};
                
                effectState.particles.push(particle);
                
                const element = document.createElement('div');
                element.className = 'explosion-particle';
                element.id = particle.id;
                element.textContent = particle.content;
                element.style.cssText = `
                    position: absolute;
                    left: ${{particle.x}}px;
                    top: ${{particle.y}}px;
                    font-size: ${{particle.size}}px;
                    opacity: 1;
                    transition: opacity ${{particle.life}}ms ease-out;
                    pointer-events: none;
                    z-index: ${{config.particleZIndex || 1001}};
                `;
                
                effectState.container.appendChild(element);
                
                // Animate particle
                setTimeout(() => {{
                    element.style.left = (particle.x + particle.vx * 100) + 'px';
                    element.style.top = (particle.y + particle.vy * 100) + 'px';
                    element.style.opacity = '0';
                }}, 10);
                
                // Remove after lifetime
                setTimeout(() => {{
                    if (element) element.remove();
                }}, particle.life);
            }}
            
            // Auto-stop explosion after all particles fade
            setTimeout(() => {{
                stopDynamicEffect(effectState.id);
            }}, config.lifetime || 2000);
        }}
        
        // 📝 Floating Text Effect
        function startFloatingText(effectState) {{
            const config = effectState.config;
            
            config.texts.forEach((text, index) => {{
                const element = document.createElement('div');
                element.className = 'floating-text';
                element.id = `text_${{effectState.id}}_${{index}}`;
                element.textContent = text.content;
                
                element.style.cssText = `
                    position: absolute;
                    left: ${{text.x || 50}}%;
                    top: ${{text.y || 50}}%;
                    transform: translate(-50%, -50%);
                    font-size: ${{text.size || 48}}px;
                    color: ${{text.color || '#ffffff'}};
                    font-weight: bold;
                    text-shadow: 2px 2px 4px rgba(0,0,0,0.8);
                    opacity: 0;
                    transition: all ${{text.duration || 1000}}ms ease-out;
                    pointer-events: none;
                    z-index: ${{config.particleZIndex || 1001}};
                    ${{text.customCSS || ''}}
                `;
                
                effectState.container.appendChild(element);
                
                // Trigger animation
                setTimeout(() => {{
                    element.style.opacity = '1';
                    element.style.transform = `translate(-50%, -50%) translateY(-${{text.floatDistance || 50}}px)`;
                }}, text.delay || 0);
                
                // Fade out
                setTimeout(() => {{
                    element.style.opacity = '0';
                }}, (text.delay || 0) + (text.duration || 1000));
                
                // Remove
                setTimeout(() => {{
                    if (element) element.remove();
                }}, (text.delay || 0) + (text.duration || 1000) + 500);
            }});
            
            // Auto-stop after all animations complete
            const maxDuration = Math.max(...config.texts.map(t => (t.delay || 0) + (t.duration || 1000)));
            setTimeout(() => {{
                stopDynamicEffect(effectState.id);
            }}, maxDuration + 1000);
        }}
        
        // 🖼️ Image Shower Effect
        function startImageShower(effectState) {{
            // Reuse particle rain logic but with images
            effectState.config.contentType = 'image';
            startParticleRain(effectState);
        }}
        
        // 🎨 Custom Animation Effect
        function startCustomAnimation(effectState) {{
            const config = effectState.config;
            
            // Execute custom JavaScript code safely
            try {{
                const customFunction = new Function('container', 'config', 'stopEffect', config.code);
                customFunction(effectState.container, config, () => stopDynamicEffect(effectState.id));
            }} catch (error) {{
                console.error('Custom animation error:', error);
                stopDynamicEffect(effectState.id);
            }}
        }}
        
        // Listen for messages from parent window (for iframe communication)
        window.addEventListener('message', (event) => {{
            if (event.data && event.data.type === 'UPDATE_ELEMENT_POSITION') {{
                const {{ elementId, x, y }} = event.data;
                const element = document.getElementById(elementId);
                if (element) {{
                    element.style.left = x + 'px';
                    element.style.top = y + 'px';
                }}
            }}
        }});
        
        function deliverWidgetConfig(element) {{
            if (!element) return;
            try {{
                const payload = JSON.parse(element.getAttribute('data-widget-config') || '{{}}');
                const revision = Number(element.getAttribute('data-widget-config-revision') || 0);
                element.querySelectorAll('iframe').forEach(iframe => {{
                    iframe.contentWindow.postMessage({{ type: 'UPDATE_CONFIG', version: 1, payload, revision }}, '*');
                }});
            }} catch (error) {{ console.warn('Invalid widget settings snapshot', error); }}
        }}

        // Connect on load
        document.addEventListener('DOMContentLoaded', () => {{
            console.log('🎨 VortiDeck Overlay initialized - Real-time mode');
            console.log('📐 Configured Layout Resolution: ' + getComputedStyle(document.body).width + ' x ' + getComputedStyle(document.body).height);
            console.log('🖥️ Browser Viewport Resolution: ' + window.innerWidth + ' x ' + window.innerHeight);

            // Replay buffered stream events + current theme vars to iframes that
            // finish loading late or reload. Capture phase is required because the
            // 'load' event does not bubble up from iframes.
            document.addEventListener('load', (event) => {{
                if (!event.target || event.target.tagName !== 'IFRAME') return;
                const iframe = event.target;
                deliverWidgetConfig(iframe.closest('.overlay-element'));
                setTimeout(() => {{
                    // Theme vars first so the widget renders with correct colors
                    if (Object.keys(currentThemeVars).length > 0) {{
                        try {{ iframe.contentWindow.postMessage({{ type: 'UPDATE_VARS', payload: currentThemeVars }}, '*'); }} catch (_) {{}}
                    }}
                    // Then replay stream history (chat messages, etc.)
                    deliverObservedStreamValues(iframe);
                    streamEventBuffer.forEach(msg => {{
                        if (!iframeAllowsStreamEvent(iframe, msg)) return;
                        try {{ iframe.contentWindow.postMessage(msg, '*'); }} catch (_) {{}}
                    }});
                }}, 50); // 50ms lets the iframe's inline <script> finish executing
            }}, true);

            document.querySelectorAll('.overlay-element').forEach(deliverWidgetConfig);
            connectWebSocket();
            
            // AGGRESSIVE polling for OBS Browser Source (it doesn't auto-refresh)
            setInterval(checkForUpdates, 100); // Check every 100ms for near real-time
            
            // Signal to parent that overlay is ready
            if (window.parent !== window) {{
                window.parent.postMessage({{ type: 'OVERLAY_READY' }}, '*');
            }}
            
            // Initial update check
            setTimeout(checkForUpdates, 50);
        }});
    </script>
</body>
</html>"#,
        theme_vars_css,
        design.canvas_size.width,
        design.canvas_size.height,
        background_style,
        elements_html,
        OVERLAY_BROWSER_PROTOCOL
    )
}

/// Generate HTML for all overlay elements
fn generate_elements_html(elements: &[OverlayElement]) -> String {
    elements
        .iter()
        .filter(|e| e.visible)
        .map(generate_element_html)
        .collect::<Vec<_>>()
        .join("\n        ")
}

/// Generate HTML for a single overlay element.
/// All element visuals come from `render_html` (set by the Vue element registry + sync).
/// The match fallback only exists for elements created before the registry existed.
fn generate_element_html(element: &OverlayElement) -> String {
    // The id is emitted into an attribute and used as a CSS selector by the
    // synced render_html; ids outside the composition alphabet are not rendered.
    if !is_safe_element_id(&element.id) {
        log::warn!("Skipping overlay element with an invalid id");
        return String::new();
    }
    let element_type = html_escape(&element.element_type);
    let position_style = format!(
        "left: {}px; top: {}px; width: {}px; height: {}px;",
        element.position.x, element.position.y, element.size.width, element.size.height
    );

    // z-index belongs on the OUTER `.overlay-element` div (the sibling), not on
    // the inner render_html wrapper, so it actually controls stacking and survives
    // the overlay's fetch+diff style patching.
    let z_index_style = match element.z_index {
        Some(z) => format!(" z-index: {};", z),
        None => String::new(),
    };

    let custom_style = html_escape(element.style.as_deref().unwrap_or(""));
    let mut authority_attributes = element.package_id.as_deref().map_or_else(
        String::new,
        |package_id| {
            let capability_list = |name: &str| {
                element
                    .capabilities
                    .as_ref()
                    .and_then(|capabilities| capabilities.get(name))
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .map(html_escape)
                    .collect::<Vec<_>>()
                    .join(",")
            };
            format!(
                r#" data-authorized-package-id="{}" data-authorized-subscribe="{}" data-authorized-actions="{}""#,
                html_escape(vortideck_core::plugins::canonical_package_id(package_id)),
                capability_list("subscribe"),
                capability_list("actions")
            )
        },
    );

    authority_attributes.push_str(&format!(
        r#" data-widget-config="{}" data-widget-config-revision="{}""#,
        html_escape(&element.config.to_string()),
        element.config_revision
    ));

    // Primary path: render_html from the element registry via sync
    if let Some(custom_html) = &element.render_html {
        return format!(
            r#"<div id="{}" class="overlay-element" data-element-type="{}"{} style="{}{} {}">{}</div>"#,
            element.id,
            element_type,
            authority_attributes,
            position_style,
            z_index_style,
            custom_style,
            custom_html
        );
    }

    // Legacy fallback for elements without render_html
    let content = if !element.content.is_empty() {
        html_escape(&element.content)
    } else {
        html_escape(&element.element_type)
    };

    format!(
        r#"<div id="{}" class="overlay-element" data-element-type="{}"{} style="{}{} color: white; font-family: Arial, sans-serif; font-size: 14px; display: flex; align-items: center; justify-content: center; {}">{}</div>"#,
        element.id,
        element_type,
        authority_attributes,
        position_style,
        z_index_style,
        custom_style,
        content
    )
}

/// Generate background CSS style for the overlay
fn generate_background_style(design: &OverlayDesign) -> String {
    let mut styles = Vec::new();

    if let Some(color) = design
        .background_color
        .as_deref()
        .filter(|color| is_safe_css_value(color))
    {
        styles.push(format!("background-color: {};", color));
    }

    if let Some(image) = &design.background_image {
        styles.push(format!(
            "background-image: url('{}'); background-size: cover; background-repeat: no-repeat;",
            html_escape(image)
        ));
    }

    styles.join(" ")
}

/// A CSS value that cannot end its declaration, rule or `<style>` element.
fn is_safe_css_value(value: &str) -> bool {
    !value.contains(['<', '>', '{', '}', ';', '\\']) && !value.contains("/*")
}

/// A custom property (`--name`) whose value stays inside its declaration.
fn is_safe_css_var(name: &str, value: &str) -> bool {
    name.len() > 2
        && name.starts_with("--")
        && name[2..]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        && is_safe_css_value(value)
}

/// Element ids share the composition id alphabet (`[A-Za-z0-9_-]`).
fn is_safe_element_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
}

/// Escape HTML special characters
fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::overlay::state::{Position, Size};

    #[test]
    fn test_html_escape() {
        assert_eq!(html_escape("<script>"), "&lt;script&gt;");
        assert_eq!(html_escape("Hello & World"), "Hello &amp; World");
        assert_eq!(html_escape(r#""a'"#), "&quot;a&#x27;");
    }

    fn element(id: &str, element_type: &str, style: &str) -> OverlayElement {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "element_type": element_type,
            "content": "",
            "position": { "x": 0, "y": 0 },
            "size": { "width": 10, "height": 10 },
            "style": style,
            "visible": true,
            "render_html": "<b>ok</b>",
            "custom_props": null
        }))
        .unwrap()
    }

    #[test]
    fn css_var_validation_rejects_style_breakouts() {
        assert!(is_safe_css_var("--theme-primary", "#ff1493"));
        assert!(is_safe_css_var("--c_bg", "linear-gradient(135deg, var(--a), #fff)"));
        assert!(!is_safe_css_var("--x", "red}</style><img src=x onerror=alert(1)>"));
        assert!(!is_safe_css_var("--x", "red; background: url(x)"));
        assert!(!is_safe_css_var("--x", "red /* comment"));
        assert!(!is_safe_css_var("--x", "\\3c"));
        assert!(!is_safe_css_var("--x}body{", "red"));
        assert!(!is_safe_css_var("color", "red"));
        assert!(!is_safe_css_var("--", "red"));
    }

    #[test]
    fn theme_vars_cannot_close_the_style_element() {
        let mut vars = std::collections::HashMap::new();
        vars.insert("--ok".to_string(), "#123456".to_string());
        vars.insert("--bad".to_string(), "red}</style><img src=x onerror=alert(1)>".to_string());
        vars.insert("--x}</style><b>".to_string(), "red".to_string());
        let html = generate_overlay_html(&OverlayDesign::default(), &vars);
        assert!(html.contains("--ok:#123456;"));
        assert!(!html.contains("onerror=alert(1)"));
        assert!(!html.contains("</style><b>"));
    }

    #[test]
    fn background_color_is_validated() {
        let mut design = OverlayDesign::default();
        design.background_color = Some("red}</style><script>alert(1)</script>".into());
        assert!(!generate_background_style(&design).contains("<script>"));
        design.background_color = Some("rgba(0, 0, 0, 0.5)".into());
        assert_eq!(generate_background_style(&design), "background-color: rgba(0, 0, 0, 0.5);");
    }

    #[test]
    fn element_attributes_are_escaped_and_ids_validated() {
        let html = generate_element_html(&element(
            "el-1_A",
            "text\" onmouseover=\"alert(1)",
            "color: red\"><script>alert(1)</script>",
        ));
        assert!(html.starts_with(r#"<div id="el-1_A" class="overlay-element" data-element-type="text&quot; onmouseover=&quot;alert(1)""#));
        assert!(html.contains("color: red&quot;&gt;&lt;script&gt;"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("<b>ok</b>"));

        assert_eq!(generate_element_html(&element("x\" onclick=\"alert(1)", "text", "")), "");
        assert_eq!(generate_element_html(&element("", "text", "")), "");
    }

    // Legacy V1 test removed - V3 uses streaming elements system
    // Test generation now handled by dynamic templates

    #[test]
    fn test_generate_overlay_html() {
        let design = OverlayDesign::default();
        let html = generate_overlay_html(&design, &std::collections::HashMap::new());

        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("VortiDeck Overlay"));
        assert!(html.contains("overlay-container"));
    }

    #[test]
    fn overlay_html_guards_community_widget_delivery() {
        let html =
            generate_overlay_html(&OverlayDesign::default(), &std::collections::HashMap::new());

        assert!(html.contains("iframeAllowsStreamEvent"));
        assert!(html.contains("iframeMatchesOverlayTarget"));
        assert!(html.contains("data-widget-package-id"));
        assert!(html.contains("message.category === 'generic' || message.category === 'flow'"));
        assert!(html.contains("OVERLAY_COMMAND_ACK"));
        assert!(html.contains("/ws/overlay${window.location.search}"));
        assert!(!html.contains("WebSocket URL:"));
        assert!(html.contains("appliedOverlayCommands"));
        assert!(html.contains("appliedRevision"));
        assert!(html.contains("overlayInstanceId"));
        assert!(html.contains("VORTIDECK_WIDGET_ACTION_REQUEST"));
        assert!(html.contains("WIDGET_ACTION_REQUEST"));
        assert!(html.contains("authoritativeWidgetAttribute(iframe, 'actions')"));
        assert!(html.contains("data-authorized-package-id"));
        assert!(html.contains("Widget action is not declared for this contribution"));
        assert!(html.contains("VORTIDECK_WIDGET_RENDERED"));
        assert!(html.contains("pendingVisualAcknowledgements"));
        assert!(html.contains("VortiDeckOverlayProtocol.acceptRenderedReceipt"));
        assert!(html.contains("message.reason === 'package_authority_changed'"));
        assert!(html.contains("streamEventBuffer.length = 0"));
        assert!(html.contains("Target widget authority changed before render"));
        assert!(html.contains("acknowledgementMode === 'widgetRendered'"));
        assert!(html.contains("No targeted package widget supports a visual ACK"));
        assert!(html.contains("overlay.alert.play"));
        assert!(html.contains("overlay.media.play"));
        assert!(html.contains("playbackStarted"));
        assert!(html.contains("playbackCompleted"));
        assert!(html.contains("receiptPhase"));
    }
}
