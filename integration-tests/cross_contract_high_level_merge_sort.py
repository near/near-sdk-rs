#! /usr/bin/env python3

from lib.runtime import Runtime
import logging
import unittest
import struct
import json

def borshify(arr):
    res = list(struct.pack('<L', len(arr)))
    for a in arr:
        res.extend(struct.pack('<B', a))
    return bytes(res)

class TestStringMethods(unittest.TestCase):
    def setUp(self):
        runtime = Runtime()
        self.account = runtime.new_account("sorter", "../examples/cross-contract-high-level/res/cross_contract_high_level.wasm")


    def merge_sort_test(self, arr, prepaid_gas=10**15):
        res = self.account.call("merge_sort", {
            'arr': arr,
        }, prepaid_gas=prepaid_gas)
        self.assertIsNot(res, False)
        self.assertIsNone(res['err'])
        self.assertEqual(bytes(res['return_data'], 'utf-8'), borshify(sorted(arr)))

    def test_simple(self):
        self.merge_sort_test([3, 1, 2])

    def test_long(self):
        self.merge_sort_test([1, 2, 5, 3, 10, 13, 20, 6, 4, 2, 1], prepaid_gas=10**16)

    def test_long_low_gas(self):
        res = self.account.call("merge_sort", {
            'arr': [1, 2, 5, 3, 10, 13, 20, 6, 4, 2, 1],
        }, prepaid_gas=10**14)
        self.assertIsNot(res, False)
        self.assertTrue('Callback computation 0 was not successful' in json.dumps(res['err']))


if __name__ == '__main__':
    logging.basicConfig(level=logging.INFO)
    unittest.main()
