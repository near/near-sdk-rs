import { context, storage, base58, base64, PersistentMap, PersistentVector, PersistentDeque, PersistentTopN, ContractPromise, math, logging, runtime_api } from "near-runtime-ts";
import { TextMessage } from "./model";
import { _testTextMessage, _testTextMessageTwo, _testBytes, _testBytesTwo } from "./util";
import { Context, VM } from "..";
import { u128 } from "near-runtime-ts";
import { Outcome } from "../entry";

beforeAll(()=> {
  VM.saveState();
})


// export function hello(): string {
//   const s = simple("a"); // Test that we can call other export functions
//   return "hello".concat(s);
// }

describe("Encodings", () => {
  it("base58 round trip", () => {
    let array: Uint8Array = _testBytes();
    const encoded = base58.encode(array);
    expect(encoded).toBe("1TMu", "Wrong encoded value for base58 encoding")
  });
  
  it("base64 round trip", () => {
    const array = _testBytes();
    const encoded = base64.encode(array);
    expect(encoded).toBe("AAFaZA==", "Incorrect keys contents");
    const decoded = base64.decode("AAFaZA==");
    expect(decoded).toStrictEqual(decoded, "Incorrect decoded value after base64 roundtrip");
  });
});

let outcome: Outcome;
describe("outcome", () => {
  beforeAll(()=> {
    VM.saveState();
  });
  afterAll(() => {
    VM.restoreState();
  });

  it("should return acturate logs", () => {
    logging.log("hello world");
    let outcome = VM.outcome();
    expect(outcome.logs).toIncludeEqual("hello world", "log should include \"hello world\"");
  });

  it("should increase the storage usage more when first added", () => {
    const key = "hello";
    const key_cost = String.UTF8.byteLength(key);
    const value = "world";
    const value_cost = String.UTF8.byteLength(value);
    const initial_base_cost = 40; 

    if (storage.contains("hello")) {
      storage.delete("hello");
    }
    let orig = VM.outcome();
    expect(orig.storage_usage).toBe(runtime_api.storage_usage(), "it should be the same when no storage has been added.");
    storage.set("hello", "world");
    expect(storage.get<string>("hello")).toBe("world", "key hello should be set to \"world\"");
    let newOutcome = VM.outcome();
    expect(newOutcome.storage_usage).toBeGreaterThan(orig.storage_usage, "the new storage usage should be greater than the original.");
    expect(runtime_api.storage_usage()).toBe(initial_base_cost + key_cost + value_cost + orig.storage_usage, "first write cost 40 + length of key + length of value");
  });

  it("should decrease storage usage with smaller value", () => {
    const oldVaule  = "world";
    const newValue = "wor";
    const lenDiff = String.UTF8.byteLength(oldVaule) - String.UTF8.byteLength(newValue);
    const usageBefore = runtime_api.storage_usage();
    storage.set("hello", newValue);
    expect(runtime_api.storage_usage()).toBe(usageBefore - lenDiff, "storage usage should be less with smaller value.");
  });
});

