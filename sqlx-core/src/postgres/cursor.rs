use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures_core::future::BoxFuture;
use futures_core::stream::BoxStream;

use crate::cursor::Cursor;
use crate::database::HasRow;
use crate::postgres::protocol::{DataRow, Message, StatementId};
use crate::postgres::{PgConnection, PgRow};
use crate::Postgres;

// TODO: &Pool<PgConnection>
// TODO: PoolConnection<PgConnection>

pub struct PgCursor<'a> {
    statement: StatementId,
    connection: &'a mut PgConnection,
}

impl<'a> PgCursor<'a> {
    pub(super) fn from_connection(
        connection: &'a mut PgConnection,
        statement: StatementId,
    ) -> Self {
        Self {
            connection,
            statement,
        }
    }
}

impl<'a> Cursor<'a> for PgCursor<'a> {
    type Database = Postgres;

    fn first(self) -> BoxFuture<'a, crate::Result<Option<<Self::Database as HasRow<'a>>::Row>>> {
        Box::pin(first(self))
    }

    fn next(&mut self) -> BoxFuture<crate::Result<Option<<Self::Database as HasRow>::Row>>> {
        todo!()
    }

    fn map<T, F>(self, f: F) -> BoxStream<'a, crate::Result<T>>
    where
        F: Fn(<Self::Database as HasRow>::Row) -> T,
    {
        todo!()
    }
}

impl<'a> Future for PgCursor<'a> {
    type Output = crate::Result<u64>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        todo!()
    }
}

async fn wait_for_ready(connection: &mut PgConnection) -> crate::Result<()> {
    if !connection.is_ready {
        loop {
            if let Message::ReadyForQuery = connection.stream.read().await? {
                // we are now ready to go
                connection.is_ready = true;
                break;
            }
        }
    }

    Ok(())
}

// noinspection RsNeedlessLifetimes
async fn first<'a>(cursor: PgCursor<'a>) -> crate::Result<Option<PgRow<'a>>> {
    wait_for_ready(cursor.connection).await?;

    cursor.connection.stream.flush().await?;
    cursor.connection.is_ready = false;

    loop {
        match cursor.connection.stream.read().await? {
            Message::ParseComplete | Message::BindComplete => {
                // ignore complete messages
            }

            Message::DataRow => {
                let data = DataRow::read(
                    cursor.connection.stream.buffer(),
                    &mut cursor.connection.data_row_values_buf,
                )?;

                return Ok(Some(PgRow {
                    connection: cursor.connection,
                    columns: Arc::default(),
                    data,
                }));
            }

            message => {
                return Err(protocol_err!("unexpected message: {:?}", message).into());
            }
        }
    }

    Ok(None)
}
