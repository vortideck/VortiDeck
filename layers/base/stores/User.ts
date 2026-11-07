import { defineStore } from "pinia";

type Role = {
  id: number | null;
  shortname: string | null;
  name: string | null;
};
// @ts-ignore: Unreachable code error
export const useUserStore = defineStore("User", {
  state: () => ({
    default_login_mode: "carrier",
    username: "",
    firstName: "",
    lastName: "",
    email: "",
    avatar: "",
    token: "",
    isLogged: false, //TODO: false
    is_activated: false,
    activated_by: null,
    accountType: "arrow-employee", //arrow_employee, carrier
    role: {
      id: null,
      shortname: null,
      name: null,
    } as Role,
  }),
  getters: {
    getUser(state) {
      return {
        username: state.username,
        email: state.email,
        token: state.token,
        isLogged: state.isLogged,
        accountType: state.accountType,
      };
    },
    getUsername(state) {
      return state.username;
    },
    getEmail(state) {
      return state.email;
    },
    getToken(state) {
      return state.token;
    },
    getIsLogged(state) {
      return state.isLogged;
    },
    getAccountType(state) {
      return state.accountType;
    },
    // getFamily(state) {
    //   if (state.accountType === "carrier") {
    //     return state.carrier.name;
    //   } else {
    //     return "Arrow";
    //   }
    // },
    getRole(state) {
      if (state.role === null) return "null";
      else return state.role.shortname;
    },
  },
  actions: {
    setUsername(username: string) {
      this.username = username;
    },
    setEmail(email: string) {
      this.email = email;
    },
    setToken(token: string) {
      this.token = token;
    },
    setIsLogged(bool: boolean) {
      this.isLogged = bool;
    },
    logout() {
      console.log("logging out");
      this.username = "";
      this.email = "";
      this.token = "";
      this.isLogged = false;
      this.accountType = "";
      //this.role = null;
      this.avatar = "";
      this.is_activated = false;
      this.activated_by = null;
      this.firstName = "";
      this.lastName = "";
      this.default_login_mode = "carrier";
      this.role = {
        id: null,
        shortname: null,
        name: null,
      } as Role;
    },

    setUser(data: any) {
      const user = data.user;

      // if (account_type === "carrier") {
      //   const carrier = data.member.carrier_family;
      //   this.carrier.id = carrier.id;
      //   this.carrier.name = carrier.name;
      //   this.carrier.phone = carrier.phone;
      //   this.carrier.city = carrier.city;
      //   this.carrier.icon_url = carrier.icon_url;
      //   this.carrier.is_activated = carrier.is_activated;
      // }
      this.firstName = user.first_name;
      this.lastName = user.last_name;
      //this.role = data.member.role;
      this.accountType = user.type;
      this.username = user.username;
      this.email = user.email;
      this.token = data.key ? data.key : data.token;
      this.isLogged = true;
      this.is_activated = user.is_activated;
      this.activated_by = user.activated_by;
      this.avatar = user.avatar;
    },
  },

  persist: {
    storage: piniaPluginPersistedstate.cookies(),
  },
});
