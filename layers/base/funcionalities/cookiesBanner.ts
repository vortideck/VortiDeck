type store = any;

export const saveCookiesSettings = (store: any) => {
  const accepted = store.acceptedCookies ?? {};
  store.acceptedCookies = {
    necessary: true,
    functional: Boolean(accepted.functional),
    analytics: Boolean(accepted.analytics),
    performance: Boolean(accepted.performance),
    advertisement: Boolean(accepted.advertisement),
    others: Boolean(accepted.others),
  };

  store.isCookiesSet = true;
};

export const acceptAllCookies = (store: any) => {
  store.acceptedCookies = {
    necessary: true,
    functional: true,
    analytics: true,
    performance: true,
    advertisement: true,
    others: true,
  };
  store.isCookiesSet = true;
};

export const rejectAllCookies = (store: any) => {
  store.acceptedCookies = {
    necessary: true,
    functional: false,
    analytics: false,
    performance: false,
    advertisement: false,
    others: false,
  };
  store.isCookiesSet = true;
};
