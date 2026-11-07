<template lang="pug">
    component(
      :is="to ? NuxtLink : 'button'"
      :to="to || undefined"
      type="button"
      @click="handleClick"
      :class="['deck-rail-btn', isActive ? 'deck-rail-btn--active' : 'deck-rail-btn--idle']"
    )
        div(class="flex flex-col items-center justify-center w-full")
            div(class="deck-rail-btn__icon")
              slot(name="icon")
            p(v-if="props.text" class="deck-rail-btn__label") {{ props.text }}

        .active-pointer(v-if="isActive" class="absolute -left-[0.55rem] w-[0.18rem] h-7 rounded-r-md" style="background: hsl(var(--button))")
    
    </template>

<script setup lang="ts">
import { NuxtLink } from "#components";

const props = defineProps({
  text: {
    type: String,
    required: false,
  },
  to: {
    type: String,
    required: false,
  },

  notifications_count: {
    type: Number,
    required: false,
  },
});

const emit = defineEmits(["click"]);

const route = useRoute();
const isActive = computed(() => {
  const path = route.path.split("/");
  return props.to ? route.path === props.to : false;
});

const handleClick = (e: MouseEvent) => {
  if (!props.to) {
    e.preventDefault();
  }
  emit("click", e);
};
</script>

<style lang="scss">
.deck-rail-btn {
  position: relative;
  width: 100%;
  border-radius: 0.6rem;
  min-height: 3.2rem;
  padding: 0.38rem 0.2rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease, color 0.2s ease;
}

.deck-rail-btn__icon {
  width: 1.95rem;
  height: 1.95rem;
  border-radius: 0.56rem;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: background 0.2s ease, color 0.2s ease;
}

.deck-rail-btn__label {
  font-size: 0.6rem;
  margin-top: 0.2rem;
  color: hsl(var(--text3));
}

.deck-rail-btn--idle {
  color: hsl(var(--text3));
}

.deck-rail-btn--idle:hover {
  background: hsl(var(--hover));
}

.deck-rail-btn--idle:hover .deck-rail-btn__icon {
  background: hsl(var(--background3));
}

.deck-rail-btn--active {
  color: hsl(var(--button));
}

.deck-rail-btn--active .deck-rail-btn__icon {
  background: hsl(var(--button) / 0.15);
  color: hsl(var(--button));
}

.deck-rail-btn--active .deck-rail-btn__label {
  color: hsl(var(--button));
  font-weight: 600;
}

.deck-rail-btn {
  .active-pointer {
    animation: slideIn 0.35s forwards;
  }
}
@keyframes slideIn {
  0% {
    transform: translateX(-1rem);
  }
  100% {
    transform: translateX(-0.6rem);
  }
}

.moving-flag-bg {
  animation: colorChange 2s infinite;
}

@keyframes colorChange {
  0% {
    background: hsl(var(--background3));
  }
  10% {
    background: hsl(var(--button));
    color: white;
  }
  30% {
    background: hsl(var(--button));
    color: white;
  }
  40% {
    background: hsl(var(--background3));
  }
  100% {
    background: hsl(var(--background3));
  }
}
</style>
