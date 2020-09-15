This directory contains tools for the contract size minification.
Requirements:
     * cargo install wasm-snip wasm-gc
     * apt install binaryen wabt

*WARNING*: minification could be rather aggressive, so you *must* test the contract after minificaion.
Standalone NEAR runtime (https://github.com/nearprotocol/nearcore/tree/master/runtime/near-vm-runner) could be helpful
here.

Current approach to minification is the following:
    * snip (i.e. just replace with `unreachable` instruction) few known fat functions from the standard library
     (such as float formatting and panic related)
    * run wasm-gc to eliminate all functions reachable from the snipped functions
    * strip unneeded sections, such as `names`
    * run Binaryen wasm-opt, which cleans up the rest
