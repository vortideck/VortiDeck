/**
 * Text Processor Suite - Web Extension
 *
 * Demonstrates VortiDeck plugin SDK web extension with 7 nodes
 */

console.log('[Text Processor Suite] Loading web extension...');

// Wait for VortiDeck API to be ready
window.addEventListener('vortideck:ready', async (event) => {
  console.log('[Text Processor Suite] API ready:', event.detail);
  initPlugin();
});

// Fallback if API is already available
if (window.vortideck?.api) {
  console.log('[Text Processor Suite] API already available');
  initPlugin();
}

function initPlugin() {
  const app = document.getElementById('app');
  if (!app) {
    console.error('[Text Processor Suite] #app element not found');
    return;
  }

  // Create UI
  app.innerHTML = `
    <div class="text-processor-container">
      <header class="plugin-header">
        <h1>📝 Text Processor Suite</h1>
        <p>7 powerful text transformation nodes</p>
      </header>

      <section class="input-section">
        <label for="input-text">Input Text</label>
        <textarea id="input-text" placeholder="Enter text..." rows="4"></textarea>
      </section>

      <section class="operations-section">
        <h3>Operations (7 Nodes)</h3>
        <div class="button-grid">
          <button class="op-btn" data-node="uppercase">
            <span class="op-icon">Aa</span>
            UPPERCASE
          </button>
          <button class="op-btn" data-node="lowercase">
            <span class="op-icon">aa</span>
            lowercase
          </button>
          <button class="op-btn" data-node="reverse">
            <span class="op-icon">↔</span>
            Reverse
          </button>
          <button class="op-btn" data-node="titlecase">
            <span class="op-icon">Tt</span>
            Title Case
          </button>
          <button class="op-btn" data-node="capitalize">
            <span class="op-icon">A</span>
            Capitalize
          </button>
          <button class="op-btn" data-node="trim">
            <span class="op-icon">✂</span>
            Trim
          </button>
          <button class="op-btn" data-node="wordcount">
            <span class="op-icon">#</span>
            Word Count
          </button>
        </div>
      </section>

      <section class="output-section">
        <div class="output-header">
          <label>Output</label>
          <span id="operation-badge" class="operation-badge hidden"></span>
        </div>
        <textarea id="output-text" placeholder="Result..." rows="4" readonly></textarea>
        <button id="copy-btn" class="copy-btn" disabled>Copy to Clipboard</button>
      </secti      <section class="history-section">
        <h3>History</h3>
        <div id="history-list" class="history-list">
          <p class="history-empty">No operations yet</p>
        </div>
      </section>

      <section class="event-monitor-section" style="padding: 0 1.5rem; margin-bottom: 1.5rem;">
        <h3 style="font-size: 1rem; font-weight: 600; color: #374151; margin: 0 0 0.75rem 0;">Live Event Monitor</h3>
        <div id="event-monitor-log" class="event-monitor-log" style="background: #18181b; color: #10b981; font-family: monospace; padding: 12px; border-radius: 6px; height: 120px; overflow-y: auto; font-size: 11px; border: 1px solid #27272a; line-height: 1.4;">
          <div style="color: #71717a;">Listening for events (stream_event, generic_event)...</div>
        </div>
      </section>
 
      <footer class="plugin-footer">
        <span class="plugin-id">Plugin: example.text-processor</span>
        <span id="status" class="status-indicator ready">Ready</span>
      </footer>
    </div>
  `;

  // Setup event listeners
  const inputEl = document.getElementById('input-text');
  const outputEl = document.getElementById('output-text');
  const copyBtn = document.getElementById('copy-btn');
  const statusEl = document.getElementById('status');
  const badge = document.getElementById('operation-badge');
  const historyList = document.getElementById('history-list');

  let history = [];

  // Operation buttons
  document.querySelectorAll('.op-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const nodeKey = btn.getAttribute('data-node');
      await executeNode(nodeKey);
    });
  });

  // Copy button
  copyBtn.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(outputEl.value);
      copyBtn.textContent = 'Copied!';
      copyBtn.classList.add('copied');
      setTimeout(() => {
        copyBtn.textContent = 'Copy to Clipboard';
        copyBtn.classList.remove('copied');
      }, 2000);
    } catch (err) {
      console.error('Copy failed:', err);
    }
  });

  async function executeNode(nodeKey) {
    const inputText = inputEl.value;
    if (!inputText.trim()) {
      showError('Please enter some text');
      return;
    }

    setStatus('Processing...', 'processing');
    badge.textContent = nodeKey.toUpperCase();
    badge.classList.remove('hidden');

    try {
      console.log(`[Text Processor Suite] Executing node: ${nodeKey}`);

      // Call the WASM function through the plugin API
      // The SDK's createPlugin exports an 'execute' function that routes to the correct node
      const result = await window.vortideck.api.callPlugin('execute', {
        inputs: { input: inputText },  // Main input (use "input" key for FlowParams.getInput())
        params: { node: nodeKey }    // Tell it which node to run
      });

      console.log('[Text Processor Suite] Result:', result);

      // Update output
      if (result.success) {
        outputEl.value = result.output || JSON.stringify(result, null, 2);
        copyBtn.disabled = false;

        // Add to history
        addToHistory(nodeKey, inputText, outputEl.value);
        setStatus('Ready', 'ready');
      } else {
        showError(result.error || 'Operation failed');
        setStatus('Error', 'error');
      }

    } catch (err) {
      console.error('[Text Processor Suite] Error:', err);
      showError(err.message || String(err));
      setStatus('Error', 'error');
    }
  }

  function showError(message) {
    outputEl.value = `Error: ${message}`;
    outputEl.classList.add('error');
    setTimeout(() => outputEl.classList.remove('error'), 3000);
  }

  function setStatus(text, state) {
    statusEl.textContent = text;
    statusEl.className = `status-indicator ${state}`;
  }

  function addToHistory(operation, input, output) {
    history.unshift({
      operation,
      input: input.substring(0, 50) + (input.length > 50 ? '...' : ''),
      output: output.substring(0, 50) + (output.length > 50 ? '...' : ''),
      timestamp: new Date().toLocaleTimeString()
    });

    if (history.length > 10) history.pop();

    updateHistoryUI();
  }

  function updateHistoryUI() {
    if (history.length === 0) {
      historyList.innerHTML = '<p class="history-empty">No operations yet</p>';
      return;
    }

    historyList.innerHTML = history.map(item => `
      <div class="history-item">
        <div class="history-header">
          <span class="history-op">${item.operation}</span>
          <span class="history-time">${item.timestamp}</span>
        </div>
        <div class="history-content">
          <span class="history-input">"${item.input}"</span>
          <span class="history-arrow">→</span>
          <span class="history-output">"${item.output}"</span>
        </div>
      </div>
    `).join('');
  }

  // Live Event Monitor subscription
  if (window.vortideck?.api?.on) {
    const logEl = document.getElementById('event-monitor-log');
    const logEvent = (type, data) => {
      if (logEl) {
        if (logEl.innerHTML.includes('Listening for events')) {
          logEl.innerHTML = '';
        }
        const time = new Date().toLocaleTimeString();
        const newLog = document.createElement('div');
        newLog.style.marginBottom = '4px';
        newLog.innerHTML = `[${time}] <span style="color: #3b82f6; font-weight: bold;">[${type}]</span> ${JSON.stringify(data)}`;
        logEl.appendChild(newLog);
        logEl.scrollTop = logEl.scrollHeight;
      }
    };

    window.vortideck.api.on('stream_event', (eventData) => {
      console.log('[Text Processor Suite] Received stream event:', eventData);
      logEvent('STREAM', eventData);
    });

    window.vortideck.api.on('generic_event', (eventData) => {
      console.log('[Text Processor Suite] Received generic event:', eventData);
      logEvent('GENERIC', eventData);
    });
  }

  console.log('[Text Processor Suite] UI initialized');
}