describe("Storage", (): void => {
  beforeEach(() => {
    VM.saveState();
  });

  afterEach(() => {
    VM.restoreState();
  });

  it("String Roundtrip", () => {
    storage.setString("someKey", "myValue1");
    storage.setString("someOtherKey", "otherValue");
    expect(storage.getString("someKey")).toBe("myValue1");
    // expect(getValueResult).toBe("myValue1", "Incorrect value from storage");
    const otherValueResult = storage.getString("someOtherKey");
    expect(otherValueResult).toBe("otherValue", "Incorrect value from someOtherKey from storage");
    expect(storage.getString("nonexistentKey")).toBeNull("Unexpectd value on getting string with a nonexistent key");
  });

  it("Bytes Roundtrip", () => {
    const bytes = _testBytes();
    const bytes2 = _testBytesTwo();
    storage.setBytes("someKey", bytes);
    storage.setBytes("someOtherKey", bytes2);
    const getValueResult = storage.getBytes("someKey");
    expect(getValueResult).toBe(getValueResult, "Incorrect bytes value from storage");
    const otherValueResult = storage.getBytes("someOtherKey");
    expect(otherValueResult).toBe(otherValueResult, "Incorrect bytes value from storage");
    expect(storage.getBytes("nonexistentKey")).toBeNull("Unexpectd value on getting bytes with a nonexistent key");
  });
  
  it("Generic Get/Set Roundtrip", () => {
    const message = _testTextMessage();
    storage.set("message1", message);
  
    const messageFromStorage = storage.get<TextMessage>("message1")!;
    expect(messageFromStorage.sender).toBe("mysteriousStranger", "Incorrect data value (sender) for retrieved object");
    expect(messageFromStorage.text).toBe("Hello world", "Incorrect data value (text) for retrieved object");
    expect(messageFromStorage.number).toBe(415, "Incorrect data value (number) for retrieved object");
    expect(storage.get<TextMessage>("nonexistent", null)).toBeNull("Incorrect data value for get<T> nonexistent key");
  
    storage.set<TextMessage>("message2", new TextMessage());
    // TODO: fix this
    expect(storage.get<TextMessage>("message2")).toStrictEqual(new TextMessage(), "Incorrect empty message on storage roundtrip");
  
    storage.set<u64>("u64key", 20);
    expect(storage.getPrimitive<u64>("u64key", 0)).toBe(20, "Incorrect data value for u64 roundtrip");
    expect(storage.getPrimitive<u64>("nonexistent", 1)).toBe(1, "Incorrect data value for u64 get nonexistent key");
  
    storage.set<u32>("u32key", 12);
    expect(storage.getPrimitive<u32>("u32key", 0)).toBe(12, "Incorrect data value for u32 roundtrip");
    expect(storage.getPrimitive<u32>("nonexistent", 2)).toBe(2, "Incorrect data value for u32 get nonexistent key");
  
    storage.set<i32>("i32key", -5);
    expect(storage.getPrimitive<i32>("i32key", 0)).toBe(-5, "Incorrect data value for i32 roundtrip");
    expect(storage.getPrimitive<i32>("nonexistent", -10)).toBe(-10, "Incorrect data value for i32 get nonexistent key");
  
    storage.set<bool>("boolkey", true);
    expect(storage.getPrimitive<bool>("boolkey", 0)).toBe(true, "Incorrect data value for bool roundtrip");
    expect(storage.getPrimitive<bool>("nonexistent", true)).toBe(true, "Incorrect data value for bool get nonexistent key");
  
    storage.set<String>("stringkey", "StringValue");
    const stringGet = storage.get<String>("stringkey");
    expect(stringGet).toBe("StringValue", "Incorrect data value for string roundtrip");
    expect(storage.get<string>("nonexistent", null)).toBeNull("Incorrect data value for get<T> string nonexistent key");
  });
  
  it("Keys", () => {
    storage.delete("someKey");
    // empty storage
    const emptyKeys = storage.keys("someKey");
    expect(emptyKeys.length).toBe(0, "Incorrect keys contents for empty storage");
  
    // // add some keys
    storage.setString("someApple", "myApple");
    storage.setString("someKey", "myValue1");
    storage.setString("someKey2", "myValue1");
    storage.setString("someKey6", "myValue2");
  
    const keyRange = storage.keyRange("someKey", "someKey3");
    expect(keyRange.length).toBe(2, "Incorrect keys length");
    expect(keyRange[0]).toBe("someKey", "Incorrect keys contents");
    expect(keyRange[1]).toBe("someKey2", "Incorrect keys contents");
    const keyRangeWithLimit = storage.keyRange("someKey", "someKey3", 1);
    expect(keyRangeWithLimit.length).toBe(1, "Incorrect keys length");
    expect(keyRangeWithLimit[0]).toBe("someKey", "Incorrect keys contents");
  
    const keys = storage.keys("someKey");
    expect(keys.length).toBe(3, "Incorrect keys length");
    expect(keys[0]).toBe("someKey", "Incorrect keys contents");
    expect(keys[1]).toBe("someKey2", "Incorrect keys contents");
    expect(keys[2]).toBe("someKey6", "Incorrect keys contents");
    const keysWithLimit = storage.keys("someKey", 1);
    expect(keysWithLimit.length).toBe(1, "Incorrect keys length");
    expect(keys[0]).toBe("someKey", "Incorrect keys contents")
  
    expect(storage.contains("someApple")).toBe(true, "Storage does not contain key");
    expect(storage.contains("someKey")).toBe(true, "Storage does not contain key");
    expect(storage.contains("someKey2")).toBe(true, "Storage does not contain key");
    expect(storage.contains("someKey6")).toBe(true, "Storage does not contain key");
    expect(!storage.contains("nonexisting")).toBe(true, "Storage has unexpected key");
    expect(storage.hasKey("someKey6")).toBe(true, "Storage does not contain key");
    expect(!storage.hasKey("nonexisting")).toBe(true, "Storage has unexpected key");
  
    // remove a key and retry some of the api calls
    storage.delete("someKey");
    expect(!storage.contains("someKey")).toBe(true, "Storage contains key that was deleted");
    expect(storage.contains("someKey2")).toBe(true, "Some other key got deleted");
    const keyswithdelete = storage.keys("someKey");
    expect(keyswithdelete.length).toBe(2, "Incorrect keys length after removing a key")
    expect(keyswithdelete[0]).toBe("someKey2", "Incorrect keyswithdelete contents");
    expect(keyswithdelete[1]).toBe("someKey6", "Incorrect keyswithdelete contents");
  });
});


