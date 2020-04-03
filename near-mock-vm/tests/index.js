const v8 = require('v8');
v8.setFlagsFromString('--experimental-wasm-bigint');

let VM = require('../dist').NearVM;
let fs = require("fs");
let assert = require("assert");
// const binaryen = require("binaryen");



// binaryen.ready.then((b) => {
  let bin = fs.readFileSync(__dirname + "/../out/main.wasm");
  console.log(bin.length)
  let instd = Buffer.from(VM.instrumentBinary(bin));
  console.log(instd.length - bin.length);
  assert(WebAssembly.validate(instd), "binary is valid wasm");
  // const mod = b.readBinary(instd);
  // mod.validate();

  // VM.run(bin, "add", JSON.stringify({"a": 1000, "b": 1000}));
  // mod.setDebugInfo(true);

  // let mod = new WebAssembly.Module(instd);

// });
// let bin = fs.readFileSync(__dirname + "/");
