use util::*;

use byteorder::{LittleEndian, WriteBytesExt};
use nom::*;
use std;

#[derive(Clone, Debug, PartialEq)]
pub struct ServerUnregisterData {
    pub master_server_version: u8,
    pub port: u16,
}

impl ByteWriter for ServerUnregisterData {
    fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.write_u8(self.master_server_version)?;
        buf.write_u16::<LittleEndian>(self.port)?;

        Ok(())
    }
}

impl ServerUnregisterData {
    named!(pub from_bytes<&[u8], Self>,
        do_parse!(
            master_server_version: le_u8 >>
            port: le_u16 >>
            (ServerUnregisterData {
                master_server_version,
                port,
            })
        )
    );
}
