// Type fix for ArrayBufferLike compatibility with Stellar SDK and Web Crypto API
// This resolves the TypeScript error where ArrayBufferLike (ArrayBuffer | SharedArrayBuffer)
// is not assignable to ArrayBuffer in strict type checking environments

declare global {
  // Extend the global ArrayBuffer interface to be compatible with ArrayBufferLike
  interface ArrayBuffer {
    readonly [Symbol.toStringTag]: 'ArrayBuffer';
  }
  
  // Ensure Uint8Array is compatible with BufferSource
  interface Uint8Array {
    readonly buffer: ArrayBuffer;
  }
}

export {};
