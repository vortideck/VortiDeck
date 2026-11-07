<template lang="pug">
div(class="deck-menu-tools flex flex-col gap-y-3")
    p(class="text-xs font-semibold tracking-[0.12em] uppercase text-themeText2 -mt-1") {{tip}}
    div(v-if="searchPlaceholder" class="search-shell flex w-full py-2 px-3 gap-x-2.5 items-center rounded-lg border border-themeBorder2 bg-themeBackground")
        PixelarticonsSearch(class="w-4 h-4 text-themeText2")
        input(:placeholder="searchPlaceholder" class="w-full focus:outline-none bg-transparent text-sm text-themeText placeholder:text-themeText2" v-model="searchPrompt" @input="emitSearch")
    slot(name="button")
</template>

<script setup lang="ts">
import PixelarticonsSearch from "~icons/pixelarticons/search?width=24px&height=24px";

const props = defineProps({
  tip: {
    type: String,
    required: true,
  },
  searchPlaceholder: {
    type: String,
    required: false,
    default: null,
  },
});

const emit = defineEmits(["search"]);
const searchPrompt = ref("");
const emitSearch = () => {
  emit("search", searchPrompt.value);
};
</script>

<style lang="scss"></style>
