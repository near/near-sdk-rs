//@notNearfile
export * from "./outcome";

export function newStringArray(): Array<string> {
  return new Array<string>();
}

export function pushString(arr: string[], str: string): Array<string> {
  arr.push(str);
  return arr;
}