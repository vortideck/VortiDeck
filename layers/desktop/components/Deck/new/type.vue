<template lang="pug">
div(@click="" class="option flex flex-col gap-y-6 h-[8rem] w-[8rem] group relative")
    div(class="hover:cursor-pointer flex w-full aspect-square items-center justify-center hover:transition-all bg-themeBackground2  relative group-hover:border-[3px] group-hover:border-[#56b6ff] rounded-md overflow-clip ")
        div(class="option group-hover:visible invisible w-full h-full bg-[rgba(86,182,255,0.2)] absolute top-0 left-0 ")
        slot(name="icon")
    p(class="hover:cursor-pointer group-hover:text-[rgb(86,182,255)]") {{description}}
    p(class="invisible w-[14rem] group-hover:visible absolute -bottom-[9rem] left-1/2 translate-x-[-50%]") {{hint}}
</template>

<script setup lang="ts">
const props = defineProps({
  hint: {
    type: String,
    required: true,
  },
  description: {
    type: String,
    required: true,
  },
});
</script>

<style lang="scss" scoped>
@keyframes fade-border {
  0% {
    background-color: transparent;
    border-color: transparent;
    color: black;
  }
  50% {
    background-color: rgb(222, 222, 255);
    border-color: rgb(37, 157, 255);
    color: blue;
  }
  100% {
    background-color: rgb(222, 222, 255);
    border-color: rgb(37, 157, 255);
    color: blue;
  }
}
</style>
