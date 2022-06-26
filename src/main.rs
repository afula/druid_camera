use anyhow::Result;
use druid::{AppLauncher, LocalizedString, WindowDesc};
use druid_camera::{
	gui,
	gui::data::{
		video::{VideoPlayerState, VideoRate, VideoViewState},
		Theme,
	},
};
use gui::{data::AppState, ui::root_widget};

fn main() -> Result<()> {
	let window = WindowDesc::new(root_widget())
		.title(LocalizedString::new("Window-Title").with_placeholder("druid video"))
		.window_size((640.0, 480.0));
	let launcher = AppLauncher::with_window(window);
	let state = AppState {
		video: VideoViewState {
			camara_record: false
		},
		theme: Theme::Light,
	};

	launcher.log_to_console().launch(state).expect("running app");
	Ok(())
}
