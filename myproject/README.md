<br />
<br />

<p>
<img src="https://nearprotocol.com/wp-content/themes/near-19/assets/img/logo.svg?t=1553011311" width="240">
</p>

<br />
<br />

## Template for NEAR dapps
### Requirements
##### IMPORTANT: Make sure you have the latest version of NEAR Shell and Node Version > 10.x 

1. [Node.js](https://nodejs.org/en/download/package-manager/)
2. (optional) near-shell

```
npm i -g near-shell
```
3. (optional) yarn
```
npm i -g yarn
```
### To run on NEAR testnet

```bash
npm install && npm dev
```

with yarn:

```bash
yarn && yarn dev
```

The server that starts is for static assets and by default serves them to http://localhost:1234. Navigate there in your browser to see the app running!

NOTE: Both contract and client-side code will auto-reload once you change source files.

### To run tests

```bash
npm test
```

with yarn:

```bash
yarn test
```

### Deploy

#### Step 1: Create account for the contract

You'll now want to authorize NEAR shell on your NEAR account, which will allow NEAR Shell to deploy contracts on your NEAR account's behalf \(and spend your NEAR account balance to do so\).

Type the command `near login` which should return a url:

```
Please navigate to this url and follow the instructions to log in:
https://wallet.nearprotocol.com/login/?title=NEAR+Shell&public_key={publicKey}
```

From there enter in your terminal the same account ID that you authorized:

`Please enter the accountId that you logged in with: <asdfasdf>`

Once you have entered your account ID, it will display the following message:

`Missing public key for <asdfasdf> in default`
`Logged in with masternode24`

This message is not an error, it just means that it will create a public key for you.

#### Step 2:

Modify `src/config.js` line that sets the account name of the contract. Set it to the account id from step 1.

NOTE: When you use [create-near-app](https://github.com/nearprotocol/create-near-app) to create the project it'll infer and pre-populate name of contract based on project folder name.

```javascript
const CONTRACT_NAME = 'react-template'; /* TODO: Change this to your contract's name! */
const DEFAULT_ENV = 'development';
...
```

#### Step 3:

Check the scripts in the package.json, for frontend and backend both, run the command:

```bash
npm run deploy
```

with yarn:

```bash
yarn deploy
```

NOTE: This uses [gh-pages](https://github.com/tschaub/gh-pages) to publish resulting website on GitHub pages. It'll only work if project already has repository set up on GitHub. Feel free to modify `deploy:pages` script in `package.json` to deploy elsewhere.

### To Explore

- `assembly/main.ts` for the contract code
- `src/index.html` for the front-end HTML
- `src/main.js` for the JavaScript front-end code and how to integrate contracts
- `src/main.test.js` for the JavaScript integration tests of smart contract