describe("Map should handle", () => {
  it("empty maps", () => {
    // TODO: values
    // empty map
    const map = new PersistentMap<string, TextMessage>("mapId");
    const valuesEmpty = map.values("", "zzz");
    expect(valuesEmpty.length).toBe(0, "Unexpected values in empty map");
    expect(!map.contains("nonexistentkey")).toBe(true, "Map contains a non existent key");
    expect(map.get("nonexistentkey")).toBeNull("Incorrect result on get with nonexistent key");
  });

  it("some entries", () => {
    const map = new PersistentMap<string, TextMessage>("mapId");
    // add some entries to the map
    const message = _testTextMessage();
    map.set("mapKey1", message);
    map.set("mapKey3", _testTextMessageTwo());
    expect(map.contains("mapKey1")).toBe(true);
    const values = map.values("mapKey1", "zzz");
    expect(values.length).toBe(2, "Unexpected values size in map with 2 entries");
    expect(values[0]).toStrictEqual(message, "Unexpected values contents in map with 2 entries");
    expect(values[1]).toStrictEqual(_testTextMessageTwo(), "Unexpected values contents in map with 2 entries");
    expect(map.values("mapKey3", "zzz").length).toBe(1, "Unexpected values size in map with 2 entries");
    expect(map.values("mapKey1", "mapKey2").length).toBe(1, "Unexpected values size in map with 2 entries");
    expect(map.values("mapKey1", "mapKey4", -1, false).length).toBe(1, "Unexpected values size in map with 2 entries");
    expect(!map.contains("nonexistentkey")).toBe(true, "Map contains a non existent key");
    expect(map.contains("mapKey1")).toBe(true, "Map does not contain a key that was added (mapKey1)");
    expect(map.contains("mapKey3")).toBe(true, "Map does not contain a key that was added (mapKey3)");
    expect(map.get("mapKey1")).toStrictEqual(message, "Incorrect result from map get");
    expect(map.get("mapKey3")).toStrictEqual(_testTextMessageTwo(), "Incorrect result from map get");
    // delete an entry and retry api calls
    map.delete("mapKey3");
    expect(map.values("", "zzz").length).toBe(1, "Unexpected values size in map after delete");
    expect(map.values("", "zzz")[0]).toStrictEqual(message, "Unexpected values contents in map after delete");
    expect(map.values("mapKey1", "zzz").length).toBe(1, "Unexpected values size in map after delete");
    expect(!map.contains("mapKey3")).toBe(true, "Map contains a key that was deleted");
    expect(map.contains("mapKey1")).toBe(true, "Map does not contain a key that should be there after deletion of another key");
    expect(map.get("mapKey1")).toStrictEqual(message, "Incorrect result from map get after delete");
    expect(map.get("mapKey3")).toBeNull("Incorrect result from map get on a deleted key");
  });

  it("should handle primitives", () => {
    // map with primitives
    const map = new PersistentMap<i32, i32>("mapPrimitives");
    map.set(1, -20);
    expect(map.getSome(1)).toBe(-20, "wrong value on map get for i32");
  });
  
  it("should handle arrays", () => {
    // map with arrays
    const map = new PersistentMap<i32, Array<string>>("mapArray");
    const arr1 = new Array<string>();
    arr1.push("123456789");
    // return arr1;
    map.set(1, arr1);
    expect(map.getSome(1)[0]).toBe("123456789");
  });
});

