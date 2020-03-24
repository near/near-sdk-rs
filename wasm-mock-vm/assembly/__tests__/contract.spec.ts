import { storage } from "near-runtime-ts";
import { Contract } from "../contract";
import { VM } from "..";

describe("Contract", () => {
  beforeAll(() => {
    VM.saveState();
  });
  
  afterAll(() => {
    VM.restoreState();
  });

  it("should be able to be stored and retrieved", () => {
    const contract = new Contract("unique name!");
    storage.set("contract", contract);
    const otherContract = storage.get<Contract>("contract")!;
    expect(contract.name).toBe(otherContract.name, "contracts should have the same name");
    expect(contract.name).not.toBe("", "contract's name should not be empty");
  });
  
  it("should still be in the storage", () => {
    const otherContract = storage.get<Contract>("contract")!;
    expect(otherContract.name).toBe("unique name!", "contracts should have the same name");
    expect(otherContract.name).not.toBe("", "contract's name should not be empty");
  });
});

describe("Contract", () => {
  beforeEach( () => {
    VM.saveState();
  });

  afterEach(() => {
    VM.restoreState();
  });

  it("should be able to be stored and retrieved", () => {
    const contract = new Contract("unique name!");
    expect(storage.contains("contract")).toBe(false, "contract shouldn't exist in storage before putting it in.");
    storage.set("contract", contract);
    const otherContract = storage.get<Contract>("contract")!;
    expect(contract.name).toBe(otherContract.name, "contracts should have the same name");
    expect(contract.name).not.toBe("", "contract's name should not be empty");
  });

  it("should not be in the storage", () => {
    // const contract = new Contract("unique name!");
    // storage.set("contract", contract);
    expect(storage.contains("contract")).toBe(false, "the contract shouldn't exist");
  });
});