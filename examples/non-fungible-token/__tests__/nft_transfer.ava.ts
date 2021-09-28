import type { Token } from './utils';
import { createRunner, TOKEN_ID } from './utils';

const runner = createRunner();

runner.test('succeeds', async (t, { alice, nft, root }) => {
  await root.call(nft, 'nft_transfer', {
    receiver_id: alice,
    token_id: TOKEN_ID,
    memo: 'simple transfer',
  }, {
    attachedDeposit: '1'
  });

  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, alice.accountId);
});

runner.test("fails when trying to send some else's token", async (t, { alice, nft, root }) => {
  const error = await t.throwsAsync(
    // alice tries to send it to herself
    alice.call_raw(nft, 'nft_transfer', {
      receiver_id: alice,
      token_id: TOKEN_ID,
      memo: 'simple transfer',
    }, {
      attachedDeposit: '1'
    })
  );
  t.regex(error.message, /Unauthorized/);

  const { owner_id } =
    await nft.view('nft_token', { token_id: TOKEN_ID }) as Token;
  t.is(owner_id, root.accountId);
});