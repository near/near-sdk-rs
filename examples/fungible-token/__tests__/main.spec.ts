import path from 'path';
import { Runner, BN, NearAccount, captureError } from 'near-runner-jest';

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

const runner = Runner.create(async ({ root }) => ({
  ft: await root.createAndDeploy(
    'fungible-token',
    path.join(__dirname, '..', 'res', 'fungible_token.wasm'),
  ),
  defi: await root.createAndDeploy(
    'defi',
    path.join(__dirname, '..', 'res', 'defi.wasm'),
  ),
  ali: await root.createAccount('ali'),
}));

describe(`Running on ${Runner.getNetworkFromEnv()}`, () => {
  runner.test('Total supply', async ({ ft, ali }) => {
    await init_ft(ft, ali, '1000');

    const totalSupply: string = await ft.view('ft_total_supply');
    expect(totalSupply).toEqual('1000');
  });

  runner.test('Simple transfer', async ({ ft, ali, root }) => {
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
    expect(new BN(rootBalance)).toEqual(initialAmount.sub(transferAmount));
    expect(new BN(aliBalance)).toEqual(transferAmount);
  });

  runner.test('Can close empty balance account', async ({ ft, ali, root }) => {
    await init_ft(ft, root);

    await registerUser(ft, ali);

    const result = await ali.call(
      ft,
      'storage_unregister',
      {},
      { attachedDeposit: '1' },
    ) as boolean;

    expect(result).toStrictEqual(true);
  });

  runner.test('Can force close non-empty balance account', async ({ ft, root }) => {
    await init_ft(ft, root, '100');
    const errorString = await captureError(async () =>
      root.call(ft, 'storage_unregister', {}, { attachedDeposit: '1' }));
    expect(errorString).toContain('Can\'t unregister the account with the positive balance without force');

    const result = await root.call_raw(
      ft,
      'storage_unregister',
      { force: true },
      { attachedDeposit: '1' },
    );

    expect(result.logs[0]).toEqual(
      `Closed @${root.accountId} with 100`,
    );
  });

  runner.test('Transfer call with burned amount', async ({ ft, defi, root }) => {
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

    expect(result.logs).toContain(
      `Closed @${root.accountId} with ${(initialAmount.sub(transferAmount)).toString()}`,
    );

    expect(result.parseResult()).toStrictEqual(true);

    expect(result.logs).toContain(
      'The account of the sender was deleted',
    );
    expect(result.logs).toContain(
      `Account @${root.accountId} burned ${burnAmount.toString()}`,
    );

    // Help: this index is diff from sim, we have 10 len when they have 4
    const callbackOutcome = result.receipts_outcomes[5];

    expect(callbackOutcome.parseResult()).toEqual(transferAmount.toString());
    const expectedAmount = transferAmount.sub(burnAmount).toString();

    const totalSupply: string = await ft.view('ft_total_supply');
    expect(totalSupply).toEqual(expectedAmount);

    const defiBalance: string = await ft.view('ft_balance_of', {
      account_id: defi,
    });
    expect(defiBalance).toEqual(expectedAmount);
  });

  runner.test('Transfer call immediate return no refund', async ({ ft, defi, root }) => {
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
    expect(new BN(rootBalance)).toEqual(initialAmount.sub(transferAmount));
    expect(new BN(defiBalance)).toEqual(transferAmount);
  });

  runner.test('Transfer call promise panics for a full refund', async ({ ft, defi, root }) => {
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
    expect(result.promiseErrorMessages.join('\n')).toMatch('ParseIntError');

    const rootBalance: string = await ft.view('ft_balance_of', {
      account_id: root,
    });
    const defiBalance: string = await ft.view('ft_balance_of', {
      account_id: defi,
    });
    expect(new BN(rootBalance)).toEqual(initialAmount);
    expect(new BN(defiBalance)).toEqual(new BN(0));
  });
});
