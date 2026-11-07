import { ref } from 'vue'
import { mediaTemplate } from './mediaTemplate'
/**
 * Component Schema System — Phase 1
 *
 * Each overlay component declares its own schema:
 *   - template: HTML with var(--c-*) references
 *   - cssVars: what CSS vars it uses + defaults + metadata
 *   - editableProps: what the Properties panel shows
 *
 * Canvas: --c-* vars are set on the wrapper div, browser cascades to children.
 * OBS:    resolveAllVars resolves --c-* → concrete values before sending HTML.
 */

// ── Types ────────────────────────────────────────────────────────

export interface CssVarDef {
  default: string            // Default value (usually a var(--theme-*) reference)
  label: string              // Display label in Properties panel
  type: 'color' | 'size' | 'font' | 'string' | 'gradient'
}

export interface EditableProp {
  key: string                // Internal key (used for storage)
  label: string              // Display label
  type: 'color' | 'range' | 'select' | 'text' | 'number' | 'font'
  cssVar?: string            // Which CSS var it controls → writes to element.style
  styleProp?: string         // Direct style property it controls (e.g. 'fontSize')
  dataProp?: string          // element.data key → writes to element.data.key (design-time content)
  options?: string[]         // For select type
  min?: number               // For range/number
  max?: number
  step?: number
  unit?: string              // e.g. 'px', '%'
  defaultValue?: string | number
}

export interface ComponentSchema {
  id: string                 // Unique registry key (e.g., 'follower')
  name: string               // Display name
  type: string               // Element type (e.g., 'alert', 'text', 'counter')
  category: 'basic' | 'widget' | 'graphic' | 'alert' | 'effect'
  width: number
  height: number

  // HTML template with var(--c-*) and {data.*} references
  template: string

  // CSS variables this component uses — schema-driven, not global
  cssVars: Record<string, CssVarDef>

  // Properties panel fields — auto-generated from this array
  editableProps: EditableProp[]

  // Default wrapper styles (also use var(--c-*))
  defaultStyle: Record<string, string>

  // Default design-time data values — populates element.data on creation.
  // Template {data.key} placeholders are substituted from element.data at render time.
  // {event.key} placeholders are reserved for the future Event Bus.
  defaultData?: Record<string, string | number>

  // Optional plain text content (for simple text elements)
  content?: string
}

// ── Shared CSS var presets ─────────────────────────────────────

/** Common CSS vars shared by most components */
const COMMON_VARS = {
  '--c-bg': { default: 'var(--theme-card-bg)', label: 'Background', type: 'color' as const },
  '--c-bg-alt': { default: 'var(--theme-surface)', label: 'Surface', type: 'color' as const },
  '--c-text': { default: 'var(--theme-text)', label: 'Text Color', type: 'color' as const },
  '--c-muted': { default: 'var(--theme-muted)', label: 'Muted Text', type: 'color' as const },
  '--c-font': { default: 'var(--theme-font-primary)', label: 'Font', type: 'font' as const },
  '--c-radius': { default: 'var(--theme-border-radius)', label: 'Border Radius', type: 'size' as const },
  '--c-border': { default: 'var(--theme-border-thin)', label: 'Border', type: 'string' as const },
  '--c-padding': { default: 'var(--theme-spacing-lg, 16px)', label: 'Padding', type: 'size' as const },
}

/** Color palette vars (for widgets that use accent/primary/secondary) */
const PALETTE_VARS = {
  '--c-primary': { default: 'var(--theme-primary)', label: 'Primary', type: 'color' as const },
  '--c-secondary': { default: 'var(--theme-secondary)', label: 'Secondary', type: 'color' as const },
  '--c-accent': { default: 'var(--theme-accent)', label: 'Accent', type: 'color' as const },
}

/** Shadow/glow vars */
const SHADOW_VARS = {
  '--c-shadow': { default: 'var(--theme-shadow)', label: 'Shadow', type: 'string' as const },
  '--c-shadow-sm': { default: 'var(--theme-shadow-sm)', label: 'Small Shadow', type: 'string' as const },
}

// ── Shared editable prop presets ───────────────────────────────

const bgColorProp: EditableProp = { key: 'bg', label: 'Background', type: 'color', cssVar: '--c-bg' }
const textColorProp: EditableProp = { key: 'text', label: 'Text Color', type: 'color', cssVar: '--c-text' }
const radiusProp: EditableProp = { key: 'radius', label: 'Border Radius', type: 'range', cssVar: '--c-radius', min: 0, max: 50, unit: 'px' }
const borderWidthProp: EditableProp = { key: 'borderWidth', label: 'Border Width', type: 'range', styleProp: 'borderWidth', min: 0, max: 10, unit: 'px', defaultValue: 1 }
const fontProp: EditableProp = { key: 'font', label: 'Font Family', type: 'font', cssVar: '--c-font' }
const opacityProp: EditableProp = { key: 'opacity', label: 'Opacity', type: 'range', styleProp: 'opacity', min: 0, max: 100, step: 1, defaultValue: 100 }
const animSpeedProp: EditableProp = { key: 'animSpeed', label: 'Speed', type: 'select', cssVar: '--c-anim-dur', options: ['0.8s', '1.6s', '2.4s', '4s'], defaultValue: '2.4s' }

interface StreamAlertOptions {
  id: string
  name: string
  eventType: string
  marker: string
  eyebrow: string
  title: string
  detail: string
}

/**
 * All standard stream alerts share one renderer and customization contract.
 * Keeping the visual shell here prevents five near-identical templates from
 * drifting while each alert still owns its event semantics and copy.
 */
const createStreamAlertSchema = (options: StreamAlertOptions): ComponentSchema => ({
  id: options.id,
  name: options.name,
  type: 'alert',
  category: 'alert',
  width: 360,
  height: 112,
  template: `<div class="stream-alert" data-event-type="${options.eventType}"><div class="alert-rail"></div><div class="alert-mark" aria-hidden="true">${options.marker}</div><div class="alert-copy"><div class="alert-eyebrow"></div><div class="alert-title"></div><div class="alert-detail"></div></div><div class="alert-live">Live</div></div>
    <style>
      *{box-sizing:border-box}.stream-alert{position:relative;width:100%;height:100%;display:grid;grid-template-columns:4px 42px minmax(0,1fr) auto;align-items:center;gap:13px;overflow:hidden;padding:14px 15px 14px 0;border-radius:inherit;background:linear-gradient(115deg,color-mix(in srgb,var(--c-bg) 96%,transparent),color-mix(in srgb,var(--c-bg) 88%,var(--c-primary)));font-family:var(--c-font);color:var(--c-text)}
      .alert-rail{align-self:stretch;border-radius:0 999px 999px 0;background:linear-gradient(180deg,var(--c-primary),var(--c-secondary))}.alert-mark{display:grid;width:42px;height:42px;place-items:center;border:1px solid color-mix(in srgb,var(--c-primary) 36%,transparent);border-radius:13px;background:color-mix(in srgb,var(--c-primary) 14%,transparent);color:var(--c-text);font-size:10px;font-weight:850;letter-spacing:.08em}.alert-copy{min-width:0}.alert-eyebrow{color:var(--c-primary);font-size:9px;font-weight:800;letter-spacing:.13em;text-transform:uppercase}.alert-title{margin-top:3px;overflow:hidden;font-size:16px;font-weight:800;line-height:1.15;text-overflow:ellipsis;white-space:nowrap}.alert-detail{margin-top:4px;overflow:hidden;color:var(--c-muted);font-size:10px;line-height:1.2;text-overflow:ellipsis;white-space:nowrap}.alert-live{align-self:start;padding:4px 7px;border:1px solid color-mix(in srgb,var(--c-text) 12%,transparent);border-radius:999px;color:var(--c-muted);font-size:8px;font-weight:750;letter-spacing:.1em;text-transform:uppercase}
      .stream-alert.is-fresh{animation:alert-enter .48s cubic-bezier(.16,1,.3,1)}@keyframes alert-enter{from{opacity:0;transform:translateY(8px) scale(.985)}to{opacity:1;transform:none}}
    </style>
    <script>
      (function(){
        const root=document.querySelector('.stream-alert');const eyebrow=root.querySelector('.alert-eyebrow');const title=root.querySelector('.alert-title');const detail=root.querySelector('.alert-detail');let data={};
        function person(event){return String(event.userDisplayName||event.userName||event.username||event.raider||event.channelName||data.title||'Community member')}
        function render(){eyebrow.textContent=String(data.eyebrow||'${options.eyebrow}');title.textContent=String(data.title||'${options.title}');detail.textContent=String(data.detail||'${options.detail}')}
        function merge(next){data=Object.assign({},data,next||{});render()}
        function accept(event){if(!event||event.type!==root.dataset.eventType)return;const name=person(event);const amount=Number(event.amount||0);const viewers=Number(event.viewerCount||0);const eventDetail=event.type==='bits'&&amount?amount.toLocaleString()+' bits':event.type==='raid'&&viewers?viewers.toLocaleString()+' viewers joined':data.detail;merge({title:name,detail:eventDetail});root.classList.remove('is-fresh');void root.offsetWidth;root.classList.add('is-fresh')}
        merge(window.vortideckData);window.addEventListener('vortideck-data-update',e=>merge(e.detail));window.addEventListener('vortideck-stream-event',e=>accept(e.detail));
      })();
    </script>`,
  cssVars: {
    ...COMMON_VARS,
    ...PALETTE_VARS,
    '--c-shadow-sm': SHADOW_VARS['--c-shadow-sm'],
  },
  editableProps: [
    { key: 'eyebrow', label: 'Event Label', type: 'text', dataProp: 'eyebrow', defaultValue: options.eyebrow },
    { key: 'title', label: 'Preview Name', type: 'text', dataProp: 'title', defaultValue: options.title },
    { key: 'detail', label: 'Supporting Text', type: 'text', dataProp: 'detail', defaultValue: options.detail },
    bgColorProp,
    textColorProp,
    { key: 'primary', label: 'Accent Start', type: 'color', cssVar: '--c-primary' },
    { key: 'secondary', label: 'Accent End', type: 'color', cssVar: '--c-secondary' },
    radiusProp,
    fontProp,
    borderWidthProp,
  ],
  defaultStyle: {
    backgroundColor: 'var(--c-bg)',
    borderRadius: 'var(--c-radius)',
    border: 'var(--c-border)',
    boxShadow: 'var(--c-shadow-sm)',
    padding: '0px',
  },
  defaultData: {
    eyebrow: options.eyebrow,
    title: options.title,
    detail: options.detail,
  },
})

