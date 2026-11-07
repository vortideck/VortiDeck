<template lang="pug">
div(class="h-screen flex flex-col overflow-hidden")

  // Main content area
  div(class="flex-1 w-full max-w-7xl mx-auto px-6 pb-6 overflow-y-scroll")
  
    // Prettier Header with gradient
    div(class="rounded-xl p-6 mb-6 border border-themeButton/20" style="background: linear-gradient(to right, hsl(var(--button) / 0.1), hsl(var(--secondary) / 0.1))")
      div(class="flex items-center gap-4")
        div(class="flex items-center gap-2")
          button(@click="selectCustomIcon" :disabled="isIconProcessing" class="w-12 h-12 rounded-xl flex items-center justify-center shadow-lg hover:shadow-xl transition-shadow cursor-pointer relative group icon-gradient" :title="'Click to select custom icon'")
            // Loading spinner when processing
            div(v-if="isIconProcessing" class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin")
            // Custom icon if selected
            img(v-else-if="customIconPreview" :src="customIconPreview" class="w-12 h-12 rounded-lg object-cover")
            // Default command icon
            HugeiconsCommandLine(v-else class="w-6 h-6 text-white")
            // Overlay on hover (only show when not processing)
            div(v-if="!isIconProcessing" class="absolute inset-0 bg-black/20 rounded-xl opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center")
              div(class="w-4 h-4 bg-white rounded-full flex items-center justify-center")
                PlusIcon(class="w-3 h-3 text-themeText")
          // Remove icon button
          button(v-if="customIconPreview" @click="removeCustomIcon" class="w-8 h-8 bg-themeError hover:brightness-90 rounded-lg flex items-center justify-center shadow-md hover:shadow-lg transition-all" :title="'Remove custom icon'")
            XMarkIcon(class="w-4 h-4 text-white")
        div
          h1(class="text-2xl font-bold text-themeText") {{ programId ? 'Edit' : 'New' }} Program
          p(class="text-themeText2") Configure executable or command to run on your deck
          //- p(v-if="customIconPath" class="text-xs text-blue-400 mt-1") Custom icon: {{ customIconPath.split(/[\\\/]/).pop() }}

    // Prettier Form with cards
    div(class="space-y-6")

      // Basic Settings Card
      div(class="bg-themeBackground2 rounded-xl p-6 border border-themeBorder2 shadow-sm")
        h3(class="text-lg font-semibold text-themeText mb-4 flex items-center gap-2")
          div(class="w-5 h-5 bg-themeButton rounded")
          | Basic Settings

        div(class="grid grid-cols-4 gap-4")
          div(class="col-span-2")
            label(class="block text-sm font-medium text-themeText2 mb-2") Program Name
            input(
              v-model="programName"
              class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
              placeholder="My Program"
            )
          div
            label(class="block text-sm font-medium text-themeText2 mb-2") Type
            select(
              v-model="programType"
              class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
            )
              option(value="command") Command
              option(value="executable") Executable
          div
            label(class="block text-sm font-medium text-themeText2 mb-2") Execution Mode
            select(
              v-model="showTerminal"
              class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
            )
              option(:value="false") Background
              option(:value="true") Terminal

        
        
        // Path/Command section
        div(class="mt-6")
          label(class="block text-sm font-medium text-themeText2 mb-2")
            | {{ programType === 'executable' ? 'Executable Path' : 'Command Line' }}
          div(class="flex gap-3")
            input(
                v-model="programPath"
                class=" px-4 w-full py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
                :placeholder="programType === 'executable' ? 'C:\\Program Files\\MyApp\\app.exe' : 'npm run dev'"
              )
            
            button(
              v-if="programType === 'executable'"
              @click="selectProgramFile"
              class="px-6 py-3 bg-themeButton hover:brightness-110 text-white rounded-lg text-base transition-all shadow-md hover:shadow-lg"
            ) Browse


        // Description field
        div(class="mt-6")
          label(class="block text-sm font-medium text-themeText2 mb-2") Description (Optional)
          textarea(
            v-model="programDescription"
            class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton resize-none transition-all"
            placeholder="Brief description of what this program does (for better AI understanding)"
            rows="2"
          )

      // Advanced Settings Card (Collapsible)
      div(class="bg-themeBackground2 rounded-xl border border-themeBorder2 shadow-sm")
        button(
          @click="showAdvanced = !showAdvanced"
          class="w-full p-6 flex items-center justify-between text-left hover:bg-themeBackground3/30 transition-colors rounded-xl"
        )
          div(class="flex items-center gap-2")
            div(class="w-5 h-5 rounded" style="background: hsl(var(--secondary))")
            h3(class="text-lg font-semibold text-themeText") Advanced Settings
          ChevronIcon(:class="showAdvanced ? 'rotate-180' : ''" class="w-5 h-5 text-themeText3 transition-transform")

        div(v-if="showAdvanced" class="px-6 pb-6 border-t border-themeBorder2")
          div(class="grid gap-4 mt-6 gap-x-4" :class="programType === 'executable' ? 'grid-cols-2' : 'grid-cols-1'")
            div(class="flex gap-y-4 w-full gap-x-8" :class="programType === 'executable' ? 'flex-col': 'flex-row'")
              div(:class="programType === 'executable' ? ' w-full': ' w-3/4'")
                label(class="block text-sm font-medium text-themeText2 mb-2 ") Working Directory
                div(class="flex gap-2  ")
                  input(
                    v-model="workingDir"
                    class="flex-1  px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
                    :placeholder="systemHomeDir"
                  )
                  button(
                    @click="selectWorkingDir"
                    class="px-4 py-3 bg-themeBackground3 hover:bg-themeBorder2 text-themeText2 rounded-lg transition-all border border-themeBorder2"
                  )
                    FolderOpenIcon(class="w-5 h-5")
              div(class="w-1/4")
                label(class="block text-sm font-medium text-themeText2 mb-2 w-max") Timeout (seconds)
                input(
                  v-model.number="timeoutSeconds"
                  type="number"
                  class=" px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton transition-all"
                  placeholder="30"
                )
            div(v-if="programType === 'executable'")
              label(class="block text-sm font-medium text-themeText2 mb-2") Arguments
              textarea(
                v-model="programArgs"
                class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton resize-none transition-all"
                placeholder="--flag value&#10;--another-flag"
                rows="3"
              )
              p(class="text-xs text-themeText3 mt-2") Context menu placeholders: #[code.bg-themeBackground3.px-1.rounded {context_menu_path}] · #[code.bg-themeBackground3.px-1.rounded {context_menu_name}] · #[code.bg-themeBackground3.px-1.rounded {context_menu_parent}] · #[code.bg-themeBackground3.px-1.rounded {context_menu_extension}]
           

          // Environment Variables
          div(class="mt-6")
            label(class="block text-sm font-medium text-themeText2 mb-2") Environment Variables
            textarea(
              v-model="envVars"
              class="w-full px-4 py-3 text-base bg-themeBackground border border-themeBorder2 rounded-lg text-themeText focus:outline-none focus:ring-2 focus:ring-themeButton resize-none transition-all"
              placeholder="NODE_ENV=production&#10;DEBUG=true&#10;API_KEY=secret123"
              rows="3"
            )
            p(class="text-sm text-themeText3 mt-1") One variable per line in KEY=value format

    // Remove the compact options row since we moved those to the main grid

    // Compact Test Results
    div(v-if="testOutput" class="bg-themeBackground3/30 rounded p-3 text-sm")
      div(class="flex items-center justify-between mb-2")
        div(class="flex items-center gap-2")
          div(:class="lastTestResult?.exit_code === 0 ? 'bg-themeSuccess' : 'bg-themeError'" class="w-2 h-2 rounded-full")
          span(:class="lastTestResult?.exit_code === 0 ? 'text-themeSuccess' : 'text-themeError'" class="text-xs font-medium")
            | {{ lastTestResult?.exit_code === 0 ? 'Success' : 'Failed' }} ({{ lastTestResult?.exit_code }})
        button(@click="testOutput = ''" class="text-themeText3 hover:text-themeText")
          XMarkIcon(class="w-4 h-4")

      div(class="grid grid-cols-2 gap-4")
        div
          div(class="flex items-center justify-between mb-1")
            span(class="text-sm font-medium text-themeText3") stdout:
            button(@click="openOutputModal('stdout')" class="text-themeButton hover:opacity-80 text-sm px-2 py-1 bg-themeButton/10 rounded") View Full
          div(
            :class="isTerminalMode() ? 'bg-red-900/60 border border-red-400 text-red-100' : 'bg-black border border-green-600/30 text-green-400'"
            class="p-3 rounded text-sm font-mono max-h-20 overflow-hidden"
          )
            | {{ getOutputDisplay('stdout').substring(0, 120) }}{{ getOutputDisplay('stdout').length > 120 ? '...' : '' }}
        div
          div(class="flex items-center justify-between mb-1")
            span(class="text-sm font-medium text-themeText3") stderr:
            button(@click="openOutputModal('stderr')" class="text-themeButton hover:opacity-80 text-sm px-2 py-1 bg-themeButton/10 rounded") View Full
          div(
            :class="isTerminalMode() ? 'bg-red-900/60 border border-red-400 text-red-100' : 'bg-black border border-red-600/30 text-red-400'"
            class="p-3 rounded text-sm font-mono max-h-20 overflow-hidden"
          )
            | {{ getOutputDisplay('stderr').substring(0, 120) }}{{ getOutputDisplay('stderr').length > 120 ? '...' : '' }}

  // Fixed Footer
  div(class="flex-shrink-0 bg-themeBackground2/95 backdrop-blur-sm border-t border-themeBorder2 p-6")
    div(class="w-full max-w-7xl mx-auto flex justify-between items-center")
      div(class="flex items-center gap-3")
        button(
          @click="router.go(-1)"
          class="px-6 py-3 bg-themeBackground3 hover:bg-themeBorder2 text-themeText2 rounded-lg transition-all border border-themeBorder2"
        ) Cancel

        // Test buttons in footer
        div(v-if="canTest" class="flex items-center gap-2")
          button(
            @click="testProgram"
            :disabled="isTestRunning"
            :class="isTestRunning ? 'bg-yellow-500' : 'bg-themeSuccess hover:brightness-110'"
            class="px-4 py-3 text-white rounded-lg text-sm transition-colors flex items-center gap-2"
          )
            PlayIcon(v-if="!isTestRunning" class="w-4 h-4")
            Spinner(v-else class="w-4 h-4")
            span {{ isTestRunning ? 'Running...' : 'Test' }}

          button(
            v-if="lastTestResult"
            @click="showTestOutput"
            class="px-3 py-3 bg-themeButton hover:brightness-110 text-white rounded-lg text-sm transition-colors flex items-center gap-1"
          )
            DocumentTextIcon(class="w-4 h-4")
            span Output

      div(class="flex gap-3")
        button(
          @click="resetProgram"
          class="px-6 py-3 bg-themeBackground3 hover:bg-themeBorder2 text-themeText2 rounded-lg transition-all border border-themeBorder2"
        ) Reset
        button(
          @click="programId ? updateProgramAction() : saveProgramAction()"
          :disabled="!canSave || isSaving"
          :class="canSave && !isSaving ? 'save-btn-gradient shadow-lg hover:shadow-xl hover:brightness-105' : 'bg-themeBorder2 cursor-not-allowed'"
          class="px-8 py-3 text-white rounded-lg transition-all font-semibold flex items-center gap-2"
        )
          // Loading spinner
          div(v-if="isSaving" class="w-4 h-4 border-2 border-white border-t-transparent rounded-full animate-spin")
          span {{ isSaving ? (isIconProcessing ? 'Processing icon...' : 'Saving...') : (programId ? 'Update Program' : 'Save Program') }}

  // Full-size Modal - consistent size
  BaseModal(:is-open="showOutputModal" :z-index="100000" @close="closeOutputModal")
    div(class="vd-modal-card w-[90vw] h-[90vh] m-4 flex flex-col")
      // Pretty Header
      div(class="rounded-t-xl p-6 border-b border-themeBorder2 flex-shrink-0" style="background: linear-gradient(to right, hsl(var(--button) / 0.1), hsl(var(--secondary) / 0.1))")
        div(class="flex items-center justify-between")
          div(class="flex items-center gap-4")
            div(class="w-8 h-8 rounded-lg flex items-center justify-center" style="background: linear-gradient(to bottom right, hsl(var(--button)), hsl(var(--secondary)))")
              DocumentTextIcon(class="w-4 h-4 text-white")
            h3(class="text-xl font-bold text-themeText") Program Output
            div(class="flex bg-themeBackground3 rounded-lg p-1")
              button(
                @click="currentOutputStream = 'stdout'"
                :class="currentOutputStream === 'stdout' ? 'bg-themeSuccess text-white shadow-md' : 'text-themeText3 hover:text-themeText'"
                class="px-4 py-2 rounded text-sm font-medium transition-all"
              ) stdout
              button(
                @click="currentOutputStream = 'stderr'"
                :class="currentOutputStream === 'stderr' ? 'bg-themeError text-white shadow-md' : 'text-themeText3 hover:text-themeText'"
                class="px-4 py-2 rounded text-sm font-medium transition-all"
              ) stderr
          button(@click="closeOutputModal" class="text-themeText3 hover:text-themeText transition-colors")
            XMarkIcon(class="w-6 h-6")

      // Content
      div(class="flex-1 overflow-hidden p-6")
        pre(
          :class="getModalOutputClass()"
          class="p-6 rounded-xl text-base font-mono h-full overflow-auto whitespace-pre-wrap leading-relaxed"
        )
          | {{ getCurrentOutputContent() }}

      // Pretty Footer
      div(class="bg-themeBackground3/30 rounded-b-xl p-4 border-t border-themeBorder2 flex justify-between items-center flex-shrink-0")
        div(class="flex items-center gap-4 text-sm text-themeText3")
          span {{ getCurrentOutputLength() }} characters
          span {{ currentOutputStream === 'stdout' ? 'Standard Output' : 'Standard Error' }}
        button(@click="copyCurrentOutput" class="px-4 py-2 bg-themeButton hover:brightness-110 text-white rounded-lg transition-all shadow-md hover:shadow-lg") Copy to Clipboard
      
</template>

<script setup lang="ts">
import BaseModal from "../../../components/Modal/BaseModal.vue";
import HugeiconsCommandLine from "~icons/hugeicons/command-line";
import FolderOpenIcon from "~icons/heroicons/folder-open";
import PlayIcon from "~icons/heroicons/play";
import DocumentTextIcon from "~icons/heroicons/document-text";
import XMarkIcon from "~icons/heroicons/x-mark";
import ChevronIcon from "~icons/heroicons/chevron-down";
import PlusIcon from "~icons/heroicons/plus";
import Spinner from "~icons/svg-spinners/ring-resize";

import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { homeDir } from "@tauri-apps/api/path";

interface ProgramExecutionResult {
  exit_code: number;
  stdout: string;
  stderr: string;
  execution_time_ms: number;
  success?: boolean;
  message?: string;
}

interface StoredProgram {
  id: number;
  name: string;
  executable: string;
  args: string | null;
  working_dir: string | null;
  env_vars: string | null;
  timeout_ms: number | null;
  show_terminal: boolean;
  program_type: string;
  description: string | null;
  custom_icon: string | null;
}

interface ServerInfo {
  ip: string;
  iconsServerPort: number;
}

type OutputStream = "stdout" | "stderr";
type SaveProgramResponse = [message: string, programId: number];

const queryString = (value: unknown): string | null => {
  if (typeof value === "string") return value;
  if (Array.isArray(value) && typeof value[0] === "string") return value[0];
  return null;
};

const route = useRoute();
const router = useRouter();

// Default settings
const DEFAULT_SETTINGS = {
  programName: "New Program",
  programType: "command",
  programPath: "",
  programArgs: "",
  workingDir: "",
  timeoutSeconds: 30,
  showTerminal: false,
};

// Route parameters
const programId = ref(queryString(route.query.programId));
const copyFromId = ref(queryString(route.query.copyFrom));

// Form data
const programName = ref(DEFAULT_SETTINGS.programName);
const programType = ref(DEFAULT_SETTINGS.programType);
const programPath = ref(DEFAULT_SETTINGS.programPath);
const programArgs = ref(DEFAULT_SETTINGS.programArgs);
const workingDir = ref(DEFAULT_SETTINGS.workingDir);
const timeoutSeconds = ref(DEFAULT_SETTINGS.timeoutSeconds);
const showTerminal = ref(DEFAULT_SETTINGS.showTerminal);
const envVars = ref("");
const programDescription = ref("");
const customIconPath = ref("");
const customIconPreview = ref("");

// Test functionality
const isTestRunning = ref(false);
const lastTestResult = ref<ProgramExecutionResult | null>(null);

// Loading states
const isSaving = ref(false);
const isIconProcessing = ref(false);

// Track icon state changes
const iconRemovalPending = ref(false); // Mark icon for removal on save
const testOutput = ref("");
const showOutputModal = ref(false);
const currentOutputStream = ref<OutputStream>("stdout");

// UI state
const systemHomeDir = ref("~");
const showAdvanced = ref(false);
const isLoadingData = ref(false);

// Computed properties
const canSave = computed(() => {
  return Boolean(programName.value.trim() && programPath.value.trim());
});

const canTest = computed(() => {
  return canSave.value;
});

// Watchers
watch(programName, (newVal) => {
  if (!newVal) {
    programName.value = DEFAULT_SETTINGS.programName;
  }
});

// File selection
const selectProgramFile = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Executable Files",
          extensions: ["exe", "bat", "cmd", "ps1", "sh", "py", "js", "ts"],
        },
        { name: "All Files", extensions: ["*"] },
      ],
    });

    if (typeof selected === "string") {
      programPath.value = selected;

      // Auto-detect program name
      if (programName.value === DEFAULT_SETTINGS.programName) {
        const baseName = selected
          .split(/[\\\/]/)
          .pop()
          ?.replace(/\.[^/.]+$/, "");
        if (baseName) {
          programName.value = baseName.charAt(0).toUpperCase() + baseName.slice(1);
        }
      }

      // Auto-detect working directory
      const lastSlash = Math.max(selected.lastIndexOf("\\"), selected.lastIndexOf("/"));
      if (lastSlash > 0) {
        workingDir.value = selected.substring(0, lastSlash);
      }
    }
  } catch (error) {
    console.error("Error selecting file:", error);
  }
};

