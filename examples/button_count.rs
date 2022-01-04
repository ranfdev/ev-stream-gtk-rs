use ev_stream_gtk_rs::ev_stream;
use futures::future::RemoteHandle;
use futures::join;
use futures::prelude::*;
use futures::task::LocalSpawnExt;
use gtk::glib;
use gtk::prelude::*;
use std::time::Duration;

fn search_in_background(
    text: String,
    search_status_label: gtk::Label,
) -> Option<RemoteHandle<()>> {
    glib::MainContext::default()
        .spawn_local_with_handle(async move {
            search_status_label.set_text(&format!("Searching {}", text));
            // Fake long search
            async_std::task::sleep(Duration::from_millis(1000)).await;
            search_status_label.set_text(&format!("RESULTS FOUND FOR {}", text));
        })
        .ok()
}
fn on_activate(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    let container = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let label = gtk::Label::new(Some("Button clicked 0 times"));
    let button = gtk::Button::with_label("Increase");
    let entry = gtk::SearchEntry::new();
    let search_status_label = gtk::Label::new(Some(""));
    container.append(&label);
    container.append(&button);
    container.append(&entry);
    container.append(&search_status_label);

    let clicks_fut = ev_stream!(button, clicked, |btn|)
        .zip(stream::iter(1..))
        .for_each(move |(_, n)| {
            future::ready({
                label.set_text(&format!("Button clicked {} times", n));
            })
        });

    // This can probably be implemented using a `debounce` adapter (currently missing from
    // `futures` crate).
    // The `RemoteHandle` ensures oldest searches get cancelled when a new one comes.
    let searches_fut = ev_stream!(entry, search_changed, |entry|).fold(
        None::<RemoteHandle<()>>,
        move |_state, entry| {
            future::ready(search_in_background(
                entry.text().to_string(),
                search_status_label.clone(),
            ))
        },
    );

    glib::MainContext::default().spawn_local(async move {
        join!(clicks_fut, searches_fut);
    });

    window.set_child(Some(&container));
    window.present();
}

fn main() {
    let app = gtk::Application::builder()
        .application_id("com.github.ev-stream-gtk-rs.examples.basic")
        .build();
    app.connect_activate(on_activate);
    app.run();
}
