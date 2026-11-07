#!/usr/bin/env node

/**
 * Example Service Plugin for VortiDeck
 *
 * This demonstrates how to create a service plugin that:
 * - Communicates via JSON-RPC over stdin/stdout
 * - Registers flow nodes
 * - Calls VortiDeck Host APIs
 * - Handles node execution requests
 */

const readline = require('readline');

// JSON-RPC request ID counter
let requestId = 0;

// Pending requests waiting for responses
const pendingRequests = new Map();

// Setup stdin/stdout communication
const rl = readline.createInterface({
  input: process.stdin,
  terminal: false
});

// Listen for messages from VortiDeck
rl.on('line', (line) => {
  try {
    const message = JSON.parse(line);

    if (message.result !== undefined || message.error !== undefined) {
      // This is a response to our request
      handleResponse(message);
    } else if (message.method) {
      // This is a request from VortiDeck
      handleRequest(message);
    }
  } catch (error) {
    console.error('Failed to parse message:', error.message);
  }
});

// Handle responses from VortiDeck
function handleResponse(response) {
  const { id, result, error } = response;

  const pending = pendingRequests.get(id);
  if (pending) {
    pendingRequests.delete(id);

    if (error) {
      pending.reject(new Error(error.message));
    } else {
      pending.resolve(result);
    }
  }
}

// Handle requests from VortiDeck (node execution)
async function handleRequest(request) {
  const { id, method, params } = request;

  try {
    let result;

    switch (method) {
      case 'execute':
        result = await executeNode(params);
        break;

      default:
        throw new Error(`Unknown method: ${method}`);
    }

    sendResponse(id, result);
  } catch (error) {
    sendError(id, -32000, error.message);
  }
}

// Execute a node
async function executeNode(params) {
  const { nodeId, inputs } = params;

  log('info', `Executing node: ${nodeId}`);
  log('debug', `Inputs: ${JSON.stringify(inputs)}`);

  // Example: Fetch data from an API
  if (nodeId === 'example-service-plugin:fetch-data') {
    const url = inputs.url || 'https://api.github.com/zen';

    // Call VortiDeck's HTTP API
    const response = await callHost('vortideck.http.request', {
      url,
      method: 'GET',
      headers: {
        'User-Agent': 'VortiDeck-Plugin/1.0'
      }
    });

    return {
      success: true,
      data: response.body,
      status: response.status
    };
  }

  // Example: Save data to file
  if (nodeId === 'example-service-plugin:save-file') {
    const path = inputs.path;
    const content = inputs.content;

    await callHost('vortideck.file.write', {
      path,
      content
    });

    return {
      success: true,
      path
    };
  }

  throw new Error(`Unknown node: ${nodeId}`);
}

// Call a VortiDeck Host API
function callHost(method, params) {
  return new Promise((resolve, reject) => {
    const id = ++requestId;

    pendingRequests.set(id, { resolve, reject });

    sendRequest(id, method, params);

    // Timeout after 30 seconds
    setTimeout(() => {
      if (pendingRequests.has(id)) {
        pendingRequests.delete(id);
        reject(new Error('Request timeout'));
      }
    }, 30000);
  });
}

// Send a JSON-RPC request
function sendRequest(id, method, params) {
  const request = {
    jsonrpc: '2.0',
    id,
    method,
    params
  };

  process.stdout.write(JSON.stringify(request) + '\n');
}

// Send a JSON-RPC response
function sendResponse(id, result) {
  const response = {
    jsonrpc: '2.0',
    id,
    result
  };

  process.stdout.write(JSON.stringify(response) + '\n');
}

// Send a JSON-RPC error
function sendError(id, code, message) {
  const response = {
    jsonrpc: '2.0',
    id,
    error: {
      code,
      message
    }
  };

  process.stdout.write(JSON.stringify(response) + '\n');
}

// Send a JSON-RPC notification (no response expected)
function sendNotification(method, params) {
  const notification = {
    jsonrpc: '2.0',
    method,
    params
  };

  process.stdout.write(JSON.stringify(notification) + '\n');
}

// Log a message via VortiDeck (fire-and-forget, no response expected)
function log(level, message) {
  // Send as notification (no ID = no response expected)
  sendNotification('vortideck.log', { level, message });
}

// Register our nodes with VortiDeck
function registerNodes() {
  const nodes = [
    {
      id: 'example-service-plugin:fetch-data',
      name: 'Fetch Data',
      version: '1.0.0',
      description: 'Fetch data from a URL using VortiDeck HTTP API',
      author: 'VortiDeck Team',
      category: 'DATA',
      type_color: '#4CAF50',
      parameters_schema: {
        type: 'object',
        properties: {
          url: {
            type: 'string',
            description: 'URL to fetch data from',
            default: 'https://api.github.com/zen'
          }
        }
      },
      output_schema: {
        type: 'object',
        properties: {
          success: {
            type: 'boolean',
            description: 'Whether the fetch was successful'
          },
          data: {
            type: 'string',
            description: 'The fetched data'
          },
          status: {
            type: 'number',
            description: 'HTTP status code'
          }
        }
      }
    },
    {
      id: 'example-service-plugin:save-file',
      name: 'Save File',
      version: '1.0.0',
      description: 'Save content to a file using VortiDeck File API',
      author: 'VortiDeck Team',
      category: 'DATA',
      type_color: '#2196F3',
      parameters_schema: {
        type: 'object',
        properties: {
          path: {
            type: 'string',
            description: 'File path to save to'
          },
          content: {
            type: 'string',
            description: 'Content to write to the file'
          }
        },
        required: ['path', 'content']
      },
      output_schema: {
        type: 'object',
        properties: {
          success: {
            type: 'boolean',
            description: 'Whether the file was saved successfully'
          },
          path: {
            type: 'string',
            description: 'Path where the file was saved'
          }
        }
      }
    }
  ];

  sendNotification('vortideck.registerNodes', nodes);
  console.error(`Registered ${nodes.length} nodes`);
}

// Initialize plugin
async function init() {
  console.error('Example Service Plugin starting...');

  // Register our flow nodes
  registerNodes();

  console.error('Example Service Plugin ready!');
}

// Start the plugin
init().catch((error) => {
  console.error('Failed to initialize plugin:', error);
  process.exit(1);
});