const selectWorkingDir = async () => {
  try {
    const selected = await open({
      title: "Select Working Directory",
      directory: true,
    });

    if (typeof selected === "string") {
      workingDir.value = selected;
    }
  } catch (error) {
    console.error("Error selecting directory:", error);
  }
};

const removeCustomIcon = () => {
  // Just clear the preview and mark for removal
  customIconPath.value = "";
  customIconPreview.value = "";

  // If editing existing program with icon, mark it for removal
  if (programId.value) {
    iconRemovalPending.value = true;
  }
};

const selectCustomIcon = async () => {
  try {
    const selected = await open({
      multiple: false,
      filters: [
        {
          name: "Image Files",
          extensions: ["png", "jpg", "jpeg", "ico", "svg", "webp"],
        },
        { name: "All Files", extensions: ["*"] },
      ],
    });

    if (typeof selected === "string") {
      customIconPath.value = selected;
      iconRemovalPending.value = false; // Clear removal flag if selecting new icon

      // Read the file as base64 for preview (like buttons do)
      try {
        const base64Data = await invoke<string>("read_file_as_base64", {
          filePath: selected,
        });

        // Create data URL for preview
        customIconPreview.value = `data:image/png;base64,${base64Data}`;
      } catch (error) {
        console.error("Error reading file for preview:", error);
        // Fallback to asset protocol if base64 reading fails
        customIconPreview.value = `asset://localhost/${selected}`;
      }
    }
  } catch (error) {
    console.error("Error selecting icon:", error);
  }
};

