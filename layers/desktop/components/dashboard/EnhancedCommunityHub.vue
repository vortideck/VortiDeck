<template lang="pug">
div(class="flex flex-col p-4 rounded-xl bg-themeBackground2 shadow-sm relative overflow-hidden")
  //- Background pattern
  div(class="absolute inset-0 opacity-5")
    svg(viewBox="0 0 100 100" class="w-full h-full")
      defs
        pattern(id="grid" width="10" height="10" patternUnits="userSpaceOnUse")
          path(d="M 10 0 L 0 0 0 10" fill="none" stroke="currentColor" stroke-width="0.5")
      rect(width="100" height="100" fill="url(#grid)")
  
  //- Header with controls
  div(class="flex w-full justify-start items-center gap-x-3 mb-4 relative z-10")
    div(class="flex items-center gap-x-3")
      div(class="w-10 h-10 rounded-full bg-gradient-to-r from-blue-500 to-indigo-600 flex items-center justify-center shadow-lg")
        Fa6SolidRocket(class="w-4 h-4 text-white")
      div
        p(class="font-semibold") Enhanced Community Hub
        p(class="text-xs text-themeText/60") Real-time collaboration space
    
    div(class="ml-auto flex items-center gap-x-2")
      //- Filter dropdown
      div(class="relative")
        select(class="text-xs bg-themeBackground3/50 border border-themeText/10 rounded px-2 py-1 pr-6 focus:outline-none focus:border-blue-400")
          option All Categories
          option UI/UX
          option Performance  
          option Integrations
          option New Features
      
      //- Sort dropdown
      div(class="relative")
        select(class="text-xs bg-themeBackground3/50 border border-themeText/10 rounded px-2 py-1 pr-6 focus:outline-none focus:border-blue-400")
          option Most Voted
          option Newest
          option In Progress
          option Hot Trending
      
      button(class="text-xs bg-gradient-to-r from-blue-500 to-indigo-600 text-white px-3 py-1 rounded-full hover:from-blue-600 hover:to-indigo-700 transition-all duration-200 shadow-md hover:shadow-lg flex items-center gap-x-1")
        Fa6SolidChartSimple(class="w-3 h-3")
        span Analytics
  
  //- Enhanced Timeline with status indicators
  div(class="relative px-4 mb-6")
    //- Dynamic background line with glow
    div(class="absolute top-6 left-0 right-0 h-0.5 bg-themeBackground3")
    div(class="absolute top-6 left-0 h-0.5 bg-gradient-to-r from-green-400 via-blue-500 to-purple-500 shadow-lg" style="width: 75%")
    div(class="absolute top-6 left-0 h-1 bg-gradient-to-r from-green-400 via-blue-500 to-purple-500 opacity-30 blur-sm" style="width: 75%")
    
    //- Enhanced timeline items with animations
    div(class="flex justify-between relative")
      //- Completed with celebration
      div(class="flex flex-col items-center group cursor-pointer")
        div(class="w-10 h-10 rounded-full bg-green-500 flex items-center justify-center mb-3 shadow-lg hover:shadow-xl transition-all duration-300 hover:scale-110 relative")
          Fa6SolidCheck(class="w-4 h-4 text-white")
          //- Pulse animation
          div(class="absolute inset-0 rounded-full bg-green-400 animate-ping opacity-20")
          //- Status badge
          div(class="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-yellow-400 flex items-center justify-center")
            span(class="text-[8px] font-bold text-white") ✨
        div(class="text-center")
          p(class="text-sm font-medium") Discord RPC
          p(class="text-xs text-green-600 font-semibold") Shipped!
          div(class="flex items-center gap-x-1 mt-1")
            Fa6SolidHeart(class="w-3 h-3 text-red-400")
            span(class="text-xs text-themeText/60") 847 loves
      
      //- In Progress with real-time updates
      div(class="flex flex-col items-center group cursor-pointer")
        div(class="w-10 h-10 rounded-full bg-blue-500 flex items-center justify-center mb-3 shadow-lg hover:shadow-xl transition-all duration-300 hover:scale-110 relative")
          Fa6SolidHammer(class="w-4 h-4 text-white")
          //- Progress ring
          svg(class="absolute -inset-1 w-12 h-12 transform -rotate-90")
            circle(cx="24" cy="24" r="22" stroke="currentColor" stroke-width="2" fill="none" class="text-blue-300/30")
            circle(cx="24" cy="24" r="22" stroke="currentColor" stroke-width="2" fill="none" stroke-dasharray="138" stroke-dashoffset="55" class="text-blue-400 transition-all duration-500")
          //- Hot badge
          div(class="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-red-500 flex items-center justify-center animate-pulse")
            span(class="text-[8px] font-bold text-white") 🔥
        div(class="text-center")
          p(class="text-sm font-medium") Linux Support
          p(class="text-xs text-blue-600 font-semibold") 60% Complete
          div(class="flex items-center gap-x-1 mt-1")
            div(class="w-1 h-1 rounded-full bg-blue-500 animate-pulse")
            span(class="text-xs text-themeText/60") ETA: 2 weeks
      
      //- Queued with vote momentum
      div(class="flex flex-col items-center group cursor-pointer opacity-75 hover:opacity-100 transition-opacity")
        div(class="w-10 h-10 rounded-full bg-themeBackground3 border-2 border-purple-400 border-dashed flex items-center justify-center mb-3 hover:border-solid transition-all duration-300")
          Fa6SolidClock(class="w-4 h-4 text-purple-400")
          //- Trending badge
          div(class="absolute -top-1 -right-1 w-4 h-4 rounded-full bg-orange-500 flex items-center justify-center")
            span(class="text-[8px] font-bold text-white") ↗
        div(class="text-center")
          p(class="text-sm font-medium") Custom Themes
          p(class="text-xs text-purple-600 font-semibold") Next Up
          div(class="flex items-center gap-x-1 mt-1")
            span(class="text-xs text-orange-600 font-semibold") +23 votes today
  
  //- Enhanced request section with live features
  div(class="mt-4 pt-4 border-t border-themeText/10")
    div(class="grid grid-cols-1 md:grid-cols-3 gap-4")
      //- Enhanced submit form
      div(class="bg-gradient-to-br from-blue-500/5 to-indigo-500/10 rounded-lg p-4 border border-blue-500/20 relative")
        div(class="flex items-center gap-x-2 mb-3")
          Fa6SolidLightbulb(class="w-5 h-5 text-blue-500")
          span(class="font-semibold") Submit Your Idea
          div(class="ml-auto")
            span(class="text-xs text-themeText/50") 💡 AI-powered duplicate detection
        
        textarea(placeholder="Describe your feature idea in detail..." rows="3" class="w-full bg-themeBackground3/50 rounded-lg px-3 py-2 text-sm placeholder-themeText/40 border border-themeText/10 focus:border-blue-500/50 focus:outline-none transition-colors resize-none")
        
        div(class="flex items-center justify-between mt-2 mb-3")
          span(class="text-xs text-themeText/60") Characters: 0/500
          div(class="flex items-center gap-x-1")
            span(class="text-xs text-themeText/50") Category:
            select(class="text-xs bg-themeBackground3/50 border border-themeText/10 rounded px-1 py-0.5")
              option Feature
              option Bug Fix
              option Integration
              option UI/UX
        
        button(class="w-full bg-gradient-to-r from-blue-500 to-indigo-600 hover:from-blue-600 hover:to-indigo-700 text-white px-4 py-2 rounded-lg transition-all duration-200 shadow-md hover:shadow-lg flex items-center justify-center gap-x-2 font-medium")
          Fa6SolidPaperPlane(class="w-3 h-3")
          span Submit Request
      
      //- Live trending requests
      div(class="space-y-3")
        div(class="flex items-center justify-between mb-3")
          div(class="text-sm font-bold text-themeText/70 uppercase") 🔥 Trending Now
          span(class="text-sm text-themeText/50") Live updates
        
        div(class="space-y-2")
          div(class="group bg-themeBackground3/30 rounded-lg px-3 py-2 hover:bg-themeBackground3/50 transition-all duration-200 cursor-pointer border-l-2 border-red-400")
            div(class="flex items-center justify-between mb-1")
              p(class="text-sm font-medium") Spotify Integration
              div(class="flex items-center gap-x-1")
                button(class="hover:scale-110 transition-transform")
                  Fa6SolidHeart(class="w-3 h-3 text-red-400")
                span(class="text-sm text-blue-600 font-semibold") 234
            div(class="flex items-center gap-x-2")
              span(class="px-1 py-0.5 bg-red-100 text-red-700 rounded text-sm font-semibold") HOT
              span(class="text-sm text-themeText/60") +12 votes in last hour
          
          div(class="group bg-themeBackground3/30 rounded-lg px-3 py-2 hover:bg-themeBackground3/50 transition-all duration-200 cursor-pointer border-l-2 border-orange-400")
            div(class="flex items-center justify-between mb-1")
              p(class="text-sm font-medium") OBS Scene Manager
              div(class="flex items-center gap-x-1")
                button(class="hover:scale-110 transition-transform")
                  Fa6SolidHeart(class="w-3 h-3 text-red-400")
                span(class="text-sm text-blue-600 font-semibold") 189
            div(class="flex items-center gap-x-2")
              span(class="px-1 py-0.5 bg-orange-100 text-orange-700 rounded text-sm font-semibold") NEW
              span(class="text-sm text-themeText/60") Submitted 2h ago
      
      //- Quick stats & insights
      div(class="space-y-3")
        div(class="text-sm font-bold text-themeText/70 uppercase mb-3") 📊 Community Insights
        
        div(class="space-y-3")
          div(class="bg-themeBackground3/20 rounded-lg p-3")
            div(class="flex items-center justify-between mb-2")
              span(class="text-sm font-medium") This Week
              span(class="text-sm text-green-600 font-semibold") +15%
            div(class="space-y-1")
              div(class="flex justify-between text-sm")
                span(class="text-themeText/70") New Requests
                span(class="font-semibold") 47
              div(class="flex justify-between text-sm")
                span(class="text-themeText/70") Total Votes
                span(class="font-semibold") 1.2k
              div(class="flex justify-between text-sm")
                span(class="text-themeText/70") Implemented
                span(class="font-semibold text-green-600") 3
          
          div(class="bg-gradient-to-r from-blue-500/10 to-purple-500/10 rounded-lg p-3 border border-blue-500/20")
            div(class="flex items-center gap-x-2 mb-2")
              span(class="text-sm font-semibold") 🎯 Most Wanted Category
              span(class="px-1 py-0.5 bg-blue-100 text-blue-700 rounded text-sm font-semibold") INTEGRATIONS
            p(class="text-sm text-themeText/60") 67% of votes this month focus on third-party integrations
            
            button(class="mt-2 text-sm bg-blue-500 text-white px-2 py-1 rounded hover:bg-blue-600 transition-colors w-full")
              span See Roadmap →
</template>

<script setup lang="ts">
import Fa6SolidRocket from "~icons/fa6-solid/rocket?width=16px&height=16px"
import Fa6SolidChartSimple from "~icons/fa6-solid/chart-simple?width=16px&height=16px"
import Fa6SolidCheck from "~icons/fa6-solid/check?width=16px&height=16px"
import Fa6SolidHeart from "~icons/fa6-solid/heart?width=16px&height=16px"
import Fa6SolidHammer from "~icons/fa6-solid/hammer?width=16px&height=16px"
import Fa6SolidClock from "~icons/fa6-solid/clock?width=16px&height=16px"
import Fa6SolidLightbulb from "~icons/fa6-solid/lightbulb?width=16px&height=16px"
import Fa6SolidPaperPlane from "~icons/fa6-solid/paper-plane?width=16px&height=16px"
</script>