import { fitCompositionToCanvas, validateCanvasDimensions, type CanvasDimensions, type CanvasGeometry } from './canvasGeometry'

/** Persist desired output before touching OBS. A timeout leaves a recoverable
 * saved intent, never an implicit rollback over a newer OBS configuration. */
export async function commitCanvas<T extends CanvasGeometry>(options: {
  from: CanvasDimensions
  to: CanvasDimensions
  elements: readonly T[]
  persist: (elements: T[], canvas: CanvasDimensions) => Promise<boolean>
  commitEditor: (elements: T[], canvas: CanvasDimensions) => void
}) {
  validateCanvasDimensions(options.to)
  const changed = options.from.width !== options.to.width || options.from.height !== options.to.height
  const elements = changed ? fitCompositionToCanvas(options.elements, options.from, options.to) : [...options.elements]
  if (!await options.persist(elements, options.to)) throw new Error('The overlay runtime could not save this canvas')
  options.commitEditor(elements, options.to)
}
