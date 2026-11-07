import { reactive } from 'vue'
import { describe, expect, it } from 'vitest'
import { fitCompositionToCanvas, validateCanvasDimensions } from './canvasGeometry'

describe('overlay canvas geometry', () => {
  const element = () => ({
    id: 'card', x: 240, y: 120, width: 480, height: 180,
    childElements: [{ relativeX: 10, relativeY: 20, relativeWidth: 100, relativeHeight: 40 }],
  })

  it('round-trips same-aspect resize without mutating or rounding the saved geometry', () => {
    const original = element()
    const smaller = fitCompositionToCanvas([original], { width: 1920, height: 1080 }, { width: 1280, height: 720 })
    const restored = fitCompositionToCanvas(smaller, { width: 1280, height: 720 }, { width: 1920, height: 1080 })
    const restoredElement = restored[0]!
    expect(restoredElement.x).toBeCloseTo(original.x, 8)
    expect(restoredElement.y).toBeCloseTo(original.y, 8)
    expect(restoredElement.width).toBeCloseTo(original.width, 8)
    expect(restoredElement.height).toBeCloseTo(original.height, 8)
    expect(restoredElement.childElements![0]!.relativeWidth).toBeCloseTo(original.childElements[0]!.relativeWidth, 8)
    expect(original).toEqual(element())
  })

  it('uses centered contain scaling for landscape-to-portrait without distorting widgets', () => {
    const [resized] = fitCompositionToCanvas(
      [{ id: 'full', x: 0, y: 0, width: 1920, height: 1080 }],
      { width: 1920, height: 1080 },
      { width: 1080, height: 1920 },
    )
    expect(resized).toMatchObject({ x: 0, y: 656.25, width: 1080, height: 607.5 })
  })

  it('supports empty compositions and rejects invalid OBS dimensions', () => {
    expect(fitCompositionToCanvas([], { width: 1920, height: 1080 }, { width: 3840, height: 2160 })).toEqual([])
    expect(() => validateCanvasDimensions({ width: 319, height: 1080 })).toThrow('between')
    expect(() => validateCanvasDimensions({ width: 1920.5, height: 1080 })).toThrow('whole pixels')
  })
})

it('resizes reactive editor geometry without cloning Vue proxies', () => {
  const elements = reactive([{ x: 10, y: 20, width: 100, height: 50, childElements: [{ relativeX: 5, relativeY: 6, relativeWidth: 20, relativeHeight: 10 }] }])
  const result = fitCompositionToCanvas(elements, { width: 1920, height: 1080 }, { width: 960, height: 540 })
  expect(result[0]).toMatchObject({ x: 5, y: 10, width: 50, height: 25 })
  expect(elements[0]!.width).toBe(100)
})
