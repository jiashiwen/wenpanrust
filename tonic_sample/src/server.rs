use tonic::{transport::Server, Request, Response, Status};

use helloproto::{
    greeter_server::{Greeter, GreeterServer},
    HelloReply, HelloRequest,
};

pub mod helloproto {
    tonic::include_proto!("firestproto"); // The string specified here must match the proto package name
}

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>, // Accept request of type HelloRequest
    ) -> Result<Response<HelloReply>, Status> {
        // Return an instance of type HelloReply
        println!("Got a request: {:?}", request);

        let reply = helloproto::HelloReply {
            message: format!("Hello {}!", request.into_inner().name).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
            number: 1,
        };

        Ok(Response::new(reply)) // Send back our formatted greeting
    }
}
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "127.0.0.1:50051".parse()?;
    let greeter = MyGreeter::default();

    Server::builder()
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
