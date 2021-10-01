import { Gas, NEAR } from 'near-units';
import { Runner, Account } from 'near-runner-ava';

const runner = Runner.create(async ({ root }) => ({
  contract: await root.createAndDeploy(
    'contract',
    'cross-contract-low-level/res/cross_contract_low_level.wasm',
  ),
}));

runner.test('factory pattern: creating sub-accounts, deploying contracts', async (t, { root, contract }) => {
  const subAccountPrefix = 'status';
  let tx = await root.call_raw(contract, 'deploy_status_message', {
    account_id: subAccountPrefix,
    amount: NEAR.parse('35N'),
  }, {
    attachedDeposit: NEAR.parse('50N'),
  });
  t.is(tx.errors.length, 0);

  const statusMessageContract = contract.getAccount(subAccountPrefix);

  const message = 'hello world';
  tx = await root.call_raw(contract, 'complex_call', {
    account_id: statusMessageContract,
    message,
  }, {
    gas: Gas.parse('99Tgas'),
  });
  t.is(tx.errors.length, 0);
  t.is(tx.parseResult(), message);

  // note that `complex_call` set the status for `root`
  t.is(
    await statusMessageContract.view('get_status', { account_id: root }),
    message
  );
});

// On-chain merge sort with parallel recursive contract calls
runner.test('merge_sort simple - no cross-contract calls', async (t, { contract }) => {
  t.deepEqual(
    await contract.view('merge_sort', { arr: [42] }),
    [42]
  );
});
runner.test('merge_sort with length-2 array (3 cross-contract calls)', async (t, { contract }) => {
  t.deepEqual(
    await contract.call(contract, 'merge_sort', {
      arr: [100, 11]
    }, {
      gas: Gas.parse('50Tgas')
    }),
    [11, 100]
  );
});
runner.test('merge_sort with length-4 array (9 cross-contract calls)', async (t, { contract }) => {
  t.deepEqual(
    await contract.call(contract, 'merge_sort', {
      arr: [255, 9, 100, 11]
    }, {
      gas: Gas.parse('150Tgas')
    }),
    [9, 11, 100, 255]
  );
});
runner.test('merge_sort with longer arrays fails', async (t, { contract }) => {
  const error = await t.throwsAsync(
    contract.call(contract, 'merge_sort', {
      arr: [7, 1, 6, 5, 255, 9, 100, 11]
    }, {
      gas: Gas.parse('300Tgas') // max allowed attached gas
    })
  );
  t.regex(error.message, /Cannot sort arrays larger than length=4 due to gas limits/);
});