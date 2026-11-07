use std::net::IpAddr;

pub(crate) type Addresses = Vec<IpAddr>;

#[derive(Default)]
pub(crate) struct Online {
    settled: Option<Addresses>,
    latest: Option<Addresses>,
}

impl Online {
    pub(crate) fn came_back(&mut self, now: Addresses) -> bool {
        let unchanged = self.latest.as_ref().is_some_and(|latest| same(latest, &now));
        if !unchanged {
            self.latest = Some(now);
            return false;
        }
        let gained = self
            .settled
            .as_ref()
            .is_some_and(|settled| !includes(settled, &now));
        self.settled = Some(now);
        gained
    }

    pub(crate) fn woke(&mut self) {
        self.settled = Some(Addresses::new());
    }
}

fn includes(all: &[IpAddr], some: &[IpAddr]) -> bool {
    some.iter().all(|address| all.contains(address))
}

fn same(one: &[IpAddr], other: &[IpAddr]) -> bool {
    includes(one, other) && includes(other, one)
}

pub(crate) fn addresses() -> Addresses {
    interfaces::running_addresses()
        .into_iter()
        .filter(reaches_beyond_this_machine)
        .collect()
}

fn reaches_beyond_this_machine(address: &IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => {
            !(address.is_loopback() || address.is_link_local() || address.is_unspecified())
        }
        IpAddr::V6(address) => {
            !(address.is_loopback() || address.is_unicast_link_local() || address.is_unspecified())
        }
    }
}

#[cfg(windows)]
mod interfaces {
    use std::net::IpAddr;

    use windows::Win32::{
        Foundation::{ERROR_BUFFER_OVERFLOW, NO_ERROR},
        NetworkManagement::{
            IpHelper::{
                GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_DNS_SERVER, GAA_FLAG_SKIP_FRIENDLY_NAME,
                GAA_FLAG_SKIP_MULTICAST, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
            },
            Ndis::IfOperStatusUp,
        },
        Networking::WinSock::{
            AF_INET, AF_INET6, AF_UNSPEC, IpDadStatePreferred, SOCKADDR, SOCKADDR_IN, SOCKADDR_IN6,
        },
    };

    const FIRST_GUESS_BYTES: u32 = 16 * 1024;
    const TRIES: usize = 3;

    pub(super) fn running_addresses() -> Vec<IpAddr> {
        let flags = GAA_FLAG_SKIP_ANYCAST
            | GAA_FLAG_SKIP_MULTICAST
            | GAA_FLAG_SKIP_DNS_SERVER
            | GAA_FLAG_SKIP_FRIENDLY_NAME;
        let mut size = FIRST_GUESS_BYTES;
        for _ in 0..TRIES {
            let mut buffer = vec![0u64; (size as usize).div_ceil(size_of::<u64>())];
            let first = buffer.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
            let status = unsafe {
                GetAdaptersAddresses(u32::from(AF_UNSPEC.0), flags, None, Some(first), &mut size)
            };
            if status == ERROR_BUFFER_OVERFLOW.0 {
                continue;
            }
            if status != NO_ERROR.0 {
                return Vec::new();
            }
            return unsafe { addresses_from(first.cast_const()) };
        }
        Vec::new()
    }

    unsafe fn addresses_from(mut adapter: *const IP_ADAPTER_ADDRESSES_LH) -> Vec<IpAddr> {
        let mut found = Vec::new();
        while let Some(current) = unsafe { adapter.as_ref() } {
            adapter = current.Next.cast_const();
            if current.OperStatus != IfOperStatusUp {
                continue;
            }
            let mut unicast = current.FirstUnicastAddress.cast_const();
            while let Some(address) = unsafe { unicast.as_ref() } {
                unicast = address.Next.cast_const();
                if address.DadState == IpDadStatePreferred
                    && let Some(ip) = unsafe { ip_of(address.Address.lpSockaddr.cast_const()) }
                {
                    found.push(ip);
                }
            }
        }
        found
    }

    unsafe fn ip_of(address: *const SOCKADDR) -> Option<IpAddr> {
        let family = unsafe { address.as_ref() }?.sa_family;
        if family == AF_INET {
            let ipv4 = unsafe { address.cast::<SOCKADDR_IN>().read_unaligned() };
            Some(IpAddr::from(unsafe { ipv4.sin_addr.S_un.S_addr }.to_ne_bytes()))
        } else if family == AF_INET6 {
            let ipv6 = unsafe { address.cast::<SOCKADDR_IN6>().read_unaligned() };
            Some(IpAddr::from(unsafe { ipv6.sin6_addr.u.Byte }))
        } else {
            None
        }
    }
}

#[cfg(not(windows))]
mod interfaces {
    use std::{net::IpAddr, ptr};

