use futures_core::stream::Stream;
use futures_core::task::{Context, Poll};
use glib::prelude::*;
use std::cell::Cell;
use std::pin::Pin;

pub use futures_channel::mpsc;
use glib::{object::Object, SignalHandlerId, WeakRef};
pub use paste;

/// `Stream` of `T` created with the [ev_stream]
/// Provides automatic callback disconnection on drop.
pub struct EvStream<T> {
    object: glib::WeakRef<glib::object::Object>,
    signal_id: Cell<Option<glib::SignalHandlerId>>,
    receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> EvStream<T> {
    // The provided function `connect_sender_fun` should connect a callback to a glib `Object`
    // and send the arguments of the callback to the provided `UnboundedSender`.
    pub fn new(
        connect_sender_fun: impl Fn(mpsc::UnboundedSender<T>) -> (WeakRef<Object>, SignalHandlerId),
    ) -> Self {
        let (s, r) = mpsc::unbounded();
        let (object, signal_id) = connect_sender_fun(s);
        Self {
            object,
            signal_id: Cell::new(Some(signal_id)),
            receiver: r,
        }
    }
}

impl<T> Stream for EvStream<T> {
    type Item = T;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut Pin::get_mut(self).receiver).poll_next(cx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.receiver.size_hint()
    }
}

impl<T> std::ops::Drop for EvStream<T> {
    fn drop(&mut self) {
        self.object
            .upgrade()
            .map(|obj| obj.disconnect(self.signal_id.take().unwrap()));
    }
}

/// This macro let's you listen to glib events as streams.
///
/// The last argument, which resembles a closure, is needed to let 
/// the type system see what arguments need to be taken by the connect
/// callback and which need to be cloned.
///
/// The arguments must be cloned because they must live longer than the
/// callback inside the `connect` function.
/// If you need to manipulate the arguments inside the callback before they get into
/// the stream, you can do so by passing a closure body.
///
///
/// # Examples
/// ```
/// // typed, uses `connect_clicked` method
/// let clicks = ev_stream!(button, clicked, |btn|);
/// // untyped, uses general `connect_local` method
/// let clicks = ev_stream!(button, "clicked", |btn|);
///
/// let edges_reached = ev_stream!(scrolled_win, edge_reached, |win, edge|);
///
/// // Here I'm directly discarding _win. Also, edge is a `Copy` type and doesn't need to be
/// // cloned. To achieve this, I'm manually adding a closure body after `|...|`, telling the
/// // macro to manually manage the data.
/// let edges_reached = ev_stream!(scrolled_win, edge_reached, |_win, edge| edge);
/// ```
#[macro_export]
macro_rules! ev_stream {
    // Typed macro
    ($this:expr, $event:ident, | $($x:ident),* | $cloning_body:expr) => {
        {
            let object = $this.clone().downgrade();
            let connect_fun = move |sender| {
                let signal_id = $crate::paste::expr!($this.[<connect_ $event>](move |$($x,)*| {
                    let args = $cloning_body;
                    sender.unbounded_send(args).expect("sending value in ev_stream");
                }));
                (object, signal_id)
            }
            EvStream::new(connect_fun)
        }
    };
    // Untyped macro (connects to the event by name, using a string)
    ($this:expr, $event:expr, | $($x:ident),* | $cloning_body:expr) => {
        {
            let object = $this.clone().downgrade();
            let connect_fun = move |sender| {
                let signal_id = $this.connect_local($event, false, move |$($x,)*| {
                    let args = $cloning_body;
                    sender.unbounded_send(args).expect("sending value in ev_stream");
                    None
                });
                (object, signal_id)
            }
            EvStream::new(connect_fun)
        }
    };
    ($this:expr, $event:ident, | $($x:ident),* |) => {
        $crate::ev_stream!($this, $event, | $($x),* | {
            ($($x.clone()),*) // tuple with cloned elements
        })
    };
    ($this:expr, $event:expr, | $($x:ident),* |) => {
        $crate::ev_stream!($this, $event, | $($x),* | {
            ($($x.clone()),*) // tuple with cloned elements
        })
    }
}