const vector = new PersistentVector<string>("vector1");

describe("Vectors should handle", () => {
  //TODO: Improve tests
  it("no items", () => {
    expect(vector != null).toBe(true, "Vector not initialized");
    expect(vector.length).toBe(0, "Empty vector has incorrect length");
    expect(!vector.containsIndex(0)).toBe(true, "Empty vector incorrectly has index 0");
    expect(vector.isEmpty).toBe(true, "isEmpty incorrect on empty vector");
    //try { expect(vector[0]).toBeNull("");} catch (e) {} not possible to test due to lack of try catch
  });
  
  it("single items", () => {
    vector.push("bb");
    expect(vector.length).toBe(1, "Vector has incorrect length");
    expect(vector.containsIndex(0)).toBe(true, "Non empty vector does not have index 0");
    expect(!vector.containsIndex(1)).toBe(true, "Vector size 1 incorrectly has index 1");
    expect(!vector.isEmpty).toBe(true, "isEmpty incorrect on non-empty vector");
    expect(vector.back).toBe("bb", "Incorrect back entry");
    expect(vector.last).toBe("bb", "Incorrect last entry");
    expect(vector.front).toBe("bb", "Incorrect front entry");
    expect(vector.first).toBe("bb", "Incorrect first entry");
    expect(vector[0]).toBe("bb", "incorrect vector contents");
    expect(_vectorHasContents(vector, ["bb"])).toBe(true, "Unexpected vector contents. Expected [bb]");
  });
  
  it("two items", () => {
    vector.pushBack("bc");
    expect(vector.length).toBe(2, "Vector has incorrect length");
    expect(vector.containsIndex(0)).toBe(true, "Non empty vector does not have index 0");
    expect(vector.containsIndex(1)).toBe(true, "Vector size 2 does not have index 1");
    expect(!vector.containsIndex(2)).toBe(true, "Vector size 2 incorrectly has index 2");
    expect(!vector.isEmpty).toBe(true, "isEmpty incorrect on non-empty vector");
    expect(_vectorHasContents(vector, ["bb", "bc"])).toBe(true, "Unexpected vector contents. Expected [ba, bb]");
    vector[1] = "bd";
    expect(_vectorHasContents(vector, ["bb", "bd"])).toBe(true, "Unexpected vector contents. Expected [ba, bd]");

    vector[0] = "aa";
    expect(_vectorHasContents(vector, ["aa", "bd"])).toBe(true, "Unexpected vector contents. Expected [aa, bd]");
    expect(vector.length).toBe(2, "Vector has incorrect length")
  });
  
  it("three items", () => {
    vector.pushBack("be");
    expect(_vectorHasContents(vector, ["aa", "bd", "be"])).toBe(true, "Unexpected vector contents. Expected [aa, bd, be]");
    expect(vector.length).toBe(3, "Vector has incorrect length")
    expect(vector.back).toBe("be", "Incorrect back entry")
    expect(vector.last).toBe("be", "Incorrect last entry")
    expect(vector.front).toBe("aa", "Incorrect front entry")
    expect(vector.first).toBe("aa", "Incorrect first entry")
  });
  
  it("popping from the front", () => {
    //pop an entry and then try various other methods
    vector.pop();
    expect(_vectorHasContents(vector, ["aa", "bd"])).toBe(true, "Unexpected vector contents. Expected [aa, bd]");
    expect(vector.length).toBe(2, "Vector has incorrect length after delete")
    vector[0] = "ba";
    expect(_vectorHasContents(vector, ["ba", "bd"])).toBe(true, "Unexpected vector contents. Expected [ba, bd]");
    expect(vector.length).toBe(2, "Vector has incorrect length");
  });
  
  it("popping from b items", () => {
    vector.pushBack("bf");
    expect(_vectorHasContents(vector, ["ba", "bd", "bf"])).toBe(true, "Unexpected vector contents. Expected [ba, bd, bf]");
    expect(vector.length).toBe(3, "Vector has incorrect length")
    vector.popBack();
    expect(_vectorHasContents(vector, ["ba", "bd"])).toBe(true, "Unexpected vector contents. Expected [ba, bd]");
    expect(vector.length).toBe(2, "Vector has incorrect length");

    // same id but different object.
    const vectorReread = new PersistentVector<string>("vector1");
    expect(vectorReread != null).toBe(true, "Vector not initialized");
    expect(vectorReread.length).toBe(2, "Vector has incorrect length");

    // vector with primitives
    const vectorI32 = new PersistentVector<i32>("vectori32");
    vectorI32.pushBack(2);
    expect(vectorI32.length).toBe(1, "Vector i32 has incorrect length");
  });
});

