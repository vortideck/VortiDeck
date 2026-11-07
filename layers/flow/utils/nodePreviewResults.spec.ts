import { describe, expect, it } from 'vitest'
import { applyNodePreviewResults } from './nodePreviewResults'
import { createEditorNodeIdProjection } from './flowSerialization'
import type { FlowNodeDefinition } from '../types/flow'

describe('run results in node previews', () => {
  it('hydrates image/text results after saved IDs are projected to numeric canvas IDs', () => {
    const saved = [{ id: 'preview-image-uuid' }, { id: 'display-text-uuid' }] as FlowNodeDefinition[]
    const projection = createEditorNodeIdProjection(saved)
    const nodes = saved.map(persistedDefinition => ({
      id: projection.canvasIdFor(persistedDefinition.id), persistedDefinition,
      executionState: { status: 'idle', last_inputs: {} },
    }))
    const inputs = { imageData: 'C:/downloads/image.png' }
    applyNodePreviewResults(nodes, {
      'preview-image-uuid': { inputs, outputs: { display: inputs } },
      'display-text-uuid': { inputs: { text: 'hello' } },
    })
    expect(nodes[0]!.executionState.last_inputs).toEqual(inputs)
    expect(nodes[1]!.executionState.last_inputs).toEqual({ text: 'hello' })
  })

  it('initializes previews for new nodes and ignores results belonging to other nodes', () => {
    const nodes: Parameters<typeof applyNodePreviewResults>[0] = [{ id: 7 }, { id: 8 }]
    applyNodePreviewResults(nodes, { '7': { inputs: { imageData: 'data:image/png;base64,abc' } }, other: { inputs: {} } })
    expect(nodes[0]!.executionState?.last_inputs?.imageData).toBe('data:image/png;base64,abc')
    expect(nodes[1]!.executionState).toBeUndefined()
  })
})
