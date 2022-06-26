use std::time::Duration;

use druid::{
	widget::{
		Axis, Button, Controller, Either, Flex, KnobStyle, Label, RangeSlider, SizedBox, Slider,
		Stepper, ViewSwitcher,
	},
	Color, Cursor, Data, Env, Event, EventCtx, KeyOrValue, MouseButton, PaintCtx, Point, Rect,
	RenderContext, Size, Widget, WidgetExt,
};
use druid_widget_nursery::DropdownSelect;

use crate::gui::{
	controller::cmd,
	data::{
		video::{VideoPlayer, VideoPlayerState, VideoRate, VideoViewState},
		AppState,
	},
	widgets::{
		empty::Empty,
		icons::{self, SvgIcon},
		theme,
	},
};

pub fn panel_widget() -> impl Widget<AppState> {

	let controls = Flex::row().with_child(Either::new(
		|video: &VideoViewState, _| !video.camara_record,
		Button::new("Start Record").on_click(|ctx, state: &mut VideoViewState, _env| {
			state.camara_record = true;
			ctx.submit_command(cmd::PLAY_RESUME)
		}),
		Button::new("Stop Record").on_click(|ctx, state: &mut VideoViewState, _env| {
			state.camara_record = false;
			ctx.submit_command(cmd::PLAY_PAUSE)
		}),
	));

	Flex::column()
		.with_child(controls)
		.lens(AppState::video)
	// .controller(PlaybackController::new())
}
