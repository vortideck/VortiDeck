import { persistedEditorNodeId, type SerializableEditorNode } from './flowSerialization'

type PreviewNode = Pick<SerializableEditorNode, 'id' | 'persistedDefinition'> & {
  executionState?: {
    status: string
    last_inputs?: Record<string, unknown>
    last_output?: unknown
  }
}

/** Runtime keys are persisted IDs, not the canvas's numeric projection. */
export function applyNodePreviewResults(
  nodes: PreviewNode[],
  results: Record<string, { inputs?: Record<string, unknown>; outputs?: unknown }>,
) {
  for (const node of nodes) {
    const result = results[persistedEditorNodeId(node)]
    if (!result) continue
    node.executionState ??= { status: 'idle' }
    node.executionState.last_inputs = result.inputs
    node.executionState.last_output = result.outputs
  }
}
