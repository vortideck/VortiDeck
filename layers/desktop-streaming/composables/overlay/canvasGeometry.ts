import { toRaw } from 'vue'
export interface CanvasDimensions {
  width: number
  height: number
}

export interface CanvasGeometry {
  x: number
  y: number
  width: number
  height: number
  childElements?: Array<{
    relativeX: number
    relativeY: number
    relativeWidth: number
    relativeHeight: number
    [key: string]: unknown
  }>
  [key: string]: unknown
}

export const validateCanvasDimensions = ({ width, height }: CanvasDimensions) => {
  if (!Number.isInteger(width) || !Number.isInteger(height)) {
    throw new Error('Canvas dimensions must be whole pixels')
  }
  if (width < 320 || width > 7680 || height < 320 || height > 4320) {
    throw new Error('Canvas dimensions must be between 320×320 and 7680×4320')
  }
}

/**
 * Fit a composition into a new canvas without distorting widgets. Aspect-ratio
 * changes use a centered contain transform. Coordinates remain floating point
 * so a shrink/grow cycle does not accumulate integer rounding drift.
 */
export const fitCompositionToCanvas = <T extends CanvasGeometry>(
  elements: readonly T[],
  from: CanvasDimensions,
  to: CanvasDimensions,
): T[] => {
  validateCanvasDimensions(from)
  validateCanvasDimensions(to)
  if (from.width === to.width && from.height === to.height) {
    return elements.map(element => structuredClone(toRaw(element)))
  }

  const scale = Math.min(to.width / from.width, to.height / from.height)
  const offsetX = (to.width - from.width * scale) / 2
  const offsetY = (to.height - from.height * scale) / 2
  const precise = (value: number) => Math.round(value * 1_000_000_000) / 1_000_000_000

  return elements.map((source) => {
    const element = structuredClone(toRaw(source))
    element.x = precise(offsetX + source.x * scale)
    element.y = precise(offsetY + source.y * scale)
    element.width = precise(source.width * scale)
    element.height = precise(source.height * scale)
    if (source.childElements) {
      element.childElements = source.childElements.map(child => ({
        ...structuredClone(toRaw(child)),
        relativeX: precise(child.relativeX * scale),
        relativeY: precise(child.relativeY * scale),
        relativeWidth: precise(child.relativeWidth * scale),
        relativeHeight: precise(child.relativeHeight * scale),
      }))
    }
    return element
  })
}
