import type { PingPalAPI } from '../../main/preload';

declare global {
  interface Window {
    pingpal: PingPalAPI;
  }
}

export {};
