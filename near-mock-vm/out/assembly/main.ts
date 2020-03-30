function add(a: i32, b: i32): i32 {
  for (let i = 0; i < a * b; i++) {
    let x = i + a + b;
  }
  return a + b;
}
function __wrapper_add(): void {
  const obj = getInput();
  let result: i32 = add(decode<i32, JSON.Obj>(obj, "a"), decode<i32, JSON.Obj>(obj, "b"));
  const val = encode<i32>(result);
  value_return(val.byteLength, val.dataStart);
}
export { __wrapper_add as add }