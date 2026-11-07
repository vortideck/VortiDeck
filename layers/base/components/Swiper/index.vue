<template lang="pug">
div(class="flex flex-col lg:items-center justify-center w-full  md:px-0  ")
  Swiper-Container(
        ref="swiperContainerRef"
      
        class="w-full  swiper-container pt-4" v-if="isSwiperLoaded")
        <!-- :creative-effect="{ prev: {shadow: false, translate: ['-20%', 0, -1],}, next: {translate: ['100%', 0, 0],},}" -->
        // @slideChange="onSlideChange"
      
        Swiper-Slide(
          v-for="item in items" 
          :key="item.id" 
          class="px-1 sm:px-6 md:px-3 xl:px-4 w-full md:w-3/4 pb-[1.2rem] pt-[2.2rem] swiper-slide-animate" 
          :class="item.active && item.id !== 0? 'slide-active' : '', item.active && item.id !== 0? '': 'hover:cursor-pointer '"
          @click="setSlideActive(item)"
     
        )
          div(
            class="flex flex-col lg:flex-row w-full justify-start h-auto items-start gap-y-12 gap-x-[2.5rem] content-animate" 
            data-aos="fade-right"
          )
            SwiperItem(:item="item" v-if="item.id !== 0")
              template(v-slot:image)
                nuxt-img(
                  v-if="item.image"
                  :src="item.image"  
                  format="webp" 
                  width="500" 
                  height="400" 
                  class="w-full h-full object-cover  image-animate"
                  :class="item.active ? '' : 'cursor-pointer'"
                )
            SwiperItem0(:item="item" v-else )
      

                  
</template>

<script setup lang="ts">
import type { Swiper } from "swiper";
import ArrowIcon from "~icons/ph/arrow-right";
import LoadingIcon from "~icons/eos-icons/loading";
import { Autoplay } from "swiper/modules";

interface SwiperItemData {
  id: string | number;
  active: boolean;
  image?: string;
  [key: string]: unknown;
}

const isSwiperLoaded = ref(false);

let swiperContainerRef = ref(null);
let swiper = useSwiper(swiperContainerRef, {
  loop: false,
  modules: [Autoplay],
  effect: "slide",
  centeredSlides: true,
  watchSlidesProgress: true,
  freeMode: {
    enabled: true,
    sticky: true,
    momentumRatio: 0.55,
  },
  spaceBetween: 30,
  breakpoints: {
    2200: { slidesPerView: 6, spaceBetween: 20 },
    1920: { slidesPerView: 6, spaceBetween: 20 },
    1160: { slidesPerView: 6, spaceBetween: 20 },
    768: { slidesPerView: 5.8, spaceBetween: 20 },
    470: { slidesPerView: 6, spaceBetween: 20 },
    100: { slidesPerView: 6, spaceBetween: 20 },
  },
  speed: 400,
  autoplay: { delay: 1000, disableOnInteraction: false },
});

const props = defineProps<{ items: SwiperItemData[] }>();

const activeService = computed(() => {
  return props.items[0];
});

const setSlideActive = (service: SwiperItemData) => {
  if (service.id === 0) {
    return;
  }
  // Find the index of the clicked service in the items array
  const index = props.items.findIndex((item) => item.id === service.id);

  // Update active states
  props.items.forEach((item) => (item.active = false));
  service.active = true;

  swiper.to(index, 400, true);
};

const onStepClick = (index: number) => {
  swiper = useSwiper(swiperContainerRef);
  swiper.to(index);
};

const onSlideChange = (swiper: Swiper) => {
  // Get real index accounting for loop
  const realIndex = swiper.realIndex;
  const progress = swiper.progress;
  const slidesPerView = typeof swiper.params.slidesPerView === "number"
    ? swiper.params.slidesPerView
    : 1;
  const slideWidth = swiper.width / slidesPerView;

  // Calculate which slide is most visible based on progress
  const visibleIndex = Math.round(progress * (props.items.length - 1));

  // Update active states with interpolation
  props.items.forEach((item, index) => {
    const slideOffset = Math.abs(index - visibleIndex);
    const visibility = Math.max(0, 1 - slideOffset / 2);

    // Smoothly update active state based on visibility
    if (visibility > 0.5) {
      item.active = true;
    } else {
      item.active = false;
    }
  });
};

const selectedSlide = computed(() => {
  return props.items.findIndex((item) => item.active);
});

onMounted(() => {
  isSwiperLoaded.value = true;

  const initialIndex = 2;
  // Set initial active slide and center it
  if (props.items.length > 1) {
    nextTick(() => {
      swiper.to(initialIndex, 0, true);
    });
  }
});
</script>

<style lang="scss">
.swiper-slide-animate {
  will-change: transform;
  transform: scale(1);
  transition: none;
  animation: slideScale 0.3s cubic-bezier(0.4, 0, 0.2, 1) forwards;
}

.slide-active {
  z-index: 10;
  animation: slideScaleActive 0.3s cubic-bezier(0.4, 0, 0.2, 1) forwards;
}

.content-animate {
  will-change: transform;
  backface-visibility: hidden;
  transform: translateZ(0);
}

.image-animate {
  will-change: transform;
  backface-visibility: hidden;
  transform: translateZ(0);
}

@keyframes slideScale {
  from {
    transform: scale(1.25);
  }
  to {
    transform: scale(1);
  }
}

@keyframes slideScaleActive {
  from {
    transform: scale(1);
  }
  to {
    transform: scale(1.25);
  }
}
</style>
