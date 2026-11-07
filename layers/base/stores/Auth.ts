

type TokenObject = {
  token: string;
  expire?: string;
  [key: string]: string | undefined;
}

const parseToken = (value: string): TokenObject => {
  const [rawToken = "", ...attributes] = value.split(";");
  const parsed: TokenObject = { token: rawToken.trim() };

  for (const attribute of attributes) {
    const separator = attribute.indexOf("=");
    if (separator < 0) continue;
    const key = attribute.slice(0, separator).trim().toLowerCase().replace(/[\s-]/g, "");
    const attributeValue = attribute.slice(separator + 1).trim();
    if (key) parsed[key] = attributeValue;
  }

  return parsed;
};
export const useAuthStore = defineStore("authStore", {
  state: () => {
    return {
      accessToken: null as TokenObject | null,


      refreshToken: null as TokenObject | null,
    };




  },
  getters: {
    getAccessToken(state) {
      return state.accessToken;
    },
    getRefreshToken(state) {
      return state.refreshToken;
    },
  },

  actions: {
    setAccessToken(token: string) {
      this.accessToken = parseToken(token);
    },

    setRefreshToken(token: string) {
      this.refreshToken = parseToken(token);
    },

    logout() {
      this.accessToken = null;
      this.refreshToken = null;
    },


  },
 
  // other options...

});
