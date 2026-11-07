async function test() {
  const list = await (await fetch('http://127.0.0.1:9222/json/list')).json();
  const target = list.find(t => t.url.includes('/deck')) || list[0];
  const ws = new WebSocket(target.webSocketDebuggerUrl);
  ws.onopen = () => {
    ws.send(JSON.stringify({
      id: 1,
      method: 'Runtime.evaluate',
      params: {
        expression: `(async () => {
          try {
            const invoke = window.__TAURI_INTERNALS__?.invoke || window.__TAURI__?.core?.invoke;
            const caps = await invoke('get_integration_capabilities', { integrationId: 'obs' });
            return { ok: true, caps: caps.map(c => ({ id: c.capabilityId, name: c.presentation.name, inputs: c.inputs })) };
          } catch (e) {
            return { ok: false, error: String(e) };
          }
        })()`,
        awaitPromise: true,
        returnByValue: true
      }
    }));
  };
  ws.onmessage = (msg) => {
    const data = JSON.parse(msg.data);
    console.log(JSON.stringify(data.result?.result?.value, null, 2));
    process.exit(0);
  };
}
test();
