const config = require('near-runner-jest/jest.config');

module.exports = {
  ...config,
  testMatch: [
    "**/__tests__/**/*.spec.ts"
  ]
}