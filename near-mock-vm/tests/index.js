const v8 = require('v8');
v8.setFlagsFromString('--experimental-wasm-bigint');
let VM = require('../dist').NearVM;
let fs = require("fs");
let assert = require("assert");

let bin = fs.readFileSync(__dirname + "/../out/main.wasm");
// console.log(bin.length)
let instd = Buffer.from(VM.instrumentBinary(bin));
assert(WebAssembly.validate(instd), "binary is valid wasm");
assert(instd.length - bin.length > 0 , "instrumented binary should be bigger");
// const original = new WebAssembly.Module(bin);
const instrumented = new WebAssembly.Module(instd);
// console.log(WebAssembly.Module.imports(original));
const newImports = WebAssembly.Module.imports(instrumented);
assert(newImports.some(_import => _import.name == "gas" && _import.kind == "function" ),
      "Instrumented module's imports should include a gas function");