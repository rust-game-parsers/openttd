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

/// Enum representing various OpenTTD UDP packet types.
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
