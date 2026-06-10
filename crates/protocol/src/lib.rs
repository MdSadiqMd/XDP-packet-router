#![no_std]
#![doc = include_str!("../README.md")]

pub const MAX_PROPOSERS: u32 = 32;
pub const DEFAULT_UDP_PORT: u16 = 8001;

#[repr(C, packed)]
#[derive(Clone, Copy)]
pub struct PshredHeader {
    pub slot: u64,
    pub proposer_index: u32,
    pub shred_index: u32,
}

impl PshredHeader {
    pub const SIZE: usize = core::mem::size_of::<Self>();
    pub const PROPOSER_INDEX_OFFSET: usize = 8;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ProposerStats {
    pub packet_count: u64,
    pub byte_count: u64,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RouterConfig {
    pub target_port: u16,
    pub enabled: u8,
    pub _pad: u8,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            target_port: DEFAULT_UDP_PORT,
            enabled: 1,
            _pad: 0,
        }
    }
}

#[cfg(feature = "std")]
impl core::fmt::Debug for PshredHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let slot = self.slot;
        let proposer_index = self.proposer_index;
        let shred_index = self.shred_index;
        f.debug_struct("PshredHeader")
            .field("slot", &slot)
            .field("proposer_index", &proposer_index)
            .field("shred_index", &shred_index)
            .finish()
    }
}

#[cfg(feature = "std")]
impl core::fmt::Debug for ProposerStats {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ProposerStats")
            .field("packet_count", &self.packet_count)
            .field("byte_count", &self.byte_count)
            .finish()
    }
}
