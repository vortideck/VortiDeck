import { computed, type Ref } from 'vue';
import type { ActiveAppContext } from './useAppAutoContext';

export interface ContextSuggestionChip {
  icon: string;
  label: string;
  prompt: string;
}

/**
 * Context-Aware Suggestion Chips Resolver
 * Encapsulates client-side prompt heuristics with extensible plugin/category support.
 */
export const useContextSuggestions = (activeContext: Ref<ActiveAppContext | null>) => {
  const contextSuggestions = computed<ContextSuggestionChip[]>(() => {
    if (!activeContext.value) {
      return [
        { icon: "🎬", label: "Switch scene in OBS", prompt: "Switch my OBS scene to Gaming" },
        { icon: "⚡", label: "Run BRB macro", prompt: "List my available macros and run BRB mode" },
        { icon: "🩺", label: "Debug recent flow", prompt: "Why did my last stream workflow fail? Inspect execution history and logs." },
        { icon: "📋", label: "List active flows", prompt: "List all my active automation flows" },
      ];
    }

    // 1. Flow Node Context
    if (activeContext.value.type === "flow_node") {
      const node = activeContext.value.data || {};
      const nodeType = (node.type || node.nodeType || activeContext.value.subtitle || "").toLowerCase();
      const nodeLabel = node.label || node.name || activeContext.value.title || "this node";

      if (nodeType.includes("obs") || nodeType.includes("scene") || nodeType.includes("stream") || nodeType.includes("source")) {
        return [
          { icon: "🎬", label: "Switch to Gaming Scene", prompt: `Configure ${nodeLabel} to switch OBS scene to 'Gaming' with a Smooth Cut transition.` },
          { icon: "🎙️", label: "Mute Desktop Audio", prompt: `Configure ${nodeLabel} to mute Desktop Audio source in OBS.` },
          { icon: "🎥", label: "Add Transition Delay", prompt: `Add a 2-second delay node right before ${nodeLabel} to allow scene transitions to buffer.` },
          { icon: "🩺", label: "Test OBS Connection", prompt: `Test if ${nodeLabel} can connect to OBS WebSocket and verify active scenes.` },
        ];
      }

      if (nodeType.includes("webhook") || nodeType.includes("trigger") || nodeType.includes("twitch") || nodeType.includes("event") || nodeType.includes("raid") || nodeType.includes("sub") || nodeType.includes("donation") || nodeType.includes("stripe")) {
        return [
          { icon: "🧪", label: "Test with Sample Payload", prompt: `Simulate a test event for ${nodeLabel} with sample viewer and donation payload.` },
          { icon: "🔒", label: "Add Secret Verification", prompt: `How do I add webhook secret verification to ${nodeLabel}?` },
          { icon: "📡", label: "Pipe to Discord Alert", prompt: `Add a Discord Webhook action node after ${nodeLabel} that announces the event in chat.` },
          { icon: "📋", label: "Parse Dynamic Variables", prompt: `Show how to map dynamic variables (username, amount, tier) from ${nodeLabel} into downstream nodes.` },
        ];
      }

      if (nodeType.includes("condition") || nodeType.includes("filter") || nodeType.includes("branch") || nodeType.includes("switch") || nodeType.includes("if") || nodeType.includes("logic")) {
        return [
          { icon: "🔀", label: "Branch by Donation > $10", prompt: `Configure ${nodeLabel} condition: if $event.payload.amount >= 10 route to VIP Alert, else route to Standard Alert.` },
          { icon: "👤", label: "Filter by Sub Tier", prompt: `Configure ${nodeLabel} to check if viewer tier is Tier 3 or Prime.` },
          { icon: "🧪", label: "Test True & False Paths", prompt: `Simulate inputs to test both True and False output wires of ${nodeLabel}.` },
          { icon: "⚙️", label: "Add Fallback Else Path", prompt: `Help me structure a safe fallback path if ${nodeLabel} evaluates to null or false.` },
        ];
      }

      if (nodeType.includes("audio") || nodeType.includes("sound") || nodeType.includes("tts") || nodeType.includes("voice") || nodeType.includes("speak") || nodeType.includes("music")) {
        return [
          { icon: "🔊", label: "Set Volume to 80%", prompt: `Set playback volume of ${nodeLabel} to 80% with smooth fade-in.` },
          { icon: "🗣️", label: "Convert to TTS Alert", prompt: `Convert ${nodeLabel} to speak the streamer donation message using text-to-speech.` },
          { icon: "🎵", label: "Play Fanfare SFX", prompt: `Set sound file in ${nodeLabel} to play fanfare alert clip on trigger.` },
          { icon: "⏱️", label: "Add Fade-out Delay", prompt: `Add audio fade-out delay after ${nodeLabel} finishes playing.` },
        ];
      }

      if (nodeType.includes("http") || nodeType.includes("api") || nodeType.includes("fetch") || nodeType.includes("request") || nodeType.includes("rest") || nodeType.includes("post") || nodeType.includes("get")) {
        return [
          { icon: "🌐", label: "Set Bearer Auth Header", prompt: `Configure Bearer Token authorization header in ${nodeLabel}.` },
          { icon: "📦", label: "Format JSON Body", prompt: `Format JSON POST payload for ${nodeLabel} with dynamic event variables.` },
          { icon: "🔄", label: "Add 3x Retry Policy", prompt: `Add automatic retry logic with exponential backoff if ${nodeLabel} returns HTTP 5xx.` },
          { icon: "⏱️", label: "Set 5s Timeout", prompt: `Configure a 5000ms request timeout on ${nodeLabel}.` },
        ];
      }

      if (nodeType.includes("timer") || nodeType.includes("delay") || nodeType.includes("wait") || nodeType.includes("sleep") || nodeType.includes("cron") || nodeType.includes("schedule")) {
        return [
          { icon: "⏱️", label: "Change Delay to 10s", prompt: `Set the wait duration of ${nodeLabel} to 10 seconds.` },
          { icon: "⏰", label: "Run Every 30 Minutes", prompt: `Configure ${nodeLabel} cron schedule to trigger every 30 minutes during stream.` },
          { icon: "⏸️", label: "Add Debounce Window", prompt: `Add a 3-second debounce window to ${nodeLabel} to prevent rapid spamming.` },
          { icon: "🔍", label: "Explain Timer Sync", prompt: `Explain how ${nodeLabel} handles asynchronous pauses without freezing the flow.` },
        ];
      }

      if (nodeType.includes("overlay") || nodeType.includes("hud") || nodeType.includes("widget") || nodeType.includes("alertbox") || nodeType.includes("goal") || nodeType.includes("chatbox")) {
        return [
          { icon: "🎨", label: "Trigger Celebration", prompt: `Configure ${nodeLabel} to send 'play_confetti' command to browser overlay.` },
          { icon: "📝", label: "Increment Goal +1", prompt: `Configure ${nodeLabel} to increment the on-screen sub goal counter.` },
          { icon: "⏱️", label: "Auto-hide after 5s", prompt: `Set ${nodeLabel} overlay message time-to-live (TTL) to 5000ms.` },
          { icon: "🩺", label: "Test Overlay Link", prompt: `Verify overlay WebSocket connection and test command delivery for ${nodeLabel}.` },
        ];
      }

      // Generic Node Fallback
      return [
        { icon: "🩺", label: `Why did this fail?`, prompt: `Why did ${nodeLabel} fail in my last flow execution? Query execution history and suggest an auto-fix.` },
        { icon: "⚙️", label: `Optimize Parameters`, prompt: `Review and optimize the configuration parameters for ${nodeLabel}.` },
        { icon: "📡", label: `Add Discord Webhook`, prompt: `Add a Discord webhook notification step right after ${nodeLabel}.` },
        { icon: "⏱️", label: `Add 5s Delay`, prompt: `Add a 5-second delay timer before executing ${nodeLabel}.` },
      ];
    }

    // 2. Deck Button Context
    if (activeContext.value.type === "deck_button") {
      const btn = activeContext.value.data || {};
      const btnMode = (btn.mode || "").toLowerCase();
      const btnAction = (btn.action || btn.action_type || "").toLowerCase();
      const title = activeContext.value.title;

      if (btnMode === "rotary") {
        return [
          { icon: "🎛️", label: "Bind to Master Volume", prompt: `Configure rotary dial ${title} to control Windows master system volume.` },
          { icon: "🎧", label: "Bind to Discord Voice", prompt: `Configure rotary dial ${title} to control Discord voice output volume.` },
          { icon: "🔄", label: "Reverse Direction", prompt: `Reverse the rotation direction sensitivity for ${title}.` },
          { icon: "📋", label: "Explain Rotary Binding", prompt: `Explain what action ${title} currently executes on rotation.` },
        ];
      }

      if (btnAction.includes("obs") || btnAction.includes("scene")) {
        return [
          { icon: "🎬", label: "Set to Switch Scene", prompt: `Configure ${title} to switch OBS to Gaming scene.` },
          { icon: "🎙️", label: "Set to Toggle Mic", prompt: `Configure ${title} to toggle microphone mute in OBS.` },
          { icon: "🔴", label: "Set to Toggle Stream", prompt: `Configure ${title} to safely start/stop live stream in OBS.` },
          { icon: "📋", label: "Explain OBS Action", prompt: `Explain the current OBS configuration on ${title}.` },
        ];
      }

      if (btnAction.includes("sound") || btnAction.includes("audio")) {
        return [
          { icon: "🔊", label: "Set Sound Clip", prompt: `Configure ${title} to play applause/fanfare sound effect.` },
          { icon: "🎚️", label: "Set Volume to 100%", prompt: `Set playback volume of ${title} to maximum 100%.` },
          { icon: "🗣️", label: "Convert to TTS Voice", prompt: `Convert ${title} to speak custom streamer voice line.` },
          { icon: "📋", label: "Explain Audio Action", prompt: `Explain the audio file and volume of ${title}.` },
        ];
      }

      if (btnAction.includes("macro")) {
        return [
          { icon: "⚡", label: "Add Keystroke Sequence", prompt: `Edit macro on ${title} to send Ctrl+Shift+M followed by F13.` },
          { icon: "⏱️", label: "Set Step Delay 200ms", prompt: `Set step delay to 200ms between actions on ${title}.` },
          { icon: "🔄", label: "Repeat While Pressing", prompt: `Convert ${title} execution mode to repeat-while-pressing.` },
          { icon: "📋", label: "Inspect Macro Steps", prompt: `Show all recorded steps inside macro ${title}.` },
        ];
      }

      return [
        { icon: "✨", label: "Toggle Mic & Discord", prompt: `Configure ${title} to toggle microphone mute and Discord mute simultaneously.` },
        { icon: "🎬", label: "Switch to Gaming Scene", prompt: `Configure ${title} to switch OBS to Gaming Scene.` },
        { icon: "🔊", label: "Add Sound Effect", prompt: `Add a sound effect trigger when ${title} is pressed.` },
        { icon: "🎛️", label: "Convert to Rotary Dial", prompt: `Convert ${title} into a rotary knob that controls system volume.` },
      ];
    }

    // 3. Deck Context
    if (activeContext.value.type === "deck") {
      return [
        { icon: "🎨", label: "Suggest Deck Layout", prompt: "Suggest an optimal streaming button layout for this deck." },
        { icon: "🚀", label: "Add Stream Starter Pack", prompt: "Add essential streaming actions (OBS Scene, Mic Mute, Discord, Clip) to this deck." },
      ];
    }

    return [
      { icon: "💬", label: "Ask about this context", prompt: `Help me configure and automate: ${activeContext.value.title}` },
    ];
  });

  return {
    contextSuggestions,
  };
};
