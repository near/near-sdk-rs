import subprocess
import json
import base64
import base58
import collections
import logging

def context(
        current_account_id="alice_near",
        signer_account_id="bob_near",
        predecessor_account_id="carol_near",
        signer_account_pk="KuTCtARNzxZQ3YvXDeLjx83FDqxv2SdQTSbiq876zR7",
        input="",
        block_index=1,
        block_timestamp=1585778575325000000,
        epoch_height=1,
        account_balance=1000000000000000000000000000000000000,
        account_locked_balance=0,
        storage_usage=100,
        attached_deposit=0,
        prepaid_gas=1000000000000000,
        random_seed="KuTCtARNzxZQ3YvXDeLjx83FDqxv2SdQTSbiq876zR7",
        is_view=False,
        output_data_receivers=[],
    ):
    if input is None:
        input = b''
    elif type(input) is dict:
        input = bytes(json.dumps(input), 'utf-8')
    elif type(input) is list:
        input = bytes(input)
    elif type(input) is str:
        input = bytes(input, 'utf-8')
    return {
        "current_account_id": current_account_id,
        "signer_account_id": signer_account_id,
        "signer_account_pk": signer_account_pk,
        "predecessor_account_id": predecessor_account_id,
        "input": base64.b64encode(input).decode('utf-8'),
        "block_index": block_index,
        "block_timestamp": block_timestamp,
        "epoch_height": epoch_height,
        "account_balance": str(account_balance),
        "account_locked_balance": str(account_locked_balance),
        "storage_usage": storage_usage,
        "attached_deposit": attached_deposit,
        "prepaid_gas": prepaid_gas,
        "random_seed": random_seed,
        "is_view": is_view,
        "output_data_receivers": output_data_receivers
    }

class Account:
    def __init__(self, account_id, wasm_file=None):
        self.account_id = account_id
        self.wasm_file = wasm_file
        self.state = {}
        self.balance = 1000000000000000000000000000000000000
        self.locked_balance = 0
        self.runtime = None
        self.signer_account_pk = base58.b58encode(account_id[:32].rjust(32, ' ')).decode('utf-8')

    def set_wasm_file(self, wasm_file):
        self.wasm_file = wasm_file
        return self

    def set_signer_account_pk(self, signer_account_pk):
        self.signer_account_pk = signer_account_pk
        return self

    def set_state(self, state):
        self.state = state
        return self

    def set_balance(self, balance):
        self.balance = balance
        return self

    def set_locked_balance(self, locked_balance):
        self.locked_balance = locked_balance
        return self

    def set_runtime(self, runtime):
        self.runtime = runtime
        return self

    def call_step_other(self, account_id, method_name, input=None, prepaid_gas=10**15):
        if self.runtime is None:
            raise Exception("Runtime is not set")
        return self.runtime.call_step(account_id, method_name, input=input, signer_account_id=self.account_id, prepaid_gas=prepaid_gas)

    def call_step(self, method_name, input=None, prepaid_gas=10**15):
        return self.call_step_other(self.account_id, method_name, input=input, prepaid_gas=prepaid_gas)

    def call_other(self, account_id, method_name, input=None, prepaid_gas=10**15):
        if self.runtime is None:
            raise Exception("Runtime is not set")
        return self.runtime.call(account_id, method_name, input=input, signer_account_id=self.account_id, prepaid_gas=prepaid_gas)

    def call(self, method_name, input=None, prepaid_gas=10**15):
        return self.call_other(self.account_id, method_name, input=input, prepaid_gas=prepaid_gas)

    def view(self, method_name, input=None):
        if self.runtime is None:
            raise Exception("Runtime is not set")
        result = self.runtime.call_step(self.account_id, method_name, input=input, is_view=True)
        return_data = result.get('outcome', {}).get('return_data', None)
        if return_data is not None:
            return_data = return_data['Value'] if 'Value' in return_data else ''
        return {
            'return_data': return_data,
            'err': result['err'],
            'result': result,
        }

