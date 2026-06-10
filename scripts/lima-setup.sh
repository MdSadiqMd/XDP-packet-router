#!/bin/bash
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
VM_NAME="xdp"

case "${1:-}" in
  create)
    echo "Creating Lima VM for XDP development..."
    limactl create --name="$VM_NAME" "$PROJECT_DIR/lima.yaml"
    echo "Starting VM..."
    limactl start "$VM_NAME"
    echo ""
    echo "VM is ready! Enter with: $0 shell"
    ;;

  start)
    echo "Starting Lima VM..."
    limactl start "$VM_NAME"
    ;;

  stop)
    echo "Stopping Lima VM..."
    limactl stop "$VM_NAME"
    ;;

  shell)
    echo "Entering Lima VM..."
    limactl shell "$VM_NAME"
    ;;

  delete)
    echo "Deleting Lima VM..."
    limactl delete "$VM_NAME" --force
    ;;

  status)
    limactl list
    ;;

  build)
    echo "Building inside Lima VM..."
    limactl shell "$VM_NAME" -- bash -c "cd '$PROJECT_DIR' && cargo build --release -p pshred-loader"
    ;;

  run)
    shift
    IFACE="${1:-lima0}"
    PORT="${2:-8001}"
    echo "Running XDP router on interface $IFACE, port $PORT..."
    limactl shell "$VM_NAME" -- sudo bash -c "cd '$PROJECT_DIR' && RUST_LOG=info ./target/release/pshred-loader -i $IFACE -p $PORT"
    ;;

  test)
    echo "Sending test packets..."
    limactl shell "$VM_NAME" -- bash -c "cd '$PROJECT_DIR' && cargo run --release --bin test-sender -- -t 127.0.0.1:8001 -n 16 -c 100"
    ;;

  *)
    echo "Lima VM helper for XDP development"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  create  - Create and start the Lima VM"
    echo "  start   - Start the Lima VM"
    echo "  stop    - Stop the Lima VM"
    echo "  shell   - Enter the Lima VM shell"
    echo "  delete  - Delete the Lima VM"
    echo "  status  - Show VM status"
    echo "  build   - Build the project inside Lima"
    echo "  run [iface] [port] - Run XDP router (default: lima0:8001)"
    echo "  test    - Send test packets"
    ;;
esac