const deque = new PersistentDeque<string>("dequeid");

describe("Deque should handle", () => {
  it("no items", () => {
    expect(deque.length).toBe(0, "empty deque length is wrong");
    expect(!deque.containsIndex(0)).toBe(true, "empty deque contains index 0");
    expect(deque.isEmpty).toBe(true, "empty deque returns false for isEmpty");
  });
  
  it("single items", () => {
    deque.pushBack("1");
    expect(deque.length).toBe(1, "deque length is wrong");
    expect(deque.containsIndex(0)).toBe(true, "deque does not contain index 0");
    expect(!deque.containsIndex(-1)).toBe(true, "deque does not contain index 0");
    expect(deque.isEmpty).toBe(false, "deque returns true for isEmpty");
    expect(deque[0]).toBe("1", "wrong element value using []");
    expect(deque.back).toBe("1", "wrong back element");
    expect(deque.front).toBe("1", "wrong front element");
    expect(deque.first).toBe("1", "wrong first element");
    expect(deque.last).toBe("1", "wrong last element");
  });
  
  it("multiple items", () => {
    deque.pushFront("-2");
    expect(deque.length).toBe(2, "deque length is wrong");
    expect(deque.containsIndex(0)).toBe(true, "deque does not contain index 0");
    expect(deque.containsIndex(1)).toBe(true, "deque does not contain index 1");
    expect(deque.isEmpty).toBe(false, "deque returns true for isEmpty");
    expect(deque[1]).toBe("1", "wrong element value using []");
    expect(deque[0]).toBe("-2", "wrong element value using []");
    expect(deque.back).toBe("1", "wrong back element");
    expect(deque.front).toBe("-2", "wrong front element");
    expect(deque.first).toBe("-2", "wrong first element");
    expect(deque.last).toBe("1", "wrong last element");
  });
  
  it("popping front", () => {
    deque.popFront();
    expect(deque.length).toBe(1, "deque length is wrong");
    expect(deque.containsIndex(0)).toBe(true, "deque does not contain index 0");
    expect(!deque.containsIndex(1)).toBe(true, "deque of length 1 contains index 1");
    expect(deque.isEmpty).toBe(false, "deque returns true for isEmpty");
    expect(deque[0]).toBe("1", "wrong element value using []");
    expect(deque.back).toBe("1", "wrong back element");
    expect(deque.front).toBe("1", "wrong front element");
    expect(deque.first).toBe("1", "wrong first element");
    expect(deque.last).toBe("1", "wrong last element");
  });
  
  it("popping back", () => {
    deque.pushFront("-2");
    deque.popBack();
    expect(deque.length).toBe(1, "deque length is wrong");
    expect(deque.containsIndex(0)).toBe(true, "deque does not contain index 0");
    expect(!deque.containsIndex(1)).toBe(true, "deque of length 1 contains index 1");
    expect(deque.isEmpty).toBe(false, "deque returns true for isEmpty");
    expect(deque[0]).toBe("-2", "wrong element value using []");
    expect(deque.back).toBe("-2", "wrong back element");
    expect(deque.front).toBe("-2", "wrong front element");
    expect(deque.first).toBe("-2", "wrong first element");
    expect(deque.last).toBe("-2", "wrong last element");
  });
});

