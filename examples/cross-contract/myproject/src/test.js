// >> tests-snippet
describe("Greeter", function() {
    let near;
    let contract;
    let accountId;

    jasmine.DEFAULT_TIMEOUT_INTERVAL = 10000;

    // Common setup below
    beforeAll(async function() {
      if (window.testSettings === undefined) {
        window.testSettings = {};
      }
      near = await nearlib.connect(testSettings);
      accountId = testSettings.accountId;
      const contractName = testSettings.contractName ?
        testSettings.contractName :
        (new URL(window.location.href)).searchParams.get("contractName");
      contract = await near.loadContract(contractName, {
        // NOTE: This configuration only needed while NEAR is still in development
        // View methods are read only. They don't modify the state, but usually return some value.
        viewMethods: ["whoSaidHi"],
        // Change methods can modify the state. But you don't receive the returned value when called.
        changeMethods: [],
        sender: accountId
      });
    });

    // Multiple tests can be described below. Search Jasmine JS for documentation.
    describe("simple", function() {
      beforeAll(async function() {
        // There can be some common setup for each test.
      });

      it("get hello message", async function() {
        const result = await contract.whoSaidHi();
        expect(result).toBeNull();
      });
  });
});
// << tests-snippet
