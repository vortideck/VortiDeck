import { useGtm as useVueGTM } from "@gtm-support/vue-gtm";

export const useGtm = () => {
  const gtm = useVueGTM();
  return gtm;
};
