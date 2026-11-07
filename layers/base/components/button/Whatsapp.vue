<template lang="pug">
a#whatsapp-button(class="hover:cursor-pointer pl-[0.65rem] pr-[0.55rem] pb-[0.65rem] pt-[0.55rem] bg-[#25d366] rounded-full " href="https://wa.me/+31657705530")
    a(href="https://wa.me/31657705530" target="_blank")
    //- nuxt-img(src="icons/whatsapp_button.png" alt="whatsapp" width="100" height="100" class="w-24 h-24")
    IcOutlineWhatsapp(class="w-[3rem] h-[3rem] invert" )
</template>

<script setup lang="ts">
import IcOutlineWhatsapp from "~icons/ic/outline-whatsapp";

const whatsappButton = {};
</script>

<style lang="scss">
#whatsapp-button {
  box-shadow: 0px 0px 11px rgba(0, 0, 0, 0.5);
}
</style>
