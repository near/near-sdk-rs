#! /usr/bin/env python3

import subprocess
import json
import struct
import copy
import base64

subprocess.run("./build.sh")

with open("./res/context.json", 'r') as f:
    context = json.load(f)

def call(method_name, input=None):
    tmp_context = copy.deepcopy(context)
    if input is not None:
        tmp_context['input'] = base64.b64encode(bytes(input)).decode('utf-8')
    args = [
        "near-vm-runner-standalone",
        "--context=%s" % (json.dumps(tmp_context),),
        "--config-file=./res/config.json",
        "--wasm-file=./res/gas_fee_tester.wasm",
        "--method-name=%s" % (method_name,),
        ]
    result = subprocess.run(args, capture_output=True)
    assert(result.returncode == 0)
    # print(result.stdout)
    return json.loads(result.stdout)

def gas_of(method_name, input=None):
    return call(method_name, input)['burnt_gas']

def json_input(obj):
    return list(json.dumps(obj).encode('utf-8'))

def borsh_input(format, *values):
    return list(struct.pack(format, *values))

def f(a):
    return f'{a:,}'

def mean(a):
    return f(int(sum(a) / len(a) + 0.5))


gas_global_noop = gas_of("global_noop")
print("Base gas cost of full noop call is %s" % (f(gas_global_noop),))
gas_structure_noop = gas_of("structure_noop")

gas_structure_init = gas_structure_noop - gas_global_noop
print("Base gas cost of calling a function with near_bindgen is %s" % (f(gas_structure_init),))

gas_json_input_u32_a = gas_of("input_json_u32_a", json_input({"a": 1}))
gas_json_input_u32_aa = gas_of("input_json_u32_aa", json_input({"aa": 1}))
diff = gas_json_input_u32_aa - gas_json_input_u32_a
print("Extra cost of JSON input for an extra character in an argument name e.g. (aa: u32) vs (a: u32) %s" % (f(diff),))

