<template lang="pug">
    div(class="deck-editor-layout flex flex-col")
        WindowsTitlebar(class="z-[10001]")
       
        div(id="main-app" style="" class="overflow-y-hidden h-[calc(100vh-30px)] z-[90] flex flex-row bg-themeBackground text-themeText justify-start items-start w-full mt-[30px]")
           
            //- div#overlay(class="absolute overlaying top-0 left-0 w-full h-screen bg-black opacity-40 z-10 hidden-overlay")
          
            SidebarDeckEditor(class="")
            div(class="flex flex-col flex-1 min-w-0 ml-[5.2rem] h-full min-h-0 overflow-hidden")
              
              Navbar(
                :title="pageMeta.title? pageMeta.title : 'Deck Editor'"
                :subtitle="pageMeta.description? pageMeta.description : 'Customize your deck layout and actions'"
                :minified="true"
                :compactLeftPadding="true"
                :bottomDivider="true"
              )
              div(class="flex flex-col w-full flex-1 min-h-0 overflow-hidden")
                <slot class="" />
                <!-- Footer(v-if="route.path!=='/login' && route.path!=='/signup'" class="mt-12") -->
        div(class="cursor cursor--small")
        
    </template>

<script setup lang="ts">
const route = useRoute();

// get pageMeta title and description
const pageMeta = computed(() => {
  return route.meta;
});
</script>

<style lang="sass">



*
    scroll-snap-type: y mandatory

body
    overflow-x: hidden



.app-container

    z-index: 10
    display: flex
    flex-direction: column
    justify-content: center



.panel
    margin-top: 60px
    text-align: center
    margin: 0 auto
    scroll-snap-align: center

    width: 90%
    p.title

        font-style: normal
        font-size: 5vh
        line-height: 0%
        align-self: center
        letter-spacing: 0.18em





.cursor
  position: fixed
  left: 0
  top: 0
  pointer-events: none

.cursor--small
  width: 5px
  height: 5px
  left: -2.5px
  top: -2.5px
  border-radius: 50%
  z-index: 11000
  background: var(--color-text)

.cursor--canvas
  width: 100vw
  height: 100vh
  z-index: 12000

.hidden-overlay
  visibility: hidden
  opacity: 0

.visible-overlay
  visibility: visible
  opacity: 0.4

</style>
