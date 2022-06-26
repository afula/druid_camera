pub mod video;

use druid::{Data, Lens};

use crate::gui::data::video::VideoViewState;

/// App UI widget state.
#[derive(Debug, Clone, Data, Lens)]
pub struct AppState {
	pub video: VideoViewState,
	pub theme: Theme,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Data)]
pub enum Theme {
	Light,
	Dark,
}

impl Default for Theme {
	fn default() -> Self {
		Self::Light
	}
}
