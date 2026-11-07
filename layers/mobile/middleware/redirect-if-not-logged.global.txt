import { useUserStore } from "@/stores/User";
export default defineNuxtRouteMiddleware((from, to) => {
  const userStore = useUserStore();
  console.log(userStore.getIsLogged);
  if (userStore.getIsLogged) {
    console.log("logged: true");
    if (from.path === "/login") return "/";
    if (from.path === "/sign-up") return "/";
    if (from.path === "/waiting-list") return "/";
  }

  if (!userStore.getIsLogged) {
    console.log("logged: false");
    if (from.path === "/login") return;
    if (from.path === "/sign-up") return;
    return "/login";
    // if (from.path === "/forgot-password") return;
    // if (from.path === "/waiting-list") return;
    // return "/waiting-list";
  }
});
