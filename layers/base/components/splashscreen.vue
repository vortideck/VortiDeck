<template lang="pug">
div(class="splash-root")
  div(class="splash-halo")
  div(class="splash-aura splash-aura--purple")
  div(class="splash-aura splash-aura--teal")
  div(class="splash-particle splash-particle--a")
  div(class="splash-particle splash-particle--b")
  div(class="splash-particle splash-particle--c")

  div(class="splash-content")
    div(class="splash-brand")
      div(class="splash-logo-wrap")
        div(class="splash-logo-glow")
        div(class="splash-logo-card")
          img(src="/logo.png" class="splash-logo" alt="VortiDeck Logo")
        div(class="splash-logo-orbit")

      h1(class="splash-title") {{ $t("pages.splashscreen.title") }}
      p(class="splash-subtitle") {{ $t("pages.splashscreen.description") }}

    div(class="splash-loader" aria-hidden="true")
      div(class="splash-loader-ring")

    div(class="splash-progress-group")
      div(class="splash-progress")
        div(class="splash-progress-row")
          span(class="splash-progress-label") {{ primaryStageLabel }}
          span(class="splash-progress-value") {{ overallProgress }}%

        div(class="splash-progress-track")
          div(class="splash-progress-fill" :style="{ width: `${overallProgress}%` }")
            div(class="splash-progress-shine")

        div(class="splash-progress-meta")
          span(class="splash-version") v{{ appVersion }}
          span(class="splash-ready")
            span(class="splash-ready-dot" :class="overallProgress >= 99 ? 'splash-ready-dot--on' : ''")
            | {{ readinessText }}

      div(class="splash-progress splash-progress--secondary")
        div(class="splash-progress-row")
          span(class="splash-progress-label") {{ syncStageLabel }}
          span(class="splash-progress-value") {{ syncProgress }}%

        div(class="splash-progress-track")
          div(class="splash-progress-fill splash-progress-fill--secondary" :style="{ width: `${syncProgress}%` }")
            div(class="splash-progress-shine")

  p(class="splash-copy") © {{ copyrightLabel }} VortiDeck
</template>

<script setup lang="ts">
import { computed, onMounted, ref } from "vue";
import { getVersion } from "@tauri-apps/api/app";

type SplashTaskStatus = "pending" | "running" | "completed" | "failed";

interface SplashTask {
  id: string;
  name: string;
  status: SplashTaskStatus;
  progress?: number;
  message?: string;
}

const props = defineProps<{
  tasks: Record<string, SplashTask> | SplashTask[];
}>();

const appVersion = ref("...");
const startYear = 2025;

const normalizedTasks = computed<SplashTask[]>(() => {
  if (!props.tasks) return [];
  return Array.isArray(props.tasks) ? props.tasks : Object.values(props.tasks);
});

const completedCount = computed(() => {
  return normalizedTasks.value.filter((task) => task.status === "completed").length;
});

const clampPercent = (value: number): number => {
  if (!Number.isFinite(value)) return 0;
  return Math.max(0, Math.min(100, value));
};

const getTaskProgress = (task: SplashTask): number => {
  if (task.status === "completed") return 100;

  if (typeof task.progress === "number") {
    return clampPercent(task.progress);
  }

  const totalTasks = normalizedTasks.value.length;
  const completionRatio = totalTasks > 0 ? completedCount.value / totalTasks : 0;

  if (task.status === "running") {
    return Math.max(18, Math.floor(completionRatio * 72));
  }

  if (task.status === "failed") {
    return Math.max(26, Math.floor(completionRatio * 42));
  }

  return Math.max(8, Math.floor(completionRatio * 26));
};

const overallProgress = computed(() => {
  if (normalizedTasks.value.length === 0) return 12;
  const sum = normalizedTasks.value.reduce((acc, task) => acc + getTaskProgress(task), 0);
  return Math.round(sum / normalizedTasks.value.length);
});

const syncProgress = computed(() => {
  const total = normalizedTasks.value.length;
  if (total === 0) return 0;
  return Math.round((completedCount.value / total) * 100);
});

const primaryTask = computed(() => {
  return (
    normalizedTasks.value.find((task) => task.status === "running") ||
    normalizedTasks.value.find((task) => task.status === "pending") ||
    normalizedTasks.value[normalizedTasks.value.length - 1]
  );
});

const primaryStageLabel = computed(() => {
  return primaryTask.value?.name || "Initializing Core";
});

const syncStageLabel = computed(() => {
  const hasFailedTask = normalizedTasks.value.some((task) => task.status === "failed");
  const total = normalizedTasks.value.length;

  if (total === 0) return "Syncing Modules";
  if (hasFailedTask) return "Recovering Services";
  if (completedCount.value === total) return "Sync Complete";

  return "Syncing Cloud Flows";
});