const createCameraPlaceholderSchema = (
  id: 'webcam' | 'webcam-frame',
  name: string,
): ComponentSchema => ({
  id,
  name,
  type: 'webcam',
  category: id === 'webcam' ? 'widget' : 'graphic',
  width: 320,
  height: 240,
  template: `<div class="camera-placeholder"><div class="camera-glyph" aria-hidden="true"><span></span></div><strong>Camera source</strong><small>Choose a video source in properties</small></div><style>*{box-sizing:border-box}.camera-placeholder{width:100%;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;border-radius:inherit;background:linear-gradient(145deg,var(--c-bg-alt),color-mix(in srgb,var(--c-bg-alt) 86%,var(--c-primary)));font-family:var(--c-font);color:var(--c-text);text-align:center}.camera-glyph{position:relative;width:42px;height:30px;margin-bottom:13px;border:1px solid color-mix(in srgb,var(--c-text) 24%,transparent);border-radius:9px;background:color-mix(in srgb,var(--c-text) 5%,transparent)}.camera-glyph:after{content:'';position:absolute;right:-9px;top:8px;width:10px;height:14px;border-radius:3px;background:var(--c-primary);clip-path:polygon(0 22%,100% 0,100% 100%,0 78%)}.camera-glyph span{position:absolute;left:50%;top:50%;width:11px;height:11px;transform:translate(-50%,-50%);border:2px solid var(--c-primary);border-radius:50%}.camera-placeholder strong{font-size:13px;font-weight:750}.camera-placeholder small{margin-top:5px;color:var(--c-muted);font-size:10px}</style>`,
  cssVars: {
    ...COMMON_VARS,
    '--c-primary': PALETTE_VARS['--c-primary'],
    '--c-shadow-sm': SHADOW_VARS['--c-shadow-sm'],
  },
  editableProps: [
    { key: 'primary', label: 'Accent', type: 'color', cssVar: '--c-primary' },
    radiusProp,
    borderWidthProp,
  ],
  defaultStyle: {
    border: 'var(--c-border)',
    borderRadius: 'var(--c-radius)',
    backgroundColor: 'var(--c-bg-alt)',
    boxShadow: id === 'webcam-frame' ? 'var(--c-shadow-sm)' : 'none',
    padding: '0px',
  },
})

// ── Component Registry ─────────────────────────────────────────

