import path from 'path';
import { Gas } from 'near-units';
import { Runner } from 'near-runner-jest';

const runner = Runner.create(async ({ root }) => ({
  contract: await root.createAndDeploy(
    'contract',
    path.join(__dirname, '..', 'res', 'callback_results.wasm'),
  ),
}));

runner.test('method `a` always returns `8`', async ({ root, contract }) => {
  expect(
    await root.call(contract, 'a', {})
  ).toBe(8);
});

describe.each`
  fail_b   | c_value | expected
  ${false} | ${1}    | ${[false, false]}
  ${true}  | ${1}    | ${[true, false]}
  ${false} | ${0}    | ${[false, true]}
  ${true}  | ${0}    | ${[true, true]}
`('call_all', ({ fail_b, c_value, expected }) => {
  runner.test(`with {fail_b: ${fail_b}, c_value: ${c_value}} gives ${expected}`, async ({ root, contract }) => {
    expect(
      await root.call(contract, 'call_all', { fail_b, c_value }, {
        gas: Gas.parse('240Tgas')
      })
    ).toEqual(expected);
  });
});