extern crate byteorder;
extern crate chrono;
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate maplit;
#[macro_use]
extern crate nom;

pub mod errors;
pub mod util;

pub mod server_response;
pub use server_response::*;

use nom::*;

/// Enum representing various OpenTTD UDP packet types.
#[derive(Clone, Copy, Debug)]
pub enum PacketType {
    /// Queries a game server for game information
    ClientInfoFindServer,
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
            ClientInfoFindServer => 0,
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
            0 => Some(ClientInfoFindServer),
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
    ServerResponse(ServerResponse),
}

impl Packet {
    named!(pub parse_packet<&[u8], Packet>,
        do_parse!(
            _packet_len: le_u16 >>
            packet_type: map_opt!(le_u8, PacketType::from_num) >>
            packet: switch!(value!(packet_type),
                PacketType::ServerResponse => map!(parse_server_response, Packet::ServerResponse) |
                _ => value!(unreachable!())
            ) >>
            (packet)
        )
    );
}