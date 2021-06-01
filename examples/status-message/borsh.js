const borsh = require("borsh");

class Assignable {
  constructor(properties) {
    Object.keys(properties).map((key) => {
      this[key] = properties[key];
    });
  }
}

class StatusMessage extends Assignable {}

class Record extends Assignable {}

const schema = new Map([
  [StatusMessage, { kind: "struct", fields: [["records", [Record]]] }],
  [
    Record,
    {
      kind: "struct",
      fields: [
        ["k", "string"],
        ["v", "string"],
      ],
    },
  ],
]);

const stateKey = "U1RBVEU=";
console.log(Buffer.from(stateKey, "base64"));
console.log(Buffer.from(stateKey, "base64").toString());
const stateValue =
  "AgAAAA8AAABhbGljZS50ZXN0Lm5lYXIFAAAAaGVsbG8NAAAAYm9iLnRlc3QubmVhcgUAAAB3b3JsZA==";
const stateValueBuffer = Buffer.from(stateValue, "base64");
let statusMessage = borsh.deserialize(schema, StatusMessage, stateValueBuffer);
console.log(statusMessage);

console.log(
  Buffer.from(borsh.serialize(schema, statusMessage)).toString("base64")
);
statusMessage.records.push(new Record({ k: "alice.near", v: "hello world" }));
console.log(statusMessage);

console.log(
  Buffer.from(borsh.serialize(schema, statusMessage)).toString("base64")
);
