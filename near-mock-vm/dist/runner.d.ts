/// <reference lib="dom" />
import { VM } from "../pkg/near_mock_vm";
import { Memory } from "./memory";
declare type stringKeys = {
    [key: number]: () => void;
} & any;
export declare class VMRunner {
    vm: VM;
    wasm: stringKeys | null;
    memory: Memory;
    gas: number;
    constructor(memory: Memory, contextPath?: string);
    static create(memory?: WebAssembly.Memory, contextPath?: string): VMRunner;
    static instrumentBinary(binary: Uint8Array): Uint8Array;
    readUTF8Str(ptr: number): string;
    createImports(): any;
    run(method: string, input: string): void;
    static setup(binary: Uint8Array, contextPath?: string, memory?: WebAssembly.Memory): VMRunner;
    outcome(): any;
    created_receipts(): any;
    static run(binary: Uint8Array, method: string, input: string, contextPath?: string): any;
}
export {};
