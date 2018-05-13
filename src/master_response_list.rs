use util::*;

use nom::*;
use std::collections::HashSet;
use std;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddrV4, SocketAddrV6};
use byteorder::{LittleEndian, WriteBytesExt};

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
            for octet in addr.ip().octets().into_iter() {
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
            for segment in addr.ip().segments().into_iter() {
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

named!(parse_master_response_v4<&[u8], HashSet<SocketAddrV4>>,
    do_parse!(
        server_count: le_u16 >>
        servers: count!(
            do_parse!(
                ip: parse_v4_ip >>
                port: le_u16 >>
                (SocketAddrV4::new(ip, port))
            ),
            server_count as usize
        ) >>
        (servers.into_iter().collect::<HashSet<_>>())
    )
);

named!(parse_master_response_v6<&[u8], HashSet<SocketAddrV6>>,
    do_parse!(
        server_count: le_u16 >>
        servers: count!(
            do_parse!(
                ip: parse_v6_ip >>
                port: le_u16 >>
                (SocketAddrV6::new(ip, port, 0, 0))
            ),
            server_count as usize
        ) >>
        (servers.into_iter().collect::<HashSet<_>>())
    )
);

named!(pub parse_master_response<&[u8], ServerList>,
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

    use std::str::FromStr;

    fn fixtures() -> (Vec<u8>, ServerList) {
        let data = vec![
            0x01, 0x0A, 0x00, 0x4A, 0xD0, 0x4B, 0xB7, 0x8B, 0x0F, 0xAC, 0xF9, 0xB0, 0x91, 0x8B, 0x0F, 0x53, 0xC7, 0x18,
            0x16, 0x8B, 0x0F, 0x3E, 0x8F, 0x2E, 0x44, 0x8B, 0x0F, 0x79, 0x2A, 0xA0, 0x97, 0x3E, 0x0F, 0x5C, 0xDE, 0x6E,
            0x7C, 0x8B, 0x0F, 0x6C, 0x34, 0xE4, 0x4C, 0x8B, 0x0F, 0xB2, 0xEB, 0xB2, 0x57, 0x8B, 0x0F, 0x80, 0x48, 0x4A,
            0x71, 0x8B, 0x0F, 0x40, 0x8A, 0xE7, 0x36, 0x8B, 0x0F, 0x42, 0x00, 0x07, 0x01, 0x01, 0x00, 0x4A, 0xD0, 0x4B,
            0xB7, 0x8C, 0x0F,
        ];
        let srv_list = ServerList::IPv4(
            vec![
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
            ].into_iter()
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
