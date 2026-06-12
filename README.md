# XDP Packet Router for Constellation pshred Demultiplexing

A high-performance XDP (eXpress Data Path) packet router that demultiplexes UDP packets based on the `proposer_index` field in the Constellation pshred header.

## Overview

This project implements an eBPF/XDP program that:

1. Filters incoming UDP packets by a configurable destination port
2. Parses the pshred header from the UDP payload
3. Extracts the `proposer_index` field (bytes 8-11, little-endian)
4. Tracks per-proposer packet and byte statistics
5. Routes packets to different interfaces based on proposer_index (`XDP_REDIRECT`)
6. Falls back to kernel stack if no redirect target is configured (`XDP_PASS`)

## pshred Header Format

Based on the MCP Protocol Specification (Section 7.2):

```
Offset  Size  Field
------  ----  -----
0       8     slot
8       4     proposer_index  <- demux key
12      4     shred_index
16      32    commitment
...
```

## Project Structure

```
xdp-packet-router/
├── crates/
│   ├── protocol/     # Shared types (no_std compatible)
│   ├── ebpf/         # XDP eBPF program
│   └── loader/       # Userspace loader and stats
├── scripts/          # Lima VM helper scripts
├── tools/            # Build and deployment utilities
└── docs/             # Documentation
```

## Quick Start (Lima VM)

Lima provides a lightweight Linux VM on macOS - perfect for XDP development.

```bash
# Install Lima (one-time)
brew install lima

# Create and start the XDP development VM
./scripts/lima-setup.sh create

# Enter the VM shell
./scripts/lima-setup.sh shell

# Inside the VM, build and run:
cargo build --release -p pshred-loader
sudo ./target/release/pshred-loader -i lo -p 8001 --skb-mode
```

## Usage

### Basic (Stats Only)

```bash
# Attach to loopback, filter port 8001
sudo ./target/release/pshred-loader -i lo -p 8001 --skb-mode

# Attach to eth0
sudo ./target/release/pshred-loader -i eth0 -p 8001
```

### With Packet Routing

```bash
# Route proposer 0 to veth0, proposer 1 to veth1
sudo ./target/release/pshred-loader -i eth0 -p 8001 \
    --redirect 0:veth0 --redirect 1:veth1

# Auto-create veth0..veth15 mappings for 16 proposers
sudo ./target/release/pshred-loader -i eth0 -p 8001 --auto-redirect 16
```

### Command Line Options

| Option | Default | Description |
|--------|---------|-------------|
| `-i, --interface` | eth0 | Network interface to attach XDP |
| `-p, --port` | 8001 | UDP port to filter |
| `--stats-interval` | 2 | Statistics print interval (seconds) |
| `--skb-mode` | false | Use SKB mode (slower but compatible) |
| `--redirect ID:IFACE` | none | Route proposer to interface (repeatable) |
| `--auto-redirect N` | none | Auto-map proposers 0..N-1 to veth0..vethN-1 |

## Output

```
--- Router Statistics ---
Total: 100  UDP matched: 50  Parsed: 50  Redirected: 50  Passed: 0  Errors: 0

Per-Proposer Stats:
  ID      Packets          Bytes
--------------------------------
   0           13           1469
   1           13           1469
   2           12           1356
   3           12           1356
```

| Counter | Description |
|---------|-------------|
| Total | All packets seen by XDP |
| UDP matched | Packets matching target port |
| Parsed | Successfully parsed pshred packets |
| Redirected | Packets routed via XDP_REDIRECT |
| Passed | Packets passed to kernel (no redirect) |
| Errors | Parse failures |

## Architecture

```
                      Kernel Space                    User Space
                     ┌─────────────┐                 ┌──────────┐
UDP Packet ────────► │  XDP prog   │                 │  Loader  │
                     │             │                 │          │
                     │ 1. Parse    │    BPF Maps     │ Config   │
                     │    headers  │◄───────────────►│ redirects│
                     │ 2. Extract  │   (CONFIG,      │ & read   │
                     │    proposer │    STATS,       │ stats    │
                     │ 3. Update   │    REDIRECT_MAP)│          │
                     │    stats    │                 │          │
                     │ 4. Redirect │                 │          │
                     └──────┬──────┘                 └──────────┘
                            │
              ┌─────────────┼─────────────┐
              ▼             ▼             ▼
           veth0         veth1   ...   vethN
        (proposer 0)  (proposer 1)  (proposer N)
```

## Testing

```bash
# Build test sender
cargo build --release --bin test-sender

# Send 100 packets with 4 proposers
./target/release/test-sender -t 127.0.0.1:8001 -n 4 -c 100
```

## Just Commands

```bash
just build          # Build XDP loader
just run-lo         # Run on loopback
just test           # Full test with packets
just bpf-list       # List loaded BPF programs
just vm-shell       # Enter Lima VM
```

## Documentation

- [Usage Guide](docs/usage.md) - Detailed usage and validation
- [Design Decisions](docs/decisions.md) - Architecture choices

## License

MIT OR Apache-2.0
