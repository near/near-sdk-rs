"use strict";
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (Object.hasOwnProperty.call(mod, k)) result[k] = mod[k];
    result["default"] = mod;
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
const js_base64_1 = require("js-base64");
const path = __importStar(require("path"));
function findContext(_path = "") {
    let paths = [
        _path,
        path.join(process.cwd(), "assembly", "__tests__"),
        process.cwd(),
        __dirname
    ]
        .map(p => {
        if (!p.endsWith("context.json")) {
            return path.join(p, "context.json");
        }
        return p;
    });
    let _paths = paths.filter(p => {
        try {
            require.resolve(p);
        }
        catch {
            return false;
        }
        return true;
    });
    let context = _paths.length > 0 ? require(_paths[0]) : null;
    if (context != null) {
        console.log("found path: " + _paths[0]);
        context.input = js_base64_1.Base64.encode(context.input);
    }
    return context;
}
const current_account_id = "alice";
const signer_account_id = "bob";
const signer_account_pk = "HuxUynD5GdrcZ5MauxJuu74sGHgS6wLfCqqhQkLWK";
const predecessor_account_id = "carol";
const input_str = "{\"a\":21,\"b\":21}";
const block_index = 10;
const block_timestamp = 42;
const epoch_height = 20;
const account_balance = "2";
const account_locked_balance = "1";
const storage_usage = 12;
const attached_deposit = "2";
const prepaid_gas = 10 ** (14);
const random_seed = "HuxUynD5GdrcZ5MauxJuu74sGHgS6wLfCqqhQkLWK";
const is_view = false;
const output_data_receivers = new Uint8Array([]);
function createDefault() {
    const _default = {
        /// The account id of the current contract that we are executing.
        current_account_id,
        /// The account id of that signed the original transaction that led to this
        /// execution.
        signer_account_id,
        /// The public key that was used to sign the original transaction that led to
        /// this execution.
        signer_account_pk,
        predecessor_account_id,
        input: js_base64_1.Base64.encode(input_str),
        block_index,
        block_timestamp,
        epoch_height,
        account_balance,
        account_locked_balance,
        storage_usage,
        attached_deposit,
        prepaid_gas,
        random_seed,
        is_view,
        output_data_receivers,
    };
    return _default;
}
exports.createDefault = createDefault;
function createContext(_path) {
    let context = findContext(_path) || createDefault();
    context.input = js_base64_1.Base64.encode(context.input);
    return context;
}
exports.createContext = createContext;
//# sourceMappingURL=context.js.map