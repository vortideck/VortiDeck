

export default defineNuxtPlugin(async (nuxtApp) => {

    await useCheckUserSession();
});
