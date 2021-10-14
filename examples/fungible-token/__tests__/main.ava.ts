import { Workspace, BN, NearAccount, captureError } from 'near-workspaces-ava';

const STORAGE_BYTE_COST = '10000000000000000000';

async function init_ft(
  ft: NearAccount,
  owner: NearAccount,
  supply: BN | string = '10000',
) {
  await ft.call(ft, 'new_default_meta', {
    owner_id: owner,
    total_supply: supply,
  });
}

async function init_defi(defi: NearAccount, ft: NearAccount) {
  await defi.call(defi, 'new', {
    fungible_token_account_id: ft,
  });
}

async function registerUser(ft: NearAccount, user: NearAccount) {
  await user.call(
    ft,
    'storage_deposit',
    { account_id: user },
    // Deposit pulled from ported sim test
    { attachedDeposit: new BN(STORAGE_BYTE_COST).mul(new BN(125)) },
  );
}

const workspace = Workspace.init(async ({ root }) => ({
  ft: await root.createAndDeploy(
    'fungible-token',
    'fungible-token/res/fungible_token.wasm',
  ),
  defi: await root.createAndDeploy(
    'defi',
    'fungible-token/res/defi.wasm',
  ),
  ali: await root.createAccount('ali'),
}));

workspace.test('Total supply', async (t, { ft, ali }) => {
  await init_ft(ft, ali, '1000');

  const totalSupply: string = await ft.view('ft_total_supply');
  t.is(totalSupply, '1000');
});

workspace.test('Simple transfer', async (t, { ft, ali, root }) => {
  const initialAmount = new BN('10000');
  const transferAmount = new BN('100');
  await init_ft(ft, root, initialAmount);

  // Register by prepaying for storage.
  await registerUser(ft, ali);

  await root.call(
    ft,
    'ft_transfer',
    {
      receiver_id: ali,
      amount: transferAmount,
    },
    { attachedDeposit: '1' },
  );

  const rootBalance: string = await ft.view('ft_balance_of', {
    account_id: root,
  });
  const aliBalance: string = await ft.view('ft_balance_of', {
    account_id: ali,
  });
  t.deepEqual(new BN(rootBalance), initialAmount.sub(transferAmount));
  t.deepEqual(new BN(aliBalance), transferAmount);
});

workspace.test('Can close empty balance account', async (t, { ft, ali, root }) => {
  await init_ft(ft, root);

  await registerUser(ft, ali);

  const result = await ali.call(
    ft,
    'storage_unregister',
    {},
    { attachedDeposit: '1' },
  ) as boolean;

  t.is(result, true);
});

workspace.test('Can force close non-empty balance account', async (t, { ft, root }) => {
  await init_ft(ft, root, '100');
  const errorString = await captureError(async () =>
    root.call(ft, 'storage_unregister', {}, { attachedDeposit: '1' }));

  t.regex(errorString, /Can't unregister the account with the positive balance without force/);

  const result = await root.call_raw(
    ft,
    'storage_unregister',
    { force: true },
    { attachedDeposit: '1' },
  );

  t.is(result.logs[0],
    `Closed @${root.accountId} with 100`,
  );
});

workspace.test('Transfer call with burned amount', async (t, { ft, defi, root }) => {
  const initialAmount = new BN(10_000);
  const transferAmount = new BN(100);
  const burnAmount = new BN(10);
  await init_ft(ft, root, initialAmount);
  await init_defi(defi, ft);

  await registerUser(ft, defi);

  const result = await root
    .createTransaction(ft)
    .functionCall(
      'ft_transfer_call',
      {
        receiver_id: defi,
        amount: transferAmount,
        msg: burnAmount,
      },
      { attachedDeposit: '1', gas: '150000000000000' },
    )
    .functionCall(
      'storage_unregister',
      { force: true },
      { attachedDeposit: '1', gas: '150000000000000' },
    )
    .signAndSend();

  t.true(result.logs.includes(
    `Closed @${root.accountId} with ${(initialAmount.sub(transferAmount)).toString()}`,
  ));

  t.is(result.parseResult(), true);

  t.true(result.logs.includes(
    'The account of the sender was deleted',
  ));
  t.true(result.logs.includes(
    `Account @${root.accountId} burned ${burnAmount.toString()}`,
  ));

  // Help: this index is diff from sim, we have 10 len when they have 4
  const callbackOutcome = result.receipts_outcomes[5];

  t.is(callbackOutcome.parseResult(), transferAmount.toString());
  const expectedAmount = transferAmount.sub(burnAmount).toString();

  const totalSupply: string = await ft.view('ft_total_supply');
  t.is(totalSupply, expectedAmount);

  const defiBalance: string = await ft.view('ft_balance_of', {
    account_id: defi,
  });
  t.is(defiBalance, expectedAmount);
});

workspace.test('Transfer call immediate return no refund', async (t, { ft, defi, root }) => {
  const initialAmount = new BN(10_000);
  const transferAmount = new BN(100);
  await init_ft(ft, root, initialAmount);
  await init_defi(defi, ft);

  await registerUser(ft, defi);

  await root.call(
    ft,
    'ft_transfer_call',
    {
      receiver_id: defi,
      amount: transferAmount,
      memo: null,
      msg: 'take-my-money',
    },
    { attachedDeposit: '1', gas: '150000000000000' },
  );

  const rootBalance: string = await ft.view('ft_balance_of', {
    account_id: root,
  });
  const defiBalance: string = await ft.view('ft_balance_of', {
    account_id: defi,
  });
  t.deepEqual(new BN(rootBalance), initialAmount.sub(transferAmount));
  t.deepEqual(new BN(defiBalance), transferAmount);
});

workspace.test('Transfer call promise panics for a full refund', async (t, { ft, defi, root }) => {
  const initialAmount = new BN(10_000);
  const transferAmount = new BN(100);
  await init_ft(ft, root, initialAmount);
  await init_defi(defi, ft);

  await registerUser(ft, defi);

  const result = await root.call_raw(
    ft,
    'ft_transfer_call',
    {
      receiver_id: defi,
      amount: transferAmount,
      memo: null,
      msg: 'this won\'t parse as an integer',
    },
    { attachedDeposit: '1', gas: '150000000000000' },
  );
  t.regex(result.promiseErrorMessages.join('\n'), /ParseIntError/);

  const rootBalance: string = await ft.view('ft_balance_of', {
    account_id: root,
  });
  const defiBalance: string = await ft.view('ft_balance_of', {
    account_id: defi,
  });
  t.deepEqual(new BN(rootBalance), initialAmount);
  t.deepEqual(new BN(defiBalance), new BN(0));
});
