import { describe, expect, it } from 'vitest'
import { mount } from '@vue/test-utils'
import WidgetSettingsForm from './WidgetSettingsForm.vue'

describe('widget settings form', () => {
  it('preserves untouched values and sends typed numeric and boolean edits atomically', async () => {
    const wrapper = mount(WidgetSettingsForm, { props: {
      schema: { type: 'object', properties: { goal: { type: 'integer' }, enabled: { type: 'boolean' } } },
      config: { goal: 100, enabled: false, untouched: 'keep' },
    } })
    await wrapper.find('input[type=number]').setValue('200')
    await wrapper.find('input[type=checkbox]').setValue(true)
    expect(wrapper.emitted('apply')).toBeUndefined()
    await wrapper.find('form').trigger('submit')
    expect(wrapper.emitted('apply')?.[0]).toEqual([{ goal: 200, enabled: true, untouched: 'keep' }, 0])
  })
  it('edits nested settings as JSON and rejects invalid JSON without emitting a write', async () => {
    const wrapper = mount(WidgetSettingsForm, { props: {
      schema: { type: 'object', properties: { items: { type: 'array' } } }, config: { items: [] },
    } })
    await wrapper.find('textarea').setValue('[]')
    await wrapper.find('form').trigger('submit')
    expect(wrapper.emitted('apply')).toBeUndefined()
    expect(wrapper.find('[role=alert]').text()).toContain('JSON object')
    await wrapper.find('textarea').setValue('{"items":[{"label":"One"}]}')
    await wrapper.find('form').trigger('submit')
    expect(wrapper.emitted('apply')?.[0]).toEqual([{ items: [{ label: 'One' }] }, 0])
  })
  it('retains the revision of a dirty draft when another editor or agent saves', async () => {
    const wrapper = mount(WidgetSettingsForm, { props: {
      schema: { type: 'object', properties: { goal: { type: 'integer' } } }, config: { goal: 100 }, revision: 1,
    } })
    await wrapper.find('input').setValue('200')
    await wrapper.setProps({ config: { goal: 300 }, revision: 2 })
    expect(wrapper.find('[role=status]').exists()).toBe(true)
    await wrapper.find('form').trigger('submit')
    expect(wrapper.emitted('apply')?.[0]).toEqual([{ goal: 200 }, 1])
  })
})
