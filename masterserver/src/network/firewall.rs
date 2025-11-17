use anyhow::{Context, Result};
use aya::{Ebpf, maps::HashMap, programs::Xdp};
use std::{convert::TryInto, net::Ipv4Addr};

pub struct Firewall {
    ebpf: Ebpf,
}

impl Firewall {
    /// Initialize firewall
    pub fn new(iface: &str) -> Result<Self> {
        let mut ebpf = Ebpf::load_file("./src/network/ebpf-filter")
            .with_context(|| format!("Error loading EBPF Program"))?;

        let prog: &mut Xdp = ebpf
            .program_mut("block_ips")
            .ok_or_else(|| anyhow::anyhow!("Program 'block_ips' not found"))?
            .try_into()?;

        prog.load()?;
        prog.attach(iface, aya::programs::XdpFlags::default())?;

        Ok(Self { ebpf })
    }

    /// Converts an IPv4 address to a u32 key
    fn ip_to_key(ip: Ipv4Addr) -> u32 {
        u32::from(ip)
    }

    /// Blocks an IP
    pub fn block_ip(&mut self, ip: Ipv4Addr) -> Result<()> {
        let key = Self::ip_to_key(ip);
        let map = self
            .ebpf
            .map_mut("BLOCKLIST")
            .ok_or_else(|| anyhow::anyhow!("'BLOCKLIST' map not found"))?;

        let mut blocklist: HashMap<_, u32, u8> = HashMap::try_from(map)?;
        blocklist.insert(key, 1, 0)?;
        Ok(())
    }

    /// Unblocks an IP
    pub fn unblock_ip(&mut self, ip: Ipv4Addr) -> Result<()> {
        let key = Self::ip_to_key(ip);
        let map = self
            .ebpf
            .map_mut("BLOCKLIST")
            .ok_or_else(|| anyhow::anyhow!("'BLOCKLIST' map not found"))?;

        let mut blocklist: HashMap<_, u32, u8> = HashMap::try_from(map)?;
        match blocklist.remove(&key) {
            Ok(_) => Ok(()),
            Err(error) => Err(anyhow::anyhow!("Failed to unblock ip: {}", error)),
        }
    }

    /// List blocked IPs
    pub fn list_blocked(&self) -> Result<Vec<Ipv4Addr>> {
        let map = self
            .ebpf
            .map("BLOCKLIST")
            .ok_or_else(|| anyhow::anyhow!("'BLOCKLIST' map not found"))?;

        let blocklist: aya::maps::HashMap<_, u32, u8> = aya::maps::HashMap::try_from(map)?;

        let mut ips = Vec::new();

        let mut keys = blocklist.keys();

        while let Some(res) = keys.next() {
            match res {
                Ok(key) => ips.push(Ipv4Addr::from(key)),
                Err(e) => return Err(anyhow::anyhow!("eBPF map read error: {}", e)),
            }
        }

        Ok(ips)
    }
}
