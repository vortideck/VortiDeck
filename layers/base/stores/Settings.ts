type WorkableTimes = {
  from: string;
  to: string;
};

export const useSettingsStore = defineStore("settingsStore", {
  state: () => {
    return {
      workable_hours: {
        from: "9:00",
        to: "17:00",
      } as WorkableTimes,
      saturday_workable_hours: {
        from: "9:00",
        to: "13:00",
      } as WorkableTimes,
      sunday_workable_hours: {
        from: "9:00",
        to: "13:00",
      } as WorkableTimes,
      is_open: false,
      is_saturday_open: false,
      is_sunday_open: false,
      max_trucks_per_hour: 0,
    };
  },
  getters: {
    getWorkableHours(state) {
      return state.workable_hours;
    },
    getSaturdayWorkableHours(state) {
      return state.saturday_workable_hours;
    },
    getSundayWorkableHours(state) {
      return state.sunday_workable_hours;
    },
  },
  actions: {
    setSettings(settings: any) {
      const convertTimeFormat = (time) => {
        const [hours, minutes] = time.split(":");
        return `${parseInt(hours)}:${minutes}`;
      };

      console.log(settings);
      this.workable_hours = {
        from: convertTimeFormat(settings.workable_open_time),
        to: convertTimeFormat(settings.workable_end_time),
      } as WorkableTimes;
      this.saturday_workable_hours = {
        from: convertTimeFormat(settings.saturday_open_time),
        to: convertTimeFormat(settings.saturday_end_time),
      } as WorkableTimes;
      this.sunday_workable_hours = {
        from: convertTimeFormat(settings.sunday_open_time),
        to: convertTimeFormat(settings.sunday_end_time),
      } as WorkableTimes;
      this.is_open = settings.is_open;
      this.is_saturday_open = settings.is_saturday_open;
      this.is_sunday_open = settings.is_sunday_open;
      this.max_trucks_per_hour = settings.max_trucks_per_hour;
    },
  },
  persist: {
    storage: piniaPluginPersistedstate.sessionStorage(),
  },
  // other options...
});
