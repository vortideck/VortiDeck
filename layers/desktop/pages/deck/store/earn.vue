<template lang="pug">
div(class="deck-store-page h-full w-full p-6")
  div(class="store-placeholder h-full flex items-center justify-center")
    div(class="text-center max-w-xl")
      p(class="store-kicker") Create & Earn
      h2(class="store-title") Creator Marketplace Is In Progress
      p(class="store-subtitle") This section will let you publish themes, plugins, and decks while keeping the same new interface language.
</template>

<script setup lang="ts">
definePageMeta({
  title: "Create & Earn",
  description: "create custom themes, decks, scripts and list them on marketplace!",
});
</script>

<style lang="scss" scoped>
.deck-store-page {
  --background: 220 34% 98%;
  --text: 220 26% 20%;
  --text2: 220 14% 42%;
  --border: 220 22% 86%;
  background: hsl(var(--background));
}

.store-placeholder {
  border: 1px solid hsl(var(--border));
  border-radius: 1rem;
  background: linear-gradient(180deg, #fbfcff 0%, #f3f6fb 100%);
}

.store-kicker {
  font-size: 0.72rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #6f7a91;
  font-weight: 700;
}

.store-title {
  margin-top: 0.45rem;
  font-size: 1.6rem;
  font-weight: 800;
  color: hsl(var(--text));
}

.store-subtitle {
  margin-top: 0.7rem;
  font-size: 0.93rem;
  color: hsl(var(--text2));
}
</style>
