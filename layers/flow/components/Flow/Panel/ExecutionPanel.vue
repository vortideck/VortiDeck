<template lang="pug">
div(
  v-if="activeFlowExecution && showFlowStatusPanel"
  class="flow-execution-panel absolute bottom-4 left-6 z-[1000]"
  :class="isInspectorOpen ? 'flow-execution-panel--with-inspector' : 'flow-execution-panel--full'"
)
  div(class="flow-execution-panel__row")
    div(class="flow-execution-panel__meta")
      div(class="flow-execution-panel__badge")
        SolarBoltLinear(class="h-4 w-4" aria-hidden="true")
      div(class="flow-execution-panel__labels")
        h3(class="flow-execution-panel__title") {{ activeFlowExecution.flowDefinition.name }}
        div(class="flow-execution-panel__facts")
          span(:class="getFlowStatusClasses()") {{ getFlowStatusText() }}
          span(v-if="getConcurrentExecutionInfo") Concurrent: {{ getConcurrentExecutionInfo.currentlyRunning }}/{{ getConcurrentExecutionInfo.maxConcurrent }}
          span(v-if="getPendingNodeCount() > 0" class="text-amber-600 dark:text-amber-300") Queued: {{ getPendingNodeCount() }}
          span(v-if="recoveryQueueCount > 0" class="text-amber-600 dark:text-amber-300") Recovery queue: {{ recoveryQueueCount }}
          span(v-if="flowProgress > 0") {{ Math.round(flowProgress) }}%
          span(v-if="activeFlowExecution.status === FlowStatus.RecoveryRequired") {{ formatRecoveryTimestamp() }}
          span(v-else-if="activeFlowExecution.startedAt") {{ formatExecutionDuration() }}

    div(class="flow-execution-panel__actions")
      button(
        v-if="activeFlowExecution"
        class="flow-execution-panel__btn flow-execution-panel__btn--history"
        @click="handleShowHistory"
        title="View execution history and logs"
      )
        MdiHistory(class="h-4 w-4" aria-hidden="true")
        span History
      button(
        v-if="isFlowRunning"
        class="flow-execution-panel__btn flow-execution-panel__btn--cancel"
        @click="handleCancelFlow"
      ) Cancel
      button(
        v-if="activeFlowExecution.status === FlowStatus.RecoveryRequired"
        class="flow-execution-panel__btn flow-execution-panel__btn--history"
        :disabled="recoveryDecisionPending"
        @click="emit('resumeRecovery')"
        title="Resume only if the persisted external outcome proves this checkpoint is safe"
      ) {{ recoveryDecisionPending ? 'Working…' : 'Resume safely' }}
      button(
        v-if="activeFlowExecution.status === FlowStatus.RecoveryRequired"
        class="flow-execution-panel__btn flow-execution-panel__btn--cancel"
        :disabled="recoveryDecisionPending"
        @click="emit('failRecovery')"
        title="Fail this interrupted run without repeating its side effects"
      ) Fail safely
      button(
        class="flow-execution-panel__close"
        @click="handleClosePanel"
        title="Close status panel"
      )
        MdiClose(class="h-4 w-4" aria-hidden="true")

  div(v-if="flowProgress > 0" class="flow-execution-panel__progress")
    div(
      class="flow-execution-panel__progress-fill"
      :style="{ width: flowProgress + '%' }"
    )
  div(v-if="recoveryDecisionError" role="alert" class="border-t border-red-500/30 bg-red-500/10 px-4 py-2 text-xs text-red-600 dark:text-red-300") {{ recoveryDecisionError }}
</template>

<script setup lang="ts">
import { computed } from "vue";
import MdiClose from "~icons/mdi/close";
import MdiHistory from "~icons/mdi/history";
import SolarBoltLinear from "~icons/solar/bolt-linear";
import type {
  FlowExecutionState,
} from "../../../types/flow";
import { FlowStatus } from "../../../types/flow";

interface ConcurrentExecutionInfo {
  currentlyRunning: number;
  maxConcurrent: number;
}

interface Props {
  activeFlowExecution: FlowExecutionState | null;
  showFlowStatusPanel: boolean;
  isFlowRunning: boolean;
  flowProgress: number;
  isInspectorOpen?: boolean;
  concurrentExecutionInfo?: ConcurrentExecutionInfo | null;
  recoveryQueueCount?: number;
  recoveryDecisionPending?: boolean;
  recoveryDecisionError?: string;
}

const props = withDefaults(defineProps<Props>(), {
  activeFlowExecution: null,
  showFlowStatusPanel: false,
  isFlowRunning: false,
  flowProgress: 0,
  isInspectorOpen: true,
  concurrentExecutionInfo: null,
  recoveryQueueCount: 0,
  recoveryDecisionPending: false,
  recoveryDecisionError: "",
});

const emit = defineEmits<{
  cancelFlow: [];
  showHistory: [];
  closePanel: [];
  failRecovery: [];
  resumeRecovery: [];
}>();

const getConcurrentExecutionInfo = computed(() => {
  return props.concurrentExecutionInfo;
});

const getPendingNodeCount = () => {
  if (!props.activeFlowExecution?.pendingNodes) return 0;
  return props.activeFlowExecution.pendingNodes.length;
};

