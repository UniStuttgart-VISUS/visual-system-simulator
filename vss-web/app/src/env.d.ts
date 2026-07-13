/// <reference types="vite/client" />
declare module './wasm/vss_web.js' {
  export default function init(): Promise<unknown>
  export class Simulator { static create(parent: string): Promise<Simulator>; post_frame(pixels: Uint8Array, width: number, height: number, rgbd: boolean): void; post_settings(settings: string): void; resize(): void; destroy(): void; free(): void }
  export function catalog(locale: string): string
  export function compose_settings(locale: string, presets: string, overrides: string): string
}