    pub(super) fn running_addresses() -> Vec<IpAddr> {
        let mut first = ptr::null_mut();
        if unsafe { libc::getifaddrs(&mut first) } != 0 {
            return Vec::new();
        }
        let running = (libc::IFF_UP | libc::IFF_RUNNING) as libc::c_uint;
        let mut found = Vec::new();
        let mut entry = first.cast_const();
        while let Some(current) = unsafe { entry.as_ref() } {
            entry = current.ifa_next.cast_const();
            if current.ifa_flags & running == running
                && let Some(ip) = unsafe { ip_of(current.ifa_addr.cast_const()) }
            {
                found.push(ip);
            }
        }
        unsafe { libc::freeifaddrs(first) };
        found
    }

    unsafe fn ip_of(address: *const libc::sockaddr) -> Option<IpAddr> {
        match i32::from(unsafe { address.as_ref() }?.sa_family) {
            libc::AF_INET => {
                let ipv4 = unsafe { address.cast::<libc::sockaddr_in>().read_unaligned() };
                Some(IpAddr::from(ipv4.sin_addr.s_addr.to_ne_bytes()))
            }
            libc::AF_INET6 => {
                let ipv6 = unsafe { address.cast::<libc::sockaddr_in6>().read_unaligned() };
                Some(IpAddr::from(ipv6.sin6_addr.s6_addr))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::IpAddr;

    use super::{Addresses, Online, addresses, reaches_beyond_this_machine};

    fn on(texts: &[&str]) -> Addresses {
        texts.iter().map(|text| text.parse().unwrap()).collect()
    }

    fn settled_on(texts: &[&str]) -> Online {
        let mut online = Online::default();
        assert!(!online.came_back(on(texts)));
        assert!(!online.came_back(on(texts)));
        online
    }

    #[test]
    fn the_addresses_found_first_are_where_it_starts_not_a_return() {
        let mut online = settled_on(&["192.168.1.20"]);
        assert!(!online.came_back(on(&["192.168.1.20"])));
    }

    #[test]
    fn an_address_that_appears_and_stays_one_more_check_is_a_return() {
        let mut online = settled_on(&[]);
        assert!(!online.came_back(on(&["192.168.1.20"])), "still settling");
        assert!(online.came_back(on(&["192.168.1.20"])));
        assert!(!online.came_back(on(&["192.168.1.20"])), "only once");
    }

    #[test]
    fn dropping_off_the_network_is_no_return_but_coming_back_is() {
        let mut online = settled_on(&["192.168.1.20", "10.0.0.5"]);
        assert!(!online.came_back(on(&["10.0.0.5"])));
        assert!(!online.came_back(on(&["10.0.0.5"])), "only lost one");
        assert!(!online.came_back(on(&["10.0.0.5", "192.168.1.20"])));
        assert!(online.came_back(on(&["10.0.0.5", "192.168.1.20"])));
    }

    #[test]
    fn addresses_that_keep_changing_wait_until_they_settle() {
        let mut online = settled_on(&[]);
        assert!(!online.came_back(on(&["192.168.1.20"])));
        assert!(!online.came_back(on(&["192.168.1.20", "2001:db8::20"])));
        assert!(online.came_back(on(&["192.168.1.20", "2001:db8::20"])));
    }

    #[test]
    fn a_vpn_that_adds_an_address_beside_the_others_is_a_return() {
        let mut online = settled_on(&["192.168.1.20"]);
        assert!(!online.came_back(on(&["192.168.1.20", "10.8.0.2"])));
        assert!(online.came_back(on(&["192.168.1.20", "10.8.0.2"])));
    }

    #[test]
    fn after_sleep_the_addresses_it_settles_on_are_a_return_even_if_unchanged() {
        let mut online = settled_on(&["192.168.1.20"]);
        online.woke();
        assert!(online.came_back(on(&["192.168.1.20"])));
        let mut offline_on_waking = settled_on(&["192.168.1.20"]);
        offline_on_waking.woke();
        assert!(!offline_on_waking.came_back(on(&[])));
        assert!(!offline_on_waking.came_back(on(&[])));
        assert!(!offline_on_waking.came_back(on(&["192.168.1.20"])));
        assert!(offline_on_waking.came_back(on(&["192.168.1.20"])));
    }

    #[test]
    fn loopback_and_link_local_addresses_reach_nothing_beyond_this_machine() {
        for kept in ["192.168.1.20", "10.8.0.2", "2001:db8::20", "fd00::1"] {
            assert!(reaches_beyond_this_machine(&kept.parse::<IpAddr>().unwrap()), "{kept}");
        }
        for dropped in ["127.0.0.1", "169.254.10.1", "0.0.0.0", "::1", "fe80::1", "::"] {
            assert!(!reaches_beyond_this_machine(&dropped.parse::<IpAddr>().unwrap()), "{dropped}");
        }
    }

    #[test]
    fn the_system_lists_running_interfaces_without_loopback() {
        assert!(!addresses().contains(&"127.0.0.1".parse::<IpAddr>().unwrap()));
    }
}