let topn: PersistentTopN<string>;
describe("TopN should", () => {

  beforeAll(() => {
    topn = new PersistentTopN<string>("topnid");
  });
    it("handle empty collection", () => {
      // empty topn cases
    expect(topn != null).toBe(true, "topn is null");
    expect(topn.isEmpty).toBe(true, "empty topn - wrong result for isEmpty");
    expect(topn.length).toBe(0, "empty topn - wrong length");
    expect(!topn.contains("nonexistentKey")).toBe(true, "empty topn - contains nonexistent key");
    topn.delete("nonexistentKey"); // this should not crash
    expect(topn.keysToRatings(new Array<string>(0)).length).toBe(0, "keys to ratings for empty topn is not empty");
    expect(topn.getTop(10).length).toBe(0, "get top for empty topn returned non empty list")
    // expect(topn.getTopFromKey(10, "somekey").length).toBe(0, "getTopFromKey for empty topn returned non empty list") // fails due to key doesn't exist
    expect(topn.getTopWithRating(10).length).toBe(0, "getTopWithRating for empty topn is not empty");
    // expect(topn.getTopWithRatingFromKey(10, "somekey").length).toBe(0, "getTopWithRatingFromKey for empty topn is not empty"); // fails due to key doesn't exist
  });
  
  it("handle single items", () => {
    topn.setRating("k1", 5);
    expect(!topn.isEmpty).toBe(true, "topn - wrong result for isEmpty");
    expect(topn.length).toBe(1, "topn - wrong length");
    expect(!topn.contains("nonexistentKey")).toBe(true, "topn - contains nonexistent key");
    expect(topn.contains("k1")).toBe(true, "topn - does not contain a key that should be there");
    topn.delete("nonexistentKey"); // this should not crash
    expect(topn.keysToRatings(["k1"]).length).toBe(1, "keys to ratings wrong for topn");
    expect(topn.keysToRatings(["k1"])[0].value).toBe(5, "keys to ratings wrong for topn");
    expect(topn.getTop(10).length).toBe(1, "get top for topn returned non empty list");
    expect(topn.getTop(10)[0]).toBe("k1", "wrong key in getTop")
    expect(topn.getTopFromKey(10, "k1").length).toBe(0, "getTopFromKey for topn wrong result");
    expect(topn.getTopWithRating(10).length).toBe(1, "getTopWithRating for topn with 1 element is wrong size");
    expect(topn.getTopWithRatingFromKey(10, "k1").length).toBe(0, "getTopWithRatingFromKey for topn is not empty");
  });

  
  it("handle two entries", () => {
    topn.setRating("k", 5);
    topn.incrementRating("k1");
    expect(!topn.isEmpty).toBe(true, "topn - wrong result for isEmpty");
    expect(topn.length).toBe(2, "topn - wrong length");
    expect(!topn.contains("nonexistentKey")).toBe(true, "topn - contains nonexistent key");
    expect(topn.contains("k")).toBe(true, "topn - does not contain a key that should be there");
    expect(topn.contains("k1")).toBe(true, "topn - does not contain a key that should be there");
    topn.delete("nonexistentKey"); // this should not crash
    expect(topn.keysToRatings(["k1"]).length).toBe(1, "keys to ratings wrong for topn");
    expect(topn.keysToRatings(["k1", "k"]).length).toBe(2, "keys to ratings wrong for topn");
    expect(topn.keysToRatings(["k1", "k"])[0].value).toBe(6, "keys to ratings wrong for topn");
    expect(topn.keysToRatings(["k1", "k"])[1].value).toBe(5, "keys to ratings wrong for topn");
    expect(topn.getTop(10).length).toBe(2, "get top for topn is wrong");
    expect(topn.getTop(10)[0]).toBe("k1", "wrong key in getTop");
    expect(topn.getTop(10)[1]).toBe("k", "wrong key in getTop");
    expect(topn.getTop(1).length).toBe(1, "get top for topn is wrong when limit is applied");
    expect(topn.getTop(1)[0]).toBe("k1", "wrong key in getTop");
    expect(topn.getTopFromKey(10, "k").length).toBe(0, "getTopFromKey for topn wrong result");
    expect(topn.getTopFromKey(10, "k1").length).toBe(1, "getTopFromKey for topn wrong result");
    expect(topn.getTopFromKey(10, "k1")[0]).toBe("k", "getTopFromKey for topn wrong result");
    expect(topn.getTopWithRating(10).length).toBe(2, "getTopWithRating for topn with 1 element is wrong size");
    expect(topn.getTopWithRating(10)[0].value).toBe(6, "getTopWithRating for topn with 1 element is wrong size");
    expect(topn.getTopWithRating(10)[1].value).toBe(5, "getTopWithRating for topn with 1 element is wrong size");
  });

  it("handle deleting items", () => {
    topn.delete("k1");
    topn.incrementRating("k");
    expect(!topn.isEmpty).toBe(true, "topn - wrong result for isEmpty");
    expect(topn.length).toBe(1, "topn - wrong length");
    expect(!topn.contains("nonexistentKey")).toBe(true, "topn - contains nonexistent key");
    expect(topn.contains("k")).toBe(true, "topn - does not contain a key that should be there");
    topn.delete("nonexistentKey"); // this should not crash
    expect(topn.keysToRatings(["k"]).length).toBe(1, "keys to ratings wrong for topn");
    expect(topn.keysToRatings(["k"])[0].value).toBe(6, "keys to ratings wrong for topn");
    expect(topn.getTop(10).length).toBe(1, "get top for topn returned non empty list");
    expect(topn.getTop(10)[0]).toBe("k", "wrong key in getTop");
    expect(topn.getTopFromKey(10, "k").length).toBe(0, "getTopFromKey for topn wrong result");
    expect(topn.getTopWithRating(10).length).toBe(1, "getTopWithRating for topn with 1 element is wrong size");
    expect(topn.getTopWithRatingFromKey(10, "k").length).toBe(0, "getTopWithRatingFromKey for topn is not empty");
  });
});

