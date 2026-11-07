import { useConnectionStore } from "@base/stores/Connection";

export const useSendCommand = async (command?: String, value?: number, buttonId?: number) => {
  const { $sendSignedMessage, $socket } = useNuxtApp();
  const connectionStore = useConnectionStore();

  console.log("🎮 [useSendCommand] Sending command:", command, "value:", value, "buttonId:", buttonId);
  console.log("🎮 [useSendCommand] Connection mode:", connectionStore.getConnectionMode);
  console.log("🎮 [useSendCommand] Connection status:", connectionStore.getStatus);

  const commandMessage = {
    action: "command",
    command: command,
    value: value,
    buttonId: buttonId,
  };

  console.log("🎮 [useSendCommand] Command message:", JSON.stringify(commandMessage));

  // Check connection mode: mobile uses signed messages, desktop simulator uses direct socket
  const isMobile = connectionStore.getConnectionMode === 'relay' || $sendSignedMessage;

  if (isMobile && $sendSignedMessage) {
    // Mobile: Use signed messages for E2EE
    $sendSignedMessage(commandMessage);
    console.log("✅ [useSendCommand] Message sent via $sendSignedMessage (mobile E2EE)");
  } else if ($socket?.value) {
    // Desktop simulator: Use direct socket (no signing needed)
    $socket.value.send(JSON.stringify(commandMessage));
    console.log("✅ [useSendCommand] Message sent via direct socket (desktop simulator)");
  } else {
    console.error("❌ [useSendCommand] No connection method available!");
  }
};
