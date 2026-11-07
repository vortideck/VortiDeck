<script setup lang="ts">
// Legacy overlay route redirect -> redirect to modern streaming overlays page
definePageMeta({
  redirect: "/streaming/overlays",
});
await navigateTo("/streaming/overlays", { replace: true });
</script>

<template>
  <div />
</template>
