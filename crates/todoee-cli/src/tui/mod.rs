pub mod app;
pub mod event;
pub mod handler;
pub mod terminal;
pub mod theme;
pub mod ui;
pub mod widgets;

pub use app::App;
pub use event::{Event, EventHandler};
pub use handler::handle_key_event;
pub use terminal::Tui;
// Theme is available for future customization: pub use theme::Theme;
