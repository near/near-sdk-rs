import path from 'path';
import { Runner } from 'near-runner-jest';

const TOTAL_SUPPLY = '1000000';

const runner = Runner.create(async ({ root }) => {
  const alice = await root.createAccount('alice');
  const ft = await root.createAndDeploy(
    'ft',
    path.join(__dirname, '..', 'res', 'fungible_token.wasm'),
  );

  await root.call(
    ft,
    'new_default_meta',
    {
      owner_id: root,
      total_supply: TOTAL_SUPPLY,
    }
  );

  return { alice, ft };
});

runner.test('simple transfer', async ({ ft }) => {
  const totalSupply = await ft.view('ft_total_supply');
  expect(totalSupply).toBe(TOTAL_SUPPLY);
});
