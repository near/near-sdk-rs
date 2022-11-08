use near_sdk::NearSchema;

struct Inner;

#[derive(NearSchema)]
struct Outer(Inner);

#[derive(NearSchema)]
#[abi]
struct Nada;

#[derive(NearSchema)]
#[abi()]
struct Empty;

#[derive(NearSchema)]
#[abi(serde)]
struct SingleUnexpected;

#[derive(NearSchema)]
#[abi(json, serde)]
struct OneUnexpected;

#[derive(NearSchema)]
#[abi(json, serde, schemars)]
struct TwoUnexpected;

#[derive(NearSchema)]
#[abi(json, serde = "?")]
struct OneUnexpectedPath;

fn main() {}
