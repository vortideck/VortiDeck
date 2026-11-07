<template>
  <section class="rounded-xl border border-themeBorder bg-themeBackground p-4">
    <div class="flex flex-wrap items-start justify-between gap-3">
      <div>
        <div class="flex items-center gap-2">
          <h3 class="text-sm font-semibold text-themeText">OBS Companion</h3>
          <span
            v-if="overview"
            class="rounded-full border border-themeBorder bg-themeBackground2 px-2 py-0.5 text-[10px] font-mono text-themeText2"
          >
            bundled v{{ overview.payload.version }} ·
            {{ architectureLabel(overview.payload.architecture) }}
          </span>
        </div>
        <p class="mt-1 max-w-2xl text-[11px] leading-relaxed text-themeText2">
          VortiDeck manages the native OBS companion transactionally. Close OBS
          before installing, repairing, updating, or removing it.
        </p>
      </div>
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="rounded-lg border border-themeBorder bg-themeBackground2 px-3 py-1.5 text-[11px] font-medium text-themeText hover:border-themeBorder2"
          :disabled="busy"
          @click="addCustomTarget"
        >
          Add custom / portable OBS
        </button>
        <button
          type="button"
          class="rounded-lg border border-themeBorder bg-themeBackground2 p-1.5 text-themeText2 hover:text-themeText"
          :disabled="busy"
          title="Refresh OBS installations"
          @click="refresh"
        >
          <HeroiconsArrowPath
            class="h-4 w-4"
            :class="loading ? 'animate-spin' : ''"
          />
        </button>
      </div>
    </div>

    <div
      v-if="overview?.obsRunning"
      class="mt-3 flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-[11px] text-amber-300"
    >
      <HeroiconsExclamationTriangle class="mt-0.5 h-4 w-4 shrink-0" />
      <span
        >OBS Studio is running. Close it before changing the companion.</span
      >
    </div>

    <div
      v-if="error"
      class="mt-3 rounded-lg border border-red-500/30 bg-red-500/10 p-3 text-[11px] text-red-300"
    >
      {{ error }}
    </div>

    <div
      v-if="loading && !overview"
      class="flex items-center justify-center py-8 text-xs text-themeText3"
    >
      Detecting OBS installations…
    </div>

    <div v-else class="mt-4 space-y-2.5">
      <article
        v-for="target in allTargets"
        :key="target.id"
        class="rounded-xl border border-themeBorder bg-themeBackground2/60 p-3.5"
      >
        <div class="flex flex-wrap items-start justify-between gap-3">
          <div class="min-w-0 flex-1">
            <div class="flex flex-wrap items-center gap-2">
              <h4 class="truncate text-xs font-semibold text-themeText">
                {{ target.displayName }}
              </h4>
              <span
                class="rounded-full border px-2 py-0.5 text-[10px] font-semibold"
                :class="statusClass(target)"
              >
                {{ statusLabel(target) }}
              </span>
              <span
                class="rounded-full border border-themeBorder px-2 py-0.5 text-[10px] text-themeText3"
              >
                {{ architectureLabel(target.architecture) }}
              </span>
            </div>
            <p class="mt-1 break-all font-mono text-[10px] text-themeText3">
              {{ target.pluginPath }}
            </p>
            <p class="mt-1 text-[10px] text-themeText2">
              {{ target.statusDetail }}
            </p>
            <p
              v-if="target.requiresElevation"
              class="mt-1 text-[10px] text-themeText3"
            >
              Windows will request administrator approval for this location.
            </p>
          </div>

          <div class="flex shrink-0 items-center gap-1.5">
            <button
              v-if="!target.installed"
              type="button"
              class="rounded-lg bg-themeButton px-3 py-1.5 text-[11px] font-semibold text-white hover:bg-themeButtonHover disabled:opacity-50"
              :disabled="
                busy || overview?.obsRunning || !architectureCompatible(target)
              "
              @click="requestAction('install', target)"
            >
              Install
            </button>
            <button
              v-else-if="
                !target.healthy ||
                target.installedVersion !== overview?.payload.version
              "
              type="button"
              class="rounded-lg bg-amber-600 px-3 py-1.5 text-[11px] font-semibold text-white hover:bg-amber-700 disabled:opacity-50"
              :disabled="
                busy || overview?.obsRunning || !architectureCompatible(target)
              "
              @click="requestAction('repair', target)"
            >
              {{
                target.installedVersion !== overview?.payload.version
                  ? "Update"
                  : "Repair"
              }}
            </button>
            <button
              v-else
              type="button"
              class="rounded-lg border border-themeBorder bg-themeBackground px-3 py-1.5 text-[11px] font-medium text-themeText hover:border-themeBorder2 disabled:opacity-50"
              :disabled="busy || overview?.obsRunning"
              @click="requestAction('repair', target)"
            >
              Verify & reinstall
            </button>
            <button
              v-if="target.installed && target.installedVersion"
              type="button"
              class="rounded-lg border border-red-500/25 bg-red-500/10 px-3 py-1.5 text-[11px] font-medium text-red-400 hover:bg-red-500/20 disabled:opacity-50"
              :disabled="busy || overview?.obsRunning"
              @click="requestAction('remove', target)"
            >
              Remove
            </button>
            <button
              v-if="
                !target.installed &&
                (target.kind === 'custom' || target.kind === 'portable')
              "
              type="button"
              class="rounded-lg border border-themeBorder bg-themeBackground px-3 py-1.5 text-[11px] font-medium text-themeText3 hover:text-themeText disabled:opacity-50"
              :disabled="busy"
              @click="forgetTarget(target)"
            >
              Forget
            </button>
          </div>
        </div>

        <p
          v-if="!architectureCompatible(target)"
          class="mt-2 rounded-lg border border-red-500/20 bg-red-500/10 px-2.5 py-2 text-[10px] text-red-300"
        >
          This VortiDeck build contains
          {{ architectureLabel(overview?.payload.architecture) }}, but this OBS
          instance is {{ architectureLabel(target.architecture) }}. A matching
          companion build is required.
        </p>
      </article>
    </div>
  </section>

  <BaseModal
    :is-open="pendingAction !== null"
    :close-on-backdrop="!busy"
    :z-index="50020"
    @close="cancelPendingAction"
  >
    <section
      class="vd-modal-card w-[min(92vw,30rem)] p-5"
      role="dialog"
      aria-modal="true"
      aria-labelledby="obs-companion-confirmation-title"
    >
      <h3
        id="obs-companion-confirmation-title"
        class="text-sm font-bold text-themeText"
      >
        {{ confirmationTitle }} OBS Companion
      </h3>
      <p class="mt-2 text-xs leading-relaxed text-themeText2">
        Close OBS Studio first. This will {{ confirmationVerb }} the companion
        at the selected target.
      </p>
      <p
        class="mt-3 break-all rounded-lg border border-themeBorder bg-themeBackground2 p-2.5 font-mono text-[10px] text-themeText3"
      >
        {{ pendingAction?.target.pluginPath }}
      </p>
      <p
        v-if="pendingAction?.target.requiresElevation"
        class="mt-3 text-[11px] text-themeText2"
      >
        Windows will request administrator approval for this location.
      </p>

      <div
        v-if="modalError"
        class="mt-3 flex items-start gap-2 rounded-lg border border-red-500/30 bg-red-500/10 p-2.5 text-[11px] text-red-300"
      >
        <HeroiconsExclamationTriangle
          class="mt-0.5 h-3.5 w-3.5 shrink-0 text-red-400"
        />
        <span class="break-words">{{ modalError }}</span>
      </div>

      <div class="mt-5 flex justify-end gap-2">
        <button
          type="button"
          class="rounded-lg border border-themeBorder bg-themeBackground2 px-3 py-1.5 text-xs font-medium text-themeText hover:border-themeBorder2 disabled:opacity-50"
          :disabled="busy"
          @click="cancelPendingAction"
        >
          Cancel
        </button>
        <button
          type="button"
          class="rounded-lg px-3 py-1.5 text-xs font-semibold text-white disabled:opacity-50"
          :class="
            pendingAction?.action === 'remove'
              ? 'bg-red-600 hover:bg-red-700'
              : 'bg-themeButton hover:bg-themeButtonHover'
          "
          :disabled="busy"
          @click="performPendingAction"
        >
          {{ busy ? "Working…" : confirmationTitle }}
        </button>
      </div>
    </section>
  </BaseModal>