describe("context", () => {

  beforeEach(() => {
    Context.saveContext();
  });

  afterEach(() => {
    Context.restoreContext();
  });

  it("should read unchanged context", () => {
    expect(context.sender).toBe("bob", "Wrong sender");
    expect(context.attachedDeposit).toBe(u128.fromU64(2), "Wrong receivedAmount");
    expect(context.accountBalance).toBe(u128.fromU32(4), "Account Balance should inclode attached deposit");
  });
  
  describe("Account Balance", () => {
      it("should be updated when attached attached deposit is updated", () => {
          Context.setAttached_deposit(u128.from(4));
          expect(context.accountBalance).toStrictEqual(u128.from(6), "Updating the attached deposit should update the account balance");
        });
    });
      
      
  it("should be editable", () => {
    Context.setCurrent_account_id("contractaccount");
    expect(context.contractName).toBe("contractaccount", "Wrong contract name");
    Context.setBlock_index(113);
    expect(context.blockIndex).toBe(113, "Wrong contract name");
    Context.setAttached_deposit(u128.from(7));
    expect(context.attachedDeposit.toString()).toBe(u128.fromU64(7).toString(), "Wrong receivedAmount");
    // Context.setAccount_balance(u128.from(14))
    
    // expect(context.accountBalance).toBe(u128.fromU64(14), "Wrong receivedAmount");
    Context.setPrepaid_gas(1000000000);
    expect(context.prepaidGas).toBe(1000000000, "Wrong prepaid gas");
    expect(context.usedGas <= 1000000000).toBe(true, "Wrong used gas");
    // expect(context.usedGas > 0).toBe(true, "Wrong used gas");
    //expect(context.storageUsage).toBe(0, "Wrong storage usage"); TODO: test when implemented
  });
});

