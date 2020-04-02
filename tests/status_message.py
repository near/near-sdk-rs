#! /usr/bin/env python3

from lib.runtime import Runtime
import logging
import unittest
import json

class TestStringMethods(unittest.TestCase):
    def setUp(self):
        runtime = Runtime()
        self.alice = runtime.new_account("alice")
        self.bob = runtime.new_account("bob")
        self.status = runtime.new_account("status", "../examples/status-message/res/status_message.wasm")

    def test_set_get(self):
        self.assertIsNone(self.alice.call_other("status", "set_status", {'message': 'hi'})['err'])
        self.assertEqual(json.loads(self.status.view("get_status", {'account_id': 'alice'})['return_data']), 'hi')

    def test_set_get_different_account(self):
        self.assertIsNone(self.alice.call_other("status", "set_status", {'message': 'Hi, Bob'})['err'])
        self.assertIsNone(self.bob.call_other("status", "set_status", {'message': 'Hi, Alice'})['err'])
        self.assertEqual(json.loads(self.status.view("get_status", {'account_id': 'alice'})['return_data']), 'Hi, Bob')
        self.assertEqual(json.loads(self.status.view("get_status", {'account_id': 'bob'})['return_data']), 'Hi, Alice')

    def test_set_twice(self):
        self.assertIsNone(self.alice.call_other("status", "set_status", {'message': 'Hi'})['err'])
        self.assertEqual(json.loads(self.status.view("get_status", {'account_id': 'alice'})['return_data']), 'Hi')
        self.assertIsNone(self.alice.call_other("status", "set_status", {'message': 'Yo'})['err'])
        self.assertEqual(json.loads(self.status.view("get_status", {'account_id': 'alice'})['return_data']), 'Yo')


if __name__ == '__main__':
    logging.basicConfig(level=logging.INFO)
    unittest.main()
