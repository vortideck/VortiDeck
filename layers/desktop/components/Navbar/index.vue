<template lang="pug">
div(
  :class="[rootClasses, route.path !== '/' && !props.compactLeftPadding ? 'pl-16' : 'pl-5', { 'navbar-shell--bottom-divider': props.bottomDivider }]"
  class="navbar-shell box-border flex items-center w-full z-50 select-none bg-themeBackground pr-4"
)
  div(class="mr-auto select-none py-1")
    div(v-if="props.title" class="flex flex-col")
      h1(class="text-xl font-bold text-themeText leading-tight") {{ props.title }}
      p(v-if="props.subtitle" class="text-xs text-themeText2 leading-tight mt-0.5") {{ props.subtitle }}

  NavbarTopbarActions(:showMeta="!props.minified")
</template>

<script setup lang="ts">
const route = useRoute();
const props = withDefaults(
  defineProps<{
    title?: string;
    subtitle?: string;
    minified?: boolean;
    compactLeftPadding?: boolean;
    bottomDivider?: boolean;
  }>(),
  {
    title: "",
    subtitle: "",
    minified: false,
    compactLeftPadding: false,
    bottomDivider: false,
  }
);

const rootClasses = computed(() => [
  props.minified ? "relative" : "fixed",
]);
</script>

<style lang="scss" scoped>
.navbar-shell {
  height: 4.2rem;
}

.navbar-shell--bottom-divider {
  box-shadow: inset 0 -1px 0 hsl(var(--border2) / 0.72);
}
</style>
