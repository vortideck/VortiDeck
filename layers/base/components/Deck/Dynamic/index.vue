<template lang="pug">
div(class="w-full max-h-full grid portrait:grid-cols-3 landscape:grid-cols-5 gap-4 px-[3rem] place-items-stretch justify-center	")
    DeckDynamicButton(
    @click="useSendCommand(button.command)"
    v-for="button in buttons"
    :key="button.id"
    :label="button.label"
    :action="button.command"
    )
    //- :layout="button.layout"

</template>

<script setup lang="ts">
import { useSendCommand } from "@base/composables/useSendCommand";

const buttons = ref([
  {
    id: 1,
    label: "backspace",
    command: "backspace",
  },
]); // Store the fetched button configs

const fetchbuttons = () => {};
</script>

<style lang="scss"></style>
