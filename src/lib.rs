extern crate byteorder;
extern crate chrono;
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate nom;

mod errors;
pub use errors::*;

mod util;
use util::*;

mod server_response;
use server_response::*;
pub use server_response::{ProtocolVer, ServerResponse, V2Data, V3Data, V4Data};

mod server_detail_info;
pub use server_detail_info::*;

mod server_register;
pub use server_register::*;

mod client_get_list;
pub use client_get_list::*;

mod master_response_list;
pub use master_response_list::*;

mod server_unregister;
pub use server_unregister::*;

use byteorder::{LittleEndian, WriteBytesExt};
use nom::*;

/// Enum representing various OpenTTD UDP packet types.
#[derive(Clone, Copy, Debug)]
pub enum PacketType {
    /// Queries a game server for game information
    ClientFindServer,
    /// Reply of the game server with game information
    ServerResponse,
    /// Queries a game server about details of the game, such as companies
    ClientDetailInfo,
    /// Reply of the game server about details of the game, such as companies
    ServerDetailInfo,
    /// Packet to register itself to the master server
    ServerRegister,
    /// Packet indicating registration has succeeded
    MasterAckRegister,
    /// Request for serverlist from master server
    ClientGetList,
    /// Response from master server with server ip's + port's
    MasterResponseList,
    /// Request to be removed from the server-list
    ServerUnregister,
    /// Requests the name for a list of GRFs (GRF_ID and MD5)
    ClientGetNewGRFs,
    /// Sends the list of NewGRF's requested.
    ServerNewGRFs,
    /// Sends a fresh session key to the client
    MasterSessionKey,
}

impl From<PacketType> for u8 {
    fn from(v: PacketType) -> Self {
        use PacketType::*;

        match v {
            ClientFindServer => 0,
            ServerResponse => 1,
            ClientDetailInfo => 2,
            ServerDetailInfo => 3,
            ServerRegister => 4,
            MasterAckRegister => 5,
            ClientGetList => 6,
            MasterResponseList => 7,
            ServerUnregister => 8,
            ClientGetNewGRFs => 9,
            ServerNewGRFs => 10,
            MasterSessionKey => 11,
        }
    }
}

impl PacketType {
    pub fn from_num(v: u8) -> Option<Self> {
        use PacketType::*;

        match v {
            0 => Some(ClientFindServer),
            1 => Some(ServerResponse),
            2 => Some(ClientDetailInfo),
            3 => Some(ServerDetailInfo),
            4 => Some(ServerRegister),
            5 => Some(MasterAckRegister),
            6 => Some(ClientGetList),
            7 => Some(MasterResponseList),
            8 => Some(ServerUnregister),
            9 => Some(ClientGetNewGRFs),
            10 => Some(ServerNewGRFs),
            11 => Some(MasterSessionKey),
            _ => None,
        }
    }
}

/// OpenTTD network packet
#[derive(Clone, Debug, PartialEq)]
pub enum Packet {
    ClientFindServer,
    ServerResponse(ServerResponse),
    ClientDetailInfo,
    ServerDetailInfo(ServerDetailInfo),
    ServerRegister(ServerRegistrationData),
    MasterAckRegister,
    ClientGetList(ClientGetListData),
    MasterResponseList(ServerList),
    ServerUnregister(ServerUnregisterData),
}

impl Packet {
    /// Get PacketType
    pub fn pkt_type(&self) -> PacketType {
        match *self {
            Packet::ClientFindServer => PacketType::ClientFindServer,
            Packet::ServerResponse(_) => PacketType::ServerResponse,
            Packet::ClientDetailInfo => PacketType::ClientDetailInfo,
            Packet::ServerDetailInfo(_) => PacketType::ServerDetailInfo,
            Packet::ServerRegister(_) => PacketType::ServerRegister,
            Packet::MasterAckRegister => PacketType::MasterAckRegister,
            Packet::ClientGetList(_) => PacketType::ClientGetList,
            Packet::MasterResponseList(_) => PacketType::MasterResponseList,
            Packet::ServerUnregister(_) => PacketType::ServerUnregister,
        }
    }

    /// Parse a UDP packet
    named!(pub from_incoming_bytes<&[u8], Packet>,
        do_parse!(
            _packet_len: le_u16 >>
            packet_type: map_opt!(le_u8, PacketType::from_num) >>
            packet: switch!(value!(packet_type),
                PacketType::ClientFindServer => value!(Packet::ClientFindServer) |
                PacketType::ServerResponse => map!(parse_server_response, Packet::ServerResponse) |
                PacketType::ClientDetailInfo => value!(Packet::ClientDetailInfo) |
                PacketType::ServerDetailInfo => map!(parse_server_detail_info, Packet::ServerDetailInfo) |
                PacketType::ServerRegister => map!(parse_server_register, Packet::ServerRegister) |
                PacketType::MasterAckRegister => value!(Packet::MasterAckRegister) |
                PacketType::ClientGetList => map!(parse_client_get_list, Packet::ClientGetList) |
                PacketType::MasterResponseList => map!(parse_master_response, Packet::MasterResponseList) |
                PacketType::ServerUnregister => map!(ServerUnregisterData::from_bytes, Packet::ServerUnregister) |
                _ => value!(unimplemented!())
            ) >>
            (packet)
        )
    );
}

impl ByteWriter for Packet {
    fn write_pkt(&self, out: &mut Vec<u8>) -> std::io::Result<()> {
        let buf = &mut vec![];
        buf.push(self.pkt_type().into());

        match *self {
            Packet::ServerResponse(ref data) => data.write_pkt(buf)?,
            Packet::ServerDetailInfo(ref data) => data.write_pkt(buf)?,
            Packet::ServerRegister(ref data) => data.write_pkt(buf)?,
            Packet::ClientGetList(ref data) => data.write_pkt(buf)?,
            Packet::MasterResponseList(ref data) => data.write_pkt(buf)?,
            Packet::ServerUnregister(ref data) => data.write_pkt(buf)?,
            _ => {}
        };

        out.write_u16::<LittleEndian>(buf.len() as u16 + 2)?;
        out.append(buf);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Vec<u8>, Packet) {
        let b = vec![3, 0, 2];
        let data = Packet::ClientDetailInfo;

        (b, data)
    }

    #[test]
    fn test_parse_packet() {
        let (input, expectation) = fixtures();

        let result = Packet::from_incoming_bytes(&input).unwrap();

        assert_eq!(expectation, result.1);
    }

    #[test]
    fn test_write_packet() {
        let (expectation, input) = fixtures();

        let mut result = Vec::new();
        input.write_pkt(&mut result).unwrap();

        assert_eq!(expectation, result);
    }
}
