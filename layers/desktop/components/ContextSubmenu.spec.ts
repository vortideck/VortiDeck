import { afterEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { nextTick } from "vue";
import ContextSubmenu from "./ContextSubmenu.vue";

afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = "";
});

describe("ContextSubmenu", () => {
  it("stays open while interacting with a portalled context-menu control", async () => {
    vi.useFakeTimers();
    const wrapper = mount(ContextSubmenu, {
      attachTo: document.body,
      props: { closeDelay: 140 },
      slots: {
        trigger: "<button type='button'>Open</button>",
        default: "<button type='button'>Menu action</button>",
      },
    });

    await wrapper.find(".context-submenu-trigger").trigger("pointerenter");
    await nextTick();
    const menu = document.body.querySelector(".context-submenu");
    expect(menu).not.toBeNull();

    const searchInput = document.createElement("input");
    menu!.appendChild(searchInput);
    await wrapper.find(".context-submenu-trigger").trigger("pointerleave");
    searchInput.focus();
    await vi.advanceTimersByTimeAsync(200);
    expect(document.body.querySelector(".context-submenu")).not.toBeNull();

    // Focus can leave the trigger before a teleported picker has mounted.
    await wrapper.find(".context-submenu-trigger").trigger("pointerleave");
    const floatingControl = document.createElement("div");
    floatingControl.className = "context-menu-floating";
    document.body.appendChild(floatingControl);
    await vi.advanceTimersByTimeAsync(200);
    expect(document.body.querySelector(".context-submenu")).not.toBeNull();

    floatingControl.dispatchEvent(new MouseEvent("pointerover", { bubbles: true }));
    floatingControl.dispatchEvent(new MouseEvent("pointerout", { bubbles: true, relatedTarget: null }));
    floatingControl.dispatchEvent(new Event("scroll", { bubbles: false }));
    floatingControl.dispatchEvent(new MouseEvent("pointerdown", { bubbles: true }));
    await vi.advanceTimersByTimeAsync(200);

    expect(document.body.querySelector(".context-submenu")).not.toBeNull();

    floatingControl.remove();
    await wrapper.find(".context-submenu-trigger").trigger("pointerleave");
    await vi.advanceTimersByTimeAsync(200);
    expect(document.body.querySelector(".context-submenu")).toBeNull();
    wrapper.unmount();
  });
});
