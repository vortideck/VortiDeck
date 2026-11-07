import { describe, expect, it } from 'vitest'
import { mediaData, mediaTemplate } from './mediaTemplate'

function image(data: Record<string, unknown>) {
  const frame = document.createElement('iframe')
  document.body.appendChild(frame)
  const win = frame.contentWindow! as Window & typeof globalThis
  win.document.body.innerHTML = mediaTemplate('image')
  const script = win.document.querySelector('script')!
  Object.defineProperty(win.document, 'currentScript', { configurable: true, value: script })
  ;(win as unknown as Record<string, unknown>).vortideckData = data
  win.eval(script.textContent!)
  return { frame, win, media: win.document.querySelector('img')!, status: win.document.querySelector('.media-status')!,
    update: (detail: Record<string, unknown>) => win.dispatchEvent(new win.CustomEvent('vortideck-data-update', { detail })) }
}

describe('shared image renderer', () => {
  it('keeps the source when changing appearance, changes it explicitly, and reports actual load errors', () => {
    const rendered = image({ src: 'https://example.com/logo.png', objectFit: 'contain' })
    try {
      expect(rendered.media.getAttribute('src')).toBe('https://example.com/logo.png')
      rendered.media.dispatchEvent(new rendered.win.Event('load'))
      expect(rendered.media.hidden).toBe(false)
      rendered.update({ objectFit: 'cover', alt: 'My channel' })
      expect(rendered.media.style.objectFit).toBe('cover')
      expect(rendered.media.alt).toBe('My channel')
      expect(rendered.media.hidden).toBe(false)
      rendered.update({ src: 'https://example.com/new.png' })
      expect(rendered.media.hidden).toBe(true)
      rendered.media.dispatchEvent(new rendered.win.Event('error'))
      expect(rendered.status.textContent).toContain('could not be loaded')
      rendered.update({ src: '' })
      expect(rendered.media.hasAttribute('src')).toBe(false)
      expect(rendered.status.textContent).toBe('Set a image source')
    } finally { rendered.frame.remove() }
  })

  it('accepts embedded raster images and rejects executable or machine-local sources', () => {
    const rendered = image({ src: 'data:image/png;base64,aGVsbG8=' })
    try {
      expect(rendered.media.getAttribute('src')).toBe('data:image/png;base64,aGVsbG8=')
      for (const src of ['javascript:alert(1)', 'file:///C:/private.png', 'data:image/svg+xml,<svg onload="bad()"/>', 'httpsx://example.com/a']) {
        rendered.update({ src })
        expect(rendered.media.hasAttribute('src')).toBe(false)
        expect(rendered.media.hidden).toBe(true)
      }
    } finally { rendered.frame.remove() }
    expect(mediaData({ schemaId: 'image', imageUrl: 'https://example.com/old.png', data: { objectFit: 'cover' } }, { src: '', objectFit: 'contain' }))
      .toEqual({ src: 'https://example.com/old.png', objectFit: 'cover' })
    expect(mediaData({ schemaId: 'image', imageUrl: 'https://example.com/old.png', data: { src: '' } }).src).toBe('')
  })
})
