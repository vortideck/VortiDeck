import { mount } from "@vue/test-utils";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import NotificationItem from "./item.vue";
import { useMainStore } from "@base/stores/Main";

vi.mock("~icons/pixelarticons/close?width=24px&height=24px", () => ({
  default: {
    name: "PixelarticonsClose",
    render: () => null,
  },
}));

describe("NotificationItem Component", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders title, message, and type class correctly", () => {
    const wrapper = mount(NotificationItem, {
      props: {
        notification: {
          id: 1,
          title: "Action Assigned",
          message: "Action added to button",
          type: "success",
          buttonText: "",
          autoClose: true,
          duration: 5000,
          color: "",
        },
      },
    });

    expect(wrapper.find(".notification-title").text()).toBe("Action Assigned");
    expect(wrapper.find(".notification-message").text()).toBe("Action added to button");
    expect(wrapper.classes()).toContain("notification-success");
  });

  it("triggers closeNotification on store when close button clicked", async () => {
    const store = useMainStore();
    const closeSpy = vi.spyOn(store, "closeNotification");

    const wrapper = mount(NotificationItem, {
      props: {
        notification: {
          id: 123,
          title: "Dismiss Me",
          message: "Sample",
          type: "info",
          buttonText: "",
          autoClose: false,
          duration: 0,
          color: "",
        },
      },
    });

    await wrapper.find(".notification-close").trigger("click");
    expect(closeSpy).toHaveBeenCalledWith(123);
  });

  it("renders action button and triggers its function on click", async () => {
    const buttonCallback = vi.fn();

    const wrapper = mount(NotificationItem, {
      props: {
        notification: {
          id: 2,
          title: "Confirm Action",
          message: "Click to proceed",
          type: "warning",
          buttonText: "Execute",
          buttonFunction: buttonCallback,
          autoClose: false,
          duration: 0,
          color: "",
        },
      },
    });

    const btn = wrapper.find(".notification-button");
    expect(btn.exists()).toBe(true);
    expect(btn.text()).toBe("Execute");

    await btn.trigger("click");
    expect(buttonCallback).toHaveBeenCalledTimes(1);
  });
});
