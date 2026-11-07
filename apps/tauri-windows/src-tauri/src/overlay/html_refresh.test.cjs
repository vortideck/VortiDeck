const test = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const { JSDOM } = require('jsdom');

// Exercise the actual embedded browser patch, undoing Rust format! brace escapes.
const source = fs.readFileSync(require.resolve('./html.rs'), 'utf8');
const start = source.indexOf('        function performOverlayFetch()');
const end = source.indexOf('        function iframeAllowsStreamEvent', start);
assert(start !== -1 && end > start);
const patch = source.slice(start, end).replaceAll('{{', '{').replaceAll('}}', '}');

test('refresh narrows native authority before delivery without restarting the widget', async () => {
    const dom = new JSDOM(`<div class="overlay-container"><div id="placed-1"
        class="overlay-element" style="left:320px;top:220px;width:480px;height:160px"
        data-authorized-package-id="alerts" data-authorized-subscribe="follow,donation"
        data-authorized-actions="triggerFlow:old" data-widget-config-revision="1"
        data-widget-config='{"goal":500}'><iframe srcdoc="unchanged widget"></iframe></div></div>`,
        { runScripts: 'outside-only', url: 'http://localhost/' });
    try {
        const { window } = dom;
        const element = window.document.getElementById('placed-1');
        const iframe = element.querySelector('iframe');
        const incoming = element.cloneNode(true);
        incoming.setAttribute('data-authorized-subscribe', 'follow');
        incoming.setAttribute('data-authorized-actions', '');
        incoming.setAttribute('data-widget-config-revision', '2');
        incoming.setAttribute('data-widget-config', '{"goal":750}');
        window.fetch = async () => ({ text: async () => `<div class="overlay-container">${incoming.outerHTML}</div>` });
        const deliveries = [];
        let readyCalls = 0;
        window.notifyOverlayReady = () => { readyCalls += 1; };
        window.deliverWidgetConfig = current => {
            deliveries.push({
                subscribe: current.getAttribute('data-authorized-subscribe'),
                actions: current.getAttribute('data-authorized-actions'),
                config: current.getAttribute('data-widget-config'),
            });
        };
        window.eval(patch);
        window.performOverlayFetch();
        await new Promise(resolve => setImmediate(resolve));
        assert.deepEqual(deliveries, [{ subscribe: 'follow', actions: '', config: '{"goal":750}' }]);
        assert.equal(element.querySelector('iframe'), iframe);
        assert.equal(iframe.getAttribute('srcdoc'), 'unchanged widget');
        assert.equal(element.style.left, '320px');
        assert.equal(element.style.top, '220px');
        assert.equal(readyCalls, 1);
    } finally {
        dom.window.close();
    }
});