class Runtime:
    def __init__(self):
        self.log = logging.getLogger('Runtime')
        self.accounts = {}

    def new_account(self, account_id, wasm_file=None):
        account = Account(account_id, wasm_file)
        self.accounts[account_id] = account
        return account.set_runtime(self)

    def get_or_create_account(self, account_id):
        account = self.accounts.get(account_id)
        if account is None:
            account = self.new_account(account_id)
        return account

    def call_step(
            self,
            account_id,
            method_name,
            input=None,
            signer_account_id=None,
            predecessor_account_id=None,
            input_data=[],
            output_data_receivers=[],
            prepaid_gas=10**15,
            attached_deposit=0,
            is_view=False,
    ):
        if signer_account_id is None:
            signer_account_id = account_id
        if predecessor_account_id is None:
            predecessor_account_id = signer_account_id

        signer_account = self.get_or_create_account(signer_account_id)
        predecessor_account = self.get_or_create_account(predecessor_account_id)

        account = self.accounts.get(account_id)
        if account is None:
            return False

        ctx = context(
            current_account_id=account.account_id,
            signer_account_id=signer_account.account_id,
            predecessor_account_id=predecessor_account.account_id,
            signer_account_pk=signer_account.signer_account_pk,
            input=input,
            account_balance=account.balance,
            account_locked_balance=account.locked_balance,
            output_data_receivers=output_data_receivers,
            prepaid_gas=prepaid_gas,
            attached_deposit=attached_deposit,
            is_view=is_view
        )
        args = [
            "near-vm-runner-standalone",
            "--context=%s" % (json.dumps(ctx),),
            "--wasm-file=%s" % account.wasm_file,
            "--method-name=%s" % (method_name,),
            "--state=%s" % (json.dumps(account.state),),
            ]
        for data in input_data:
            args.append("--promise-results=%s" % (json.dumps(data),))

        exec_result = subprocess.run(args, capture_output=True)
        if exec_result.returncode != 0:
            raise Exception("Failed to run successfully: %s", (exec_result,))
        result = json.loads(exec_result.stdout)
        if not is_view and result['err'] is None:
            account.set_balance(result['outcome']['balance'])
            account.set_state(result['state'])
        return result

    def call(self, account_id, method_name, input=None, signer_account_id=None, prepaid_gas=10**15):
        q = collections.deque([{
            'index': 0,
            'account_id': account_id,
            'method_name': method_name,
            'input': input,
            'signer_account_id': signer_account_id,
            'predecessor_account_id': signer_account_id,
            'input_data': [],
            'prepaid_gas': prepaid_gas,
            'attached_deposit': 0,
        }])
        num_receipts = 1
        all_input_data = {}
        all_output_data = collections.defaultdict(list)
        num_data = 0
        return_index = 0
        calls = {}
        results = {}
        while len(q) > 0:
            c = q.popleft()
            index = c['index']

            input_data = []
            for d in c['input_data']:
                if d in all_input_data:
                    input_data.append(all_input_data[d])
                else:
                    break

            if len(input_data) < len(c['input_data']):
                q.append(c)
                continue

            output_data = all_output_data.get(index, [])

            calls[index] = c
            self.log.debug('Call %s Output %s' % (c, output_data))
            result = self.call_step(
                account_id=c['account_id'],
                method_name=c['method_name'],
                input=c['input'],
                signer_account_id=c['signer_account_id'],
                predecessor_account_id=c['predecessor_account_id'],
                input_data=input_data,
                output_data_receivers=[d['account_id'] for d in output_data],
                prepaid_gas=c['prepaid_gas'],
                attached_deposit=c['attached_deposit']
            )
            results[index] = result
            self.log.debug('Result %s' % (result, ))
            if result:
                if result['outcome']:
                    for log in result['outcome']['logs']:
                            self.log.info('%s: %s' % (c['account_id'], log))
                    if result['err'] is None:
                        ret = result['outcome']['return_data']
                        if 'ReceiptIndex' in ret:
                            adj_index = ret['ReceiptIndex'] + num_receipts
                            for d in output_data:
                                all_output_data[adj_index].append(d)
                            if return_index == index:
                                return_index = adj_index
                        else:
                            result_data = {
                                'Successful': ret['Value'] if 'Value' in ret else ''
                            }
                            for d in output_data:
                                all_input_data[d['data_id']] = result_data

                        for i, receipt in enumerate(result['receipts'], num_receipts):
                            assert(len(receipt['actions']) == 1)
                            assert('FunctionCall' in receipt['actions'][0])
                            fca = receipt['actions'][0]['FunctionCall']
                            new_input_data = []
                            for ind in receipt['receipt_indices']:
                                data_id = num_data
                                num_data += 1
                                new_input_data.append(data_id)
                                adj_index = ind + num_receipts
                                all_output_data[adj_index].append({
                                    'account_id': receipt['receiver_id'],
                                    'data_id': data_id,
                                })

                            q.append({
                                'index': i,
                                'account_id': receipt['receiver_id'],
                                'method_name': fca['method_name'],
                                'input': fca['args'],
                                'signer_account_id': signer_account_id,
                                'predecessor_account_id': c['account_id'],
                                'input_data': new_input_data,
                                'prepaid_gas': fca['gas'],
                                'attached_deposit': fca['deposit'],
                            })

                        num_receipts += len(result['receipts'])
                    else:
                        result_data = {
                            'Failed': None,
                        }
                        for d in output_data:
                            all_input_data[d['data_id']] = result_data

            self.log.debug('Queue %s' % (q, ))

        result = results[return_index]
        self.log.debug("Final result %s" % (result,))
        return_data = result.get('outcome', {}).get('return_data', None)
        if return_data is not None:
            return_data = return_data['Value'] if 'Value' in return_data else ''

        return {
            'return_data': return_data,
            'err': result['err'],
            'result': result,
            'calls': calls,
            'results': results,
        }