export const componentRegistry: Record<string, ComponentSchema> = {

  // ── Basic Elements ──────────────────────────────────────────

  text: {
    id: 'text',
    name: 'Text',
    type: 'text',
    category: 'basic',
    width: 200,
    height: 60,
    content: 'Edit this text',
    template: '',  // text elements use content, not template
    cssVars: {
      '--c-text': COMMON_VARS['--c-text'],
      '--c-font': COMMON_VARS['--c-font'],
    },
    editableProps: [
      textColorProp,
      fontProp,
      { key: 'fontSize', label: 'Font Size', type: 'range', styleProp: 'fontSize', min: 8, max: 72, unit: 'px', defaultValue: 16 },
      { key: 'fontWeight', label: 'Bold', type: 'select', styleProp: 'fontWeight', options: ['normal', 'bold', '300', '500', '700'] },
      { key: 'textAlign', label: 'Align', type: 'select', styleProp: 'textAlign', options: ['left', 'center', 'right'] },
    ],
    defaultStyle: {
      fontSize: '16px',
      fontFamily: 'var(--c-font)',
      fontWeight: 'normal',
      color: 'var(--c-text)',
      textAlign: 'left',
      backgroundColor: 'transparent',
      borderStyle: 'none',
    },
  },

  image: {
    id: 'image',
    name: 'Image',
    type: 'image',
    category: 'basic',
    width: 200,
    height: 150,
    template: mediaTemplate('image'),
    cssVars: {
      '--c-bg-alt': COMMON_VARS['--c-bg-alt'],
      '--c-muted': COMMON_VARS['--c-muted'],
      '--c-radius': COMMON_VARS['--c-radius'],
    },
    editableProps: [
      { key: 'src', label: 'Image URL or embedded image', type: 'text', dataProp: 'src' },
      { key: 'alt', label: 'Description', type: 'text', dataProp: 'alt' },
      { key: 'objectFit', label: 'Fit', type: 'select', dataProp: 'objectFit', options: ['contain', 'cover', 'fill'] },
      radiusProp,
      opacityProp,
    ],
    defaultStyle: {
      borderRadius: 'var(--c-radius)',
      overflow: 'hidden',
    },
    defaultData: { src: '', alt: '', objectFit: 'contain' },
  },

  shape: {
    id: 'shape',
    name: 'Shape',
    type: 'rectangle',
    category: 'basic',
    width: 160,
    height: 120,
    template: '',
    cssVars: {
      '--c-bg-alt': COMMON_VARS['--c-bg-alt'],
      '--c-radius': COMMON_VARS['--c-radius'],
      '--c-border': COMMON_VARS['--c-border'],
    },
    editableProps: [
      { key: 'bg', label: 'Fill Color', type: 'color', cssVar: '--c-bg-alt' },
      radiusProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg-alt)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
    },
  },

  video: {
    id: 'video',
    name: 'Video',
    type: 'video',
    category: 'basic',
    width: 320,
    height: 180,
    template: mediaTemplate('video'),
    cssVars: {
      '--c-bg-alt': COMMON_VARS['--c-bg-alt'],
      '--c-radius': COMMON_VARS['--c-radius'],
      '--c-muted': COMMON_VARS['--c-muted'],
    },
    editableProps: [
      { key: 'src', label: 'Video URL (HTTPS)', type: 'text', dataProp: 'src' },
      { key: 'objectFit', label: 'Fit', type: 'select', dataProp: 'objectFit', options: ['contain', 'cover', 'fill'] },
      { key: 'loop', label: 'Loop (muted)', type: 'select', dataProp: 'loop', options: ['on', 'off'] },
      { key: 'surface', label: 'Surface', type: 'color', cssVar: '--c-bg-alt' },
      radiusProp,
      opacityProp,
    ],
    defaultStyle: {
      borderRadius: 'var(--c-radius)',
      overflow: 'hidden',
      backgroundColor: 'var(--c-bg-alt)',
    },
    defaultData: { src: '', objectFit: 'contain', loop: 'on' },
  },

  // ── Stream Widgets ──────────────────────────────────────────

  chat: {
    id: 'chat',
    name: 'Chat Box',
    type: 'chat',
    category: 'widget',
    width: 360,
    height: 300,
    template: `<div class="chat-widget">
      <header><span class="status-dot"></span><span class="chat-title">{data.title}</span></header>
      <div class="messages" aria-live="polite"></div>
      <div class="empty-state">Messages will appear when chat is live</div>
    </div>
    <style>
      *{box-sizing:border-box}.chat-widget{width:100%;height:100%;display:flex;flex-direction:column;overflow:hidden;font-family:var(--c-font);color:var(--c-text)}
      header{display:flex;align-items:center;gap:8px;padding:13px 15px;border-bottom:1px solid color-mix(in srgb,var(--c-text) 12%,transparent);font-size:11px;font-weight:700;letter-spacing:.08em;text-transform:uppercase}
      .status-dot{width:7px;height:7px;border-radius:50%;background:var(--c-accent);box-shadow:0 0 10px var(--c-accent)}
      .messages{display:flex;flex:1;flex-direction:column;justify-content:flex-end;gap:7px;min-height:0;padding:12px 14px;overflow:hidden}
      .chat-msg{padding:7px 10px;border-radius:8px;background:color-mix(in srgb,var(--c-text) 6%,transparent);font-size:13px;line-height:1.35;overflow-wrap:anywhere;animation:chat-in .22s ease-out}
      .author{margin-right:6px;font-weight:700}.empty-state{margin:auto;padding:0 20px;color:var(--c-muted);font-size:12px;text-align:center}
      @keyframes chat-in{from{opacity:0;transform:translateY(6px)}to{opacity:1;transform:none}}
    </style>
    <script>
      (function() {
        const root = document.querySelector('.chat-widget');
        const box = root.querySelector('.messages');
        const empty = root.querySelector('.empty-state');
        const title = root.querySelector('.chat-title');
        const seen = new Set();

        function addMessage(msg) {
          if (!msg) return;
          const id = String(msg.eventId || msg.event_id || msg.id || [msg.accountId || msg.account_id || '', msg.occurredAt || msg.timestamp || '', msg.userDisplayName || msg.user_display_name || msg.userName || msg.user_name || '', msg.message || ''].join(':'));
          if (seen.has(id)) return;
          seen.add(id);
          const div = document.createElement('div');
          div.className = 'chat-msg';
          const author = document.createElement('span');
          author.className = 'author';
          author.style.color = msg.platformColor || msg.platform_color || 'var(--c-accent)';
          author.textContent = (msg.userDisplayName || msg.user_display_name || msg.userName || msg.user_name || 'Viewer') + ':';
          div.append(author, document.createTextNode(String(msg.message || '')));
          box.appendChild(div);
          empty.hidden = true;
          const limit = Math.max(1, Number(window.vortideckData?.maxMessages || 8));
          while (box.children.length > limit) {
            box.removeChild(box.firstChild);
          }
          box.scrollTop = box.scrollHeight;
        }

        function update(data) {
          if (!data) return;
          if (title) title.textContent = String(data.title || 'Live chat');
          if (Array.isArray(data.chatHistory)) data.chatHistory.forEach(addMessage);
          if (data.chatMessage) addMessage(data.chatMessage);
        }
        update(window.vortideckData);
        window.addEventListener('vortideck-data-update', event => update(event.detail));
        window.addEventListener('vortideck-stream-event', (event) => {
          const p = event.detail;
          if (p?.type === 'chatMessage') addMessage(p);
        });
      })();
    </script>`,
    cssVars: {
      ...COMMON_VARS,
      ...PALETTE_VARS,
    },
    editableProps: [
      { key: 'title', label: 'Title', type: 'text', dataProp: 'title', defaultValue: 'Live chat' },
      { key: 'maxMessages', label: 'Visible Messages', type: 'number', dataProp: 'maxMessages', min: 1, max: 30, defaultValue: 8 },
      bgColorProp,
      textColorProp,
      radiusProp,
      fontProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      color: 'var(--c-text)',
      padding: 'var(--c-padding)',
    },
    defaultData: {
      title: 'Live chat',
      maxMessages: 8,
    },
  },

  goal: {
    id: 'goal',
    name: 'Goal Bar',
    type: 'progress_bar',
    category: 'widget',
    width: 340,
    height: 104,
    template: `<div class="goal-widget"><div class="goal-top"><div><div class="goal-label"></div><div class="goal-caption"></div></div><div class="goal-value"></div></div><div class="goal-track"><div class="goal-bar"></div></div><div class="goal-percent"></div></div>
    <style>
      *{box-sizing:border-box}.goal-widget{width:100%;height:100%;display:flex;flex-direction:column;justify-content:center;font-family:var(--c-font);color:var(--c-text)}
      .goal-top{display:flex;align-items:flex-end;justify-content:space-between;gap:12px}.goal-label{font-size:13px;font-weight:750}.goal-caption{margin-top:2px;color:var(--c-muted);font-size:10px;text-transform:uppercase;letter-spacing:.08em}.goal-value{font-size:14px;font-weight:750;font-variant-numeric:tabular-nums}
      .goal-track{height:9px;margin-top:11px;overflow:hidden;border-radius:999px;background:var(--c-bg-alt)}.goal-bar{height:100%;width:0;border-radius:inherit;background:linear-gradient(90deg,var(--c-primary),var(--c-secondary));box-shadow:0 0 14px color-mix(in srgb,var(--c-primary) 55%,transparent);transition:width .35s ease}
      .goal-percent{margin-top:5px;color:var(--c-muted);font-size:9px;text-align:right}
    </style>
    <script>
      (function() {
        const container = document.currentScript.parentElement;
        const bar = container.querySelector('.goal-bar');
        const labelEl = container.querySelector('.goal-label');
        const captionEl = container.querySelector('.goal-caption');
        const textEl = container.querySelector('.goal-value');
        const percentEl = container.querySelector('.goal-percent');
        let data = {};

        function update(data) {
          if (!data) return;
          const metric = (window.vortideckObservedValues || []).find(value => value.metric === data.goalType && value.platform === data.platform && String(value.accountId) === String(data.accountId));
          const observed = data.valueSource === 'observed' ? (window.vortideckRehearsal ? Number(window.vortideckRehearsalTotal || 0) : Number(metric?.total || 0)) : 0;
          const current = Math.max(0, Number(data.current || 0) + observed);
          const goal = Math.max(1, Number(data.goal || 1));
          const pct = Math.max(0, Math.min(100, Math.round((current / goal) * 100)));
          bar.style.width = pct + '%';
          textEl.textContent = current.toLocaleString() + ' / ' + goal.toLocaleString();
          labelEl.textContent = String(data.label || 'Community goal');
          captionEl.textContent = String(data.goalType || 'followers');
          percentEl.textContent = pct + '% complete';
        }

        function merge(next) { data = Object.assign({}, data, next || {}); update(data); }
        merge(window.vortideckData);
        window.addEventListener('vortideck-data-update', e => merge(e.detail));
        window.addEventListener('vortideck-stream-values', e => { window.vortideckObservedValues = e.detail; update(data); });
        window.addEventListener('vortideck-stream-event', e => {
          const event = e.detail;
          const matches = event && ((data.goalType === 'followers' && event.type === 'follow') || (data.goalType === 'subscribers' && event.type === 'subscription') || (data.goalType === 'bits' && event.type === 'bits'));
          if (!matches) return;
          // Real counts belong to the native accepted-event projection. Rehearsal
          // has its own resettable values and never writes that projection.
          if (!window.vortideckRehearsal || data.valueSource !== 'observed') return;
          const increment = event.type === 'bits' ? Number(event.amount || 0) : 1;
          window.vortideckRehearsalTotal = Number(window.vortideckRehearsalTotal || 0) + increment;
          update(data);
        });
      })();
    </script>`,
    cssVars: {
      ...COMMON_VARS,
      ...PALETTE_VARS,
    },
    defaultData: {
      label: 'Community goal',
      goalType: 'followers',
      goal: 1000,
      current: 750,
      valueSource: 'manual',
      platform: 'twitch',
      accountId: '',
    },
    editableProps: [
      { key: 'label', label: 'Title', type: 'text', dataProp: 'label', defaultValue: 'Community goal' },
      { key: 'goalType', label: 'Event Source', type: 'select', dataProp: 'goalType', options: ['followers', 'subscribers', 'bits'], defaultValue: 'followers' },
      { key: 'goal', label: 'Goal', type: 'number', dataProp: 'goal', defaultValue: 1000 },
      { key: 'current', label: 'Current', type: 'number', dataProp: 'current', defaultValue: 750 },
      { key: 'valueSource', label: 'Count source (observed = events accepted by VortiDeck)', type: 'select', dataProp: 'valueSource', options: ['manual', 'observed'], defaultValue: 'manual' },
      { key: 'platform', label: 'Observed platform', type: 'text', dataProp: 'platform', defaultValue: 'twitch' },
      { key: 'accountId', label: 'Observed account ID', type: 'text', dataProp: 'accountId', defaultValue: '' },
      bgColorProp,
      textColorProp,
      { key: 'primary', label: 'Bar Start', type: 'color', cssVar: '--c-primary' },
      { key: 'secondary', label: 'Bar End', type: 'color', cssVar: '--c-secondary' },
      radiusProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      padding: 'var(--c-padding)',
    },
  },

  recent: {
    id: 'recent',
    name: 'Recent Events',
    type: 'recent_events',
    category: 'widget',
    width: 340,
    height: 260,
    template: `<div class="activity-widget"><header><span class="activity-title"></span><span class="live-label">Live</span></header><div class="activity-list" aria-live="polite"></div><div class="activity-empty">New channel activity will appear here</div></div>
    <style>
      *{box-sizing:border-box}.activity-widget{width:100%;height:100%;display:flex;flex-direction:column;overflow:hidden;font-family:var(--c-font);color:var(--c-text)}
      header{display:flex;align-items:center;justify-content:space-between;padding-bottom:11px;border-bottom:1px solid color-mix(in srgb,var(--c-text) 12%,transparent)}.activity-title{font-size:13px;font-weight:750}.live-label{color:var(--c-primary);font-size:9px;font-weight:800;letter-spacing:.1em;text-transform:uppercase}
      .activity-list{display:flex;flex:1;flex-direction:column;justify-content:flex-start;gap:7px;min-height:0;padding-top:10px;overflow:hidden}.activity-item{display:grid;grid-template-columns:7px 1fr;gap:10px;align-items:center;padding:8px 9px;border-radius:8px;background:color-mix(in srgb,var(--c-text) 5%,transparent);animation:activity-in .22s ease-out}.activity-dot{width:7px;height:7px;border-radius:50%;background:var(--c-primary)}.activity-copy{min-width:0;font-size:12px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.activity-copy strong{font-weight:750}.activity-empty{margin:auto;color:var(--c-muted);font-size:11px;text-align:center}
      @keyframes activity-in{from{opacity:0;transform:translateX(-6px)}to{opacity:1;transform:none}}
    </style>
    <script>
      (function(){
        const root=document.querySelector('.activity-widget');const list=root.querySelector('.activity-list');const empty=root.querySelector('.activity-empty');const title=root.querySelector('.activity-title');let data={};
        function person(event){return String(event.userDisplayName||event.userName||event.raider||'A viewer')}
        function describe(event){if(event.type==='follow')return [person(event),'followed'];if(event.type==='subscription')return [person(event),'subscribed'];if(event.type==='raid')return [person(event),'raided with '+Number(event.viewerCount||0).toLocaleString()+' viewers'];if(event.type==='bits')return [person(event),'cheered '+Number(event.amount||0).toLocaleString()+' bits'];return null}
        function add(event){const description=describe(event);if(!description)return;const item=document.createElement('div');item.className='activity-item';const dot=document.createElement('span');dot.className='activity-dot';const copy=document.createElement('div');copy.className='activity-copy';const strong=document.createElement('strong');strong.textContent=description[0]+' ';copy.append(strong,document.createTextNode(description[1]));item.append(dot,copy);list.prepend(item);empty.hidden=true;while(list.children.length>Math.max(1,Number(data.maxItems||6)))list.lastChild.remove()}
        function merge(next){data=Object.assign({},data,next||{});title.textContent=String(data.title||'Recent activity')}
        merge(window.vortideckData);window.addEventListener('vortideck-data-update',e=>merge(e.detail));window.addEventListener('vortideck-stream-event',e=>add(e.detail||{}));
      })();
    </script>`,
    cssVars: {
      ...COMMON_VARS,
      '--c-primary': PALETTE_VARS['--c-primary'],
    },
    editableProps: [
      { key: 'title', label: 'Title', type: 'text', dataProp: 'title', defaultValue: 'Recent activity' },
      { key: 'maxItems', label: 'Visible Events', type: 'number', dataProp: 'maxItems', min: 1, max: 12, defaultValue: 6 },
      bgColorProp,
      textColorProp,
      { key: 'primary', label: 'Heading Color', type: 'color', cssVar: '--c-primary' },
      radiusProp,
      fontProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      color: 'var(--c-text)',
      padding: 'var(--c-padding)',
    },
    defaultData: {
      title: 'Recent activity',
      maxItems: 6,
    },
  },

  webcam: createCameraPlaceholderSchema('webcam', 'Webcam'),

  timer: {
    id: 'timer',
    name: 'Timer',
    type: 'timer',
    category: 'widget',
    width: 240,
    height: 104,
    template: `<div class="timer-widget"><div class="timer-label"></div><div class="timer-value">00:00</div><div class="timer-state"></div></div>
    <style>*{box-sizing:border-box}.timer-widget{width:100%;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;font-family:var(--c-font);color:var(--c-text)}.timer-label{color:var(--c-muted);font-size:9px;font-weight:750;letter-spacing:.12em;text-transform:uppercase}.timer-value{margin-top:3px;font-family:ui-monospace,SFMono-Regular,Consolas,monospace;font-size:30px;font-weight:800;line-height:1;font-variant-numeric:tabular-nums;text-shadow:0 0 16px color-mix(in srgb,var(--c-primary) 55%,transparent)}.timer-state{margin-top:7px;color:var(--c-primary);font-size:9px;font-weight:700;text-transform:uppercase}</style>
    <script>
      (function(){
        const root=document.querySelector('.timer-widget');const label=root.querySelector('.timer-label');const value=root.querySelector('.timer-value');const state=root.querySelector('.timer-state');let data={};let anchor=Date.now();let anchorSeconds=0;
        function seconds(){if(data.running!=='yes')return anchorSeconds;const elapsed=Math.floor((Date.now()-anchor)/1000);return data.mode==='countdown'?Math.max(0,anchorSeconds-elapsed):anchorSeconds+elapsed}
        function render(){const total=seconds();const hours=Math.floor(total/3600);const minutes=Math.floor((total%3600)/60);const secs=total%60;value.textContent=(hours?String(hours).padStart(2,'0')+':':'')+String(minutes).padStart(2,'0')+':'+String(secs).padStart(2,'0');label.textContent=String(data.label||'Stream timer');state.textContent=data.running==='yes'?(data.mode==='countdown'?'Countdown':'Elapsed'):'Paused'}
        function merge(next){const current=seconds();data=Object.assign({},data,next||{});anchorSeconds=Number(data.mode==='countdown'?data.durationMinutes*60:data.startSeconds??current)||0;anchor=Date.now();render()}
        merge(window.vortideckData);setInterval(render,250);window.addEventListener('vortideck-data-update',e=>merge(e.detail));
      })();
    </script>`,
    cssVars: {
      '--c-bg': COMMON_VARS['--c-bg'],
      '--c-text': COMMON_VARS['--c-text'],
      '--c-radius': COMMON_VARS['--c-radius'],
      '--c-border': COMMON_VARS['--c-border'],
      '--c-primary': PALETTE_VARS['--c-primary'],
    },
    editableProps: [
      { key: 'label', label: 'Label', type: 'text', dataProp: 'label', defaultValue: 'Stream timer' },
      { key: 'mode', label: 'Mode', type: 'select', dataProp: 'mode', options: ['elapsed', 'countdown'], defaultValue: 'elapsed' },
      { key: 'durationMinutes', label: 'Countdown Minutes', type: 'number', dataProp: 'durationMinutes', min: 1, max: 1440, defaultValue: 10 },
      { key: 'running', label: 'State', type: 'select', dataProp: 'running', options: ['yes', 'no'], defaultValue: 'yes' },
      bgColorProp,
      textColorProp,
      { key: 'primary', label: 'Glow Color', type: 'color', cssVar: '--c-primary' },
      radiusProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      padding: 'var(--c-padding)',
    },
    defaultData: {
      label: 'Stream timer',
      mode: 'elapsed',
      durationMinutes: 10,
      startSeconds: 0,
      running: 'yes',
    },
  },

  counter: {
    id: 'counter',
    name: 'Counter',
    type: 'counter',
    category: 'widget',
    width: 220,
    height: 104,
    template: `<div class="counter-widget"><div class="counter-label"></div><div class="counter-value"></div></div><style>*{box-sizing:border-box}.counter-widget{width:100%;height:100%;display:flex;flex-direction:column;align-items:center;justify-content:center;font-family:var(--c-font);color:var(--c-text)}.counter-label{color:var(--c-muted);font-size:10px;font-weight:700;letter-spacing:.1em;text-transform:uppercase}.counter-value{margin-top:4px;font-size:31px;font-weight:800;line-height:1;font-variant-numeric:tabular-nums}</style><script>(function(){const root=document.querySelector('.counter-widget');const label=root.querySelector('.counter-label');const value=root.querySelector('.counter-value');function update(data){label.textContent=String(data?.label||'Counter');const amount=Number(data?.value||0).toLocaleString();value.textContent=String(data?.prefix||'')+amount+String(data?.suffix||'')}update(window.vortideckData);window.addEventListener('vortideck-data-update',e=>update(e.detail))})();</script>`,
    cssVars: {
      ...COMMON_VARS,
    },
    editableProps: [
      { key: 'label', label: 'Label', type: 'text', dataProp: 'label', defaultValue: 'Counter' },
      { key: 'value', label: 'Value', type: 'number', dataProp: 'value', defaultValue: 0 },
      { key: 'prefix', label: 'Prefix', type: 'text', dataProp: 'prefix', defaultValue: '' },
      { key: 'suffix', label: 'Suffix', type: 'text', dataProp: 'suffix', defaultValue: '' },
      bgColorProp,
      textColorProp,
      radiusProp,
      fontProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      padding: 'var(--c-padding)',
    },
    defaultData: {
      label: 'Counter',
      value: 0,
      prefix: '',
      suffix: '',
    },
  },

  // ── Stream Graphics ─────────────────────────────────────────

  'webcam-frame': createCameraPlaceholderSchema('webcam-frame', 'Webcam Frame'),

  banner: {
    id: 'banner',
    name: 'Banner',
    type: 'text',
    category: 'graphic',
    width: 400,
    height: 50,
    content: 'Welcome to the stream!',
    template: '',
    cssVars: {
      ...COMMON_VARS,
      '--c-radius-sm': { default: 'var(--theme-border-radius-sm)', label: 'Radius', type: 'size' },
    },
    editableProps: [
      bgColorProp,
      textColorProp,
      fontProp,
      { key: 'fontSize', label: 'Font Size', type: 'range', styleProp: 'fontSize', min: 10, max: 48, unit: 'px', defaultValue: 16 },
      radiusProp,
      borderWidthProp,
    ],
    defaultStyle: {
      fontSize: '16px',
      fontFamily: 'var(--c-font)',
      fontWeight: 'bold',
      color: 'var(--c-text)',
      textAlign: 'center',
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius-sm)',
      border: 'var(--c-border)',
      padding: '0px',
    },
  },

  logo: {
    id: 'logo',
    name: 'Logo Watermark',
    type: 'image',
    category: 'graphic',
    width: 120,
    height: 120,
    template: `<div class="brand-mark"><span>{data.mark}</span></div><style>*{box-sizing:border-box}.brand-mark{width:100%;height:100%;display:grid;place-items:center;border:1px solid color-mix(in srgb,var(--c-text) 16%,transparent);border-radius:50%;background:color-mix(in srgb,var(--c-bg) 78%,transparent);font-family:var(--c-font);color:var(--c-text)}.brand-mark span{font-size:clamp(16px,30%,32px);font-weight:850;letter-spacing:.08em}</style>`,
    cssVars: {
      '--c-bg': COMMON_VARS['--c-bg'],
      '--c-text': COMMON_VARS['--c-text'],
      '--c-font': COMMON_VARS['--c-font'],
    },
    editableProps: [
      { key: 'mark', label: 'Brand Initials', type: 'text', dataProp: 'mark', defaultValue: 'VD' },
      bgColorProp,
      textColorProp,
      fontProp,
      opacityProp,
    ],
    defaultStyle: {
      opacity: '0.82',
    },
    defaultData: { mark: 'VD' },
  },

  social: {
    id: 'social',
    name: 'Social Media Bar',
    type: 'text',
    category: 'graphic',
    width: 300,
    height: 40,
    content: '@channel',
    template: `<div class="social-bar"><span class="social-label">Follow</span><span>{data.primary}</span><i></i><span>{data.secondary}</span></div><style>*{box-sizing:border-box}.social-bar{width:100%;height:100%;display:flex;align-items:center;gap:10px;overflow:hidden;padding:0 14px;border-radius:inherit;background:var(--c-bg);font-family:var(--c-font);font-size:11px;color:var(--c-text);white-space:nowrap}.social-label{color:var(--c-primary);font-size:9px;font-weight:800;letter-spacing:.12em;text-transform:uppercase}.social-bar i{width:3px;height:3px;flex:none;border-radius:50%;background:var(--c-muted)}</style>`,
    cssVars: {
      ...COMMON_VARS,
      '--c-primary': PALETTE_VARS['--c-primary'],
    },
    editableProps: [
      { key: 'primaryHandle', label: 'Primary Handle', type: 'text', dataProp: 'primary', defaultValue: '@channel' },
      { key: 'secondaryHandle', label: 'Secondary Handle', type: 'text', dataProp: 'secondary', defaultValue: 'youtube.com/@channel' },
      bgColorProp,
      textColorProp,
      { key: 'primary', label: 'Accent', type: 'color', cssVar: '--c-primary' },
      fontProp,
      radiusProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: '20px',
      border: 'var(--c-border)',
      padding: '0px',
    },
    defaultData: {
      primary: '@channel',
      secondary: 'youtube.com/@channel',
    },
  },

  'follower-counter-gradient': {
    id: 'follower-counter-gradient',
    name: 'Gradient Follower Counter',
    type: 'counter',
    category: 'graphic',
    width: 240,
    height: 104,
    template: `<div class="follower-widget"><div class="follower-mark"><span></span></div><div><div class="follower-label"></div><div class="follower-value"></div></div></div><style>*{box-sizing:border-box}.follower-widget{width:100%;height:100%;display:flex;align-items:center;gap:13px;font-family:var(--c-font);color:var(--c-text)}.follower-mark{display:grid;width:38px;height:38px;flex:none;place-items:center;border-radius:11px;background:color-mix(in srgb,var(--c-text) 14%,transparent)}.follower-mark span{width:10px;height:10px;border-radius:50%;background:var(--c-text);box-shadow:0 0 14px color-mix(in srgb,var(--c-text) 65%,transparent)}.follower-label{font-size:9px;font-weight:750;letter-spacing:.12em;text-transform:uppercase;opacity:.72}.follower-value{margin-top:2px;font-size:27px;font-weight:850;line-height:1;font-variant-numeric:tabular-nums}</style><script>(function(){const root=document.querySelector('.follower-widget');const label=root.querySelector('.follower-label');const value=root.querySelector('.follower-value');let data={};function render(){label.textContent=String(data.label||'Followers');value.textContent=Number(data.current||0).toLocaleString()}function merge(next){data=Object.assign({},data,next||{});render()}merge(window.vortideckData);window.addEventListener('vortideck-data-update',e=>merge(e.detail));window.addEventListener('vortideck-stream-event',e=>{if(e.detail?.type==='follow')merge({current:Number(data.current||0)+1})})})();</script>`,
    cssVars: {
      '--c-bg': { default: 'linear-gradient(135deg, var(--theme-primary), var(--theme-secondary))', label: 'Background', type: 'gradient' },
      '--c-text': COMMON_VARS['--c-text'],
      '--c-font': COMMON_VARS['--c-font'],
      '--c-radius': COMMON_VARS['--c-radius'],
      '--c-border': COMMON_VARS['--c-border'],
    },
    editableProps: [
      { key: 'label', label: 'Label', type: 'text', dataProp: 'label', defaultValue: 'Followers' },
      { key: 'current', label: 'Starting Count', type: 'number', dataProp: 'current', defaultValue: 12492 },
      bgColorProp,
      textColorProp,
      fontProp,
      radiusProp,
      borderWidthProp,
    ],
    defaultStyle: {
      background: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      padding: 'var(--c-padding)',
    },
    defaultData: {
      label: 'Followers',
      current: 12492,
    },
  },

  'donation-goal-neon': {
    id: 'donation-goal-neon',
    name: 'Neon Donation Goal',
    type: 'progress_bar',
    category: 'graphic',
    width: 340,
    height: 112,
    template: `<div class="cheer-widget"><div class="cheer-head"><div><div class="cheer-kicker">Community support</div><div class="cheer-label"></div></div><div class="cheer-value"></div></div><div class="cheer-track"><div class="cheer-bar"></div></div><div class="cheer-remaining"></div></div>
    <style>*{box-sizing:border-box}.cheer-widget{width:100%;height:100%;display:flex;flex-direction:column;justify-content:center;font-family:var(--c-font);color:var(--c-text)}.cheer-head{display:flex;align-items:flex-end;justify-content:space-between;gap:12px}.cheer-kicker{color:var(--c-muted);font-size:9px;font-weight:750;letter-spacing:.1em;text-transform:uppercase}.cheer-label{margin-top:2px;font-size:13px;font-weight:750}.cheer-value{font-size:15px;font-weight:800;font-variant-numeric:tabular-nums}.cheer-track{height:8px;margin-top:10px;overflow:hidden;border-radius:999px;background:var(--c-bg-alt);box-shadow:var(--c-shadow-sm)}.cheer-bar{width:0;height:100%;border-radius:inherit;background:linear-gradient(90deg,var(--c-secondary),var(--c-accent));box-shadow:0 0 15px color-mix(in srgb,var(--c-accent) 55%,transparent);transition:width .35s ease}.cheer-remaining{margin-top:5px;color:var(--c-muted);font-size:9px;text-align:right}</style>
    <script>
      (function() {
        const container = document.currentScript.parentElement;
        const bar = container.querySelector('.cheer-bar');const labelEl = container.querySelector('.cheer-label');const textEl = container.querySelector('.cheer-value');const remainingEl=container.querySelector('.cheer-remaining');let data={};
        function update() {
          const current=Math.max(0,Number(data.current||0));const goal=Math.max(1,Number(data.goal||1));const unit=String(data.unit||' bits');const pct=Math.max(0,Math.min(100,Math.round(current/goal*100)));
          bar.style.width=pct+'%';labelEl.textContent=String(data.label||'Cheer goal');textEl.textContent=current.toLocaleString()+' / '+goal.toLocaleString()+unit;remainingEl.textContent=Math.max(0,goal-current).toLocaleString()+unit+' to go';
        }
        function merge(next){data=Object.assign({},data,next||{});update()}
        merge(window.vortideckData);window.addEventListener('vortideck-data-update',e=>merge(e.detail));window.addEventListener('vortideck-stream-event',e=>{if(e.detail?.type==='bits')merge({current:Number(data.current||0)+Number(e.detail.amount||0)})});
      })();
    </script>`,
    cssVars: {
      ...COMMON_VARS,
      '--c-text': { default: 'var(--theme-secondary)', label: 'Text Color', type: 'color' as const },
      ...PALETTE_VARS,
      ...SHADOW_VARS,
    },
    defaultData: {
      label: 'Cheer goal',
      unit: ' bits',
      goal: 5000,
      current: 3200,
    },
    editableProps: [
      { key: 'label', label: 'Title', type: 'text', dataProp: 'label', defaultValue: 'Cheer goal' },
      { key: 'unit', label: 'Unit', type: 'text', dataProp: 'unit', defaultValue: ' bits' },
      { key: 'goal', label: 'Goal', type: 'number', dataProp: 'goal', defaultValue: 5000 },
      { key: 'current', label: 'Current', type: 'number', dataProp: 'current', defaultValue: 3200 },
      bgColorProp,
      textColorProp,
      { key: 'secondary', label: 'Bar Start', type: 'color', cssVar: '--c-secondary' },
      { key: 'accent', label: 'Bar End', type: 'color', cssVar: '--c-accent' },
      radiusProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      padding: 'var(--c-padding)',
    },
  },

  'subscriber-milestone': {
    id: 'subscriber-milestone',
    name: 'Subscriber Milestone',
    type: 'counter',
    category: 'graphic',
    width: 300,
    height: 112,
    template: `<div class="milestone-widget"><div class="milestone-label"></div><div class="milestone-row"><div class="milestone-text"></div><div class="milestone-percent"></div></div><div class="milestone-track"><div class="milestone-bar"></div></div></div><style>*{box-sizing:border-box}.milestone-widget{width:100%;height:100%;display:flex;flex-direction:column;justify-content:center;font-family:var(--c-font);color:var(--c-text)}.milestone-label{color:var(--c-accent);font-size:9px;font-weight:800;letter-spacing:.12em;text-transform:uppercase}.milestone-row{display:flex;align-items:flex-end;justify-content:space-between;margin-top:4px}.milestone-text{font-size:27px;font-weight:850;line-height:1;font-variant-numeric:tabular-nums}.milestone-percent{color:var(--c-muted);font-size:10px}.milestone-track{height:7px;margin-top:10px;overflow:hidden;border-radius:999px;background:var(--c-bg-alt)}.milestone-bar{width:0;height:100%;border-radius:inherit;background:var(--c-accent);box-shadow:0 0 14px color-mix(in srgb,var(--c-accent) 55%,transparent);transition:width .35s ease}</style>
    <script>
      (function() {
        const container = document.currentScript.parentElement;
        const bar = container.querySelector('.milestone-bar');
        const labelEl = container.querySelector('.milestone-label');
        const textEl = container.querySelector('.milestone-text');const percentEl=container.querySelector('.milestone-percent');let data={};
        function update() {
          const current=Math.max(0,Number(data.current||0));const goal=Math.max(1,Number(data.goal||1));const pct=Math.max(0,Math.min(100,Math.round(current/goal*100)));
          bar.style.width=pct+'%';textEl.textContent=current.toLocaleString()+' / '+goal.toLocaleString();labelEl.textContent=String(data.label||'Subscriber milestone');percentEl.textContent=pct+'%';
        }
        function merge(next){data=Object.assign({},data,next||{});update()}
        merge(window.vortideckData);window.addEventListener('vortideck-data-update',e=>merge(e.detail));window.addEventListener('vortideck-stream-event',e=>{if(e.detail?.type==='subscription')merge({current:Number(data.current||0)+1})});
      })();
    </script>`,
    cssVars: {
      ...COMMON_VARS,
      '--c-accent': PALETTE_VARS['--c-accent'],
    },
    defaultData: {
      label: 'Subscriber milestone',
      goal: 500,
      current: 247,
    },
    editableProps: [
      { key: 'label', label: 'Title', type: 'text', dataProp: 'label', defaultValue: 'Subscriber milestone' },
      { key: 'goal', label: 'Goal', type: 'number', dataProp: 'goal', defaultValue: 500 },
      { key: 'current', label: 'Current', type: 'number', dataProp: 'current', defaultValue: 247 },
      bgColorProp,
      textColorProp,
      { key: 'accent', label: 'Progress Color', type: 'color', cssVar: '--c-accent' },
      radiusProp,
      fontProp,
      borderWidthProp,
    ],
    defaultStyle: {
      backgroundColor: 'var(--c-bg)',
      borderRadius: 'var(--c-radius)',
      border: 'var(--c-border)',
      padding: 'var(--c-padding)',
    },
  },

  // ── Alerts ──────────────────────────────────────────────────

  follower: createStreamAlertSchema({
    id: 'follower', name: 'New Follower', eventType: 'follow', marker: 'FLW',
    eyebrow: 'New follower', title: 'Jordan joined', detail: 'Welcome to the community',
  }),

  subscriber: createStreamAlertSchema({
    id: 'subscriber', name: 'New Subscriber', eventType: 'subscription', marker: 'SUB',
    eyebrow: 'New subscriber', title: 'Morgan subscribed', detail: 'Thank you for the support',
  }),

  donation: createStreamAlertSchema({
    id: 'donation', name: 'Cheer Alert', eventType: 'bits', marker: 'BITS',
    eyebrow: 'New cheer', title: 'Alex cheered', detail: '500 bits',
  }),

  raid: createStreamAlertSchema({
    id: 'raid', name: 'Raid Alert', eventType: 'raid', marker: 'RAID',
    eyebrow: 'Incoming raid', title: 'StudioNorth', detail: '150 viewers joined',
  }),

  host: createStreamAlertSchema({
    id: 'host', name: 'Host Alert', eventType: 'host', marker: 'HOST',
    eyebrow: 'Channel host', title: 'NightShift Live', detail: 'is hosting the stream',
  }),

  // ── Effects / Animations ────────────────────────────────────

  particles: {
    id: 'particles',
    name: 'Particle Effects',
    type: 'effect',
    category: 'effect',
    width: 300,
    height: 200,
    template: `<style>@keyframes vd-particle-rise{0%{transform:translateY(0) scale(1);opacity:.85}100%{transform:translateY(-110px) scale(.2);opacity:0}}</style><div style="width:100%;height:100%;position:relative;overflow:hidden;background:transparent;border-radius:var(--c-radius);"><span style="position:absolute;left:8%;bottom:12%;width:6px;height:6px;border-radius:50%;background:var(--c-primary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in 0s infinite;"></span><span style="position:absolute;left:20%;bottom:8%;width:4px;height:4px;border-radius:50%;background:var(--c-secondary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in .3s infinite;"></span><span style="position:absolute;left:32%;bottom:20%;width:7px;height:7px;border-radius:50%;background:var(--c-primary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in .6s infinite;"></span><span style="position:absolute;left:44%;bottom:6%;width:5px;height:5px;border-radius:50%;background:var(--c-accent);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in .9s infinite;"></span><span style="position:absolute;left:56%;bottom:16%;width:4px;height:4px;border-radius:50%;background:var(--c-secondary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in 1.2s infinite;"></span><span style="position:absolute;left:68%;bottom:10%;width:6px;height:6px;border-radius:50%;background:var(--c-primary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in 1.5s infinite;"></span><span style="position:absolute;left:80%;bottom:22%;width:3px;height:3px;border-radius:50%;background:var(--c-accent);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in 1.8s infinite;"></span><span style="position:absolute;left:90%;bottom:8%;width:5px;height:5px;border-radius:50%;background:var(--c-primary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in 2.1s infinite;"></span><span style="position:absolute;left:14%;bottom:35%;width:4px;height:4px;border-radius:50%;background:var(--c-secondary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in .15s infinite;"></span><span style="position:absolute;left:38%;bottom:38%;width:5px;height:5px;border-radius:50%;background:var(--c-primary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in .45s infinite;"></span><span style="position:absolute;left:62%;bottom:30%;width:6px;height:6px;border-radius:50%;background:var(--c-accent);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in .75s infinite;"></span><span style="position:absolute;left:84%;bottom:34%;width:4px;height:4px;border-radius:50%;background:var(--c-secondary);animation:vd-particle-rise var(--c-anim-dur,2.4s) ease-in 1.05s infinite;"></span></div>`,
    cssVars: {
      '--c-bg-alt': COMMON_VARS['--c-bg-alt'],
      '--c-muted': COMMON_VARS['--c-muted'],
      '--c-radius': COMMON_VARS['--c-radius'],
      '--c-font': COMMON_VARS['--c-font'],
      '--c-primary': PALETTE_VARS['--c-primary'],
      '--c-secondary': PALETTE_VARS['--c-secondary'],
      '--c-accent': PALETTE_VARS['--c-accent'],
      '--c-anim-dur': { default: '2.4s', label: 'Animation Duration', type: 'string' as const },
    },
    editableProps: [
      { key: 'primary', label: 'Particle Color', type: 'color', cssVar: '--c-primary' },
      { key: 'secondary', label: 'Secondary Particle', type: 'color', cssVar: '--c-secondary' },
      { key: 'accent', label: 'Accent Particle', type: 'color', cssVar: '--c-accent' },
      animSpeedProp,
      radiusProp,
      opacityProp,
    ],
    defaultStyle: {},
  },

  'webgl-confetti': {
    id: 'webgl-confetti',
    name: 'Canvas Confetti',
    type: 'effect',
    category: 'effect',
    width: 400,
    height: 300,
    template: `
<canvas id="confetti-canvas" style="width:100%;height:100%;display:block;"></canvas>
<script>
  const canvas = document.getElementById('confetti-canvas');
  const ctx = canvas.getContext('2d');
  
  let particles = [];
  const colors = [
    '--c-primary', 
    '--c-secondary', 
    '--c-accent'
  ];

  // Sizing is now handled globally by the sandbox iframe parent wrapper

  function spawn() {
    particles.push({
      x: canvas.width / 2,
      y: canvas.height / 2,
      vx: (Math.random() - 0.5) * 10,
      vy: (Math.random() - 0.5) * 10 - 5,
      size: Math.random() * 5 + 2,
      color: colors[Math.floor(Math.random() * colors.length)],
      life: 1.0
    });
  }

  function resolveColor(cssVar) {
    return getComputedStyle(document.documentElement).getPropertyValue(cssVar).trim() || '#fff';
  }

  // Physics state
  let lastTime = performance.now();
  let accumulator = 0;
  const TIME_STEP = 1000 / 60; // Fixed 60 ticks per second

  function updatePhysics() {
    // Check if we should spawn based on live data from Properties
    const spawnRate = window.vortideckData?.spawnRate || 30;
    // Adjust spawn chance for the fixed tick
    if (Math.random() * 100 < spawnRate) {
      spawn();
      spawn();
    }
    
    for (let i = particles.length - 1; i >= 0; i--) {
      let p = particles[i];
      p.x += p.vx * 1.5; // INCREASED SPEED MULTIPLIER
      p.y += p.vy * 1.5; // INCREASED SPEED MULTIPLIER
      p.vy += 0.3; // Increased gravity to match
      p.life -= 0.015; // Fade out slightly faster to match the speed
      
      if (p.life <= 0) particles.splice(i, 1);
    }
  }

  function draw() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    
    for (let i = 0; i < particles.length; i++) {
      let p = particles[i];
      ctx.globalAlpha = Math.max(0, p.life);
      ctx.fillStyle = resolveColor(p.color);
      ctx.beginPath();
      ctx.arc(p.x, p.y, p.size, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function loop() {
    requestAnimationFrame(loop);
    
    let now = performance.now();
    let dt = now - lastTime;
    if (dt > 250) dt = 250; 
    lastTime = now;
    
    accumulator += dt;
    while (accumulator >= TIME_STEP) {
      updatePhysics();
      accumulator -= TIME_STEP;
    }
    
    draw();
  }
  loop();
</script>
    `,
    cssVars: {
      '--c-primary': PALETTE_VARS['--c-primary'],
      '--c-secondary': PALETTE_VARS['--c-secondary'],
      '--c-accent': PALETTE_VARS['--c-accent'],
    },
    editableProps: [
      { key: 'spawnRate', label: 'Spawn Rate', type: 'number', dataProp: 'spawnRate' },
      { key: 'primary', label: 'Color 1', type: 'color', cssVar: '--c-primary' },
      { key: 'secondary', label: 'Color 2', type: 'color', cssVar: '--c-secondary' },
      { key: 'accent', label: 'Color 3', type: 'color', cssVar: '--c-accent' },
    ],
    defaultData: {
      spawnRate: 30
    },
    defaultStyle: {},
  },

  'webgl-fireballs': {
    id: 'webgl-fireballs',
    name: 'Fireball Physics',
    type: 'effect',
    category: 'effect',
    width: 600,
    height: 400,
    template: `
<canvas id="fireball-canvas" style="width:100%;height:100%;display:block;"></canvas>
<script>
  const canvas = document.getElementById('fireball-canvas');
  const ctx = canvas.getContext('2d');
  
  let fireballs = [];
  let sparks = [];

  // Sizing is now handled globally by the sandbox iframe parent wrapper

  function spawnFireball() {
    const side = Math.floor(Math.random() * 4);
    let x, y, vx, vy;
    const speed = 4 + Math.random() * 4;
    
    // Spawn from outside the edges and fly inwards
    if (side === 0) { x = Math.random() * canvas.width; y = -50; vx = (Math.random()-0.5)*6; vy = speed; } 
    if (side === 1) { x = canvas.width + 50; y = Math.random() * canvas.height; vx = -speed; vy = (Math.random()-0.5)*6; } 
    if (side === 2) { x = Math.random() * canvas.width; y = canvas.height + 50; vx = (Math.random()-0.5)*6; vy = -speed; } 
    if (side === 3) { x = -50; y = Math.random() * canvas.height; vx = speed; vy = (Math.random()-0.5)*6; } 
    
    fireballs.push({ x, y, vx, vy, radius: 15 + Math.random() * 15, life: 1 });
  }

  function spawnExplosion(x, y, colorBase) {
    const numSparks = 40 + Math.random() * 30;
    for(let i=0; i<numSparks; i++) {
      const angle = Math.random() * Math.PI * 2;
      const speed = Math.random() * 12 + 2;
      sparks.push({
        x: x, y: y,
        vx: Math.cos(angle) * speed,
        vy: Math.sin(angle) * speed,
        radius: Math.random() * 6 + 3,
        life: 1.0,
        decay: 0.02 + Math.random() * 0.04,
        color: colorBase
      });
    }
  }

  function resolveColor(cssVar) {
    return getComputedStyle(document.documentElement).getPropertyValue(cssVar).trim() || '#ff5500';
  }

  // Physics state
  let lastTime = performance.now();
  let accumulator = 0;
  const TIME_STEP = 1000 / 60; // Fixed 60 ticks per second

  function updatePhysics() {
    const spawnRate = window.vortideckData?.spawnRate || 3;
    // Adjust spawn chance to be per-tick rather than per-frame
    if (Math.random() * 100 < spawnRate) spawnFireball();
    
    const primaryColor = resolveColor('--c-primary');
    const accentColor = resolveColor('--c-accent');

    // Update fireballs
    for (let i = fireballs.length - 1; i >= 0; i--) {
      let fb = fireballs[i];
      // SPEED BOOST: Multiplied by 2.0 to make it feel much faster on large displays
      fb.x += fb.vx * 2.0;
      fb.y += fb.vy * 2.0;
      
      // Fireball leaves a trail of sparks
      sparks.push({
        x: fb.x + (Math.random()-0.5)*fb.radius, 
        y: fb.y + (Math.random()-0.5)*fb.radius,
        vx: fb.vx * -0.3 * 2.0 + (Math.random()-0.5)*3,
        vy: fb.vy * -0.3 * 2.0 + (Math.random()-0.5)*3,
        radius: fb.radius * 0.7,
        life: 1.0,
        decay: 0.08 + Math.random() * 0.04, // Fade trails faster because they are moving faster
        color: primaryColor
      });

      // Collision detection with other fireballs
      for (let j = i - 1; j >= 0; j--) {
        let fb2 = fireballs[j];
        let dx = fb.x - fb2.x;
        let dy = fb.y - fb2.y;
        let dist = Math.sqrt(dx*dx + dy*dy);
        
        // If they overlap significantly, explode!
        if (dist < (fb.radius + fb2.radius) * 0.7) {
          spawnExplosion((fb.x + fb2.x)/2, (fb.y + fb2.y)/2, accentColor);
          fireballs.splice(i, 1);
          fireballs.splice(j, 1);
          i--; // Adjust index since we removed the current fireball
          break;
        }
      }
      
      // Cleanup fireballs that fly too far off screen
      if (fb && (fb.x < -150 || fb.x > canvas.width + 150 || fb.y < -150 || fb.y > canvas.height + 150)) {
        if(fireballs[i] === fb) fireballs.splice(i, 1);
      }
    }

    // Update sparks
    for (let i = sparks.length - 1; i >= 0; i--) {
      let s = sparks[i];
      s.x += s.vx;
      s.y += s.vy;
      s.vx *= 0.94; // air friction
      s.vy *= 0.94;
      s.life -= s.decay;
      
      if (s.life <= 0) {
        sparks.splice(i, 1);
      }
    }
  }

  function draw() {
    ctx.globalCompositeOperation = 'source-over';
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.globalCompositeOperation = 'lighter'; // Additive blending
    
    // Draw fireballs (they are just the core of the flame)
    // We don't draw the fireball bodies explicitly since the trails look better,
    // but if we wanted to, we could draw a white-hot core here.

    // Draw sparks/trails
    for (let i = sparks.length - 1; i >= 0; i--) {
      let s = sparks[i];
      const gradient = ctx.createRadialGradient(s.x, s.y, 0, s.x, s.y, s.radius);
      gradient.addColorStop(0, s.color);
      gradient.addColorStop(1, 'transparent');
      
      ctx.globalAlpha = s.life;
      ctx.fillStyle = gradient;
      ctx.beginPath();
      ctx.arc(s.x, s.y, s.radius, 0, Math.PI * 2);
      ctx.fill();
    }
  }

  function loop() {
    requestAnimationFrame(loop);
    
    let now = performance.now();
    let dt = now - lastTime;
    // Cap dt to prevent massive catch-up lag if tab is backgrounded
    if (dt > 250) dt = 250; 
    lastTime = now;
    
    accumulator += dt;
    // Run physics steps at fixed 60 FPS regardless of monitor refresh rate
    while (accumulator >= TIME_STEP) {
      updatePhysics();
      accumulator -= TIME_STEP;
    }
    
    draw();
  }
  loop();
</script>
    `,
    cssVars: {
      '--c-primary': PALETTE_VARS['--c-primary'],
      '--c-accent': PALETTE_VARS['--c-accent'],
    },
    editableProps: [
      { key: 'spawnRate', label: 'Spawn Rate', type: 'number', dataProp: 'spawnRate' },
      { key: 'primary', label: 'Fire Color', type: 'color', cssVar: '--c-primary' },
      { key: 'accent', label: 'Explosion Color', type: 'color', cssVar: '--c-accent' },
    ],
    defaultData: {
      spawnRate: 3
    },
    defaultStyle: {},
  },

  transition: {
    id: 'transition',
    name: 'Scene Transition',
    type: 'effect',
    category: 'effect',
    width: 240,
    height: 130,
    template: `<style>@keyframes vd-transition-wipe{0%,100%{transform:translateX(-101%)}40%,60%{transform:translateX(0)}}</style><div style="width:100%;height:100%;position:relative;overflow:hidden;background:transparent;border-radius:var(--c-radius);"><div style="position:absolute;inset:0;background:linear-gradient(135deg,var(--c-primary),var(--c-secondary));animation:vd-transition-wipe var(--c-anim-dur,3s) ease-in-out infinite;"></div></div>`,
    cssVars: {
      '--c-bg': COMMON_VARS['--c-bg'],
      '--c-text': COMMON_VARS['--c-text'],
      '--c-muted': COMMON_VARS['--c-muted'],
      '--c-radius': COMMON_VARS['--c-radius'],
      '--c-font': COMMON_VARS['--c-font'],
      '--c-primary': PALETTE_VARS['--c-primary'],
      '--c-secondary': PALETTE_VARS['--c-secondary'],
      '--c-anim-dur': { default: '3s', label: 'Transition Duration', type: 'string' as const },
    },
    editableProps: [
      { key: 'primary', label: 'Color Start', type: 'color', cssVar: '--c-primary' },
      { key: 'secondary', label: 'Color End', type: 'color', cssVar: '--c-secondary' },
      { key: 'animSpeed', label: 'Speed', type: 'select', cssVar: '--c-anim-dur', options: ['1s', '2s', '3s', '5s'], defaultValue: '3s' },
      radiusProp,
      opacityProp,
    ],
    defaultStyle: {},
  },

  glow: {
    id: 'glow',
    name: 'Glow Effect',
    type: 'effect',
    category: 'effect',
    width: 200,
    height: 200,
    template: `<style>@keyframes vd-glow-pulse{0%,100%{opacity:.3;transform:scale(.8)}50%{opacity:.9;transform:scale(1.08)}}</style><div style="width:100%;height:100%;position:relative;overflow:visible;display:flex;align-items:center;justify-content:center;border-radius:var(--c-radius);"><div style="position:absolute;inset:-20%;background:radial-gradient(circle,var(--c-primary) 0%,transparent 70%);animation:vd-glow-pulse var(--c-anim-dur,2s) ease-in-out infinite;border-radius:50%;pointer-events:none;"></div></div>`,
    cssVars: {
      '--c-primary': PALETTE_VARS['--c-primary'],
      '--c-muted': COMMON_VARS['--c-muted'],
      '--c-radius': COMMON_VARS['--c-radius'],
      '--c-font': COMMON_VARS['--c-font'],
      '--c-anim-dur': { default: '2s', label: 'Pulse Speed', type: 'string' as const },
    },
    editableProps: [
      { key: 'primary', label: 'Glow Color', type: 'color', cssVar: '--c-primary' },
      { key: 'animSpeed', label: 'Speed', type: 'select', cssVar: '--c-anim-dur', options: ['0.8s', '1.6s', '2s', '3.5s'], defaultValue: '2s' },
      radiusProp,
      opacityProp,
    ],
    defaultStyle: {},
  },
}

