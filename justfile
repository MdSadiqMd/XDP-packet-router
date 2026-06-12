set dotenv-load := false

# Default recipe - show help
default:
    @just --list

# Create and start the Lima VM for XDP development
vm-create:
    limactl create --name=xdp ./lima.yaml
    limactl start xdp

# Start the Lima VM
vm-start:
    limactl start xdp

# Stop the Lima VM
vm-stop:
    limactl stop xdp

# Delete the Lima VM
vm-delete:
    limactl delete xdp --force

# Show Lima VM status
vm-status:
    limactl list

# Enter the Lima VM shell
vm-shell:
    limactl shell xdp

# Build the XDP loader (must run inside Lima VM)
build:
    cargo build --release -p pshred-loader

# Build in debug mode
build-debug:
    cargo build -p pshred-loader

# Build the test sender
build-test-sender:
    cargo build --release --bin test-sender

# Build all binaries
build-all: build build-test-sender

# Clean build artifacts
clean:
    cargo clean

# Run XDP router on loopback (for testing)
run-lo port="8001":
    sudo ./target/release/pshred-loader -i lo -p {{port}} --skb-mode

# Run XDP router on eth0
run-eth0 port="8001":
    sudo ./target/release/pshred-loader -i eth0 -p {{port}}

# Run XDP router on eth0 with SKB mode
run-eth0-skb port="8001":
    sudo ./target/release/pshred-loader -i eth0 -p {{port}} --skb-mode

# Run XDP router with custom interface
run interface="eth0" port="8001" skb="":
    sudo ./target/release/pshred-loader -i {{interface}} -p {{port}} {{skb}}

# Run XDP router with auto-redirect to veth interfaces
run-redirect interface="eth0" port="8001" num_proposers="16":
    sudo ./target/release/pshred-loader -i {{interface}} -p {{port}} --auto-redirect {{num_proposers}}

# Send test packets to localhost
test-send target="127.0.0.1:8001" proposers="4" count="100":
    ./target/release/test-sender -t {{target}} -n {{proposers}} -c {{count}}

# Run full test: start router, send packets, show stats
test: build-all
    #!/usr/bin/env bash
    set -e
    echo "Starting XDP router on loopback..."
    sudo ./target/release/pshred-loader -i lo -p 8001 --skb-mode --stats-interval 2 &
    LOADER_PID=$!
    sleep 2
    echo "Sending 100 test packets (4 proposers)..."
    ./target/release/test-sender -t 127.0.0.1:8001 -n 4 -c 100 --delay-ms 5
    echo "Waiting for stats..."
    sleep 3
    echo "Stopping router..."
    sudo kill $LOADER_PID 2>/dev/null || true
    echo "Done!"

# Quick smoke test
test-quick: build-all
    #!/usr/bin/env bash
    set -e
    sudo ./target/release/pshred-loader -i lo -p 8001 --skb-mode --stats-interval 1 &
    PID=$!
    sleep 1
    ./target/release/test-sender -t 127.0.0.1:8001 -n 2 -c 10 --delay-ms 1
    sleep 2
    sudo kill $PID 2>/dev/null || true

# List loaded BPF programs
bpf-list:
    sudo bpftool prog list

# Show XDP programs attached to interfaces
bpf-net:
    sudo bpftool net show

# Dump COUNTERS map
bpf-counters:
    sudo bpftool map dump name COUNTERS

# Dump PROPOSER_STATS map
bpf-proposer-stats:
    sudo bpftool map dump name PROPOSER_STATS

# Dump CONFIG map
bpf-config:
    sudo bpftool map dump name CONFIG

# Watch packets on loopback port 8001
tcpdump-lo port="8001":
    sudo tcpdump -i lo -n udp port {{port}}

# Watch packets on eth0
tcpdump-eth0 port="8001":
    sudo tcpdump -i eth0 -n udp port {{port}}

# Build inside Lima VM (run from macOS)
lima-build:
    limactl shell xdp -- bash -c 'source ~/.cargo/env && cd {{justfile_directory()}} && cargo build --release -p pshred-loader'

# Run router inside Lima VM (run from macOS)
lima-run interface="eth0" port="8001":
    limactl shell xdp -- sudo bash -c 'cd {{justfile_directory()}} && ./target/release/pshred-loader -i {{interface}} -p {{port}} --skb-mode'

# Run test inside Lima VM (run from macOS)
lima-test:
    limactl shell xdp -- bash -c 'source ~/.cargo/env && cd {{justfile_directory()}} && just test'

# Check code without building
check:
    cargo check -p pshred-protocol

# Format code
fmt:
    cargo fmt --all

# Run clippy lints (protocol crate only, others need Linux)
lint:
    cargo clippy -p pshred-protocol

# Build Docker image
docker-build:
    docker compose build

# Run router in Docker
docker-run:
    docker compose up xdp-router

# Enter Docker dev shell
docker-shell:
    docker compose run --rm dev bash

# Send test packets via Docker
docker-test:
    docker compose up test-sender