const readinessText = computed(() => {
  if (overallProgress.value >= 99) return "Ready: Your workflow, your rules";
  return "Preparing workspace";
});

const copyrightLabel = computed(() => {
  const currentYear = new Date().getFullYear();
  if (currentYear <= startYear) return `${startYear}`;
  return `${startYear}-${currentYear}`;
});

onMounted(async () => {
  try {
    appVersion.value = await getVersion();
  } catch {
    appVersion.value = "2.x";
  }
});
</script>

<style lang="scss" scoped>
.splash-root {
  position: relative;
  width: 100%;
  min-height: 100vh;
  overflow: hidden;
  background: #f4f6fb;
  color: #233047;
  user-select: none;
}

.splash-halo {
  position: absolute;
  top: 50%;
  left: 50%;
  width: min(92vw, 58rem);
  height: min(92vw, 58rem);
  transform: translate(-50%, -50%);
  border-radius: 999px;
  background: radial-gradient(circle, rgba(146, 156, 255, 0.2) 0%, rgba(146, 156, 255, 0.08) 46%, rgba(146, 156, 255, 0) 76%);
  filter: blur(2px);
  pointer-events: none;
}

.splash-aura {
  position: absolute;
  border-radius: 999px;
  filter: blur(72px);
  pointer-events: none;
}

.splash-aura--purple {
  width: 30rem;
  height: 30rem;
  top: 48%;
  left: 50%;
  transform: translate(-48%, -50%);
  background: rgba(157, 126, 255, 0.15);
  animation: auraDriftA 11s ease-in-out infinite;
}

.splash-aura--teal {
  width: 22rem;
  height: 22rem;
  top: 48%;
  left: 49%;
  transform: translate(-54%, -45%);
  background: rgba(32, 205, 192, 0.12);
  animation: auraDriftB 12.5s ease-in-out infinite;
}

.splash-particle {
  position: absolute;
  width: 0.25rem;
  height: 0.25rem;
  border-radius: 50%;
  pointer-events: none;
}

.splash-particle--a {
  left: 25%;
  top: 28%;
  background: rgba(32, 205, 192, 0.62);
  animation: particleFloatA 5.6s ease-in-out infinite;
}

.splash-particle--b {
  right: 25%;
  bottom: 34%;
  background: rgba(155, 121, 255, 0.55);
  animation: particleFloatB 6.1s ease-in-out infinite;
}

.splash-particle--c {
  right: 35%;
  top: 36%;
  width: 0.15rem;
  height: 0.15rem;
  background: rgba(255, 255, 255, 0.78);
  animation: particleFloatC 4.8s ease-in-out infinite;
}

.splash-content {
  position: relative;
  z-index: 2;
  min-height: 100vh;
  width: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1.15rem;
  padding: 1.75rem 1.25rem 3.2rem;
}

.splash-brand {
  width: min(88vw, 26rem);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.74rem;
}

.splash-logo-wrap {
  position: relative;
  width: 5.25rem;
  height: 5.25rem;
  display: grid;
  place-items: center;
}

.splash-logo-glow {
  position: absolute;
  inset: -0.52rem;
  border-radius: 1.2rem;
  background: linear-gradient(140deg, rgba(32, 205, 192, 0.27), rgba(155, 121, 255, 0.24));
  filter: blur(16px);
  animation: glowBreath 3.2s ease-in-out infinite;
}

.splash-logo-card {
  position: relative;
  width: 100%;
  height: 100%;
  border-radius: 1rem;
  border: 1px solid rgba(163, 178, 210, 0.45);
  background: rgba(246, 249, 255, 0.9);
  backdrop-filter: blur(5px);
  display: flex;
  align-items: center;
  justify-content: center;
  box-shadow: 0 12px 30px rgba(65, 89, 141, 0.16);
}

.splash-logo {
  width: 2.72rem;
  height: 2.72rem;
  object-fit: contain;
}

.splash-logo-orbit {
  position: absolute;
  inset: -0.5rem;
  border-radius: 1.35rem;
  border: 1px solid transparent;
  border-left-color: rgba(32, 205, 192, 0.5);
  border-bottom-color: rgba(155, 121, 255, 0.38);
  animation: orbitSpin 7.5s linear infinite;
  opacity: 0.8;
}

.splash-title {
  margin-top: 0.4rem;
  font-size: clamp(2.55rem, 5.1vw, 3.35rem);
  line-height: 1.03;
  font-weight: 780;
  letter-spacing: -0.018em;
  color: #1f2b41;
}

