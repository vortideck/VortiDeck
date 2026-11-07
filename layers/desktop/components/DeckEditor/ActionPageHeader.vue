<template lang="pug">
header(class="action-page-header")
  div(class="action-page-header__icon" aria-hidden="true")
    slot(name="icon")
  div(class="min-w-0")
    p(class="action-page-header__eyebrow") {{ eyebrow }}
    h1(class="action-page-header__title") {{ title }}
    p(class="action-page-header__description") {{ description }}
</template>

<script setup lang="ts">
defineProps<{
  eyebrow: string;
  title: string;
  description: string;
}>();
</script>

<style scoped lang="scss">
.action-page-header {
  display: flex;
  align-items: flex-start;
  gap: 0.9rem;
  padding: 1.25rem 1.35rem;
  border: 1px solid hsl(var(--border2) / 0.78);
  border-radius: 0.9rem;
  background: hsl(var(--background2));
  box-shadow: 0 0.8rem 2.4rem hsl(var(--text) / 0.05);
}

.action-page-header__icon {
  width: 2.7rem;
  height: 2.7rem;
  flex: 0 0 2.7rem;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 1px solid hsl(var(--border2));
  border-radius: 0.75rem;
  background: hsl(var(--background3));
  color: hsl(var(--text));
}

.action-page-header__eyebrow {
  color: hsl(var(--button));
  font-size: 0.68rem;
  font-weight: 750;
  letter-spacing: 0.08em;
  line-height: 1;
  text-transform: uppercase;
}

.action-page-header__title {
  margin-top: 0.35rem;
  color: hsl(var(--text));
  font-size: 1.25rem;
  font-weight: 750;
  letter-spacing: -0.02em;
  line-height: 1.2;
}

.action-page-header__description {
  margin-top: 0.35rem;
  color: hsl(var(--text2));
  font-size: 0.78rem;
  line-height: 1.45;
}
</style>
