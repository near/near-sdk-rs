let rust = require("../pkg/near_mock_vm");
// let binar
let fs = require("fs");
const binaryen = require("binaryen");


// Proxy Binaryen's ready event
// Object.defineProperty(b, "ready", {
//   get: function() { return binaryen.ready; }
// });
binaryen.ready.then((b) => {
  let bin = fs.readFileSync(__dirname + "/../../../Near-AssemblyScript/examples/counter/out/main.wasm")
  let instd = new Buffer(rust.inject_contract(bin));
  b.setDebugInfo(true);
  const mod = b.readBinary(instd);
  // mod.setDebugInfo(true);

  // let mod = new WebAssembly.Module(instd);
  debugger;

});
// global.run_binary = (bin, method) => {
//   let mod = new WebAssembly.Module(bin);
//   let inst = new WebAssembly.Instance(mod, {});
//   return BigInt(inst.exports[method](1, 2));
// }
// process.stdout.write(bin);
// process.stdout.write("\0");
// process.stdout.end()
// process.exit(0);