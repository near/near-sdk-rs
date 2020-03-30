export interface VMContext {
    current_account_id: string;
    signer_account_id: string;
    signer_account_pk: string;
    predecessor_account_id: string;
    input: string;
    block_index: number;
    block_timestamp: number;
    account_balance: number;
    account_locked_balance: number;
    storage_usage: number;
    attached_deposit: number;
    prepaid_gas: number;
    random_seed: string;
    is_view: boolean;
    output_data_receivers: Uint8Array;
}
export declare function createDefault(): VMContext;
export declare function createContext(_path?: string): VMContext;