describe("promises", () => {
  it("should work", () => {
    const emptyResults = ContractPromise.getResults();
    expect(emptyResults.length).toBe(0, "wrong length for results");
    const promise = ContractPromise.create("contractNameForPromise", "methodName", new Uint8Array(0), 10000000000000);
    const promise2 = promise.then("contractNameForPromise", "methodName", new Uint8Array(0), 10000000000000);
    const promise3 = ContractPromise.all([promise2]);
  });
});

const stringValue = "toHash";

describe("Math should handle", () => {
  it("hash 32 from bytes", () => {
    const array = _testBytes();
    const hash = math.hash32Bytes(array);
    expect(hash).toBe(3593695342, "wrong hash");
  });

  it("hash 32 from string ", () => {
    const hashOfString = math.hash32(stringValue);
    expect(hashOfString).toBe(3394043202, "wrong hash of the string");
  });

  it("hash Uint8Aray from string", () => {
    const hash256 = math.hash(stringValue);
    let x: i32[] = [1, 6, 7];
    expect(hash256.length).toBe(32, "wrong output length for hash256");
    expect(hash256[0]).toBe(202, "wrong contents of hash256");
    expect(hash256[1]).toBe(76, "wrong contents of hash256");
    expect(hash256[31]).toBe(184, "wrong contents of hash256");
  });

  it("handle random", () => {

    const randBuf = math.randomBuffer(14);
    const randBuf2 = math.randomBuffer(14);
    const randBuf3 = math.randomBuffer(14);
    const randBuf4 = math.randomBuffer(32);
    const randBuf5 = math.randomBuffer(35);
  });
});


// cruft to compare test objects. TODO: use something standard
function _arrayEqual(first: Uint8Array | null, second: Uint8Array | null): bool {
  if (first == null || second == null) {
    return first == second;
  }
  if (first.length != second.length) {
    return false;
  }
  for (let i = 0; i < first.length; ++i) {
    if (first[i] != second[i]) {
      return false;
    }
  }
  return true;
}

function _modelObjectEqual(first: TextMessage | null, second: TextMessage | null): bool {
  //@ts-ignore
  if (first == null) {
    return second == null;
  }
  if (second == null) return false;
  if (first.sender != second.sender) {
    return false;
  }
  if (first.text != second.text) {
    return false;
  }
  if (first.number != second.number) {
    return false;
  }
  return true;
}

function _vectorHasContents(vector: PersistentVector<string>, expectedContents: Array<string>): bool {
  if (vector.length != expectedContents.length) {
    return false;
  }
  for (let i = 0; i < expectedContents.length; i++) {
    if (expectedContents[i] != vector[i]) {
      //return false;
    }
  }
  return true;
}

export function simple(s: string): string {
  return s;
}

export function setKeyValue(key: string, value: string): void {
  storage.set<string>(key, value);
}

export function getValueByKey(key: string): string | null {
  return storage.get<string>(key);
}

export function setValue(value: string): string {
  storage.set<string>("name", value);
  return value;
}

export function getValue(): string | null {
  return storage.get<string>("name");
}
