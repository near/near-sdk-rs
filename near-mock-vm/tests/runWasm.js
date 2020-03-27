let rust = require("../pkg/wasm_mock_vm");
let fs = require("fs");


global.run_binary = (bin, method) => {
  let mod = new WebAssembly.Module(bin);
  let inst = new WebAssembly.Instance(mod, {});
  return BigInt(inst.exports[method](1, 2));
}
let bin = fs.readFileSync("src/mock/add.wasm")
rust.run(bin);
