/// <reference types="node" />
export declare class Memory {
    Memory: WebAssembly.Memory;
    constructor(memory?: WebAssembly.Memory);
    get memory(): Uint8Array;
    fits_memory(offset: number, len: number): boolean;
    read_memory(offset: number, buffer: Buffer): void;
    read_memory_u8(offset: number): void;
    write_memory(offset: number, buffer: Buffer): void;
    set(arr: Uint8Array, offset: number): void;
    slice(ptr: number, len: number): Uint8Array;
    readUTF8Str(ptr: number): string;
}