const getFlowStatusClasses = () => {
  if (!props.activeFlowExecution?.status) return "text-gray-400";

  switch (props.activeFlowExecution.status) {
    case FlowStatus.Running:
      return "text-orange-600 dark:text-orange-300";
    case FlowStatus.Completed:
      return "text-emerald-600 dark:text-emerald-300";
    case FlowStatus.CompletedWithErrors:
      return "text-amber-600 dark:text-amber-300";
    case FlowStatus.Failed:
      return "text-red-600 dark:text-red-300";
    case FlowStatus.Cancelled:
      return "text-amber-600 dark:text-amber-300";
    case FlowStatus.RecoveryRequired:
      return "text-red-600 dark:text-red-300";
    default:
      return "text-themeText2";
  }
};

const getFlowStatusText = () => {
  if (!props.activeFlowExecution?.status) return "Unknown";

  switch (props.activeFlowExecution.status) {
    case FlowStatus.Running:
      return "Running";
    case FlowStatus.Completed:
      return "Completed";
    case FlowStatus.CompletedWithErrors:
      return "Completed with Errors";
    case FlowStatus.Failed:
      return "Failed";
    case FlowStatus.Cancelled:
      return "Cancelled";
    case FlowStatus.RecoveryRequired:
      return "Recovery required";
    default:
      return "Unknown";
  }
};

const formatExecutionDuration = () => {
  if (!props.activeFlowExecution?.startedAt) return "N/A";

  const startTime = new Date(props.activeFlowExecution.startedAt).getTime();
  const currentTime = new Date().getTime();
  const diffMs = currentTime - startTime;
  const diffSeconds = Math.floor(diffMs / 1000);

  if (diffSeconds < 60) {
    return `${diffSeconds}s`;
  } else {
    const minutes = Math.floor(diffSeconds / 60);
    const seconds = diffSeconds % 60;
    return `${minutes}m ${seconds}s`;
  }
};

const formatRecoveryTimestamp = () => {
  const detectedAt = props.activeFlowExecution?.recoveryDetectedAt;
  if (!detectedAt) return "Interruption time unavailable";
  const timestamp = new Date(detectedAt);
  if (Number.isNaN(timestamp.getTime())) return "Interruption time unavailable";
  return `Detected ${timestamp.toLocaleString()}`;
};

const handleCancelFlow = () => {
  emit("cancelFlow");
};

const handleShowHistory = () => {
  emit("showHistory");
};

const handleClosePanel = () => {
  emit("closePanel");
};
</script>

<script lang="ts">
export default {
  name: "ExecutionPanel",
};
</script>

<style scoped lang="scss">
.flow-execution-panel {
  border: 1px solid hsl(var(--border2) / 0.85);
  border-radius: 0.9rem;
  padding: 0.7rem 0.8rem;
  background: hsl(var(--background) / 0.96);
  backdrop-filter: blur(10px);
  box-shadow:
    0 14px 30px -22px hsl(var(--text) / 0.34),
    inset 0 1px 0 hsl(var(--text) / 0.04);
  transition: right 0.24s ease;
}

.flow-execution-panel--full {
  right: 1.5rem;
}

.flow-execution-panel--with-inspector {
  right: 23.4rem;
}

.flow-execution-panel__row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.75rem;
}

.flow-execution-panel__meta {
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 0.6rem;
}

.flow-execution-panel__badge {
  width: 1.6rem;
  height: 1.6rem;
  border-radius: 0.52rem;
  border: 1px solid hsl(var(--button) / 0.35);
  display: flex;
  align-items: center;
  justify-content: center;
  background: hsl(var(--button) / 0.12);
  color: hsl(var(--button));
  font-size: 0.7rem;
}

.flow-execution-panel__labels {
  min-width: 0;
}

.flow-execution-panel__title {
  margin: 0;
  line-height: 1.1;
  font-size: 0.86rem;
  font-weight: 700;
  color: hsl(var(--text));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.flow-execution-panel__facts {
  margin-top: 0.1rem;
  display: flex;
  align-items: center;
  gap: 0.55rem;
  font-size: 0.7rem;
  color: hsl(var(--text2));
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.flow-execution-panel__actions {
  display: flex;
  align-items: center;
  gap: 0.45rem;
}

.flow-execution-panel__btn,
.flow-execution-panel__close {
  border: 1px solid hsl(var(--border2));
  border-radius: 0.56rem;
  background: hsl(var(--background2));
  color: hsl(var(--text));
  font-size: 0.73rem;
  line-height: 1;
  height: 1.75rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.3rem;
  padding: 0 0.58rem;
  transition: border-color 0.18s ease, background-color 0.18s ease;
}

.flow-execution-panel__btn:hover,
.flow-execution-panel__close:hover {
  border-color: hsl(var(--text2) / 0.65);
  background: hsl(var(--background3));
}

.flow-execution-panel__btn--history {
  border-color: hsl(var(--button) / 0.4);
}

.flow-execution-panel__btn--cancel {
  border-color: rgb(239 68 68 / 0.42);
  color: rgb(220 38 38);
  background: rgb(239 68 68 / 0.1);
}

:global(.theme-default-dark) .flow-execution-panel__btn--cancel,
:global(.theme-glassy-dark) .flow-execution-panel__btn--cancel {
  color: rgb(252 165 165);
}

.flow-execution-panel__close {
  width: 1.75rem;
  padding: 0;
}

.flow-execution-panel__progress {
  margin-top: 0.58rem;
  width: 100%;
  height: 0.36rem;
  border-radius: 999px;
  border: 1px solid hsl(var(--border2) / 0.6);
  background: hsl(var(--background2));
  overflow: hidden;
}

.flow-execution-panel__progress-fill {
  height: 100%;
  border-radius: inherit;
  background: hsl(var(--button));
  transition: width 0.26s ease;
}
</style>
