<template lang="pug">
SettingsModal(:is-page="true")
</template>

<script setup lang="ts">
import SettingsModal from "../components/Modal/SettingsModal.vue";

definePageMeta({
  title: "Settings",
  layoutScroll: false,
});
</script>
