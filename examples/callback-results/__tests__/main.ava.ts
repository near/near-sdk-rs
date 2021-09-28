import path from 'path';
import { Gas } from 'near-units';
import { Runner } from 'near-runner-ava';

const runner = Runner.create(async ({ root }) => ({
  contract: await root.createAndDeploy(
    'contract',
    path.join(__dirname, '..', 'res', 'callback_results.wasm'),
  ),
}));

runner.test('method `a` always returns `8`', async (t, { root, contract }) => {
  t.is(
    await root.call(contract, 'a', {}),
    8
  );
});

[
  { fail_b: false, c_value: 1, expected: [false, false] },
  { fail_b: true, c_value: 1, expected: [true, false] },
  { fail_b: false, c_value: 0, expected: [false, true] },
  { fail_b: true, c_value: 0, expected: [true, true] },
].forEach(({ fail_b, c_value, expected }) => {
  runner.test(`call_all with {fail_b: ${fail_b}, c_value: ${c_value}} gives ${expected}`, async (t, { root, contract }) => {
    t.deepEqual(
      await root.call(contract, 'call_all', { fail_b, c_value }, {
        gas: Gas.parse('240Tgas')
      }),
      expected
    );
  });
})