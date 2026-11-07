<template lang="pug">
OverlayModal(
  :show="show"
  title="Overlay projects"
  subtitle="Open, duplicate, import, or export saved overlay designs"
  modal-class="max-w-4xl max-h-[80vh] h-[72vh]"
  @close="$emit('close')"
)
  div(class="flex min-h-0 flex-1 flex-col bg-themeBackground")
    div(class="flex items-center justify-between border-b border-themeBorder px-6 py-3")
      p(class="text-xs text-themeText2") {{ projects.length }} saved {{ projects.length === 1 ? 'project' : 'projects' }}
      button(
        type="button"
        @click="$emit('create-project')"
        class="inline-flex items-center gap-2 rounded-xl bg-themeButton px-4 py-2 text-xs font-semibold text-white shadow-xs transition-colors hover:bg-themeButtonHover"
      )
        Icon(icon="heroicons:plus" class="h-4 w-4")
        span New project

    div(class="min-h-0 flex-1 overflow-y-auto p-6")
      div(v-if="projects.length === 0" class="grid h-full min-h-52 place-items-center text-center")
        div
          div(class="mx-auto mb-3 grid h-11 w-11 place-items-center rounded-xl border border-themeBorder bg-themeBackground2 text-themePrimary")
            Icon(icon="heroicons:squares-2x2" class="h-5 w-5")
          p(class="text-sm font-semibold text-themeText") No overlay projects yet
          p(class="mt-1 text-xs text-themeText2") Create a project to save your canvas and theme.

      div(v-else class="space-y-3")
        article(
          v-for="project in projects"
          :key="project.id"
          class="flex items-center justify-between gap-4 rounded-xl border border-themeBorder bg-themeBackground2 p-4 shadow-xs transition-colors hover:border-themePrimary/40"
        )
          div(class="flex min-w-0 items-center gap-4")
            div(class="grid h-12 w-12 shrink-0 place-items-center rounded-xl border border-themePrimary/25 bg-themePrimary/10 text-themePrimary")
              Icon(icon="heroicons:rectangle-group" class="h-5 w-5")
            div(class="min-w-0")
              h3(class="truncate text-sm font-semibold text-themeText") {{ project.name }}
              p(class="mt-0.5 truncate text-xs text-themeText2") {{ project.description || 'No description' }}
              div(class="mt-1.5 flex flex-wrap items-center gap-x-3 gap-y-1 text-[11px] text-themeText3")
                span {{ formatDate(project.lastModified) }}
                span {{ project.elements?.length || project.elementCount || 0 }} elements
                span {{ project.size || '0 KB' }}
              div(v-if="project.tags?.length" class="mt-2 flex flex-wrap gap-1.5")
                span(v-for="tag in project.tags" :key="tag" class="rounded-full border border-themeBorder bg-themeBackground3 px-2 py-0.5 text-[10px] text-themeText2") {{ tag }}

          div(class="flex shrink-0 items-center gap-1.5")
            button(
              type="button"
              @click="$emit('load-project', project)"
              class="rounded-lg bg-themeButton px-3 py-2 text-xs font-semibold text-white transition-colors hover:bg-themeButtonHover"
            ) Open
            button(
              type="button"
              @click="$emit('duplicate-project', project)"
              class="rounded-lg p-2 text-themeText2 transition-colors hover:bg-themeBackground3 hover:text-themeText"
              title="Duplicate project"
            )
              Icon(icon="heroicons:document-duplicate" class="h-4 w-4")
            button(
              type="button"
              @click="$emit('delete-project', project.id)"
              class="rounded-lg p-2 text-red-400 transition-colors hover:bg-red-500/10 hover:text-red-300"
              title="Delete project"
            )
              Icon(icon="heroicons:trash" class="h-4 w-4")

  template(#footer)
    div(class="flex items-center justify-between gap-4")
      p(class="text-xs text-themeText2") Total storage: {{ totalStorage }}
      div(class="flex items-center gap-2")
        button(@click="$emit('import-project')" class="rounded-xl px-4 py-2 text-xs font-medium text-themeText2 transition-colors hover:bg-themeBackground3 hover:text-themeText") Import
        button(@click="$emit('export-all')" class="rounded-xl border border-themeBorder bg-themeBackground3 px-4 py-2 text-xs font-semibold text-themeText transition-colors hover:border-themePrimary/40") Export all
</template>

<script setup lang="ts">
import { Icon } from '@iconify/vue'
import OverlayModal from './OverlayModal.vue'

interface Project {
  id: string
  name: string
  description?: string
  lastModified: string | Date
  elementCount?: number
  size?: string
  elements?: unknown[]
  tags?: string[]
}

const props = withDefaults(defineProps<{
  show?: boolean
  projects?: Project[]
  totalStorage?: string
}>(), {
  show: false,
  projects: () => [],
  totalStorage: '0 KB'
})

const emit = defineEmits<{
  close: []
  'create-project': []
  'load-project': [project: Project]
  'duplicate-project': [project: Project]
  'delete-project': [id: string]
  'import-project': []
  'export-all': []
}>()

const formatDate = (date: string | Date) => {
  const d = new Date(date)
  return d.toLocaleDateString() + ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}
</script>
