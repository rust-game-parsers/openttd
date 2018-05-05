use util::*;

use byteorder::{LittleEndian, WriteBytesExt};
use chrono::prelude::*;
use nom;
use nom::*;
use std;
use std::collections::{BTreeMap, HashMap};
use std::ffi::CString;

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
pub struct V4Data {
    pub active_newgrf: HashMap<u32, [u8; 16]>,
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
        for (id, hash) in self.active_newgrf
            .clone()
            .into_iter()
            .collect::<BTreeMap<_, _>>()
            .into_iter()
        {
            buf.write_u32::<LittleEndian>(id)?;
            buf.extend_from_slice(&hash);
        }

        Ok(())
    }
}

named!(newgrf_md5<&[u8], [u8; 16]>,
    map!(take!(16), |v| {
        let mut out = [0; 16];
        out.copy_from_slice(v);
        out
    })
);

named!(newgrf_entry<&[u8], (u32, [u8; 16])>,
    do_parse!(
        id:  le_u32 >>
        md5: newgrf_md5 >>
        (id, md5)
    )
);

fn parse_v4_data(buf: &[u8]) -> nom::IResult<&[u8], V4Data> {
    let mut active_newgrf = HashMap::new();
    let (mut buf, active_newgrf_num) = be_u8(buf)?;

    for _ in 0..active_newgrf_num {
        let (new_buf, v) = newgrf_entry(buf)?;
        active_newgrf.insert(v.0, v.1);
        buf = new_buf;
    }
    Ok((buf, V4Data { active_newgrf }))
}

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
        _ => Err(nom::Err::Failure(Context::Code(
            buf,
            ErrorKind::Custom(999),
        ))),
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
            buf.append(&mut vec![0, 0, 0, 0]);
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
        server_name: read_string >>
        server_revision: read_string >>

        server_lang: le_u8 >>
        use_password: map!(le_u8, |v| v > 0) >>
        clients_max: le_u8 >>
        clients_on: le_u8 >>
        spectators_on: le_u8 >>

        cond!(u8::from(&protocol_ver) < 3, take!(4)) >>

        map_name: read_string >>
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

    pub(crate) fn fixtures() -> (Vec<u8>, ServerResponse) {
        let b = vec![
            0x04, 0x03, 0x44, 0x4E, 0x07, 0x00, 0x48, 0xB3, 0xF9, 0xE4, 0xFD, 0x0D, 0xF2, 0xA7,
            0x2B, 0x5F, 0x44, 0xD3, 0xC8, 0xA2, 0xF4, 0xA0, 0x4D, 0x47, 0x03, 0x05, 0x2E, 0x96,
            0xB9, 0xAB, 0x2B, 0xEA, 0x68, 0x6B, 0xFF, 0x94, 0x96, 0x1A, 0xD4, 0x33, 0xA7, 0x01,
            0x32, 0x32, 0x33, 0x22, 0x31, 0x61, 0x80, 0xDA, 0x1B, 0xA6, 0x44, 0x4A, 0x06, 0xCD,
            0x17, 0xF8, 0xFA, 0x79, 0xD6, 0x0A, 0x63, 0xEC, 0x0A, 0x00, 0x63, 0xEC, 0x0A, 0x00,
            0x0F, 0x00, 0x0A, 0x4F, 0x6E, 0x6C, 0x79, 0x46, 0x72, 0x69, 0x65, 0x6E, 0x64, 0x73,
            0x20, 0x4F, 0x70, 0x65, 0x6E, 0x54, 0x54, 0x44, 0x20, 0x53, 0x65, 0x72, 0x76, 0x65,
            0x72, 0x20, 0x23, 0x31, 0x00, 0x31, 0x2E, 0x35, 0x2E, 0x33, 0x00, 0x16, 0x00, 0x19,
            0x00, 0x00, 0x52, 0x61, 0x6E, 0x64, 0x6F, 0x6D, 0x20, 0x4D, 0x61, 0x70, 0x00, 0x00,
            0x04, 0x00, 0x04, 0x01, 0x01,
        ];
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
                        478788 => [0x48, 0xb3, 0xf9, 0xe4, 0xfd, 0x0d, 0xf2, 0xa7, 0x2b, 0x5f, 0x44, 0xd3, 0xc8, 0xa2, 0xf4, 0xa0],
                        84100941 => [0x2e, 0x96, 0xb9, 0xab, 0x2b, 0xea, 0x68, 0x6b, 0xff, 0x94, 0x96, 0x1a, 0xd4, 0x33, 0xa7, 0x01],
                        573780530 => [0x31, 0x61, 0x80, 0xda, 0x1b, 0xa6, 0x44, 0x4a, 0x06, 0xcd, 0x17, 0xf8, 0xfa, 0x79, 0xd6, 0x0a],
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
