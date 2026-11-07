<template lang="pug">
div.bubble-effect(class="flex flex-col gap-y-3 h-auto aspect-square w-full items-center justify-center  p-3 text-center rounded-full ")
    p(class="text-blue-500 text-xl") We are still cooking this feature.
</template>

<script setup lang="ts"></script>

<style lang="scss" scoped>
.bubble-effect {
  position: relative;
  animation: bubble 2s ease-in-out infinite;
  background: rgba(255, 255, 255, 0.8);
  box-shadow: inset -4px -4px 8px rgba(59, 130, 246, 0.2);
  border-radius: 50%;

  &::before,
  &::after {
    content: "";
    position: absolute;
    border-radius: 50%;
    inset: -1px;
    transform-origin: center;
  }

  &::before {
    animation: float 3s ease-in-out infinite;
  }

  &::after {
    animation: float 3s ease-in-out infinite;
  }

  // Mini bubble
  &::after {
    content: "";
    position: absolute;
    width: 20px;
    height: 20px;
    background: rgba(255, 255, 255, 0.8);
    border-radius: 50%;
    top: -10px;
    right: 30%;
    animation: float 3s ease-in-out infinite;
    box-shadow: inset -2px -2px 4px rgba(59, 130, 246, 0.2);
  }
}

@keyframes bubble-pulse {
  0% {
    transform: scale(1);
    opacity: 0.5;
  }
  50% {
    transform: scale(1.1);
    opacity: 0;
  }
  100% {
    transform: scale(1);
    opacity: 0;
  }
}

@keyframes bubble {
  0%,
  100% {
    transform: scale(1);
  }
  50% {
    transform: scale(0.98) skew(-2deg);
  }
}

@keyframes float {
  0%,
  100% {
    transform: translateY(0);
  }
  50% {
    transform: translateY(-10px);
  }
}
</style>
