use std::path::PathBuf;
use anyhow::Error;
use druid::{
	kurbo::Circle,
	piet::{ImageFormat, InterpolationMode},
	widget::{Controller, FillStrat, Image},
	BoxConstraints, Color, Env, Event, EventCtx, ExtEventSink, ImageBuf, LayoutCtx, LifeCycle,
	LifeCycleCtx, MouseButton, PaintCtx, RenderContext, Selector, SingleUse, Size, Target,
	UpdateCtx, Widget,
};
use gst::prelude::*;
use gstreamer as gst;
use gstreamer::{event::Seek, Element, SeekFlags, SeekType, Caps, ElementFactory, Pipeline, State};
use gstreamer_app as gst_app;
use num_rational::Ratio;
use num_traits::ToPrimitive;

use crate::{
	gui::{
		controller::cmd,
		data::video::{
			Position, VideoError, VideoError::Duration, VideoPlayer, VideoPlayerState, VideoView,
			VideoViewState,
		},
	},
	media::thumbnail::Thumbnail,
};

impl VideoView {
	/// Create new camera view
	pub fn new() -> Self {
		let image_buf = ImageBuf::default();
		let image = Image::new(image_buf)
			.fill_mode(FillStrat::Fill)
			.interpolation_mode(InterpolationMode::Bilinear);

		Self { image, player: None, event: None }
	}
}

impl Widget<VideoViewState> for VideoView {
	fn event(&mut self, ctx: &mut EventCtx, event: &Event, data: &mut VideoViewState, env: &Env) {
		if let Event::Command(command) = event {
			if let Some(image_buf) = command.get(cmd::VIDEO_FRAME) {
				self.image.set_image_data(image_buf.to_owned());
				ctx.request_paint();
			}
			if let Some(_) = command.get(cmd::PLAY_PAUSE) {
				if let Some(ref player) = self.player {
					player.pipeline.set_state(gst::State::Paused);

				}
				// ctx.request_paint();
			}
			if let Some(_) = command.get(cmd::PLAY_RESUME) {
				if let Some(ref player) = self.player {
					player.pipeline.set_state(gst::State::Playing);

				}
			}
		}

		self.image.event(ctx, event, data, env)
	}

	fn lifecycle(
		&mut self,
		ctx: &mut LifeCycleCtx,
		event: &LifeCycle,
		data: &VideoViewState,
		env: &Env,
	) {
		match event {
			LifeCycle::WidgetAdded => {
				let uri = ".media/druid.mkv";
				let player = VideoPlayer::new(uri, false, ctx.get_external_handle()).unwrap();
				self.player = Some(player);
			}
			_ => {}
		}
		self.image.lifecycle(ctx, event, data, env)
	}

	fn update(
		&mut self,
		ctx: &mut UpdateCtx,
		old_data: &VideoViewState,
		data: &VideoViewState,
		env: &Env,
	) {
		self.image.update(ctx, old_data, data, env)
	}

	fn layout(
		&mut self,
		ctx: &mut LayoutCtx,
		bc: &BoxConstraints,
		data: &VideoViewState,
		env: &Env,
	) -> Size {
		self.image.layout(ctx, bc, data, env)
	}

	fn paint(&mut self, ctx: &mut PaintCtx, data: &VideoViewState, env: &Env) {
		self.image.paint(ctx, data, env);
	}
}


impl Drop for VideoPlayer {
	fn drop(&mut self) {
		self.pipeline.set_state(gst::State::Null).expect("failed to set state");
	}
}
/** Framerate */
#[derive(Debug, Clone, Copy)]
pub enum FrameRate {
	F24 = 24 as isize,
	F30 = 30 as isize,
}

