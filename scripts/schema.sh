START_DIR=$(pwd)

# ${f    <-- from variable f
#   ##   <-- greedy front trim
#   *    <-- matches anything
#   /    <-- until the last '/'
#  }
# <https://stackoverflow.com/a/3162500>

echo "generating schema for account contract"
cd contracts/account
cargo run --example schema > /dev/null
rm -rf ./schema/raw
cd "$START_DIR"

echo "generating schema for treasury contract"
cd contracts/treasury
cargo run --example schema > /dev/null
rm -rf ./schema/raw
cd "$START_DIR"