</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import HeroiconsArrowPath from "~icons/heroicons/arrow-path";
import HeroiconsExclamationTriangle from "~icons/heroicons/exclamation-triangle";
import BaseModal from "../Modal/BaseModal.vue";

type Architecture = "x86_64" | "arm64" | "unknown";
type InstallKind = "shared" | "standard" | "custom" | "portable";
type CompanionAction = "install" | "repair" | "remove";

interface CompanionManifest {
  version: string;
  architecture: Architecture;
  sha256: string;
}

interface CompanionTarget {
  id: string;
  displayName: string;
  kind: InstallKind;
  rootPath: string;
  executablePath?: string;
  architecture: Architecture;
  pluginPath: string;
  receiptPath: string;
  installed: boolean;
  installedVersion?: string;
  installedSha256?: string;
  healthy: boolean;
  statusDetail: string;
  requiresElevation: boolean;
}

interface CompanionOverview {
  payload: CompanionManifest;
  obsRunning: boolean;
  targets: CompanionTarget[];
}

const overview = ref<CompanionOverview | null>(null);
const customTargets = ref<CompanionTarget[]>([]);
const loading = ref(false);
const busy = ref(false);
const error = ref("");
const modalError = ref("");
const pendingAction = ref<{
  action: CompanionAction;
  target: CompanionTarget;
} | null>(null);