def test_integers():
    print(" -> INTEGERS <-")

    print(" -> JSON Inputs <- ")

    last = 0
    diffs = []
    for ap in range(10):
        a = 10 ** ap
        gas_json_input_u32_a = gas_of("input_json_u32_a", json_input({"a": a})) - gas_structure_noop
        diff = gas_json_input_u32_a - last
        last = gas_json_input_u32_a
        diffs.append(diff)
        diff_str = " difference for 1 digit is %s" % f(diff) if ap else ""
        if ap < 3:
            print("Cost of JSON input of (a: u32) where a=%s is %s%s" % (f(a), f(gas_json_input_u32_a), diff_str))
    print("Average diff", mean(diffs[2:]))

    last = 0
    diffs = []
    for ap in range(10):
        a = 10 ** ap
        gas_json_input_u32_ab = gas_of("input_json_u32_ab", json_input({"a": a, "b": a})) - gas_structure_noop
        diff = gas_json_input_u32_ab - last
        last = gas_json_input_u32_ab
        diffs.append(diff)
        diff_str = " difference for 2 digit is %s" % f(diff) if ap else ""
        if ap < 3:
            print("Cost of JSON input of (a: u32, b: u32) where a=b=%s is %s%s" % (f(a), f(gas_json_input_u32_ab), diff_str))
    print("Average diff", mean(diffs[2:]))


    print(" -> JSON Outputs <- ")

    last = 0
    diffs = []
    for ap in range(10):
        a = 10 ** ap
        gas_json_input_u32_a = gas_of("input_json_u32_a", json_input({"a": a}))
        gas_json_output_u32_a = gas_of("output_json_u32_a", json_input({"a": a})) - gas_json_input_u32_a
        diff = gas_json_output_u32_a - last
        last = gas_json_output_u32_a
        diffs.append(diff)
        diff_str = " difference for 1 digit is %s" % f(diff) if ap else ""
        if ap < 3:
            print("Cost of JSON output of u32=%s is %s%s" % (f(a), f(gas_json_output_u32_a), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Input/Output <- ")

    gas_borsh_input_u32_a = gas_of("input_borsh_u32_a", borsh_input("<L", 1000000000)) - gas_structure_noop
    print("Cost of Borsh input of (a: u32) is %s" % (f(gas_borsh_input_u32_a),))

    gas_borsh_input_u32_ab = gas_of("input_borsh_u32_ab", borsh_input("<LL", 1000000000, 1000000000)) - gas_structure_noop
    print("Cost of Borsh input of (a: u32, b: u32) is %s" % (f(gas_borsh_input_u32_ab),))

    gas_borsh_output_u32_a = gas_of("output_borsh_u32_a", borsh_input("<L", 1000000000)) - gas_borsh_input_u32_a - gas_structure_noop
    print("Cost of Borsh output of u32 is %s" % (f(gas_borsh_output_u32_a),))


def test_strings():
    print(" -> STRINGS <-")

    full_string = "hello world"

    print(" -> JSON Inputs <- ")

    last = 0
    diffs = []
    for sl in range(len(full_string) + 1):
        s = full_string[:sl]
        gas_json_input_string_a = gas_of("input_json_string_s", json_input({"s": s})) - gas_structure_noop
        diff = gas_json_input_string_a - last
        last = gas_json_input_string_a
        diffs.append(diff)
        diff_str = " difference for 1 char is %s" % f(diff) if sl else ""
        if sl < 3:
            print("Cost of JSON input of (s: String) where s=\"%s\" is %s%s" % (s, f(gas_json_input_string_a), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> JSON Outputs <- ")

    last = 0
    diffs = []
    for sl in range(len(full_string) + 1):
        s = full_string[:sl]
        gas_json_input_string_a = gas_of("input_json_string_s", json_input({"s": s}))
        gas_json_output_string_a = gas_of("output_json_string_s", json_input({"s": s})) - gas_json_input_string_a
        diff = gas_json_output_string_a - last
        last = gas_json_output_string_a
        diffs.append(diff)
        diff_str = " difference for 1 char is %s" % f(diff) if sl else ""
        if sl < 3:
            print("Cost of JSON output of String where s=\"%s\" is %s%s" % (s, f(gas_json_output_string_a), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Input <- ")

    last = 0
    diffs = []
    for sl in range(len(full_string) + 1):
        s = full_string[:sl]
        gas_borsh_input_string_a = gas_of("input_borsh_string_s", borsh_input("<L", len(s)) + list(s.encode('utf-8'))) - gas_structure_noop
        diff = gas_borsh_input_string_a - last
        last = gas_borsh_input_string_a
        diffs.append(diff)
        diff_str = " difference for 1 char is %s" % f(diff) if sl else ""
        if sl < 3:
            print("Cost of Borsh input of (s: String) where s=\"%s\" is %s%s" % (s, f(gas_borsh_input_string_a), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Outputs <- ")

    last = 0
    diffs = []
    for sl in range(len(full_string) + 1):
        s = full_string[:sl]
        gas_borsh_input_string_a = gas_of("input_borsh_string_s", borsh_input("<L", len(s)) + list(s.encode('utf-8')))
        gas_borsh_output_string_a = gas_of("output_borsh_string_s", borsh_input("<L", len(s)) + list(s.encode('utf-8'))) - gas_borsh_input_string_a
        diff = gas_borsh_output_string_a - last
        last = gas_borsh_output_string_a
        diffs.append(diff)
        diff_str = " difference for 1 digit is %s" % f(diff) if sl else ""
        if sl < 3:
            print("Cost of Borsh output of String where s=\"%s\" is %s%s" % (s, f(gas_borsh_output_string_a), diff_str))
    print("Average diff", mean(diffs[2:]))


def test_vec_u8():
    print(" -> Vec<u8> <-")

    vec = list(range(1, 11))

    print(" -> JSON Inputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_u8_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON input of (v: Vec<u8>) where v=%s is %s%s" % (v, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> JSON Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_u8_v", input)
        gas_output = gas_of("output_json_vec_u8_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON output of Vec<u8> where v=%s is %s%s" % (v, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Input <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for a in v:
            input += borsh_input("<B", a)
        gas_input = gas_of("input_borsh_vec_u8_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh input of (v: Vec<u8>) where v=%s is %s%s" % (v, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for a in v:
            input += borsh_input("<B", a)
        gas_input = gas_of("input_borsh_vec_u8_v", input)
        gas_output = gas_of("output_borsh_vec_u8_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh output of Vec<u8> where v=%s is %s%s" % (v, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))


def test_vec_u32():
    print(" -> Vec<u32> <-")

    vec = list(range(1, 11))

    print(" -> JSON Inputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_u32_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON input of (v: Vec<u32>) where v=%s is %s%s" % (v, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> JSON Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_u32_v", input)
        gas_output = gas_of("output_json_vec_u32_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON output of Vec<u32> where v=%s is %s%s" % (v, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Input <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for a in v:
            input += borsh_input("<L", a)
        gas_input = gas_of("input_borsh_vec_u32_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh input of (v: Vec<u32>) where v=%s is %s%s" % (v, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for a in v:
            input += borsh_input("<L", a)
        gas_input = gas_of("input_borsh_vec_u32_v", input)
        gas_output = gas_of("output_borsh_vec_u32_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh output of Vec<u32> where v=%s is %s%s" % (v, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))

def test_simple_struct():
    print(" -> SIMPLE STRUCT <-")

    gas_json_input_u32_a = gas_of("input_json_u32_a", json_input({"a": 1}))
    gas_json_input_struct_aa = gas_of("input_json_struct_a", json_input({"a": {"a": 1}}))
    diff = gas_json_input_struct_aa - gas_json_input_u32_a
    print("Extra cost of JSON input of {\"a\": {\"a\": 1}} vs {\"a\": 1} %s" % (f(diff),))

    gas_json_output_u32_a = gas_of("output_json_u32_a", json_input({"a": 1})) - gas_json_input_u32_a
    gas_json_output_struct_aa = gas_of("output_json_struct_a", json_input({"a": {"a": 1}})) - gas_json_input_struct_aa
    diff = gas_json_output_struct_aa - gas_json_output_u32_a
    print("Extra cost of JSON output of {\"a\": {\"a\": 1}} vs {\"a\": 1} %s" % (f(diff),))

    gas_borsh_input_u32_a = gas_of("input_borsh_u32_a", borsh_input("<L", 1))
    gas_borsh_input_struct_aa = gas_of("input_borsh_struct_a", borsh_input("<L", 1))
    diff = gas_borsh_input_struct_aa - gas_borsh_input_u32_a
    print("Extra cost of Borsh input of {\"a\": {\"a\": 1}} vs {\"a\": 1} %s" % (f(diff),))

    gas_borsh_output_u32_a = gas_of("output_borsh_u32_a", borsh_input("<L", 1)) - gas_borsh_input_u32_a
    gas_borsh_output_struct_aa = gas_of("output_borsh_struct_a", borsh_input("<L", 1)) - gas_borsh_input_struct_aa
    diff = gas_borsh_output_struct_aa - gas_borsh_output_u32_a
    print("Extra cost of Borsh output of {\"a\": {\"a\": 1}} vs {\"a\": 1} %s" % (f(diff),))


def test_vec_vec_u8():
    print(" -> Vec<Vec<u8>> <-")

    vec = list(range(1, 11))
    vec = [vec for _ in range(11)]

    print(" -> JSON Inputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_vec_u8_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON input of (v: Vec<Vec<u8>>) where len(v)=%d is %s%s" % (vl, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> JSON Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_vec_u8_v", input)
        gas_output = gas_of("output_json_vec_vec_u8_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON output of Vec<Vec<u8>> where len(v)=%d is %s%s" % (vl, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Input <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for a in v:
            input += borsh_input("<L", len(a))
            for b in a:
                input += borsh_input("<B", b)
        gas_input = gas_of("input_borsh_vec_vec_u8_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh input of (v: Vec<Vec<u8>>) where len(v)=%d is %s%s" % (vl, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for a in v:
            input += borsh_input("<L", len(a))
            for b in a:
                input += borsh_input("<B", b)
        gas_input = gas_of("input_borsh_vec_vec_u8_v", input)
        gas_output = gas_of("output_borsh_vec_vec_u8_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh output of Vec<Vec<u8>> where len(v)=%d is %s%s" % (vl, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))


def test_vec_string():
    print(" -> Vec<String> <-")

    vec = "hello world"
    vec = [vec for _ in range(11)]

    print(" -> JSON Inputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_string_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON input of (v: Vec<String>) where len(v)=%d is %s%s" % (vl, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> JSON Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = json_input({"v": v})
        gas_input = gas_of("input_json_vec_string_v", input)
        gas_output = gas_of("output_json_vec_string_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of JSON output of Vec<String> where len(v)=%d is %s%s" % (vl, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Input <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for s in v:
            input += borsh_input("<L", len(s))
            input += list(s.encode('utf-8'))
        gas_input = gas_of("input_borsh_vec_string_v", input) - gas_structure_noop
        diff = gas_input - last
        last = gas_input
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh input of (v: Vec<String>) where len(v)=%d is %s%s" % (vl, f(gas_input), diff_str))
    print("Average diff", mean(diffs[2:]))

    print(" -> Borsh Outputs <- ")

    last = 0
    diffs = []
    for vl in range(len(vec) + 1):
        v = vec[:vl]
        input = borsh_input("<L", len(v))
        for s in v:
            input += borsh_input("<L", len(s))
            input += list(s.encode('utf-8'))
        gas_input = gas_of("input_borsh_vec_string_v", input)
        gas_output = gas_of("output_borsh_vec_string_v", input) - gas_input
        diff = gas_output - last
        last = gas_output
        diffs.append(diff)
        diff_str = " difference for 1 element is %s" % f(diff) if vl else ""
        if vl < 3:
            print("Cost of Borsh output of Vec<String> where len(v)=%d is %s%s" % (vl, f(gas_output), diff_str))
    print("Average diff", mean(diffs[2:]))




test_integers()
test_strings()
test_vec_u8()
test_vec_u32()
test_vec_vec_u8()
test_vec_string()
test_simple_struct()
