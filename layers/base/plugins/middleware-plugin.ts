export default defineNuxtPlugin((nuxtApp) => {
  // now available on `nuxtApp.$injected`
  nuxtApp.provide("injected", () => "my injected function");
  // You can alternatively use this format, which comes with automatic type support
});
