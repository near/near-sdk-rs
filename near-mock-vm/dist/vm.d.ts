/// <reference lib="dom" />
import { VM } from "../pkg";
import { Memory } from "./memory";
export declare class NearVM {
    vm: VM;
    wasm: any | null;
    memory: Memory;
    gas: number;
    constructor(memory: Memory, contextPath?: string);
    static create(memory?: WebAssembly.Memory, contextPath?: string): NearVM;
    static instrumentBinary(binary: Uint8Array): Uint8Array;
    readUTF8Str(ptr: number): string;
    createImports(): any;
    static run(binary: Uint8Array, method: string, input: string): any;
}
