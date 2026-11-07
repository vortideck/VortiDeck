import { createGtm } from "@gtm-support/vue-gtm";
import { useCookiesStore } from "@base/stores/CookiesSettings";

// const { $cookies } = useNuxtApp();
// const gtmCookieGroupName = "marketing";

export default defineNuxtPlugin((nuxtApp) => {
  const cookiesSettings = useCookiesStore();

  nuxtApp.vueApp.use(
    createGtm({
      id: "GTM-KZ86ZXS5",
      defer: false, // defaults to false.
      compatibility: false, // Will add `async` and `defer` to the script tag to not block requests for old browsers that do not support `async`
      // nonce: '2726c7f26c',     // Will be added to script tag as 'nonce' attr
      enabled: true && cookiesSettings.getAnalitycs, // && $cookies.isEnabled(gtmCookieGroupName), // defaults to true. Plugin can be disabled by setting this to false for Ex: enabled: !!GDPR_Cookie (optional)
      debug: true, // Whether or not display console logs debugs (optional)
      loadScript: true, // Whether the GTM <script> tag should be added
      vueRouter: useRouter(), // Pass the router instance to automatically sync with router (optional)
      trackOnNextTick: false, // Whether or not call trackView in Vue.nextTick
    })
  );
});
