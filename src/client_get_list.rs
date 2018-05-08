use super::ServerListType;

use byteorder::WriteBytesExt;
use nom::*;
use std;

#[derive(Clone, Debug, PartialEq)]
pub struct ClientGetListData {
    pub master_server_version: u8,
    pub request_type: ServerListType,
}

impl ClientGetListData {
    pub fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.write_u8(self.master_server_version)?;
        buf.write_u8(self.request_type.into())?;

        Ok(())
    }
}

named!(pub parse_client_get_list<&[u8], ClientGetListData>,
    do_parse!(
        master_server_version: le_u8 >>
        request_type: map_opt!(le_u8, ServerListType::from_num) >>
        (ClientGetListData {
            master_server_version,
            request_type,
        })
    )
);
