import path from 'path';
import { Gas } from 'near-units';
import type { Token } from './utils';
import { createRunner, TOKEN_ID } from './utils';

describe('nft_transfer', () => {
  const runner = createRunner();

  runner.test('success', async ({ alice, nft, root }) => {
    await root.call(nft, 'nft_transfer', {
      receiver_id: alice,
      token_id: TOKEN_ID,
      memo: 'simple transfer',
    }, {
      attachedDeposit: '1'
    });

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(alice.accountId);
  });

  runner.test("failure; cannot send some else's token", async ({ alice, nft, root }) => {
    await expect(
      // alice tries to send it to herself
      alice.call_raw(nft, 'nft_transfer', {
        receiver_id: alice,
        token_id: TOKEN_ID,
        memo: 'simple transfer',
      }, {
        attachedDeposit: '1'
      })
    ).rejects.toThrow('Unauthorized');

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });
});

describe('nft_transfer_call', () => {
  const runner = createRunner(async ({ root, nft }) => ({
    tokenReceiver: await root.createAndDeploy(
      'token-receiver',
      path.join(__dirname, '..', 'res', 'token_receiver.wasm'),
      {
        method: 'new',
        args: { non_fungible_token_account_id: nft }
      }
    )
  }));

  runner.test('refund (no cross-contract call from receiver)', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(nft, 'nft_transfer_call', {
      receiver_id: tokenReceiver,
      token_id: TOKEN_ID,
      memo: 'transfer and call',
      msg: 'return-it-now',
    }, {
      attachedDeposit: '1',
      gas: Gas.parse('31 Tgas'),
    });

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });

  runner.test('refund after receiver makes cross-contract call', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(nft, 'nft_transfer_call', {
      receiver_id: tokenReceiver,
      token_id: TOKEN_ID,
      memo: 'transfer and call',
      msg: 'return-it-later',
    }, {
      attachedDeposit: '1',
      gas: Gas.parse('41 Tgas'),
    });

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });

  runner.test('success (no cross-contract call from receiver)', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(nft, 'nft_transfer_call', {
      receiver_id: tokenReceiver,
      token_id: TOKEN_ID,
      memo: 'transfer and call',
      msg: 'keep-it-now',
    }, {
      attachedDeposit: '1',
      gas: Gas.parse('31 Tgas'),
    });

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(tokenReceiver.accountId);
  });

  runner.test('success after receiver makes cross-contract call', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(nft, 'nft_transfer_call', {
      receiver_id: tokenReceiver,
      token_id: TOKEN_ID,
      memo: 'transfer and call',
      msg: 'keep-it-later',
    }, {
      attachedDeposit: '1',
      gas: Gas.parse('41 Tgas'),
    });

    // Make sure all cross-contract calls worked as expected!
    // This is needed because transfer_call gracefully ignores cross-contract call failures,
    // so the transaction can pass even if cross-contract calls fail.
    expect(tx.errors.length).toBe(0);

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(tokenReceiver.accountId);
  });

  runner.test('refund if receiver panics', async ({ root, nft, tokenReceiver }) => {
    const tx = await root.call_raw(nft, 'nft_transfer_call', {
      receiver_id: tokenReceiver,
      token_id: TOKEN_ID,
      memo: 'transfer and call',
      msg: 'keep-it-later',
    }, {
      attachedDeposit: '1',
      gas: Gas.parse('40 Tgas'), // not enough gas!
    });

    expect(tx.promiseErrors.length).toBe(1);
    expect(tx.promiseErrorMessages[0]).toMatch('FunctionCallZeroAttachedGas');

    const { owner_id } =
      await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
    expect(owner_id).toBe(root.accountId);
  });
});