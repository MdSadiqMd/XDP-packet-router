#![no_std]
#![no_main]

use core::mem;

use aya_ebpf::{
    bindings::xdp_action,
    macros::{map, xdp},
    maps::{Array, PerCpuArray},
    programs::XdpContext,
};
use network_types::{eth::EthHdr, ip::Ipv4Hdr, udp::UdpHdr};
use pshred_protocol::{ProposerStats, RouterConfig, MAX_PROPOSERS};

const ETH_P_IP: u16 = 0x0800;
const IPPROTO_UDP: u8 = 17;

#[map]
static CONFIG: Array<RouterConfig> = Array::with_max_entries(1, 0);

#[map]
static PROPOSER_STATS: PerCpuArray<ProposerStats> = PerCpuArray::with_max_entries(MAX_PROPOSERS, 0);

#[map]
static COUNTERS: PerCpuArray<u64> = PerCpuArray::with_max_entries(4, 0);

const IDX_TOTAL: u32 = 0;
const IDX_UDP_MATCHED: u32 = 1;
const IDX_PSHRED_PARSED: u32 = 2;
const IDX_ERRORS: u32 = 3;

#[xdp]
pub fn pshred_router(ctx: XdpContext) -> u32 {
    match process_packet(&ctx) {
        Ok(action) => action,
        Err(_) => {
            increment_counter(IDX_ERRORS);
            xdp_action::XDP_PASS
        }
    }
}

#[inline(always)]
fn process_packet(ctx: &XdpContext) -> Result<u32, ()> {
    increment_counter(IDX_TOTAL);

    let eth = ptr_at::<EthHdr>(ctx, 0)?;
    let ether_type = unsafe { u16::from_be((*eth).ether_type) };
    
    if ether_type != ETH_P_IP {
        return Ok(xdp_action::XDP_PASS);
    }

    let ip = ptr_at::<Ipv4Hdr>(ctx, EthHdr::LEN)?;
    let proto = unsafe { (*ip).proto };

    if proto != IPPROTO_UDP {
        return Ok(xdp_action::XDP_PASS);
    }

    let udp_offset = EthHdr::LEN + Ipv4Hdr::LEN;
    let udp = ptr_at::<UdpHdr>(ctx, udp_offset)?;
    let dest_port = unsafe { (*udp).dst_port() };

    let config = match CONFIG.get(0) {
        Some(c) => c,
        None => return Ok(xdp_action::XDP_PASS),
    };

    if config.enabled == 0 || dest_port != config.target_port {
        return Ok(xdp_action::XDP_PASS);
    }

    increment_counter(IDX_UDP_MATCHED);

    let payload_offset = udp_offset + mem::size_of::<UdpHdr>();
    let proposer_index = read_proposer_index(ctx, payload_offset)?;

    increment_counter(IDX_PSHRED_PARSED);

    let packet_len = (ctx.data_end() - ctx.data()) as u64;
    update_proposer_stats(proposer_index, packet_len);

    Ok(xdp_action::XDP_PASS)
}

#[inline(always)]
fn read_proposer_index(ctx: &XdpContext, payload_offset: usize) -> Result<u32, ()> {
    let proposer_offset = payload_offset + 8;
    let proposer_ptr = ptr_at::<u32>(ctx, proposer_offset)?;
    let proposer_index = unsafe { core::ptr::read_unaligned(proposer_ptr) };
    Ok(u32::from_le(proposer_index))
}

#[inline(always)]
fn ptr_at<T>(ctx: &XdpContext, offset: usize) -> Result<*const T, ()> {
    let start = ctx.data();
    let end = ctx.data_end();
    let len = mem::size_of::<T>();

    if start + offset + len > end {
        return Err(());
    }

    Ok((start + offset) as *const T)
}

#[inline(always)]
fn increment_counter(index: u32) {
    if let Some(counter) = COUNTERS.get_ptr_mut(index) {
        unsafe { *counter += 1 };
    }
}

#[inline(always)]
fn update_proposer_stats(proposer_index: u32, packet_len: u64) {
    let idx = proposer_index % MAX_PROPOSERS;
    if let Some(stats) = PROPOSER_STATS.get_ptr_mut(idx) {
        unsafe {
            (*stats).packet_count += 1;
            (*stats).byte_count += packet_len;
        }
    }
}

// #[cfg(not(test))]
// #[panic_handler]
// fn panic(_info: &core::panic::PanicInfo) -> ! {
//     loop {}
// }

#[unsafe(link_section = "license")]
#[unsafe(no_mangle)]
static LICENSE: [u8; 13] = *b"Dual MIT/GPL\0";
