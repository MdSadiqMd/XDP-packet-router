use std::net::UdpSocket;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;

#[derive(Parser)]
#[command(name = "test-sender")]
#[command(about = "Send test pshred packets for router verification")]
struct Args {
    #[arg(short, long, default_value = "127.0.0.1:8001")]
    target: String,

    #[arg(short, long, default_value_t = 16)]
    num_proposers: u32,

    #[arg(short, long, default_value_t = 100)]
    count: u32,

    #[arg(long, default_value_t = 10)]
    delay_ms: u64,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let socket = UdpSocket::bind("0.0.0.0:0")?;

    println!(
        "Sending {} packets to {} ({} proposers)",
        args.count, args.target, args.num_proposers
    );

    for i in 0..args.count {
        let proposer_index = i % args.num_proposers;
        let packet = build_pshred_packet(i as u64, proposer_index, i);

        socket.send_to(&packet, &args.target)?;

        if args.delay_ms > 0 {
            std::thread::sleep(Duration::from_millis(args.delay_ms));
        }
    }

    println!("Done!");
    Ok(())
}

fn build_pshred_packet(slot: u64, proposer_index: u32, shred_index: u32) -> Vec<u8> {
    let mut packet = Vec::with_capacity(64);

    packet.extend_from_slice(&slot.to_le_bytes());
    packet.extend_from_slice(&proposer_index.to_le_bytes());
    packet.extend_from_slice(&shred_index.to_le_bytes());
    packet.extend_from_slice(&[0u8; 32]);
    packet.extend_from_slice(b"test_shred_data_payload");

    packet
}