// ── Lookup helpers ──────────────────────────────────────────────

/** Get a component schema by ID. Returns undefined if not found. */
export const getComponentSchema = (id: string): ComponentSchema | undefined => {
  return componentRegistry[id]
}

const LEGACY_COMPONENT_VAR_MIGRATIONS: Record<string, Record<string, Record<string, string>>> = {
  'webgl-fireballs': {
    '--c-primary': { '#ff4400': 'var(--theme-primary)' },
    '--c-accent': { '#ffff00': 'var(--theme-accent)' },
  },
}

const normalizeLegacyComponentVar = (schemaId: string, property: string, value: string): string =>
  LEGACY_COMPONENT_VAR_MIGRATIONS[schemaId]?.[property]?.[value.toLowerCase()] || value

/**
 * Replace only known historical schema defaults. User-selected colors remain
 * untouched; this is intentionally an exact-value migration rather than a
 * blanket reset of instance customization.
 */
export function migrateLegacyComponentStyle(
  schemaId: string,
  elementStyle?: Record<string, string>,
): boolean {
  const migrations = LEGACY_COMPONENT_VAR_MIGRATIONS[schemaId]
  if (!migrations || !elementStyle) return false

  let changed = false
  for (const [property, replacements] of Object.entries(migrations)) {
    const current = elementStyle[property]
    const replacement = current ? replacements[current.toLowerCase()] : undefined
    if (replacement && replacement !== current) {
      elementStyle[property] = replacement
      changed = true
    }
  }
  return changed
}

