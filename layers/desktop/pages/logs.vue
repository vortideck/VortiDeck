<template lang="pug">
div(class="flex flex-col h-full w-full bg-themeBackground text-themeText select-text overflow-hidden")
  //- Logs Toolbar
  div(class="flex flex-wrap items-center justify-between gap-3 p-3.5 bg-themeBackground2 border-b border-themeBorder shrink-0")
    div(class="flex items-center gap-2.5")
      div(class="w-7 h-7 rounded-lg bg-themePrimary/10 border border-themePrimary/20 flex items-center justify-center text-themePrimary")
        MdiFileDocumentOutline(class="w-4 h-4")
      div
        div(class="flex items-center gap-2")
          h1(class="text-xs font-bold text-themeText uppercase tracking-wider") System Log Stream
          span(class="px-2 py-0.5 rounded-full text-[10px] font-semibold bg-themeBackground border border-themeBorder text-themeText3 font-mono") {{ filteredLines.length }} lines

    //- Controls & Search
    div(class="flex flex-wrap items-center gap-2")
      //- Search input
      div(class="relative")
        input(
          v-model="searchQuery"
          type="text"
          placeholder="Filter log text..."
          class="w-40 sm:w-56 pl-7 pr-3 py-1.5 bg-themeBackground border border-themeBorder rounded-xl text-xs text-themeText placeholder:text-themeText3 focus:outline-none focus:border-themePrimary shadow-2xs"
        )
        MdiTextBoxSearch(class="w-3.5 h-3.5 text-themeText3 absolute left-2 top-2")

      //- Level filter
      select(
        v-model="selectedLevel"
        class="px-2.5 py-1.5 bg-themeBackground border border-themeBorder rounded-xl text-xs text-themeText focus:outline-none focus:border-themePrimary shadow-2xs cursor-pointer"
      )
        option(value="ALL") All Levels
        option(value="INFO") INFO
        option(value="WARN") WARN
        option(value="ERROR") ERROR
        option(value="DEBUG") DEBUG

      //- Auto-scroll toggle
      button(
        @click="autoScroll = !autoScroll"
        :class="autoScroll ? 'bg-themePrimary/10 text-themePrimary border-themePrimary/20' : 'bg-themeBackground text-themeText3 border-themeBorder'"
        class="px-2.5 py-1.5 rounded-xl border text-xs font-medium transition-colors flex items-center gap-1 cursor-pointer shadow-2xs"
        title="Toggle auto-scroll to latest log entries"
      )
        span(class="w-1.5 h-1.5 rounded-full" :class="autoScroll ? 'bg-themePrimary animate-pulse' : 'bg-themeText3'")
        span Auto-scroll

      //- Copy button
      button(
        @click="copyLogs"
        class="px-2.5 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors flex items-center gap-1 cursor-pointer shadow-2xs active:scale-95"
        title="Copy logs to clipboard"
      )
        MdiContentCopy(class="w-3.5 h-3.5")
        span Copy

      //- Reload button
      button(
        @click="loadLogs"
        class="px-2.5 py-1.5 rounded-xl bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors flex items-center gap-1 cursor-pointer shadow-2xs active:scale-95"
        title="Reload from log file"
      )
        MdiRefresh(class="w-3.5 h-3.5")
        span Reload

      //- Clear button
      button(
        @click="clearLogsView"
        class="p-1.5 rounded-xl bg-red-500/10 hover:bg-red-500/20 text-red-400 border border-red-500/20 text-xs transition-colors flex items-center cursor-pointer shadow-2xs"
        title="Clear display"
      )
        MdiTrashCan(class="w-3.5 h-3.5")

  //- Logs Display Area
  div.log-viewer(ref="logContainer" class="flex-1 overflow-y-auto p-4 font-mono text-xs leading-relaxed space-y-0.5 select-text")
    div(v-if="filteredLines.length === 0" class="flex flex-col items-center justify-center h-full text-center space-y-2 text-themeText3")
      div(class="w-10 h-10 rounded-xl bg-themeBackground2 border border-themeBorder flex items-center justify-center")
        MdiFileDocumentOutline(class="w-5 h-5")
      p(class="text-xs font-semibold") {{ searchQuery ? 'No log lines match your query' : 'No logs recorded' }}

    div(
      v-for="(line, idx) in filteredLines"
      :key="idx"
      :class="getLineClass(line)"
      class="py-0.5 px-2 rounded font-mono break-all hover:bg-themeBackground2/60 transition-colors"
    )
      | {{ line }}

  //- Toast
  transition(name="fade")
    div(v-if="toast" class="fixed bottom-4 right-4 px-3.5 py-2 rounded-xl bg-themeBackground2 border border-themeBorder text-xs text-themeText shadow-lg z-50 flex items-center gap-2")
      MdiCheck(class="w-3.5 h-3.5 text-emerald-400")
      span {{ toast }}
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick } from "vue";
import { attachLogger } from "@tauri-apps/plugin-log";
import { readTextFile, BaseDirectory } from "@tauri-apps/plugin-fs";
import { getCurrentWindow } from "@tauri-apps/api/window";

