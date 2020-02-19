use std::convert::TryInto;
use std::ops::Range;

use byteorder::NetworkEndian;
use futures_core::future::BoxFuture;

use crate::cache::StatementCache;
use crate::connection::{Connect, Connection};
use crate::io::{Buf, BufStream, MaybeTlsStream};
use crate::postgres::protocol::{
    self, Authentication, Decode, Encode, Message, StartupMessage, StatementId, Terminate,
};
use crate::postgres::stream::PgStream;
use crate::postgres::PgError;
use crate::url::Url;
use crate::Postgres;

// TODO: Authentcation
// TODO: SASL
// TODO: TLS

/// An asynchronous connection to a [Postgres][super::Postgres] database.
///
/// The connection string expected by [Connect::connect] should be a PostgreSQL connection
/// string, as documented at
/// <https://www.postgresql.org/docs/12/libpq-connect.html#LIBPQ-CONNSTRING>
///
/// ### TLS Support (requires `tls` feature)
/// This connection type supports the same `sslmode` query parameter that `libpq` does in
/// connection strings: <https://www.postgresql.org/docs/12/libpq-ssl.html>
///
/// ```text
/// postgresql://<user>[:<password>]@<host>[:<port>]/<database>[?sslmode=<ssl-mode>[&sslcrootcert=<path>]]
/// ```
/// where
/// ```text
/// ssl-mode = disable | allow | prefer | require | verify-ca | verify-full
/// path = percent (URL) encoded path on the local machine
/// ```
///
/// If the `tls` feature is not enabled, `disable`, `allow` and `prefer` are no-ops and `require`,
/// `verify-ca` and `verify-full` are forbidden (attempting to connect with these will return
/// an error).
///
/// If the `tls` feature is enabled, an upgrade to TLS is attempted on every connection by default
/// (equivalent to `sslmode=prefer`). If the server does not support TLS (because it was not
/// started with a valid certificate and key, see <https://www.postgresql.org/docs/12/ssl-tcp.html>)
/// then it falls back to an unsecured connection and logs a warning.
///
/// Add `sslmode=require` to your connection string to emit an error if the TLS upgrade fails.
///
/// If you're running Postgres locally, your connection string might look like this:
/// ```text
/// postgresql://root:password@localhost/my_database?sslmode=require
/// ```
///
/// However, like with `libpq` the server certificate is **not** checked for validity by default.
///
/// Specifying `sslmode=verify-ca` will cause the TLS upgrade to verify the server's SSL
/// certificate against a local CA root certificate; this is not the system root certificate
/// but is instead expected to be specified in one of a few ways:
///
/// * The path to the certificate can be specified by adding the `sslrootcert` query parameter
/// to the connection string. (Remember to percent-encode it!)
///
/// * The path may also be specified via the `PGSSLROOTCERT` environment variable (which
/// should *not* be percent-encoded.)
///
/// * Otherwise, the library will look for the Postgres global root CA certificate in the default
/// location:
///
///     * `$HOME/.postgresql/root.crt` on POSIX systems
///     * `%APPDATA%\postgresql\root.crt` on Windows
///
/// These locations are documented here: <https://www.postgresql.org/docs/12/libpq-ssl.html#LIBQ-SSL-CERTIFICATES>
/// If the root certificate cannot be found by any of these means then the TLS upgrade will fail.
///
/// If `sslmode=verify-full` is specified, in addition to checking the certificate as with
/// `sslmode=verify-ca`, the hostname in the connection string will be verified
/// against the hostname in the server certificate, so they must be the same for the TLS
/// upgrade to succeed.
pub struct PgConnection {
    pub(super) stream: PgStream,
    pub(super) next_statement_id: u32,
    pub(super) is_ready: bool,

    pub(super) data_row_values_buf: Vec<Option<Range<u32>>>,
}

// https://www.postgresql.org/docs/12/protocol-flow.html#id-1.10.5.7.3
async fn startup(stream: &mut PgStream, url: &Url) -> crate::Result<()> {
    // Defaults to postgres@.../postgres
    let username = url.username().unwrap_or("postgres");
    let database = url.database().unwrap_or("postgres");

    // See this doc for more runtime parameters
    // https://www.postgresql.org/docs/12/runtime-config-client.html
    let params = &[
        ("user", username),
        ("database", database),
        // Sets the display format for date and time values,
        // as well as the rules for interpreting ambiguous date input values.
        ("DateStyle", "ISO, MDY"),
        // Sets the display format for interval values.
        ("IntervalStyle", "iso_8601"),
        // Sets the time zone for displaying and interpreting time stamps.
        ("TimeZone", "UTC"),
        // Adjust postgres to return percise values for floats
        // NOTE: This is default in postgres 12+
        ("extra_float_digits", "3"),
        // Sets the client-side encoding (character set).
        ("client_encoding", "UTF-8"),
    ];

    stream.write(StartupMessage { params });
    stream.flush().await?;

    loop {
        match stream.read().await? {
            Message::Authentication => match Authentication::read(stream.buffer())? {
                Authentication::Ok => {
                    // do nothing. no password is needed to continue.
                }

                auth => {
                    return Err(
                        protocol_err!("requested unsupported authentication: {:?}", auth).into(),
                    );
                }
            },

            Message::BackendKeyData => {
                // do nothing. we do not care about the server values here.
                // todo: we should care and store these on the connection
            }

            Message::ParameterStatus => {
                // do nothing. we do not care about the server values here.
            }

            Message::ReadyForQuery => {
                // done. connection is now fully established and can accept
                // queries for execution.
                break;
            }

            type_ => {
                return Err(protocol_err!("unexpected message: {:?}", type_).into());
            }
        }
    }

    Ok(())
}

// https://www.postgresql.org/docs/12/protocol-flow.html#id-1.10.5.7.10
async fn terminate(mut stream: PgStream) -> crate::Result<()> {
    stream.write(Terminate);
    stream.flush().await?;
    stream.shutdown()?;

    Ok(())
}

impl PgConnection {
    pub(super) async fn new(url: crate::Result<Url>) -> crate::Result<Self> {
        let url = url?;
        let mut stream = PgStream::new(&url).await?;

        startup(&mut stream, &url).await?;

        Ok(Self {
            stream,
            data_row_values_buf: Vec::new(),
            next_statement_id: 1,
            is_ready: true,
        })
    }
}

impl Connect for PgConnection {
    fn connect<T>(url: T) -> BoxFuture<'static, crate::Result<PgConnection>>
    where
        T: TryInto<Url, Error = crate::Error>,
        Self: Sized,
    {
        Box::pin(PgConnection::new(url.try_into()))
    }
}

impl Connection for PgConnection {
    type Database = Postgres;

    fn close(self) -> BoxFuture<'static, crate::Result<()>> {
        Box::pin(terminate(self.stream))
    }
}
