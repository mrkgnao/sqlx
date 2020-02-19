use std::collections::HashMap;
use std::sync::Arc;

use crate::decode::Decode;
use crate::postgres::protocol::DataRow;
use crate::postgres::{PgConnection, Postgres};
use crate::row::Row;
use crate::types::HasSqlType;

pub struct PgRow<'a> {
    pub(super) connection: &'a PgConnection,
    pub(super) data: DataRow,
    pub(super) columns: Arc<HashMap<Box<str>, usize>>,
}

impl<'a> Row<'a> for PgRow<'a> {
    type Database = Postgres;

    fn len(&self) -> usize {
        // self.data.len()
        0
    }

    fn try_get(&self, index: usize) -> crate::Result<Option<&[u8]>>
// where
    //     Self::Database: HasSqlType<T>,
    //     I: RowIndex<Self>,
    //     T: Decode<Self::Database>,
    {
        //index.try_get(self).unwrap()
        Ok(self.data.get(
            self.connection.stream.buffer(),
            &self.connection.data_row_values_buf,
            index,
        ))
    }
}
//
// impl RowIndex<PgRow> for usize {
//     fn try_get<T>(&self, row: &PgRow) -> crate::Result<T>
//     where
//         <PgRow as Row>::Database: HasSqlType<T>,
//         T: Decode<<PgRow as Row>::Database>,
//     {
//         Ok(Decode::decode_nullable(
//             row.data.get(row.connection.stream.buffer(), *self),
//         )?)
//     }
// }
//
// impl RowIndex<PgRow> for &'_ str {
//     fn try_get<T>(&self, row: &PgRow) -> crate::Result<T>
//     where
//         <PgRow as Row>::Database: HasSqlType<T>,
//         T: Decode<<PgRow as Row>::Database>,
//     {
//         todo!()
//
//         // let index = row
//         //     .columns
//         //     .get(*self)
//         //     .ok_or_else(|| crate::Error::ColumnNotFound((*self).into()))?;
//         // let value = Decode::decode_nullable(row.data.get(*index))?;
//         //
//         // Ok(value)
//     }
// }

// impl_from_row_for_row!(PgRow);
