import path from 'path';
import { Gas, NEAR } from 'near-units';
import { Runner, Account } from 'near-runner-jest';

const runner = Runner.create(async ({ root }) => ({
  contract: await root.createAndDeploy(
    'contract',
    path.join(__dirname, '..', 'res', 'cross_contract_high_level.wasm'),
  ),
}));

runner.test('factory pattern: creating sub-accounts, deploying contracts', async ({ root, contract }) => {
  const subAccountPrefix = 'status';
  let tx = await root.call_raw(contract, 'deploy_status_message', {
    account_id: subAccountPrefix,
    amount: NEAR.parse('35N'),
  }, {
    attachedDeposit: NEAR.parse('50N'),
  });
  expect(tx.errors.length).toBe(0);

  const statusMessageContract = contract.getAccount(subAccountPrefix);

  const message = 'hello world';
  tx = await root.call_raw(contract, 'complex_call', {
    account_id: statusMessageContract,
    message,
  }, {
    gas: Gas.parse('99Tgas'),
  });
  expect(tx.errors.length).toBe(0);
  expect(tx.parseResult()).toBe(message);

  // note that `complex_call` set the status for `root`
  expect(
    await statusMessageContract.view('get_status', { account_id: root })
  ).toBe(message);
});

// On-chain merge sort with parallel recursive contract calls
describe('merge_sort', () => {
  runner.test('simplest case - no cross-contract calls', async ({ contract }) => {
    expect(
      await contract.view('merge_sort', { arr: [42] })
    ).toEqual([42]);
  });
  runner.test('sorting a length-2 array (3 cross-contract calls)', async ({ contract }) => {
    expect(
      await contract.call(contract, 'merge_sort', {
        arr: [100, 11]
      }, {
        gas: Gas.parse('50Tgas')
      })
    ).toEqual([11, 100]);
  });
  runner.test('sorting a length-4 array (9 cross-contract calls)', async ({ contract }) => {
    expect(
      await contract.call(contract, 'merge_sort', {
        arr: [255, 9, 100, 11]
      }, {
        gas: Gas.parse('150Tgas')
      })
    ).toEqual([9, 11, 100, 255]);
  });
  runner.test('longer arrays fail', async ({ contract }) => {
    await expect(
      contract.call(contract, 'merge_sort', {
        arr: [7, 1, 6, 5, 255, 9, 100, 11]
      }, {
        gas: Gas.parse('300Tgas') // max allowed attached gas
      })
    ).rejects.toThrowError('Cannot sort arrays larger than length=4 due to gas limits');
  });
})