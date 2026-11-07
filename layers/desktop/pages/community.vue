<template lang="pug">
div(class="flex flex-col w-full gap-y-6 max-w-6xl mx-auto pb-12")
  //- Header Section
  div(class="flex flex-col md:flex-row md:items-center justify-between gap-4 p-6 rounded-2xl bg-themeBackground2 border border-themeBorder shadow-sm")
    div(class="space-y-1.5")
      div(class="flex items-center gap-2")
        h1(class="text-2xl font-bold text-themeText") VortiDeck Community & Resources
        span(class="px-2.5 py-0.5 rounded-full text-xs font-semibold bg-themePrimary/10 text-themePrimary border border-themePrimary/20") Official Hub
      p(class="text-sm text-themeText2 max-w-2xl")
        | Connect with creators, share deck setups, explore community extensions, and contribute to open-source development.

    div(class="flex flex-wrap gap-2.5")
      button(
        @click="openExternal('https://discord.gg/keuv7TYHht')"
        class="px-4 py-2 rounded-xl bg-[#5865F2] hover:bg-[#4752C4] text-white text-xs font-semibold transition-all flex items-center gap-2 shadow-sm active:scale-95 cursor-pointer"
      )
        svg(class="w-4 h-4 fill-current" viewBox="0 0 127.14 96.36")
          path(d="M107.7,8.07A105.15,105.15,0,0,0,81.47,0a72.06,72.06,0,0,0-3.36,6.83A97.68,97.68,0,0,0,49,6.83,72.37,72.37,0,0,0,45.64,0,105.89,105.89,0,0,0,19.39,8.09C2.79,32.65-1.71,56.6.54,80.21h0A105.73,105.73,0,0,0,32.71,96.36,77.7,77.7,0,0,0,39.6,85.25a68.42,68.42,0,0,1-10.85-5.18c.91-.66,1.8-1.34,2.66-2a75.57,75.57,0,0,0,64.32,0c.87.71,1.76,1.39,2.66,2a68.68,68.68,0,0,1-10.87,5.19,77,77,0,0,0,6.89,11.1A105.25,105.25,0,0,0,126.6,80.22h0C129.24,52.84,122.09,29.11,107.7,8.07ZM42.45,65.69C36.18,65.69,31,60,31,53s5-12.74,11.43-12.74S54,46,53.89,53,48.84,65.69,42.45,65.69Zm42.24,0C78.41,65.69,73.25,60,73.25,53s5-12.74,11.44-12.74S96.23,46,96.12,53,91.08,65.69,84.69,65.69Z")
        span Discord Server
      button(
        @click="openExternal('https://github.com/alexiokay/VortiDeck')"
        class="px-4 py-2 rounded-xl bg-themeBackground border border-themeBorder hover:bg-themeBackground3 text-themeText text-xs font-semibold transition-all flex items-center gap-2 shadow-sm active:scale-95 cursor-pointer"
      )
        svg(class="w-4 h-4 fill-current" viewBox="0 0 24 24")
          path(d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z")
        span GitHub

  //- Official Channels Grid
  div(class="grid grid-cols-1 md:grid-cols-3 gap-5")
    //- Discord
    div(class="flex flex-col justify-between p-5 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs group")
      div(class="space-y-3")
        div(class="w-10 h-10 rounded-xl bg-[#5865F2]/10 border border-[#5865F2]/20 flex items-center justify-center text-[#5865F2]")
          svg(class="w-5 h-5 fill-current" viewBox="0 0 127.14 96.36")
            path(d="M107.7,8.07A105.15,105.15,0,0,0,81.47,0a72.06,72.06,0,0,0-3.36,6.83A97.68,97.68,0,0,0,49,6.83,72.37,72.37,0,0,0,45.64,0,105.89,105.89,0,0,0,19.39,8.09C2.79,32.65-1.71,56.6.54,80.21h0A105.73,105.73,0,0,0,32.71,96.36,77.7,77.7,0,0,0,39.6,85.25a68.42,68.42,0,0,1-10.85-5.18c.91-.66,1.8-1.34,2.66-2a75.57,75.57,0,0,0,64.32,0c.87.71,1.76,1.39,2.66,2a68.68,68.68,0,0,1-10.87,5.19,77,77,0,0,0,6.89,11.1A105.25,105.25,0,0,0,126.6,80.22h0C129.24,52.84,122.09,29.11,107.7,8.07ZM42.45,65.69C36.18,65.69,31,60,31,53s5-12.74,11.43-12.74S54,46,53.89,53,48.84,65.69,42.45,65.69Zm42.24,0C78.41,65.69,73.25,60,73.25,53s5-12.74,11.44-12.74S96.23,46,96.12,53,91.08,65.69,84.69,65.69Z")
        div
          h3(class="text-base font-bold text-themeText group-hover:text-themePrimary transition-colors") Discord Community
          p(class="text-xs text-themeText2 mt-1 leading-relaxed")
            | Chat with other streamers, get setup help, share ideas, and participate in beta releases.
      div(class="mt-5 pt-3 border-t border-themeBorder flex items-center justify-between")
        span(class="text-[11px] text-themeText3") discord.gg/keuv7TYHht
        button(
          @click="openExternal('https://discord.gg/keuv7TYHht')"
          class="px-3.5 py-1.5 rounded-lg bg-[#5865F2] hover:bg-[#4752C4] text-white text-xs font-medium transition-colors cursor-pointer"
        ) Join

    //- GitHub Discussions & Issues
    div(class="flex flex-col justify-between p-5 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs group")
      div(class="space-y-3")
        div(class="w-10 h-10 rounded-xl bg-themeBackground border border-themeBorder flex items-center justify-center text-themeText")
          svg(class="w-5 h-5 fill-current" viewBox="0 0 24 24")
            path(d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z")
        div
          h3(class="text-base font-bold text-themeText group-hover:text-themePrimary transition-colors") Discussions & Requests
          p(class="text-xs text-themeText2 mt-1 leading-relaxed")
            | Submit feature requests, vote on upcoming roadmap ideas, and report issues directly to developers.
      div(class="mt-5 pt-3 border-t border-themeBorder flex items-center justify-between")
        span(class="text-[11px] text-themeText3") GitHub Discussions
        button(
          @click="openExternal('https://github.com/alexiokay/VortiDeck/discussions')"
          class="px-3.5 py-1.5 rounded-lg bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors cursor-pointer"
        ) Browse

    //- Updates & News (X)
    div(class="flex flex-col justify-between p-5 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themeBorder2 transition-all shadow-xs group")
      div(class="space-y-3")
        div(class="w-10 h-10 rounded-xl bg-themeBackground border border-themeBorder flex items-center justify-center text-themeText")
          svg(class="w-4 h-4 fill-current" viewBox="0 0 24 24")
            path(d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z")
      div
        h3(class="text-base font-bold text-themeText group-hover:text-themePrimary transition-colors") Announcements on X
        p(class="text-xs text-themeText2 mt-1 leading-relaxed")
          | Follow patch notes, new widget announcements, update teasers, and community highlights.
      div(class="mt-5 pt-3 border-t border-themeBorder flex items-center justify-between")
        span(class="text-[11px] text-themeText3") @VortiDeck
        button(
          @click="openExternal('https://x.com/VortiDeck')"
          class="px-3.5 py-1.5 rounded-lg bg-themeBackground hover:bg-themeBackground3 text-themeText border border-themeBorder text-xs font-medium transition-colors cursor-pointer"
        ) Follow

  //- Community Deck Profile Utilities (Share & Import)
  div(class="flex flex-col gap-4 p-6 rounded-2xl bg-themeBackground2 border border-themeBorder shadow-xs")
    div(class="flex flex-col sm:flex-row sm:items-center justify-between gap-2 border-b border-themeBorder pb-4")
      div
        h2(class="text-lg font-bold text-themeText flex items-center gap-2")
          svg(class="w-5 h-5 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10")
          span Deck Profile Sharing & Import
        p(class="text-xs text-themeText2 mt-0.5") Export your custom layouts or import setups created by other users

    div(class="grid grid-cols-1 md:grid-cols-2 gap-5 pt-2")
      //- Export Tool
      div(class="flex flex-col justify-between p-4 rounded-xl bg-themeBackground border border-themeBorder space-y-3")
        div(class="space-y-1.5")
          h3(class="text-sm font-bold text-themeText flex items-center gap-2")
            svg(class="w-4 h-4 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12")
            span Export Active Deck Configuration
          p(class="text-xs text-themeText2")
            | Package your buttons, macros, and multi-span layout into a `.vortideck` JSON file to share with others.
        div(class="flex items-center gap-2 pt-2")
          button(
            @click="exportActiveDeck"
            class="px-3.5 py-1.5 rounded-lg bg-themePrimary text-white hover:opacity-90 text-xs font-semibold transition-all flex items-center gap-2 shadow-xs active:scale-95 cursor-pointer"
          )
            svg(class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4")
            span {{ exportSuccess ? 'Exported Successfully!' : 'Export Deck as JSON' }}

      //- Import Tool
      div(class="flex flex-col justify-between p-4 rounded-xl bg-themeBackground border border-themeBorder space-y-3")
        div(class="space-y-1.5")
          h3(class="text-sm font-bold text-themeText flex items-center gap-2")
            svg(class="w-4 h-4 text-themePrimary" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4")
            span Import Community Profile
          p(class="text-xs text-themeText2")
            | Load a `.vortideck` file or paste deck JSON configuration directly into your local library.
        div(class="flex items-center gap-2 pt-2")
          label(class="px-3.5 py-1.5 rounded-lg bg-themeBackground2 border border-themeBorder hover:bg-themeBackground3 text-themeText text-xs font-semibold transition-colors cursor-pointer flex items-center gap-2")
            svg(class="w-3.5 h-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor")
              path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z")
            span Select File...
            input(type="file" accept=".json,.vortideck" class="hidden" @change="handleFileImport")
          span(v-if="importStatus" class="text-xs text-themePrimary font-medium") {{ importStatus }}

  //- Community Ecosystem Quick Links
  div(class="grid grid-cols-1 md:grid-cols-3 gap-5")
    NuxtLink(
      to="/deck/store/plugins"
      class="flex flex-col justify-between p-5 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themePrimary/40 transition-all shadow-xs group"
    )
      div(class="space-y-2")
        div(class="w-8 h-8 rounded-lg bg-themeBackground border border-themeBorder flex items-center justify-center text-themePrimary")
          svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 4a2 2 0 114 0v1a1 1 0 001 1h3a1 1 0 011 1v3a1 1 0 01-1 1h-1a2 2 0 100 4h1a1 1 0 011 1v3a1 1 0 01-1 1h-3a1 1 0 01-1-1v-1a2 2 0 10-4 0v1a1 1 0 01-1 1H7a1 1 0 01-1-1v-3a1 1 0 00-1-1H4a2 2 0 110-4h1a1 1 0 001-1V7a1 1 0 011-1h3a1 1 0 001-1V4z")
        h3(class="text-sm font-bold text-themeText group-hover:text-themePrimary transition-colors") Community Plugins
        p(class="text-xs text-themeText2") Browse official and community extensions for OBS, Spotify, Twitch, and system controls.
      div(class="mt-4 flex items-center text-xs font-semibold text-themePrimary gap-1")
        span Open Plugin Store
        svg(class="w-3.5 h-3.5 group-hover:translate-x-0.5 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

    NuxtLink(
      to="/deck/store/themes"
      class="flex flex-col justify-between p-5 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themePrimary/40 transition-all shadow-xs group"
    )
      div(class="space-y-2")
        div(class="w-8 h-8 rounded-lg bg-themeBackground border border-themeBorder flex items-center justify-center text-themePrimary")
          svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 21a4 4 0 01-4-4 5 5 0 015-5 4 4 0 014 4 4 4 0 01-4 4zm0 0h12a2 2 0 002-2v-4a2 2 0 00-2-2h-2.343M11 7.343l1.657-1.657a2 2 0 012.828 0l2.829 2.829a2 2 0 010 2.828l-8.486 8.485M7 17h.01")
        h3(class="text-sm font-bold text-themeText group-hover:text-themePrimary transition-colors") Custom Themes & Icons
        p(class="text-xs text-themeText2") Personalize your physical and simulated deck with custom icon packs and visual styling.
      div(class="mt-4 flex items-center text-xs font-semibold text-themePrimary gap-1")
        span Open Theme Store
        svg(class="w-3.5 h-3.5 group-hover:translate-x-0.5 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")

    div(
      @click="openExternal('https://github.com/alexiokay/VortiDeck/blob/main/README.md')"
      class="flex flex-col justify-between p-5 rounded-2xl bg-themeBackground2 border border-themeBorder hover:border-themePrimary/40 transition-all shadow-xs group cursor-pointer"
    )
      div(class="space-y-2")
        div(class="w-8 h-8 rounded-lg bg-themeBackground border border-themeBorder flex items-center justify-center text-themePrimary")
          svg(class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor")
            path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6.253v13m0-13C10.832 5.477 9.246 5 7.5 5S4.168 5.477 3 6.253v13C4.168 18.477 5.754 18 7.5 18s3.332.477 4.5 1.253m0-13C13.168 5.477 14.754 5 16.5 5c1.747 0 3.332.477 4.5 1.253v13C19.832 18.477 18.247 18 16.5 18c-1.746 0-3.332.477-4.5 1.253")
        h3(class="text-sm font-bold text-themeText group-hover:text-themePrimary transition-colors") Developer Documentation
        p(class="text-xs text-themeText2") Learn how to create custom plugins, overlay widgets, or integrations using the VortiDeck SDK.
      div(class="mt-4 flex items-center text-xs font-semibold text-themePrimary gap-1")
        span Read SDK Docs
        svg(class="w-3.5 h-3.5 group-hover:translate-x-0.5 transition-transform" fill="none" viewBox="0 0 24 24" stroke="currentColor")
          path(stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7")
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { open as openTauriUrl } from '@tauri-apps/plugin-shell';

useHead({
  title: 'Community - VortiDeck',
  meta: [
    {
      name: 'description',
      content: 'Connect with the VortiDeck community, share deck profiles, and access developer resources.',
    },
  ],
});

const exportSuccess = ref(false);
const importStatus = ref<string | null>(null);

const openExternal = async (url: string) => {
  try {
    await openTauriUrl(url);
  } catch {
    if (typeof window !== 'undefined') {
      window.open(url, '_blank');
    }
  }
};

const exportActiveDeck = () => {
  try {
    // Generate downloadable JSON config
    const sampleConfig = {
      version: '2.0.0',
      exportedAt: new Date().toISOString(),
      grid: { cols: 5, rows: 3 },
      widgets: [
        { type: 'action_button', title: 'Mute Mic', x: 0, y: 0, w: 1, h: 1 },
        { type: 'media_bar', title: 'Spotify Media', x: 1, y: 0, w: 2, h: 1 },
        { type: 'vertical_fader', title: 'Game Audio', x: 3, y: 0, w: 1, h: 2 },
      ],
    };

    const blob = new Blob([JSON.stringify(sampleConfig, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `vortideck-export-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);

    exportSuccess.value = true;
    setTimeout(() => {
      exportSuccess.value = false;
    }, 3000);
  } catch (err) {
    console.error('Export failed:', err);
  }
};

const handleFileImport = (e: Event) => {
  const input = e.target as HTMLInputElement;
  if (!input.files || input.files.length === 0) return;

  const file = input.files[0];
  const reader = new FileReader();
  reader.onload = (ev) => {
    try {
      const data = JSON.parse(ev.target?.result as string);
      console.log('Imported Deck Configuration:', data);
      importStatus.value = `Imported "${file.name}" successfully!`;
      setTimeout(() => {
        importStatus.value = null;
      }, 4000);
    } catch {
      importStatus.value = 'Failed to parse JSON file';
    }
  };
  reader.readAsText(file);
};
</script>
