import * as utils from "./utils";

const DEFAULT_MEMORY_DESC = { initial: 1024, maximum: 2048 };

export class Memory {
  readonly Memory: WebAssembly.Memory;

  constructor(
    memory:
      | WebAssembly.Memory
      | WebAssembly.MemoryDescriptor = DEFAULT_MEMORY_DESC
  ) {
    if (memory instanceof WebAssembly.Memory) {
      this.Memory = memory;
    } else {
      this.Memory = new WebAssembly.Memory(memory);
    }
  }

  /** Access to memories buffer */
  get memory(): Uint8Array {
    return new Uint8Array(this.Memory.buffer);
  }

  // Returns whether the memory interval is completely inside the smart contract memory.
  fits_memory(offset: number, len: number) {
    return utils.toNum(offset) + utils.toNum(len) < this.memory.length;
  }

  // Reads the content of the given memory interval.
  //
  // # Panics
  //
  // If memory interval is outside the smart contract memory.
  read_memory(offset: number, buffer: Buffer) {
    offset = utils.toNum(offset);
    buffer.set(this.memory.slice(offset, offset + buffer.length), 0);
  }

  // Reads a single byte from the memory.
  //
  // # Panics
  //
  // If pointer is outside the smart contract memory.
  read_memory_u8(offset: number) {
    this.memory[utils.toNum(offset)];
  }

  // Writes the buffer into the smart contract memory.
  //
  // # Panics
  //
  // If `offset + buffer.len()` is outside the smart contract memory.
  write_memory(offset: number, buffer: Buffer) {
    this.memory.set(buffer, utils.toNum(offset));
  }

  set(arr: Uint8Array, offset: number) {
    this.memory.set(arr, offset);
  }

  slice(ptr: number, len: number) {
    return this.memory.slice(ptr, len);
  }

  readUTF8Str(ptr: number) {
    let arr = [];
    const mem = this.memory;
    while (mem[ptr] != 0) {
      arr.push(mem[ptr]);
      ptr++;
    }
    return utils.UTF8toStr(arr);
  }
}
