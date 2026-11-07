<template lang="pug">

NuxtLink( class=" group hover:cursor-pointer  font-semibold text-lg    rounded-md py-3 px-3 md:px-4 flex items-center hover:shadow-[0_2px_30px_-15px_rgba(0,0,0,0.9)] justify-center ") {{text}}
    slot(name="icon")

</template>

<script setup lang="ts">
defineProps({
  text: String,
});
</script>

<style lang="scss"></style>
