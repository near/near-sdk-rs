"use strict";
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (Object.hasOwnProperty.call(mod, k)) result[k] = mod[k];
    result["default"] = mod;
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
const yargs_1 = require("yargs");
const runner_1 = require("./runner");
const fs = __importStar(require("fs"));
const v8 = require('v8');
v8.setFlagsFromString('--experimental-wasm-bigint');
function serve(port) {
    console.info(`Serve on port ${port}.`);
}
yargs_1.scriptName("near-vm")
    .command({
    command: 'run <wasmPath> <method> [input]',
    describe: "execute smart contract",
    builder: (yargs) => yargs
        .option('context', {
        describe: "path to VM context json file."
    }),
    handler: (argv) => {
        const wasmBinary = fs.readFileSync(argv["wasmPath"]);
        const method = argv["method"];
        const input = argv["input"];
        runner_1.VMRunner.run(wasmBinary, method, input, argv["context"]);
    }
})
    .demandCommand(2)
    .help()
    .showHelpOnFail(true)
    .argv;
//# sourceMappingURL=cli.js.map