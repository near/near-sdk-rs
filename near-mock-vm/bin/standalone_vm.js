#!/usr/bin/env node
const v8 = require('v8');
v8.setFlagsFromString('--experimental-wasm-bigint');
let VM = require('../dist').NearVM;
let fs = require("fs");
let assert = require("assert");

if (process.argv.length < 4) {
  console.log("Usage: near-vm wasm-file method [input]");
  process.exit(1);
}
const wasmBinary = fs.readFileSync(process.argv[2]);
const method = process.argv[3];
let input = "";
if (process.argv.length >= 5) {
  input = process.argv[4];
}

VM.run(wasmBinary, method, input);

