import { Gas } from 'near-workspaces-ava';
import type { Token } from './utils';
import { createWorkspace, TOKEN_ID } from './utils';

const workspace = createWorkspace(async ({ root, nft }) => ({
  tokenReceiver: await root.createAndDeploy(
    'token-receiver',
    'non-fungible-token/res/token_receiver.wasm',
    {
      method: 'new',
      args: { non_fungible_token_account_id: nft }
    }
  )
}));

workspace.test('refund (no cross-contract call from receiver)', async (t, { root, nft, tokenReceiver }) => {
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
  t.is(tx.errors.length, 0);

  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, root.accountId);
});

workspace.test('refund after receiver makes cross-contract call', async (t, { root, nft, tokenReceiver }) => {
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
  t.is(tx.errors.length, 0);

  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, root.accountId);
});

workspace.test('success (no cross-contract call from receiver)', async (t, { root, nft, tokenReceiver }) => {
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
  t.is(tx.errors.length, 0);

  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, tokenReceiver.accountId);
});

workspace.test('success after receiver makes cross-contract call', async (t, { root, nft, tokenReceiver }) => {
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
  t.is(tx.errors.length, 0);

  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, tokenReceiver.accountId);
});

workspace.test('refund if receiver panics', async (t, { root, nft, tokenReceiver }) => {
  const tx = await root.call_raw(nft, 'nft_transfer_call', {
    receiver_id: tokenReceiver,
    token_id: TOKEN_ID,
    memo: 'transfer and call',
    msg: 'keep-it-later',
  }, {
    attachedDeposit: '1',
    gas: Gas.parse('40 Tgas'), // not enough gas!
  });

  t.is(tx.promiseErrors.length, 1);
  t.regex(tx.promiseErrorMessages[0], /FunctionCallZeroAttachedGas/);

  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, root.accountId);
});
