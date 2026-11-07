import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import Host from "./Host.vue";
import { legacyActionButtonDefinition } from "@base/utils/legacyDeckWidget";
import type { DeckWidgetInstance, WidgetRuntimeState } from "@base/types/deckWidget";

const instance: DeckWidgetInstance = {
  id: "button:1", definitionId: legacyActionButtonDefinition.id, configVersion: 1,
  config: {}, layouts: {}, bindings: [], enabled: true,
};
const failure = (): WidgetRuntimeState => ({
  instanceId: instance.id, revision: 7, status: "error", values: { muted: false },
  error: { code: "IntegrationError", message: "OBS disconnected" },
});
const badge = "[data-widget-error-indicator]";
const mounts: ReturnType<typeof mount>[] = [];
const render = (runtimeState = failure()) => {
  const wrapper = mount(Host, {
    props: { instance, definition: legacyActionButtonDefinition, mode: "interactive", runtimeState },
    slots: { default: '<button @click.stop="() => {}">Control</button>' },
  });
  mounts.push(wrapper);
  return wrapper;
};

describe("transient widget error presentation", () => {
  beforeEach(() => { vi.useFakeTimers(); });
  afterEach(() => { mounts.splice(0).forEach(wrapper => wrapper.unmount()); vi.useRealTimers(); });

  it("dismisses after three seconds without mutating authoritative failure or values", async () => {
    const state = failure();
    const wrapper = render(state);
    expect(wrapper.findAll(badge)).toHaveLength(1);
    await vi.advanceTimersByTimeAsync(2999);
    expect(wrapper.find(badge).exists()).toBe(true);
    await vi.advanceTimersByTimeAsync(1);
    expect(wrapper.find(badge).exists()).toBe(false);
    expect(state).toEqual(failure());
  });

  it("does not keep extending the badge on unrelated state revisions", async () => {
    const state = failure();
    const wrapper = render(state);
    await vi.advanceTimersByTimeAsync(2000);
    await wrapper.setProps({ runtimeState: { ...state, revision: 8, values: { muted: true } } });
    await vi.advanceTimersByTimeAsync(1000);
    expect(wrapper.find(badge).exists()).toBe(false);
  });

  it("clears on pointer/touch activation even if the child stops click bubbling", async () => {
    const wrapper = render();
    await wrapper.find("button").trigger("click");
    expect(wrapper.find(badge).exists()).toBe(false);
    await wrapper.setProps({ runtimeState: { ...failure(), status: "executing" } });
    await wrapper.setProps({ runtimeState: failure() });
    await wrapper.find("button").trigger("pointerdown", { pointerType: "touch" });
    expect(wrapper.find(badge).exists()).toBe(false);
  });

  it("clears on keyboard activation and shows a later failure for a fresh three seconds", async () => {
    const wrapper = render();
    await wrapper.trigger("keydown", { key: "Enter" });
    expect(wrapper.find(badge).exists()).toBe(false);
    expect(wrapper.emitted("intent")).toHaveLength(1);
    await wrapper.setProps({ runtimeState: { ...failure(), status: "executing", error: undefined } });
    await wrapper.setProps({ runtimeState: { ...failure(), revision: 9 } });
    expect(wrapper.find(badge).exists()).toBe(true);
    await vi.advanceTimersByTimeAsync(3000);
    expect(wrapper.find(badge).exists()).toBe(false);
  });

  it("cleans up its timer on unmount and does not invent provider unavailability", async () => {
    const wrapper = render();
    expect(wrapper.text()).not.toContain("Unavailable");
    wrapper.unmount();
    mounts.pop();
    await nextTick();
    expect(vi.getTimerCount()).toBe(0);
  });
});
