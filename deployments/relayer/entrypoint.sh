#!/bin/bash
# Container entrypoint for running an IBC relayer for Penumbra,
# specifically between penumbra-testnet and penumbra-preview.
set -euo pipefail


# Generate latest configs, polling chain id from RPC endpoints
cd /usr/src/penumbra-relayer || exit 1
./generate-configs preview
./generate-configs testnet

# Generate relayer YAML config, specifying Penumbra path.
./configure-relayer
rly --debug transact link penumbra_path
cat <<EOM
##############################################
Finished configuring the relayer for Penumbra!
Starting service...
##############################################
EOM
# Run the relayer as a blocking service.
# We set a custom debug address (default is 5183) to support
# healthchecks determining whether it's running.
exec rly start penumbra_path --debug-addr 127.0.0.1:5100