// Test functionality
const testProgram = async () => {
  if (isTestRunning.value) return;

  isTestRunning.value = true;
  testOutput.value = "";

  try {
    const programData = buildProgramData();

    const result = await invoke<ProgramExecutionResult>("execute_program", {
      executable: programData.executable,
      args: programData.args,
      workingDir: programData.workingDir,
      envVars: programData.envVars,
      timeoutMs: programData.timeoutMs,
      showTerminal: showTerminal.value,
    });

    lastTestResult.value = result;
    showTestOutput();
  } catch (error) {
    console.error("Test execution failed:", error);
    lastTestResult.value = {
      exit_code: -1,
      stdout: "",
      stderr: error instanceof Error ? error.message : String(error),
      execution_time_ms: 0,
    };
    showTestOutput();
  } finally {
    isTestRunning.value = false;
  }
};

const showTestOutput = () => {
  testOutput.value = "visible";
};

// Build program data
const buildProgramData = () => {
  const args = programArgs.value
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  // Parse environment variables
  const envVarsObj: Record<string, string> = {};
  if (envVars.value.trim()) {
    envVars.value
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line.length > 0 && line.includes("="))
      .forEach((line) => {
        const [key, ...valueParts] = line.split("=");
        if (key && valueParts.length > 0) {
          envVarsObj[key.trim()] = valueParts.join("=").trim();
        }
      });
  }

  let executable: string;
  let finalArgs: string[];

  if (programType.value === "command") {
    // For commands, store the entire command line in executable
    // The command will be executed through a shell
    executable = programPath.value.trim();
    finalArgs = args; // Additional args from the args field
  } else {
    executable = programPath.value;
    finalArgs = args;
  }

  return {
    executable,
    args: finalArgs,
    workingDir: workingDir.value || null,
    envVars: Object.keys(envVarsObj).length > 0 ? envVarsObj : null,
    timeoutMs: timeoutSeconds.value ? timeoutSeconds.value * 1000 : null,
    description: programDescription.value || null,
    customIcon: customIconPath.value || null,
  };
};

