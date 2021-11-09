//! Minimal Redis client implementation
//!
//! Provides an async connect and methods for issuing the supported commands.

use crate::cmd::{Get,Set};
use crate::{Connection, Frame};

use std::net::ToSocketAddrs;
use bytes::Bytes;
use std::io::{Error, ErrorKind};

use glommio::{
    net::{TcpStream},
};


/// Established connection with a Redis server.
///
/// Backed by a single `TcpStream`, `Client` provides basic network client
/// functionality (no pooling, retrying, ...). Connections are established using
/// the [`connect`](fn@connect) function.
///
/// Requests are issued using the various methods of `Client`.
pub struct Client {
    /// The TCP connection decorated with the redis protocol encoder / decoder
    /// implemented using a buffered `TcpStream`.
    ///
    /// When `Listener` receives an inbound connection, the `TcpStream` is
    /// passed to `Connection::new`, which initializes the associated buffers.
    /// `Connection` allows the handler to operate at the "frame" level and keep
    /// the byte level protocol parsing details encapsulated in `Connection`.
    connection: Connection,
}


/// Establish a connection with the Redis server located at `addr`.
///
/// `addr` may be any type that can be asynchronously converted to a
/// `SocketAddr`. This includes `SocketAddr` and strings. The `ToSocketAddrs`
/// trait is the Tokio version and not the `std` version.
///
/// # Examples
///
/// ```no_run
/// use mini_redis::client;
//
/// #[tokio::main]
/// async fn main() {
///     let client = match client::connect("localhost:6379").await {
///         Ok(client) => client,
///         Err(_) => panic!("failed to establish connection"),
///     };
/// # drop(client);
/// }
/// ```
///
pub async fn connect<T: ToSocketAddrs>(addr: T) -> Client {
    // The `addr` argument is passed directly to `TcpStream::connect`. This
    // performs any asynchronous DNS lookup and attempts to establish the TCP
    // connection. An error at either step returns an error, which is then
    // bubbled up to the caller of `mini_redis` connect.
    if let Ok(socket) = TcpStream::connect(addr).await {
        let connection = Connection::new(socket);
        Client {connection }
    } else {
        panic!("failed to establish connection");
    }
}

impl Client {
    /// Get the value of key.
    ///
    /// If the key does not exist the special value `None` is returned.
    ///
    /// # Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use mini_redis::client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = client::connect("localhost:6379").await.unwrap();
    ///
    ///     let val = client.get("foo").await.unwrap();
    ///     println!("Got = {:?}", val);
    /// }
    /// ```
    pub async fn get(&mut self, key: &str) -> crate::Result<Option<Bytes>> {
        // Create a `Get` command for the `key` and convert it to a frame.
        let frame = Get::new(key).into_frame();

        // Write the frame to the socket. This writes the full frame to the
        // socket, waiting if necessary.
        self.connection.write_frame(&frame).await?;

        println!("read frame is {:?}", frame);

        // Wait for the response from the server
        //
        // Both `Simple` and `Bulk` frames are accepted. `Null` represents the
        // key not being present and `None` is returned.
        match self.read_response().await? {
            Frame::Simple(value) => Ok(Some(value.into())),
            Frame::Bulk(value) => Ok(Some(value)),
            Frame::Null => Ok(None),
            frame => Err(frame.to_error()),
        }
    }

    /// Set `key` to hold the given `value`.
    ///
    /// The `value` is associated with `key` until it is overwritten by the next
    /// call to `set` or it is removed.
    ///
    /// If key already holds a value, it is overwritten. Any previous time to
    /// live associated with the key is discarded on successful SET operation.
    ///
    /// # Examples
    ///
    /// Demonstrates basic usage.
    ///
    /// ```no_run
    /// use mini_redis::client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let mut client = client::connect("localhost:6379").await.unwrap();
    ///
    ///     client.set("foo", "bar".into()).await.unwrap();
    ///
    ///     // Getting the value immediately works
    ///     let val = client.get("foo").await.unwrap().unwrap();
    ///     assert_eq!(val, "bar");
    /// }
    /// ```
    pub async fn set(&mut self, key: &str, value: Bytes) -> crate::Result<()> {
        // Create a `Set` command and pass it to `set_cmd`. A separate method is
        // used to set a value with an expiration. The common parts of both
        // functions are implemented by `set_cmd`.
        self.set_cmd(Set::new(key, value, None)).await
    }



    /// The core `SET` logic, used by both `set` and `set_expires.
    async fn set_cmd(&mut self, cmd: Set) -> crate::Result<()> {
        // Convert the `Set` command into a frame
        let frame = cmd.into_frame();

        // Write the frame to the socket. This writes the full frame to the
        // socket, waiting if necessary.
        self.connection.write_frame(&frame).await.unwrap();

        // Wait for the response from the server. On success, the server
        // responds simply with `OK`. Any other response indicates an error.
        match self.read_response().await? {
            Frame::Simple(response) if response == "OK" => Ok(()),
            frame => Err(frame.to_error()),
        }
    }


    /// Reads a response frame from the socket.
    ///
    /// If an `Error` frame is received, it is converted to `Err`.
    async fn read_response(&mut self) -> crate::Result<Frame> {
        let response = self.connection.read_frame().await?;
        
        match response {
            // Error frames are converted to `Err`
            Some(Frame::Error(msg)) => Err(msg.into()),
            Some(frame) => Ok(frame),
            None => {
                // Receiving `None` here indicates the server has closed the
                // connection without sending a frame. This is unexpected and is
                // represented as a "connection reset by peer" error.
                let err = Error::new(ErrorKind::ConnectionReset, "connection reset by server");

                Err(err.into())
            }
        }
    }
}