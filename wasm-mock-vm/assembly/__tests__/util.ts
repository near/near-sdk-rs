import {TextMessage} from "./model";

//Testing helper functions
export function _testBytes(): Uint8Array {
  const array = new Uint8Array(4);
  array[0] = 0;
  array[1] = 1;
  array[2] = 90;
  array[3] = 100;
  return array;
}
export function _testBytesTwo(): Uint8Array {
  const array = new Uint8Array(3);
  array[0] = 8;
  array[1] = 2;
  array[2] = 101;
  return array;
}
export function _testTextMessage(): TextMessage {
  const message = new TextMessage();
  message.sender = "mysteriousStranger";
  message.text = "Hello world";
  message.number = 415;
  return message;
}
export function _testTextMessageTwo(): TextMessage {
  const message = new TextMessage();
  message.sender = "joe";
  message.text = "Howdy";
  message.number = 20;
  return message;
}

export function roundtrip<T>(obj: T): T {
  return decode<T>(encode<T>(obj));
}