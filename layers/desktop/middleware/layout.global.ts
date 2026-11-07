import { useUserStore } from "@/stores/User";
import { useCookiesStore } from "@/stores/CookiesSettings";

export default defineNuxtRouteMiddleware((to) => {
  if (to.path === "/deck/simulator-v2") {
    return navigateTo({ path: "/deck/simulator", query: to.query }, { replace: true });
  }

  const cookiesStore = useCookiesStore();
  if (
    to.path.startsWith("/waiting-list") ||
    to.path.startsWith("/login") ||
    to.path.startsWith("/sign-up") ||
    to.path.startsWith("/splashscreen")
  ) {
    setPageLayout("clean");
    console.log("set layout to cleap");

    // console.log("current layout:" + this.$nuxt.$data.layoutName);
  } else if (to.path.startsWith("/logs") ) {
    setPageLayout("logs")
  } else if (to.path.startsWith("/overlay")) {
    setPageLayout("overlay");
  }
  else if (to.path.startsWith("/flow")) {
    setPageLayout("flow-editor");
  }
  
  else if (to.path.startsWith("/deck")) {
    setPageLayout("deck-editor");
    if (to.path === "/deck/simulator" || to.path === "/deck/simulator-v2") {
      setPageLayout("simulator");
    } 
    if (to.path.startsWith("/deck/new")) {
      setPageLayout("new-deckitem")
    } 
 
  }  else if(to.path.startsWith("/assistant-window")) {
    setPageLayout(false)
  } else {
    // setPageLayout("default");
    // console.log("set layout to main-app");
  }
});
