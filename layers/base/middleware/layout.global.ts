import { useUserStore } from "@base/stores/User";
import { useCookiesStore } from "@base/stores/CookiesSettings";

export default defineNuxtRouteMiddleware((to) => {
  const cookiesStore = useCookiesStore();
  if (
    to.path.startsWith("/waiting-list") ||
    to.path.startsWith("/login") ||
    to.path.startsWith("/sign-up")
  ) {
    setPageLayout("clean");
    console.log("set layout to cleap");

    // console.log("current layout:" + this.$nuxt.$data.layoutName);
  } else {
    setPageLayout("main-app");
    console.log("set layout to main-app");
  }
});