// Save/Update methods
const saveProgramAction = async () => {
  isSaving.value = true;
  let iconStatus: "processing" | null = null;

  try {
    const programData = buildProgramData();

    // Step 1: Process icon first if selected (but don't save to icon service yet)
    if (customIconPath.value) {
      isIconProcessing.value = true;
      try {
        // Just mark that we have an icon to process
        iconStatus = "processing";
      } catch (iconError) {
        console.error("❌ Error with icon:", iconError);
      } finally {
        isIconProcessing.value = false;
      }
    }

    // Step 2: Save program ONCE with icon status
    const result = await invoke<SaveProgramResponse>("save_program_to_db", {
      programData: {
        name: programName.value,
        executable: programData.executable,
        args: programData.args,
        working_dir: programData.workingDir,
        env_vars: programData.envVars,
        timeout_ms: programData.timeoutMs,
        show_terminal: showTerminal.value,
        program_type: programType.value,
        description: programData.description,
        custom_icon: iconStatus ? "exists" : null,
      },
    });

    // Step 3: Process icon after we have program ID (but don't update DB again)
    const savedProgramId = result[1];

    if (iconStatus && savedProgramId) {
      try {
        const base64Data = await invoke<string>("read_file_as_base64", {
          filePath: customIconPath.value,
        });

        await invoke("save_icon", {
          buttonId: `program_${savedProgramId}`,
          base64Image: base64Data,
        });
      } catch (iconError) {
        console.error("❌ Error saving icon:", iconError);
        // Icon failed but program is already saved - that's OK
      }
    }

    await router.push("/deck");
  } catch (error) {
    console.error("Error saving program:", error);
  } finally {
    isSaving.value = false;
  }
};

