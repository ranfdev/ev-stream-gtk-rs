use futures_core::stream::Stream;
use futures_core::task::{Context, Poll};
use glib::prelude::*;
use std::cell::Cell;
use std::pin::Pin;

pub use futures_channel::mpsc;
pub use glib::{object::Object, SignalHandlerId, WeakRef};
pub use paste;

/// `Stream` of `T` created with the [ev_stream]
/// Provides automatic callback disconnection on drop.
pub struct EvStream<T> {
    object: glib::WeakRef<glib::object::Object>,
    signal_id: Cell<Option<glib::SignalHandlerId>>,
    receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> EvStream<T> {
    pub fn new(
        object: WeakRef<Object>,
        signal_id: SignalHandlerId,
        receiver: mpsc::UnboundedReceiver<T>
    ) -> Self {
        Self {
            object,
            signal_id: Cell::new(Some(signal_id)),
            receiver,
        }
    }
}

impl<T> Stream for EvStream<T> {
    type Item = T;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.get_mut().receiver).poll_next(cx)
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
/// ```ignore
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
            let (s, r) = $crate::mpsc::unbounded();
            let object = $this.clone().upcast::<$crate::Object>().downgrade();
            let signal_id = $crate::paste::expr!($this.[<connect_ $event>](move |$($x,)*| {
                let args = $cloning_body;
                s.unbounded_send(args).expect("sending value in ev_stream");
            }));
            $crate::EvStream::new(object, signal_id, r)
        }
    };
    // Untyped macro (connects to the event by name, using a string)
    ($this:expr, $event:expr, | $($x:ident),* | $cloning_body:expr) => {
        {
            let (s, r) = $crate::mpsc::unbounded();
            let object = $this.clone().upcast::<Object>().downgrade();
            let signal_id = $this.connect_local($event, false, move |$($x,)*| {
                let args = $cloning_body;
                s.unbounded_send(args).expect("sending value in ev_stream");
                None
            });
            $crate::EvStream::new(object, signal_id, r)
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
