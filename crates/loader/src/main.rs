use std::time::Duration;

use anyhow::{Context, Result};
use aya::{
    Ebpf, Pod,
    maps::{Array, PerCpuArray, PerCpuValues},
    programs::{Xdp, xdp::XdpMode},
};
use clap::Parser;
use log::{info, warn};
use pshred_protocol::{DEFAULT_UDP_PORT, MAX_PROPOSERS};
use tokio::{signal, time};

#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProposerStats {
    packet_count: u64,
    byte_count: u64,
}

unsafe impl Pod for ProposerStats {}

#[repr(C)]
#[derive(Clone, Copy)]
struct RouterConfig {
    target_port: u16,
    enabled: u8,
    _pad: u8,
}

#[derive(Debug, Parser)]
#[command(name = "pshred-loader")]
#[command(about = "XDP packet router for Constellation pshred demultiplexing")]
struct Args {
    #[arg(short, long, default_value = "eth0")]
    interface: String,

    #[arg(short, long, default_value_t = DEFAULT_UDP_PORT)]
    port: u16,

    #[arg(long, default_value_t = 2)]
    stats_interval: u64,

    #[arg(long)]
    skb_mode: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let args = Args::parse();

    bump_memlock_rlimit()?;

    let mut ebpf = load_ebpf_program()?;
    // Skip logger for now - setup_logger(&mut ebpf);

    // Load and attach the XDP program first to create maps
    attach_xdp(&mut ebpf, &args.interface, args.skb_mode)?;

    // Then configure the router
    configure_router(&mut ebpf, args.port)?;

    info!(
        "pshred router attached to {} (port={}, skb_mode={})",
        args.interface, args.port, args.skb_mode
    );

    run_stats_loop(&mut ebpf, Duration::from_secs(args.stats_interval)).await?;

    Ok(())
}

fn load_ebpf_program() -> Result<Ebpf> {
    #[cfg(target_os = "linux")]
    let ebpf_bytes = aya::include_bytes_aligned!(concat!(env!("OUT_DIR"), "/pshred-router"));
    
    #[cfg(not(target_os = "linux"))]
    let ebpf_bytes: &[u8] = &[];
    
    Ebpf::load(ebpf_bytes).context("failed to load eBPF program")
}

fn setup_logger(ebpf: &mut Ebpf) {
    if let Err(e) = aya_log::EbpfLogger::init(ebpf) {
        warn!("failed to initialize eBPF logger: {}", e);
    }
}

unsafe impl Pod for RouterConfig {}

fn configure_router(ebpf: &mut Ebpf, port: u16) -> Result<()> {
    let mut config: Array<_, RouterConfig> =
        Array::try_from(ebpf.map_mut("CONFIG").context("CONFIG map not found")?)?;

    let router_config = RouterConfig {
        target_port: port,
        enabled: 1,
        _pad: 0,
    };

    config.set(0, router_config, 0)?;
    info!("configured router: port={}", port);
    Ok(())
}

fn attach_xdp(ebpf: &mut Ebpf, interface: &str, skb_mode: bool) -> Result<()> {
    let program: &mut Xdp = ebpf
        .program_mut("pshred_router")
        .context("pshred_router program not found")?
        .try_into()?;

    program.load()?;

    let mode = if skb_mode {
        XdpMode::Skb
    } else {
        XdpMode::Default
    };

    program
        .attach(interface, mode)
        .with_context(|| format!("failed to attach XDP to {}", interface))?;

    Ok(())
}

async fn run_stats_loop(ebpf: &mut Ebpf, interval: Duration) -> Result<()> {
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    let mut ticker = time::interval(interval);

    loop {
        tokio::select! {
            _ = &mut ctrl_c => {
                info!("shutting down...");
                break;
            }
            _ = ticker.tick() => {
                if let Err(e) = print_stats(ebpf) {
                    warn!("failed to read stats: {}", e);
                }
            }
        }
    }

    Ok(())
}

fn print_stats(ebpf: &mut Ebpf) -> Result<()> {
    let counters: PerCpuArray<_, u64> =
        PerCpuArray::try_from(ebpf.map_mut("COUNTERS").context("COUNTERS not found")?)?;

    let total = sum_percpu(&counters.get(&0, 0)?);
    let udp_matched = sum_percpu(&counters.get(&1, 0)?);
    let pshred_parsed = sum_percpu(&counters.get(&2, 0)?);
    let errors = sum_percpu(&counters.get(&3, 0)?);

    println!("\n--- Router Statistics ---");
    println!(
        "Total: {}  UDP matched: {}  Parsed: {}  Errors: {}",
        total, udp_matched, pshred_parsed, errors
    );

    let proposer_stats: PerCpuArray<_, ProposerStats> = PerCpuArray::try_from(
        ebpf.map_mut("PROPOSER_STATS")
            .context("PROPOSER_STATS not found")?,
    )?;

    println!("\nPer-Proposer Stats:");
    println!("{:>4} {:>12} {:>14}", "ID", "Packets", "Bytes");
    println!("{}", "-".repeat(32));

    for i in 0..MAX_PROPOSERS {
        if let Ok(values) = proposer_stats.get(&i, 0) {
            let (packets, bytes) = sum_proposer_stats(&values);
            if packets > 0 {
                println!("{:>4} {:>12} {:>14}", i, packets, bytes);
            }
        }
    }

    Ok(())
}

fn sum_percpu(values: &PerCpuValues<u64>) -> u64 {
    values.iter().sum()
}

fn sum_proposer_stats(values: &PerCpuValues<ProposerStats>) -> (u64, u64) {
    values
        .iter()
        .fold((0, 0), |(p, b), s| (p + s.packet_count, b + s.byte_count))
}

fn bump_memlock_rlimit() -> Result<()> {
    let rlim = libc::rlimit {
        rlim_cur: libc::RLIM_INFINITY,
        rlim_max: libc::RLIM_INFINITY,
    };

    let ret = unsafe { libc::setrlimit(libc::RLIMIT_MEMLOCK, &rlim) };
    if ret != 0 {
        warn!("failed to increase memlock rlimit (may need root)");
    }
    Ok(())
}
