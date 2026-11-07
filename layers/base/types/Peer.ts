export type PeerInfo = {
  client_id: string;
  device_type: string; // Keep only serializable fields
  device: string;
};