const allTargets = computed(() => {
  const result = [...(overview.value?.targets ?? [])];
  for (const custom of customTargets.value) {
    if (!result.some((item) => item.id === custom.id)) result.push(custom);
  }
  return result;
});

const architectureLabel = (architecture?: Architecture) => {
  if (architecture === "x86_64") return "Windows x64";
  if (architecture === "arm64") return "Windows ARM64";
  return "Unknown architecture";
};

const architectureCompatible = (target: CompanionTarget) => {
  const payload = overview.value?.payload.architecture;
  return (
    target.architecture === "unknown" ||
    !payload ||
    target.architecture === payload
  );
};

const statusLabel = (target: CompanionTarget) => {
  if (
    target.healthy &&
    target.installedVersion === overview.value?.payload.version
  )
    return `Installed v${target.installedVersion}`;
  if (
    target.installed &&
    target.installedVersion !== overview.value?.payload.version
  )
    return "Update available";
  if (target.installed) return "Needs repair";
  return "Not installed";
};

const statusClass = (target: CompanionTarget) => {
  if (
    target.healthy &&
    target.installedVersion === overview.value?.payload.version
  ) {
    return "border-emerald-500/25 bg-emerald-500/10 text-emerald-400";
  }
  if (target.installed)
    return "border-amber-500/25 bg-amber-500/10 text-amber-400";
  return "border-themeBorder bg-themeBackground text-themeText3";
};

const refresh = async () => {
  loading.value = true;
  error.value = "";
  try {
    overview.value = await invoke<CompanionOverview>(
      "get_obs_companion_overview",
    );
    const refreshed: CompanionTarget[] = [];
    for (const target of customTargets.value) {
      try {
        refreshed.push(
          await invoke<CompanionTarget>("inspect_obs_companion_target", {
            path: target.rootPath,
          }),
        );
      } catch {
        // A portable drive may be disconnected. Keep it out of this refresh.
      }
    }
    customTargets.value = refreshed;
  } catch (cause) {
    error.value = String(cause);
  } finally {
    loading.value = false;
  }
};

const addCustomTarget = async () => {
  error.value = "";
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Select the OBS Studio root folder",
  });
  if (!selected || Array.isArray(selected)) return;
  try {
    const target = await invoke<CompanionTarget>(
      "inspect_obs_companion_target",
      { path: selected },
    );
    if (!customTargets.value.some((item) => item.id === target.id))
      customTargets.value.push(target);
  } catch (cause) {
    error.value = String(cause);
  }
};

const confirmationVerb = computed(() => {
  if (pendingAction.value?.action === "remove") return "remove";
  if (pendingAction.value?.action === "repair") return "repair or update";
  return "install";
});

const confirmationTitle = computed(() => {
  if (pendingAction.value?.action === "remove") return "Remove";
  if (pendingAction.value?.action === "repair") return "Repair / update";
  return "Install";
});

const requestAction = (action: CompanionAction, target: CompanionTarget) => {
  error.value = "";
  modalError.value = "";
  pendingAction.value = { action, target };
};

const cancelPendingAction = () => {
  if (!busy.value) {
    modalError.value = "";
    pendingAction.value = null;
  }
};

const performPendingAction = async () => {
  const pending = pendingAction.value;
  if (!pending) return;

  busy.value = true;
  modalError.value = "";
  try {
    const command =
      pending.action === "install"
        ? "install_obs_companion"
        : pending.action === "repair"
          ? "repair_obs_companion"
          : "remove_obs_companion";
    await invoke(command, {
      target: {
        kind: pending.target.kind,
        rootPath: pending.target.rootPath,
      },
    });
    pendingAction.value = null;
    await refresh();
  } catch (cause) {
    modalError.value = String(cause);
  } finally {
    busy.value = false;
  }
};

const forgetTarget = async (target: CompanionTarget) => {
  busy.value = true;
  error.value = "";
  try {
    await invoke("forget_obs_companion_target", {
      target: { kind: target.kind, rootPath: target.rootPath },
    });
    customTargets.value = customTargets.value.filter(
      (item) => item.id !== target.id,
    );
    await refresh();
  } catch (cause) {
    error.value = String(cause);
  } finally {
    busy.value = false;
  }
};

let pollInterval: ReturnType<typeof setInterval> | null = null;

onMounted(() => {
  refresh();
  pollInterval = setInterval(() => {
    if (!busy.value && !pendingAction.value) {
      refresh();
    }
  }, 3000);
});

onUnmounted(() => {
  if (pollInterval) {
    clearInterval(pollInterval);
    pollInterval = null;
  }
});
</script>
