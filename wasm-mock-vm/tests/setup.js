
const utils = require('./utils');

module.exports = async () => {
  const res = {
    decode: (bs) => {
      return JSON.parse(utils.UTF8toStr(bs));
    },
    encode: (obj) => {
      return utils.StrtoUTF8(JSON.stringify(obj));
    }
  }
  Object.setPrototypeOf(Object, {...Object.prototype, 
  ...res})
  return res
}