#![no_std]
#![no_main]
#![allow(static_mut_refs)]
use core::panic::PanicInfo;
use aya_ebpf::{
    macros::{map, xdp},
    maps::HashMap,
    programs::XdpContext,
};
use aya_ebpf::bindings::xdp_action;

#[map(name = "BLOCKLIST")]
static mut BLOCKLIST: HashMap<u32, u8> = HashMap::<u32, u8>::with_max_entries(2048, 0);

#[xdp]
pub fn block_ips(ctx: XdpContext) -> u32 {
    match try_block_ips(ctx) {
        Ok(ret) => ret,
        Err(_) => xdp_action::XDP_PASS as u32,
    }
}

fn try_block_ips(ctx: XdpContext) -> Result<u32, ()> {
    let data = ctx.data() as usize;
    let data_end = ctx.data_end() as usize;
    let eth_hdr_len = 14usize;
    if data + eth_hdr_len + 20 > data_end { return Ok(xdp_action::XDP_PASS as u32); }

    let eth_type_ptr = (data + 12) as *const u16;
    let eth_type = unsafe { core::ptr::read_unaligned(eth_type_ptr) };
    if u16::from_be(eth_type) != 0x0800 { return Ok(xdp_action::XDP_PASS as u32); }

    let ip_src_ptr = (data + eth_hdr_len + 12) as *const u32;
    let saddr_be = unsafe { core::ptr::read_unaligned(ip_src_ptr) };
    let saddr = u32::from_be(saddr_be);

    unsafe {
        if let Some(_) = BLOCKLIST.get(&saddr) {
            return Ok(xdp_action::XDP_DROP as u32);
        }
    }

    Ok(xdp_action::XDP_PASS as u32)
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}