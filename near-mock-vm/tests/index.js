const v8 = require('v8');
v8.setFlagsFromString('--experimental-wasm-bigint');

let VM = require('../dist').NearVM;
let fs = require("fs");
const binaryen = require("binaryen");


// Proxy Binaryen's ready event
// Object.defineProperty(b, "ready", {
//   get: function() { return binaryen.ready; }
// });
binaryen.ready.then((b) => {
  let bin = fs.readFileSync(__dirname + "/../out/main.wasm")
  let instd = Buffer.from(VM.instrumentBinary(bin));
  const mod = b.readBinary(instd);
  mod.validate();
  debugger;
  b.setDebugInfo(true);
  VM.run(bin, "add", JSON.stringify({"a": 1000, "b": 1000}))
  // mod.setDebugInfo(true);

  // let mod = new WebAssembly.Module(instd);

});
// let bin = fs.readFileSync(__dirname + "/");
