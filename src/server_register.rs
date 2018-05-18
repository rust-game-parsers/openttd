use util::*;

use byteorder::{LittleEndian, WriteBytesExt};
use nom::*;
use std;
use std::ffi::CString;

#[derive(Clone, Debug, PartialEq)]
pub struct ServerRegistrationData {
    pub welcome_message: CString,
    pub server_version: u8,
    pub port: u16,
    pub session_key: u64,
}

impl ServerRegistrationData {
    pub fn write_pkt(&self, buf: &mut Vec<u8>) -> std::io::Result<()> {
        buf.append(&mut self.welcome_message.clone().into_bytes_with_nul());
        buf.write_u8(self.server_version)?;
        buf.write_u16::<LittleEndian>(self.port)?;
        buf.write_u64::<LittleEndian>(self.session_key)?;

        Ok(())
    }
}

named!(pub parse_server_register<&[u8], ServerRegistrationData>,
    do_parse!(
        welcome_message: read_cstring >>
        server_version: le_u8 >>
        port: le_u16 >>
        session_key: le_u64 >>
        (ServerRegistrationData {
            welcome_message,
            server_version,
            port,
            session_key,
        })
    )
);