/**
 * Build the --c-* CSS variable map for a component instance.
 * Starts from schema defaults, overrides with element-level values.
 */
export function buildComponentVars(
  schema: ComponentSchema,
  elementStyle?: Record<string, any>,
): Record<string, string> {
  // Start with schema defaults
  const vars: Record<string, string> = {}
  for (const [varName, def] of Object.entries(schema.cssVars)) {
    vars[varName] = def.default
  }

  if (!elementStyle) return vars

  // Override from direct --c-* values set on the element
  for (const [key, val] of Object.entries(elementStyle)) {
    if (key.startsWith('--c-') && val && schema.cssVars[key]) {
      vars[key] = normalizeLegacyComponentVar(schema.id, key, String(val))
    }
  }

  // Override from editableProps that map styleProp → cssVar
  for (const prop of schema.editableProps) {
    if (prop.cssVar && prop.styleProp) {
      // Not applicable here — cssVar and styleProp are separate paths
      continue
    }
    // If a cssVar prop has a concrete (non-var) value set via the Properties panel,
    // pick it up from the element style key that matches the var name
    if (prop.cssVar) {
      const val = elementStyle[prop.key]
      if (val && typeof val === 'string' && !val.startsWith('var(')) {
        vars[prop.cssVar] = val
      }
    }
  }

  return vars
}

