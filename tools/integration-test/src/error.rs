//! Error type used for the tests.

use core::convert::{From, Into};
use eyre::Report;
use flex_error::{define_error, TraceError};
use ibc_relayer::channel::error::ChannelError;
use ibc_relayer::connection::ConnectionError;
use ibc_relayer::error::Error as RelayerError;
use ibc_relayer::supervisor::error::Error as SupervisorError;
use ibc_relayer::transfer::PacketError;
use std::io::Error as IoError;

define_error! {
    Error {
        Generic
            [ TraceError<Report> ]
            | _ | { "generic error" },

        Io
            [ TraceError<IoError> ]
            | _ | { "io error"},

        Relayer
            [ RelayerError ]
            | _ | { "relayer error"},

        Supervisor
            [ SupervisorError ]
            | _ | { "supervisor error"},

        Channel
            [ ChannelError ]
            | _ | { "channel error"},

        Connection
            [ ConnectionError ]
            | _ | { "connection error"},

        Packet
            [ PacketError ]
            | _ | { "packet error"},
    }
}

pub fn handle_generic_error(e: impl Into<Report>) -> Error {
    Error::generic(e.into())
}

impl From<Report> for Error {
    fn from(e: Report) -> Self {
        Error::generic(e)
    }
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::io(e)
    }
}

impl From<RelayerError> for Error {
    fn from(e: RelayerError) -> Self {
        Error::relayer(e)
    }
}

impl From<SupervisorError> for Error {
    fn from(e: SupervisorError) -> Self {
        Error::supervisor(e)
    }
}

impl From<ChannelError> for Error {
    fn from(e: ChannelError) -> Self {
        Error::channel(e)
    }
}

impl From<ConnectionError> for Error {
    fn from(e: ConnectionError) -> Self {
        Error::connection(e)
    }
}

impl From<PacketError> for Error {
    fn from(e: PacketError) -> Self {
        Error::packet(e)
    }
}
