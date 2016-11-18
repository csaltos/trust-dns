// Copyright 2015-2016 Benjamin Fry <benjaminfry@me.com>
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.
use std;
use std::io;
use std::sync::Arc;

use futures::{Async, Future, Poll};
use futures::stream::Stream;
use tokio_core::reactor::Core;

use trust_dns::op::RequestHandler;
use trust_dns::udp::UdpStream;

use ::server::{Request, RequestStream, ResponseHandle};
use ::authority::Catalog;

// TODO, would be nice to have a Slab for buffers here...
pub struct ServerFuture {
  io_loop: Core,
  catalog: Arc<Catalog>, // should the catalog just be static?
}

impl ServerFuture {
  pub fn new(catalog: Catalog) -> io::Result<ServerFuture> {
    Ok(ServerFuture {
      io_loop: try!(Core::new()),
      catalog: Arc::new(catalog),
    })
  }

  /// register a UDP socket. Should be bound before calling this function.
  pub fn register_socket(&mut self, socket: std::net::UdpSocket) {
    // create the new UdpStream
    let (udp_stream, stream_handle) = UdpStream::with_bound(socket, self.io_loop.handle());
    let request_stream = RequestStream::new(udp_stream, stream_handle);
    let catalog = self.catalog.clone();

    self.io_loop.handle().spawn(request_stream.for_each(move |(request, response_handle)| {
      Self::handle_request(request, response_handle, catalog.clone())
    }).map_err(|e| debug!("error in request_stream handler? {}", e)));
  }

  /// register a TcpListener to the Server. This should already be bound to either an IPv6 or an
  ///  IPv4 address.
  pub fn register_listener(&mut self, listener: std::net::TcpListener) {

  }

  /// TODO how to do threads? should we do a bunch of listener threads and then query threads?
  /// Ideally the processing would be n-threads for recieving, which hand off to m-threads for
  ///  request handling. It would generally be the case that n <= m.
  pub fn listen(&mut self) -> io::Result<()> {
    info!("Server starting up");
    try!(self.io_loop.run(Forever));

    Err(io::Error::new(io::ErrorKind::Interrupted, "Server stopping due to interruption"))
  }

  fn handle_request(request: Request, response_handle: ResponseHandle, catalog: Arc<Catalog>) -> io::Result<()> {
    let response = catalog.handle_request(&request.message);
    response_handle.send(response)
  }
}

struct Forever;

impl Future for Forever {
  type Item = ();
  type Error = io::Error;

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    // run forever...
    Ok(Async::NotReady)
  }
}
