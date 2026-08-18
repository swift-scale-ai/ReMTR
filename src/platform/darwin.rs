use std::io;
use std::mem::{size_of, zeroed};
use std::net::Ipv4Addr;
use std::os::fd::RawFd;
use std::time::{Duration, Instant};

const ICMP_ECHO_REPLY: u8 = 0;
const ICMP_DESTINATION_UNREACHABLE: u8 = 3;
const ICMP_ECHO_REQUEST: u8 = 8;
const ICMP_TIME_EXCEEDED: u8 = 11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplyKind {
    EchoReply,
    TimeExceeded,
    DestinationUnreachable,
}

#[derive(Clone, Copy, Debug)]
pub struct ProbeReply {
    pub address: Ipv4Addr,
    pub elapsed: Duration,
    pub kind: ReplyKind,
}

pub struct IcmpSocket {
    descriptor: RawFd,
}

impl IcmpSocket {
    pub fn open() -> io::Result<Self> {
        // SAFETY: socket has no borrowed pointers. The returned descriptor is
        // owned by this type and closed exactly once in Drop.
        let descriptor = unsafe { libc::socket(libc::AF_INET, libc::SOCK_RAW, libc::IPPROTO_ICMP) };
        if descriptor < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { descriptor })
    }

    pub fn probe(
        &self,
        target: Ipv4Addr,
        ttl: u8,
        identifier: u16,
        sequence: u16,
        timeout: Duration,
    ) -> io::Result<Option<ProbeReply>> {
        self.set_ttl(ttl)?;
        let packet = echo_request(identifier, sequence);
        let destination = ipv4_sockaddr(target);
        let started = Instant::now();

        // SAFETY: destination and packet are initialized for the supplied
        // lengths and remain alive for the duration of sendto.
        let sent = unsafe {
            libc::sendto(
                self.descriptor,
                packet.as_ptr().cast(),
                packet.len(),
                0,
                (&destination as *const libc::sockaddr_in).cast(),
                size_of::<libc::sockaddr_in>() as libc::socklen_t,
            )
        };
        if sent < 0 {
            return Err(io::Error::last_os_error());
        }

        loop {
            let elapsed = started.elapsed();
            if elapsed >= timeout {
                return Ok(None);
            }
            let remaining = timeout - elapsed;
            if !self.wait_readable(remaining)? {
                return Ok(None);
            }
            if let Some((address, kind)) = self.receive_matching(identifier, sequence)? {
                return Ok(Some(ProbeReply {
                    address,
                    elapsed: started.elapsed(),
                    kind,
                }));
            }
        }
    }

    fn set_ttl(&self, ttl: u8) -> io::Result<()> {
        let value = libc::c_int::from(ttl);
        // SAFETY: value points to a valid c_int with the matching size.
        let result = unsafe {
            libc::setsockopt(
                self.descriptor,
                libc::IPPROTO_IP,
                libc::IP_TTL,
                (&value as *const libc::c_int).cast(),
                size_of::<libc::c_int>() as libc::socklen_t,
            )
        };
        if result < 0 {
            Err(io::Error::last_os_error())
        } else {
            Ok(())
        }
    }

    fn wait_readable(&self, timeout: Duration) -> io::Result<bool> {
        let mut descriptor = libc::pollfd {
            fd: self.descriptor,
            events: libc::POLLIN,
            revents: 0,
        };
        let milliseconds = timeout.as_millis().max(1).min(i32::MAX as u128) as i32;
        loop {
            // SAFETY: descriptor points to one initialized pollfd.
            let result = unsafe { libc::poll(&mut descriptor, 1, milliseconds) };
            if result >= 0 {
                return Ok(result > 0 && descriptor.revents & libc::POLLIN != 0);
            }
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::Interrupted {
                return Err(error);
            }
        }
    }

    fn receive_matching(
        &self,
        identifier: u16,
        sequence: u16,
    ) -> io::Result<Option<(Ipv4Addr, ReplyKind)>> {
        let mut buffer = [0_u8; 2048];
        // SAFETY: zeroed is valid for sockaddr_in and its length is initialized.
        let mut source: libc::sockaddr_in = unsafe { zeroed() };
        let mut source_length = size_of::<libc::sockaddr_in>() as libc::socklen_t;
        // SAFETY: all pointers refer to writable initialized storage of the
        // supplied sizes.
        let received = unsafe {
            libc::recvfrom(
                self.descriptor,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                0,
                (&mut source as *mut libc::sockaddr_in).cast(),
                &mut source_length,
            )
        };
        if received < 0 {
            return Err(io::Error::last_os_error());
        }
        let packet = &buffer[..received as usize];
        let kind = parse_reply(packet, identifier, sequence);
        Ok(kind.map(|kind| (Ipv4Addr::from(source.sin_addr.s_addr.to_ne_bytes()), kind)))
    }
}

