use futures_core::stream::Stream;
use futures_core::task::{Context, Poll};
use glib::prelude::*;
use std::cell::Cell;
use std::pin::Pin;

pub use futures_channel::mpsc;
pub use paste;

pub struct EvStream<T> {
    object: glib::WeakRef<glib::object::Object>,
    signal_id: Cell<Option<glib::SignalHandlerId>>,
    receiver: mpsc::UnboundedReceiver<T>,
}

impl<T> EvStream<T> {
    pub fn new(
        object: glib::WeakRef<glib::object::Object>,
        signal_id: glib::SignalHandlerId,
        receiver: mpsc::UnboundedReceiver<T>,
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

#[macro_export]
macro_rules! ev_stream {
    ($this:expr, $event:ident, | $($x:ident),* | $cloning_body:expr) => {
        {
            let (s, r) = $crate::mpsc::unbounded();
            let signal_id = $crate::paste::expr!($this.[<connect_ $event>](move |$($x,)*| {
                let args = $cloning_body;
                s.unbounded_send(args).expect("sending value in ev_stream");
            }));
            $crate::EvStream::new($this.clone().upcast::<glib::Object>().downgrade(), signal_id, r)
        }
    };
    ($this:expr, $event:ident, | $($x:ident),* |) => {
        $crate::ev_stream!($this, $event, | $($x),* | {
            ($($x.clone()),*) // tuple with cloned elements
        })
    }
}
