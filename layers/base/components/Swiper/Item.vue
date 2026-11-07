<template lang="pug">
div( class="relative flex flex-col  lg:text-start items-center lg:items-start   w-full mt-6 md:mt-0 overflow-hidden " )
    
    NuxtLink( class="w-full 2xl:h-[17.5rem]  xs:h-[13rem] h-[12rem] rounded-[6px] overflow-hidden ") 
        slot(name="image")
    
    div(class="absolute top-3 left-0 p-2 lg:p-4 bg-[#Ffffff] font-medium text-sm lg:text-lg text-[#64626E] shadow-lg " v-if="!item.installed && item.id !== 0")
        p Item is not Installed

    div(class=" gap-y-2 w-full flex flex-col px-4 lg:px-5 3xl:px-8 py-3 lg:py-3 4xl:py-6 3xl:gap-y-3  ")


        NuxtLink(class="text-lg font-normal  w-full  text-center" :class="item.active ? '' : 'hover:cursor-pointer'" v-if="item.active" ) 
            h5 {{item.title}}
    
        p(class='lg:text-xl  text-[#64626E] text-start line-clamp-5') 

        //- NuxtLink(:to="'/blog/' + item.slug" class='text-[#241969] text-lg font-bold flex mt-2 mr-auto md:mr-0') Read More
        
</template>

<script setup lang="ts">
import ArrowIcon from "~icons/ph/arrow-right";

import CommentsIcon from "~icons/octicon/comment-24";
import ViewsIcon from "~icons/carbon/view";

const router = useRouter();

defineProps({
  item: {
    type: Object,
    required: true,
  },
});
</script>

<style lang="scss"></style>
