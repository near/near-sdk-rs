import { Runner, ReturnedAccounts } from 'near-runner-ava';

export interface Token {
  token_id: string;
  owner_id: string;
  metadata?: TokenMetadata,
  approved_account_ids?: Record<string, number>;
}

export interface TokenMetadata {
  title?: string;
  description?: string;
  media?: string;
  media_hash?: string;
  copies?: number;
  issued_at?: string;
  expires_at?: string;
  starts_at?: string;
  updated_at?: string;
  extra?: string;
  reference?: string;
  reference_hash?: string;
}

export const TOKEN_ID = '0';

export function createRunner(more: ((accounts: ReturnedAccounts) => Promise<ReturnedAccounts | void>) = async () => ({})) {
  return Runner.create(async ({ root }) => {
    const alice = await root.createAccount('alice');
    const nft = await root.createAndDeploy(
      'nft',
      'non-fungible-token/res/non_fungible_token.wasm',
      {
        method: 'new_default_meta',
        args: { owner_id: root }
      }
    );

    await root.call(
      nft,
      'nft_mint',
      {
        token_id: TOKEN_ID,
        token_owner_id: root,
        token_metadata: {
          title: 'Olympus Mons',
          description: 'The tallest mountain in the charted solar system',
          copies: 1,
        }
      },
      {
        attachedDeposit: '7000000000000000000000'
      }
    );

    const additionalAccounts = await more({ alice, root, nft });

    return {
      ...(additionalAccounts || {}),
      alice,
      nft
    };
  });
}