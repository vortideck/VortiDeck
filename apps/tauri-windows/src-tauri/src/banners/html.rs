/// Banner HTML generation module
/// Provides the HTML template for banner display in OBS

pub fn generate_banner_html() -> String {
    r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="Cache-Control" content="no-cache, no-store, must-revalidate">
    <meta http-equiv="Pragma" content="no-cache">
    <meta http-equiv="Expires" content="0">
    <title>VortiDeck Banner</title>
    <style id="banner-styles">
        /* Reset and base styles */
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            background: transparent;
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            overflow: hidden;
            width: 100vw;
            height: 100vh;
        }
        
        #banner-container {
            position: absolute;
            width: 100%;
            height: 100%;
            display: flex;
            align-items: flex-end;
            justify-content: center;
            pointer-events: none;
        }
        
        .banner {
            position: absolute;
            left: 50%;
            bottom: 0;
            pointer-events: auto;
            transition: transform 0.3s ease, opacity 0.3s ease;
            opacity: 1;
            transform: translate(-50%, 100%);
        }
        .banner.active { opacity: 1; }
        
        /* Highly responsive banner styles */
        .banner-content {
            position: relative;
            background: linear-gradient(135deg, rgba(102, 126, 234, 0.95) 0%, rgba(118, 75, 162, 0.95) 100%);
            color: white;
            /* Responsive padding - scales with viewport */
            padding: clamp(15px, 2vw, 40px) clamp(30px, 4vw, 80px);
            /* Responsive border radius */
            border-radius: clamp(10px, 1.5vw, 25px) clamp(10px, 1.5vw, 25px) 0 0;
            box-shadow: 0 -4px clamp(16px, 2vw, 40px) rgba(0, 0, 0, 0.3);
            text-align: center;
            /* Much more flexible width - works from mobile to 4K */
            min-width: clamp(300px, 40vw, 600px);
            max-width: clamp(600px, 70vw, 1200px);
            margin-bottom: 0;
        }

        /* When explicit size is provided by the JSON, do not clamp/ pad */
        .banner-content.fixed-size {
            padding: 0 !important;
            min-width: 0 !important;
            max-width: none !important;
            min-height: 0 !important;
            max-height: none !important;
        }
        
        .banner-content img {
            max-width: 100%;
            height: auto;
            /* Responsive border radius for images */
            border-radius: clamp(5px, 1vw, 15px);
            /* Responsive max height */
            max-height: clamp(80px, 8vh, 200px);
            object-fit: cover;
        }
        
        .banner-content h3 {
            /* Responsive font size - scales beautifully */
            font-size: clamp(18px, 2.5vw, 32px);
            margin-bottom: clamp(8px, 1vh, 15px);
            font-weight: 600;
            line-height: 1.2;
        }
        
        .banner-content p {
            /* Responsive paragraph text */
            font-size: clamp(14px, 1.8vw, 20px);
            opacity: 0.9;
            line-height: 1.4;
            margin: 0;
        }

        /* Creator-style animation system (vd-*) */
        .vd-anim { animation-fill-mode: both; }
        .vd-fade-in { animation-name: vd-fade-in-kf; }
        .vd-slide-up { animation-name: vd-slide-up-kf; }
        .vd-slide-down { animation-name: vd-slide-down-kf; }
        .vd-slide-left { animation-name: vd-slide-left-kf; }
        .vd-slide-right { animation-name: vd-slide-right-kf; }
        .vd-zoom-in { animation-name: vd-zoom-in-kf; }
        .vd-exit-fade-out { animation-name: vd-exit-fade-out-kf; }
        .vd-exit-down { animation-name: vd-exit-down-kf; }
        .vd-exit-up { animation-name: vd-exit-up-kf; }
        .vd-exit-left { animation-name: vd-exit-left-kf; }
        .vd-exit-right { animation-name: vd-exit-right-kf; }

        @keyframes vd-fade-in-kf { from { opacity: 0; } to { opacity: 1; } }
        @keyframes vd-slide-up-kf { from { transform: translateY(20px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
        @keyframes vd-slide-down-kf { from { transform: translateY(-20px); opacity: 0; } to { transform: translateY(0); opacity: 1; } }
        @keyframes vd-slide-left-kf { from { transform: translateX(20px); opacity: 0; } to { transform: translateX(0); opacity: 1; } }
        @keyframes vd-slide-right-kf { from { transform: translateX(-20px); opacity: 0; } to { transform: translateX(0); opacity: 1; } }
        @keyframes vd-zoom-in-kf { from { transform: scale(0.95); opacity: 0; } to { transform: scale(1); opacity: 1; } }

        @keyframes vd-exit-fade-out-kf { from { opacity: 1; } to { opacity: 0; } }
        @keyframes vd-exit-down-kf { from { transform: translateY(0); opacity: 1; } to { transform: translateY(20px); opacity: 0; } }
        @keyframes vd-exit-up-kf { from { transform: translateY(0); opacity: 1; } to { transform: translateY(-20px); opacity: 0; } }
        @keyframes vd-exit-left-kf { from { transform: translateX(0); opacity: 1; } to { transform: translateX(-20px); opacity: 0; } }
        @keyframes vd-exit-right-kf { from { transform: translateX(0); opacity: 1; } to { transform: translateX(20px); opacity: 0; } }

        /* Position Preset Classes (mirror creator preview) */
        /* Position presets EXACT like creator preview */
        .position-bottom-center .banner { left: 50%; bottom: -1px; transform: translate(-50%, 100%); }
        .position-bottom-left .banner { left: 0; bottom: -1px; transform: translate(0, 100%); }
        .position-bottom-right .banner { left: 100%; bottom: -1px; transform: translate(-100%, 100%); }
        .position-top-center .banner { left: 50%; bottom: auto; top: 0; transform: translate(-50%, -100%); }
        .position-top-left .banner { left: 0; bottom: auto; top: 0; transform: translate(0, -100%); }
        .position-top-right .banner { left: 100%; bottom: auto; top: 0; transform: translate(-100%, -100%); }
        .position-left-center .banner { left: 0; top: 50%; bottom: auto; transform: translate(-100%, -50%); }
        .position-right-center .banner { left: 100%; top: 50%; bottom: auto; transform: translate(100%, -50%); }
        .position-floating .banner { left: 50%; top: 50%; bottom: auto; transform: translate(-50%, -50%); }

        .position-bottom-center .banner.active { transform: translate(-50%, 0); }
        .position-bottom-left .banner.active { transform: translate(0, 0); }
        .position-bottom-right .banner.active { transform: translate(-100%, 0); }
        .position-top-center .banner.active { transform: translate(-50%, 0); }
        .position-top-left .banner.active { transform: translate(0, 0); }
        .position-top-right .banner.active { transform: translate(-100%, 0); }
        .position-left-center .banner.active { transform: translate(0, -50%); }
        .position-right-center .banner.active { transform: translate(-100%, -50%); }
        .position-floating .banner.active { transform: translate(-50%, -50%); }
        
        /* Entrance / Exit / Movement Animations */
        .enter-slideUp { animation: enterSlideUp 300ms ease forwards; }
        .enter-slideDown { animation: enterSlideDown 300ms ease forwards; }
        .enter-slideLeft { animation: enterSlideLeft 300ms ease forwards; }
        .enter-slideRight { animation: enterSlideRight 300ms ease forwards; }
        .enter-fadeIn { animation: enterFadeIn 300ms ease forwards; }

        .exit-slideDown { animation: exitSlideDown 300ms ease forwards; }
        .exit-slideUp { animation: exitSlideUp 300ms ease forwards; }
        .exit-slideLeft { animation: exitSlideLeft 300ms ease forwards; }
        .exit-slideRight { animation: exitSlideRight 300ms ease forwards; }
        .exit-fadeOut { animation: exitFadeOut 300ms ease forwards; }

        .move-float { animation: moveFloat 3s ease-in-out infinite alternate; }
        .move-pulse { animation: movePulse 1.8s ease-in-out infinite; }
        .move-sway { animation: moveSway 3.2s ease-in-out infinite; }
        .move-bounce { animation: moveBounce 1.5s cubic-bezier(.28,.84,.42,1) infinite; }

        @keyframes enterSlideUp {
            from { transform: translate(-50%, -30%); opacity: 0; }
            to { transform: translate(-50%, -50%); opacity: 1; }
        }
        @keyframes enterSlideDown {
            from { transform: translate(-50%, -70%); opacity: 0; }
            to { transform: translate(-50%, -50%); opacity: 1; }
        }
        @keyframes enterSlideLeft {
            from { transform: translate(-60%, -50%); opacity: 0; }
            to { transform: translate(-50%, -50%); opacity: 1; }
        }
        @keyframes enterSlideRight {
            from { transform: translate(-40%, -50%); opacity: 0; }
            to { transform: translate(-50%, -50%); opacity: 1; }
        }
        @keyframes enterFadeIn { from { opacity: 0; } to { opacity: 1; } }

        @keyframes exitSlideDown {
            from { transform: translate(-50%, -50%); opacity: 1; }
            to { transform: translate(-50%, -30%); opacity: 0; }
        }
        @keyframes exitSlideUp {
            from { transform: translate(-50%, -50%); opacity: 1; }
            to { transform: translate(-50%, -70%); opacity: 0; }
        }
        @keyframes exitSlideLeft {
            from { transform: translate(-50%, -50%); opacity: 1; }
            to { transform: translate(-60%, -50%); opacity: 0; }
        }
        @keyframes exitSlideRight {
            from { transform: translate(-50%, -50%); opacity: 1; }
            to { transform: translate(-40%, -50%); opacity: 0; }
        }
        @keyframes exitFadeOut { from { opacity: 1; } to { opacity: 0; } }

        @keyframes moveFloat { from { transform: translateY(-2%);} to { transform: translateY(2%);} }
        @keyframes movePulse { 0%,100% { transform: scale(1);} 50% { transform: scale(1.03);} }
        @keyframes moveSway { 0%,100% { transform: translateX(-2%);} 50% { transform: translateX(2%);} }
        @keyframes moveBounce { 0%,100% { transform: translateY(0);} 50% { transform: translateY(-4%);} }
        
        /* Enhanced responsive breakpoints */
        @media (max-width: 720px) {
            .banner-content {
                min-width: 280px;
                max-width: 90vw;
            }
        }
        
        @media (min-width: 2560px) {
            .banner-content {
                min-width: 800px;
                max-width: 1400px;
            }
        }
        
        /* Specific OBS canvas optimizations */
        @media (aspect-ratio: 16/9) {
            .banner-content {
                /* Optimize for standard 16:9 streams */
                max-width: 65vw;
            }
        }
        
        @media (aspect-ratio: 4/3) {
            .banner-content {
                /* Optimize for 4:3 streams */
                max-width: 75vw;
            }
        }
        
        /* Loading state */
        .loading {
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 40px;
        }
        
        .loading::after {
            content: '';
            width: 40px;
            height: 40px;
            border: 3px solid rgba(255, 255, 255, 0.3);
            border-top-color: white;
            border-radius: 50%;
            animation: spin 1s linear infinite;
        }
        
        @keyframes spin {
            to { transform: rotate(360deg); }
        }
        
        /* Error state */
        .error {
            background: rgba(231, 76, 60, 0.95);
            padding: 20px 40px;
            border-radius: 10px;
            color: white;
            text-align: center;
        }
        
    </style>
</head>
<body>
    <div id="banner-container">
        <!-- Banners will be added dynamically via WebSocket -->
    </div>
    
    <!-- Audio element for banner sound -->
    <audio id="banner-audio" preload="auto">
        <source src="data:audio/wav;base64,UklGRnoGAABXQVZFZm10IBAAAAABAAEAQB8AAEAfAAABAAgAZGF0YQoGAACBhYqFbF1fdJivrJBhNjVgodDbq2EcBj+a2/LDciUFLIHO8tiJNwgZaLvt559NEAxQp+PwtmMcBjiR1/LMeSwFJHfH8N2QQAoUXrTp66hVFApGn+L2u2w=" type="audio/wav">
    </audio>
    
    <!-- Dynamic audio element for custom ad audio -->
    <audio id="custom-audio" preload="auto autoplay" style="display: none;">
    </audio>
    
    <script>
        // Configuration - Banner-specific WebSocket URL  
        const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const WS_URL = protocol + '//' + window.location.host + '/ws/banner';
        const RECONNECT_DELAY = 3000;
        
        console.log('🔌 WebSocket URL: ' + WS_URL);
        
        // State
        let ws = null;
        let reconnectTimeout = null;
        let currentBanner = null;
        let hideTimeout = null;
        
        // DOM elements
        const container = document.getElementById('banner-container');
        
        // WebSocket connection
        function connect() {
            console.log('Connecting to VortiDeck Banner WebSocket...');
            
            ws = new WebSocket(WS_URL);
            
            ws.onopen = () => {
                console.log('Connected to VortiDeck');
                container.innerHTML = ''; // Clear loading state
                // No identification message - connect silently like overlay.html
            };
            
            ws.onmessage = (event) => {
                try {
                    const data = JSON.parse(event.data);
                    handleMessage(data);
                } catch (error) {
                    console.error('Failed to parse message:', error);
                }
            };
            
            ws.onerror = (error) => {
                console.error('WebSocket error:', error);
            };
            
            ws.onclose = () => {
                console.log('Disconnected from VortiDeck');
                
                // Attempt to reconnect
                clearTimeout(reconnectTimeout);
                reconnectTimeout = setTimeout(connect, RECONNECT_DELAY);
            };
        }
        
        // Message handlers
        function handleMessage(data) {
            console.log('Received message:', data);
            
            switch (data.type) {
                case 'SHOW_STANDARD_BANNER':
                    showStandardBanner(data.banner || data);
                    break;
                    
                case 'HIDE_BANNER':
                    hideBanner(data.banner_id);
                    break;
                    
                case 'BANNER_QUEUE_UPDATE':
                    handleQueueUpdate(data.queue);
                    break;
                    
                case 'BANNER_STYLE_UPDATE':
                    updateBannerStyles(data.styles);
                    break;
                    
                case 'BANNER_HTML_UPDATED':
                    // WebSocket message to trigger HTML/CSS refresh
                    console.log('📡 Banner HTML updated via WebSocket');
                    updateBannerContent();
                    break;
                    
                    
                case 'PING':
                    ws.send(JSON.stringify({ type: 'PONG' }));
                    break;
            }
        }
        
        // Banner display functions (legacy removed; using standardized banner format)
        function pickPrimaryAsset(assets) {
            if (!assets || !assets.length) return null;
            const priority = { 'html': 0, 'text': 1, 'video': 2, 'image': 3 };
            return assets.slice().sort((a, b) => (priority[a.type] ?? 99) - (priority[b.type] ?? 99))[0];
        }

        /* Note: Position Preset Classes are defined in creator preview CSS. */

        function deriveDurationSeconds(stdBanner, primary) {
            try {
                const kfs = (stdBanner.timeline && primary && stdBanner.timeline[primary.id]) || [];
                if (!kfs.length) return Math.max(1, Math.round(stdBanner.duration || 5));
                let start = null, end = null;
                for (let i = 0; i < kfs.length; i++) {
                    const props = (kfs[i].properties || {});
                    const visible = typeof props.visible === 'boolean' ? props.visible : undefined;
                    if (start === null && visible === true) start = kfs[i].time || 0;
                    if (start !== null && visible === false) { end = kfs[i].time || 0; break; }
                }
                if (start === null) start = 0;
                if (end === null) end = (stdBanner.duration || kfs[kfs.length - 1].time || 5);
                const seconds = Math.max(1, Math.round(end - start));
                return seconds;
            } catch (e) {
                console.warn('Failed deriving duration, using fallback', e);
                return Math.max(1, Math.round(stdBanner.duration || 5));
            }
        }

        function buildContentHtml(primary) {
            if (!primary) return `<div class="banner-content" style="position:relative; overflow:hidden;"><div class="std-media" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;">VortiDeck Banner</div></div>`;
            if (primary.type === 'image' && primary.url) {
                return `<div class="banner-content" style="position:relative; overflow:hidden;"><img class="std-media" style="width:100%;height:100%;object-fit:cover;object-position:center;display:block;" src="${primary.url}" alt="Banner" /></div>`;
            }
            if (primary.type === 'video') {
                const src = primary.url || '';
                return `<div class="banner-content" style="position:relative; overflow:hidden;"><video class="std-media" style="width:100%;height:100%;object-fit:cover;object-position:center;display:block;" autoplay muted loop><source src="${src}" /></video></div>`;
            }
            if (primary.type === 'text') {
                const text = primary.content || '';
                return `<div class="banner-content" style="position:relative; overflow:hidden;"><div class="std-media" style="width:100%;height:100%;display:flex;align-items:center;justify-content:center;"><div style="padding:16px 24px;">${text}</div></div></div>`;
            }
            // html
            const html = primary.content || '';
            return `<div class="banner-content" style="position:relative; overflow:hidden;"><div class="std-media" style="width:100%;height:100%;">${html}</div></div>`;
        }

        // Track scheduled timers for multi-step playback
        let stdTimers = [];
        function clearStdTimers() {
            try { stdTimers.forEach(t => clearTimeout(t)); } catch(_) {}
            stdTimers = [];
        }
        let exitHideTimer = null;
        let positionPresetName = '';
        let applyVersion = 0; // increments each time we apply a keyframe

        function normalizeAnimName(name) {
            if (!name) return null;
            const map = {
                slideUp: 'slideUp', slideDown: 'slideDown', slideLeft: 'slideLeft', slideRight: 'slideRight',
                fadeIn: 'fadeIn', fadeOut: 'fadeOut'
            };
            return map[name] || null;
        }

        function applyKeyframeStyles(wrapper, kf, stdBanner, inner, isLastFrame = false) {
            applyVersion++;
            const localVersion = applyVersion;
            const props = kf.properties || {};
            // Position preset alignment (mirrors creator placements for common presets)
            if (props.positionPreset && typeof props.positionPreset === 'string') {
                const body = document.body;
                const preset = String(props.positionPreset).replace(/_/g,'-');
                // Only change class if preset changed to avoid unwanted transitions
                if (positionPresetName !== preset) {
                    // Disable wrapper transition temporarily to avoid sliding from old corner
                    const prev = wrapper.style.transition;
                    wrapper.style.transition = 'none';
                    Array.from(body.classList).filter(c=>c.startsWith('position-')).forEach(c=>body.classList.remove(c));
                    body.classList.add('position-' + preset);
                    // Force reflow
                    try { void wrapper.offsetHeight; } catch(_) {}
                    // Restore transition
                    wrapper.style.transition = prev;
                    positionPresetName = preset;
                }
                // Clear inline positioning to let preset CSS control it
                wrapper.style.left = '';
                wrapper.style.top = '';
                wrapper.style.transform = '';
            } else if (props.position && typeof props.position.x === 'number' && typeof props.position.y === 'number') {
                // Fallback to normalized position
                const x = Math.max(0, Math.min(1, props.position.x));
                const y = Math.max(0, Math.min(1, props.position.y));
                wrapper.style.left = (x * 100) + '%';
                wrapper.style.top = (y * 100) + '%';
                wrapper.style.transform = 'translate(-50%, -50%)';
            }
            // Size: apply to content box so media fills properly
            if (props.size && props.size.mode) {
                const contentEl = inner ? inner.querySelector('.banner-content') : null;
                const target = contentEl || wrapper;
                // prevent clamping by base CSS when explicit sizes are provided
                if (contentEl) contentEl.classList.add('fixed-size');
                if (props.size.mode === 'responsive') {
                    if (typeof props.size.width === 'number') target.style.width = props.size.width + 'vw';
                    if (typeof props.size.height === 'number') target.style.height = props.size.height + 'vh';
                } else {
                    if (typeof props.size.width === 'number') target.style.width = props.size.width + 'px';
                    if (typeof props.size.height === 'number') target.style.height = props.size.height + 'px';
                }
            }
            // Orientation-driven corner rounding to mimic creator
            try {
                const contentEl = inner ? inner.querySelector('.banner-content') : null;
                if (contentEl) {
                    const radius = 'clamp(10px, 1.5vw, 25px)';
                    const preset = (props.positionPreset || '').toString();
                    const orient = (props.orientation || '').toString();
                    let br = `${radius} ${radius} 0 0`; // default like creator bottom banner
                    if (preset.includes('top')) {
                        // round bottom corners
                        br = `0 0 ${radius} ${radius}`;
                    } else if (preset.includes('left') && preset.includes('center')) {
                        // left side: round right corners if vertical, else keep top
                        br = orient === 'vertical' ? `0 ${radius} ${radius} 0` : `${radius} ${radius} 0 0`;
                    } else if (preset.includes('right') && preset.includes('center')) {
                        // right side: round left corners if vertical
                        br = orient === 'vertical' ? `${radius} 0 0 ${radius}` : `${radius} ${radius} 0 0`;
                    } else if (preset.includes('floating')) {
                        br = `${radius}`; // all corners
                    }
                    contentEl.style.borderRadius = br;
                }
            } catch(_) {}
            // Opacity / visibility
            if (typeof props.opacity === 'number') {
                wrapper.style.opacity = String(props.opacity);
            }
            if (typeof props.visible === 'boolean') {
                if (!props.visible) {
                    // Apply exit animation to inner content, not wrapper
                    const exitName = normalizeAnimName(kf.animations && kf.animations.exit);
                    if (exitName) {
                        const contentEl = inner ? inner.querySelector('.banner-content') : null;
                        const target = contentEl || wrapper;
                        
                        // Clear previous exit classes
                        target.classList.remove('vd-exit-fade-out','vd-exit-down','vd-exit-up','vd-exit-left','vd-exit-right');
                        
                        // Map to vd-* classes that work with any position
                        const exitMap = { 
                            fadeOut:'vd-exit-fade-out', 
                            slideDown:'vd-exit-down', 
                            slideUp:'vd-exit-up', 
                            slideLeft:'vd-exit-left', 
                            slideRight:'vd-exit-right' 
                        };
                        const exitClass = exitMap[exitName] || 'vd-exit-fade-out';
                        
                        target.classList.add('vd-anim', exitClass);
                        target.style.animationDuration = '300ms';
                        target.style.animationTimingFunction = 'ease-out';
                        target.style.animationFillMode = 'forwards';
                        
                        // Clean up after animation
                        setTimeout(() => {
                            target.classList.remove('vd-anim', exitClass);
                        }, 320);
                    }
                    // For final frame, hide after animation
                    if (isLastFrame) {
                        if (exitHideTimer) clearTimeout(exitHideTimer);
                        exitHideTimer = setTimeout(() => {
                            wrapper.style.display = 'none';
                        }, 350); // Slightly longer than CSS animation
                    }
                } else {
                    // Show the banner
                    wrapper.style.display = 'block';
                    wrapper.classList.add('active');
                    wrapper.style.opacity = '';
                    
                    // Clear exit classes from content when showing
                    const contentEl = inner ? inner.querySelector('.banner-content') : null;
                    if (contentEl) {
                        contentEl.classList.remove('vd-anim','vd-exit-fade-out','vd-exit-down','vd-exit-up','vd-exit-left','vd-exit-right');
                        contentEl.style.opacity = '';
                        contentEl.style.transform = '';
                    }
                    
                    // Clear any pending exit timer
                    if (exitHideTimer) {
                        clearTimeout(exitHideTimer);
                        exitHideTimer = null;
                    }
                }
            }
            // If visible not provided, ensure element is visible and active by default
            if (typeof props.visible === 'undefined') {
                wrapper.style.display = 'block';
                wrapper.classList.add('active');
            }
            // Scale
            if (inner && typeof props.scale === 'number') {
                inner.style.transform = 'scale(' + props.scale + ')';
            }
            // Media object-fit and object-position
            try {
                const media = wrapper.querySelector('img.std-media, video.std-media');
                if (media) {
                    if (props.assetFit) media.style.objectFit = String(props.assetFit);
                    if (props.assetPosition) media.style.objectPosition = String(props.assetPosition);
                }
            } catch(_) {}
            // Entrance and movement - target inner content to avoid moving the positioned wrapper
            const enterName = normalizeAnimName(kf.animations && kf.animations.entrance);
            const moveName = kf.animations && kf.animations.movement;
            try {
                const contentEl = inner ? inner.querySelector('.banner-content') : null;
                const target = contentEl || wrapper;
                if (enterName) {
                    // Map to creator-style vd-* classes and use exported timing if provided
                    const map = { slideUp:'vd-slide-up', slideDown:'vd-slide-down', slideLeft:'vd-slide-left', slideRight:'vd-slide-right', fadeIn:'vd-fade-in', zoomIn:'vd-zoom-in' };
                    const cls = map[enterName] || 'vd-fade-in';
                    const dur = (kf.animations && kf.animations.durationMs) || 600;
                    const delay = (kf.animations && kf.animations.delayMs) || 0;
                    const easing = (kf.animations && kf.animations.easing) || 'ease-out';
                    target.classList.add('vd-anim', cls);
                    target.style.animationDuration = dur + 'ms';
                    target.style.animationDelay = delay + 'ms';
                    target.style.animationTimingFunction = easing;
                    setTimeout(() => { target.classList.remove('vd-anim', cls); }, dur + delay + 20);
                }
                if (moveName) {
                    target.classList.remove('move-float','move-pulse','move-sway','move-bounce');
                    const map = { 'float':'move-float','pulse':'move-pulse','sway':'move-sway','bounce':'move-bounce' };
                    if (map[moveName]) target.classList.add(map[moveName]);
                }
            } catch(_) {}
        }

        function showStandardBanner(stdBanner) {
            console.log('Showing standard banner:', stdBanner);
            clearTimeout(hideTimeout);
            clearStdTimers();
            if (currentBanner) hideBanner();

            const primary = pickPrimaryAsset((stdBanner && stdBanner.assets) || []);
            const content = buildContentHtml(primary);

            // Build wrapper to control position/size over time
            const banner = document.createElement('div');
            banner.className = 'banner';
            banner.id = `banner-${stdBanner && stdBanner.id ? stdBanner.id : Date.now()}`;
            banner.style.position = 'absolute';
            banner.style.transition = 'all 300ms ease';

            const inner = document.createElement('div');
            inner.className = 'std-banner-wrapper';
            inner.style.position = 'relative';
            inner.style.width = 'auto';
            inner.style.height = 'auto';
            inner.style.transition = 'all 300ms ease';

            // Click URL support
            const clickUrl = stdBanner && stdBanner.globalSettings && stdBanner.globalSettings.clickUrl;
            if (clickUrl) {
                const a = document.createElement('a');
                a.href = clickUrl;
                a.target = '_blank';
                a.rel = 'noopener';
                a.innerHTML = content;
                inner.appendChild(a);
            } else {
                inner.innerHTML = content;
            }
            banner.appendChild(inner);
            
            // Add to container
            container.innerHTML = '';
            container.appendChild(banner);
            currentBanner = banner;
            
            // Audio (optional)
            const defaultAudio = document.getElementById('banner-audio');
            const customAudio = document.getElementById('custom-audio');
            if (defaultAudio) { defaultAudio.pause(); defaultAudio.currentTime = 0; }
            if (customAudio) { customAudio.pause(); customAudio.currentTime = 0; customAudio.src = ''; }
            const audioUrl = stdBanner && stdBanner.globalSettings && stdBanner.globalSettings.audioUrl;
            if (audioUrl && customAudio) {
                customAudio.src = audioUrl;
                customAudio.play().catch(()=>{});
            }

            // Multi-step playback: apply keyframes over time
            const kfs = (stdBanner && stdBanner.timeline && primary && stdBanner.timeline[primary.id]) || [];
            const sorted = kfs.slice().sort((a,b) => (a.time||0) - (b.time||0));
            const t0 = performance.now();
            const total = Math.max(stdBanner && stdBanner.duration ? (stdBanner.duration * 1000) : (sorted.length ? (sorted[sorted.length-1].time * 1000) : 5000), 1000);

            // Initial defaults
            if (sorted.length) {
                applyKeyframeStyles(banner, sorted[0], stdBanner, inner);
                // Activate after first application so preset CSS transitions into place without jumping
                requestAnimationFrame(() => { banner.classList.add('active'); });
            }
            // If no preset or position set on first frame, default inline bottom-center
            try {
                const first = sorted[0] || {};
                const props = first.properties || {};
                const hasPreset = typeof props.positionPreset === 'string';
                const hasNorm = props.position && typeof props.position.x === 'number' && typeof props.position.y === 'number';
                if (!hasPreset && !hasNorm) {
                    banner.style.left = '50%';
                    banner.style.right = '';
                    banner.style.top = '';
                    banner.style.bottom = '2%';
                    banner.style.transform = 'translate(-50%, 0)';
                }
            } catch {}

            // Apply keyframes with proper exit/entrance sequencing
            for (let i = 0; i < sorted.length; i++) {
                const kf = sorted[i];
                const baseDelay = Math.max(0, (kf.time || 0) * 1000);
                const isLastFrame = i === sorted.length - 1;
                
                // Check if this is an entrance that has a matching exit at the same time
                const isEntrance = kf.properties?.visible === true;
                const isExit = kf.properties?.visible === false;
                
                let actualDelay = baseDelay;
                
                // If this is an entrance and there's an exit at the same time, delay the entrance
                if (isEntrance && i > 0) {
                    const prevKf = sorted[i - 1];
                    const prevTime = (prevKf.time || 0) * 1000;
                    const prevIsExit = prevKf.properties?.visible === false;
                    
                    // If previous frame is an exit at the same time, delay entrance by 350ms
                    if (prevIsExit && Math.abs(baseDelay - prevTime) < 10) {
                        actualDelay = baseDelay + 350;
                    }
                }
                
                const timer = setTimeout(() => {
                    applyKeyframeStyles(banner, kf, stdBanner, inner, isLastFrame);
                }, actualDelay);
                stdTimers.push(timer);
            }

            // Auto-hide after total duration
            hideTimeout = setTimeout(() => { hideBanner(banner.id); clearStdTimers(); }, total);
        }

        // Legacy showBanner removed
        
        function hideBanner(bannerId) {
            console.log('Hiding banner:', bannerId);
            
            const banner = bannerId ? 
                document.getElementById(bannerId) : 
                currentBanner;
                
            if (banner) {
                banner.classList.remove('active');
                
                // Stop all audio when hiding banner
                const defaultAudio = document.getElementById('banner-audio');
                if (defaultAudio) {
                    defaultAudio.pause();
                    defaultAudio.currentTime = 0;
                }
                
                const customAudio = document.getElementById('custom-audio');
                if (customAudio) {
                    customAudio.pause();
                    customAudio.currentTime = 0;
                    customAudio.src = ''; // Clear the source
                }
                
                console.log('🔇 Stopped all banner audio');
                
                // Clean up custom styles
                const styleId = banner.getAttribute('data-style-id');
                if (styleId) {
                    const styleEl = document.getElementById(styleId);
                    if (styleEl) {
                        styleEl.remove();
                    }
                }
                
                // Remove after animation
                setTimeout(() => {
                    if (banner.parentNode) {
                        banner.parentNode.removeChild(banner);
                    }
                    if (banner === currentBanner) {
                        currentBanner = null;
                    }
                }, 300);
            }
            
            // Only clear timeout if this is a manual hide, not auto-hide
            if (bannerId) {
                clearTimeout(hideTimeout);
            }
        }
        
        function handleQueueUpdate(queue) {
            console.log('Banner queue updated:', queue);
            // Queue handling logic would go here
            // For now, we'll just show the first banner if no current banner
            if (!currentBanner && queue && queue.length > 0) {
                showBanner(queue[0]);
            }
        }
        
        function updateBannerStyles(styles) {
            const styleEl = document.getElementById('banner-styles');
            if (styleEl && styles) {
                styleEl.textContent = styles;
            }
        }
        
        function trackImpression(bannerData) {
            // Send impression tracking back to VortiDeck
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({
                    type: 'BANNER_IMPRESSION',
                    banner_id: bannerData.id,
                    campaign_id: bannerData.campaign_id,
                    timestamp: Date.now()
                }));
            }
        }
        
        // Error display
        function showError(message) {
            container.innerHTML = `
                <div class="error">
                    <h3>Connection Error</h3>
                    <p>${message}</p>
                </div>
            `;
        }
        
        
        // Initialize connection
        connect();
        
        // WebSocket-based auto-update system (like overlay)
        let updateThrottleTimer = null;
        
        function updateBannerContent() {
            // Throttle updates to prevent overwhelming OBS
            if (updateThrottleTimer) return;
            
            updateThrottleTimer = setTimeout(() => {
                updateThrottleTimer = null;
            }, 100); // Throttle to max 10 updates per second
            
            console.log('🔄 Refreshing banner content via WebSocket...');
            
            // Fetch new banner HTML
            fetch('/banners')
                .then(response => response.text())
                .then(html => {
                    // Parse the new HTML
                    const parser = new DOMParser();
                    const doc = parser.parseFromString(html, 'text/html');
                    
                    // Update styles
                    const newStyles = doc.querySelector('#banner-styles');
                    const currentStyles = document.querySelector('#banner-styles');
                    if (newStyles && currentStyles) {
                        currentStyles.innerHTML = newStyles.innerHTML;
                        console.log('✅ Banner styles updated');
                    }
                    
                    // Update body content (preserve existing banners but allow structure updates)
                    const newBody = doc.body;
                    const currentContainer = document.querySelector('#banner-container');
                    const newContainer = newBody.querySelector('#banner-container');
                    
                    if (newContainer && currentContainer) {
                        // Check if structure changed
                        if (newContainer.innerHTML !== currentContainer.innerHTML) {
                            console.log('✅ Banner structure updated');
                            // You can update structure here if needed, but preserve active banners
                        }
                    }
                    
                    console.log('✅ WebSocket banner content refresh complete');
                })
                .catch(e => console.error('Failed to update banner content:', e));
        }
        
        // Banner system ready - waiting for WebSocket messages from debug panel
        console.log('🎬 Banner system ready - waiting for ads from control panel...');
        
        // Handle page visibility changes
        document.addEventListener('visibilitychange', () => {
            if (document.hidden && hideTimeout) {
                // Pause hide timeout when page is hidden
                clearTimeout(hideTimeout);
            }
        });
        
        // Cleanup on unload
        window.addEventListener('beforeunload', () => {
            clearTimeout(reconnectTimeout);
            clearTimeout(hideTimeout);
            if (ws) {
                ws.close();
            }
        });
    </script>
</body>
</html>"#.to_string()
}