const updateProgramAction = async () => {
  const editingProgramId = Number(programId.value);
  if (!Number.isInteger(editingProgramId)) {
    console.error("Cannot update program: invalid program ID");
    return;
  }

  isSaving.value = true;
  let finalIconStatus: "exists" | null = null;

  try {
    const programData = buildProgramData();

    // Step 1: Determine final icon status
    if (iconRemovalPending.value) {
      finalIconStatus = null; // Remove icon
    } else if (customIconPath.value === "existing") {
      finalIconStatus = "exists"; // Keep existing icon
    } else if (customIconPath.value) {
      finalIconStatus = "exists"; // New icon will be processed
    }

    // Step 2: Update program ONCE with final icon status
    await invoke("update_program_in_db", {
      programId: editingProgramId,
      programData: {
        name: programName.value,
        executable: programData.executable,
        args: programData.args,
        working_dir: programData.workingDir,
        env_vars: programData.envVars,
        timeout_ms: programData.timeoutMs,
        show_terminal: showTerminal.value,
        program_type: programType.value,
        description: programData.description,
        custom_icon: finalIconStatus,
      },
    });

    // Step 3: Handle icon operations after DB update (no more DB updates)
    if (iconRemovalPending.value) {
      try {
        await invoke("remove_icon", {
          buttonId: `program_${programId.value}`,
        });
      } catch (iconError) {
        console.error("Error removing icon:", iconError);
      }
    } else if (customIconPath.value && customIconPath.value !== "existing") {
      isIconProcessing.value = true;
      try {
        const base64Data = await invoke<string>("read_file_as_base64", {
          filePath: customIconPath.value,
        });

        await invoke("save_icon", {
          buttonId: `program_${programId.value}`,
          base64Image: base64Data,
        });
      } catch (iconError) {
        console.error("❌ Error saving icon:", iconError);
      } finally {
        isIconProcessing.value = false;
      }
    }

    await router.push("/deck");
  } catch (error) {
    console.error("Error updating program:", error);
  } finally {
    isSaving.value = false;
  }
};