.splash-subtitle {
  margin-top: -0.24rem;
  font-size: clamp(1.08rem, 1.95vw, 1.45rem);
  color: #5f6f90;
  font-weight: 400;
}

.splash-loader {
  margin-top: 0.62rem;
  margin-bottom: 0.36rem;
}

.splash-loader-ring {
  width: 2.3rem;
  height: 2.3rem;
  border-radius: 50%;
  border: 2px solid rgba(32, 205, 192, 0.16);
  border-top-color: rgba(32, 205, 192, 0.92);
  border-right-color: rgba(32, 205, 192, 0.42);
  position: relative;
  animation: spinnerSpin 1.05s linear infinite;
}

.splash-progress-group {
  width: min(84vw, 22.5rem);
  display: flex;
  flex-direction: column;
  gap: 0.66rem;
}

.splash-progress {
  width: 100%;
}

.splash-progress--secondary {
  opacity: 0.9;
}

.splash-progress-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.8rem;
}

.splash-progress-label {
  font-size: 0.77rem;
  font-weight: 700;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: #6f7f9d;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.splash-progress-value {
  font-size: 0.78rem;
  color: #6b7b98;
  font-weight: 700;
  flex-shrink: 0;
}

.splash-progress-track {
  margin-top: 0.32rem;
  height: 0.28rem;
  border-radius: 999px;
  background: rgba(154, 170, 204, 0.74);
  overflow: hidden;
}

.splash-progress-fill {
  height: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, #58cfdf 0%, #8f74ed 100%);
  box-shadow: 0 0 9px rgba(88, 207, 223, 0.25);
  position: relative;
  transition: width 0.3s ease;
}

.splash-progress-fill--secondary {
  background: linear-gradient(90deg, rgba(88, 207, 223, 0.86) 0%, rgba(88, 207, 223, 0.58) 100%);
  box-shadow: 0 0 6px rgba(88, 207, 223, 0.2);
}

.splash-progress-shine {
  position: absolute;
  inset: 0;
  width: 34%;
  background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.58), transparent);
  animation: shimmer 2s linear infinite;
}

.splash-progress-meta {
  margin-top: 0.4rem;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.8rem;
}

.splash-version {
  font-size: 0.7rem;
  color: #7687a8;
  font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
}

.splash-ready {
  display: inline-flex;
  align-items: center;
  gap: 0.28rem;
  font-size: 0.7rem;
  color: #697b98;
  white-space: nowrap;
}

.splash-ready-dot {
  width: 0.24rem;
  height: 0.24rem;
  border-radius: 999px;
  background: #35c9b7;
  opacity: 0.7;
}

.splash-ready-dot--on {
  background: #12b981;
  opacity: 1;
}

.splash-copy {
  position: absolute;
  left: 50%;
  bottom: 1rem;
  transform: translateX(-50%);
  font-size: 0.62rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: #7687a8;
}

@keyframes auraDriftA {
  0%,
  100% { transform: translate(-48%, -50%); }
  50% { transform: translate(-51%, -47%); }
}

@keyframes auraDriftB {
  0%,
  100% { transform: translate(-54%, -45%); }
  50% { transform: translate(-51%, -48%); }
}

@keyframes particleFloatA {
  0%,
  100% { transform: translateY(0) scale(1); opacity: 0.35; }
  50% { transform: translateY(-0.35rem) scale(1.5); opacity: 0.95; }
}

@keyframes particleFloatB {
  0%,
  100% { transform: translateY(0) scale(1); opacity: 0.3; }
  50% { transform: translateY(0.3rem) scale(1.4); opacity: 0.82; }
}

@keyframes particleFloatC {
  0%,
  100% { transform: scale(1); opacity: 0.2; }
  50% { transform: scale(1.8); opacity: 0.64; }
}

@keyframes glowBreath {
  0%,
  100% { opacity: 0.75; transform: scale(0.98); }
  50% { opacity: 1; transform: scale(1.02); }
}

@keyframes orbitSpin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes spinnerSpin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

@keyframes shimmer {
  from { transform: translateX(-120%); }
  to { transform: translateX(320%); }
}

@media (max-width: 640px) {
  .splash-progress-group {
    width: min(89vw, 20rem);
  }

  .splash-title {
    font-size: clamp(2.18rem, 9.4vw, 2.82rem);
  }

  .splash-subtitle {
    font-size: clamp(1rem, 4.3vw, 1.22rem);
  }

  .splash-progress-label {
    font-size: 0.72rem;
  }

  .splash-progress-value {
    font-size: 0.72rem;
  }
}
</style>
