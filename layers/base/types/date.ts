interface DateObject {
  day: number;
  id: number;
  date: Date;
  is_open: boolean;
  workable_times: Array<string>;
}

export { DateObject };
