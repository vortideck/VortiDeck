import { afterEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import { flushPromises } from "@vue/test-utils";
import { invoke } from "@tauri-apps/api/core";
import ActionPicker, { invalidateActionCache } from "./ActionPicker.vue";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

afterEach(() => {
  vi.restoreAllMocks();
  document.body.innerHTML = "";
  invalidateActionCache();
});

describe("ActionPicker", () => {
  it("shows only the selected action when a binding already has a value", async () => {
    vi.mocked(invoke).mockImplementation(async (command) => (
      command === "get_available_actions"
        ? [{ id: "mirror", name: "Mirror" }]
        : []
    ));
    const wrapper = mount(ActionPicker, {
      attachTo: document.body,
      props: {
        modelValue: { type: "system", id: "mirror" },
        placeholder: "Replace bound action…",
      },
    });
    await flushPromises();

    expect(wrapper.find(".multiselect__single").text()).toBe("Mirror");
    expect(wrapper.find<HTMLInputElement>(".multiselect__input").attributes("placeholder")).toBe("");
    expect(wrapper.classes()).toContain("action-picker-wrap--selected");

    wrapper.unmount();
  });

  it("opens from the full visual field without leaking the click to parent menus", async () => {
    const outsideMouseDown = vi.fn();
    document.addEventListener("mousedown", outsideMouseDown);
    const wrapper = mount(ActionPicker, { attachTo: document.body });

    expect(wrapper.find(".multiselect").classes()).not.toContain("multiselect--active");
    await wrapper.find(".multiselect__tags").trigger("mousedown");
    await nextTick();
    await nextTick();

    expect(wrapper.find(".multiselect").classes()).toContain("multiselect--active");
    expect(outsideMouseDown).not.toHaveBeenCalled();
    await vi.waitFor(() => {
      expect(document.body.querySelector(".action-picker-dropdown--positioned")).not.toBeNull();
    });

    const input = wrapper.find<HTMLInputElement>(".multiselect__input");
    await input.trigger("mousedown");
    await input.trigger("click");
    await input.setValue("discord");
    expect(wrapper.find(".multiselect").classes()).toContain("multiselect--active");
    expect(input.element.value).toBe("discord");

    wrapper.unmount();
    document.removeEventListener("mousedown", outsideMouseDown);
  });

  it("keeps its initial side while open when viewport events recalculate its position", async () => {
    const wrapper = mount(ActionPicker, { attachTo: document.body });
    const picker = wrapper.find(".action-picker-wrap").element as HTMLElement;
    const rect = (top: number, bottom: number) => ({
      top,
      bottom,
      left: 40,
      right: 280,
      width: 240,
      height: bottom - top,
      x: 40,
      y: top,
      toJSON: () => ({}),
    } as DOMRect);
    const getBounds = vi.spyOn(picker, "getBoundingClientRect").mockReturnValue(rect(650, 688));

    await wrapper.find(".multiselect__tags").trigger("mousedown");
    await vi.waitFor(() => {
      expect(document.body.querySelector(".action-picker-dropdown--above")).not.toBeNull();
    });

    getBounds.mockReturnValue(rect(50, 88));
    window.dispatchEvent(new Event("resize"));

    const dropdown = document.body.querySelector(".action-picker-dropdown");
    expect(dropdown?.classList.contains("action-picker-dropdown--above")).toBe(true);
    expect(dropdown?.classList.contains("action-picker-dropdown--below")).toBe(false);
    const originalTop = (dropdown as HTMLElement).style.getPropertyValue("top");

    // vue-multiselect patches these inline properties again when its option
    // pointer changes. The picker must restore its geometry before paint.
    (dropdown as HTMLElement).style.setProperty("position", "absolute");
    (dropdown as HTMLElement).style.setProperty("top", "999px");
    await vi.waitFor(() => {
      expect((dropdown as HTMLElement).style.getPropertyValue("position")).toBe("fixed");
      expect((dropdown as HTMLElement).style.getPropertyPriority("position")).toBe("important");
      expect((dropdown as HTMLElement).style.getPropertyValue("top")).toBe(originalTop);
      expect((dropdown as HTMLElement).style.getPropertyPriority("top")).toBe("important");
    });
    wrapper.unmount();
  });
});