const resetProgram = () => {
  programName.value = DEFAULT_SETTINGS.programName;
  programType.value = DEFAULT_SETTINGS.programType;
  programPath.value = DEFAULT_SETTINGS.programPath;
  programArgs.value = DEFAULT_SETTINGS.programArgs;
  workingDir.value = DEFAULT_SETTINGS.workingDir;
  timeoutSeconds.value = DEFAULT_SETTINGS.timeoutSeconds;
  showTerminal.value = DEFAULT_SETTINGS.showTerminal;
  envVars.value = "";
  programDescription.value = "";
  customIconPath.value = "";
  customIconPreview.value = "";
  testOutput.value = "";
  lastTestResult.value = null;
};

// Modal functions
const openOutputModal = (stream: OutputStream = "stdout") => {
  currentOutputStream.value = stream;
  showOutputModal.value = true;
};

const closeOutputModal = () => {
  showOutputModal.value = false;
};

const isTerminalMode = () => {
  return (
    lastTestResult.value?.message &&
    lastTestResult.value.message.includes("Program launched with terminal")
  );
};

const getOutputDisplay = (type: OutputStream) => {
  if (!lastTestResult.value) return "(no data)";

  const output =
    type === "stdout" ? lastTestResult.value.stdout : lastTestResult.value.stderr;

  if (isTerminalMode()) {
    return type === "stdout"
      ? "⚠️ No output captured - program ran in terminal mode"
      : "⚠️ No errors captured - program ran in terminal mode";
  }

  return output || (type === "stdout" ? "(no output)" : "(no errors)");
};

