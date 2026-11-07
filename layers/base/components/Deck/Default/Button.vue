<template lang="pug">
div( class="border-[1px] rounded-md border-gray-400 aspect-square w-full button ")
</template>

<script setup lang="ts"></script>

<style lang="scss" scoped>
.button {
  transition: all 0.2s ease-in-out;
}

.button:active {
  transform: scale(1.1);
  transition: 0.1s;
}
</style>
