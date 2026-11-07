import { invoke } from "@tauri-apps/api/core";
import { useConnectionStore } from "@/stores/Connection";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

// import {default as TauriWebSocket}  from '@tauri-apps/plugin-websocket';





type Device = {
  client_id: string;
  device_type: string;
  device: string;
};

export default defineNuxtPlugin(() => {

  const currentWindowLabel = getCurrentWindow().label;
  if(currentWindowLabel !== "main" && currentWindowLabel !== "mobile_simulator") return
  const connectionStore = useConnectionStore();
  const config = useRuntimeConfig();
  const socket = ref<WebSocket | null>(null); // Reactive WebSocket instance
  const isWebSocketConnected = ref(false); // Reactive connection status
  const router = useRouter();
  const paired: Ref<Device[] | null> = ref([]);
  // const lastPongReceived = ref<number>(Date.now()); // Track last pong timestamp

  listen<Device>("new_mobile_peer_added", async (event) => {
    console.log(`peer ${event.payload.device_type} added`);
    console.log(event.payload);
    connectionStore.setPeers(event.payload);
    connectionStore.setStatus("paired");
    router.push("/");
  });

  // Define the return type of discoverService
  interface ServiceInfo {
    name: string;
    websocket_url: string;
  }

  async function discoverService(): Promise<ServiceInfo[] | null> {
    // const ws = await TauriWebSocket.connect('ws://host.vortideck.internal:9002');


    // ws.addListener((msg) => {
    //   console.log('Received Message:', msg);
    // });

    // await ws.send('Hello World!');


    try {
      // Safely invoke the Tauri command
      const result = await invoke<ServiceInfo[]>("discover_websocket");
      console.log("Discovered services:", result); // Log the result
      if (result && result.length > 0) {
        // alert(`Found ${result.length} services: ${JSON.stringify(result)}`);
      } else {
        // alert("No services found");
      }
      return result; // Return the list of services
    } catch (error) {
      console.error("Error discovering services:", error);
      // alert("Error discovering services");
      return null; // Return null in case of error
    }
  }

  let reconnectInterval: NodeJS.Timeout | null = null;
  // let pingInterval: NodeJS.Timeout | null = null;
  // let pongCheckInterval: NodeJS.Timeout | null = null;

  // // Function to send ping to server
  // const sendPing = () => {
  //   if (socket.value && socket.value.readyState === WebSocket.OPEN) {
  //     const pingMessage = {
  //       action: "ping",
  //     };
  //     socket.value.send(JSON.stringify(pingMessage));
  //     console.log("Ping sent to server");
  //   }
  // };

  // // Function to check if pong was received within timeout
  // const checkPongReceived = () => {
  //   const now = Date.now();
  //   const timeSinceLastPong = now - lastPongReceived.value;
    
  //   if (timeSinceLastPong > 10000) { // 10 seconds timeout
  //     console.log("No pong received in 10 seconds, reconnecting...");
      
  //     // Close the socket if it's still open
  //     if (socket.value && socket.value.readyState === WebSocket.OPEN) {
  //       socket.value.close();
  //     }
      
  //     // Reset connection status
  //     isWebSocketConnected.value = false;
  //     connectionStore.setStatus("disconnected");
      
  //     // Clear intervals
  //     clearIntervals();
      
  //     // Attempt to reconnect
  //     attemptReconnect();
  //   }
  // };

  // Function to clear all intervals
  const clearIntervals = () => {
    // if (pingInterval !== null) {
    //   clearInterval(pingInterval);
    //   pingInterval = null;
    // }
    
    // if (pongCheckInterval !== null) {
    //   clearInterval(pongCheckInterval);
    //   pongCheckInterval = null;
    // }
  };

  // Function to establish WebSocket connection
  const connectWebSocket = async () => {
    // alert("connecting");
    // Prevent redundant connections
    if (socket.value && (
      socket.value.readyState === WebSocket.OPEN
      || socket.value.readyState === WebSocket.CONNECTING
    )) {
      console.log("WebSocket is already connected or connecting.");
      if (reconnectInterval !== null) {
        clearInterval(reconnectInterval);
        reconnectInterval = null;
      }
      return;
    }

    // Fetch all the WebSocket URLs from the discovered services
    const serviceInfos = await discoverService();
    if (!serviceInfos || serviceInfos.length === 0) {
      console.error("No WebSocket services found.");

      attemptReconnect();
      return;
    }

    // For now, connect to the first service (you can modify this to connect to any)
    const serviceInfo = serviceInfos[0];
    if (!serviceInfo.websocket_url) {
      console.error("Invalid WebSocket URL.");
      return;
    }

    // Create a new WebSocket instance
    socket.value = new WebSocket(serviceInfo.websocket_url);
    //socket.value = new WebSocket("ws://host.vortideck.internal:9004/");
 
    // Attach event listeners to the WebSocket
    socket.value.onopen = async () => {
  
      console.log("WebSocket connection established");
      isWebSocketConnected.value = true;
      connectionStore.setStatus("connected");
      // Reset pong timestamp on new connection
      // lastPongReceived.value = Date.now();
      
      // alert("opened")

      // alert(token.value)
 
      // Send initial device information
      socket.value?.send(
        JSON.stringify({
          clientId: `desktop-host:${currentWindowLabel}`,
          device: {
            type: "host",
            model: currentWindowLabel,
          },
        })
      );

      // Start sending ping messages every 5 seconds
      clearIntervals(); // Clear any existing intervals
      
      // pingInterval = setInterval(sendPing, 5000);
      // pongCheckInterval = setInterval(checkPongReceived, 2000); // Check for pong every 2 seconds

      // alert("send auth message")

      // Stop reconnection attempts on successful connection
      if (reconnectInterval !== null) {
        clearInterval(reconnectInterval);
        reconnectInterval = null;
      }
    };

    socket.value.onerror = (error) => {
      
      console.error("WebSocket error:", error);
      isWebSocketConnected.value = false;
      connectionStore.setStatus("disconnected");
      
      // Clean up intervals
      clearIntervals();
    
      attemptReconnect();
    };

    socket.value.onclose = (event) => {
      console.log("WebSocket connection closed:", event);
      isWebSocketConnected.value = false;
      connectionStore.setStatus("disconnected");
      
      // Clear intervals
      clearIntervals();
      
      // alert("closed");
      if (!event.wasClean) {
        attemptReconnect();
      }
    };

    socket.value.onmessage = (event) => {
      const message = JSON.parse(event.data);
      console.log("Received message:", message);

      // // Handle pong response from server
      // if (message.action === "pong") {
      //   console.log(`Received pong from server.`);
      //   lastPongReceived.value = Date.now(); // Update last pong timestamp
      // }
    };
  };

  // Function to handle reconnection logic
  const attemptReconnect = () => {
    connectionStore.setStatus("connecting");
    if (reconnectInterval === null) {
      reconnectInterval = setInterval(() => {
        console.log("Attempting to reconnect...");
        connectWebSocket();
      }, 5000); // Attempt reconnection every 5 seconds
    }
  };

  // Initially connect WebSocket
  connectWebSocket();

  return {
    provide: {
      socket,
      isWebSocketConnected,
      paired,
    },
  };
});
