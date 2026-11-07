import { useUserStore } from "@/stores/User";
import { useCookiesStore } from "@/stores/CookiesSettings";

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
  } else if (to.path === "/qr") {
    setPageLayout("qr");
    console.log("set layout to qr");
  } else if (to.path === "/deck") {
    setPageLayout("clean");
    console.log("set layout to deck");
  } else {
    setPageLayout("main-app");
    console.log("set layout to main-app");
  }

 
});


