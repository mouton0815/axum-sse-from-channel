# Serve axum SSE from a Tokio channel 

This snippet demonstrates how an `axum` SSE ("Server Sent Events") handler can receive messages
from a Tokio broadcast channel.

The trick is to pass the message `Sender` to the SSE handler via `axum`'s
[Router.with_state](https://docs.rs/axum/latest/axum/struct.Router.html#method.with_state) function.
When a HTTP client connects to the SSE handler, a new `Receiver` is subscribed and wrapped into a
[BroadcastStream](https://docs.rs/tokio-stream/0.1.14/tokio_stream/wrappers/struct.BroadcastStream.html).

Note that by definition of a [broadcast channel](https://docs.rs/tokio/latest/tokio/sync/broadcast/index.html),
the HTTP client will receive messages sent **after** the call to subscribe.
In the example, messages are created by a periodic task, but any source would be fine.

# Running the Server

```shell
RUST_LOG=debug cargo run
```

# Connecting a Client
```shell
curl http://localhost:3000/sse
```
