<template lang="pug">
div( class="relative flex flex-col lg:text-start items-center lg:items-start w-full mt-6 md:mt-0 overflow-hidden bg-white  rounded-[8px] " )
    
    div( class="w-full 2xl:h-[17.5rem] xs:h-[13rem] h-[12rem] p-4 items-center justify-center flex flex-col gap-y-2 text-center relative z-10 minimal-border  " 
     
    ) 
        Fa6SolidPlus(class="w-8 h-8")
        p {{ item.title }}
    



   

        //- NuxtLink(:to="'/blog/' + item.slug" class='text-[#241969] text-lg font-bold flex mt-2 mr-auto md:mr-0') Read More
        
</template>

<script setup lang="ts">
import Fa6SolidPlus from "~icons/fa6-solid/plus?width=392px&height=448px";

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

<style lang="scss" scoped>
.minimal-border {
  position: relative;
  isolation: isolate;
  transition: all 0.2s ease;
  border-radius: 8px;

  // Light mode styles
  @media (prefers-color-scheme: dark) {
    background: #ffffff;
    border: 1px solid #e2e8f0;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.05);

    &:hover {
      border-color: #cbd5e1;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.07);
    }
  }

  // Dark mode styles
  @media (prefers-color-scheme: light) {
    background: #1a1a1a;
    border: 1px solid #334155;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);

    &:hover {
      border-color: #475569;
      box-shadow: 0 4px 6px rgba(0, 0, 0, 0.15);
    }
  }
}
</style>