import MdiFileDocumentOutline from "~icons/mdi/file-document-outline";
import MdiTextBoxSearch from "~icons/mdi/text-box-search-outline";
import MdiContentCopy from "~icons/mdi/content-copy";
import MdiTrashCan from "~icons/mdi/trash-can-outline";
import MdiRefresh from "~icons/mdi/refresh";
import MdiCheck from "~icons/mdi/check";

const rawLogs = ref<string>("");
const logContainer = ref<HTMLElement | null>(null);
const searchQuery = ref("");
const selectedLevel = ref("ALL");
const autoScroll = ref(true);
const toast = ref("");

let unlisten: (() => void) | null = null;

const allLines = computed(() => {
  if (!rawLogs.value) return [];
  return rawLogs.value.split("\n").filter((l) => l.trim().length > 0);
});

const filteredLines = computed(() => {
  let lines = allLines.value;

  if (selectedLevel.value !== "ALL") {
    const lvl = `[${selectedLevel.value}]`;
    const altLvl = selectedLevel.value;
    lines = lines.filter(
      (l) =>
        l.toUpperCase().includes(lvl) || l.toUpperCase().includes(` ${altLvl} `)
    );
  }

  if (searchQuery.value.trim()) {
    const q = searchQuery.value.toLowerCase();
    lines = lines.filter((l) => l.toLowerCase().includes(q));
  }

  return lines;
});

const getLineClass = (line: string) => {
  const upper = line.toUpperCase();
  if (upper.includes("ERROR") || upper.includes("[ERROR]") || upper.includes("PANIC")) {
    return "text-red-400 bg-red-500/5";
  }
  if (upper.includes("WARN") || upper.includes("[WARN]")) {
    return "text-amber-400 bg-amber-500/5";
  }
  if (upper.includes("INFO") || upper.includes("[INFO]")) {
    return "text-themeText";
  }
  if (upper.includes("DEBUG") || upper.includes("[DEBUG]")) {
    return "text-themeText3";
  }
  return "text-themeText2";
};

const showToast = (msg: string) => {
  toast.value = msg;
  setTimeout(() => {
    if (toast.value === msg) toast.value = "";
  }, 2500);
};

const copyLogs = async () => {
  try {
    await navigator.clipboard.writeText(filteredLines.value.join("\n"));
    showToast("Copied logs to clipboard");
  } catch (err) {
    showToast("Failed to copy logs");
  }
};

const clearLogsView = () => {
  rawLogs.value = "";
  showToast("Cleared log view");
};

/** Load existing logs from file */
async function loadLogs() {
  try {
    const content = await readTextFile("logs/logs.log", {
      baseDir: BaseDirectory.AppLocalData,
    });
    rawLogs.value = content;
    scrollToBottom();
    showToast("Loaded log file");
  } catch (error) {
    console.warn("No existing logs found or error reading log file:", error);
  }
}

function scrollToBottom() {
  if (!autoScroll.value) return;
  nextTick(() => {
    if (logContainer.value) {
      logContainer.value.scrollTop = logContainer.value.scrollHeight;
    }
  });
}

/** Start listening for new logs */
async function startLogger() {
  try {
    unlisten = await attachLogger((entry) => {
      rawLogs.value += `\n${entry.message}`;
      scrollToBottom();
    });
  } catch (error) {
    console.error("Error attaching logger:", error);
  }
}

// Lifecycle hooks
onMounted(async () => {
  try {
    await getCurrentWindow().show();
  } catch (e) {
    // browser preview fallback
  }

  await loadLogs();
  await startLogger();
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

definePageMeta({
  layout: "logs",
});
</script>

<style scoped>
.log-viewer {
  scrollbar-width: thin;
  scrollbar-color: hsl(var(--border)) hsl(var(--background));
}

.log-viewer::-webkit-scrollbar {
  width: 6px;
}

.log-viewer::-webkit-scrollbar-thumb {
  background-color: hsl(var(--border));
  border-radius: 3px;
}

.fade-enter-active,
.fade-leave-active {
  transition: opacity 0.2s ease;
}
.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}
</style>
