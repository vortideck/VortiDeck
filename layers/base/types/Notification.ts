export type NotificationItem = {
  id: number;
  /** Stable correlation key for de-duplication and authoritative cleanup. */
  key?: string;
  title: string;
  message: string;
  buttonText: string;
  /** Runtime-only callback. Functions must never be persisted or cross-window synced. */
  buttonFunction?: () => void;
  autoClose: boolean;
  duration: number;
  type: string;
  color: string;
  windowId?: string;
};

/** Caller-owned notification fields before the store assigns persistence/UI defaults. */
export type NotificationInput = Pick<NotificationItem, "message" | "type"> &
  Partial<Omit<NotificationItem, "message" | "type" | "windowId">> & {
    /** Legacy caller name; normalized to duration by the store. */
    timeout?: number;
  };
