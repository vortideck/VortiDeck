<template>
  <div class="duration-picker">
    <div class="duration-inputs">
      <div class="duration-field">
        <label>Hours</label>
        <div class="input-with-buttons">
          <input
            type="number"
            :value="hours"
            @input="updateHours"
            @click.stop
            @mousedown.stop
            min="0"
            max="999"
            :disabled="noTimeout"
            class="duration-input"
          />
          <div class="spinner-buttons">
            <button @click.stop="incrementHours" @mousedown.stop :disabled="noTimeout" class="spinner-btn">▲</button>
            <button @click.stop="decrementHours" @mousedown.stop :disabled="noTimeout" class="spinner-btn">▼</button>
          </div>
        </div>
      </div>

      <div class="duration-separator">:</div>

      <div class="duration-field">
        <label>Minutes</label>
        <div class="input-with-buttons">
          <input
            type="number"
            :value="minutes"
            @input="updateMinutes"
            @click.stop
            @mousedown.stop
            min="0"
            max="59"
            :disabled="noTimeout"
            class="duration-input"
          />
          <div class="spinner-buttons">
            <button @click.stop="incrementMinutes" @mousedown.stop :disabled="noTimeout" class="spinner-btn">▲</button>
            <button @click.stop="decrementMinutes" @mousedown.stop :disabled="noTimeout" class="spinner-btn">▼</button>
          </div>
        </div>
      </div>

      <div class="duration-separator">:</div>

      <div class="duration-field">
        <label>Seconds</label>
        <div class="input-with-buttons">
          <input
            type="number"
            :value="seconds"
            @input="updateSeconds"
            @click.stop
            @mousedown.stop
            min="0"
            max="59"
            :disabled="noTimeout"
            class="duration-input"
          />
          <div class="spinner-buttons">
            <button @click.stop="incrementSeconds" @mousedown.stop :disabled="noTimeout" class="spinner-btn">▲</button>
            <button @click.stop="decrementSeconds" @mousedown.stop :disabled="noTimeout" class="spinner-btn">▼</button>
          </div>
        </div>
      </div>
    </div>

    <label class="no-timeout-checkbox">
      <input
        type="checkbox"
        :checked="noTimeout"
        @change="toggleNoTimeout"
        @click.stop
        @mousedown.stop
      />
      <span>No timeout</span>
    </label>
  </div>
</template>

<script setup lang="ts">
import { computed } from "vue";

interface Props {
  modelValue: number; // Total seconds
}

const props = defineProps<Props>();

const emit = defineEmits<{
  "update:modelValue": [value: number];
}>();

// Check if no timeout (0 seconds)
const noTimeout = computed(() => props.modelValue === 0);

// Convert total seconds to hours, minutes, seconds
const hours = computed(() => {
  if (noTimeout.value) return 0;
  return Math.floor(props.modelValue / 3600);
});

const minutes = computed(() => {
  if (noTimeout.value) return 0;
  return Math.floor((props.modelValue % 3600) / 60);
});

const seconds = computed(() => {
  if (noTimeout.value) return 0;
  return props.modelValue % 60;
});

const updateHours = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const h = Math.max(0, Math.min(999, parseInt(target.value) || 0));
  const totalSeconds = h * 3600 + minutes.value * 60 + seconds.value;
  emit("update:modelValue", totalSeconds);
};

const updateMinutes = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const m = Math.max(0, Math.min(59, parseInt(target.value) || 0));
  const totalSeconds = hours.value * 3600 + m * 60 + seconds.value;
  emit("update:modelValue", totalSeconds);
  // Force input to show clamped value
  target.value = m.toString();
};

const updateSeconds = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const s = Math.max(0, Math.min(59, parseInt(target.value) || 0));
  const totalSeconds = hours.value * 3600 + minutes.value * 60 + s;
  emit("update:modelValue", totalSeconds);
  // Force input to show clamped value
  target.value = s.toString();
};

