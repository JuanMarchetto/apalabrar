// Type declaration for Vite's `?url` asset import suffix. Editor-bridge is
// consumed by `@apalabrar/app`, which IS bundled by Vite; the suffix is
// processed at the consumer's bundle step. We declare it here (instead of
// pulling in `vite/client`) so editor-bridge does not need vite as a dep.

declare module '*?url' {
  const src: string;
  export default src;
}
