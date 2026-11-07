import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import Button from "./Button.vue";

vi.mock("@tauri-apps/plugin-haptics", () => ({
  impactFeedback: vi.fn(),
}));

vi.mock("@base/stores/Deck", () => ({
  useDeckStore: () => ({
    getActiveDeck: {
      settings: {
        buttons_padding: 4,
      },
    },
  }),
}));

describe("Deck Dynamic Button", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders a clean tile without raw action name or 'Action' label by default when no icon is provided", () => {
    const wrapper = mount(Button, {
      props: {
        data: {
          position: 1,
          action: "Run MP3 Flow",
          label: "Run MP3 Flow",
        },
        iconUrl: "",
        isAlt: false,
        isShift: false,
        theme: {},
      },
      global: {
        stubs: {
          "nuxt-img": true,
        },
      },
    });

    // Invariant: Blank buttons without custom icons must NOT display action text
    expect(wrapper.find("p").exists()).toBe(false);
    expect(wrapper.text()).toBe("");
    expect(wrapper.text()).not.toContain("Run MP3 Flow");
    expect(wrapper.text()).not.toContain("Action");
  });

  it("displays grid position number when Alt or Shift is held", () => {
    const wrapperAlt = mount(Button, {
      props: {
        data: {
          position: "G1",
          action: "Run MP3 Flow",
        },
        iconUrl: "",
        isAlt: true,
        isShift: false,
        theme: {},
      },
      global: {
        stubs: {
          "nuxt-img": true,
        },
      },
    });

    expect(wrapperAlt.find("p").exists()).toBe(true);
    expect(wrapperAlt.find("p").text()).toBe("G1");

    const wrapperShift = mount(Button, {
      props: {
        data: {
          position: "G2",
          action: "Run MP3 Flow",
        },
        iconUrl: "",
        isAlt: false,
        isShift: true,
        theme: {},
      },
      global: {
        stubs: {
          "nuxt-img": true,
        },
      },
    });

    expect(wrapperShift.find("p").exists()).toBe(true);
    expect(wrapperShift.find("p").text()).toBe("G2");
  });

  it("renders image icon when iconUrl is provided and neither Alt nor Shift is held", () => {
    const wrapper = mount(Button, {
      props: {
        data: {
          position: 1,
          action: "Discord Mute",
        },
        iconUrl: "https://example.com/icon.png",
        isAlt: false,
        isShift: false,
        theme: {},
      },
      global: {
        stubs: {
          "nuxt-img": true,
        },
      },
    });

    expect(wrapper.find("nuxt-img-stub").exists()).toBe(true);
    expect(wrapper.find("p").exists()).toBe(false);
  });
});