impl Default for FrameRate {
	fn default() -> Self {
		FrameRate::F24
	}
}
impl VideoPlayer {
	/// Create a new video player from a given video which loads from `uri`.
	///
	/// If `live` is set then no duration is queried (as this will result in an
	/// error and is non-sensical for live streams). Set `live` if the streaming
	/// source is indefinite (e.g. a live stream). Note that this will cause the
	/// duration to be zero.
	pub fn new(uri: &str, live: bool, event_sink: ExtEventSink) -> Result<Self, VideoError> {
		let rate = Ratio::new(FrameRate::default() as i32, 1);
		// Pipeline creation
		gstreamer::init().expect("cannot start gstreamer");
		let main_pipeline = Pipeline::new(Some("recorder"));
		// Video elements
		#[cfg(target_os = "linux")]
			let src_video = ElementFactory::make("v4l2src", Some("desktop-video-source"))
			.expect("Unable to make desktop-video-source");
		#[cfg(target_os = "macos")]
			let src_video = ElementFactory::make("autovideosrc", Some("desktop-video-source"))
			.expect("Unable to make desktop-video-source");

		let video_tee = ElementFactory::make("tee", Some("video_tee")).unwrap();
		let video_queue0 = ElementFactory::make("queue2", Some("video_queue0")).unwrap();

		let video_queue1 = ElementFactory::make("queue2", Some("video_queue1")).unwrap();
		let video_sink1 = ElementFactory::make("appsink", Some("video_sink")).unwrap();
		let rate_video1 = ElementFactory::make("videorate", Some("desktop-video-framerate1"))
			.expect("Unable to make desktop-video-framerate");
		let convert_video1 = ElementFactory::make("videoconvert", Some("desktop-video-converter1"))
			.expect("Unable to make desktop-video-converter");

		let rate_video = ElementFactory::make("videorate", Some("desktop-video-framerate"))
			.expect("Unable to make desktop-video-framerate");
		let convert_video = ElementFactory::make("videoconvert", Some("desktop-video-converter"))
			.expect("Unable to make desktop-video-converter");
		let raw_video_caps = ElementFactory::make("capsfilter", Some("desktop-video-raw-caps"))
			.expect("Unable to make desktop-video-raw-caps");
		let encoder_video = ElementFactory::make("x264enc", Some("desktop-video-encoder"))
			.expect("Unable to make desktop-video-encoder");
		let encoder_video_caps =
			ElementFactory::make("capsfilter", Some("desktop-video-encoder-caps"))
				.expect("Unable to make desktop-video-encoder-caps");
		let queue_video = ElementFactory::make("queue2", Some("desktop-video-queue-1"))
			.expect("Unable to make desktop-video-queue-1");

		// Audio elements
		let src_audio = ElementFactory::make("alsasrc", Some("desktop-audio-source"))
			.expect("Unable to make desktop-audio-source");
		let raw_audio_caps = ElementFactory::make("capsfilter", Some("desktop-raw-audio-caps"))
			.expect("Unable to make desktop-raw-audio-caps");
		let queue_audio = ElementFactory::make("queue2", Some("desktop-audio-queue"))
			.expect("Unable to make desktop-audio-queue");
		let encoder_audio = ElementFactory::make("voaacenc", Some("desktop-audio-encoder"))
			.expect("Unable to make desktop-audio-encoder");

		// Mux and sink -- maybe sink, maybe rtmp
		let muxer = ElementFactory::make("matroskamux", Some("mkv-muxer"))
			.expect("Unable to make mkv-muxer"); // trying different muxer here
		let sink = ElementFactory::make("filesink", Some("mkv-filesink"))
			.expect("Unable to make mkv-filesink");
		// Adding video elements
		main_pipeline
			.add_many(&[
				&src_video,
				&video_tee,
				&video_queue0,
				&rate_video,
				&convert_video,
				&raw_video_caps,
				&encoder_video,
				&encoder_video_caps,
				&queue_video,

				&video_queue1,
				&rate_video1,
				&convert_video1,
				&video_sink1,
			])
			.expect("unable to add video elements to recording pipeline");
		// Adding audio elements
		main_pipeline
			.add_many(&[&src_audio, &raw_audio_caps, &queue_audio, &encoder_audio])
			.expect("unable to add audio elements to recording pipeline");
		// Adding tail elements
		main_pipeline
			.add_many(&[&muxer, &sink])
			.expect("unable to add audio elements to recording pipeline");

		// Creating capsfilters
		let raw_video_capsfilter = Caps::builder("video/x-raw")
			.field("framerate", &(gstreamer::Fraction(rate)))
			.build();
		let encoded_video_capsfilter = Caps::builder("video/x-h264")
			.field("profile", &"constrained-baseline")
			.build();
		let raw_audio_capsfilter = Caps::builder("audio/x-raw")
			.field("framerate", &(gstreamer::Fraction(rate)))
			.field("channels", 1)
			.field("rate", 48000) // does not work
			.build();
		// Setting properties
		// src_video.set_property("use-damage", true).unwrap();

		unsafe {
			src_video.set_data("num-buffers", 3000);

		}
		raw_video_caps
			.set_property("caps", &raw_video_capsfilter);

		encoder_video_caps
			.set_property("caps", &encoded_video_capsfilter);
		raw_audio_caps
			.set_property("caps", &raw_audio_capsfilter);

		encoder_video
			.set_properties(&[
				(&"intra-refresh", &true),
				(&"vbv-buf-capacity", &(0 as u32)),
				(&"qp-min", &(30 as u32)),
				(&"key-int-max", &(36 as u32)),
				// (&"pass", &"pass1"),
				// (&"speed-preset", &"fast"),
				// (&"tune", &"stillimage"),
			]);
		queue_video
			.set_properties(&[
				(&"max-size-bytes", &(0 as u32)),
				(&"max-size-buffers", &(0 as u32)),
				// (&"max-size-time", &(0 as u32)),
			]);
		queue_video.set_property("max-size-time", 0 as u64);

		// encoder_audio.set_property("bitrate-type", "constrained-vbr").unwrap();
		queue_audio
			.set_properties(&[
				(&"max-size-bytes", &(0 as u32)),
				(&"max-size-buffers", &(0 as u32)),
				// (&"max-size-time", &(0 as u32)),
			]);
		queue_audio.set_property("max-size-time", 0 as u64);


		video_queue1
			.set_properties(&[
				(&"max-size-bytes", &(512000000 as u32)),
				(&"max-size-buffers", &(0 as u32)),
				// (&"max-size-time", &(0 as u32)),
			]);
		video_queue1.set_property("max-size-time", 0 as u64);
		video_queue0
			.set_properties(&[
				(&"max-size-bytes", &(512000000 as u32)),
				(&"max-size-buffers", &(0 as u32)),
				// (&"max-size-time", &(0 as u32)),
			])
		;
		video_queue0.set_property("max-size-time", 0 as u64);


		sink.set_property("location", uri)
		;

		// Linking video elements
		Element::link_many(&[
			&src_video,
			&video_tee,
		])
			.expect("unable to link video elements in recording pipeline");
		// Linking video elements
		Element::link_many(&[
			&video_queue0,
			&rate_video,
			&convert_video,
			&raw_video_caps,
			&encoder_video,
			&encoder_video_caps,
			&queue_video,
		])
			.expect("unable to link video elements in recording pipeline");

		Element::link_many(&[
			&video_queue1,
			&rate_video1,
			&convert_video1,
			&video_sink1,
		])
			.expect("unable to link video elements in recording pipeline");

		let tee_video0_pad = video_tee.request_pad_simple("src_%u").unwrap();
		println!(
			"Obtained request pad {} for audio branch",
			tee_video0_pad.name()
		);
		let queue_video0_pad = video_queue0.static_pad("sink").unwrap();
		tee_video0_pad.link(&queue_video0_pad).unwrap();

		let tee_video1_pad = video_tee.request_pad_simple("src_%u").unwrap();
		println!(
			"Obtained request pad {} for video branch",
			tee_video1_pad.name()
		);
		let queue_video1_pad = video_queue1.static_pad("sink").unwrap();
		tee_video1_pad.link(&queue_video1_pad).unwrap();

		// Linking audio elements
		Element::link_many(&[&src_audio, &raw_audio_caps, &queue_audio, &encoder_audio])
			.expect("unable to link audio elements in recording pipeline");
		// Linking tail elements
		queue_video.link(&muxer).unwrap(); // Video to muxer // TODO (probably overcomplicating): use `link_pad` with sync handler
		encoder_audio.link(&muxer).unwrap(); // Audio to muxer // TODO (probably overcomplicating): use `link_pad` with sync handler
		Element::link_many(&[&muxer, &sink])
			.expect("unable to link audio elements in recording pipeline");

		let video_sink1 = video_sink1
			.dynamic_cast::<gstreamer_app::AppSink>()
			.expect("Sink element is expected to be an appsink!");
		video_sink1.set_caps(Some(&gstreamer::Caps::new_simple(
			"video/x-raw",
			&[("format", &"RGBA"), ("pixel-aspect-ratio", &gstreamer::Fraction::from((1, 1)))],
		)));
		video_sink1.set_callbacks(
			gstreamer_app::AppSinkCallbacks::builder()
				.new_sample(move |sink| {
					let sample = sink.pull_sample().map_err(|_| gstreamer::FlowError::Eos)?;
					let buffer = sample.buffer().ok_or(gstreamer::FlowError::Error)?;
					let map = buffer.map_readable().map_err(|_| gstreamer::FlowError::Error)?;

					let pad = sink.static_pad("sink").ok_or(gstreamer::FlowError::Error)?;

					let caps = pad.current_caps().ok_or(gstreamer::FlowError::Error)?;
					let s = caps.structure(0).ok_or(gstreamer::FlowError::Error)?;
					let width = s.get::<i32>("width").map_err(|_| gstreamer::FlowError::Error)?;
					let height = s.get::<i32>("height").map_err(|_| gstreamer::FlowError::Error)?;
					// Send original and processed image.
					                    let image = ImageBuf::from_raw(
                                            map.as_slice().to_owned(),
                                            ImageFormat::RgbaSeparate,
                                            width as _,
                                            height as _,
                                        );
                                        event_sink
                                            .submit_command(cmd::VIDEO_FRAME, image, Target::Auto)
                                            .map_err(|e| gstreamer::FlowError::Error)?;
					// let position = 		std::time::Duration::from_nanos(
					// 	pipeline.query_position::<gst::ClockTime>().map_or(0, |pos| pos.nseconds()),
					// ).as_secs();
					// event_sink
					// 	.submit_command(cmd::PLAYBACK_PROGRESS, position, Target::Auto)
					// 	.map_err(|e| gstreamer::FlowError::Error)?;

					Ok(gstreamer::FlowSuccess::Ok)
				})
				.build(),
		);
		Ok(VideoPlayer {
			bus: main_pipeline.bus().unwrap(),
			pipeline: main_pipeline,

			paused: false,
			muted: false,

		})
	}

}
