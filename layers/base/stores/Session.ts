export const useSessionStore = defineStore("User", {
  state: () => {
    return {
      messages: [],
      userWantsSendMessage: false,
    };
  },
  getters: {
    getMessages(state) {
      return state.messages;
    },
    getuserWantsSendMessage(state) {
      return state.userWantsSendMessage;
    },
  },
  actions: {
    setuserWantsSendMessage(bool: boolean) {
      this.userWantsSendMessage = bool;
    },
    addMessage(message: Object) {
      this.messages = [...this.messages, message];
    },
  },
  persist: {
    storage: piniaPluginPersistedstate.sessionStorage(),
    // only save isCookiesSet as cookie
  },
});
