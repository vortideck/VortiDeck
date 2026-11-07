import { useUserStore } from "@/stores/User";
export default defineNuxtRouteMiddleware((to) => {
  const userStore = useUserStore();
  console.log(userStore.getIsLogged);
  if (userStore.getIsLogged) {
    return navigateTo("/");
  }
});
