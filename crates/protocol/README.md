# pshred-protocol

Shared protocol definitions for the Constellation pshred packet format.

This crate is `no_std` compatible and can be used in both eBPF and userspace contexts.

## pshred Header Format

Based on the MCP Protocol Specification (Section 7.2):

```text
| Field           | Offset | Size   |
|-----------------|--------|--------|
| slot            | 0      | 8      |
| proposer_index  | 8      | 4      | <- demux key
| shred_index     | 12     | 4      |
| commitment      | 16     | 32     |
| shred_data      | 48     | var    |
| ...             |        |        |
```

The `proposer_index` field at byte offset 8 is used for demultiplexing packets
to identify which of the ~16 proposers sent this pshred.
