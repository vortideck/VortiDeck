<template lang="pug">
div(class=" h-full bg-themeBackground items-center justify-start flex flex-col gap-y-8 font-semibold  -mx-20  ")
    div(class="w-full h-auto flex flex-col items-center justify-center  ")
        div( class=" w-full   flex items-center justify-between   px-20")
            p(class="text-2xl font-semibold") Games and Applications
            button(class="text-sm ") Scan
        div(class="w-full h-full  flex items-center justify-center ")
            Swiper(:items="items")
    
    div(class="w-full h-full  flex items-center justify-center ")
        div(class="w-full h-full  flex items-center justify-center ")
        
            //- p( alt="app.name" v-for="app in apps") {{app.publisher}}
            p Still in development

       
  
        

</template>

<script setup lang="ts">
import { invoke } from "@tauri-apps/api/core";

// const apps = await invoke("get_installed_apps");

const createItem = (app: any) => {
  return {
    id: app.id,
    title: app.name,
    image: app.image,
    active: app.active,
    installed: app.installed,
  };
};
const items = ref([
  {
    id: 0,
    title: "Add Game or Application",
    image: "",
    active: false,
    installed: false,
  },
  {
    id: 1,
    title: "Item 1",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
  {
    id: 2,
    title: "Item 2",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
  {
    id: 3,
    title: "Item 3",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
  {
    id: 4,
    title: "Item 4",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
  {
    id: 5,
    title: "Item 5",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
  {
    id: 6,
    title: "Item 6",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
  {
    id: 7,
    title: "Item 7",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
  {
    id: 8,
    title: "Item 8",
    image:
      "https://encrypted-tbn0.gstatic.com/images?q=tbn:ANd9GcT1Y7C1_g9UKwGwhIJ7jRQjCn2jG4l8GZAnWg&s",
    active: false,
    installed: true,
  },
]);
</script>

<style lang="scss" scoped></style>
