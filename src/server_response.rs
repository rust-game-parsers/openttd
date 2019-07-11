use crate::util::*;

use byteorder::{LittleEndian, WriteBytesExt};
use chrono::prelude::*;
use nom::{self, number::complete::*, *};
use std;
use std::collections::{BTreeMap, HashMap};
use std::ffi::CString;
use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub struct V2Data {
    pub max_companies: u8,
    pub current_companies: u8,
    pub max_spectators: u8,
}

impl ByteWriter for V2Data {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.push(self.max_companies);
        buf.push(self.current_companies);
        buf.push(self.max_spectators);

        Ok(())
    }
}

named!(parse_v2_data<&[u8], V2Data>,
    do_parse!(
        max_companies: be_u8 >>
        current_companies: be_u8 >>
        max_spectators: be_u8 >>
        (V2Data { max_companies, current_companies, max_spectators })
    )
);

#[derive(Clone, Debug, PartialEq)]
pub struct V3Data {
    pub game_date: DateTime<Utc>,
    pub start_date: DateTime<Utc>,
}

impl ByteWriter for V3Data {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.write_u32::<LittleEndian>(self.game_date.timestamp() as u32)?;
        buf.write_u32::<LittleEndian>(self.start_date.timestamp() as u32)?;

        Ok(())
    }
}

named!(datetime<&[u8], DateTime<Utc>>,
    map!(le_u32, datetime_from_ts)
);

named!(parse_v3_data<&[u8], V3Data>,
    do_parse!(
        game_date: datetime >>
        start_date: datetime >>
        (V3Data { game_date, start_date })
    )
);

#[derive(Clone, Debug, PartialEq)]
pub struct NewGRFHash(pub [u8; 16]);

impl fmt::Display for NewGRFHash {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        for byte in self.0.iter() {
            write!(fmt, "{:02x}", byte)?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct V4Data {
    pub active_newgrf: HashMap<u32, NewGRFHash>,
}

impl ByteWriter for V4Data {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        if self.active_newgrf.len() > 255 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "NewGRF maximum number is 255",
            ));
        }

        buf.push(self.active_newgrf.len() as u8);
        for (id, hash) in self
            .active_newgrf
            .clone()
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .into_iter()
        {
            buf.write_u32::<LittleEndian>(id)?;
            buf.extend_from_slice(&hash.0);
        }

        Ok(())
    }
}

named!(newgrf_md5<&[u8], NewGRFHash>,
    map!(take!(16), |v| {
        let mut out = [0; 16];
        out.copy_from_slice(v);
        NewGRFHash(out)
    })
);

named!(newgrf_entry<&[u8], (u32, NewGRFHash)>,
    do_parse!(
        id:  le_u32 >>
        md5: newgrf_md5 >>
        (id, md5)
    )
);

named!(parse_v4_data<&[u8], V4Data>,
    do_parse!(
        active_newgrf_num: le_u8 >>
        newgrf_data: count!(newgrf_entry, active_newgrf_num as usize) >>
        (V4Data {
            active_newgrf: newgrf_data.into_iter().collect::<HashMap<_, _>>()
        })
    )
);

#[derive(Clone, Debug, PartialEq)]
pub enum ProtocolVer {
    V1,
    V2(V2Data),
    V3(V2Data, V3Data),
    V4(V2Data, V3Data, V4Data),
}

impl<'a> From<&'a ProtocolVer> for u8 {
    fn from(v: &'a ProtocolVer) -> u8 {
        match *v {
            ProtocolVer::V1 => 1,
            ProtocolVer::V2(_) => 2,
            ProtocolVer::V3(_, _) => 3,
            ProtocolVer::V4(_, _, _) => 4,
        }
    }
}

impl ByteWriter for ProtocolVer {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.push((&*self).into());
        match *self {
            ProtocolVer::V1 => {}
            ProtocolVer::V2(ref v2data) => {
                v2data.write_pkt(buf)?;
            }
            ProtocolVer::V3(ref v2data, ref v3data) => {
                v3data.write_pkt(buf)?;
                v2data.write_pkt(buf)?;
            }
            ProtocolVer::V4(ref v2data, ref v3data, ref v4data) => {
                v4data.write_pkt(buf)?;
                v3data.write_pkt(buf)?;
                v2data.write_pkt(buf)?;
            }
        }
        Ok(())
    }
}