impl Drop for IcmpSocket {
    fn drop(&mut self) {
        // SAFETY: descriptor is owned by this type and has not been closed.
        unsafe { libc::close(self.descriptor) };
    }
}

fn ipv4_sockaddr(address: Ipv4Addr) -> libc::sockaddr_in {
    libc::sockaddr_in {
        sin_len: size_of::<libc::sockaddr_in>() as u8,
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: 0,
        sin_addr: libc::in_addr {
            s_addr: u32::from_ne_bytes(address.octets()),
        },
        sin_zero: [0; 8],
    }
}

fn echo_request(identifier: u16, sequence: u16) -> [u8; 40] {
    let mut packet = [0_u8; 40];
    packet[0] = ICMP_ECHO_REQUEST;
    packet[4..6].copy_from_slice(&identifier.to_be_bytes());
    packet[6..8].copy_from_slice(&sequence.to_be_bytes());
    for (index, byte) in packet[8..].iter_mut().enumerate() {
        *byte = index as u8;
    }
    let checksum = internet_checksum(&packet);
    packet[2..4].copy_from_slice(&checksum.to_be_bytes());
    packet
}

fn internet_checksum(bytes: &[u8]) -> u16 {
    let mut sum = 0_u32;
    for chunk in bytes.chunks(2) {
        let word = if chunk.len() == 2 {
            u16::from_be_bytes([chunk[0], chunk[1]])
        } else {
            u16::from_be_bytes([chunk[0], 0])
        };
        sum = sum.wrapping_add(u32::from(word));
    }
    while sum >> 16 != 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

fn parse_reply(packet: &[u8], identifier: u16, sequence: u16) -> Option<ReplyKind> {
    let outer_header_length = ipv4_header_length(packet)?;
    let icmp = packet.get(outer_header_length..)?;
    match *icmp.first()? {
        ICMP_ECHO_REPLY if echo_identity(icmp) == Some((identifier, sequence)) => {
            Some(ReplyKind::EchoReply)
        }
        ICMP_TIME_EXCEEDED | ICMP_DESTINATION_UNREACHABLE => {
            let embedded_ip = icmp.get(8..)?;
            let embedded_header_length = ipv4_header_length(embedded_ip)?;
            let embedded_icmp = embedded_ip.get(embedded_header_length..)?;
            if echo_identity(embedded_icmp) != Some((identifier, sequence)) {
                return None;
            }
            if icmp[0] == ICMP_TIME_EXCEEDED {
                Some(ReplyKind::TimeExceeded)
            } else {
                Some(ReplyKind::DestinationUnreachable)
            }
        }
        _ => None,
    }
}

fn ipv4_header_length(packet: &[u8]) -> Option<usize> {
    let first = *packet.first()?;
    if first >> 4 != 4 {
        return None;
    }
    let length = usize::from(first & 0x0f) * 4;
    (length >= 20 && packet.len() >= length + 8).then_some(length)
}

fn echo_identity(icmp: &[u8]) -> Option<(u16, u16)> {
    Some((
        u16::from_be_bytes([*icmp.get(4)?, *icmp.get(5)?]),
        u16::from_be_bytes([*icmp.get(6)?, *icmp.get(7)?]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checksum_validates_echo_packet() {
        let packet = echo_request(42, 9);
        assert_eq!(internet_checksum(&packet), 0);
    }

    #[test]
    fn recognizes_echo_reply() {
        let mut packet = vec![0_u8; 28];
        packet[0] = 0x45;
        packet[20] = ICMP_ECHO_REPLY;
        packet[24..26].copy_from_slice(&42_u16.to_be_bytes());
        packet[26..28].copy_from_slice(&9_u16.to_be_bytes());
        assert_eq!(parse_reply(&packet, 42, 9), Some(ReplyKind::EchoReply));
    }

    #[test]
    fn recognizes_embedded_time_exceeded_probe() {
        let mut packet = vec![0_u8; 56];
        packet[0] = 0x45;
        packet[20] = ICMP_TIME_EXCEEDED;
        packet[28] = 0x45;
        packet[48] = ICMP_ECHO_REQUEST;
        packet[52..54].copy_from_slice(&7_u16.to_be_bytes());
        packet[54..56].copy_from_slice(&11_u16.to_be_bytes());
        assert_eq!(parse_reply(&packet, 7, 11), Some(ReplyKind::TimeExceeded));
    }
}
