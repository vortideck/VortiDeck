<template lang="pug">
nav.pagination(aria-label="Page navigation")
    ul.flex.items-center.justify-center.gap-2
        li
            button(@click="$emit('onPageChange', 1)" class="flex items-center justify-center px-4 py-2 rounded-lg bg-gray-800 text-white hover:bg-gray-700 transition-all duration-300 shadow-md")
                | First
        li(v-if="page - 1 < pages && page - 2 == pages - 1 || page >= 3 && page === pages")
            button(@click="$emit('onPageChange', page - 2)" class="flex items-center justify-center w-10 h-10 rounded-full bg-gray-800 text-white hover:bg-gray-700 transition-all duration-300")
                | {{ page - 2 }}
        li(v-if="page - 1 > 0" rel="prev")
            button(@click="$emit('onPageChange', page - 1)" class="flex items-center justify-center w-10 h-10 rounded-full bg-gray-800 text-white hover:bg-gray-700 transition-all duration-300")
                | {{ page - 1 }}
        li(aria-current="page")
            span(class="flex items-center justify-center w-10 h-10 rounded-full bg-blue-600 text-white font-medium shadow-md")
                | {{ page }}
        li(v-if="page + 1 <= pages" rel="next")
            button(@click="$emit('onPageChange', page + 1)" class="flex items-center justify-center w-10 h-10 rounded-full bg-gray-800 text-white hover:bg-gray-700 transition-all duration-300")
                | {{ page + 1 }}
        li(v-if="page - 1 === 0 && page + 2 <= pages")
            button(@click="$emit('onPageChange', page + 2)" class="flex items-center justify-center w-10 h-10 rounded-full bg-gray-800 text-white hover:bg-gray-700 transition-all duration-300")
                | {{ page + 2 }}
        li
            button(@click="$emit('onPageChange', pages)" class="flex items-center justify-center px-4 py-2 rounded-lg bg-gray-800 text-white hover:bg-gray-700 transition-all duration-300 shadow-md")
                | Last
</template>

<script setup lang="ts">
import { ref } from "vue";
import { useRoute } from "vue-router";

defineProps({
  pages: {
    type: Number,
    required: true,
  },
  page: {
    type: Number,
    required: true,
  },
});

const route = useRoute();
const currentPath = ref(route.path);
</script>

<style lang="scss"></style>
