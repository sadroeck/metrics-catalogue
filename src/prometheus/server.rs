use crate::prometheus::StringRender;
use hyper::server::{conn::AddrStream, Server as HyperServer};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Error, Response};
use std::future::Future;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::pin::Pin;

type ServerFuture = Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'static>>;

pub struct Server {
    listen_address: SocketAddr,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            listen_address: SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::from([0, 0, 0, 0]), 9000)),
        }
    }
}

impl Server {
    pub fn new(listen_address: impl Into<SocketAddr>) -> Self {
        Self {
            listen_address: listen_address.into(),
        }
    }

    pub fn run(
        self,
        root_name: &'static str,
        renderer: impl StringRender + Send + Sync + Clone + 'static,
    ) -> Result<ServerFuture, Error> {
        let server = HyperServer::try_bind(&self.listen_address)?;
        let exporter = async move {
            let make_svc = make_service_fn(move |_socket: &AddrStream| {
                let renderer = renderer.clone();
                async move {
                    Ok::<_, Error>(service_fn(move |_| {
                        let mut output = String::new();
                        renderer.render(root_name, "", &mut output);
                        async move { Ok::<_, Error>(Response::new(Body::from(output))) }
                    }))
                }
            });
            server.serve(make_svc).await
        };
        Ok(Box::pin(exporter))
    }
}
