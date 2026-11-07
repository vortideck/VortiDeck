import { describe, expect, it } from 'vitest'
import {
  buildComponentVars,
  componentRegistry,
  migrateLegacyComponentStyle,
} from './componentSchema'
import { normalizeWidgetStreamEvent } from './widgetStreamProtocol'

const FIRST_PARTY_WIDGETS = [
  'chat',
  'goal',
  'follower-counter-gradient',
  'donation-goal-neon',
  'subscriber-milestone',
  'recent',
  'timer',
  'counter',
] as const

const STANDARD_ALERTS = [
  'follower',
  'subscriber',
  'donation',
  'raid',
  'host',
] as const

describe('first-party overlay widgets', () => {
  it('keeps every package schema customizable and self-contained', () => {
    for (const id of FIRST_PARTY_WIDGETS) {
      const schema = componentRegistry[id]
      expect(schema, id).toBeDefined()
      expect(schema.template.length, id).toBeGreaterThan(100)
      expect(schema.width, id).toBeGreaterThan(0)
      expect(schema.height, id).toBeGreaterThan(0)
      expect(schema.editableProps.length, id).toBeGreaterThan(2)
    }
  })

  it('renders chat content with DOM text nodes instead of untrusted HTML', () => {
    const template = componentRegistry.chat.template
    expect(template).toContain('document.createTextNode')
    expect(template).toContain('author.textContent')
    expect(template).not.toContain('div.innerHTML')
  })

  it('gives all event-driven widgets a canonical stream-event listener', () => {
    for (const id of [
      'chat',
      'goal',
      'follower-counter-gradient',
      'donation-goal-neon',
      'subscriber-milestone',
      'recent',
    ]) {
      expect(componentRegistry[id].template, id).toContain('vortideck-stream-event')
    }
  })

  it('ships syntactically valid inline widget programs', () => {
    for (const id of FIRST_PARTY_WIDGETS) {
      const scripts = [...componentRegistry[id].template.matchAll(/<script>([\s\S]*?)<\/script>/g)]
      expect(scripts.length, id).toBeGreaterThan(0)
      for (const [, source] of scripts) {
        expect(() => new Function(source), id).not.toThrow()
      }
    }
  })

  it('keeps standard alerts responsive, event-aware, and free of emoji artwork', () => {
    for (const id of STANDARD_ALERTS) {
      const schema = componentRegistry[id]
      expect(schema.template, id).toContain('width:100%')
      expect(schema.template, id).toContain('vortideck-stream-event')
      expect(schema.template, id).not.toMatch(/[\u{1F300}-\u{1FAFF}]/u)

      const scripts = [...schema.template.matchAll(/<script>([\s\S]*?)<\/script>/g)]
      expect(scripts.length, id).toBe(1)
      expect(() => new Function(scripts[0][1]), id).not.toThrow()
    }
  })

  it('uses responsive themed primitives for camera, brand, and social placeholders', () => {
    for (const id of ['video', 'webcam', 'webcam-frame', 'logo', 'social']) {
      const schema = componentRegistry[id]
      expect(schema.template, id).toContain('width:100%')
      expect(schema.template, id).toContain('height:100%')
      expect(schema.template, id).not.toMatch(/[\u{1F300}-\u{1FAFF}]/u)
      expect(schema.editableProps.length, id).toBeGreaterThan(2)
    }
  })

  it('keeps every built-in renderer free of emoji artwork and hardcoded palette defaults', () => {
    for (const [id, schema] of Object.entries(componentRegistry)) {
      expect(schema.template, id).not.toMatch(/[\u{1F300}-\u{1FAFF}]/u)

      for (const [name, definition] of Object.entries(schema.cssVars)) {
        if (['--c-primary', '--c-secondary', '--c-accent'].includes(name)) {
          expect(definition.default, `${id}:${name}`).toMatch(/^var\(--theme-/)
        }
      }
    }
  })

  it('migrates only the obsolete fireball defaults back to theme ownership', () => {
    const legacy = { '--c-primary': '#ff4400', '--c-accent': '#ffff00' }
    expect(migrateLegacyComponentStyle('webgl-fireballs', legacy)).toBe(true)
    expect(legacy).toEqual({
      '--c-primary': 'var(--theme-primary)',
      '--c-accent': 'var(--theme-accent)',
    })

    const customized = { '--c-primary': '#123456', '--c-accent': '#abcdef' }
    expect(migrateLegacyComponentStyle('webgl-fireballs', customized)).toBe(false)
    expect(customized).toEqual({ '--c-primary': '#123456', '--c-accent': '#abcdef' })

    expect(buildComponentVars(componentRegistry['webgl-fireballs'], {
      '--c-primary': '#ff4400',
      '--c-accent': '#ffff00',
    })).toMatchObject({
      '--c-primary': 'var(--theme-primary)',
      '--c-accent': 'var(--theme-accent)',
    })
  })

  it('normalizes the desktop relay envelope to the OBS widget protocol', () => {
    expect(normalizeWidgetStreamEvent({
      type: 'chat_message',
      account_id: 42,
      platform: 'twitch',
      data: {
        id: 'message-1',
        user_name: 'viewer',
        user_display_name: 'Viewer',
        platform_color: '#9146ff',
        message: 'hello',
      },
    })).toMatchObject({
      type: 'chatMessage',
      accountId: 42,
      userName: 'viewer',
      userDisplayName: 'Viewer',
      platformColor: '#9146ff',
      message: 'hello',
    })
  })
})
