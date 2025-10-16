set -ex

cargo build --profile relwithdebinfo

xctrace record --output . --template "Time Profiler" --launch -- \
    ./target/relwithdebinfo/nanosat-rs \
    ./res/success/hardware_verification.cnf.xz
