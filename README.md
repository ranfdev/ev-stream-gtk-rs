# ev-stream-gtk-rs
This is a little macro to help you handle gtk-rs events in an async way, by treating them as streams.

Example:
```rust
let my_stream = ev_stream!(button, clicked, |target|);
```

### Advantages
#### Streams can have inner state
Instead of having mutable shared state, streams can have some inner state.
Skipping shared state means less cloning => more performance and better ergonomics.

Let's say I want to print the number of times a button gets clicked:
```rust
let click_stream = ev_stream!(button, clicked, |target|);
let fut = click_stream
  .zip(0..)
  .for_each(|(_, n)| println!("Clicked {} times!", n));
```

You can also keep some data between one event and the next, using fold.
I use this to keep a request in the background, until a new event comes.
When the next event comes, the previous request gets dropped and cancelled (automatically).
```rust
let search_changed = ev_stream!(search_text_box, search_changed, |target|);
let fut = search_changed
  .fold(None::<RemoteHandle<()>>, |state, target| {
    fetch_data(target.text())
  });
```

#### Multiple streams can be combined:
```rust
// Basic:
let stream1 = /*...*/;
let stream2 = /*...*/;
let stream3 = /*...*/;
let big_stream = stream1.chain(stream2).chain(stream3);


// Print "Hi" one time. Then print "Hi" again after every click.
let click_stream = /*as before*/;
let click_stream_tokenized = click_stream.map(|_| ())
let fut = future::once(async {()})
  .chain(click_stream_tokenized)
  .for_each(|_| println!("Hi"));
```
The last example in a real context may become: "Load data once, then load next data when `bottom_reached` event is fired" (I use this in my app).


#### They do automatic cleanup
The callback gets disconnected from the target widget when the stream is dropped.

#### They require a single async ctx.
It's true that streams must be awaited for them to work.
But you can await multiple streams in a single async ctx, without the need to create an async ctx for each
callback!

```rust
spawn!(async move {
  join!(button_clicked, bottom_reached)
});
```