const getCurrentOutputContent = () => {
  return getOutputDisplay(currentOutputStream.value);
};

const getCurrentOutputLength = () => {
  return getCurrentOutputContent().length;
};

const getModalOutputClass = () => {
  if (isTerminalMode()) {
    return "bg-red-900/60 border border-red-400 text-red-100";
  }
  return currentOutputStream.value === "stdout"
    ? "bg-black border border-green-600/30 text-green-400"
    : "bg-black border border-red-600/30 text-red-400";
};

const copyCurrentOutput = async () => {
  const content =
    currentOutputStream.value === "stdout"
      ? lastTestResult.value?.stdout || ""
      : lastTestResult.value?.stderr || "";

  try {
    await navigator.clipboard.writeText(content);
    console.log("Output copied to clipboard");
  } catch (error) {
    console.error("Failed to copy:", error);
  }
};

// Load data on mount
onMounted(async () => {
  try {
    const homePath = await homeDir();
    systemHomeDir.value = homePath;
  } catch (error) {
    console.warn("Could not get home directory:", error);
  }

  // Check if we're in copy mode or edit mode
  const idToLoad = copyFromId.value || programId.value;
  if (idToLoad) {
    try {
      isLoadingData.value = true;
      const programData = await invoke<StoredProgram | null>("get_program_by_id", {
        programId: parseInt(idToLoad),
      });

      if (programData) {
        console.log("Loading program data:", programData);
        // If copying, append " (Copy)" to the name
        programName.value = copyFromId.value ? `${programData.name} (Copy)` : (programData.name || "Program");
        programPath.value = programData.executable || "";

        // Parse args from JSON string
        if (programData.args) {
          try {
            const argsArray = JSON.parse(programData.args);
            programArgs.value = argsArray.join("\n");
          } catch (e) {
            programArgs.value = "";
          }
        } else {
          programArgs.value = "";
        }

        workingDir.value = programData.working_dir || "";
        timeoutSeconds.value = programData.timeout_ms
          ? programData.timeout_ms / 1000
          : 30;
        // Load program type and show_terminal
        programType.value = programData.program_type || "executable";
        showTerminal.value = programData.show_terminal || false;

        // Parse env_vars from JSON string
        if (programData.env_vars) {
          try {
            const envVarsObj = JSON.parse(programData.env_vars);
            envVars.value = Object.entries(envVarsObj)
              .map(([key, value]) => `${key}=${value}`)
              .join("\n");
          } catch (e) {
            envVars.value = "";
          }
        } else {
          envVars.value = "";
        }

        // Load description
        programDescription.value = programData.description || "";

        // Check if custom icon exists using database field
        if (programData.custom_icon) {
          try {
            // Get server info for icon preview
            const serverInfo = await invoke<ServerInfo | null>("get_server_info_new");

            if (serverInfo?.ip && serverInfo?.iconsServerPort) {
              // Use HTTP service URL for preview with cache buster
              const timestamp = Date.now();
              customIconPreview.value = `http://${serverInfo.ip}:${serverInfo.iconsServerPort}/icons/program_${programData.id}/icon.webp?t=${timestamp}`;
              customIconPath.value = "existing"; // Mark as existing icon
            }
          } catch (iconError) {
            console.error("Error setting up custom icon preview:", iconError);
          }
        }

        // If copying, clear the programId so it creates a new entry
        if (copyFromId.value) {
          programId.value = null;
        }
      }
    } catch (error) {
      console.error("Error loading program data:", error);
    } finally {
      isLoadingData.value = false;
    }
  }
});
</script>

<style scoped lang="scss">
@use "../../../components/Modal/modalTheme.scss" as *;

/* Minimal custom styles */
input[type="number"]::-webkit-outer-spin-button,
input[type="number"]::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

input[type="number"] {
  -moz-appearance: textfield;
}

.icon-gradient {
  background: linear-gradient(to bottom right, hsl(var(--button)), hsl(var(--secondary)));
}

.save-btn-gradient {
  background: linear-gradient(to right, hsl(var(--button)), hsl(var(--secondary)));
}
</style>