fn protocol_ver(buf: &[u8]) -> nom::IResult<&[u8], ProtocolVer> {
    let (buf, protocol_num) = be_u8(buf)?;
    match protocol_num {
        1 => Ok((buf, ProtocolVer::V1)),
        2 => {
            let (buf, v2) = parse_v2_data(buf)?;
            Ok((buf, ProtocolVer::V2(v2)))
        }
        3 => {
            let (buf, v3) = parse_v3_data(buf)?;
            let (buf, v2) = parse_v2_data(buf)?;
            Ok((buf, ProtocolVer::V3(v2, v3)))
        }
        4 => {
            let (buf, v4) = parse_v4_data(buf)?;
            let (buf, v3) = parse_v3_data(buf)?;
            let (buf, v2) = parse_v2_data(buf)?;
            Ok((buf, ProtocolVer::V4(v2, v3, v4)))
        }
        _ => Err(nom::Err::Failure((buf, nom::error::ErrorKind::OneOf))),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerResponse {
    pub protocol_ver: ProtocolVer,
    pub server_name: CString,
    pub server_revision: CString,
    pub server_lang: u8,
    pub use_password: bool,
    pub clients_max: u8,
    pub clients_on: u8,
    pub spectators_on: u8,
    pub map_name: CString,
    pub map_width: u16,
    pub map_height: u16,
    pub map_set: u8,
    pub dedicated: bool,
}

impl ByteWriter for ServerResponse {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        self.protocol_ver.write_pkt(buf)?;
        buf.append(&mut self.server_name.clone().into_bytes_with_nul());
        buf.append(&mut self.server_revision.clone().into_bytes_with_nul());
        buf.push(self.server_lang);
        buf.push(if self.use_password { 1 } else { 0 });
        buf.push(self.clients_max);
        buf.push(self.clients_on);
        buf.push(self.spectators_on);

        if u8::from(&self.protocol_ver) < 3 {
            buf.append(&mut vec![0; 4]);
        }

        buf.append(&mut self.map_name.clone().into_bytes_with_nul());
        buf.write_u16::<LittleEndian>(self.map_width)?;
        buf.write_u16::<LittleEndian>(self.map_height)?;
        buf.push(self.map_set);
        buf.push(if self.dedicated { 1 } else { 0 });

        Ok(())
    }
}

named!(pub parse_server_response<&[u8], ServerResponse>,
    do_parse!(
        protocol_ver: protocol_ver >>
        server_name: read_cstring >>
        server_revision: read_cstring >>

        server_lang: le_u8 >>
        use_password: map!(le_u8, |v| v > 0) >>
        clients_max: le_u8 >>
        clients_on: le_u8 >>
        spectators_on: le_u8 >>

        cond!(u8::from(&protocol_ver) < 3, take!(4)) >>

        map_name: read_cstring >>
        map_width: le_u16 >>
        map_height: le_u16 >>
        map_set: le_u8 >>
        dedicated: map!(le_u8, |v| v > 0) >>

        (ServerResponse {
            protocol_ver,
            server_name,
            server_revision,
            server_lang,
            use_password,
            clients_max,
            clients_on,
            spectators_on,
            map_name,
            map_width,
            map_height,
            map_set,
            dedicated,
        })
    )
);

#[cfg(test)]
mod tests {
    use super::*;

    use hex_literal::hex;

    pub(crate) fn fixtures() -> (Vec<u8>, ServerResponse) {
        let b = hex!(
            "
            0403444E070048B3F9E4FD0DF2A72B5F44D3C8A
            2F4A04D4703052E96B9AB2BEA686BFF94961AD4
            33A70132323322316180DA1BA6444A06CD17F8F
            A79D60A63EC0A0063EC0A000F000A4F6E6C7946
            7269656E6473204F70656E54544420536572766
            57220233100312E352E3300160019000052616E
            646F6D204D617000000400040101
        "
        )
        .to_vec();

        let srv = ServerResponse {
            protocol_ver: ProtocolVer::V4(
                V2Data {
                    max_companies: 15,
                    current_companies: 0,
                    max_spectators: 10,
                },
                V3Data {
                    game_date: DateTime::from_utc(NaiveDateTime::from_timestamp(715875, 0), Utc),
                    start_date: DateTime::from_utc(NaiveDateTime::from_timestamp(715875, 0), Utc),
                },
                V4Data {
                    active_newgrf: hashmap! {
                        0x00074e44 => NewGRFHash(hex!("48b3f9e4fd0df2a72b5f44d3c8a2f4a0")),
                        0x0503474d => NewGRFHash(hex!("2e96b9ab2bea686bff94961ad433a701")),
                        0x22333232 => NewGRFHash(hex!("316180da1ba6444a06cd17f8fa79d60a")),
                    },
                },
            ),
            server_name: CString::new("OnlyFriends OpenTTD Server #1").unwrap(),
            map_name: CString::new("Random Map").unwrap(),
            clients_on: 0,
            clients_max: 25,
            use_password: false,
            server_revision: CString::new("1.5.3").unwrap(),
            server_lang: 22,
            spectators_on: 0,
            map_width: 1024,
            map_height: 1024,
            map_set: 1,
            dedicated: true,
        };

        (b, srv)
    }

    #[test]
    fn test_parse_server_response() {
        let (input, expectation) = fixtures();

        let result = parse_server_response(&input).unwrap();

        assert_eq!(expectation, result.1);
    }

    #[test]
    fn test_write_server_response() {
        let (expectation, input) = fixtures();

        let mut result = Vec::new();
        input.write_pkt(&mut result).unwrap();

        assert_eq!(expectation, result);
    }
}
