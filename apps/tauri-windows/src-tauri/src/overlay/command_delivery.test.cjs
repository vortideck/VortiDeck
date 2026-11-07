const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { JSDOM } = require('jsdom');
const protocol = require('./browser_protocol.cjs');
const source = fs.readFileSync(require.resolve('./html.rs'), 'utf8');
const start = source.indexOf('        function handleMessage(message)');
const end = source.indexOf('        function updateElementPosition', start);
assert(start !== -1 && end > start);
const handler = source.slice(start, end).replaceAll('{{', '{').replaceAll('}}', '}');

test('source readiness requests trailing replay after a quick refresh without sending before load', () => {
    const dom = new JSDOM('<iframe></iframe>', { runScripts: 'outside-only' });
    const window = dom.window;
    const begin = source.indexOf('        let overlayReadyTimer = null;');
    const finish = source.indexOf('        // Install during parsing', begin);
    assert(begin !== -1 && finish > begin);
    const timers = new Map();
    let sequence = 0;
    window.setTimeout = (callback, delay) => { timers.set(++sequence, { callback, delay }); return sequence; };
    window.clearTimeout = id => timers.delete(id);
    const sent = [];
    window.ws = { readyState: window.WebSocket.OPEN, send: message => sent.push(JSON.parse(message)) };
    window.readyWidgetFrames = new WeakSet();
    try {
        window.eval(source.slice(begin, finish).replaceAll('{{', '{').replaceAll('}}', '}'));
        window.notifyOverlayReady();
        window.notifyOverlayReady();
        assert.equal(timers.size, 1, 'refresh cancels the earlier replay timer');
        const pending = [...timers.values()][0];
        assert(pending.delay > 250, 'legitimate readiness must exceed the native throttle');
        pending.callback();
        assert.equal(sent.length, 0);
        window.readyWidgetFrames.add(window.document.querySelector('iframe'));
        window.notifyOverlayReady();
        [...timers.values()][0].callback();
        assert.deepEqual(sent, [{ type: 'OVERLAY_READY', version: 1 }]);
    } finally { window.close(); }
});

function fixture() {
    const dom = new JSDOM('<div class="overlay-element" id="placed-one"><iframe data-widget-package-id="alerts" data-widget-instance-id="placed-one"></iframe></div>', { runScripts: 'outside-only', url: 'http://localhost/' });
    const window = dom.window;
    window.VortiDeckOverlayProtocol = protocol;
    window.readyWidgetFrames = new WeakSet();
    window.appliedOverlayCommands = new Map();
    window.pendingVisualAcknowledgements = new Map();
    window.streamEventBuffer = [];
    window.STREAM_BUFFER_MAX = 20;
    window.iframeMatchesOverlayTarget = (frame, target) => protocol.frameMatchesTarget({ elementInstanceId: frame.closest('.overlay-element').id, packageId: 'alerts' }, target);
    window.authoritativeWidgetPackageId = () => 'alerts';
    window.iframeAllowsStreamEvent = () => true;
    window.rememberAppliedOverlayCommand = (id, revision) => protocol.rememberApplied(window.appliedOverlayCommands, id, revision);
    const acks = [];
    window.sendOverlayAcknowledgement = (...args) => acks.push(args);
    let hostPlayback = 0;
    window.handleBuiltinOverlayPlayback = () => { hostPlayback += 1; };
    window.eval(handler);
    const frame = window.document.querySelector('iframe');
    const deliveries = [];
    frame.contentWindow.postMessage = message => deliveries.push(message);
    return { dom, window, frame, deliveries, acks, hostPlayback: () => hostPlayback };
}

test('a pending command is neither ACKed nor applied before its target is ready; outbox replay applies it once', () => {
    const f = fixture();
    const message = { category: 'overlay', eligibleWidgetIds: ['placed-one'], payload: {
        commandId: 'command-one', deliveryRevision: 1, target: { type: 'widget', widgetId: 'placed-one' },
        command: 'overlay.alert.play', payload: {}, requiresAck: true, acknowledgementMode: 'hostAccepted',
    } };
    try {
        f.window.handleMessage(message);
        assert.equal(f.acks.length, 0); assert.equal(f.deliveries.length, 0); assert.equal(f.window.appliedOverlayCommands.size, 0);
        f.window.readyWidgetFrames.add(f.frame);
        f.window.handleMessage({ ...message, replay: true });
        assert.equal(f.deliveries.length, 1); assert.equal(f.acks[0][1], 'accepted');
        assert.equal(f.hostPlayback(), 0, 'widget targeting cannot invoke global host playback');
        f.window.handleMessage(message);
        assert.equal(f.deliveries.length, 1);
    } finally { f.dom.window.close(); }
});

test('only chat is buffered; source reload cannot replay follow/alert effects', () => {
    const f = fixture();
    try {
        for (const type of ['follow', 'subscription', 'bits', 'chatMessage']) {
            f.window.handleMessage({ category: 'stream', payload: { type } });
        }
        assert.equal(f.window.streamEventBuffer.length, 1);
        assert.equal(f.window.streamEventBuffer[0].payload.type, 'chatMessage');
    } finally { f.dom.window.close(); }
});
