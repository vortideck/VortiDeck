<template lang="pug">
div(
  class="system-widget flex flex-col justify-between h-full w-full p-2.5 rounded-xl transition-all duration-200 select-none relative overflow-hidden bg-slate-900 border border-white/10 shadow-md"
)
  //- Header
  div(class="flex items-center justify-between")
    div(class="flex items-center gap-1.5")
      span(class="w-2 h-2 rounded-full bg-emerald-400 animate-ping")
      span(class="text-[11px] font-bold text-white/90") System Hardware
    span(class="text-[10px] font-mono text-emerald-400") {{ Math.round(fps) }} FPS

  //- Metrics Row (CPU, GPU, RAM)
  div(class="grid grid-cols-3 gap-2 my-auto")
    //- CPU Metric
    div(class="metric-box flex flex-col p-1.5 rounded-lg bg-slate-800/80 border border-white/5")
      div(class="flex items-center justify-between text-[9px] text-white/60 mb-0.5")
        span CPU
        span(class="font-mono text-blue-400 font-bold") {{ Math.round(cpuUsage) }}%
      div(class="w-full h-1 bg-slate-700 rounded-full overflow-hidden")
        div(
          class="h-full bg-blue-400 rounded-full transition-all duration-500"
          :style="{ width: `${cpuUsage}%` }"
        )

    //- GPU Metric
    div(class="metric-box flex flex-col p-1.5 rounded-lg bg-slate-800/80 border border-white/5")
      div(class="flex items-center justify-between text-[9px] text-white/60 mb-0.5")
        span GPU
        span(class="font-mono text-purple-400 font-bold") {{ Math.round(gpuTemp) }}°C
      div(class="w-full h-1 bg-slate-700 rounded-full overflow-hidden")
        div(
          class="h-full bg-purple-400 rounded-full transition-all duration-500"
          :style="{ width: `${(gpuTemp / 90) * 100}%` }"
        )

    //- RAM Metric
    div(class="metric-box flex flex-col p-1.5 rounded-lg bg-slate-800/80 border border-white/5")
      div(class="flex items-center justify-between text-[9px] text-white/60 mb-0.5")
        span RAM
        span(class="font-mono text-amber-400 font-bold") {{ ramUsed.toFixed(1) }}G
      div(class="w-full h-1 bg-slate-700 rounded-full overflow-hidden")
        div(
          class="h-full bg-amber-400 rounded-full transition-all duration-500"
          :style="{ width: `${(ramUsed / 32) * 100}%` }"
        )

  //- Mini Sparkline Waveform
  div(class="flex items-end gap-1 h-3 w-full opacity-60")
    div(
      v-for="(val, idx) in history"
      :key="idx"
      class="flex-1 bg-blue-400/80 rounded-t-sm transition-all duration-300"
      :style="{ height: `${val}%` }"
    )
</template>

<script setup lang="ts">
import type { DeckWidgetItem } from '../types';

const props = defineProps<{
  item: DeckWidgetItem;
  isInteractive?: boolean;
}>();

const cpuUsage = ref(38);
const gpuTemp = ref(54);
const ramUsed = ref(14.2);
const fps = ref(144);
const history = ref([30, 45, 38, 60, 50, 40, 55, 38, 42, 48, 38]);

let timer: any = null;

onMounted(() => {
  timer = setInterval(() => {
    // Generate realistic fluctuating metrics
    const nextCpu = Math.max(15, Math.min(92, cpuUsage.value + (Math.random() * 12 - 6)));
    cpuUsage.value = nextCpu;
    gpuTemp.value = Math.max(42, Math.min(78, gpuTemp.value + (Math.random() * 2 - 1)));
    ramUsed.value = Math.max(10, Math.min(28, ramUsed.value + (Math.random() * 0.4 - 0.2)));
    fps.value = Math.max(138, Math.min(144, 144 - Math.random() * 4));

    history.value.shift();
    history.value.push(nextCpu);
  }, 2000);
});

onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});
</script>
