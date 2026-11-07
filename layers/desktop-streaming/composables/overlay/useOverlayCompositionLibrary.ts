import { invoke } from '@tauri-apps/api/core'
import { ref } from 'vue'

export interface CompositionContent {
  name: string
  elements: Record<string, unknown>[]
  settings: Record<string, unknown>
}
export interface CompositionDocument {
  schemaVersion: 1
  id: string
  revision: number
  created: string
  lastModified: string
  content: CompositionContent
  publishedRevision: number | null
  canUndo: boolean
  canRedo: boolean
}
export interface CompositionProject extends CompositionContent {
  id: string
  created: string
  lastModified: string
}
export const cloneComposition = <T>(value: T): T => JSON.parse(JSON.stringify(value)) as T

export function compositionContent(project: CompositionContent): CompositionContent {
  const content = cloneComposition(project)
  content.elements.forEach(element => {
    // Selection and runtime grant/schema caches are not composition edits.
    delete element.selected
    delete element.nativeSyncPending
    delete element.configSchema
    delete element.packageInfo
    delete element.configRevision
  })
  return { name: content.name, elements: content.elements, settings: content.settings }
}

export const compositionProject = (document: CompositionDocument): CompositionProject => ({
  id: document.id, created: document.created, lastModified: document.lastModified,
  ...cloneComposition(document.content),
})

export function useOverlayCompositionLibrary() {
  const documents = ref<Record<string, CompositionDocument>>({})
  const error = ref('')
  const busy = ref(false)
  let queue: Promise<unknown> = Promise.resolve()
  let failed = false

  const run = <T>(operation: () => Promise<T>): Promise<T> => {
    const result = queue.then(async () => {
      if (failed) throw new Error(error.value || 'Reload the composition before editing again')
      busy.value = true
      try { return await operation() }
      catch (cause) { failed = true; error.value = String(cause); throw cause }
      finally { busy.value = false }
    })
    queue = result.catch(() => undefined)
    return result
  }

  const accept = (document: CompositionDocument) => {
    documents.value[document.id] = cloneComposition(document)
    return document
  }

  const save = (project: CompositionProject) => {
    const id = project.id
    const content = compositionContent(project)
    return run(async () => {
      const previous = documents.value[id]
      const document = previous
        ? await invoke<CompositionDocument>('edit_overlay_composition', { id, expectedRevision: previous.revision, edit: { type: 'replace', content } })
        : await invoke<CompositionDocument>('create_overlay_composition', { id, content })
      return accept(document)
    })
  }

  const history = (id: string, direction: 'undo' | 'redo') => run(async () => {
    const previous = documents.value[id]
    if (!previous) throw new Error('Composition has not been saved')
    return accept(await invoke<CompositionDocument>('edit_overlay_composition', { id, expectedRevision: previous.revision, edit: { type: direction } }))
  })

  const reload = async (id: string) => {
    await queue
    const document = await invoke<CompositionDocument>('read_overlay_composition', { id })
    failed = false; error.value = ''
    return accept(document)
  }

  const remove = (id: string) => run(async () => {
    const previous = documents.value[id]
    if (!previous) throw new Error('Composition has not been saved')
    await invoke('delete_overlay_composition', { id, expectedRevision: previous.revision })
    documents.value = Object.fromEntries(Object.entries(documents.value).filter(([key]) => key !== id))
  })

  const load = async (legacy: CompositionProject[]) => {
    try {
    const saved = await invoke<CompositionDocument[]>('list_overlay_compositions')
    saved.forEach(accept)
    // Existing native revisions win over old browser copies; migrate each
    // absent legacy project intact, without rescaling its geometry.
    for (const project of legacy) if (!documents.value[project.id]) await save(project)
    return Object.values(documents.value).map(compositionProject)
    } catch (cause) { failed = true; error.value = String(cause); throw cause }
  }

  return { documents, error, busy, accept, save, history, reload, load, remove, flush: () => queue }
}