/**
 * Backward-compat bridge: convert a ComponentSchema to the old ElementDefaults format.
 * Used during migration so existing code (addElement, etc.) still works.
 */
export function schemaToElementDefaults(schema: ComponentSchema): {
  type: string
  name: string
  width: number
  height: number
  content?: string
  customHTML?: string
  style: Record<string, string>
  schemaId: string
  defaultData?: Record<string, string | number>
} {
  // Build initial --c-* vars from schema defaults
  const style: Record<string, string> = { ...schema.defaultStyle }
  for (const [varName, def] of Object.entries(schema.cssVars)) {
    style[varName] = def.default
  }

  return {
    type: schema.type,
    name: schema.name,
    width: schema.width,
    height: schema.height,
    content: schema.content,
    customHTML: schema.template || undefined,
    style,
    schemaId: schema.id,
    // Seed element.data with schema defaults so {data.*} placeholders have values immediately
    defaultData: schema.defaultData ? { ...schema.defaultData } : undefined,
  }
}

/** Get element defaults for a component (backward compat with old addElement). */
export const getSchemaElementDefaults = (typeId: string) => {
  const schema = componentRegistry[typeId]
  if (!schema) return undefined
  return schemaToElementDefaults(schema)
}

export const getElementDefaults = getSchemaElementDefaults

export const activeDraggingElement = ref<any | null>(null);
