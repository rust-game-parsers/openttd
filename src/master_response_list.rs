use crate::util::*;

use byteorder::{LittleEndian, WriteBytesExt};
use nom::{self, number::complete::*, *};
use std;
use std::collections::HashSet;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};

#[derive(Clone, Copy, Debug, PartialEq, Hash)]
enum ServerType {
    IPv4,
    IPv6,
}

impl ServerType {
    fn from_num(v: u8) -> Option<Self> {
        use self::ServerType::*;

        match v {
            1 => Some(IPv4),
            2 => Some(IPv6),
            _ => None,
        }
    }
}

pub type V4Set = HashSet<SocketAddrV4>;

impl ByteWriter for V4Set {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.write_u16::<LittleEndian>(self.len() as u16)?;
        for addr in self.iter() {
            for octet in &addr.ip().octets() {
                buf.write_u8(*octet)?;
            }
            buf.write_u16::<LittleEndian>(addr.port())?;
        }

        Ok(())
    }
}

pub type V6Set = HashSet<SocketAddrV6>;

impl ByteWriter for V6Set {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.write_u16::<LittleEndian>(self.len() as u16)?;
        for addr in self.iter() {
            for segment in &addr.ip().segments() {
                buf.write_u16::<LittleEndian>(*segment)?;
            }
            buf.write_u16::<LittleEndian>(addr.port())?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerList {
    IPv4(V4Set),
    IPv6(V6Set),
}

impl ByteWriter for ServerList {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        match *self {
            ServerList::IPv4(ref data) => data.write_pkt(buf),
            ServerList::IPv6(ref data) => data.write_pkt(buf),
        }
    }
}

named!(parse_v4_ip<&[u8], Ipv4Addr>,
    do_parse!(
        a: le_u8 >>
        b: le_u8 >>
        c: le_u8 >>
        d: le_u8 >>
        (Ipv4Addr::new(a, b, c, d))
    )
);

named!(parse_v6_ip<&[u8], Ipv6Addr>,
    do_parse!(
        a: le_u16 >>
        b: le_u16 >>
        c: le_u16 >>
        d: le_u16 >>
        e: le_u16 >>
        f: le_u16 >>
        g: le_u16 >>
        h: le_u16 >>
        (Ipv6Addr::new(a, b, c, d, e, f, g, h))
    )
);

named!(parse_master_response_v4_server_entry(&[u8]) -> SocketAddrV4,
    do_parse!(
        ip: parse_v4_ip >>
        port: le_u16 >>
        (SocketAddrV4::new(ip, port))
    )
);

named!(parse_master_response_v4(&[u8]) -> HashSet<SocketAddrV4>,
    do_parse!(
        server_count: le_u16 >>
        servers: count!(parse_master_response_v4_server_entry, server_count.into()) >>
        (servers.into_iter().collect::<HashSet<_>>())
    )
);

named!(parse_master_response_v6_server_entry(&[u8]) -> SocketAddrV6,
    do_parse!(
        ip: parse_v6_ip >>
        port: le_u16 >>
        (SocketAddrV6::new(ip, port, 0, 0))
    )
);

named!(parse_master_response_v6(&[u8]) -> HashSet<SocketAddrV6>,
    do_parse!(
        server_count: le_u16 >>
        servers: count!(parse_master_response_v6_server_entry, server_count.into()) >>
        (servers.into_iter().collect::<HashSet<_>>())
    )
);

named!(pub parse_master_response(&[u8]) -> ServerList,
    do_parse!(
        server_type: map_opt!(le_u8, ServerType::from_num) >>
        server_lists: switch!(value!(server_type),
            ServerType::IPv4 => map!(parse_master_response_v4, ServerList::IPv4) |
            ServerType::IPv6 => map!(parse_master_response_v6, ServerList::IPv6)
        ) >>
        (server_lists)
    )
);

#[cfg(test)]
mod tests {
    use super::*;

    use hex_literal::hex;

    use std::str::FromStr;

    fn fixtures() -> (Vec<u8>, ServerList) {
        let data = hex!(
            "
            010A004AD04BB78B0FACF9B0918B
            0F53C718168B0F3E8F2E448B0F79
            2AA0973E0F5CDE6E7C8B0F6C34E4
            4C8B0FB2EBB2578B0F80484A718B
            0F408AE7368B0F4200070101004A
            D04BB78C0F
        "
        )
        .to_vec();

        let srv_list = ServerList::IPv4(
            [
                "74.208.75.183:3979",
                "172.249.176.145:3979",
                "83.199.24.22:3979",
                "62.143.46.68:3979",
                "121.42.160.151:3902",
                "92.222.110.124:3979",
                "108.52.228.76:3979",
                "178.235.178.87:3979",
                "128.72.74.113:3979",
                "64.138.231.54:3979",
            ]
            .iter()
            .map(|s| SocketAddrV4::from_str(s).unwrap())
            .collect::<HashSet<SocketAddrV4>>(),
        );

        (data, srv_list)
    }

    #[test]
    fn test_parse_master_response() {
        let (input, expectation) = fixtures();

        let result = parse_master_response(&input).unwrap();

        assert_eq!(expectation, result.1);
    }
}
