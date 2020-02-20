use std::convert::TryFrom;

use crate::postgres::protocol::{
    Authentication, BackendKeyData, CommandComplete, DataRow, NotificationResponse,
    ParameterDescription, ParameterStatus, ReadyForQuery, Response, RowDescription,
};

#[derive(Debug, Copy, Clone)]
#[repr(u8)]
pub enum Message {
    Authentication,
    BackendKeyData,
    BindComplete,
    CloseComplete,
    CommandComplete,
    DataRow,
    NoData,
    NotificationResponse,
    ParameterDescription,
    ParameterStatus,
    ParseComplete,
    PortalSuspended,
    ReadyForQuery,
    Response,
    RowDescription,
}

impl TryFrom<u8> for Message {
    type Error = crate::Error;

    fn try_from(type_: u8) -> crate::Result<Self> {
        // https://www.postgresql.org/docs/12/protocol-message-formats.html
        Ok(match type_ {
            b'N' | b'E' => Message::Response,
            b'D' => Message::DataRow,
            b'S' => Message::ParameterStatus,
            b'Z' => Message::ReadyForQuery,
            b'R' => Message::Authentication,
            b'K' => Message::BackendKeyData,
            b'C' => Message::CommandComplete,
            b'A' => Message::NotificationResponse,
            b'1' => Message::ParseComplete,
            b'2' => Message::BindComplete,
            b'3' => Message::CloseComplete,
            b'n' => Message::NoData,
            b's' => Message::PortalSuspended,
            b't' => Message::ParameterDescription,
            b'T' => Message::RowDescription,

            id => {
                return Err(protocol_err!("unknown message: {:?}", id).into());
            }
        })
    }
}
