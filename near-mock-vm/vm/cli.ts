
import { scriptName } from "yargs";
import { VMRunner } from './runner';
import * as fs from "fs";
const v8 = require('v8');
v8.setFlagsFromString('--experimental-wasm-bigint');


function serve(port: string) {
    console.info(`Serve on port ${port}.`);
}

scriptName("near-vm")
   .command({
    command: 'run <wasmPath> <method> [input]',
    describe: "execute smart contract",
    builder: (yargs) => yargs
                       .option('context', {
                         describe: "path to VM context json file."
                       }),
    handler: (argv: any) => {
      const wasmBinary = fs.readFileSync(argv["wasmPath"]);
      const method = argv["method"];
      const input = argv["input"];
      VMRunner.run(wasmBinary, method, input, argv["context"]);
    }
   })
   .demandCommand(2)
   .help()
   .showHelpOnFail(true)
   .argv;