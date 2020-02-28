const CONTRACT_NAME = process.env.CONTRACT_NAME ||'myproject';

function getConfig(env) {
    switch (env) {

    case 'production':
    case 'development':
        return {
            networkId: 'default',
            nodeUrl: 'http://localhost:3030',
            contractName: CONTRACT_NAME,
            walletUrl: 'https://wallet.nearprotocol.com',
            helperUrl: 'https://near-contract-helper.onrender.com',
        };
    case 'staging':
        return {
            networkId: 'staging',
            nodeUrl: 'https://staging-rpc.nearprotocol.com/',
            contractName: CONTRACT_NAME,
            walletUrl: 'https://near-wallet-staging.onrender.com',
            helperUrl: 'https://near-contract-helper-staging.onrender.com',
        };
    case 'local':
        return {
            networkId: 'local',
            nodeUrl: 'http://localhost:3030',
            keyPath: `${process.env.HOME}/.near/validator_key.json`,
            walletUrl: 'http://localhost:4000/wallet',
            contractName: CONTRACT_NAME,
        };
    case 'test':
    case 'ci':
        return {
            networkId: 'shared-test',
            nodeUrl: 'http://shared-test.nearprotocol.com:3030',
            contractName: CONTRACT_NAME,
            masterAccount: 'test.near',
        };
    case 'ci-staging':
        return {
            networkId: 'shared-test-staging',
            nodeUrl: 'http://staging-shared-test.nearprotocol.com:3030',
            contractName: CONTRACT_NAME,
            masterAccount: 'test.near',
        };
    case 'tatooine':
        return {
            networkId: 'tatooine',
            nodeUrl: 'https://rpc.tatooine.nearprotocol.com',
            contractName: CONTRACT_NAME,
            walletUrl: 'https://wallet.tatooine.nearprotocol.com',
        };
    default:
        throw Error(`Unconfigured environment '${env}'. Can be configured in src/config.js.`);
    }
}

module.exports = getConfig;