const incrementHours = () => {
  const h = Math.min(999, hours.value + 1);
  const totalSeconds = h * 3600 + minutes.value * 60 + seconds.value;
  emit("update:modelValue", totalSeconds);
};

const decrementHours = () => {
  const h = Math.max(0, hours.value - 1);
  const totalSeconds = h * 3600 + minutes.value * 60 + seconds.value;
  emit("update:modelValue", totalSeconds);
};

const incrementMinutes = () => {
  const m = Math.min(59, minutes.value + 1);
  const totalSeconds = hours.value * 3600 + m * 60 + seconds.value;
  emit("update:modelValue", totalSeconds);
};

const decrementMinutes = () => {
  const m = Math.max(0, minutes.value - 1);
  const totalSeconds = hours.value * 3600 + m * 60 + seconds.value;
  emit("update:modelValue", totalSeconds);
};

const incrementSeconds = () => {
  const s = Math.min(59, seconds.value + 1);
  const totalSeconds = hours.value * 3600 + minutes.value * 60 + s;
  emit("update:modelValue", totalSeconds);
};

const decrementSeconds = () => {
  const s = Math.max(0, seconds.value - 1);
  const totalSeconds = hours.value * 3600 + minutes.value * 60 + s;
  emit("update:modelValue", totalSeconds);
};

const toggleNoTimeout = (event: Event) => {
  const target = event.target as HTMLInputElement;
  if (target.checked) {
    emit("update:modelValue", 0); // 0 = no timeout
  } else {
    emit("update:modelValue", 30); // Default to 30 seconds
  }
};
</script>

<style scoped>
.duration-picker {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 100%;
}

.duration-inputs {
  display: flex;
  align-items: center;
  justify-content: start;
  gap: 0.5rem;
  width: 100%;
}

.duration-field {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.duration-field label {
  font-size: 9px;
  color: hsl(var(--text3));
  margin-bottom: 1px;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  white-space: nowrap;
}

.input-with-buttons {
  position: relative;
  display: flex;
  align-items: center;
}

.duration-input {
  width: 3.5rem;
  padding: 4px 6px;
  padding-right: 20px;
  text-align: center;
  font-size: 12px;
  background: hsl(var(--background));
  color: hsl(var(--text));
  border: 1px solid hsl(var(--border));
  border-radius: 6px;
  transition: all 0.2s;
  -moz-appearance: textfield;
  appearance: textfield;
}

/* Hide native spinner buttons */
.duration-input::-webkit-outer-spin-button,
.duration-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.spinner-buttons {
  position: absolute;
  right: 2px;
  top: 50%;
  transform: translateY(-50%);
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.spinner-btn {
  width: 16px;
  height: 11px;
  padding: 0;
  background: hsl(var(--background3));
  color: hsl(var(--text2));
  border: none;
  border-radius: 2px;
  font-size: 8px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
}

.spinner-btn:hover:not(:disabled) {
  background: hsl(var(--hover));
  color: hsl(var(--text));
}

.spinner-btn:active:not(:disabled) {
  background: hsl(var(--background3));
}

.spinner-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.duration-input:focus {
  outline: none;
  border-color: hsl(var(--primary));
  box-shadow: 0 0 0 1px hsl(var(--primary) / 0.3);
}

.duration-input:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  background: hsl(var(--background3));
}

.duration-separator {
  font-size: 14px;
  font-weight: bold;
  color: hsl(var(--text3));
  margin-top: 10px;
  user-select: none;
}

.no-timeout-checkbox {
  display: flex;
  align-items: center;
  gap: 0.4rem;
  font-size: 11px;
  color: hsl(var(--text2));
  cursor: pointer;
  user-select: none;
  padding: 2px 0;
}

.no-timeout-checkbox input[type="checkbox"] {
  width: 14px;
  height: 14px;
  cursor: pointer;
  accent-color: hsl(var(--primary));
}

.no-timeout-checkbox:hover {
  color: hsl(var(--text));
}
</style>
