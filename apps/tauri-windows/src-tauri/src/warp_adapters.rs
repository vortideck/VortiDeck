//! Converts Warp WebSocket halves to the tungstenite sink/stream contracts
//! used by the legacy device protocol dispatcher.

use futures_util::{Sink, Stream};
use tokio_tungstenite::tungstenite::{self, Message};

pub(super) struct WarpWriteAdapter {
    inner: futures_util::stream::SplitSink<warp::ws::WebSocket, warp::ws::Message>,
}

impl WarpWriteAdapter {
    pub(super) fn new(
        inner: futures_util::stream::SplitSink<warp::ws::WebSocket, warp::ws::Message>,
    ) -> Self {
        Self { inner }
    }
}

impl Sink<Message> for WarpWriteAdapter {
    type Error = tungstenite::Error;

    fn poll_ready(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::pin::Pin::new(&mut self.inner)
            .poll_ready(cx)
            .map_err(|_| tungstenite::Error::ConnectionClosed)
    }

    fn start_send(mut self: std::pin::Pin<&mut Self>, item: Message) -> Result<(), Self::Error> {
        let message = match item {
            Message::Text(text) => warp::ws::Message::text(text.to_string()),
            Message::Binary(data) => warp::ws::Message::binary(data),
            Message::Close(_) => warp::ws::Message::close(),
            _ => return Ok(()),
        };

        std::pin::Pin::new(&mut self.inner)
            .start_send(message)
            .map_err(|_| tungstenite::Error::ConnectionClosed)
    }

    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::pin::Pin::new(&mut self.inner)
            .poll_flush(cx)
            .map_err(|_| tungstenite::Error::ConnectionClosed)
    }

    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::pin::Pin::new(&mut self.inner)
            .poll_close(cx)
            .map_err(|_| tungstenite::Error::ConnectionClosed)
    }
}

pub(super) struct WarpReadAdapter {
    inner: futures_util::stream::SplitStream<warp::ws::WebSocket>,
}

impl WarpReadAdapter {
    pub(super) fn new(inner: futures_util::stream::SplitStream<warp::ws::WebSocket>) -> Self {
        Self { inner }
    }
}

impl Stream for WarpReadAdapter {
    type Item = Result<Message, tungstenite::Error>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match std::pin::Pin::new(&mut self.inner).poll_next(cx) {
            std::task::Poll::Ready(Some(Ok(message))) if message.is_text() => {
                match message.to_str() {
                    Ok(text) => {
                        std::task::Poll::Ready(Some(Ok(Message::Text(text.to_string().into()))))
                    }
                    Err(_) => std::task::Poll::Ready(Some(Err(tungstenite::Error::Utf8))),
                }
            }
            std::task::Poll::Ready(Some(Ok(message))) if message.is_binary() => {
                std::task::Poll::Ready(Some(Ok(Message::Binary(
                    message.as_bytes().to_vec().into(),
                ))))
            }
            std::task::Poll::Ready(Some(Ok(message))) if message.is_close() => {
                std::task::Poll::Ready(Some(Ok(Message::Close(None))))
            }
            std::task::Poll::Ready(Some(Ok(_))) => std::task::Poll::Ready(None),
            std::task::Poll::Ready(Some(Err(_))) => {
                std::task::Poll::Ready(Some(Err(tungstenite::Error::ConnectionClosed)))
            }
            std::task::Poll::Ready(None) => std::task::Poll::Ready(None),
            std::task::Poll::Pending => std::task::Poll::Pending,
        }
    }
}
