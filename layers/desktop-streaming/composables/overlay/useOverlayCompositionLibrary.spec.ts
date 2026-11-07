import { beforeEach, describe, expect, it, vi } from 'vitest'
import { compositionContent, useOverlayCompositionLibrary, type CompositionDocument, type CompositionProject } from './useOverlayCompositionLibrary'

const invoke = vi.hoisted(() => vi.fn())
vi.mock('@tauri-apps/api/core', () => ({ invoke }))

const project = (): CompositionProject => ({ id: 'starting', name: 'Starting', created: 'old', lastModified: 'old',
  elements: [{ id: 'widget', x: 17, y: 21, width: 300, height: 400, selected: true, configRevision: 7, config: { color: 'blue' }, extension: { preserved: true } }],
  settings: { canvasWidth: 1920, canvasHeight: 1080, zoom: 100 } })
const document = (revision = 1): CompositionDocument => ({ schemaVersion: 1, id: 'starting', revision, created: 'old', lastModified: 'new',
  content: compositionContent(project()), publishedRevision: null, canUndo: revision > 1, canRedo: false })

describe('shared overlay draft ownership', () => {
  beforeEach(() => invoke.mockReset())
  it('migrates geometry and extension data while native revisions win over browser copies', async () => {
    invoke.mockResolvedValueOnce([document(4)])
    const library = useOverlayCompositionLibrary()
    const legacy = project(); legacy.elements[0]!.x = 99
    const restored = await library.load([legacy])
    expect(restored[0]!.elements[0]!.x).toBe(17)
    expect(invoke).toHaveBeenCalledTimes(1)
    restored[0]!.elements[0]!.extension = { preserved: false }
    expect(library.documents.value.starting!.content.elements[0]!.extension).toEqual({ preserved: true })
  })
  it('serializes edit revisions and does not mutate input or call live publication', async () => {
    invoke.mockResolvedValueOnce(document(1)).mockResolvedValueOnce(document(2))
    const library = useOverlayCompositionLibrary()
    const first = project(); const second = project(); second.elements[0]!.x = 80
    await Promise.all([library.save(first), library.save(second)])
    expect(invoke.mock.calls.map(call => call[0])).toEqual(['create_overlay_composition', 'edit_overlay_composition'])
    expect(invoke.mock.calls[1]![1]).toMatchObject({ expectedRevision: 1, edit: { type: 'replace', content: { elements: [{ x: 80 }] } } })
    expect(first.elements[0]!.configRevision).toBe(7)
    expect(invoke.mock.calls[0]![1].content.elements[0]).not.toHaveProperty('configRevision')
    expect(invoke.mock.calls[0]![1].content.elements[0]).not.toHaveProperty('selected')
  })
  it('stops queued writes after a conflict instead of rebasing over another caller', async () => {
    const library = useOverlayCompositionLibrary(); library.accept(document(1))
    invoke.mockRejectedValueOnce(new Error('Stale composition revision'))
    const results = await Promise.allSettled([library.save(project()), library.save(project())])
    expect(results.every(result => result.status === 'rejected')).toBe(true)
    expect(invoke).toHaveBeenCalledTimes(1)
    expect(library.documents.value.starting!.revision).toBe(1)
    invoke.mockResolvedValueOnce(document(9)).mockResolvedValueOnce(document(10))
    await library.reload('starting'); await library.save(project())
    expect(invoke.mock.calls[2]![1].expectedRevision).toBe(9)
  })
})
